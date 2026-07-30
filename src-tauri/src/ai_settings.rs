use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_AVATAR_BYTES: u64 = 5 * 1024 * 1024;
const APP_IDENTIFIER: &str = "dev.zhiyu.env";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AiSettings {
    pub enabled: bool,
    pub protocol: String,
    pub base_url: String,
    pub model: String,
    pub timeout_seconds: u64,
    pub max_output_tokens: u32,
    pub user_avatar_path: String,
    pub assistant_avatar_path: String,
}

impl Default for AiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            protocol: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            model: String::new(),
            timeout_seconds: 60,
            max_output_tokens: 2_048,
            user_avatar_path: String::new(),
            assistant_avatar_path: String::new(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct AiCredentials {
    api_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsInput {
    pub enabled: bool,
    pub protocol: String,
    pub base_url: String,
    pub model: String,
    pub timeout_seconds: u64,
    pub max_output_tokens: u32,
    #[serde(default)]
    pub user_avatar_path: String,
    #[serde(default)]
    pub assistant_avatar_path: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub clear_api_key: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettingsView {
    pub enabled: bool,
    pub protocol: String,
    pub base_url: String,
    pub model: String,
    pub timeout_seconds: u64,
    pub max_output_tokens: u32,
    pub api_key_configured: bool,
    pub user_avatar_path: String,
    pub assistant_avatar_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConnectionTestResult {
    pub success: bool,
    pub protocol: String,
    pub model: String,
    pub latency_millis: u128,
    pub message: String,
}

fn config_directory() -> Result<PathBuf, String> {
    dirs::config_dir()
        .map(|directory| directory.join("zhiyu-env"))
        .ok_or_else(|| "无法确定应用配置目录".to_string())
}

fn settings_path() -> Result<PathBuf, String> {
    Ok(config_directory()?.join("ai.json"))
}

fn credentials_path() -> Result<PathBuf, String> {
    Ok(config_directory()?.join("ai-credentials.json"))
}

fn avatar_directory() -> Result<PathBuf, String> {
    dirs::config_dir()
        .map(|directory| directory.join(APP_IDENTIFIER).join("ai-assets"))
        .ok_or_else(|| "无法确定应用头像目录".to_string())
}

fn legacy_avatar_directory() -> Result<PathBuf, String> {
    Ok(config_directory()?.join("ai-assets"))
}

fn validate_avatar_role(role: &str) -> Result<&str, String> {
    match role {
        "user" | "assistant" => Ok(role),
        _ => Err("头像类型无效".into()),
    }
}

fn avatar_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        Some("jpg")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("webp")
    } else {
        None
    }
}

fn managed_avatar_path(role: &str, extension: &str) -> Result<PathBuf, String> {
    Ok(avatar_directory()?.join(format!("{role}-avatar.{extension}")))
}

fn legacy_avatar_path(role: &str, extension: &str) -> Result<PathBuf, String> {
    Ok(legacy_avatar_directory()?.join(format!("{role}-avatar.{extension}")))
}

fn migrate_legacy_avatar(value: &str, role: &str) -> Option<String> {
    if value.is_empty() || !normalize_avatar_path(value, role).is_empty() {
        return None;
    }
    let source = ["png", "jpg", "webp"]
        .into_iter()
        .filter_map(|extension| legacy_avatar_path(role, extension).ok())
        .find(|candidate| candidate == Path::new(value) && candidate.is_file())?;
    let bytes = fs::read(&source).ok()?;
    let extension = avatar_extension(&bytes)?;
    let destination = managed_avatar_path(role, extension).ok()?;
    fs::create_dir_all(destination.parent()?).ok()?;
    fs::write(&destination, bytes).ok()?;
    set_private_permissions(&destination).ok()?;
    let _ = fs::remove_file(source);
    Some(destination.display().to_string())
}

fn normalize_avatar_path(value: &str, role: &str) -> String {
    if value.is_empty() {
        return String::new();
    }
    let supplied = Path::new(value);
    ["png", "jpg", "webp"]
        .into_iter()
        .filter_map(|extension| managed_avatar_path(role, extension).ok())
        .find(|candidate| candidate == supplied && candidate.is_file())
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}

fn remove_managed_avatars(role: &str, except: Option<&Path>) -> Result<(), String> {
    for extension in ["png", "jpg", "webp", "tmp"] {
        let path = managed_avatar_path(role, extension)?;
        if except.is_some_and(|kept| kept == path) || !path.exists() {
            continue;
        }
        fs::remove_file(&path)
            .map_err(|error| format!("无法删除旧头像 {}: {error}", path.display()))?;
    }
    Ok(())
}

fn load_settings() -> AiSettings {
    let mut settings: AiSettings = settings_path()
        .ok()
        .and_then(|path| fs::read(path).ok())
        .and_then(|contents| serde_json::from_slice(&contents).ok())
        .unwrap_or_default();
    let mut migrated = false;
    if let Some(path) = migrate_legacy_avatar(&settings.user_avatar_path, "user") {
        settings.user_avatar_path = path;
        migrated = true;
    }
    if let Some(path) = migrate_legacy_avatar(&settings.assistant_avatar_path, "assistant") {
        settings.assistant_avatar_path = path;
        migrated = true;
    }
    if migrated {
        if let Ok(path) = settings_path() {
            let _ = persist_json(&path, &settings);
        }
    }
    settings
}

fn load_credentials() -> AiCredentials {
    credentials_path()
        .ok()
        .and_then(|path| fs::read(path).ok())
        .and_then(|contents| serde_json::from_slice(&contents).ok())
        .unwrap_or_default()
}

pub(crate) fn load_ai_runtime() -> Result<(AiSettings, String), String> {
    let settings = load_settings();
    if !settings.enabled {
        return Err("请先在设置中心启用 AI 能力".into());
    }
    if settings.model.trim().is_empty() {
        return Err("请先在设置中心配置模型名称".into());
    }
    let credentials = load_credentials();
    if credentials.api_key.trim().is_empty() {
        return Err("请先在设置中心配置 API Key".into());
    }
    Ok((settings, credentials.api_key))
}

fn persist_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "配置文件路径无效".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("无法创建配置目录: {error}"))?;
    let temporary = path.with_extension("tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("无法写入配置: {error}"))?;
    set_private_permissions(&temporary)?;
    fs::rename(&temporary, path).map_err(|error| format!("无法保存配置: {error}"))?;
    set_private_permissions(path)
}

#[cfg(unix)]
fn set_private_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("无法限制配置文件权限: {error}"))
}

#[cfg(not(unix))]
fn set_private_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn normalize_settings(input: &AiSettingsInput) -> Result<AiSettings, String> {
    if !matches!(input.protocol.as_str(), "openai" | "anthropic") {
        return Err("API 格式只支持 OpenAI Compatible 或 Anthropic Compatible".into());
    }
    let base_url = input.base_url.trim().trim_end_matches('/').to_string();
    let parsed =
        reqwest::Url::parse(&base_url).map_err(|_| "请输入有效的 API Base URL".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err("API Base URL 必须是有效的 HTTP 或 HTTPS 地址".into());
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("API Base URL 不能包含查询参数或锚点".into());
    }
    let model = input.model.trim().to_string();
    if model.is_empty() {
        return Err("请输入模型名称".into());
    }
    if !(5..=600).contains(&input.timeout_seconds) {
        return Err("请求超时时间必须在 5 到 600 秒之间".into());
    }
    if !(16..=65_536).contains(&input.max_output_tokens) {
        return Err("最大输出 Token 必须在 16 到 65536 之间".into());
    }
    Ok(AiSettings {
        enabled: input.enabled,
        protocol: input.protocol.clone(),
        base_url,
        model,
        timeout_seconds: input.timeout_seconds,
        max_output_tokens: input.max_output_tokens,
        user_avatar_path: normalize_avatar_path(&input.user_avatar_path, "user"),
        assistant_avatar_path: normalize_avatar_path(&input.assistant_avatar_path, "assistant"),
    })
}

fn settings_view(settings: AiSettings, credentials: &AiCredentials) -> AiSettingsView {
    let user_avatar_path = normalize_avatar_path(&settings.user_avatar_path, "user");
    let assistant_avatar_path = normalize_avatar_path(&settings.assistant_avatar_path, "assistant");
    AiSettingsView {
        enabled: settings.enabled,
        protocol: settings.protocol,
        base_url: settings.base_url,
        model: settings.model,
        timeout_seconds: settings.timeout_seconds,
        max_output_tokens: settings.max_output_tokens,
        api_key_configured: !credentials.api_key.trim().is_empty(),
        user_avatar_path,
        assistant_avatar_path,
    }
}

fn resolve_credentials(
    input: &AiSettingsInput,
    require_api_key: bool,
) -> Result<AiCredentials, String> {
    let mut credentials = load_credentials();
    if input.clear_api_key {
        credentials.api_key.clear();
    } else if !input.api_key.trim().is_empty() {
        credentials.api_key = input.api_key.trim().to_string();
    }
    if require_api_key && credentials.api_key.is_empty() {
        return Err("请输入 API Key".into());
    }
    Ok(credentials)
}

pub(crate) fn endpoint(base_url: &str, protocol: &str) -> String {
    let suffix = if protocol == "anthropic" {
        "/messages"
    } else {
        "/chat/completions"
    };
    if base_url.ends_with(suffix) {
        base_url.to_string()
    } else {
        format!("{base_url}{suffix}")
    }
}

fn safe_error_body(body: &str, api_key: &str) -> String {
    let redacted = if api_key.is_empty() {
        body.to_string()
    } else {
        body.replace(api_key, "[REDACTED]")
    };
    let compact = redacted.split_whitespace().collect::<Vec<_>>().join(" ");
    compact.chars().take(500).collect()
}

fn test_connection(
    settings: &AiSettings,
    credentials: &AiCredentials,
) -> Result<AiConnectionTestResult, String> {
    let url = endpoint(&settings.base_url, &settings.protocol);
    let client = crate::settings::reqwest_client_builder(crate::settings::ProxyScope::Network)?
        .timeout(Duration::from_secs(settings.timeout_seconds))
        .build()
        .map_err(|error| format!("无法创建 HTTP 客户端: {error}"))?;
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let body = if settings.protocol == "anthropic" {
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&credentials.api_key)
                .map_err(|_| "API Key 包含无效字符".to_string())?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        json!({
            "model": settings.model,
            "max_tokens": 8,
            "messages": [{"role": "user", "content": "Reply with OK only."}]
        })
    } else {
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", credentials.api_key))
                .map_err(|_| "API Key 包含无效字符".to_string())?,
        );
        json!({
            "model": settings.model,
            "messages": [{"role": "user", "content": "Reply with OK only."}]
        })
    };

    let started = Instant::now();
    let response = client
        .post(url)
        .headers(headers)
        .body(serde_json::to_vec(&body).map_err(|error| error.to_string())?)
        .send()
        .map_err(|error| {
            if error.is_timeout() {
                "连接测试超时，请检查地址、网络或超时设置".to_string()
            } else {
                format!("无法连接模型 API: {error}")
            }
        })?;
    let latency_millis = started.elapsed().as_millis();
    let status = response.status();
    let response_body = response
        .text()
        .map_err(|error| format!("无法读取模型响应: {error}"))?;
    if !status.is_success() {
        return Err(format!(
            "模型 API 返回 HTTP {}: {}",
            status.as_u16(),
            safe_error_body(&response_body, &credentials.api_key)
        ));
    }
    let payload: Value =
        serde_json::from_str(&response_body).map_err(|_| "模型 API 返回了无效 JSON".to_string())?;
    let valid = if settings.protocol == "anthropic" {
        payload.get("content").and_then(Value::as_array).is_some()
    } else {
        payload.get("choices").and_then(Value::as_array).is_some()
    };
    if !valid {
        return Err("API 请求成功，但响应格式与所选兼容协议不一致".into());
    }

    Ok(AiConnectionTestResult {
        success: true,
        protocol: settings.protocol.clone(),
        model: settings.model.clone(),
        latency_millis,
        message: "模型连接成功".into(),
    })
}

#[tauri::command]
pub fn ai_settings_get() -> AiSettingsView {
    settings_view(load_settings(), &load_credentials())
}

#[tauri::command]
pub async fn ai_settings_save(input: AiSettingsInput) -> Result<AiSettingsView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let settings = normalize_settings(&input)?;
        let credentials = resolve_credentials(&input, settings.enabled)?;
        persist_json(&settings_path()?, &settings)?;
        persist_json(&credentials_path()?, &credentials)?;
        Ok(settings_view(settings, &credentials))
    })
    .await
    .map_err(|error| format!("AI 设置保存任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn ai_connection_test(input: AiSettingsInput) -> Result<AiConnectionTestResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let settings = normalize_settings(&input)?;
        let credentials = resolve_credentials(&input, true)?;
        test_connection(&settings, &credentials)
    })
    .await
    .map_err(|error| format!("连接测试任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn ai_avatar_import(role: String, source_path: String) -> Result<AiSettingsView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let role = validate_avatar_role(&role)?;
        let source =
            fs::canonicalize(source_path).map_err(|error| format!("无法读取所选头像: {error}"))?;
        let metadata =
            fs::metadata(&source).map_err(|error| format!("无法读取头像信息: {error}"))?;
        if !metadata.is_file() {
            return Err("请选择一个图片文件".into());
        }
        if metadata.len() > MAX_AVATAR_BYTES {
            return Err("头像图片不能超过 5 MiB".into());
        }
        let bytes = fs::read(&source).map_err(|error| format!("无法读取头像: {error}"))?;
        let extension = avatar_extension(&bytes)
            .ok_or_else(|| "头像仅支持 PNG、JPEG 和 WebP 图片".to_string())?;
        let directory = avatar_directory()?;
        fs::create_dir_all(&directory).map_err(|error| format!("无法创建头像目录: {error}"))?;
        let temporary = managed_avatar_path(role, "tmp")?;
        let destination = managed_avatar_path(role, extension)?;
        fs::write(&temporary, bytes).map_err(|error| format!("无法复制头像: {error}"))?;
        set_private_permissions(&temporary)?;
        if destination.exists() {
            fs::remove_file(&destination).map_err(|error| format!("无法替换头像: {error}"))?;
        }
        fs::rename(&temporary, &destination).map_err(|error| format!("无法保存头像: {error}"))?;
        set_private_permissions(&destination)?;
        remove_managed_avatars(role, Some(&destination))?;

        let mut settings = load_settings();
        let stored_path = destination.display().to_string();
        if role == "user" {
            settings.user_avatar_path = stored_path;
        } else {
            settings.assistant_avatar_path = stored_path;
        }
        persist_json(&settings_path()?, &settings)?;
        Ok(settings_view(settings, &load_credentials()))
    })
    .await
    .map_err(|error| format!("头像导入任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn ai_avatar_remove(role: String) -> Result<AiSettingsView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let role = validate_avatar_role(&role)?;
        remove_managed_avatars(role, None)?;
        let mut settings = load_settings();
        if role == "user" {
            settings.user_avatar_path.clear();
        } else {
            settings.assistant_avatar_path.clear();
        }
        persist_json(&settings_path()?, &settings)?;
        Ok(settings_view(settings, &load_credentials()))
    })
    .await
    .map_err(|error| format!("头像清理任务异常结束: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(protocol: &str, base_url: &str) -> AiSettingsInput {
        AiSettingsInput {
            enabled: true,
            protocol: protocol.into(),
            base_url: base_url.into(),
            model: "test-model".into(),
            timeout_seconds: 30,
            max_output_tokens: 2_048,
            user_avatar_path: String::new(),
            assistant_avatar_path: String::new(),
            api_key: "secret".into(),
            clear_api_key: false,
        }
    }

    #[test]
    fn appends_protocol_endpoint_to_base_url() {
        assert_eq!(
            endpoint("https://api.openai.com/v1", "openai"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            endpoint("https://api.anthropic.com/v1", "anthropic"),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn accepts_full_endpoint_without_duplication() {
        assert_eq!(
            endpoint("https://example.com/v1/chat/completions", "openai"),
            "https://example.com/v1/chat/completions"
        );
    }

    #[test]
    fn rejects_invalid_protocol_and_limits() {
        assert!(normalize_settings(&input("unknown", "https://example.com/v1")).is_err());
        let mut invalid = input("openai", "https://example.com/v1");
        invalid.timeout_seconds = 1;
        assert!(normalize_settings(&invalid).is_err());
    }

    #[test]
    fn removes_api_key_from_provider_error() {
        assert_eq!(
            safe_error_body("invalid credential secret-value", "secret-value"),
            "invalid credential [REDACTED]"
        );
    }

    #[test]
    fn avatar_formats_are_detected_from_content() {
        assert_eq!(avatar_extension(b"\x89PNG\r\n\x1a\nrest"), Some("png"));
        assert_eq!(avatar_extension(b"\xff\xd8\xff\xe0rest"), Some("jpg"));
        assert_eq!(avatar_extension(b"not-an-image"), None);
    }

    #[test]
    fn rejects_unknown_avatar_roles() {
        assert!(validate_avatar_role("user").is_ok());
        assert!(validate_avatar_role("assistant").is_ok());
        assert!(validate_avatar_role("system").is_err());
    }
}
