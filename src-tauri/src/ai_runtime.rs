use crate::ai_settings::{endpoint, load_ai_runtime};
use reqwest::blocking::Response;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Debug, Clone)]
pub struct ModelMessage {
    pub role: String,
    pub content: String,
}

pub struct StreamCompletion {
    pub answer: String,
    pub cancelled: bool,
    pub model: String,
}

fn anthropic_message_values(messages: &[ModelMessage]) -> (String, Vec<Value>) {
    let system = messages
        .iter()
        .filter(|message| message.role == "system")
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let values = messages
        .iter()
        .filter(|message| message.role != "system")
        .map(|message| json!({"role": message.role, "content": message.content}))
        .collect();
    (system, values)
}

fn safe_response_error(response: Response, api_key: &str) -> String {
    let status = response.status();
    let body = response
        .text()
        .unwrap_or_default()
        .replace(api_key, "[REDACTED]");
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    format!(
        "模型 API 返回 HTTP {}: {}",
        status.as_u16(),
        compact.chars().take(500).collect::<String>()
    )
}

fn stream_text<'a>(protocol: &str, payload: &'a Value) -> Option<&'a str> {
    if protocol == "anthropic" {
        payload
            .get("delta")
            .and_then(|delta| delta.get("text"))
            .and_then(Value::as_str)
    } else {
        payload
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("delta"))
            .and_then(|delta| delta.get("content"))
            .and_then(Value::as_str)
    }
}

pub fn stream_completion(
    messages: &[ModelMessage],
    cancel: &AtomicBool,
    mut on_delta: impl FnMut(&str),
) -> Result<StreamCompletion, String> {
    let (settings, api_key) = load_ai_runtime()?;
    let url = endpoint(&settings.base_url, &settings.protocol);
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));
    let body = if settings.protocol == "anthropic" {
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&api_key).map_err(|_| "API Key 包含无效字符".to_string())?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        let (system, message_values) = anthropic_message_values(messages);
        let mut value = json!({
            "model": settings.model,
            "max_tokens": settings.max_output_tokens,
            "stream": true,
            "messages": message_values
        });
        if !system.is_empty() {
            value["system"] = json!(system);
        }
        value
    } else {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {api_key}"))
                .map_err(|_| "API Key 包含无效字符".to_string())?,
        );
        let message_values = messages
            .iter()
            .map(|message| json!({"role": message.role, "content": message.content}))
            .collect::<Vec<_>>();
        let token_key = if settings.base_url.contains("api.openai.com") {
            "max_completion_tokens"
        } else {
            "max_tokens"
        };
        let mut value = json!({
            "model": settings.model,
            "stream": true,
            "messages": message_values
        });
        value[token_key] = json!(settings.max_output_tokens);
        value
    };
    let client = crate::settings::reqwest_client_builder(crate::settings::ProxyScope::Network)?
        .connect_timeout(Duration::from_secs(settings.timeout_seconds.min(30)))
        .timeout(Duration::from_secs(settings.timeout_seconds))
        .build()
        .map_err(|error| format!("无法创建 AI HTTP 客户端: {error}"))?;
    let response = client
        .post(url)
        .headers(headers)
        .body(serde_json::to_vec(&body).map_err(|error| error.to_string())?)
        .send()
        .map_err(|error| {
            if error.is_timeout() {
                "AI 请求超时，请调整超时时间或检查模型服务".to_string()
            } else {
                format!("无法连接模型 API: {error}")
            }
        })?;
    if !response.status().is_success() {
        return Err(safe_response_error(response, &api_key));
    }

    let mut reader = BufReader::new(response);
    let mut line = String::new();
    let mut answer = String::new();
    while reader
        .read_line(&mut line)
        .map_err(|error| format!("读取 AI 流式响应失败: {error}"))?
        > 0
    {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let trimmed = line.trim();
        if let Some(data) = trimmed.strip_prefix("data:") {
            let data = data.trim();
            if data == "[DONE]" {
                break;
            }
            if !data.is_empty() {
                let payload: Value = serde_json::from_str(data)
                    .map_err(|error| format!("模型返回了无效的流式数据: {error}"))?;
                if let Some(message) = payload
                    .get("error")
                    .and_then(|error| error.get("message"))
                    .and_then(Value::as_str)
                {
                    return Err(format!("模型流式响应失败: {message}"));
                }
                if let Some(delta) =
                    stream_text(&settings.protocol, &payload).filter(|value| !value.is_empty())
                {
                    answer.push_str(delta);
                    on_delta(delta);
                }
            }
        }
        line.clear();
    }
    let cancelled = cancel.load(Ordering::Relaxed);
    if answer.is_empty() && !cancelled {
        return Err("模型没有返回可显示的文本内容".into());
    }
    Ok(StreamCompletion {
        answer,
        cancelled,
        model: settings.model,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_and_anthropic_stream_deltas() {
        let openai = json!({"choices": [{"delta": {"content": "你好"}}]});
        let anthropic = json!({"type": "content_block_delta", "delta": {
            "type": "text_delta", "text": "智屿"
        }});
        assert_eq!(stream_text("openai", &openai), Some("你好"));
        assert_eq!(stream_text("anthropic", &anthropic), Some("智屿"));
    }

    #[test]
    fn anthropic_system_prompt_is_not_sent_as_a_message() {
        let messages = vec![
            ModelMessage {
                role: "system".into(),
                content: "Safety rules".into(),
            },
            ModelMessage {
                role: "user".into(),
                content: "Hello".into(),
            },
        ];
        let (system, values) = anthropic_message_values(&messages);
        assert_eq!(system, "Safety rules");
        assert_eq!(values.len(), 1);
        assert_eq!(values[0]["role"], "user");
    }
}
