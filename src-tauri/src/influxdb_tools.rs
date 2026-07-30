use reqwest::blocking::RequestBuilder;
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;

const BASE_URL: &str = "http://127.0.0.1:8181";
const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
const MAX_ROWS: usize = 500;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfluxdbOverview {
    ready: bool,
    database_count: usize,
    endpoint: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfluxdbDatabase {
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfluxdbQueryResult {
    columns: Vec<String>,
    rows: Vec<Vec<Value>>,
    row_count: usize,
    truncated: bool,
}

#[tauri::command]
pub async fn influxdb_overview() -> Result<InfluxdbOverview, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let databases = list_databases()?;
        Ok(InfluxdbOverview {
            ready: true,
            database_count: databases.len(),
            endpoint: BASE_URL,
        })
    })
    .await
    .map_err(|error| format!("InfluxDB 概览任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn influxdb_databases() -> Result<Vec<InfluxdbDatabase>, String> {
    tauri::async_runtime::spawn_blocking(list_databases)
        .await
        .map_err(|error| format!("InfluxDB 数据库任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn influxdb_database_create(
    name: String,
    retention_period: Option<String>,
) -> Result<Vec<InfluxdbDatabase>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_database(&name)?;
        let mut body = json!({ "db": name });
        if let Some(retention) = retention_period.filter(|value| !value.trim().is_empty()) {
            if retention.len() > 32
                || !retention.bytes().all(|byte| {
                    byte.is_ascii_digit() || matches!(byte, b'h' | b'd' | b'w' | b'm' | b'y')
                })
            {
                return Err("保留周期应使用 24h、30d 或 4w 等格式".into());
            }
            body["retention_period"] = Value::String(retention);
        }
        send_json("POST", "/api/v3/configure/database", Some(body))?;
        list_databases()
    })
    .await
    .map_err(|error| format!("InfluxDB 创建数据库任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn influxdb_database_delete(name: String) -> Result<Vec<InfluxdbDatabase>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_database(&name)?;
        let path = format!("/api/v3/configure/database?db={}", percent_encode(&name));
        send_json("DELETE", &path, None)?;
        list_databases()
    })
    .await
    .map_err(|error| format!("InfluxDB 删除数据库任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn influxdb_query(
    database: String,
    query: String,
) -> Result<InfluxdbQueryResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_database(&database)?;
        let query = query.trim();
        if query.is_empty() {
            return Err("请输入 SQL 查询".into());
        }
        if query.len() > MAX_INPUT_BYTES {
            return Err("SQL 查询不能超过 2 MiB".into());
        }
        let path = "/api/v3/query_sql";
        let value = send_json(
            "POST",
            path,
            Some(json!({ "db": database, "q": query, "format": "json" })),
        )?;
        tabular_result(value)
    })
    .await
    .map_err(|error| format!("InfluxDB 查询任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn influxdb_write(
    database: String,
    line_protocol: String,
    precision: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_database(&database)?;
        if line_protocol.trim().is_empty() {
            return Err("请输入 Line Protocol 数据".into());
        }
        if line_protocol.len() > MAX_INPUT_BYTES {
            return Err("单次写入不能超过 2 MiB".into());
        }
        let precision = match precision.as_str() {
            "second" | "millisecond" | "microsecond" | "nanosecond" | "auto" => precision,
            _ => return Err("不支持的时间精度".into()),
        };
        let path = format!(
            "/api/v3/write_lp?db={}&precision={}",
            percent_encode(&database),
            precision
        );
        let response = request("POST", &path, Some("text/plain"))
            .body(line_protocol)
            .send()
            .map_err(request_error)?;
        ensure_success(response).map(|_| ())
    })
    .await
    .map_err(|error| format!("InfluxDB 写入任务异常结束: {error}"))?
}

fn list_databases() -> Result<Vec<InfluxdbDatabase>, String> {
    let value = send_json("GET", "/api/v3/configure/database?format=json", None)?;
    let values = value
        .get("databases")
        .and_then(Value::as_array)
        .or_else(|| value.as_array())
        .cloned()
        .unwrap_or_default();
    let mut result = values
        .into_iter()
        .filter_map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .or_else(|| {
                    value
                        .get("iox::database")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .or_else(|| {
                    value
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
        })
        .map(|name| InfluxdbDatabase { name })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(result)
}

fn send_json(method: &str, path: &str, body: Option<Value>) -> Result<Value, String> {
    let mut builder = request(method, path, Some("application/json"));
    if let Some(body) = body {
        builder = builder.body(
            serde_json::to_string(&body)
                .map_err(|error| format!("无法编码 InfluxDB 请求: {error}"))?,
        );
    }
    let response = builder.send().map_err(request_error)?;
    let text = ensure_success(response)?;
    if text.trim().is_empty() {
        Ok(Value::Null)
    } else {
        serde_json::from_str(&text).map_err(|error| format!("无法解析 InfluxDB 响应: {error}"))
    }
}

fn request(method: &str, path: &str, content_type: Option<&str>) -> RequestBuilder {
    let client = crate::settings::reqwest_client_builder(crate::settings::ProxyScope::Network)
        .unwrap_or_else(|_| reqwest::blocking::Client::builder().no_proxy())
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(15))
        .build()
        .expect("InfluxDB HTTP client configuration is valid");
    let method = reqwest::Method::from_bytes(method.as_bytes()).expect("valid HTTP method");
    let mut request = client.request(method, format!("{BASE_URL}{path}"));
    if let Some(content_type) = content_type {
        request = request.header("Content-Type", content_type);
    }
    request
}

fn ensure_success(response: reqwest::blocking::Response) -> Result<String, String> {
    let status = response.status();
    let text = response
        .text()
        .map_err(|error| format!("无法读取 InfluxDB 响应: {error}"))?;
    if status.is_success() {
        Ok(text)
    } else {
        Err(format!(
            "InfluxDB HTTP {}: {}",
            status.as_u16(),
            text.trim()
        ))
    }
}

fn request_error(error: reqwest::Error) -> String {
    format!("无法连接 InfluxDB，请确认服务已启动: {error}")
}

fn tabular_result(value: Value) -> Result<InfluxdbQueryResult, String> {
    let objects = value
        .as_array()
        .cloned()
        .ok_or_else(|| "InfluxDB 查询响应不是 JSON 数组".to_string())?;
    let row_count = objects.len();
    let truncated = row_count > MAX_ROWS;
    let mut columns = Vec::new();
    for value in objects.iter().take(MAX_ROWS) {
        if let Some(object) = value.as_object() {
            for key in object.keys() {
                if !columns.contains(key) {
                    columns.push(key.clone());
                }
            }
        }
    }
    let rows = objects
        .into_iter()
        .take(MAX_ROWS)
        .map(|value| {
            columns
                .iter()
                .map(|column| value.get(column).cloned().unwrap_or(Value::Null))
                .collect()
        })
        .collect();
    Ok(InfluxdbQueryResult {
        columns,
        rows,
        row_count,
        truncated,
    })
}

fn validate_database(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 64
        || name.starts_with('_')
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'/'))
    {
        Err("数据库名称只能包含字母、数字、下划线、连字符和斜杠，且不能以下划线开头".into())
    } else {
        Ok(())
    }
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_database_names() {
        assert!(validate_database("metrics-dev").is_ok());
        assert!(validate_database("telegraf/autogen").is_ok());
        assert!(validate_database("../metrics").is_err());
        assert!(validate_database("_internal").is_err());
    }

    #[test]
    fn converts_query_rows() {
        let result = tabular_result(json!([
            {"time": "2026-01-01T00:00:00Z", "value": 12.5},
            {"time": "2026-01-01T00:01:00Z", "value": 13.0}
        ]))
        .unwrap();
        assert_eq!(result.row_count, 2);
        assert_eq!(result.columns, vec!["time", "value"]);
    }
}
