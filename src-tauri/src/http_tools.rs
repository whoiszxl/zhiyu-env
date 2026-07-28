use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{redirect::Policy, Method};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::time::{Duration, Instant};

const MAX_REQUEST_BODY: usize = 2 * 1024 * 1024;
const MAX_RESPONSE_BODY: u64 = 2 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequestInput {
    method: String,
    url: String,
    headers: Vec<HttpHeader>,
    body: String,
    timeout_seconds: u64,
    follow_redirects: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpHeader {
    name: String,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponseOutput {
    status_code: u16,
    status_text: String,
    headers: Vec<HttpHeader>,
    body: String,
    content_type: String,
    elapsed_ms: u128,
    size_bytes: u64,
    truncated: bool,
    effective_url: String,
}

#[tauri::command]
pub async fn http_request_execute(request: HttpRequestInput) -> Result<HttpResponseOutput, String> {
    tauri::async_runtime::spawn_blocking(move || execute(request))
        .await
        .map_err(|error| format!("HTTP 请求任务异常：{error}"))?
}

fn execute(request: HttpRequestInput) -> Result<HttpResponseOutput, String> {
    validate_request(&request)?;
    let method = Method::from_bytes(request.method.trim().to_ascii_uppercase().as_bytes())
        .map_err(|_| "请求方法无效".to_string())?;
    let mut headers = HeaderMap::new();
    for header in request.headers {
        if header.name.trim().is_empty() {
            continue;
        }
        let name = HeaderName::from_bytes(header.name.trim().as_bytes())
            .map_err(|_| format!("请求头名称无效：{}", header.name))?;
        let value = HeaderValue::from_str(&header.value)
            .map_err(|_| format!("请求头值无效：{}", header.name))?;
        headers.append(name, value);
    }

    let redirect = if request.follow_redirects {
        Policy::limited(5)
    } else {
        Policy::none()
    };
    let client = Client::builder()
        .timeout(Duration::from_secs(request.timeout_seconds))
        .redirect(redirect)
        .build()
        .map_err(|error| format!("无法创建 HTTP 客户端：{error}"))?;

    let started = Instant::now();
    let mut builder = client.request(method.clone(), request.url).headers(headers);
    if !request.body.is_empty() && method != Method::GET && method != Method::HEAD {
        builder = builder.body(request.body);
    }
    let mut response = builder
        .send()
        .map_err(|error| format_request_error(&error))?;
    let elapsed_ms = started.elapsed().as_millis();
    let status = response.status();
    let effective_url = response.url().to_string();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let response_headers = response
        .headers()
        .iter()
        .map(|(name, value)| HttpHeader {
            name: name.to_string(),
            value: value.to_str().unwrap_or("<二进制值>").to_string(),
        })
        .collect();
    let announced_size = response.content_length();
    let mut bytes = Vec::new();
    response
        .by_ref()
        .take(MAX_RESPONSE_BODY + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("读取响应内容失败：{error}"))?;
    let truncated = bytes.len() as u64 > MAX_RESPONSE_BODY;
    if truncated {
        bytes.truncate(MAX_RESPONSE_BODY as usize);
    }
    let size_bytes = announced_size.unwrap_or(bytes.len() as u64);
    let body = String::from_utf8_lossy(&bytes).into_owned();

    Ok(HttpResponseOutput {
        status_code: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        headers: response_headers,
        body,
        content_type,
        elapsed_ms,
        size_bytes,
        truncated,
        effective_url,
    })
}

fn validate_request(request: &HttpRequestInput) -> Result<(), String> {
    if !request.url.starts_with("http://") && !request.url.starts_with("https://") {
        return Err("请求地址必须以 http:// 或 https:// 开头".into());
    }
    if request.url.len() > 8192 {
        return Err("请求地址过长".into());
    }
    if !matches!(
        request.method.trim().to_ascii_uppercase().as_str(),
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
    ) {
        return Err("仅支持常用 HTTP 请求方法".into());
    }
    if request.body.len() > MAX_REQUEST_BODY {
        return Err("请求体不能超过 2 MiB".into());
    }
    if !(1..=120).contains(&request.timeout_seconds) {
        return Err("超时时间必须在 1 到 120 秒之间".into());
    }
    if request
        .headers
        .iter()
        .any(|header| header.name.contains(['\r', '\n']) || header.value.contains(['\r', '\n']))
    {
        return Err("请求头不能包含换行符".into());
    }
    Ok(())
}

fn format_request_error(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "请求超时，请检查地址或调大超时时间".into()
    } else if error.is_connect() {
        format!("连接失败，请确认服务已启动且地址、端口正确：{error}")
    } else {
        format!("HTTP 请求失败：{error}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(url: &str) -> HttpRequestInput {
        HttpRequestInput {
            method: "GET".into(),
            url: url.into(),
            headers: vec![],
            body: String::new(),
            timeout_seconds: 10,
            follow_redirects: true,
        }
    }

    #[test]
    fn rejects_non_http_urls() {
        assert!(validate_request(&request("file:///etc/hosts")).is_err());
    }

    #[test]
    fn rejects_header_newlines() {
        let mut input = request("http://127.0.0.1:9321");
        input.headers.push(HttpHeader {
            name: "X-Test".into(),
            value: "yes\r\nInjected: true".into(),
        });
        assert!(validate_request(&input).is_err());
    }
}
