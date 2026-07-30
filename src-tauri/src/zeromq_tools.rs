use serde::Serialize;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, timeout};
use zeromq::{
    PubSocket, PullSocket, PushSocket, Socket, SocketRecv, SocketSend, SubSocket, ZmqMessage,
};

const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ZeroMqResult {
    endpoint: String,
    pattern: &'static str,
    direction: &'static str,
    frames: Vec<String>,
    bytes: usize,
    timestamp_millis: u128,
}

fn validated_endpoint(value: &str) -> Result<String, String> {
    let endpoint = value.trim();
    if endpoint.len() > 512 || !endpoint.starts_with("tcp://") {
        return Err("ZeroMQ 地址必须是 tcp://host:port".into());
    }
    let address = endpoint
        .strip_prefix("tcp://")
        .ok_or_else(|| "ZeroMQ 地址格式无效".to_string())?;
    let (host, port) = address
        .rsplit_once(':')
        .ok_or_else(|| "ZeroMQ 地址必须包含端口".to_string())?;
    if host.trim().is_empty() || port.parse::<u16>().is_err() {
        return Err("ZeroMQ 主机或端口无效".into());
    }
    Ok(endpoint.to_string())
}

fn message_result(
    endpoint: String,
    pattern: &'static str,
    direction: &'static str,
    message: ZmqMessage,
) -> ZeroMqResult {
    let raw_frames = message.into_vec();
    let bytes = raw_frames.iter().map(|frame| frame.len()).sum();
    let frames = raw_frames
        .into_iter()
        .map(|frame| String::from_utf8_lossy(&frame).into_owned())
        .collect();
    ZeroMqResult {
        endpoint,
        pattern,
        direction,
        frames,
        bytes,
        timestamp_millis: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    }
}

async fn attach(socket: &mut impl Socket, endpoint: &str, bind: bool) -> Result<(), String> {
    if bind {
        socket.bind(endpoint).await.map(|_| ())
    } else {
        socket.connect(endpoint).await
    }
    .map_err(|error| format!("ZeroMQ 连接失败: {error}"))
}

fn payload_message(topic: Option<&str>, payload: String) -> Result<ZmqMessage, String> {
    if payload.len() > MAX_MESSAGE_BYTES {
        return Err("单条消息不能超过 1 MiB".into());
    }
    let content = match topic.map(str::trim).filter(|value| !value.is_empty()) {
        Some(topic) => format!("{topic} {payload}"),
        None => payload,
    };
    Ok(content.into())
}

#[tauri::command]
pub async fn zeromq_publish(
    endpoint: String,
    bind: bool,
    topic: String,
    payload: String,
) -> Result<ZeroMqResult, String> {
    let endpoint = validated_endpoint(&endpoint)?;
    let message = payload_message(Some(&topic), payload)?;
    let mut socket = PubSocket::new();
    attach(&mut socket, &endpoint, bind).await?;
    // PUB/SUB 需要给订阅握手留出很短的时间，避免首条消息被丢弃。
    sleep(Duration::from_millis(if bind { 350 } else { 150 })).await;
    socket
        .send(message.clone())
        .await
        .map_err(|error| format!("ZeroMQ 发布失败: {error}"))?;
    Ok(message_result(endpoint, "PUB/SUB", "sent", message))
}

#[tauri::command]
pub async fn zeromq_subscribe(
    endpoint: String,
    bind: bool,
    topic: String,
    timeout_seconds: u64,
) -> Result<ZeroMqResult, String> {
    let endpoint = validated_endpoint(&endpoint)?;
    let mut socket = SubSocket::new();
    attach(&mut socket, &endpoint, bind).await?;
    socket
        .subscribe(topic.trim())
        .await
        .map_err(|error| format!("ZeroMQ 订阅失败: {error}"))?;
    let message = timeout(
        Duration::from_secs(timeout_seconds.clamp(1, 60)),
        socket.recv(),
    )
    .await
    .map_err(|_| "等待 ZeroMQ 消息超时".to_string())?
    .map_err(|error| format!("ZeroMQ 接收失败: {error}"))?;
    Ok(message_result(endpoint, "PUB/SUB", "received", message))
}

#[tauri::command]
pub async fn zeromq_push(
    endpoint: String,
    bind: bool,
    payload: String,
) -> Result<ZeroMqResult, String> {
    let endpoint = validated_endpoint(&endpoint)?;
    let message = payload_message(None, payload)?;
    let mut socket = PushSocket::new();
    attach(&mut socket, &endpoint, bind).await?;
    socket
        .send(message.clone())
        .await
        .map_err(|error| format!("ZeroMQ 发送失败: {error}"))?;
    Ok(message_result(endpoint, "PUSH/PULL", "sent", message))
}

#[tauri::command]
pub async fn zeromq_pull(
    endpoint: String,
    bind: bool,
    timeout_seconds: u64,
) -> Result<ZeroMqResult, String> {
    let endpoint = validated_endpoint(&endpoint)?;
    let mut socket = PullSocket::new();
    attach(&mut socket, &endpoint, bind).await?;
    let message = timeout(
        Duration::from_secs(timeout_seconds.clamp(1, 60)),
        socket.recv(),
    )
    .await
    .map_err(|_| "等待 ZeroMQ 消息超时".to_string())?
    .map_err(|error| format!("ZeroMQ 接收失败: {error}"))?;
    Ok(message_result(endpoint, "PUSH/PULL", "received", message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_accepts_tcp_endpoints_with_ports() {
        assert!(validated_endpoint("tcp://127.0.0.1:5555").is_ok());
        assert!(validated_endpoint("ipc:///tmp/zhiyu").is_err());
        assert!(validated_endpoint("tcp://127.0.0.1").is_err());
        assert!(validated_endpoint("tcp://127.0.0.1:70000").is_err());
    }

    #[test]
    fn enforces_message_size_limit() {
        assert!(payload_message(None, "ok".into()).is_ok());
        assert!(payload_message(None, "x".repeat(MAX_MESSAGE_BYTES + 1)).is_err());
    }
}
