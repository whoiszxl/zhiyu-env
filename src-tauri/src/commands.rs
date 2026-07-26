use devbox_core::{
    installer::{
        MAILPIT_SERIES, MAILPIT_VERSION, MONGODB_SERIES, MONGODB_VERSION, MYSQL_SERIES,
        MYSQL_VERSION, POSTGRES_SERIES, POSTGRES_VERSION, REDIS_SERIES, REDIS_VERSION,
    },
    MailpitInstaller, MailpitService, MongodbInstaller, MongodbService, MysqlInstaller,
    MysqlService, PostgresInstaller, PostgresService, RedisInstaller, RedisService, ServiceConfig,
    ServiceKind, ServiceManager, ServiceStatus,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKindInput {
    Redis,
    Mysql,
    Postgres,
    Mongodb,
    Mailpit,
}

impl From<ServiceKindInput> for ServiceKind {
    fn from(value: ServiceKindInput) -> Self {
        match value {
            ServiceKindInput::Redis => Self::Redis,
            ServiceKindInput::Mysql => Self::Mysql,
            ServiceKindInput::Postgres => Self::Postgres,
            ServiceKindInput::Mongodb => Self::Mongodb,
            ServiceKindInput::Mailpit => Self::Mailpit,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    kind: ServiceKind,
    name: &'static str,
    version: &'static str,
    port: u16,
    status: &'static str,
    pid: Option<u32>,
    instance_dir: PathBuf,
    config_path: PathBuf,
    data_path: PathBuf,
    log_path: PathBuf,
    executable_path: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMetrics {
    running: bool,
    cpu_percent: Option<f32>,
    memory_bytes: Option<u64>,
    uptime: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceDiskUsage {
    total_bytes: u64,
    installation_bytes: u64,
    data_bytes: u64,
    logs_bytes: u64,
    config_bytes: u64,
    cache_bytes: u64,
    other_bytes: u64,
}

fn devbox_root() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".devbox"))
        .ok_or_else(|| "无法确定当前用户目录".to_string())
}

fn service_config(kind: ServiceKind) -> Result<ServiceConfig, String> {
    let root = devbox_root()?;
    let (name, version, port, executable, arguments) = match kind {
        ServiceKind::Redis => {
            let instance = root.join("instances/redis/default");
            (
                "Redis",
                REDIS_VERSION,
                6379,
                root.join(format!(
                    "installations/redis/{REDIS_SERIES}/bin/redis-server"
                )),
                vec![instance.join("conf/redis.conf").display().to_string()],
            )
        }
        ServiceKind::Mysql => {
            let instance = root.join("instances/mysql/default");
            (
                "MySQL",
                MYSQL_VERSION,
                3306,
                root.join(format!("installations/mysql/{MYSQL_SERIES}/bin/mysqld")),
                vec![format!(
                    "--defaults-file={}",
                    instance.join("conf/my.cnf").display()
                )],
            )
        }
        ServiceKind::Postgres => {
            let instance = root.join("instances/postgres/default");
            (
                "PostgreSQL",
                POSTGRES_VERSION,
                5432,
                root.join(format!(
                    "installations/postgres/{POSTGRES_SERIES}/bin/postgres"
                )),
                vec![
                    "-D".into(),
                    instance.join("data").display().to_string(),
                    "-c".into(),
                    format!(
                        "config_file={}",
                        instance.join("conf/postgresql.conf").display()
                    ),
                ],
            )
        }
        ServiceKind::Mongodb => {
            let instance = root.join("instances/mongodb/default");
            (
                "MongoDB",
                MONGODB_VERSION,
                27017,
                root.join(format!("installations/mongodb/{MONGODB_SERIES}/bin/mongod")),
                vec![
                    "--config".into(),
                    instance.join("conf/mongod.conf").display().to_string(),
                ],
            )
        }
        ServiceKind::Mailpit => (
            "Mailpit",
            MAILPIT_VERSION,
            1025,
            root.join(format!(
                "installations/mailpit/{MAILPIT_SERIES}/bin/mailpit"
            )),
            Vec::new(),
        ),
    };

    let instance_dir = root.join("instances").join(kind.as_str()).join("default");
    let environment = if kind == ServiceKind::Mailpit {
        mailpit_environment(&instance_dir)
    } else {
        BTreeMap::new()
    };

    Ok(ServiceConfig {
        name: name.into(),
        kind,
        version: version.into(),
        port,
        executable,
        arguments,
        environment,
        instance_dir,
    })
}

fn mailpit_environment(instance_dir: &std::path::Path) -> BTreeMap<String, String> {
    const ALLOWED_KEYS: &[&str] = &[
        "MP_SMTP_BIND_ADDR",
        "MP_UI_BIND_ADDR",
        "MP_DATABASE",
        "MP_MAX_MESSAGES",
        "MP_MAX_MESSAGE_SIZE",
        "MP_MAX_AGE",
        "MP_DISABLE_VERSION_CHECK",
        "MP_BLOCK_REMOTE_CSS_AND_FONTS",
        "MP_QUIET",
        "MP_COMPRESSION",
    ];

    let mut environment = BTreeMap::from([
        ("MP_SMTP_BIND_ADDR".into(), "127.0.0.1:1025".into()),
        ("MP_UI_BIND_ADDR".into(), "127.0.0.1:8025".into()),
        (
            "MP_DATABASE".into(),
            instance_dir.join("data/mailpit.db").display().to_string(),
        ),
        ("MP_MAX_MESSAGES".into(), "500".into()),
        ("MP_MAX_MESSAGE_SIZE".into(), "10".into()),
        ("MP_DISABLE_VERSION_CHECK".into(), "true".into()),
        ("MP_BLOCK_REMOTE_CSS_AND_FONTS".into(), "true".into()),
        ("MP_QUIET".into(), "true".into()),
    ]);

    let path = instance_dir.join("conf/mailpit.env");
    if let Ok(contents) = fs::read_to_string(path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            if ALLOWED_KEYS.contains(&key) && mailpit_value_allowed(key, value, instance_dir) {
                environment.insert(key.into(), value.into());
            }
        }
    }
    environment
}

fn mailpit_value_allowed(key: &str, value: &str, instance_dir: &std::path::Path) -> bool {
    match key {
        "MP_SMTP_BIND_ADDR" => value == "127.0.0.1:1025",
        "MP_UI_BIND_ADDR" => value == "127.0.0.1:8025",
        "MP_DATABASE" => std::path::Path::new(value) == instance_dir.join("data/mailpit.db"),
        "MP_MAX_MESSAGES" => value
            .parse::<u32>()
            .is_ok_and(|value| (1..=5_000).contains(&value)),
        "MP_MAX_MESSAGE_SIZE" => value
            .parse::<u32>()
            .is_ok_and(|value| (1..=50).contains(&value)),
        "MP_COMPRESSION" => value.parse::<u8>().is_ok_and(|value| value <= 3),
        "MP_MAX_AGE" => {
            let number = value.strip_suffix('h').or_else(|| value.strip_suffix('d'));
            value.len() <= 12 && number.is_some_and(|number| number.parse::<u32>().is_ok())
        }
        "MP_DISABLE_VERSION_CHECK" | "MP_BLOCK_REMOTE_CSS_AND_FONTS" => value == "true",
        "MP_QUIET" => matches!(value, "true" | "false"),
        _ => false,
    }
}

fn with_service<T>(
    kind: ServiceKindInput,
    operation: impl FnOnce(&dyn ServiceManager) -> Result<T, devbox_core::DevBoxError>,
) -> Result<T, String> {
    let config = service_config(kind.into())?;
    match kind {
        ServiceKindInput::Redis => operation(&RedisService::new(config).map_err(stringify_error)?),
        ServiceKindInput::Mysql => operation(&MysqlService::new(config).map_err(stringify_error)?),
        ServiceKindInput::Postgres => {
            operation(&PostgresService::new(config).map_err(stringify_error)?)
        }
        ServiceKindInput::Mongodb => {
            operation(&MongodbService::new(config).map_err(stringify_error)?)
        }
        ServiceKindInput::Mailpit => {
            operation(&MailpitService::new(config).map_err(stringify_error)?)
        }
    }
    .map_err(stringify_error)
}

fn stringify_error(error: devbox_core::DevBoxError) -> String {
    error.to_string()
}

fn status_parts(status: ServiceStatus) -> (&'static str, Option<u32>) {
    match status {
        ServiceStatus::NotInstalled => ("not_installed", None),
        ServiceStatus::Stopped => ("stopped", None),
        ServiceStatus::Running { pid } => ("running", Some(pid)),
        ServiceStatus::StalePid { pid } => ("stale_pid", Some(pid)),
    }
}

fn info(kind: ServiceKindInput) -> Result<ServiceInfo, String> {
    let config = service_config(kind.into())?;
    let status = with_service(kind, |service| service.status())?;
    let (status, pid) = status_parts(status);
    let name = match kind {
        ServiceKindInput::Redis => "Redis",
        ServiceKindInput::Mysql => "MySQL",
        ServiceKindInput::Postgres => "PostgreSQL",
        ServiceKindInput::Mongodb => "MongoDB",
        ServiceKindInput::Mailpit => "Mailpit",
    };
    let version = match kind {
        ServiceKindInput::Redis => REDIS_VERSION,
        ServiceKindInput::Mysql => MYSQL_VERSION,
        ServiceKindInput::Postgres => POSTGRES_VERSION,
        ServiceKindInput::Mongodb => MONGODB_VERSION,
        ServiceKindInput::Mailpit => MAILPIT_VERSION,
    };

    Ok(ServiceInfo {
        kind: kind.into(),
        name,
        version,
        port: config.port,
        status,
        pid,
        instance_dir: config.instance_dir.clone(),
        config_path: native_config_path(&config),
        data_path: config.data_dir(),
        log_path: primary_log_path(&config),
        executable_path: config.executable,
    })
}

fn run_action(
    kind: ServiceKindInput,
    action: impl FnOnce(&dyn ServiceManager) -> Result<(), devbox_core::DevBoxError>,
) -> Result<ServiceInfo, String> {
    with_service(kind, action)?;
    info(kind)
}

#[derive(Clone, Copy)]
enum LifecycleAction {
    Start,
    Stop,
    Restart,
}

fn lifecycle_action(
    kind: ServiceKindInput,
    action: LifecycleAction,
) -> Result<ServiceInfo, String> {
    match action {
        LifecycleAction::Start => run_action(kind, |service| service.start().map(|_| ())),
        LifecycleAction::Stop => run_action(kind, |service| service.stop()),
        LifecycleAction::Restart => run_action(kind, |service| service.restart().map(|_| ())),
    }
}

async fn run_lifecycle_task(
    kind: ServiceKindInput,
    action: LifecycleAction,
) -> Result<ServiceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || lifecycle_action(kind, action))
        .await
        .map_err(|error| format!("服务管理任务异常结束: {error}"))?
}

#[tauri::command]
pub fn service_list() -> Result<Vec<ServiceInfo>, String> {
    [
        ServiceKindInput::Redis,
        ServiceKindInput::Mysql,
        ServiceKindInput::Postgres,
        ServiceKindInput::Mongodb,
        ServiceKindInput::Mailpit,
    ]
    .into_iter()
    .map(info)
    .collect()
}

fn install_service(kind: ServiceKindInput) -> Result<ServiceInfo, String> {
    match kind {
        ServiceKindInput::Redis => {
            RedisInstaller::new(devbox_root()?)
                .install()
                .map_err(stringify_error)?;
            run_action(kind, |service| service.install())
        }
        ServiceKindInput::Mysql => {
            let root = devbox_root()?;
            let installer = MysqlInstaller::new(&root);
            installer.install().map_err(stringify_error)?;
            with_service(kind, |service| service.install())?;
            let config = service_config(ServiceKind::Mysql)?;
            installer
                .initialize(&config.data_dir())
                .map_err(stringify_error)?;
            info(kind)
        }
        ServiceKindInput::Postgres => {
            let root = devbox_root()?;
            let installer = PostgresInstaller::new(&root);
            installer.install().map_err(stringify_error)?;
            with_service(kind, |service| service.install())?;
            let config = service_config(ServiceKind::Postgres)?;
            installer
                .initialize(&config.data_dir())
                .map_err(stringify_error)?;
            info(kind)
        }
        ServiceKindInput::Mongodb => {
            MongodbInstaller::new(devbox_root()?)
                .install()
                .map_err(stringify_error)?;
            run_action(kind, |service| service.install())
        }
        ServiceKindInput::Mailpit => {
            MailpitInstaller::new(devbox_root()?)
                .install()
                .map_err(stringify_error)?;
            run_action(kind, |service| service.install())
        }
    }
}

#[tauri::command]
pub async fn service_install(kind: ServiceKindInput) -> Result<ServiceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || install_service(kind))
        .await
        .map_err(|error| format!("服务安装任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn service_start(kind: ServiceKindInput) -> Result<ServiceInfo, String> {
    run_lifecycle_task(kind, LifecycleAction::Start).await
}

#[tauri::command]
pub async fn service_stop(kind: ServiceKindInput) -> Result<ServiceInfo, String> {
    run_lifecycle_task(kind, LifecycleAction::Stop).await
}

#[tauri::command]
pub async fn service_restart(kind: ServiceKindInput) -> Result<ServiceInfo, String> {
    run_lifecycle_task(kind, LifecycleAction::Restart).await
}

#[tauri::command]
pub async fn service_metrics(kind: ServiceKindInput) -> Result<ServiceMetrics, String> {
    tauri::async_runtime::spawn_blocking(move || collect_metrics(kind))
        .await
        .map_err(|error| format!("资源监控任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn service_disk_usage(kind: ServiceKindInput) -> Result<ServiceDiskUsage, String> {
    tauri::async_runtime::spawn_blocking(move || collect_disk_usage(kind))
        .await
        .map_err(|error| format!("磁盘占用统计任务异常结束: {error}"))?
}

#[tauri::command]
pub fn service_config_read(kind: ServiceKindInput) -> Result<String, String> {
    let config = service_config(kind.into())?;
    let path = native_config_path(&config);
    fs::read_to_string(&path).map_err(|error| format!("无法读取 {}: {error}", path.display()))
}

#[tauri::command]
pub fn service_config_save(kind: ServiceKindInput, content: String) -> Result<(), String> {
    if content.len() > 1024 * 1024 {
        return Err("配置文件不能超过 1 MiB".into());
    }
    if content.contains('\0') {
        return Err("配置文件不能包含 NUL 字符".into());
    }

    let config = service_config(kind.into())?;
    let path = native_config_path(&config);
    let parent = path
        .parent()
        .ok_or_else(|| "配置文件路径无效".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;

    if path.is_file() {
        fs::copy(&path, path.with_extension("bak")).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, content).map_err(|error| error.to_string())?;
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn service_logs(kind: ServiceKindInput) -> Result<String, String> {
    let config = service_config(kind.into())?;
    let mut sections = Vec::new();
    let mut sources = vec![
        ("STDOUT", config.stdout_log_path()),
        ("STDERR", config.stderr_log_path()),
    ];
    match config.kind {
        ServiceKind::Mysql => {
            sources.push(("MYSQL", config.logs_dir().join("mysql-error.log")));
        }
        ServiceKind::Mongodb => {
            sources.push(("MONGODB", config.logs_dir().join("mongodb.log")));
        }
        ServiceKind::Redis | ServiceKind::Postgres | ServiceKind::Mailpit => {}
    }
    for (label, path) in sources {
        if path.is_file() {
            let contents = tail_file(&path, 64 * 1024)?;
            if !contents.trim().is_empty() {
                sections.push(format!("── {label} ──\n{contents}"));
            }
        }
    }
    Ok(if sections.is_empty() {
        "暂无日志".into()
    } else {
        sections.join("\n\n")
    })
}

fn primary_log_path(config: &ServiceConfig) -> PathBuf {
    match config.kind {
        ServiceKind::Mysql => config.logs_dir().join("mysql-error.log"),
        ServiceKind::Mongodb => config.logs_dir().join("mongodb.log"),
        ServiceKind::Redis | ServiceKind::Postgres | ServiceKind::Mailpit => {
            config.stdout_log_path()
        }
    }
}

fn native_config_path(config: &ServiceConfig) -> PathBuf {
    let name = match config.kind {
        ServiceKind::Redis => "redis.conf",
        ServiceKind::Mysql => "my.cnf",
        ServiceKind::Postgres => "postgresql.conf",
        ServiceKind::Mongodb => "mongod.conf",
        ServiceKind::Mailpit => "mailpit.env",
    };
    config.config_dir().join(name)
}

fn collect_metrics(kind: ServiceKindInput) -> Result<ServiceMetrics, String> {
    let status = with_service(kind, |service| service.status())?;
    let ServiceStatus::Running { pid } = status else {
        return Ok(ServiceMetrics {
            running: false,
            cpu_percent: None,
            memory_bytes: None,
            uptime: None,
        });
    };

    let output = Command::new("/bin/ps")
        .args([
            "-p",
            &pid.to_string(),
            "-o",
            "rss=",
            "-o",
            "%cpu=",
            "-o",
            "etime=",
        ])
        .output()
        .map_err(|error| format!("无法读取进程指标: {error}"))?;

    if !output.status.success() {
        return Ok(ServiceMetrics {
            running: false,
            cpu_percent: None,
            memory_bytes: None,
            uptime: None,
        });
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut parts = text.split_whitespace();
    let rss_kib = parts.next().and_then(|value| value.parse::<u64>().ok());
    let cpu_percent = parts.next().and_then(|value| value.parse::<f32>().ok());
    let uptime = parts.next().map(str::to_string);

    Ok(ServiceMetrics {
        running: true,
        cpu_percent,
        memory_bytes: rss_kib.map(|value| value * 1024),
        uptime,
    })
}

fn collect_disk_usage(kind: ServiceKindInput) -> Result<ServiceDiskUsage, String> {
    let root = devbox_root()?;
    let kind: ServiceKind = kind.into();
    let installation_bytes = directory_size(&root.join("installations").join(kind.as_str()))?;
    let instance = root.join("instances").join(kind.as_str());
    let data_bytes = directory_size(&instance.join("default/data"))?;
    let logs_bytes = directory_size(&instance.join("default/logs"))?;
    let config_bytes = directory_size(&instance.join("default/conf"))?;
    let instance_bytes = directory_size(&instance)?;
    let cache_bytes = download_cache_size(&root.join("downloads"), kind)?;
    let other_bytes = instance_bytes.saturating_sub(
        data_bytes
            .saturating_add(logs_bytes)
            .saturating_add(config_bytes),
    );
    let total_bytes = installation_bytes
        .saturating_add(instance_bytes)
        .saturating_add(cache_bytes);

    Ok(ServiceDiskUsage {
        total_bytes,
        installation_bytes,
        data_bytes,
        logs_bytes,
        config_bytes,
        cache_bytes,
        other_bytes,
    })
}

fn download_cache_size(downloads_dir: &Path, kind: ServiceKind) -> Result<u64, String> {
    if !downloads_dir.exists() {
        return Ok(0);
    }
    let prefix = match kind {
        ServiceKind::Redis => "redis",
        ServiceKind::Mysql => "mysql",
        ServiceKind::Postgres => "postgres",
        ServiceKind::Mongodb => "mongodb",
        ServiceKind::Mailpit => "mailpit",
    };
    let mut total = 0_u64;
    for entry in fs::read_dir(downloads_dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry.file_name();
        if name
            .to_string_lossy()
            .to_ascii_lowercase()
            .starts_with(prefix)
        {
            total = total.saturating_add(directory_size(&entry.path())?);
        }
    }
    Ok(total)
}

fn directory_size(path: &Path) -> Result<u64, String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(format!("无法读取 {}: {error}", path.display())),
    };
    if metadata.is_file() || metadata.file_type().is_symlink() {
        return Ok(metadata_disk_bytes(&metadata));
    }

    let mut total = metadata_disk_bytes(&metadata);
    let mut directories = vec![path.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("无法读取 {}: {error}", directory.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let metadata = fs::symlink_metadata(entry.path()).map_err(|error| error.to_string())?;
            if metadata.is_dir() {
                directories.push(entry.path());
            }
            total = total.saturating_add(metadata_disk_bytes(&metadata));
        }
    }
    Ok(total)
}

#[cfg(unix)]
fn metadata_disk_bytes(metadata: &fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    metadata.blocks().saturating_mul(512)
}

#[cfg(not(unix))]
fn metadata_disk_bytes(metadata: &fs::Metadata) -> u64 {
    metadata.len()
}

fn tail_file(path: &PathBuf, max_bytes: u64) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|error| error.to_string())?;
    let length = file.metadata().map_err(|error| error.to_string())?.len();
    let start = length.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start))
        .map_err(|error| error.to_string())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|error| error.to_string())?;
    if start > 0 {
        if let Some(first_newline) = contents.find('\n') {
            contents.drain(..=first_newline);
        }
    }
    Ok(contents)
}

#[cfg(test)]
mod tests {
    use super::{directory_size, mailpit_value_allowed};
    use std::fs;
    use std::path::Path;

    #[test]
    fn mailpit_config_stays_local_and_bounded() {
        let instance = Path::new("/tmp/mailpit-instance");
        assert!(mailpit_value_allowed(
            "MP_SMTP_BIND_ADDR",
            "127.0.0.1:1025",
            instance,
        ));
        assert!(!mailpit_value_allowed(
            "MP_SMTP_BIND_ADDR",
            "0.0.0.0:1025",
            instance,
        ));
        assert!(!mailpit_value_allowed(
            "MP_DATABASE",
            "http://remote.example/db",
            instance,
        ));
        assert!(mailpit_value_allowed("MP_MAX_MESSAGES", "500", instance,));
        assert!(!mailpit_value_allowed("MP_MAX_MESSAGES", "0", instance,));
    }

    #[test]
    fn directory_size_counts_nested_files() {
        let root =
            std::env::temp_dir().join(format!("zhiyu-disk-usage-test-{}", std::process::id()));
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("first.bin"), [0_u8; 7]).unwrap();
        fs::write(root.join("nested/second.bin"), [0_u8; 11]).unwrap();

        assert!(directory_size(&root).unwrap() >= 18);
        fs::remove_dir_all(root).unwrap();
    }
}
