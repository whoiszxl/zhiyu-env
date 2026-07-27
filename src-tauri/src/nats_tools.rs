use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

const NATS_ADDRESS: &str = "127.0.0.1:4222";
const MONITOR_ADDRESS: &str = "127.0.0.1:8222";
const MAX_SUBJECT_LENGTH: usize = 256;
const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;

#[derive(Deserialize)]
struct VarzResponse {
    version: String,
    connections: u64,
    subscriptions: u64,
    in_msgs: u64,
    out_msgs: u64,
    in_bytes: u64,
    out_bytes: u64,
    slow_consumers: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NatsOverview {
    version: String,
    connections: u64,
    subscriptions: u64,
    in_messages: u64,
    out_messages: u64,
    in_bytes: u64,
    out_bytes: u64,
    slow_consumers: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NatsPublishResult {
    subject: String,
    payload_bytes: usize,
    elapsed_ms: u128,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NatsMessage {
    subject: String,
    payload: String,
    payload_bytes: usize,
    elapsed_ms: u128,
}

#[tauri::command]
pub async fn nats_overview() -> Result<NatsOverview, String> {
    tauri::async_runtime::spawn_blocking(read_overview)
        .await
        .map_err(|error| format!("NATS 概览任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn nats_publish(subject: String, payload: String) -> Result<NatsPublishResult, String> {
    tauri::async_runtime::spawn_blocking(move || publish(subject, payload))
        .await
        .map_err(|error| format!("NATS 发布任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn nats_receive(subject: String) -> Result<NatsMessage, String> {
    tauri::async_runtime::spawn_blocking(move || receive_one(subject))
        .await
        .map_err(|error| format!("NATS 订阅任务异常结束: {error}"))?
}

fn read_overview() -> Result<NatsOverview, String> {
    let mut stream = connect(MONITOR_ADDRESS, Duration::from_secs(2))?;
    stream
        .write_all(b"GET /varz HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .map_err(|error| format!("无法请求 NATS 监控接口: {error}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("无法读取 NATS 监控响应: {error}"))?;
    let (_, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| "NATS 监控接口返回了无效的 HTTP 响应".to_string())?;
    let varz: VarzResponse =
        serde_json::from_str(body).map_err(|error| format!("无法解析 NATS 指标: {error}"))?;
    Ok(NatsOverview {
        version: varz.version,
        connections: varz.connections,
        subscriptions: varz.subscriptions,
        in_messages: varz.in_msgs,
        out_messages: varz.out_msgs,
        in_bytes: varz.in_bytes,
        out_bytes: varz.out_bytes,
        slow_consumers: varz.slow_consumers,
    })
}

fn publish(subject: String, payload: String) -> Result<NatsPublishResult, String> {
    validate_subject(&subject, false)?;
    validate_payload(&payload)?;
    let started = Instant::now();
    let mut connection = nats_connection(Duration::from_secs(3))?;
    let payload_bytes = payload.len();
    write!(
        connection.get_mut(),
        "PUB {subject} {payload_bytes}\r\n{payload}\r\nPING\r\n"
    )
    .map_err(|error| format!("无法向 NATS 发布消息: {error}"))?;
    wait_for_pong(&mut connection)?;
    Ok(NatsPublishResult {
        subject,
        payload_bytes,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn receive_one(subject: String) -> Result<NatsMessage, String> {
    validate_subject(&subject, true)?;
    let started = Instant::now();
    let mut connection = nats_connection(Duration::from_secs(8))?;
    write!(
        connection.get_mut(),
        "SUB {subject} 1\r\nUNSUB 1 1\r\nPING\r\n"
    )
    .map_err(|error| format!("无法创建 NATS 订阅: {error}"))?;

    let mut line = String::new();
    loop {
        line.clear();
        let read = connection
            .read_line(&mut line)
            .map_err(|error| format!("等待 NATS 消息失败: {error}"))?;
        if read == 0 {
            return Err("NATS 在返回消息前关闭了连接".into());
        }
        let header = line.trim_end_matches(['\r', '\n']);
        if header == "PING" {
            connection
                .get_mut()
                .write_all(b"PONG\r\n")
                .map_err(|error| format!("无法响应 NATS PING: {error}"))?;
            continue;
        }
        if let Some(message) = parse_message_header(header)? {
            let mut payload = vec![0; message.payload_bytes];
            connection
                .read_exact(&mut payload)
                .map_err(|error| format!("NATS 消息内容不完整: {error}"))?;
            let mut terminator = [0; 2];
            connection
                .read_exact(&mut terminator)
                .map_err(|error| format!("NATS 消息结尾不完整: {error}"))?;
            if terminator != *b"\r\n" {
                return Err("NATS 消息使用了无效的结尾".into());
            }
            return Ok(NatsMessage {
                subject: message.subject,
                payload: String::from_utf8_lossy(&payload).into_owned(),
                payload_bytes: payload.len(),
                elapsed_ms: started.elapsed().as_millis(),
            });
        }
        if header.starts_with("-ERR") {
            return Err(format!("NATS 返回错误: {header}"));
        }
    }
}

struct MessageHeader {
    subject: String,
    payload_bytes: usize,
}

fn parse_message_header(header: &str) -> Result<Option<MessageHeader>, String> {
    if !header.starts_with("MSG ") {
        return Ok(None);
    }
    let parts = header.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 4 && parts.len() != 5 {
        return Err("NATS 返回了无效的 MSG 消息头".into());
    }
    let payload_bytes = parts
        .last()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value <= MAX_PAYLOAD_BYTES)
        .ok_or_else(|| "NATS 消息长度无效或超过 1 MiB".to_string())?;
    Ok(Some(MessageHeader {
        subject: parts[1].into(),
        payload_bytes,
    }))
}

fn nats_connection(timeout: Duration) -> Result<BufReader<TcpStream>, String> {
    let mut stream = connect(NATS_ADDRESS, timeout)?;
    let mut connection = BufReader::new(
        stream
            .try_clone()
            .map_err(|error| format!("无法复制 NATS 连接: {error}"))?,
    );
    let mut info = String::new();
    connection
        .read_line(&mut info)
        .map_err(|error| format!("无法读取 NATS INFO: {error}"))?;
    if !info.starts_with("INFO ") {
        return Err("NATS 没有返回有效的 INFO 握手".into());
    }
    stream
        .write_all(b"CONNECT {\"verbose\":false,\"pedantic\":true,\"name\":\"zhiyu-env\"}\r\n")
        .map_err(|error| format!("无法完成 NATS CONNECT: {error}"))?;
    Ok(BufReader::new(stream))
}

fn wait_for_pong(connection: &mut BufReader<TcpStream>) -> Result<(), String> {
    let mut line = String::new();
    loop {
        line.clear();
        if connection
            .read_line(&mut line)
            .map_err(|error| format!("等待 NATS 确认失败: {error}"))?
            == 0
        {
            return Err("NATS 在确认消息前关闭了连接".into());
        }
        match line.trim_end_matches(['\r', '\n']) {
            "PONG" => return Ok(()),
            "PING" => connection
                .get_mut()
                .write_all(b"PONG\r\n")
                .map_err(|error| format!("无法响应 NATS PING: {error}"))?,
            line if line.starts_with("-ERR") => return Err(format!("NATS 返回错误: {line}")),
            _ => {}
        }
    }
}

fn connect(address: &str, timeout: Duration) -> Result<TcpStream, String> {
    let socket = address
        .to_socket_addrs()
        .map_err(|error| format!("NATS 地址无效: {error}"))?
        .next()
        .ok_or_else(|| "NATS 地址无法解析".to_string())?;
    let stream = TcpStream::connect_timeout(&socket, Duration::from_secs(2))
        .map_err(|error| format!("无法连接 NATS，请确认服务已启动: {error}"))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|error| format!("无法设置 NATS 读取超时: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("无法设置 NATS 写入超时: {error}"))?;
    Ok(stream)
}

fn validate_subject(subject: &str, wildcard_allowed: bool) -> Result<(), String> {
    if subject.is_empty()
        || subject.len() > MAX_SUBJECT_LENGTH
        || !subject.is_ascii()
        || subject.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return Err("NATS Subject 不能为空、不能包含空白且不能超过 256 字节".into());
    }
    if !wildcard_allowed && (subject.contains('*') || subject.contains('>')) {
        return Err("发布消息时 Subject 不能包含通配符".into());
    }
    if subject.starts_with('.') || subject.ends_with('.') || subject.contains("..") {
        return Err("NATS Subject 的分段格式无效".into());
    }
    Ok(())
}

fn validate_payload(payload: &str) -> Result<(), String> {
    if payload.len() > MAX_PAYLOAD_BYTES {
        Err("消息内容不能超过 1 MiB".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_publish_and_subscription_subjects() {
        assert!(validate_subject("orders.created", false).is_ok());
        assert!(validate_subject("orders.*", true).is_ok());
        assert!(validate_subject("orders.*", false).is_err());
        assert!(validate_subject("orders created", true).is_err());
        assert!(validate_subject(".orders", true).is_err());
    }

    #[test]
    fn parses_message_headers() {
        let simple = parse_message_header("MSG orders.created 1 5")
            .unwrap()
            .unwrap();
        assert_eq!(simple.subject, "orders.created");
        assert_eq!(simple.payload_bytes, 5);
        assert!(parse_message_header("PONG").unwrap().is_none());
        assert!(parse_message_header("MSG broken").is_err());
    }
}
