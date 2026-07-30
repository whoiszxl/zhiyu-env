use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{redirect::Policy, Method};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MAX_REQUEST_BODY: usize = 2 * 1024 * 1024;
const MAX_RESPONSE_BODY: u64 = 2 * 1024 * 1024;
const WORKSPACE_VERSION: u8 = 2;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpWorkspaceVariable {
    key: String,
    value: String,
    #[serde(default)]
    secret: bool,
    #[serde(default = "enabled_by_default")]
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpWorkspaceEnvironment {
    id: String,
    name: String,
    #[serde(default)]
    variables: Vec<HttpWorkspaceVariable>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpWorkspaceAuth {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    token: String,
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    placement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpWorkspaceRequest {
    id: String,
    name: String,
    #[serde(default)]
    folder: String,
    method: String,
    url: String,
    #[serde(default)]
    query_params: Vec<HttpHeader>,
    #[serde(default)]
    headers: Vec<HttpHeader>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    auth: HttpWorkspaceAuth,
    #[serde(default)]
    updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpWorkspaceState {
    #[serde(default = "workspace_version")]
    version: u8,
    #[serde(default)]
    active_environment_id: String,
    #[serde(default)]
    environments: Vec<HttpWorkspaceEnvironment>,
    #[serde(default)]
    requests: Vec<HttpWorkspaceRequest>,
}

impl Default for HttpWorkspaceState {
    fn default() -> Self {
        Self {
            version: WORKSPACE_VERSION,
            active_environment_id: "default".into(),
            environments: vec![HttpWorkspaceEnvironment {
                id: "default".into(),
                name: "本地开发".into(),
                variables: Vec::new(),
            }],
            requests: Vec::new(),
        }
    }
}

fn enabled_by_default() -> bool {
    true
}

fn workspace_version() -> u8 {
    WORKSPACE_VERSION
}

#[tauri::command]
pub fn http_workspace_get() -> Result<HttpWorkspaceState, String> {
    let root = crate::settings::devbox_root()?;
    let path = workspace_path(&root);
    if !path.is_file() {
        return Ok(HttpWorkspaceState::default());
    }
    let bytes = fs::read(path).map_err(|error| format!("读取 HTTP 工作区失败：{error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("解析 HTTP 工作区失败：{error}"))
}

#[tauri::command]
pub fn http_workspace_save(
    mut workspace: HttpWorkspaceState,
) -> Result<HttpWorkspaceState, String> {
    validate_workspace(&workspace)?;
    workspace.version = WORKSPACE_VERSION;
    if workspace.environments.is_empty() {
        workspace.environments = HttpWorkspaceState::default().environments;
        workspace.active_environment_id = "default".into();
    }
    if !workspace
        .environments
        .iter()
        .any(|environment| environment.id == workspace.active_environment_id)
    {
        workspace.active_environment_id = workspace.environments[0].id.clone();
    }
    let root = crate::settings::devbox_root()?;
    let bytes = serde_json::to_vec_pretty(&workspace)
        .map_err(|error| format!("序列化 HTTP 工作区失败：{error}"))?;
    atomic_write(&workspace_path(&root), &bytes)?;
    Ok(workspace)
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
    let client = crate::settings::reqwest_client_builder(crate::settings::ProxyScope::Network)?
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

fn validate_workspace(workspace: &HttpWorkspaceState) -> Result<(), String> {
    if workspace.requests.len() > 2_000 {
        return Err("HTTP 工作区最多保存 2000 个请求".into());
    }
    if workspace.environments.len() > 50 {
        return Err("HTTP 工作区最多保存 50 套环境".into());
    }
    for environment in &workspace.environments {
        if environment.id.trim().is_empty() || environment.name.trim().is_empty() {
            return Err("环境名称不能为空".into());
        }
        if environment.variables.len() > 200 {
            return Err(format!("环境“{}”的变量不能超过 200 个", environment.name));
        }
        for variable in &environment.variables {
            if variable.key.contains(['\r', '\n']) || variable.value.len() > 64 * 1024 {
                return Err("环境变量无效或内容过长".into());
            }
        }
    }
    for request in &workspace.requests {
        if request.id.trim().is_empty() || request.name.trim().is_empty() {
            return Err("请求名称不能为空".into());
        }
        if request.body.len() > MAX_REQUEST_BODY
            || request.headers.len() > 200
            || request.query_params.len() > 200
        {
            return Err(format!("请求“{}”内容过大", request.name));
        }
    }
    Ok(())
}

fn workspace_path(root: &Path) -> PathBuf {
    root.join("tools/http-workspace.json")
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "保存路径无效".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary).map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(_) if path.exists() => {
            fs::remove_file(path).map_err(|remove_error| remove_error.to_string())?;
            fs::rename(temporary, path).map_err(|error| error.to_string())
        }
        Err(error) => Err(error.to_string()),
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

    #[test]
    fn rejects_oversized_workspace() {
        let mut workspace = HttpWorkspaceState::default();
        workspace.environments[0].variables = (0..201)
            .map(|index| HttpWorkspaceVariable {
                key: format!("KEY_{index}"),
                value: String::new(),
                secret: false,
                enabled: true,
            })
            .collect();
        assert!(validate_workspace(&workspace).is_err());
    }
}
