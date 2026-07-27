use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const ADDRESS: &str = "127.0.0.1:7700";
const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatsResponse {
    database_size: u64,
    used_database_size: u64,
    indexes: BTreeMap<String, IndexStats>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndexStats {
    number_of_documents: u64,
    is_indexing: bool,
}

#[derive(Deserialize)]
struct VersionResponse {
    pkg_version: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeilisearchOverview {
    version: String,
    index_count: usize,
    document_count: u64,
    database_size_bytes: u64,
    used_database_size_bytes: u64,
    indexing_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeilisearchIndex {
    uid: String,
    document_count: u64,
    indexing: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeilisearchTask {
    task_uid: u64,
    status: String,
    index_uid: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeilisearchSearchResult {
    hits: Vec<Value>,
    estimated_total_hits: u64,
    processing_time_ms: u64,
    query: String,
}

#[tauri::command]
pub async fn meilisearch_overview() -> Result<MeilisearchOverview, String> {
    tauri::async_runtime::spawn_blocking(read_overview)
        .await
        .map_err(|error| format!("Meilisearch 概览任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn meilisearch_indexes() -> Result<Vec<MeilisearchIndex>, String> {
    tauri::async_runtime::spawn_blocking(read_indexes)
        .await
        .map_err(|error| format!("Meilisearch 索引任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn meilisearch_add_documents(
    index_uid: String,
    primary_key: String,
    documents: String,
) -> Result<MeilisearchTask, String> {
    tauri::async_runtime::spawn_blocking(move || add_documents(index_uid, primary_key, documents))
        .await
        .map_err(|error| format!("Meilisearch 导入任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn meilisearch_search(
    index_uid: String,
    query: String,
) -> Result<MeilisearchSearchResult, String> {
    tauri::async_runtime::spawn_blocking(move || search(index_uid, query))
        .await
        .map_err(|error| format!("Meilisearch 搜索任务异常结束: {error}"))?
}

fn read_overview() -> Result<MeilisearchOverview, String> {
    let stats: StatsResponse = json_request("GET", "/stats", None)?;
    let version: VersionResponse = json_request("GET", "/version", None)?;
    Ok(MeilisearchOverview {
        version: version.pkg_version,
        index_count: stats.indexes.len(),
        document_count: stats
            .indexes
            .values()
            .map(|index| index.number_of_documents)
            .sum(),
        indexing_count: stats
            .indexes
            .values()
            .filter(|index| index.is_indexing)
            .count(),
        database_size_bytes: stats.database_size,
        used_database_size_bytes: stats.used_database_size,
    })
}

fn read_indexes() -> Result<Vec<MeilisearchIndex>, String> {
    let stats: StatsResponse = json_request("GET", "/stats", None)?;
    Ok(stats
        .indexes
        .into_iter()
        .map(|(uid, stats)| MeilisearchIndex {
            uid,
            document_count: stats.number_of_documents,
            indexing: stats.is_indexing,
        })
        .collect())
}

fn add_documents(
    index_uid: String,
    primary_key: String,
    documents: String,
) -> Result<MeilisearchTask, String> {
    validate_index_uid(&index_uid)?;
    if primary_key.is_empty()
        || primary_key.len() > 64
        || !primary_key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err("主键字段只能包含字母、数字、下划线和连字符".into());
    }
    if documents.len() > MAX_BODY_BYTES {
        return Err("单次导入的 JSON 不能超过 2 MiB".into());
    }
    let value: Value =
        serde_json::from_str(&documents).map_err(|error| format!("文档 JSON 无效: {error}"))?;
    if !value.is_array() {
        return Err("文档必须是 JSON 数组".into());
    }
    let path = format!("/indexes/{index_uid}/documents?primaryKey={primary_key}");
    let response: Value = json_request("POST", &path, Some(&documents))?;
    task_from_value(response)
}

fn search(index_uid: String, query: String) -> Result<MeilisearchSearchResult, String> {
    validate_index_uid(&index_uid)?;
    if query.len() > 1_000 {
        return Err("搜索内容不能超过 1000 个字符".into());
    }
    let body = serde_json::to_string(&json!({ "q": query, "limit": 50 }))
        .map_err(|error| error.to_string())?;
    let value: Value = json_request("POST", &format!("/indexes/{index_uid}/search"), Some(&body))?;
    Ok(MeilisearchSearchResult {
        hits: value["hits"].as_array().cloned().unwrap_or_default(),
        estimated_total_hits: value["estimatedTotalHits"].as_u64().unwrap_or_default(),
        processing_time_ms: value["processingTimeMs"].as_u64().unwrap_or_default(),
        query: value["query"].as_str().unwrap_or_default().into(),
    })
}

fn task_from_value(value: Value) -> Result<MeilisearchTask, String> {
    Ok(MeilisearchTask {
        task_uid: value["taskUid"]
            .as_u64()
            .ok_or_else(|| "Meilisearch 没有返回任务编号".to_string())?,
        status: value["status"].as_str().unwrap_or("enqueued").into(),
        index_uid: value["indexUid"].as_str().map(str::to_string),
    })
}

fn json_request<T: for<'de> Deserialize<'de>>(
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<T, String> {
    let body = body.unwrap_or("");
    let socket = ADDRESS
        .to_socket_addrs()
        .map_err(|error| format!("Meilisearch 地址无效: {error}"))?
        .next()
        .ok_or_else(|| "Meilisearch 地址无法解析".to_string())?;
    let mut stream = TcpStream::connect_timeout(&socket, Duration::from_secs(2))
        .map_err(|error| format!("无法连接 Meilisearch，请确认服务已启动: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(8)))
        .map_err(|error| error.to_string())?;
    write!(
        stream,
        "{method} {path} HTTP/1.0\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .map_err(|error| format!("无法请求 Meilisearch: {error}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("无法读取 Meilisearch 响应: {error}"))?;
    let (header, response_body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "Meilisearch 返回了无效的 HTTP 响应".to_string())?;
    let status = header
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or_default();
    if !(200..300).contains(&status) {
        let message = serde_json::from_str::<Value>(response_body)
            .ok()
            .and_then(|value| value["message"].as_str().map(str::to_string))
            .unwrap_or_else(|| response_body.trim().to_string());
        return Err(format!("Meilisearch HTTP {status}: {message}"));
    }
    serde_json::from_str(response_body)
        .map_err(|error| format!("无法解析 Meilisearch 响应: {error}"))
}

fn validate_index_uid(uid: &str) -> Result<(), String> {
    if uid.is_empty()
        || uid.len() > 64
        || !uid
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        Err("索引 UID 只能包含字母、数字、下划线和连字符".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_index_identifiers() {
        assert!(validate_index_uid("movies_2026").is_ok());
        assert!(validate_index_uid("../movies").is_err());
        assert!(validate_index_uid("movies list").is_err());
    }

    #[test]
    fn parses_task_response() {
        let task = task_from_value(json!({
            "taskUid": 42,
            "indexUid": "movies",
            "status": "enqueued"
        }))
        .unwrap();
        assert_eq!(task.task_uid, 42);
        assert_eq!(task.index_uid.as_deref(), Some("movies"));
    }
}
