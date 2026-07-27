use crate::commands::service_config;
use devbox_core::ServiceKind;
use serde::Serialize;
use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Instant;

const BROKER: &str = "tcp://127.0.0.1:9092";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KafkaOverview {
    version: String,
    broker: &'static str,
    topic_count: usize,
    storage_engine: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KafkaTopic {
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KafkaPublishResult {
    topic: String,
    payload_bytes: usize,
    elapsed_ms: u128,
}

#[tauri::command]
pub async fn kafka_overview() -> Result<KafkaOverview, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let config = service_config(ServiceKind::Kafka)?;
        let topics = list_topics(&config.executable)?;
        Ok(KafkaOverview {
            version: config.version,
            broker: BROKER,
            topic_count: topics.len(),
            storage_engine: "SQLite",
        })
    })
    .await
    .map_err(|error| format!("Kafka 概览任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn kafka_topics() -> Result<Vec<KafkaTopic>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let config = service_config(ServiceKind::Kafka)?;
        list_topics(&config.executable)
    })
    .await
    .map_err(|error| format!("Kafka 主题读取任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn kafka_topic_create(name: String, partitions: u16) -> Result<Vec<KafkaTopic>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_topic(&name)?;
        if !(1..=32).contains(&partitions) {
            return Err("分区数必须在 1 到 32 之间".into());
        }
        let config = service_config(ServiceKind::Kafka)?;
        run_tansu(
            &config.executable,
            &[
                "topic",
                "create",
                "--broker",
                BROKER,
                "--partitions",
                &partitions.to_string(),
                &name,
            ],
        )?;
        list_topics(&config.executable)
    })
    .await
    .map_err(|error| format!("Kafka 主题创建任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn kafka_topic_delete(name: String) -> Result<Vec<KafkaTopic>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_topic(&name)?;
        let config = service_config(ServiceKind::Kafka)?;
        run_tansu(
            &config.executable,
            &["topic", "delete", "--broker", BROKER, &name],
        )?;
        list_topics(&config.executable)
    })
    .await
    .map_err(|error| format!("Kafka 主题删除任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn kafka_publish(
    topic: String,
    key: Option<String>,
    payload: String,
) -> Result<KafkaPublishResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_topic(&topic)?;
        if payload.as_bytes().len() > 1024 * 1024 {
            return Err("测试消息不能超过 1 MB".into());
        }
        let config = service_config(ServiceKind::Kafka)?;
        let value =
            serde_json::from_str::<Value>(&payload).unwrap_or(Value::String(payload.clone()));
        let record = match key.filter(|key| !key.is_empty()) {
            Some(key) => json!({ "key": key, "value": value }),
            None => json!({ "value": value }),
        };
        let started = Instant::now();
        let mut child = Command::new(&config.executable)
            .args(["cat", "produce", "--broker", BROKER, &topic, "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("无法启动 Tansu 客户端: {error}"))?;
        child
            .stdin
            .take()
            .ok_or_else(|| "无法打开 Tansu 标准输入".to_string())?
            .write_all(format!("{record}\n").as_bytes())
            .map_err(|error| format!("写入测试消息失败: {error}"))?;
        let output = child
            .wait_with_output()
            .map_err(|error| format!("等待 Tansu 客户端失败: {error}"))?;
        if !output.status.success() {
            return Err(command_error(&output.stderr, &output.stdout));
        }
        Ok(KafkaPublishResult {
            topic,
            payload_bytes: payload.len(),
            elapsed_ms: started.elapsed().as_millis(),
        })
    })
    .await
    .map_err(|error| format!("Kafka 消息发布任务异常结束: {error}"))?
}

fn list_topics(executable: &std::path::Path) -> Result<Vec<KafkaTopic>, String> {
    let output = run_tansu(executable, &["topic", "list", "--broker", BROKER])?;
    let mut names = parse_topic_names(&output);
    names.sort();
    names.dedup();
    Ok(names.into_iter().map(|name| KafkaTopic { name }).collect())
}

fn run_tansu(executable: &std::path::Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new(executable)
        .args(arguments)
        .output()
        .map_err(|error| format!("无法启动 Tansu 客户端: {error}"))?;
    if !output.status.success() {
        return Err(command_error(&output.stderr, &output.stdout));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn command_error(stderr: &[u8], stdout: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let stdout = String::from_utf8_lossy(stdout);
    let detail = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    format!(
        "Kafka 操作失败：{}",
        if detail.is_empty() {
            "Tansu 未返回错误详情"
        } else {
            detail
        }
    )
}

fn validate_topic(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 249
        || name.starts_with('.')
        || name.starts_with('-')
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err("主题名只能包含字母、数字、点、下划线和连字符，且不能以点或连字符开头".into());
    }
    Ok(())
}

fn parse_topic_names(output: &str) -> Vec<String> {
    if let Ok(Value::Array(values)) = serde_json::from_str::<Value>(output) {
        return values
            .into_iter()
            .filter_map(|value| match value {
                Value::String(name) => Some(name),
                Value::Object(mut object) => object
                    .remove("name")
                    .or_else(|| object.remove("topic"))
                    .and_then(|value| value.as_str().map(str::to_owned)),
                _ => None,
            })
            .collect();
    }
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let name = line
                .trim_matches(|character| matches!(character, '[' | ']' | '"' | ','))
                .split_whitespace()
                .next()?;
            validate_topic(name).ok().map(|_| name.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_topic_names() {
        assert!(validate_topic("orders.created-v1").is_ok());
        assert!(validate_topic("../bad").is_err());
        assert!(validate_topic("has space").is_err());
    }

    #[test]
    fn parses_json_and_line_topic_lists() {
        assert_eq!(parse_topic_names("[\"alpha\",\"beta\"]"), ["alpha", "beta"]);
        assert_eq!(parse_topic_names("alpha\nbeta\n"), ["alpha", "beta"]);
    }
}
