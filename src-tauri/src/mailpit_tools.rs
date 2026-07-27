use serde::Serialize;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const MAILPIT_API: &str = "127.0.0.1:8025";
const MAX_RESPONSE_BYTES: u64 = 12 * 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailpitOverview {
    total: u64,
    unread: u64,
    smtp_address: &'static str,
    web_address: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailSummary {
    id: String,
    from: String,
    to: Vec<String>,
    subject: String,
    created: String,
    size_bytes: u64,
    read: bool,
    snippet: String,
    attachment_count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailDetail {
    id: String,
    from: String,
    to: Vec<String>,
    cc: Vec<String>,
    subject: String,
    created: String,
    text: String,
    html: String,
    headers: Vec<MailHeader>,
    attachment_count: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MailHeader {
    name: String,
    value: String,
}

#[tauri::command]
pub async fn mailpit_overview() -> Result<MailpitOverview, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let root = request_json("/api/v1/messages?limit=1")?;
        Ok(MailpitOverview {
            total: unsigned(field(&root, &["total", "Total"])),
            unread: unsigned(field(&root, &["unread", "Unread"])),
            smtp_address: "127.0.0.1:1025",
            web_address: "http://127.0.0.1:8025",
        })
    })
    .await
    .map_err(|error| format!("邮件概览任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn mailpit_messages() -> Result<Vec<MailSummary>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let root = request_json("/api/v1/messages?limit=100")?;
        let messages = field(&root, &["messages", "Messages"])
            .and_then(Value::as_array)
            .ok_or_else(|| "Mailpit 返回了无法识别的邮件列表".to_string())?;
        Ok(messages.iter().map(mail_summary).collect())
    })
    .await
    .map_err(|error| format!("邮件列表任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn mailpit_message_detail(id: String) -> Result<MailDetail, String> {
    if id.is_empty() || id.len() > 512 || id.contains('\0') {
        return Err("邮件 ID 无效".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let root = request_json(&format!("/api/v1/message/{}", percent_encode(&id)))?;
        Ok(mail_detail(&root))
    })
    .await
    .map_err(|error| format!("邮件详情任务异常结束: {error}"))?
}

fn request_json(path: &str) -> Result<Value, String> {
    let address = MAILPIT_API
        .to_socket_addrs()
        .map_err(|error| format!("无法解析 Mailpit 地址: {error}"))?
        .next()
        .ok_or_else(|| "无法解析 Mailpit 地址".to_string())?;
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|error| format!("无法连接 Mailpit，请先启动服务: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;

    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:8025\r\nAccept: application/json\r\nAccept-Encoding: identity\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("Mailpit 请求发送失败: {error}"))?;

    let mut response = Vec::new();
    stream
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut response)
        .map_err(|error| format!("Mailpit 响应读取失败: {error}"))?;
    if response.len() as u64 > MAX_RESPONSE_BYTES {
        return Err("Mailpit 响应超过 12 MiB 安全上限".into());
    }
    let body = parse_http_response(&response)?;
    serde_json::from_slice(&body).map_err(|error| format!("Mailpit JSON 解析失败: {error}"))
}

fn parse_http_response(response: &[u8]) -> Result<Vec<u8>, String> {
    let separator = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "Mailpit 返回了无效的 HTTP 响应".to_string())?;
    let headers = std::str::from_utf8(&response[..separator])
        .map_err(|_| "Mailpit HTTP 头不是有效文本".to_string())?;
    let status = headers
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| "Mailpit HTTP 状态无效".to_string())?;
    if !(200..300).contains(&status) {
        return Err(format!("Mailpit API 返回 HTTP {status}"));
    }
    let body = &response[separator + 4..];
    if headers
        .lines()
        .any(|line| line.eq_ignore_ascii_case("transfer-encoding: chunked"))
    {
        decode_chunked(body)
    } else {
        Ok(body.to_vec())
    }
}

fn decode_chunked(mut input: &[u8]) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    loop {
        let line_end = input
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| "Mailpit 分块响应不完整".to_string())?;
        let size_text = std::str::from_utf8(&input[..line_end])
            .map_err(|_| "Mailpit 分块长度无效".to_string())?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| "Mailpit 分块长度无效".to_string())?;
        input = &input[line_end + 2..];
        if size == 0 {
            return Ok(output);
        }
        if input.len() < size + 2 || &input[size..size + 2] != b"\r\n" {
            return Err("Mailpit 分块响应不完整".into());
        }
        output.extend_from_slice(&input[..size]);
        input = &input[size + 2..];
    }
}

fn mail_summary(value: &Value) -> MailSummary {
    MailSummary {
        id: text(field(value, &["ID", "id"])),
        from: address(field(value, &["From", "from"])),
        to: addresses(field(value, &["To", "to"])),
        subject: text(field(value, &["Subject", "subject"])),
        created: text(field(value, &["Created", "created", "Date", "date"])),
        size_bytes: unsigned(field(value, &["Size", "size"])),
        read: boolean(field(value, &["Read", "read"])),
        snippet: text(field(value, &["Snippet", "snippet"])),
        attachment_count: unsigned(field(
            value,
            &["Attachments", "attachments", "AttachmentCount"],
        )),
    }
}

fn mail_detail(value: &Value) -> MailDetail {
    MailDetail {
        id: text(field(value, &["ID", "id"])),
        from: address(field(value, &["From", "from"])),
        to: addresses(field(value, &["To", "to"])),
        cc: addresses(field(value, &["Cc", "CC", "cc"])),
        subject: text(field(value, &["Subject", "subject"])),
        created: text(field(value, &["Created", "created", "Date", "date"])),
        text: text(field(value, &["Text", "text"])),
        html: text(field(value, &["HTML", "Html", "html"])),
        headers: headers(field(value, &["Headers", "headers"])),
        attachment_count: unsigned(field(
            value,
            &["Attachments", "attachments", "AttachmentCount"],
        )),
    }
}

fn field<'a>(value: &'a Value, names: &[&str]) -> Option<&'a Value> {
    names.iter().find_map(|name| value.get(name))
}

fn text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(value)) => value.clone(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

fn unsigned(value: Option<&Value>) -> u64 {
    value
        .and_then(|value| value.as_u64().or_else(|| value.as_str()?.parse().ok()))
        .unwrap_or_default()
}

fn boolean(value: Option<&Value>) -> bool {
    value
        .and_then(|value| value.as_bool().or_else(|| value.as_str()?.parse().ok()))
        .unwrap_or_default()
}

fn address(value: Option<&Value>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    if let Some(value) = value.as_str() {
        return value.into();
    }
    let name = text(field(value, &["Name", "name"]));
    let email = text(field(value, &["Address", "address", "Email", "email"]));
    match (name.is_empty(), email.is_empty()) {
        (_, true) => name,
        (true, false) => email,
        (false, false) => format!("{name} <{email}>"),
    }
}

fn addresses(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(values)) => values
            .iter()
            .map(|value| address(Some(value)))
            .filter(|value| !value.is_empty())
            .collect(),
        Some(value) => {
            let value = address(Some(value));
            (!value.is_empty()).then_some(value).into_iter().collect()
        }
        None => Vec::new(),
    }
}

fn headers(value: Option<&Value>) -> Vec<MailHeader> {
    let Some(Value::Object(values)) = value else {
        return Vec::new();
    };
    values
        .iter()
        .map(|(name, value)| MailHeader {
            name: name.clone(),
            value: match value {
                Value::Array(values) => values
                    .iter()
                    .map(|value| text(Some(value)))
                    .collect::<Vec<_>>()
                    .join(", "),
                _ => text(Some(value)),
            },
        })
        .collect()
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use devbox_core::{MailpitService, ServiceConfig, ServiceKind, ServiceManager};
    use std::collections::BTreeMap;
    use std::fs;
    use std::thread;
    use std::time::Instant;

    #[test]
    fn parses_chunked_http_json() {
        let response =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n7\r\n{\"ok\":1\r\n1\r\n}\r\n0\r\n\r\n";
        assert_eq!(parse_http_response(response).unwrap(), br#"{"ok":1}"#);
    }

    #[test]
    fn maps_mailpit_message_fields() {
        let value = serde_json::json!({
            "ID": "abc",
            "From": {"Name": "智屿", "Address": "dev@zhiyu.local"},
            "To": [{"Address": "user@example.test"}],
            "Subject": "测试邮件",
            "Size": 128,
            "Read": false,
            "Attachments": 1
        });
        let message = mail_summary(&value);
        assert_eq!(message.id, "abc");
        assert_eq!(message.from, "智屿 <dev@zhiyu.local>");
        assert_eq!(message.to, ["user@example.test"]);
        assert_eq!(message.attachment_count, 1);
    }

    #[test]
    fn encodes_mail_id_as_path_segment() {
        assert_eq!(percent_encode("id@example.test"), "id%40example.test");
    }

    #[test]
    #[ignore = "requires Mailpit installed in ~/.devbox"]
    fn live_mailpit_captures_and_reads_a_message() {
        let root = std::env::temp_dir().join(format!("zhiyu-mailpit-live-{}", std::process::id()));
        let executable = dirs::home_dir()
            .unwrap()
            .join(".devbox/installations/mailpit/1.30/bin/mailpit");
        let service = MailpitService::new(ServiceConfig {
            name: "Mailpit live test".into(),
            kind: ServiceKind::Mailpit,
            version: "1.30.5".into(),
            port: 1025,
            executable,
            arguments: Vec::new(),
            environment: BTreeMap::from([
                ("MP_SMTP_BIND_ADDR".into(), "127.0.0.1:1025".into()),
                ("MP_UI_BIND_ADDR".into(), "127.0.0.1:8025".into()),
                (
                    "MP_DATABASE".into(),
                    root.join("data/mailpit.db").display().to_string(),
                ),
                ("MP_MAX_MESSAGES".into(), "10".into()),
                ("MP_DISABLE_VERSION_CHECK".into(), "true".into()),
                ("MP_BLOCK_REMOTE_CSS_AND_FONTS".into(), "true".into()),
            ]),
            instance_dir: root.clone(),
            wait_for_port: true,
        })
        .unwrap();

        service.install().unwrap();
        service.start().unwrap();
        let result = (|| -> Result<(), String> {
            let started = Instant::now();
            while request_json("/api/v1/messages?limit=1").is_err() {
                if started.elapsed() > Duration::from_secs(5) {
                    return Err("Mailpit API did not become ready".into());
                }
                thread::sleep(Duration::from_millis(100));
            }

            let mut smtp =
                TcpStream::connect("127.0.0.1:1025").map_err(|error| error.to_string())?;
            smtp.set_read_timeout(Some(Duration::from_secs(2)))
                .map_err(|error| error.to_string())?;
            let mut greeting = [0_u8; 512];
            smtp.read(&mut greeting)
                .map_err(|error| error.to_string())?;
            smtp.write_all(
                b"EHLO localhost\r\n\
                  MAIL FROM:<dev@zhiyu.local>\r\n\
                  RCPT TO:<user@example.test>\r\n\
                  DATA\r\n\
                  From: Zhiyu Dev <dev@zhiyu.local>\r\n\
                  To: Local User <user@example.test>\r\n\
                  Subject: Zhiyu live integration test\r\n\
                  Content-Type: text/plain; charset=utf-8\r\n\
                  \r\n\
                  Mailpit integration is working.\r\n\
                  .\r\n\
                  QUIT\r\n",
            )
            .map_err(|error| error.to_string())?;

            let started = Instant::now();
            loop {
                let root = request_json("/api/v1/messages?limit=10")?;
                let messages = field(&root, &["messages", "Messages"])
                    .and_then(Value::as_array)
                    .ok_or_else(|| "message list missing".to_string())?;
                if let Some(message) = messages
                    .iter()
                    .map(mail_summary)
                    .find(|message| message.subject == "Zhiyu live integration test")
                {
                    let detail =
                        request_json(&format!("/api/v1/message/{}", percent_encode(&message.id)))?;
                    if mail_detail(&detail).text.contains("integration is working") {
                        return Ok(());
                    }
                }
                if started.elapsed() > Duration::from_secs(5) {
                    return Err("captured message did not appear".into());
                }
                thread::sleep(Duration::from_millis(100));
            }
        })();

        let _ = service.stop();
        let _ = fs::remove_dir_all(&root);
        result.unwrap();
    }
}
