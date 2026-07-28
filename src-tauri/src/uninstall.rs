use crate::commands::{self, ServiceInfo, ServiceKindInput};
use devbox_core::{
    installer::{
        caddy_release, etcd_release, mailpit_release, mysql_release, nats_release, nginx_release,
        postgres_release, redis_release, CADDY_RELEASES, ETCD_RELEASES, MAILPIT_RELEASES,
        MYSQL_RELEASES, NATS_RELEASES, NGINX_RELEASES, POSTGRES_RELEASES, REDIS_RELEASES,
    },
    CaddyInstaller, EtcdInstaller, MailpitInstaller, MysqlInstaller, NatsInstaller, NginxInstaller,
    PostgresInstaller, RedisInstaller, ServiceKind, ServiceStatus,
};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionUninstallResult {
    kind: ServiceKind,
    version: String,
    freed_bytes: u64,
    fallback_version: Option<String>,
    data_preserved: bool,
    service: ServiceInfo,
}

struct VersionTarget {
    directory: PathBuf,
    version: String,
}

#[tauri::command]
pub async fn service_version_uninstall(
    kind: ServiceKindInput,
    version: String,
) -> Result<VersionUninstallResult, String> {
    tauri::async_runtime::spawn_blocking(move || uninstall_version(kind, &version))
        .await
        .map_err(|error| format!("版本卸载任务异常结束: {error}"))?
}

fn uninstall_version(
    kind: ServiceKindInput,
    requested_version: &str,
) -> Result<VersionUninstallResult, String> {
    if commands::has_active_install_tasks() {
        return Err("有服务正在安装，请等待安装任务结束后再卸载版本".into());
    }
    let root = crate::settings::devbox_root()?;
    let target = version_target(&root, kind, requested_version)?;
    let base = root
        .join("installations")
        .join(ServiceKind::from(kind).as_str());
    ensure_no_symlinked_install_root(&root, &base)?;
    ensure_direct_child(&base, &target.directory)?;

    let metadata = fs::symlink_metadata(&target.directory).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            format!("{} {} 尚未安装", service_name(kind), target.version)
        } else {
            format!("无法读取 {}：{error}", target.directory.display())
        }
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("版本安装路径不是安全的普通目录，已拒绝删除".into());
    }

    let current = commands::service_config(kind.into())?;
    let deleting_current = current.executable.starts_with(&target.directory);
    if deleting_current
        && matches!(
            commands::service_status(kind),
            Ok(ServiceStatus::Running { .. })
        )
    {
        return Err(format!(
            "请先停止 {}，再卸载当前运行版本",
            service_name(kind)
        ));
    }

    let fallback_version = deleting_current
        .then(|| installed_fallback(&root, kind, &target.version))
        .flatten();
    if let Some(fallback) = fallback_version.as_deref() {
        commands::activate_installed_version(kind, fallback)?;
    }

    let freed_bytes = commands::path_disk_size(&target.directory)?;
    if let Err(error) = move_and_remove(&root, kind, &target) {
        return Err(error);
    }

    if base
        .read_dir()
        .ok()
        .is_some_and(|mut entries| entries.next().is_none())
    {
        let _ = fs::remove_dir(&base);
    }

    Ok(VersionUninstallResult {
        kind: kind.into(),
        version: target.version,
        freed_bytes,
        fallback_version,
        data_preserved: true,
        service: commands::service_info(kind)?,
    })
}

fn version_target(
    root: &Path,
    kind: ServiceKindInput,
    version: &str,
) -> Result<VersionTarget, String> {
    let (directory, exact_version) = match kind {
        ServiceKindInput::Redis => {
            let release =
                redis_release(version).ok_or_else(|| format!("不支持 Redis 版本 {version}"))?;
            (
                RedisInstaller::for_version(root, release.version)
                    .map_err(|error| error.to_string())?
                    .installation_dir(),
                release.version.to_string(),
            )
        }
        ServiceKindInput::Mysql => {
            let release =
                mysql_release(version).ok_or_else(|| format!("不支持 MySQL 版本 {version}"))?;
            (
                MysqlInstaller::for_version(root, release.version)
                    .map_err(|error| error.to_string())?
                    .installation_dir(),
                release.version.to_string(),
            )
        }
        ServiceKindInput::Postgres => {
            let release = postgres_release(version)
                .ok_or_else(|| format!("不支持 PostgreSQL 版本 {version}"))?;
            (
                PostgresInstaller::for_version(root, release.version)
                    .map_err(|error| error.to_string())?
                    .installation_dir(),
                release.version.to_string(),
            )
        }
        ServiceKindInput::Nginx => {
            let release =
                nginx_release(version).ok_or_else(|| format!("不支持 Nginx 版本 {version}"))?;
            (
                NginxInstaller::for_version(root, release.version)
                    .map_err(|error| error.to_string())?
                    .installation_dir(),
                release.version.to_string(),
            )
        }
        ServiceKindInput::Mailpit => {
            let release =
                mailpit_release(version).ok_or_else(|| format!("不支持 Mailpit 版本 {version}"))?;
            (
                MailpitInstaller::for_version(root, release.version)
                    .map_err(|error| error.to_string())?
                    .installation_dir(),
                release.version.to_string(),
            )
        }
        ServiceKindInput::Nats => {
            let release =
                nats_release(version).ok_or_else(|| format!("不支持 NATS 版本 {version}"))?;
            (
                NatsInstaller::for_version(root, release.version)
                    .map_err(|error| error.to_string())?
                    .installation_dir(),
                release.version.to_string(),
            )
        }
        ServiceKindInput::Etcd => {
            let release =
                etcd_release(version).ok_or_else(|| format!("不支持 etcd 版本 {version}"))?;
            (
                EtcdInstaller::for_version(root, release.version)
                    .map_err(|error| error.to_string())?
                    .installation_dir(),
                release.version.to_string(),
            )
        }
        ServiceKindInput::Caddy => {
            let release =
                caddy_release(version).ok_or_else(|| format!("不支持 Caddy 版本 {version}"))?;
            (
                CaddyInstaller::for_version(root, release.version)
                    .map_err(|error| error.to_string())?
                    .installation_dir(),
                release.version.to_string(),
            )
        }
        _ => {
            let config = commands::service_config(kind.into())?;
            if config.version != version {
                return Err(format!(
                    "{} 当前只安装了版本 {}",
                    config.name, config.version
                ));
            }
            let base = root
                .join("installations")
                .join(ServiceKind::from(kind).as_str());
            let relative = config
                .executable
                .strip_prefix(&base)
                .map_err(|_| "服务可执行文件不在智屿安装目录中，已拒绝删除")?;
            let series = relative
                .components()
                .next()
                .and_then(|component| match component {
                    std::path::Component::Normal(value) => Some(value),
                    _ => None,
                })
                .ok_or("无法确定服务版本目录，已拒绝删除")?;
            (base.join(series), config.version)
        }
    };
    Ok(VersionTarget {
        directory,
        version: exact_version,
    })
}

fn installed_fallback(
    root: &Path,
    kind: ServiceKindInput,
    removed_version: &str,
) -> Option<String> {
    match kind {
        ServiceKindInput::Redis => REDIS_RELEASES
            .iter()
            .filter(|release| release.version != removed_version)
            .filter(|release| {
                RedisInstaller::for_version(root, release.version)
                    .is_ok_and(|installer| installer.is_installed())
            })
            .max_by_key(|release| release.recommended)
            .map(|release| release.version.into()),
        ServiceKindInput::Mysql => MYSQL_RELEASES
            .iter()
            .filter(|release| release.version != removed_version)
            .filter(|release| {
                MysqlInstaller::for_version(root, release.version)
                    .is_ok_and(|installer| installer.is_installed())
            })
            .max_by_key(|release| release.recommended)
            .map(|release| release.version.into()),
        ServiceKindInput::Postgres => POSTGRES_RELEASES
            .iter()
            .filter(|release| release.version != removed_version)
            .filter(|release| {
                PostgresInstaller::for_version(root, release.version)
                    .is_ok_and(|installer| installer.is_installed())
            })
            .max_by_key(|release| release.recommended)
            .map(|release| release.version.into()),
        ServiceKindInput::Nginx => NGINX_RELEASES
            .iter()
            .filter(|release| release.version != removed_version)
            .filter(|release| {
                NginxInstaller::for_version(root, release.version)
                    .is_ok_and(|installer| installer.is_installed())
            })
            .max_by_key(|release| release.recommended)
            .map(|release| release.version.into()),
        ServiceKindInput::Mailpit => MAILPIT_RELEASES
            .iter()
            .filter(|release| release.version != removed_version)
            .filter(|release| {
                MailpitInstaller::for_version(root, release.version)
                    .is_ok_and(|installer| installer.is_installed())
            })
            .max_by_key(|release| release.recommended)
            .map(|release| release.version.into()),
        ServiceKindInput::Nats => NATS_RELEASES
            .iter()
            .filter(|release| release.version != removed_version)
            .filter(|release| {
                NatsInstaller::for_version(root, release.version)
                    .is_ok_and(|installer| installer.is_installed())
            })
            .max_by_key(|release| release.recommended)
            .map(|release| release.version.into()),
        ServiceKindInput::Etcd => ETCD_RELEASES
            .iter()
            .filter(|release| release.version != removed_version)
            .filter(|release| {
                EtcdInstaller::for_version(root, release.version)
                    .is_ok_and(|installer| installer.is_installed())
            })
            .max_by_key(|release| release.recommended)
            .map(|release| release.version.into()),
        ServiceKindInput::Caddy => CADDY_RELEASES
            .iter()
            .filter(|release| release.version != removed_version)
            .filter(|release| {
                CaddyInstaller::for_version(root, release.version)
                    .is_ok_and(|installer| installer.is_installed())
            })
            .max_by_key(|release| release.recommended)
            .map(|release| release.version.into()),
        _ => None,
    }
}

fn ensure_direct_child(base: &Path, target: &Path) -> Result<(), String> {
    if target.parent() == Some(base) && target.file_name().is_some() {
        Ok(())
    } else {
        Err("版本安装路径超出允许的服务目录，已拒绝删除".into())
    }
}

fn ensure_no_symlinked_install_root(root: &Path, base: &Path) -> Result<(), String> {
    for path in [
        root.to_path_buf(),
        root.join("installations"),
        base.to_path_buf(),
    ] {
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| format!("无法验证卸载目录 {}：{error}", path.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(format!(
                "卸载目录 {} 不是安全的普通目录，已拒绝删除",
                path.display()
            ));
        }
    }
    Ok(())
}

fn move_and_remove(
    root: &Path,
    kind: ServiceKindInput,
    target: &VersionTarget,
) -> Result<(), String> {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let trash_root = root.join("tmp");
    fs::create_dir_all(&trash_root).map_err(|error| error.to_string())?;
    let trash = trash_root.join(format!(
        "uninstall-{}-{}-{suffix}",
        ServiceKind::from(kind).as_str(),
        target.version
    ));
    fs::rename(&target.directory, &trash)
        .map_err(|error| format!("无法移动待卸载版本：{error}"))?;
    if let Err(error) = fs::remove_dir_all(&trash) {
        if target.directory.exists() || fs::rename(&trash, &target.directory).is_err() {
            return Err(format!(
                "程序版本已移出安装目录，但清理失败：{error}。可使用诊断修复清理临时目录"
            ));
        }
        return Err(format!("删除程序版本失败，已恢复原目录：{error}"));
    }
    Ok(())
}

fn service_name(kind: ServiceKindInput) -> &'static str {
    match kind {
        ServiceKindInput::Redis => "Redis",
        ServiceKindInput::Mysql => "MySQL",
        ServiceKindInput::Postgres => "PostgreSQL",
        ServiceKindInput::Mongodb => "MongoDB",
        ServiceKindInput::Mailpit => "Mailpit",
        ServiceKindInput::Nats => "NATS",
        ServiceKindInput::Kafka => "Kafka Sandbox",
        ServiceKindInput::Meilisearch => "Meilisearch",
        ServiceKindInput::Minio => "MinIO",
        ServiceKindInput::Rustfs => "RustFS",
        ServiceKindInput::Etcd => "etcd",
        ServiceKindInput::Consul => "Consul",
        ServiceKindInput::Rnacos => "rnacos",
        ServiceKindInput::Rabbitmq => "RabbitMQ",
        ServiceKindInput::Nginx => "Nginx",
        ServiceKindInput::Caddy => "Caddy",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deletion_target_must_be_a_direct_version_directory() {
        let base = Path::new("/tmp/devbox/installations/redis");
        assert!(ensure_direct_child(base, &base.join("7.2")).is_ok());
        assert!(ensure_direct_child(base, &base.join("7.2/bin")).is_err());
        assert!(ensure_direct_child(base, Path::new("/tmp/devbox/instances/redis")).is_err());
    }

    #[test]
    fn moving_program_directory_never_touches_instance_data() {
        let root =
            std::env::temp_dir().join(format!("zhiyu-uninstall-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let program = root.join("installations/redis/7.2");
        let data = root.join("instances/redis/default/data/7.2");
        fs::create_dir_all(&program).unwrap();
        fs::create_dir_all(&data).unwrap();
        fs::write(program.join("redis-server"), b"binary").unwrap();
        fs::write(data.join("dump.rdb"), b"data").unwrap();

        move_and_remove(
            &root,
            ServiceKindInput::Redis,
            &VersionTarget {
                directory: program.clone(),
                version: "7.2.12".into(),
            },
        )
        .unwrap();

        assert!(!program.exists());
        assert!(data.join("dump.rdb").is_file());
        fs::remove_dir_all(root).unwrap();
    }
}
