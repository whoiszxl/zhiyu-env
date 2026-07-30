use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAX_SQL_BYTES: usize = 1024 * 1024;
const MAX_ROWS: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OlapEngine {
    Clickhouse,
    Doris,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredProfile {
    id: String,
    name: String,
    engine: OlapEngine,
    endpoint: String,
    username: String,
    password: String,
    database: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OlapProfile {
    id: String,
    name: String,
    engine: OlapEngine,
    endpoint: String,
    username: String,
    database: String,
    has_password: bool,
}

impl From<&StoredProfile> for OlapProfile {
    fn from(value: &StoredProfile) -> Self {
        Self {
            id: value.id.clone(),
            name: value.name.clone(),
            engine: value.engine,
            endpoint: value.endpoint.clone(),
            username: value.username.clone(),
            database: value.database.clone(),
            has_password: !value.password.is_empty(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OlapProfileInput {
    id: Option<String>,
    name: String,
    engine: OlapEngine,
    endpoint: String,
    username: String,
    password: Option<String>,
    database: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OlapConnectionTest {
    version: String,
    elapsed_ms: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OlapDatabaseInfo {
    name: String,
    system: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OlapTableInfo {
    name: String,
    engine: String,
    rows: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OlapQueryResult {
    columns: Vec<String>,
    rows: Vec<Vec<Option<String>>>,
    summary: String,
    elapsed_ms: u128,
    truncated: bool,
}

#[tauri::command]
pub fn olap_profile_list(engine: OlapEngine) -> Result<Vec<OlapProfile>, String> {
    Ok(load_profiles()?
        .iter()
        .filter(|profile| profile.engine == engine)
        .map(OlapProfile::from)
        .collect())
}

#[tauri::command]
pub fn olap_profile_save(input: OlapProfileInput) -> Result<OlapProfile, String> {
    validate_profile_input(&input)?;
    let mut profiles = load_profiles()?;
    let existing_index = input
        .id
        .as_ref()
        .and_then(|id| profiles.iter().position(|profile| &profile.id == id));
    let password = match (input.password, existing_index) {
        (Some(password), _) if !password.is_empty() => password,
        (_, Some(index)) => profiles[index].password.clone(),
        _ => String::new(),
    };
    let profile = StoredProfile {
        id: input.id.unwrap_or_else(new_id),
        name: input.name.trim().to_string(),
        engine: input.engine,
        endpoint: normalize_endpoint(&input.endpoint)?,
        username: input.username.trim().to_string(),
        password,
        database: input.database.trim().to_string(),
    };
    if let Some(index) = existing_index {
        profiles[index] = profile.clone();
    } else {
        profiles.push(profile.clone());
    }
    persist_profiles(&profiles)?;
    Ok(OlapProfile::from(&profile))
}

#[tauri::command]
pub fn olap_profile_delete(id: String) -> Result<(), String> {
    let mut profiles = load_profiles()?;
    let original_len = profiles.len();
    profiles.retain(|profile| profile.id != id);
    if profiles.len() == original_len {
        return Err("连接配置不存在".into());
    }
    persist_profiles(&profiles)
}

#[tauri::command]
pub async fn olap_connection_test(id: String) -> Result<OlapConnectionTest, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let profile = find_profile(&id)?;
        let started = Instant::now();
        let result = query(
            &profile,
            effective_database(&profile, None),
            "SELECT version() AS version",
        )?;
        Ok(OlapConnectionTest {
            version: result
                .rows
                .first()
                .and_then(|row| row.first())
                .cloned()
                .flatten()
                .unwrap_or_else(|| "unknown".into()),
            elapsed_ms: started.elapsed().as_millis(),
        })
    })
    .await
    .map_err(|error| format!("连接测试任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn olap_database_list(id: String) -> Result<Vec<OlapDatabaseInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let profile = find_profile(&id)?;
        let sql = match profile.engine {
            OlapEngine::Clickhouse => {
                "SELECT name, name IN ('system','information_schema','INFORMATION_SCHEMA') FROM system.databases ORDER BY name"
            }
            OlapEngine::Doris => "SHOW DATABASES",
        };
        let result = query(&profile, effective_database(&profile, None), sql)?;
        Ok(result
            .rows
            .into_iter()
            .map(|row| {
                let name = cell(&row, 0);
                let system = match profile.engine {
                    OlapEngine::Clickhouse => cell(&row, 1) == "1",
                    OlapEngine::Doris => {
                        matches!(name.as_str(), "information_schema" | "mysql" | "__internal_schema")
                    }
                };
                OlapDatabaseInfo { name, system }
            })
            .collect())
    })
    .await
    .map_err(|error| format!("数据库列表任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn olap_table_list(id: String, database: String) -> Result<Vec<OlapTableInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_identifier(&database, "数据库")?;
        let profile = find_profile(&id)?;
        let sql = match profile.engine {
            OlapEngine::Clickhouse => format!(
                "SELECT name, engine, total_rows FROM system.tables WHERE database = '{}' ORDER BY name",
                sql_literal(&database)
            ),
            OlapEngine::Doris => format!("SHOW TABLE STATUS FROM {}", mysql_identifier(&database)),
        };
        let result = query(&profile, &database, &sql)?;
        Ok(result
            .rows
            .into_iter()
            .map(|row| OlapTableInfo {
                name: cell(&row, 0),
                engine: match profile.engine {
                    OlapEngine::Clickhouse => cell(&row, 1),
                    OlapEngine::Doris => cell(&row, 1),
                },
                rows: match profile.engine {
                    OlapEngine::Clickhouse => cell(&row, 2).parse().ok(),
                    OlapEngine::Doris => cell(&row, 4).parse().ok(),
                },
            })
            .collect())
    })
    .await
    .map_err(|error| format!("数据表列表任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn olap_execute(
    id: String,
    database: String,
    sql: String,
    confirmed: bool,
) -> Result<OlapQueryResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        if sql.trim().is_empty() || sql.len() > MAX_SQL_BYTES {
            return Err("SQL 不能为空且不能超过 1 MiB".into());
        }
        if !database.is_empty() {
            validate_identifier(&database, "数据库")?;
        }
        if !confirmed {
            if let Some(keyword) = destructive_keyword(&sql) {
                return Err(format!("CONFIRM_REQUIRED:{keyword}"));
            }
        }
        let profile = find_profile(&id)?;
        query(
            &profile,
            effective_database(&profile, Some(&database)),
            &sql,
        )
    })
    .await
    .map_err(|error| format!("SQL 执行任务异常结束: {error}"))?
}

fn query(profile: &StoredProfile, database: &str, sql: &str) -> Result<OlapQueryResult, String> {
    match profile.engine {
        OlapEngine::Clickhouse => clickhouse_query(profile, database, sql),
        OlapEngine::Doris => doris_query(profile, database, sql),
    }
}

fn clickhouse_query(
    profile: &StoredProfile,
    database: &str,
    sql: &str,
) -> Result<OlapQueryResult, String> {
    let mut url = reqwest::Url::parse(&profile.endpoint)
        .map_err(|error| format!("ClickHouse Endpoint 无效: {error}"))?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("database", database);
        pairs.append_pair("default_format", "JSONCompact");
        pairs.append_pair("max_result_rows", &(MAX_ROWS + 1).to_string());
        pairs.append_pair("result_overflow_mode", "break");
    }
    let client = http_client()?;
    let started = Instant::now();
    let response = client
        .post(url)
        .basic_auth(&profile.username, Some(&profile.password))
        .body(sql.to_string())
        .send()
        .map_err(|error| format!("无法连接 ClickHouse: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .map_err(|error| format!("读取 ClickHouse 响应失败: {error}"))?;
    if !status.is_success() {
        return Err(format!("ClickHouse HTTP {status}: {}", body.trim()));
    }
    parse_clickhouse_response(&body, started.elapsed().as_millis())
}

fn doris_query(
    profile: &StoredProfile,
    database: &str,
    sql: &str,
) -> Result<OlapQueryResult, String> {
    let mut url = reqwest::Url::parse(&profile.endpoint)
        .map_err(|error| format!("Doris Endpoint 无效: {error}"))?;
    url.set_path("");
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| "Doris Endpoint 不能作为基础地址".to_string())?;
        segments.extend(["api", "query", "default_cluster", database]);
    }
    let client = http_client()?;
    let started = Instant::now();
    let response = client
        .post(url)
        .basic_auth(&profile.username, Some(&profile.password))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(
            serde_json::to_vec(&json!({ "stmt": sql }))
                .map_err(|error| format!("创建 Doris 请求失败: {error}"))?,
        )
        .send()
        .map_err(|error| format!("无法连接 Doris FE: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .map_err(|error| format!("读取 Doris 响应失败: {error}"))?;
    if !status.is_success() {
        return Err(format!("Doris HTTP {status}: {}", body.trim()));
    }
    parse_doris_response(&body, started.elapsed().as_millis())
}

fn parse_clickhouse_response(body: &str, elapsed_ms: u128) -> Result<OlapQueryResult, String> {
    if body.trim().is_empty() {
        return Ok(empty_result(elapsed_ms));
    }
    let value: Value = serde_json::from_str(body)
        .map_err(|error| format!("ClickHouse 返回了无法解析的数据: {error}"))?;
    let columns = value["meta"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| item["name"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default();
    let (rows, truncated) = json_rows(&value["data"]);
    Ok(OlapQueryResult {
        summary: format!("返回 {} 行", rows.len()),
        columns,
        rows,
        elapsed_ms,
        truncated,
    })
}

fn parse_doris_response(body: &str, elapsed_ms: u128) -> Result<OlapQueryResult, String> {
    let value: Value = serde_json::from_str(body)
        .map_err(|error| format!("Doris 返回了无法解析的数据: {error}"))?;
    if value["code"].as_i64().unwrap_or(-1) != 0 {
        return Err(value["msg"]
            .as_str()
            .unwrap_or("Doris SQL 执行失败")
            .to_string());
    }
    let data = &value["data"];
    if data["type"].as_str() != Some("result_set") {
        return Ok(empty_result(elapsed_ms));
    }
    let columns = data["meta"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    item["name"]
                        .as_str()
                        .or_else(|| item["column_name"].as_str())
                        .unwrap_or_default()
                        .to_string()
                })
                .collect()
        })
        .unwrap_or_default();
    let (rows, truncated) = json_rows(&data["data"]);
    Ok(OlapQueryResult {
        summary: format!("返回 {} 行", rows.len()),
        columns,
        rows,
        elapsed_ms,
        truncated,
    })
}

fn json_rows(value: &Value) -> (Vec<Vec<Option<String>>>, bool) {
    let items = value.as_array().cloned().unwrap_or_default();
    let truncated = items.len() > MAX_ROWS;
    let rows = items
        .into_iter()
        .take(MAX_ROWS)
        .map(|row| {
            row.as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(json_cell)
                .collect()
        })
        .collect();
    (rows, truncated)
}

fn json_cell(value: Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(value) => Some(value),
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        other => Some(other.to_string()),
    }
}

fn empty_result(elapsed_ms: u128) -> OlapQueryResult {
    OlapQueryResult {
        columns: Vec::new(),
        rows: Vec::new(),
        summary: "执行完成".into(),
        elapsed_ms,
        truncated: false,
    }
}

fn http_client() -> Result<Client, String> {
    crate::settings::reqwest_client_builder(crate::settings::ProxyScope::Network)?
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("创建 HTTP 客户端失败: {error}"))
}

fn find_profile(id: &str) -> Result<StoredProfile, String> {
    load_profiles()?
        .into_iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| "连接配置不存在".into())
}

fn load_profiles() -> Result<Vec<StoredProfile>, String> {
    let path = profiles_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let bytes = fs::read(&path).map_err(|error| format!("读取 OLAP 连接配置失败: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("解析 OLAP 连接配置失败: {error}"))
}

fn persist_profiles(profiles: &[StoredProfile]) -> Result<(), String> {
    let path = profiles_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("创建配置目录失败: {error}"))?;
    }
    let bytes = serde_json::to_vec_pretty(profiles)
        .map_err(|error| format!("序列化 OLAP 连接配置失败: {error}"))?;
    fs::write(&path, bytes).map_err(|error| format!("保存 OLAP 连接配置失败: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("设置连接配置权限失败: {error}"))?;
    }
    Ok(())
}

fn profiles_path() -> Result<PathBuf, String> {
    dirs::config_dir()
        .map(|directory| directory.join("zhiyu-env/olap-connections.json"))
        .ok_or_else(|| "无法定位用户配置目录".into())
}

fn validate_profile_input(input: &OlapProfileInput) -> Result<(), String> {
    if input.name.trim().is_empty() || input.name.len() > 80 {
        return Err("连接名称不能为空且不能超过 80 个字符".into());
    }
    if input.username.trim().is_empty() || input.username.len() > 128 {
        return Err("用户名不能为空且不能超过 128 个字符".into());
    }
    if input.database.len() > 256 {
        return Err("默认数据库名称过长".into());
    }
    normalize_endpoint(&input.endpoint)?;
    Ok(())
}

fn normalize_endpoint(value: &str) -> Result<String, String> {
    let mut endpoint = value.trim().trim_end_matches('/').to_string();
    if !endpoint.contains("://") {
        endpoint = format!("http://{endpoint}");
    }
    let url =
        reqwest::Url::parse(&endpoint).map_err(|_| "请输入有效的 HTTP Endpoint".to_string())?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err("Endpoint 仅支持 HTTP 或 HTTPS".into());
    }
    Ok(endpoint)
}

fn effective_database<'a>(profile: &'a StoredProfile, requested: Option<&'a str>) -> &'a str {
    let requested = requested.unwrap_or_default();
    if !requested.is_empty() {
        requested
    } else if !profile.database.is_empty() {
        &profile.database
    } else {
        match profile.engine {
            OlapEngine::Clickhouse => "default",
            OlapEngine::Doris => "information_schema",
        }
    }
}

fn validate_identifier(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 256 || value.contains('\0') {
        Err(format!("{label}名称无效"))
    } else {
        Ok(())
    }
}

fn destructive_keyword(sql: &str) -> Option<&'static str> {
    let normalized = sql.trim_start().to_ascii_uppercase();
    ["DROP", "TRUNCATE", "DELETE", "ALTER", "RENAME"]
        .into_iter()
        .find(|keyword| normalized.starts_with(keyword))
}

fn sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn mysql_identifier(value: &str) -> String {
    format!("`{}`", value.replace('`', "``"))
}

fn cell(row: &[Option<String>], index: usize) -> String {
    row.get(index).cloned().flatten().unwrap_or_default()
}

fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("olap-{nanos:x}-{:x}", std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_is_normalized_and_restricted_to_http() {
        assert_eq!(
            normalize_endpoint("127.0.0.1:8123/").unwrap(),
            "http://127.0.0.1:8123"
        );
        assert!(normalize_endpoint("file:///tmp/database").is_err());
    }

    #[test]
    fn parses_clickhouse_compact_json() {
        let result = parse_clickhouse_response(
            r#"{"meta":[{"name":"name","type":"String"}],"data":[["demo"]],"rows":1}"#,
            4,
        )
        .unwrap();
        assert_eq!(result.columns, ["name"]);
        assert_eq!(result.rows[0][0].as_deref(), Some("demo"));
    }

    #[test]
    fn parses_doris_statement_api_result() {
        let result = parse_doris_response(
            r#"{"msg":"success","code":0,"data":{"type":"result_set","data":[["4.1.1"]],"meta":[{"name":"version","type":"VARCHAR"}]}}"#,
            5,
        )
        .unwrap();
        assert_eq!(result.columns, ["version"]);
        assert_eq!(result.rows[0][0].as_deref(), Some("4.1.1"));
    }

    #[test]
    fn destructive_queries_need_confirmation() {
        assert_eq!(destructive_keyword("  DROP TABLE events"), Some("DROP"));
        assert_eq!(destructive_keyword("SELECT 1"), None);
    }
}
