use crate::commands::{self, ServiceKindInput};
use crate::port_tools;
use devbox_core::{ServiceKind, ServiceStatus};
use serde::Serialize;
use std::fs;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STALE_INSTALL_ARTIFACT_AGE: Duration = Duration::from_secs(24 * 60 * 60);

const SERVICE_KINDS: [ServiceKindInput; 16] = [
    ServiceKindInput::Redis,
    ServiceKindInput::Mysql,
    ServiceKindInput::Postgres,
    ServiceKindInput::Mongodb,
    ServiceKindInput::Mailpit,
    ServiceKindInput::Nats,
    ServiceKindInput::Kafka,
    ServiceKindInput::Meilisearch,
    ServiceKindInput::Minio,
    ServiceKindInput::Rustfs,
    ServiceKindInput::Etcd,
    ServiceKindInput::Consul,
    ServiceKindInput::Rnacos,
    ServiceKindInput::Rabbitmq,
    ServiceKindInput::Nginx,
    ServiceKindInput::Caddy,
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    generated_at_millis: u64,
    summary: DiagnosticSummary,
    items: Vec<DiagnosticItem>,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticSummary {
    passed: usize,
    warnings: usize,
    errors: usize,
    repairable: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticItem {
    id: String,
    scope: String,
    title: String,
    status: &'static str,
    message: String,
    detail: Option<String>,
    repairable: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticRepairResult {
    repaired_count: usize,
    messages: Vec<String>,
    report: DiagnosticReport,
}

impl DiagnosticItem {
    fn new(
        id: impl Into<String>,
        scope: impl Into<String>,
        title: impl Into<String>,
        status: &'static str,
        message: impl Into<String>,
        repairable: bool,
    ) -> Self {
        Self {
            id: id.into(),
            scope: scope.into(),
            title: title.into(),
            status,
            message: message.into(),
            detail: None,
            repairable,
        }
    }

    fn with_detail(mut self, detail: impl Into<String>) -> Self {
        let detail = detail.into();
        if !detail.trim().is_empty() {
            self.detail = Some(detail);
        }
        self
    }
}

#[tauri::command]
pub async fn app_diagnostics_run() -> Result<DiagnosticReport, String> {
    tauri::async_runtime::spawn_blocking(run_diagnostics)
        .await
        .map_err(|error| format!("诊断任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn app_diagnostics_repair() -> Result<DiagnosticRepairResult, String> {
    tauri::async_runtime::spawn_blocking(repair_and_diagnose)
        .await
        .map_err(|error| format!("修复任务异常结束: {error}"))?
}

fn run_diagnostics() -> Result<DiagnosticReport, String> {
    let root = crate::settings::devbox_root()?;
    let mut items = Vec::new();

    diagnose_root(&root, &mut items);
    diagnose_disk_space(&root, &mut items);
    diagnose_install_cache(&root, &mut items);

    let listeners = port_tools::read_port_listeners().unwrap_or_default();
    for kind in SERVICE_KINDS {
        diagnose_service(kind, &listeners, &mut items);
    }

    Ok(build_report(items))
}

fn diagnose_root(root: &Path, items: &mut Vec<DiagnosticItem>) {
    if !root.exists() {
        items.push(DiagnosticItem::new(
            "root-missing",
            "智屿",
            "安装目录",
            "warning",
            format!("安装目录 {} 尚未创建", root.display()),
            true,
        ));
        return;
    }
    if !root.is_dir() {
        items.push(DiagnosticItem::new(
            "root-invalid",
            "智屿",
            "安装目录",
            "error",
            format!("{} 不是目录", root.display()),
            false,
        ));
        return;
    }

    let probe = root.join(format!(".diagnostic-write-{}", std::process::id()));
    match fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(probe);
            items.push(DiagnosticItem::new(
                "root-writable",
                "智屿",
                "安装目录",
                "passed",
                format!("{} 可正常读写", root.display()),
                false,
            ));
        }
        Err(error) => items.push(DiagnosticItem::new(
            "root-not-writable",
            "智屿",
            "安装目录权限",
            "error",
            format!("无法写入 {}：{error}", root.display()),
            false,
        )),
    }
}

fn diagnose_disk_space(root: &Path, items: &mut Vec<DiagnosticItem>) {
    let Some(bytes) = available_disk_bytes(root) else {
        items.push(DiagnosticItem::new(
            "disk-space-unknown",
            "智屿",
            "可用磁盘空间",
            "warning",
            "无法读取当前磁盘的可用空间",
            false,
        ));
        return;
    };
    let (status, message) = if bytes < 512 * 1024 * 1024 {
        (
            "error",
            format!("仅剩 {}，安装服务可能失败", format_bytes(bytes)),
        )
    } else if bytes < 2 * 1024 * 1024 * 1024 {
        (
            "warning",
            format!("剩余 {}，建议尽快清理", format_bytes(bytes)),
        )
    } else {
        ("passed", format!("剩余 {}，空间充足", format_bytes(bytes)))
    };
    items.push(DiagnosticItem::new(
        "disk-space",
        "智屿",
        "可用磁盘空间",
        status,
        message,
        false,
    ));
}

fn diagnose_install_cache(root: &Path, items: &mut Vec<DiagnosticItem>) {
    let artifacts = incomplete_install_artifacts(root);
    if artifacts.is_empty() {
        items.push(DiagnosticItem::new(
            "install-cache-clean",
            "安装器",
            "安装残留",
            "passed",
            "未发现未完成的安装临时文件",
            false,
        ));
    } else {
        items.push(
            DiagnosticItem::new(
                "install-cache-incomplete",
                "安装器",
                "安装残留",
                "warning",
                format!(
                    "发现 {} 个超过 24 小时的未完成下载或安装目录",
                    artifacts.len()
                ),
                !commands::has_active_install_tasks(),
            )
            .with_detail(
                artifacts
                    .iter()
                    .take(20)
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        );
    }
}

fn diagnose_service(
    kind: ServiceKindInput,
    listeners: &[port_tools::PortListener],
    items: &mut Vec<DiagnosticItem>,
) {
    let service_kind: ServiceKind = kind.into();
    let Ok(config) = commands::service_config(service_kind) else {
        return;
    };
    let metadata_exists = config.metadata_path().is_file();
    let executable_exists = config.executable.is_file();
    if !metadata_exists && !executable_exists {
        return;
    }

    let scope = config.name.clone();
    if executable_exists {
        items.push(DiagnosticItem::new(
            format!("{}-executable", service_kind.as_str()),
            &scope,
            "可执行文件",
            "passed",
            format!("{} 已就绪", config.executable.display()),
            false,
        ));
    } else {
        items.push(DiagnosticItem::new(
            format!("{}-executable-missing", service_kind.as_str()),
            &scope,
            "可执行文件",
            "error",
            format!("缺少 {}", config.executable.display()),
            false,
        ));
    }

    let native_config = commands::native_config_path(&config);
    if native_config.is_file() && metadata_exists {
        items.push(DiagnosticItem::new(
            format!("{}-config", service_kind.as_str()),
            &scope,
            "服务配置",
            "passed",
            "实例元数据和配置文件完整",
            false,
        ));
    } else {
        let mut missing = Vec::new();
        if !metadata_exists {
            missing.push(config.metadata_path().display().to_string());
        }
        if !native_config.is_file() {
            missing.push(native_config.display().to_string());
        }
        items.push(
            DiagnosticItem::new(
                format!("{}-config-missing", service_kind.as_str()),
                &scope,
                "服务配置",
                "error",
                "实例配置不完整，建议重新安装该版本",
                false,
            )
            .with_detail(missing.join("\n")),
        );
    }

    let missing_dirs = [
        config.config_dir(),
        config.data_dir(),
        config.logs_dir(),
        config.run_dir(),
    ]
    .into_iter()
    .filter(|path| !path.is_dir())
    .collect::<Vec<_>>();
    if !missing_dirs.is_empty() {
        items.push(
            DiagnosticItem::new(
                format!("{}-directories", service_kind.as_str()),
                &scope,
                "运行目录",
                "warning",
                format!("缺少 {} 个必要目录", missing_dirs.len()),
                true,
            )
            .with_detail(
                missing_dirs
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
        );
    }

    let status = commands::service_status(kind);
    match status {
        Ok(ServiceStatus::Running { pid }) => {
            if port_ready(config.port) || !config.wait_for_port {
                items.push(DiagnosticItem::new(
                    format!("{}-runtime", service_kind.as_str()),
                    &scope,
                    "运行状态",
                    "passed",
                    format!("进程 PID {pid} 正常，端口 {} 已就绪", config.port),
                    false,
                ));
            } else {
                items.push(
                    DiagnosticItem::new(
                        format!("{}-port-not-ready", service_kind.as_str()),
                        &scope,
                        "端口就绪",
                        "error",
                        format!("进程 PID {pid} 存在，但端口 {} 未监听", config.port),
                        false,
                    )
                    .with_detail(service_log_tail(kind)),
                );
            }
        }
        Ok(ServiceStatus::StalePid { pid }) => items.push(DiagnosticItem::new(
            format!("{}-stale-pid", service_kind.as_str()),
            &scope,
            "PID 状态",
            "error",
            format!("PID {pid} 已被其他进程复用或记录已过期"),
            true,
        )),
        Ok(ServiceStatus::Crashed { pid }) => items.push(
            DiagnosticItem::new(
                format!("{}-crashed", service_kind.as_str()),
                &scope,
                "异常退出",
                "error",
                format!("原进程 PID {pid} 已意外退出"),
                true,
            )
            .with_detail(service_log_tail(kind)),
        ),
        Ok(ServiceStatus::Stopped) | Ok(ServiceStatus::NotInstalled) => {
            if let Some(listener) = listeners.iter().find(|item| item.port == config.port) {
                items.push(DiagnosticItem::new(
                    format!("{}-port-conflict", service_kind.as_str()),
                    &scope,
                    "端口冲突",
                    "warning",
                    format!(
                        "端口 {} 被 {}（PID {}）占用",
                        config.port, listener.process, listener.pid
                    ),
                    false,
                ));
            }
        }
        Err(error) => items.push(DiagnosticItem::new(
            format!("{}-status-error", service_kind.as_str()),
            &scope,
            "状态检测",
            "error",
            error,
            false,
        )),
    }
}

fn repair_and_diagnose() -> Result<DiagnosticRepairResult, String> {
    let root = crate::settings::devbox_root()?;
    let mut repaired_count = 0;
    let mut messages = Vec::new();

    if !root.exists() {
        fs::create_dir_all(&root)
            .map_err(|error| format!("无法创建安装目录 {}：{error}", root.display()))?;
        repaired_count += 1;
        messages.push("已创建智屿安装目录".into());
    }
    for directory in ["downloads", "installations", "instances", "backups", "tmp"] {
        fs::create_dir_all(root.join(directory))
            .map_err(|error| format!("无法创建 {directory} 目录：{error}"))?;
    }

    if !commands::has_active_install_tasks() {
        let removed = remove_incomplete_install_artifacts(&root)?;
        if removed > 0 {
            repaired_count += removed;
            messages.push(format!("已清理 {removed} 个未完成的安装文件"));
        }
    }

    for kind in SERVICE_KINDS {
        let service_kind: ServiceKind = kind.into();
        let Ok(config) = commands::service_config(service_kind) else {
            continue;
        };
        if !config.executable.is_file() && !config.metadata_path().is_file() {
            continue;
        }
        let mut created = 0;
        for directory in [
            config.config_dir(),
            config.data_dir(),
            config.logs_dir(),
            config.run_dir(),
        ] {
            if !directory.is_dir() {
                fs::create_dir_all(&directory)
                    .map_err(|error| format!("无法创建 {}：{error}", directory.display()))?;
                created += 1;
            }
        }
        if created > 0 {
            repaired_count += created;
            messages.push(format!("已补齐 {} 的 {created} 个运行目录", config.name));
        }

        if matches!(
            commands::service_status(kind),
            Ok(ServiceStatus::StalePid { .. } | ServiceStatus::Crashed { .. })
        ) {
            commands::repair_service(kind)?;
            repaired_count += 1;
            messages.push(format!("已修复 {} 的异常运行状态", config.name));
        }
    }

    Ok(DiagnosticRepairResult {
        repaired_count,
        messages,
        report: run_diagnostics()?,
    })
}

fn build_report(items: Vec<DiagnosticItem>) -> DiagnosticReport {
    let mut summary = DiagnosticSummary::default();
    for item in &items {
        match item.status {
            "passed" => summary.passed += 1,
            "warning" => summary.warnings += 1,
            "error" => summary.errors += 1,
            _ => {}
        }
        if item.repairable {
            summary.repairable += 1;
        }
    }
    DiagnosticReport {
        generated_at_millis: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64,
        summary,
        items,
    }
}

fn port_ready(port: u16) -> bool {
    ("127.0.0.1", port)
        .to_socket_addrs()
        .ok()
        .and_then(|mut addresses| addresses.next())
        .is_some_and(|address| {
            TcpStream::connect_timeout(&address, Duration::from_millis(250)).is_ok()
        })
}

fn service_log_tail(kind: ServiceKindInput) -> String {
    commands::service_logs(kind)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(50)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n")
}

fn incomplete_install_artifacts(root: &Path) -> Vec<PathBuf> {
    incomplete_install_artifacts_older_than(root, STALE_INSTALL_ARTIFACT_AGE)
}

fn incomplete_install_artifacts_older_than(root: &Path, minimum_age: Duration) -> Vec<PathBuf> {
    let mut artifacts = Vec::new();
    let downloads = root.join("downloads");
    if let Ok(entries) = fs::read_dir(downloads) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) == Some("partial")
                && artifact_is_old_enough(&path, minimum_age)
            {
                artifacts.push(path);
            }
        }
    }
    let temporary = root.join("tmp");
    if let Ok(entries) = fs::read_dir(temporary) {
        artifacts.extend(
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| artifact_is_old_enough(path, minimum_age)),
        );
    }
    artifacts
}

fn artifact_is_old_enough(path: &Path, minimum_age: Duration) -> bool {
    if minimum_age.is_zero() {
        return path.exists();
    }
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age >= minimum_age)
}

fn remove_incomplete_install_artifacts(root: &Path) -> Result<usize, String> {
    let artifacts = incomplete_install_artifacts(root);
    remove_artifacts(&artifacts)
}

fn remove_artifacts(artifacts: &[PathBuf]) -> Result<usize, String> {
    for path in artifacts {
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("无法读取 {}：{error}", path.display()))?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(path)
                .map_err(|error| format!("无法删除 {}：{error}", path.display()))?;
        } else {
            fs::remove_file(path)
                .map_err(|error| format!("无法删除 {}：{error}", path.display()))?;
        }
    }
    Ok(artifacts.len())
}

#[cfg(unix)]
fn available_disk_bytes(path: &Path) -> Option<u64> {
    let target = if path.exists() {
        path
    } else {
        path.parent().unwrap_or(path)
    };
    let output = Command::new("df").args(["-Pk"]).arg(target).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().last()?;
    line.split_whitespace()
        .nth(3)?
        .parse::<u64>()
        .ok()
        .map(|kilobytes| kilobytes.saturating_mul(1024))
}

#[cfg(windows)]
fn available_disk_bytes(path: &Path) -> Option<u64> {
    let target = path.to_string_lossy().replace('\'', "''");
    let script = format!(
        "(Get-Item -LiteralPath '{}').PSDrive.Free",
        if path.exists() {
            target
        } else {
            path.parent()?.to_string_lossy().replace('\'', "''")
        }
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

#[cfg(not(any(unix, windows)))]
fn available_disk_bytes(_path: &Path) -> Option<u64> {
    None
}

fn format_bytes(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / GIB)
    } else {
        format!("{:.0} MB", bytes as f64 / MIB)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_artifacts_only_include_partial_downloads_and_temp_entries() {
        let root = std::env::temp_dir().join(format!(
            "zhiyu-diagnostics-artifacts-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("downloads")).unwrap();
        fs::create_dir_all(root.join("tmp/work")).unwrap();
        fs::write(root.join("downloads/redis.tar.gz"), b"cache").unwrap();
        fs::write(root.join("downloads/redis.tar.gz.partial"), b"partial").unwrap();

        let artifacts = incomplete_install_artifacts_older_than(&root, Duration::ZERO);
        assert_eq!(artifacts.len(), 2);
        assert!(artifacts
            .iter()
            .any(|path| path.ends_with("redis.tar.gz.partial")));
        assert!(artifacts.iter().any(|path| path.ends_with("tmp/work")));

        assert_eq!(remove_artifacts(&artifacts).unwrap(), 2);
        assert!(root.join("downloads/redis.tar.gz").is_file());
        assert!(!root.join("downloads/redis.tar.gz.partial").exists());
        assert!(!root.join("tmp/work").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn report_summary_counts_status_and_repairable_items() {
        let report = build_report(vec![
            DiagnosticItem::new("a", "app", "a", "passed", "ok", false),
            DiagnosticItem::new("b", "app", "b", "warning", "warn", true),
            DiagnosticItem::new("c", "app", "c", "error", "bad", false),
        ]);
        assert_eq!(report.summary.passed, 1);
        assert_eq!(report.summary.warnings, 1);
        assert_eq!(report.summary.errors, 1);
        assert_eq!(report.summary.repairable, 1);
    }
}
