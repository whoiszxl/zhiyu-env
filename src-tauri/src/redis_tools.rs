use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

const REDIS_PORT: u16 = 6379;
const MAX_PATTERN_LENGTH: usize = 512;
const MAX_COMMAND_ARGUMENTS: usize = 128;
const MAX_ARGUMENT_LENGTH: usize = 64 * 1024;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_STRING_BYTES: u64 = 64 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisOverview {
    version: String,
    used_memory_bytes: u64,
    connected_clients: u64,
    operations_per_second: u64,
    total_keys: u64,
    hit_rate_percent: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisScanResult {
    next_cursor: String,
    keys: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisKeyDetail {
    key: String,
    key_type: String,
    ttl_seconds: i64,
    memory_bytes: Option<u64>,
    value: Value,
    truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisCommandResult {
    output: String,
    elapsed_ms: u128,
}

#[tauri::command]
pub async fn redis_overview() -> Result<RedisOverview, String> {
    tauri::async_runtime::spawn_blocking(read_overview)
        .await
        .map_err(|error| format!("Redis 信息任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn redis_scan_keys(
    database: u8,
    cursor: String,
    pattern: String,
) -> Result<RedisScanResult, String> {
    tauri::async_runtime::spawn_blocking(move || scan_keys(database, cursor, pattern))
        .await
        .map_err(|error| format!("Redis Key 扫描任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn redis_key_detail(database: u8, key: String) -> Result<RedisKeyDetail, String> {
    tauri::async_runtime::spawn_blocking(move || read_key_detail(database, key))
        .await
        .map_err(|error| format!("Redis Key 读取任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn redis_execute(
    database: u8,
    arguments: Vec<String>,
    confirmed: bool,
) -> Result<RedisCommandResult, String> {
    tauri::async_runtime::spawn_blocking(move || execute_command(database, arguments, confirmed))
        .await
        .map_err(|error| format!("Redis 命令任务异常结束: {error}"))?
}

fn redis_cli() -> Result<PathBuf, String> {
    let config = crate::commands::service_config(devbox_core::ServiceKind::Redis)?;
    let executable = config
        .executable
        .parent()
        .map(|directory| directory.join("redis-cli"))
        .ok_or_else(|| "Redis 可执行程序路径无效".to_string())?;
    if !executable.is_file() {
        return Err(format!("redis-cli 不存在: {}", executable.display()));
    }
    Ok(executable)
}

fn base_command(database: u8) -> Result<Command, String> {
    if database > 15 {
        return Err("Redis database 必须在 0 到 15 之间".into());
    }
    let mut command = Command::new(redis_cli()?);
    command.args([
        "-h",
        "127.0.0.1",
        "-p",
        &REDIS_PORT.to_string(),
        "-n",
        &database.to_string(),
    ]);
    Ok(command)
}

fn read_overview() -> Result<RedisOverview, String> {
    let output = run_raw(0, &["INFO".into()])?;
    let mut version = String::new();
    let mut used_memory_bytes = 0;
    let mut connected_clients = 0;
    let mut operations_per_second = 0;
    let mut total_keys = 0;
    let mut hits = 0_u64;
    let mut misses = 0_u64;

    for line in output.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        match name {
            "redis_version" => version = value.trim().into(),
            "used_memory" => used_memory_bytes = parse_u64(value),
            "connected_clients" => connected_clients = parse_u64(value),
            "instantaneous_ops_per_sec" => operations_per_second = parse_u64(value),
            "keyspace_hits" => hits = parse_u64(value),
            "keyspace_misses" => misses = parse_u64(value),
            name if name.starts_with("db") => {
                if let Some(keys) = value.split(',').find_map(|part| part.strip_prefix("keys=")) {
                    total_keys += parse_u64(keys);
                }
            }
            _ => {}
        }
    }

    let lookups = hits + misses;
    let hit_rate_percent = if lookups == 0 {
        0.0
    } else {
        hits as f64 / lookups as f64 * 100.0
    };

    Ok(RedisOverview {
        version,
        used_memory_bytes,
        connected_clients,
        operations_per_second,
        total_keys,
        hit_rate_percent,
    })
}

fn scan_keys(database: u8, cursor: String, pattern: String) -> Result<RedisScanResult, String> {
    if pattern.len() > MAX_PATTERN_LENGTH {
        return Err("Key 搜索条件过长".into());
    }
    let cursor = cursor.parse::<u64>().unwrap_or(0).to_string();
    let pattern = if pattern.trim().is_empty() {
        "*".into()
    } else {
        pattern
    };
    let value = run_json(
        database,
        &[
            "SCAN".into(),
            cursor,
            "MATCH".into(),
            pattern,
            "COUNT".into(),
            "100".into(),
        ],
    )?;
    let values = value
        .as_array()
        .ok_or_else(|| "Redis 返回了无效的 SCAN 结果".to_string())?;
    let next_cursor = values
        .first()
        .and_then(json_scalar)
        .unwrap_or_else(|| "0".into());
    let keys = values
        .get(1)
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(json_scalar).collect())
        .unwrap_or_default();

    Ok(RedisScanResult { next_cursor, keys })
}

fn read_key_detail(database: u8, key: String) -> Result<RedisKeyDetail, String> {
    if key.len() > MAX_ARGUMENT_LENGTH {
        return Err("Key 名称过长".into());
    }
    let key_type = run_raw(database, &["TYPE".into(), key.clone()])?
        .trim()
        .to_lowercase();
    if key_type == "none" {
        return Err("Key 已不存在".into());
    }
    let ttl_seconds = run_raw(database, &["TTL".into(), key.clone()])?
        .trim()
        .parse::<i64>()
        .unwrap_or(-2);
    let memory_bytes = run_raw(database, &["MEMORY".into(), "USAGE".into(), key.clone()])
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok());

    let (arguments, truncated) = match key_type.as_str() {
        "string" => {
            let length = run_raw(database, &["STRLEN".into(), key.clone()])?
                .trim()
                .parse::<u64>()
                .unwrap_or_default();
            (
                vec![
                    "GETRANGE".into(),
                    key.clone(),
                    "0".into(),
                    (MAX_STRING_BYTES - 1).to_string(),
                ],
                length > MAX_STRING_BYTES,
            )
        }
        "hash" => (
            vec![
                "HSCAN".into(),
                key.clone(),
                "0".into(),
                "COUNT".into(),
                "100".into(),
            ],
            true,
        ),
        "list" => (
            vec!["LRANGE".into(), key.clone(), "0".into(), "99".into()],
            true,
        ),
        "set" => (
            vec![
                "SSCAN".into(),
                key.clone(),
                "0".into(),
                "COUNT".into(),
                "100".into(),
            ],
            true,
        ),
        "zset" => (
            vec![
                "ZRANGE".into(),
                key.clone(),
                "0".into(),
                "99".into(),
                "WITHSCORES".into(),
            ],
            true,
        ),
        "stream" => (
            vec![
                "XRANGE".into(),
                key.clone(),
                "-".into(),
                "+".into(),
                "COUNT".into(),
                "50".into(),
            ],
            true,
        ),
        _ => return Err(format!("暂不支持查看 {key_type} 类型")),
    };
    let value = run_json(database, &arguments)?;

    Ok(RedisKeyDetail {
        key,
        key_type,
        ttl_seconds,
        memory_bytes,
        value,
        truncated,
    })
}

fn execute_command(
    database: u8,
    arguments: Vec<String>,
    confirmed: bool,
) -> Result<RedisCommandResult, String> {
    if arguments.is_empty() {
        return Err("请输入 Redis 命令".into());
    }
    if arguments.len() > MAX_COMMAND_ARGUMENTS
        || arguments
            .iter()
            .any(|value| value.len() > MAX_ARGUMENT_LENGTH)
    {
        return Err("命令参数过多或过长".into());
    }

    let name = arguments[0].to_ascii_uppercase();
    if [
        "MONITOR",
        "SUBSCRIBE",
        "PSUBSCRIBE",
        "BLPOP",
        "BRPOP",
        "BLMOVE",
        "BZPOPMIN",
        "BZPOPMAX",
        "XREAD",
        "XREADGROUP",
        "EVAL",
        "EVALSHA",
        "FUNCTION",
        "SHUTDOWN",
        "DEBUG",
    ]
    .contains(&name.as_str())
    {
        return Err(format!("智屿命令台不执行可能长期阻塞或中断服务的 {name}"));
    }
    if ["FLUSHALL", "FLUSHDB"].contains(&name.as_str()) && !confirmed {
        return Err(format!("CONFIRM_REQUIRED:{name}"));
    }

    let started = Instant::now();
    let mut output = run_raw(database, &arguments)?;
    if output.len() > MAX_OUTPUT_BYTES {
        output.truncate(MAX_OUTPUT_BYTES);
        output.push_str("\n…输出已截断");
    }
    Ok(RedisCommandResult {
        output,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn run_raw(database: u8, arguments: &[String]) -> Result<String, String> {
    let mut command = base_command(database)?;
    let output = command.arg("--raw").args(arguments).output();
    parse_output(output)
}

fn run_json(database: u8, arguments: &[String]) -> Result<Value, String> {
    let mut command = base_command(database)?;
    let output = command.arg("--json").args(arguments).output();
    let text = parse_output(output)?;
    serde_json::from_str(text.trim()).map_err(|error| format!("Redis JSON 解析失败: {error}"))
}

fn parse_output(output: std::io::Result<std::process::Output>) -> Result<String, String> {
    let output = output.map_err(|error| format!("redis-cli 执行失败: {error}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if is_redis_error(&stdout) {
        return Err(stdout.trim().into());
    }
    Ok(stdout)
}

fn parse_u64(value: &str) -> u64 {
    value.trim().parse().unwrap_or_default()
}

fn json_scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn is_redis_error(output: &str) -> bool {
    let first_word = output.split_whitespace().next();
    matches!(
        first_word,
        Some(
            "ERR"
                | "WRONGTYPE"
                | "NOAUTH"
                | "NOPERM"
                | "BUSY"
                | "NOSCRIPT"
                | "READONLY"
                | "MOVED"
                | "ASK"
                | "CLUSTERDOWN"
                | "LOADING"
                | "MASTERDOWN"
                | "MISCONF"
                | "OOM"
                | "EXECABORT"
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_redis_errors_without_mistaking_regular_values() {
        assert!(is_redis_error("ERR unknown command"));
        assert!(is_redis_error("WRONGTYPE Operation against a key"));
        assert!(!is_redis_error("hello world"));
        assert!(!is_redis_error(""));
    }

    #[test]
    fn blocks_unsafe_commands_before_starting_redis_cli() {
        let blocked = execute_command(0, vec!["monitor".into()], false).unwrap_err();
        assert!(blocked.contains("MONITOR"));

        let confirmation = execute_command(0, vec!["flushdb".into()], false).unwrap_err();
        assert_eq!(confirmation, "CONFIRM_REQUIRED:FLUSHDB");
    }
}
