use crate::commands::{path_disk_size, stopped_service_instance, ServiceKindInput};
use devbox_core::ServiceKind;
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheCleanupResult {
    removed_items: u32,
    freed_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceBackup {
    id: String,
    created_at_millis: u64,
    size_bytes: u64,
    automatic: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    safety_backup: ServiceBackup,
}

#[tauri::command]
pub async fn service_cache_clean(kind: ServiceKindInput) -> Result<CacheCleanupResult, String> {
    tauri::async_runtime::spawn_blocking(move || clean_cache_at_root(&devbox_root()?, kind.into()))
        .await
        .map_err(|error| format!("缓存清理任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn service_backup_list(kind: ServiceKindInput) -> Result<Vec<ServiceBackup>, String> {
    tauri::async_runtime::spawn_blocking(move || list_backups(&devbox_root()?, kind.into()))
        .await
        .map_err(|error| format!("备份列表任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn service_backup_create(kind: ServiceKindInput) -> Result<ServiceBackup, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let instance = stopped_service_instance(kind)?;
        create_backup(&devbox_root()?, kind.into(), &instance, false)
    })
    .await
    .map_err(|error| format!("数据备份任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn service_backup_restore(
    kind: ServiceKindInput,
    backup_id: String,
) -> Result<RestoreResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let service_kind: ServiceKind = kind.into();
        let root = devbox_root()?;
        let instance = stopped_service_instance(kind)?;
        restore_backup(&root, service_kind, &instance, &backup_id)
    })
    .await
    .map_err(|error| format!("数据恢复任务异常结束: {error}"))?
}

fn devbox_root() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".devbox"))
        .ok_or_else(|| "无法确定当前用户目录".to_string())
}

fn service_prefix(kind: ServiceKind) -> &'static str {
    match kind {
        ServiceKind::Redis => "redis",
        ServiceKind::Mysql => "mysql",
        ServiceKind::Postgres => "postgres",
        ServiceKind::Mongodb => "mongodb",
        ServiceKind::Mailpit => "mailpit",
        ServiceKind::Nats => "nats",
        ServiceKind::Meilisearch => "meilisearch",
        ServiceKind::Minio => "minio.",
        ServiceKind::Rustfs => "rustfs-",
        ServiceKind::Etcd => "etcd-",
        ServiceKind::Consul => "consul_",
    }
}

fn clean_cache_at_root(root: &Path, kind: ServiceKind) -> Result<CacheCleanupResult, String> {
    let mut result = CacheCleanupResult {
        removed_items: 0,
        freed_bytes: 0,
    };
    for cache_dir in [root.join("downloads"), root.join("tmp")] {
        if !cache_dir.exists() {
            continue;
        }
        for entry in fs::read_dir(&cache_dir)
            .map_err(|error| format!("无法读取 {}: {error}", cache_dir.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            if !entry
                .file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .starts_with(service_prefix(kind))
            {
                continue;
            }
            let path = entry.path();
            let size = path_disk_size(&path)?;
            let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                fs::remove_dir_all(&path)
                    .map_err(|error| format!("无法删除 {}: {error}", path.display()))?;
            } else {
                fs::remove_file(&path)
                    .map_err(|error| format!("无法删除 {}: {error}", path.display()))?;
            }
            result.removed_items += 1;
            result.freed_bytes = result.freed_bytes.saturating_add(size);
        }
    }
    Ok(result)
}

fn backup_dir(root: &Path, kind: ServiceKind) -> PathBuf {
    root.join("backups").join(kind.as_str())
}

fn list_backups(root: &Path, kind: ServiceKind) -> Result<Vec<ServiceBackup>, String> {
    let directory = backup_dir(root, kind);
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut backups = Vec::new();
    for entry in fs::read_dir(&directory)
        .map_err(|error| format!("无法读取 {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let Some(id) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let metadata = fs::symlink_metadata(&path).map_err(|error| error.to_string())?;
        if !backup_id_is_safe(id) || !metadata.is_file() || metadata.file_type().is_symlink() {
            continue;
        }
        let created_at_millis = metadata
            .modified()
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        backups.push(ServiceBackup {
            id: id.into(),
            created_at_millis,
            size_bytes: metadata.len(),
            automatic: id.contains("-before-restore"),
        });
    }
    backups.sort_by(|left, right| {
        right
            .created_at_millis
            .cmp(&left.created_at_millis)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(backups)
}

fn create_backup(
    root: &Path,
    kind: ServiceKind,
    instance: &Path,
    automatic: bool,
) -> Result<ServiceBackup, String> {
    let data = instance.join("data");
    let config = instance.join("conf");
    if !data.is_dir() || !config.is_dir() {
        return Err("实例的数据目录或配置目录不存在".into());
    }

    let directory = backup_dir(root, kind);
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis() as u64;
    let suffix = if automatic { "-before-restore" } else { "" };
    let mut sequence = 0_u32;
    let (id, destination) = loop {
        let sequence_suffix = if sequence == 0 {
            String::new()
        } else {
            format!("-{sequence}")
        };
        let id = format!("{now}{suffix}{sequence_suffix}.tar.gz");
        let destination = directory.join(&id);
        if !destination.exists() {
            break (id, destination);
        }
        sequence += 1;
    };
    let temporary = destination.with_extension("partial");
    let output = Command::new("/usr/bin/tar")
        .args(["-czf"])
        .arg(&temporary)
        .arg("-C")
        .arg(instance)
        .args(["data", "conf"])
        .output()
        .map_err(|error| format!("无法启动 tar: {error}"))?;
    if !output.status.success() {
        let _ = fs::remove_file(&temporary);
        return Err(format!(
            "创建备份失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    fs::rename(&temporary, &destination).map_err(|error| error.to_string())?;
    let metadata = fs::metadata(&destination).map_err(|error| error.to_string())?;
    Ok(ServiceBackup {
        id,
        created_at_millis: now,
        size_bytes: metadata.len(),
        automatic,
    })
}

fn restore_backup(
    root: &Path,
    kind: ServiceKind,
    instance: &Path,
    backup_id: &str,
) -> Result<RestoreResult, String> {
    if !backup_id_is_safe(backup_id) {
        return Err("备份文件标识无效".into());
    }
    let archive = backup_dir(root, kind).join(backup_id);
    let archive_metadata =
        fs::symlink_metadata(&archive).map_err(|_| "指定的备份不存在".to_string())?;
    if !archive_metadata.is_file() || archive_metadata.file_type().is_symlink() {
        return Err("指定的备份不存在".into());
    }
    validate_archive(&archive)?;

    let safety_backup = create_backup(root, kind, instance, true)?;
    let restore_root = instance
        .parent()
        .ok_or_else(|| "实例目录无效".to_string())?
        .join(format!(
            ".restore-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
    let extracted = restore_root.join("extracted");
    fs::create_dir_all(&extracted).map_err(|error| error.to_string())?;

    let output = Command::new("/usr/bin/tar")
        .args(["-xzf"])
        .arg(&archive)
        .arg("-C")
        .arg(&extracted)
        .output()
        .map_err(|error| format!("无法启动 tar: {error}"))?;
    if !output.status.success() {
        let _ = fs::remove_dir_all(&restore_root);
        return Err(format!(
            "解压备份失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    if !extracted.join("data").is_dir() || !extracted.join("conf").is_dir() {
        let _ = fs::remove_dir_all(&restore_root);
        return Err("备份中缺少 data 或 conf 目录".into());
    }

    let result = replace_instance_contents(instance, &restore_root, &extracted);
    if result.is_err() {
        let _ = fs::remove_dir_all(&restore_root);
    }
    result?;
    let _ = fs::remove_dir_all(&restore_root);
    Ok(RestoreResult { safety_backup })
}

fn replace_instance_contents(
    instance: &Path,
    workspace: &Path,
    extracted: &Path,
) -> Result<(), String> {
    let current_data = instance.join("data");
    let current_config = instance.join("conf");
    let previous_data = workspace.join("previous-data");
    let previous_config = workspace.join("previous-conf");
    fs::rename(&current_data, &previous_data).map_err(|error| error.to_string())?;
    if let Err(error) = fs::rename(&current_config, &previous_config) {
        let _ = fs::rename(&previous_data, &current_data);
        return Err(error.to_string());
    }
    if let Err(error) = fs::rename(extracted.join("data"), &current_data) {
        let _ = fs::rename(&previous_data, &current_data);
        let _ = fs::rename(&previous_config, &current_config);
        return Err(error.to_string());
    }
    if let Err(error) = fs::rename(extracted.join("conf"), &current_config) {
        let _ = fs::rename(&current_data, extracted.join("data"));
        let _ = fs::rename(&previous_data, &current_data);
        let _ = fs::rename(&previous_config, &current_config);
        return Err(error.to_string());
    }
    Ok(())
}

fn validate_archive(archive: &Path) -> Result<(), String> {
    let names = Command::new("/usr/bin/tar")
        .args(["-tzf"])
        .arg(archive)
        .output()
        .map_err(|error| format!("无法读取备份: {error}"))?;
    if !names.status.success() {
        return Err("备份压缩包已损坏".into());
    }
    let entries =
        String::from_utf8(names.stdout).map_err(|_| "备份文件名不是有效 UTF-8".to_string())?;
    if entries.lines().any(|entry| !archive_entry_is_safe(entry)) {
        return Err("备份包含不安全的文件路径".into());
    }

    let verbose = Command::new("/usr/bin/tar")
        .args(["-tvzf"])
        .arg(archive)
        .output()
        .map_err(|error| format!("无法检查备份内容: {error}"))?;
    if !verbose.status.success()
        || String::from_utf8_lossy(&verbose.stdout)
            .lines()
            .any(|line| !matches!(line.as_bytes().first(), Some(b'-' | b'd')))
    {
        return Err("备份中包含不支持的链接或特殊文件".into());
    }
    Ok(())
}

fn archive_entry_is_safe(entry: &str) -> bool {
    let path = Path::new(entry.trim_end_matches('/'));
    let mut components = path.components();
    let Some(Component::Normal(first)) = components.next() else {
        return false;
    };
    if first != "data" && first != "conf" {
        return false;
    }
    components.all(|component| matches!(component, Component::Normal(_)))
}

fn backup_id_is_safe(id: &str) -> bool {
    id.len() <= 96
        && id.ends_with(".tar.gz")
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unsafe_backup_paths_and_ids() {
        assert!(archive_entry_is_safe("data/database.bin"));
        assert!(archive_entry_is_safe("conf/redis.conf"));
        assert!(!archive_entry_is_safe("../data/database.bin"));
        assert!(!archive_entry_is_safe("data/../../outside"));
        assert!(!archive_entry_is_safe("logs/output.log"));
        assert!(backup_id_is_safe("123-before-restore.tar.gz"));
        assert!(!backup_id_is_safe("../backup.tar.gz"));
    }

    #[test]
    fn cache_cleanup_only_removes_the_selected_service() {
        let root =
            std::env::temp_dir().join(format!("zhiyu-cache-clean-test-{}", std::process::id()));
        fs::create_dir_all(root.join("downloads")).unwrap();
        fs::create_dir_all(root.join("tmp/redis-build")).unwrap();
        fs::write(root.join("downloads/redis-7.tar.gz"), [1_u8; 32]).unwrap();
        fs::write(root.join("downloads/mysql-8.tar.gz"), [1_u8; 16]).unwrap();
        fs::write(root.join("tmp/redis-build/object.o"), [1_u8; 8]).unwrap();

        let result = clean_cache_at_root(&root, ServiceKind::Redis).unwrap();
        assert_eq!(result.removed_items, 2);
        assert!(!root.join("downloads/redis-7.tar.gz").exists());
        assert!(!root.join("tmp/redis-build").exists());
        assert!(root.join("downloads/mysql-8.tar.gz").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn backup_and_restore_round_trip_preserves_current_state() {
        let root = std::env::temp_dir().join(format!(
            "zhiyu-backup-test-{}-{}",
            std::process::id(),
            unique_suffix()
        ));
        let instance = root.join("instances/redis/default");
        fs::create_dir_all(instance.join("data")).unwrap();
        fs::create_dir_all(instance.join("conf")).unwrap();
        fs::write(instance.join("data/value.txt"), "before").unwrap();
        fs::write(instance.join("conf/redis.conf"), "port 6379").unwrap();

        let backup = create_backup(&root, ServiceKind::Redis, &instance, false).unwrap();
        fs::write(instance.join("data/value.txt"), "after").unwrap();
        fs::write(instance.join("conf/redis.conf"), "port 6380").unwrap();

        let restored = restore_backup(&root, ServiceKind::Redis, &instance, &backup.id).unwrap();
        assert_eq!(
            fs::read_to_string(instance.join("data/value.txt")).unwrap(),
            "before"
        );
        assert_eq!(
            fs::read_to_string(instance.join("conf/redis.conf")).unwrap(),
            "port 6379"
        );
        assert!(restored.safety_backup.automatic);
        assert_eq!(list_backups(&root, ServiceKind::Redis).unwrap().len(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
