use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub locale: String,
    pub theme_mode: String,
    pub color_theme: String,
    pub background_pattern: String,
    pub ui_scale: u8,
    pub background_image_path: String,
    pub background_style: String,
    pub background_position: String,
    pub background_overlay: u8,
    pub hidden_services: Vec<String>,
    pub service_order: Vec<String>,
    pub hidden_tools: Vec<String>,
    pub tool_order: Vec<String>,
    pub launch_at_login: bool,
    pub keep_services_running_on_close: bool,
    pub download_mirror: String,
    pub public_github_mirror: bool,
    pub download_concurrency: u8,
    pub download_timeout_seconds: u64,
    pub install_root: String,
    pub log_retention_days: u32,
    pub backup_retention_count: u32,
    pub auto_check_updates: bool,
    pub onboarding_completed: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: "zh-CN".into(),
            theme_mode: "system".into(),
            color_theme: "classic".into(),
            background_pattern: "auto".into(),
            ui_scale: 100,
            background_image_path: String::new(),
            background_style: "off".into(),
            background_position: "center".into(),
            background_overlay: 58,
            hidden_services: Vec::new(),
            service_order: Vec::new(),
            hidden_tools: Vec::new(),
            tool_order: Vec::new(),
            launch_at_login: false,
            keep_services_running_on_close: true,
            download_mirror: String::new(),
            public_github_mirror: true,
            download_concurrency: 2,
            download_timeout_seconds: 180,
            install_root: default_devbox_root().display().to_string(),
            log_retention_days: 14,
            backup_retention_count: 10,
            auto_check_updates: true,
            onboarding_completed: false,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    current_version: String,
    latest_version: Option<String>,
    update_available: bool,
    release_url: Option<String>,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallerSettings<'a> {
    download_mirror: Option<&'a str>,
    public_github_mirror: bool,
    download_concurrency: u8,
    download_timeout_seconds: u64,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
}

pub(crate) fn default_devbox_root() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".devbox")
}

pub(crate) fn devbox_root() -> Result<PathBuf, String> {
    let settings = load_settings();
    let root = PathBuf::from(&settings.install_root);
    validate_install_root(&root)?;
    Ok(root)
}

pub(crate) fn load_settings() -> AppSettings {
    let Some(path) = settings_path() else {
        return AppSettings::default();
    };
    fs::read(&path)
        .ok()
        .and_then(|contents| serde_json::from_slice(&contents).ok())
        .unwrap_or_default()
}

pub(crate) fn toggle_launch_at_login(app: &AppHandle) -> Result<bool, String> {
    let mut settings = load_settings();
    let enabled = app
        .autolaunch()
        .is_enabled()
        .unwrap_or(settings.launch_at_login);
    let next = !enabled;
    if next {
        app.autolaunch()
            .enable()
            .map_err(|error| format!("无法启用开机启动: {error}"))?;
    } else {
        app.autolaunch()
            .disable()
            .map_err(|error| format!("无法关闭开机启动: {error}"))?;
    }
    settings.launch_at_login = next;
    persist(&settings)?;
    Ok(next)
}

fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|directory| directory.join("zhiyu-env/settings.json"))
}

fn validate(settings: &mut AppSettings) -> Result<(), String> {
    if !matches!(settings.locale.as_str(), "system" | "zh-CN" | "en-US") {
        return Err("界面语言不受支持".into());
    }
    validate_theme_mode(&settings.theme_mode)?;
    validate_color_theme(&settings.color_theme)?;
    if !matches!(
        settings.background_pattern.as_str(),
        "auto"
            | "none"
            | "grid"
            | "dots"
            | "diagonal"
            | "crosshatch"
            | "circuit"
            | "rings"
            | "paper"
            | "checker"
    ) {
        return Err("背景纹理不受支持".into());
    }
    if !matches!(settings.ui_scale, 90 | 100 | 110 | 120) {
        return Err("界面字号只支持 90%、100%、110% 或 120%".into());
    }
    if !matches!(
        settings.background_style.as_str(),
        "off" | "original" | "frosted" | "blur" | "mist"
    ) {
        return Err("背景显示风格不受支持".into());
    }
    if !matches!(
        settings.background_position.as_str(),
        "center" | "top" | "bottom"
    ) {
        return Err("背景位置不受支持".into());
    }
    if !(20..=90).contains(&settings.background_overlay) {
        return Err("背景遮罩强度必须在 20% 到 90% 之间".into());
    }
    normalize_ids(
        &mut settings.hidden_services,
        &[
            "redis",
            "mysql",
            "postgres",
            "mongodb",
            "mailpit",
            "nats",
            "kafka",
            "meilisearch",
            "minio",
            "rustfs",
            "etcd",
            "consul",
            "rnacos",
            "rabbitmq",
            "nginx",
            "caddy",
        ],
    );
    normalize_ids(
        &mut settings.service_order,
        &[
            "redis",
            "mysql",
            "postgres",
            "mongodb",
            "mailpit",
            "nats",
            "kafka",
            "meilisearch",
            "minio",
            "rustfs",
            "etcd",
            "consul",
            "rnacos",
            "rabbitmq",
            "nginx",
            "caddy",
        ],
    );
    normalize_ids(
        &mut settings.hidden_tools,
        &[
            "ports",
            "mockapi",
            "http",
            "realtime",
            "time",
            "regex",
            "cron",
            "qrcode",
            "ssh",
            "duckdb",
            "sqlite",
            "dataformat",
            "jwt",
            "clipboard",
            "s3",
        ],
    );
    normalize_ids(
        &mut settings.tool_order,
        &[
            "ports",
            "mockapi",
            "http",
            "realtime",
            "time",
            "regex",
            "cron",
            "qrcode",
            "ssh",
            "duckdb",
            "sqlite",
            "dataformat",
            "jwt",
            "clipboard",
            "s3",
        ],
    );
    settings.download_mirror = settings
        .download_mirror
        .trim()
        .trim_end_matches('/')
        .to_string();
    if !settings.download_mirror.is_empty() && !settings.download_mirror.starts_with("https://") {
        return Err("下载镜像必须使用 HTTPS 地址".into());
    }
    if !(1..=4).contains(&settings.download_concurrency) {
        return Err("下载并发数必须在 1 到 4 之间".into());
    }
    if !(15..=600).contains(&settings.download_timeout_seconds) {
        return Err("下载超时时间必须在 15 到 600 秒之间".into());
    }
    if !(1..=365).contains(&settings.log_retention_days) {
        return Err("日志保留天数必须在 1 到 365 天之间".into());
    }
    if !(1..=100).contains(&settings.backup_retention_count) {
        return Err("备份保留数量必须在 1 到 100 之间".into());
    }
    validate_install_root(Path::new(&settings.install_root))
}

fn normalize_ids(values: &mut Vec<String>, supported: &[&str]) {
    let mut seen = HashSet::new();
    values.retain(|value| supported.contains(&value.as_str()) && seen.insert(value.clone()));
}

fn validate_theme_mode(theme_mode: &str) -> Result<(), String> {
    if matches!(theme_mode, "system" | "light" | "dark") {
        Ok(())
    } else {
        Err("主题模式只支持 system、light 或 dark".into())
    }
}

fn validate_color_theme(color_theme: &str) -> Result<(), String> {
    if matches!(
        color_theme,
        "classic"
            | "ocean"
            | "forest"
            | "sand"
            | "twilight"
            | "aurora"
            | "graphite"
            | "coral"
            | "sunset"
            | "neon"
            | "nord"
            | "sakura"
            | "coffee"
            | "solarized"
            | "lavender"
    ) {
        Ok(())
    } else {
        Err("配色主题不受支持".into())
    }
}

fn validate_install_root(root: &Path) -> Result<(), String> {
    let home = dirs::home_dir().ok_or_else(|| "无法确定当前用户目录".to_string())?;
    if !root.is_absolute() || root == home || !root.starts_with(&home) {
        return Err("安装目录必须是用户目录下的独立绝对路径".into());
    }
    Ok(())
}

fn persist(settings: &AppSettings) -> Result<(), String> {
    persist_settings_document(settings)?;

    let root = PathBuf::from(&settings.install_root);
    fs::create_dir_all(&root).map_err(|error| error.to_string())?;
    let installer = InstallerSettings {
        download_mirror: (!settings.download_mirror.is_empty())
            .then_some(settings.download_mirror.as_str()),
        public_github_mirror: settings.public_github_mirror,
        download_concurrency: settings.download_concurrency,
        download_timeout_seconds: settings.download_timeout_seconds,
    };
    fs::write(
        root.join("installer-settings.json"),
        serde_json::to_vec_pretty(&installer).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn persist_settings_document(settings: &AppSettings) -> Result<(), String> {
    let path = settings_path().ok_or_else(|| "无法确定应用配置目录".to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "设置文件路径无效".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(settings).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    fs::rename(&temporary, &path).map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn app_settings_get(app: AppHandle) -> Result<AppSettings, String> {
    let mut settings = load_settings();
    settings.launch_at_login = app
        .autolaunch()
        .is_enabled()
        .unwrap_or(settings.launch_at_login);
    Ok(settings)
}

#[tauri::command]
pub async fn app_settings_save(
    app: AppHandle,
    mut settings: AppSettings,
) -> Result<AppSettings, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate(&mut settings)?;
        if settings.launch_at_login {
            app.autolaunch()
                .enable()
                .map_err(|error| format!("无法启用开机启动: {error}"))?;
        } else {
            app.autolaunch()
                .disable()
                .map_err(|error| format!("无法关闭开机启动: {error}"))?;
        }
        persist(&settings)?;
        apply_log_retention(&settings)?;
        Ok(settings)
    })
    .await
    .map_err(|error| format!("设置保存任务异常结束: {error}"))?
}

const MAX_BACKGROUND_BYTES: u64 = 15 * 1024 * 1024;

fn background_directory(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join("appearance"))
        .map_err(|error| format!("无法确定应用配置目录: {error}"))
}

fn detect_background_extension(bytes: &[u8]) -> Option<&'static str> {
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

fn remove_managed_backgrounds(directory: &Path, except: Option<&Path>) -> Result<(), String> {
    if !directory.exists() {
        return Ok(());
    }
    for extension in ["png", "jpg", "webp", "tmp"] {
        let path = directory.join(format!("background.{extension}"));
        if except.is_some_and(|kept| kept == path) || !path.exists() {
            continue;
        }
        fs::remove_file(&path)
            .map_err(|error| format!("无法删除旧背景图 {}: {error}", path.display()))?;
    }
    Ok(())
}

#[tauri::command]
pub async fn app_background_import(app: AppHandle, source_path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source =
            fs::canonicalize(source_path).map_err(|error| format!("无法读取所选图片: {error}"))?;
        let metadata =
            fs::metadata(&source).map_err(|error| format!("无法读取图片信息: {error}"))?;
        if !metadata.is_file() {
            return Err("请选择一个图片文件".into());
        }
        if metadata.len() > MAX_BACKGROUND_BYTES {
            return Err("背景图不能超过 15 MiB".into());
        }

        let bytes = fs::read(&source).map_err(|error| format!("无法读取图片: {error}"))?;
        let extension = detect_background_extension(&bytes)
            .ok_or_else(|| "仅支持 PNG、JPEG 和 WebP 图片".to_string())?;
        let directory = background_directory(&app)?;
        fs::create_dir_all(&directory).map_err(|error| format!("无法创建背景图目录: {error}"))?;

        let temporary = directory.join("background.tmp");
        let destination = directory.join(format!("background.{extension}"));
        fs::write(&temporary, bytes).map_err(|error| format!("无法复制背景图: {error}"))?;
        if destination.exists() {
            fs::remove_file(&destination).map_err(|error| format!("无法替换背景图: {error}"))?;
        }
        fs::rename(&temporary, &destination).map_err(|error| format!("无法保存背景图: {error}"))?;
        remove_managed_backgrounds(&directory, Some(&destination))?;
        Ok(destination.display().to_string())
    })
    .await
    .map_err(|error| format!("背景图导入任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn app_background_remove(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let directory = background_directory(&app)?;
        remove_managed_backgrounds(&directory, None)
    })
    .await
    .map_err(|error| format!("背景图清理任务异常结束: {error}"))?
}

pub(crate) fn apply_log_retention(settings: &AppSettings) -> Result<u32, String> {
    let logs_root = Path::new(&settings.install_root).join("instances");
    if !logs_root.exists() {
        return Ok(0);
    }
    let cutoff = SystemTime::now()
        .checked_sub(Duration::from_secs(
            u64::from(settings.log_retention_days) * 24 * 60 * 60,
        ))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut removed = 0;
    visit_files(&logs_root, &mut |path, metadata| {
        if !path
            .components()
            .any(|component| component.as_os_str() == "logs")
        {
            return Ok(());
        }
        if metadata.modified().unwrap_or(SystemTime::now()) < cutoff {
            fs::remove_file(path).map_err(|error| error.to_string())?;
            removed += 1;
        }
        Ok(())
    })?;
    Ok(removed)
}

fn visit_files(
    directory: &Path,
    operation: &mut impl FnMut(&Path, &fs::Metadata) -> Result<(), String>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_dir() {
            visit_files(&path, operation)?;
        } else if metadata.is_file() {
            operation(&path, &metadata)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn app_update_check() -> Result<UpdateStatus, String> {
    tauri::async_runtime::spawn_blocking(check_update)
        .await
        .map_err(|error| format!("更新检查任务异常结束: {error}"))?
}

fn check_update() -> Result<UpdateStatus, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let settings = load_settings();
    if !settings.auto_check_updates {
        return Ok(UpdateStatus {
            current_version: current,
            latest_version: None,
            update_available: false,
            release_url: None,
            message: "自动检查更新已关闭".into(),
        });
    }
    let output = Command::new("/usr/bin/curl")
        .args([
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--connect-timeout",
            "3",
            "--max-time",
            "8",
            "--header",
            "User-Agent: zhiyu-env",
            "https://api.github.com/repos/whoiszxl/zhiyu-env/releases/latest",
        ])
        .output()
        .map_err(|error| format!("无法启动更新检查: {error}"))?;
    if !output.status.success() {
        return Err("暂时无法访问 GitHub Release".into());
    }
    let release: GithubRelease =
        serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let latest = release.tag_name.trim_start_matches('v').to_string();
    let available = version_parts(&latest) > version_parts(&current);
    Ok(UpdateStatus {
        current_version: current.clone(),
        latest_version: Some(latest.clone()),
        update_available: available,
        release_url: Some(release.html_url),
        message: if available {
            format!("发现新版本 {latest}")
        } else {
            format!("当前已经是最新版本 {current}")
        },
    })
}

fn version_parts(value: &str) -> Vec<u64> {
    value
        .split('.')
        .map(|part| part.parse::<u64>().unwrap_or_default())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_numbers_compare_numerically() {
        assert!(version_parts("0.10.0") > version_parts("0.9.9"));
        assert!(version_parts("1.0.0") == vec![1, 0, 0]);
    }

    #[test]
    fn default_settings_are_bounded() {
        let mut settings = AppSettings::default();
        assert!(validate(&mut settings).is_ok());
    }

    #[test]
    fn invalid_theme_mode_is_rejected() {
        let mut settings = AppSettings {
            theme_mode: "midnight".into(),
            ..AppSettings::default()
        };
        assert!(validate(&mut settings).is_err());
    }

    #[test]
    fn valid_theme_modes_are_accepted() {
        for mode in ["system", "light", "dark"] {
            assert!(validate_theme_mode(mode).is_ok());
        }
        assert!(validate_theme_mode("auto").is_err());
    }

    #[test]
    fn valid_color_themes_are_accepted() {
        for theme in [
            "classic",
            "ocean",
            "forest",
            "sand",
            "twilight",
            "aurora",
            "graphite",
            "coral",
            "sunset",
            "neon",
            "nord",
            "sakura",
            "coffee",
            "solarized",
            "lavender",
        ] {
            assert!(validate_color_theme(theme).is_ok());
        }
        assert!(validate_color_theme("rainbow").is_err());
    }

    #[test]
    fn settings_without_theme_keep_system_default() {
        let settings: AppSettings = serde_json::from_str(
            r#"{
                "launchAtLogin": false,
                "keepServicesRunningOnClose": true
            }"#,
        )
        .unwrap();
        assert_eq!(settings.theme_mode, "system");
        assert_eq!(settings.locale, "zh-CN");
        assert_eq!(settings.color_theme, "classic");
        assert_eq!(settings.background_pattern, "auto");
        assert_eq!(settings.ui_scale, 100);
        assert_eq!(settings.background_style, "off");
        assert_eq!(settings.background_position, "center");
        assert_eq!(settings.background_overlay, 58);
        assert!(settings.hidden_services.is_empty());
        assert!(settings.service_order.is_empty());
        assert!(settings.hidden_tools.is_empty());
        assert!(settings.tool_order.is_empty());
        assert!(!settings.onboarding_completed);
    }

    #[test]
    fn locale_only_accepts_supported_values() {
        for locale in ["system", "zh-CN", "en-US"] {
            let mut settings = AppSettings {
                locale: locale.into(),
                ..AppSettings::default()
            };
            assert!(validate(&mut settings).is_ok());
        }
        let mut settings = AppSettings {
            locale: "fr-FR".into(),
            ..AppSettings::default()
        };
        assert!(validate(&mut settings).is_err());
    }

    #[test]
    fn background_image_formats_are_detected_by_content() {
        assert_eq!(
            detect_background_extension(b"\x89PNG\r\n\x1a\nrest"),
            Some("png")
        );
        assert_eq!(
            detect_background_extension(b"\xff\xd8\xff\xe0rest"),
            Some("jpg")
        );
        assert_eq!(
            detect_background_extension(b"RIFF1234WEBPrest"),
            Some("webp")
        );
        assert_eq!(detect_background_extension(b"not an image"), None);
    }

    #[test]
    fn invalid_background_preferences_are_rejected() {
        let mut settings = AppSettings {
            background_pattern: "noise".into(),
            ..AppSettings::default()
        };
        assert!(validate(&mut settings).is_err());

        let mut settings = AppSettings {
            background_style: "neon".into(),
            ..AppSettings::default()
        };
        assert!(validate(&mut settings).is_err());

        let mut settings = AppSettings {
            background_overlay: 10,
            ..AppSettings::default()
        };
        assert!(validate(&mut settings).is_err());
    }

    #[test]
    fn all_background_patterns_are_accepted() {
        for pattern in [
            "auto",
            "none",
            "grid",
            "dots",
            "diagonal",
            "crosshatch",
            "circuit",
            "rings",
            "paper",
            "checker",
        ] {
            let mut settings = AppSettings {
                background_pattern: pattern.into(),
                ..AppSettings::default()
            };
            assert!(validate(&mut settings).is_ok());
        }
    }

    #[test]
    fn ui_scale_only_accepts_supported_steps() {
        for scale in [90, 100, 110, 120] {
            let mut settings = AppSettings {
                ui_scale: scale,
                ..AppSettings::default()
            };
            assert!(validate(&mut settings).is_ok());
        }
        let mut settings = AppSettings {
            ui_scale: 135,
            ..AppSettings::default()
        };
        assert!(validate(&mut settings).is_err());
    }

    #[test]
    fn sidebar_preferences_remove_unknown_and_duplicate_ids() {
        let mut settings = AppSettings {
            hidden_services: vec!["redis".into(), "unknown".into(), "redis".into()],
            service_order: vec!["mysql".into(), "mysql".into(), "postgres".into()],
            hidden_tools: vec!["ssh".into(), "removed".into()],
            tool_order: vec!["http".into(), "http".into(), "ports".into()],
            ..AppSettings::default()
        };
        validate(&mut settings).unwrap();
        assert_eq!(settings.hidden_services, ["redis"]);
        assert_eq!(settings.service_order, ["mysql", "postgres"]);
        assert_eq!(settings.hidden_tools, ["ssh"]);
        assert_eq!(settings.tool_order, ["http", "ports"]);
    }
}
