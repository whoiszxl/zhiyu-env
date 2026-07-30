use devbox_core::config::ServiceKind;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Write;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STATE_VERSION: u8 = 2;
const MAX_ROUTES: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalDomainRoute {
    id: String,
    name: String,
    hostname: String,
    target: String,
    #[serde(default = "default_route_path")]
    path: String,
    https: bool,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalDomainsState {
    #[serde(default = "state_version")]
    version: u8,
    #[serde(default = "default_http_port")]
    http_port: u16,
    #[serde(default = "default_https_port")]
    https_port: u16,
    #[serde(default)]
    routes: Vec<LocalDomainRoute>,
    #[serde(default)]
    last_backup_path: String,
    #[serde(default)]
    last_applied_at_millis: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalDomainCheck {
    reachable: bool,
    latency_millis: u128,
    message: String,
}

fn state_version() -> u8 {
    STATE_VERSION
}

fn default_http_port() -> u16 {
    8082
}

fn default_https_port() -> u16 {
    8443
}

fn default_route_path() -> String {
    "/".into()
}

impl Default for LocalDomainsState {
    fn default() -> Self {
        Self {
            version: STATE_VERSION,
            http_port: default_http_port(),
            https_port: default_https_port(),
            routes: Vec::new(),
            last_backup_path: String::new(),
            last_applied_at_millis: 0,
        }
    }
}

#[tauri::command]
pub fn local_domains_get() -> Result<LocalDomainsState, String> {
    let root = crate::settings::devbox_root()?;
    load_state(&root)
}

#[tauri::command]
pub fn local_domains_save(mut state: LocalDomainsState) -> Result<LocalDomainsState, String> {
    validate_state(&state)?;
    state.version = STATE_VERSION;
    let root = crate::settings::devbox_root()?;
    save_state(&root, &state)?;
    Ok(state)
}

#[tauri::command]
pub async fn local_domains_apply(
    mut state: LocalDomainsState,
) -> Result<LocalDomainsState, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_state(&state)?;
        let root = crate::settings::devbox_root()?;
        let config = crate::commands::service_config(ServiceKind::Caddy)?;
        if !config.executable.is_file() {
            return Err("请先安装 Caddy，再应用本地域名配置".into());
        }

        let path = crate::commands::native_config_path(&config);
        let parent = path
            .parent()
            .ok_or_else(|| "Caddy 配置文件路径无效".to_string())?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;

        let backup_path = if path.is_file() {
            let backup_dir = root.join("backups/caddy/local-domains");
            fs::create_dir_all(&backup_dir).map_err(|error| error.to_string())?;
            let backup = backup_dir.join(format!("Caddyfile-{}.bak", now_millis()));
            fs::copy(&path, &backup).map_err(|error| format!("备份 Caddy 配置失败：{error}"))?;
            prune_backups(&backup_dir, 10);
            backup
        } else {
            PathBuf::new()
        };

        let content = build_caddyfile(&state, &config.logs_dir())?;
        let temporary = path.with_extension("local-domains.tmp");
        let mut candidate =
            fs::File::create(&temporary).map_err(|error| format!("创建临时配置失败：{error}"))?;
        candidate
            .write_all(content.as_bytes())
            .map_err(|error| format!("写入临时配置失败：{error}"))?;
        candidate.sync_all().map_err(|error| error.to_string())?;
        let output = std::process::Command::new(&config.executable)
            .args(["validate", "--config"])
            .arg(&temporary)
            .args(["--adapter", "caddyfile"])
            .output()
            .map_err(|error| format!("无法运行 Caddy 配置校验：{error}"))?;
        if !output.status.success() {
            let _ = fs::remove_file(&temporary);
            let detail = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Caddy 配置校验失败：{}", trim_detail(&detail)));
        }

        replace_file(&temporary, &path).map_err(|error| format!("写入 Caddy 配置失败：{error}"))?;
        state.version = STATE_VERSION;
        state.last_applied_at_millis = now_millis();
        if !backup_path.as_os_str().is_empty() {
            state.last_backup_path = backup_path.display().to_string();
        }
        save_state(&root, &state)?;
        Ok(state)
    })
    .await
    .map_err(|error| format!("本地域名任务异常：{error}"))?
}

#[tauri::command]
pub fn local_domains_restore() -> Result<LocalDomainsState, String> {
    let root = crate::settings::devbox_root()?;
    let mut state = load_state(&root)?;
    if state.last_backup_path.trim().is_empty() {
        return Err("没有可恢复的 Caddy 配置备份".into());
    }
    let backup = PathBuf::from(&state.last_backup_path);
    let backup_root = root.join("backups/caddy/local-domains");
    let canonical = backup
        .canonicalize()
        .map_err(|_| "Caddy 配置备份不存在".to_string())?;
    let canonical_root = backup_root
        .canonicalize()
        .map_err(|_| "Caddy 备份目录不存在".to_string())?;
    if !canonical.starts_with(canonical_root) {
        return Err("拒绝恢复智屿目录之外的配置".into());
    }
    let config = crate::commands::service_config(ServiceKind::Caddy)?;
    let target = crate::commands::native_config_path(&config);
    let bytes = fs::read(canonical).map_err(|error| format!("读取备份失败：{error}"))?;
    atomic_write(&target, &bytes)?;
    state.last_backup_path.clear();
    save_state(&root, &state)?;
    Ok(state)
}

#[tauri::command]
pub async fn local_domain_target_check(target: String) -> Result<LocalDomainCheck, String> {
    tauri::async_runtime::spawn_blocking(move || check_target(&target))
        .await
        .map_err(|error| format!("端口检查任务异常：{error}"))?
}

fn check_target(target: &str) -> Result<LocalDomainCheck, String> {
    validate_target(target)?;
    let address: SocketAddr = target
        .to_socket_addrs()
        .map_err(|error| format!("无法解析目标地址：{error}"))?
        .next()
        .ok_or_else(|| "无法解析目标地址".to_string())?;
    let started = std::time::Instant::now();
    match TcpStream::connect_timeout(&address, Duration::from_millis(800)) {
        Ok(_) => Ok(LocalDomainCheck {
            reachable: true,
            latency_millis: started.elapsed().as_millis(),
            message: "目标端口可连接".into(),
        }),
        Err(error) => Ok(LocalDomainCheck {
            reachable: false,
            latency_millis: started.elapsed().as_millis(),
            message: format!("目标端口不可连接：{error}"),
        }),
    }
}

fn validate_state(state: &LocalDomainsState) -> Result<(), String> {
    if state.routes.len() > MAX_ROUTES {
        return Err(format!("本地域名最多支持 {MAX_ROUTES} 条路由"));
    }
    if state.http_port == state.https_port {
        return Err("HTTP 与 HTTPS 入口端口不能相同".into());
    }
    let mut unique = HashSet::new();
    for route in &state.routes {
        validate_hostname(&route.hostname)?;
        validate_target(&route.target)?;
        validate_path(&route.path)?;
        let key = format!(
            "{}|{}|{}",
            route.hostname.to_ascii_lowercase(),
            route.https,
            normalized_path(&route.path)
        );
        if !unique.insert(key) {
            return Err(format!(
                "路由重复：{} {}",
                route.hostname,
                normalized_path(&route.path)
            ));
        }
    }
    Ok(())
}

fn validate_hostname(hostname: &str) -> Result<(), String> {
    let hostname = hostname.trim().to_ascii_lowercase();
    if hostname.len() > 253
        || !hostname.ends_with(".localhost")
        || hostname.starts_with('.')
        || hostname.contains("..")
    {
        return Err(format!("本地域名无效：{hostname}"));
    }
    if hostname.split('.').any(|part| {
        part.is_empty()
            || part.len() > 63
            || part.starts_with('-')
            || part.ends_with('-')
            || !part
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
    }) {
        return Err(format!("本地域名无效：{hostname}"));
    }
    Ok(())
}

fn validate_target(target: &str) -> Result<(), String> {
    let target = target.trim();
    let (host, port) = if let Some(port) = target.strip_prefix("[::1]:") {
        ("::1", port)
    } else {
        target
            .rsplit_once(':')
            .ok_or_else(|| "目标格式应为 127.0.0.1:端口".to_string())?
    };
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return Err("目标只能使用本机地址 127.0.0.1、localhost 或 ::1".into());
    }
    port.parse::<u16>()
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| "目标端口无效".to_string())?;
    Ok(())
}

fn validate_path(path: &str) -> Result<(), String> {
    let path = path.trim();
    if !path.starts_with('/')
        || path.len() > 256
        || path.contains(['{', '}', '\r', '\n', '\t', ' '])
    {
        return Err(format!("路由路径无效：{path}"));
    }
    Ok(())
}

fn normalized_path(path: &str) -> String {
    let trimmed = path.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "/".into()
    } else {
        trimmed.into()
    }
}

fn build_caddyfile(state: &LocalDomainsState, log_dir: &Path) -> Result<String, String> {
    let mut sites: BTreeMap<(bool, String), Vec<&LocalDomainRoute>> = BTreeMap::new();
    for route in state.routes.iter().filter(|route| route.enabled) {
        sites
            .entry((route.https, route.hostname.to_ascii_lowercase()))
            .or_default()
            .push(route);
    }
    let log_path = log_dir.join("local-domains-access.log");
    let mut blocks = Vec::new();
    for ((https, hostname), mut routes) in sites {
        routes.sort_by_key(|route| std::cmp::Reverse(normalized_path(&route.path).len()));
        let address = if https {
            format!("https://{hostname}:{}", state.https_port)
        } else {
            format!("http://{hostname}:{}", state.http_port)
        };
        let mut directives = Vec::new();
        if https {
            directives.push("    tls internal".to_string());
        }
        for route in routes {
            let path = normalized_path(&route.path);
            if path == "/" {
                directives.push(format!(
                    "    handle {{\n        reverse_proxy {}\n    }}",
                    route.target
                ));
            } else {
                directives.push(format!(
                    "    handle_path {path}* {{\n        reverse_proxy {}\n    }}",
                    route.target
                ));
            }
        }
        directives.push(format!(
            "    log {{\n        output file \"{}\"\n    }}",
            log_path.display()
        ));
        blocks.push(format!("{address} {{\n{}\n}}", directives.join("\n")));
    }
    if blocks.is_empty() {
        blocks.push(format!(
            "http://127.0.0.1:{} {{\n    respond \"Zhiyu local domain gateway is ready\" 200\n}}",
            state.http_port
        ));
    }
    Ok(format!(
        "# Generated by Zhiyu Local Domains 2.0. Apply changes from Zhiyu.\n{{\n    admin off\n}}\n\n{}\n",
        blocks.join("\n\n")
    ))
}

fn state_path(root: &Path) -> PathBuf {
    root.join("tools/local-domains.json")
}

fn load_state(root: &Path) -> Result<LocalDomainsState, String> {
    let path = state_path(root);
    if !path.is_file() {
        return Ok(LocalDomainsState::default());
    }
    let bytes = fs::read(&path).map_err(|error| format!("读取本地域名配置失败：{error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("解析本地域名配置失败：{error}"))
}

fn save_state(root: &Path, state: &LocalDomainsState) -> Result<(), String> {
    let bytes =
        serde_json::to_vec_pretty(state).map_err(|error| format!("序列化配置失败：{error}"))?;
    atomic_write(&state_path(root), &bytes)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or_else(|| "保存路径无效".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("tmp");
    let mut file = fs::File::create(&temporary).map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    replace_file(&temporary, path).map_err(|error| error.to_string())
}

fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    match fs::rename(source, target) {
        Ok(()) => Ok(()),
        Err(_) if target.exists() => {
            fs::remove_file(target)?;
            fs::rename(source, target)
        }
        Err(error) => Err(error),
    }
}

fn prune_backups(dir: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut paths = entries
        .flatten()
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path()))
        })
        .collect::<Vec<_>>();
    paths.sort_by_key(|item| std::cmp::Reverse(item.0));
    for (_, path) in paths.into_iter().skip(keep) {
        let _ = fs::remove_file(path);
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn trim_detail(detail: &str) -> String {
    detail.trim().chars().take(4000).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_local_targets() {
        assert!(validate_target("10.0.0.2:3000").is_err());
        assert!(validate_target("127.0.0.1:3000").is_ok());
    }

    #[test]
    fn groups_multiple_paths_in_one_site() {
        let state = LocalDomainsState {
            routes: vec![
                LocalDomainRoute {
                    id: "1".into(),
                    name: "web".into(),
                    hostname: "demo.localhost".into(),
                    target: "127.0.0.1:3000".into(),
                    path: "/".into(),
                    https: false,
                    enabled: true,
                },
                LocalDomainRoute {
                    id: "2".into(),
                    name: "api".into(),
                    hostname: "demo.localhost".into(),
                    target: "127.0.0.1:8080".into(),
                    path: "/api".into(),
                    https: false,
                    enabled: true,
                },
            ],
            ..Default::default()
        };
        let output = build_caddyfile(&state, Path::new("/tmp")).unwrap();
        assert_eq!(output.matches("http://demo.localhost:8082 {").count(), 1);
        assert!(output.contains("handle_path /api*"));
    }
}
