use devbox_core::{
    installer::{
        mysql_release, postgres_release, redis_release, MysqlRelease, PostgresRelease,
        RedisRelease, MAILPIT_SERIES, MAILPIT_VERSION, MONGODB_SERIES, MONGODB_VERSION,
        MYSQL_RELEASES, MYSQL_VERSION, POSTGRES_RELEASES, POSTGRES_VERSION, REDIS_RELEASES,
        REDIS_VERSION,
    },
    report_install_progress, with_install_reporter, ConfigManager, InstallReporter,
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
use tauri::{AppHandle, Emitter};

pub(crate) const INSTALL_PROGRESS_EVENT: &str = "install-progress";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallProgressEvent {
    operation_id: String,
    kind: String,
    percent: Option<u8>,
    stage: String,
    message: String,
    status: &'static str,
}

fn emit_install_event(app: &AppHandle, event: InstallProgressEvent) {
    let _ = app.emit(INSTALL_PROGRESS_EVENT, event);
}

pub(crate) fn run_install_task<T>(
    app: AppHandle,
    operation_id: String,
    kind: String,
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    if operation_id.is_empty() || operation_id.len() > 100 {
        return Err("无效的安装任务标识".into());
    }

    emit_install_event(
        &app,
        InstallProgressEvent {
            operation_id: operation_id.clone(),
            kind: kind.clone(),
            percent: Some(1),
            stage: "创建任务".into(),
            message: "安装任务已进入后台线程".into(),
            status: "running",
        },
    );

    let event_app = app.clone();
    let event_operation_id = operation_id.clone();
    let event_kind = kind.clone();
    let reporter = InstallReporter::new(move |update| {
        emit_install_event(
            &event_app,
            InstallProgressEvent {
                operation_id: event_operation_id.clone(),
                kind: event_kind.clone(),
                percent: update.percent,
                stage: update.stage,
                message: update.message,
                status: "running",
            },
        );
    });

    let result = with_install_reporter(reporter, operation);
    match &result {
        Ok(_) => emit_install_event(
            &app,
            InstallProgressEvent {
                operation_id,
                kind,
                percent: Some(100),
                stage: "安装完成".into(),
                message: "安装、配置和初始化均已完成".into(),
                status: "completed",
            },
        ),
        Err(error) => emit_install_event(
            &app,
            InstallProgressEvent {
                operation_id,
                kind,
                percent: None,
                stage: "安装失败".into(),
                message: error.clone(),
                status: "failed",
            },
        ),
    }
    result
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
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
    version: String,
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
    backup_bytes: u64,
    other_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedisVersionInfo {
    series: &'static str,
    version: &'static str,
    installed: bool,
    selected: bool,
    support_label: &'static str,
    legacy: bool,
    recommended: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MysqlVersionInfo {
    series: &'static str,
    version: &'static str,
    installed: bool,
    selected: bool,
    support_label: &'static str,
    legacy: bool,
    recommended: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostgresVersionInfo {
    series: &'static str,
    version: &'static str,
    installed: bool,
    selected: bool,
    support_label: &'static str,
    legacy: bool,
    recommended: bool,
}

fn devbox_root() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".devbox"))
        .ok_or_else(|| "无法确定当前用户目录".to_string())
}

fn selected_redis_release(root: &Path) -> &'static RedisRelease {
    let metadata = root.join("instances/redis/default/service.json");
    ConfigManager
        .load(metadata)
        .ok()
        .filter(|config| config.kind == ServiceKind::Redis)
        .and_then(|config| redis_release(&config.version))
        .unwrap_or_else(|| {
            redis_release(REDIS_VERSION).expect("default Redis release is registered")
        })
}

fn redis_service_config(root: &Path, release: &RedisRelease) -> ServiceConfig {
    let instance = root.join("instances/redis/default");
    let data_dir = instance.join("data").join(release.series);
    ServiceConfig {
        name: "Redis".into(),
        kind: ServiceKind::Redis,
        version: release.version.into(),
        port: 6379,
        executable: root
            .join("installations")
            .join("redis")
            .join(release.series)
            .join("bin/redis-server"),
        arguments: vec![
            instance.join("conf/redis.conf").display().to_string(),
            "--dir".into(),
            data_dir.display().to_string(),
        ],
        environment: BTreeMap::new(),
        instance_dir: instance,
    }
}

fn selected_mysql_release(root: &Path) -> &'static MysqlRelease {
    let metadata = root.join("instances/mysql/default/service.json");
    ConfigManager
        .load(metadata)
        .ok()
        .filter(|config| config.kind == ServiceKind::Mysql)
        .and_then(|config| mysql_release(&config.version))
        .unwrap_or_else(|| {
            mysql_release(MYSQL_VERSION).expect("default MySQL release is registered")
        })
}

fn mysql_service_config(root: &Path, release: &MysqlRelease) -> ServiceConfig {
    let instance = root.join("instances/mysql/default");
    let installation = root.join("installations/mysql").join(release.series);
    let data_dir = instance.join("data").join(release.series);
    ServiceConfig {
        name: "MySQL".into(),
        kind: ServiceKind::Mysql,
        version: release.version.into(),
        port: 3306,
        executable: installation.join("bin/mysqld"),
        arguments: vec![
            format!("--defaults-file={}", instance.join("conf/my.cnf").display()),
            format!("--basedir={}", installation.display()),
            format!("--datadir={}", data_dir.display()),
        ],
        environment: BTreeMap::new(),
        instance_dir: instance,
    }
}

fn selected_postgres_release(root: &Path) -> &'static PostgresRelease {
    let metadata = root.join("instances/postgres/default/service.json");
    ConfigManager
        .load(metadata)
        .ok()
        .filter(|config| config.kind == ServiceKind::Postgres)
        .and_then(|config| postgres_release(&config.version))
        .unwrap_or_else(|| {
            postgres_release(POSTGRES_VERSION).expect("default PostgreSQL release is registered")
        })
}

fn postgres_service_config(root: &Path, release: &PostgresRelease) -> ServiceConfig {
    let instance = root.join("instances/postgres/default");
    let data_dir = instance.join("data").join(release.series);
    ServiceConfig {
        name: "PostgreSQL".into(),
        kind: ServiceKind::Postgres,
        version: release.version.into(),
        port: 5432,
        executable: root
            .join("installations/postgres")
            .join(release.series)
            .join("bin/postgres"),
        arguments: vec![
            "-D".into(),
            data_dir.display().to_string(),
            "-c".into(),
            format!(
                "config_file={}",
                instance.join("conf/postgresql.conf").display()
            ),
            "-c".into(),
            format!("data_directory={}", data_dir.display()),
        ],
        environment: BTreeMap::new(),
        instance_dir: instance,
    }
}

fn service_config(kind: ServiceKind) -> Result<ServiceConfig, String> {
    let root = devbox_root()?;
    if kind == ServiceKind::Redis {
        return Ok(redis_service_config(&root, selected_redis_release(&root)));
    }
    if kind == ServiceKind::Mysql {
        return Ok(mysql_service_config(&root, selected_mysql_release(&root)));
    }
    if kind == ServiceKind::Postgres {
        return Ok(postgres_service_config(
            &root,
            selected_postgres_release(&root),
        ));
    }
    let (name, version, port, executable, arguments) = match kind {
        ServiceKind::Redis => unreachable!(),
        ServiceKind::Mysql => unreachable!(),
        ServiceKind::Postgres => unreachable!(),
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
    Ok(ServiceInfo {
        kind: kind.into(),
        name,
        version: config.version.clone(),
        port: config.port,
        status,
        pid,
        instance_dir: config.instance_dir.clone(),
        config_path: native_config_path(&config),
        data_path: match kind {
            ServiceKindInput::Redis => RedisService::new(config.clone())
                .map_err(stringify_error)?
                .data_dir(),
            ServiceKindInput::Mysql => MysqlService::new(config.clone())
                .map_err(stringify_error)?
                .data_dir(),
            ServiceKindInput::Postgres => PostgresService::new(config.clone())
                .map_err(stringify_error)?
                .data_dir(),
            _ => config.data_dir(),
        },
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
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Redis)?;
            RedisInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 Redis 实例配置");
            RedisService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            info(kind)
        }
        ServiceKindInput::Mysql => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Mysql)?;
            let installer =
                MysqlInstaller::for_version(&root, &config.version).map_err(stringify_error)?;
            installer.install().map_err(stringify_error)?;
            report_install_progress(92, "写入配置", "正在创建 MySQL 实例配置");
            let service = MysqlService::new(config).map_err(stringify_error)?;
            service.install().map_err(stringify_error)?;
            installer
                .initialize(&service.data_dir())
                .map_err(stringify_error)?;
            info(kind)
        }
        ServiceKindInput::Postgres => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Postgres)?;
            let installer =
                PostgresInstaller::for_version(&root, &config.version).map_err(stringify_error)?;
            installer.install().map_err(stringify_error)?;
            report_install_progress(92, "写入配置", "正在创建 PostgreSQL 实例配置");
            let service = PostgresService::new(config).map_err(stringify_error)?;
            service.install().map_err(stringify_error)?;
            installer
                .initialize(&service.data_dir())
                .map_err(stringify_error)?;
            info(kind)
        }
        ServiceKindInput::Mongodb => {
            MongodbInstaller::new(devbox_root()?)
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 MongoDB 实例配置");
            run_action(kind, |service| service.install())
        }
        ServiceKindInput::Mailpit => {
            MailpitInstaller::new(devbox_root()?)
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 Mailpit 实例配置");
            run_action(kind, |service| service.install())
        }
    }
}

#[tauri::command]
pub async fn service_install(
    app: AppHandle,
    kind: ServiceKindInput,
    operation_id: String,
) -> Result<ServiceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_install_task(
            app,
            operation_id,
            ServiceKind::from(kind).as_str().into(),
            || install_service(kind),
        )
    })
    .await
    .map_err(|error| format!("服务安装任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn redis_versions() -> Result<Vec<RedisVersionInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let root = devbox_root()?;
        let selected = selected_redis_release(&root);
        REDIS_RELEASES
            .iter()
            .map(|release| {
                let installer =
                    RedisInstaller::for_version(&root, release.version).map_err(stringify_error)?;
                Ok(RedisVersionInfo {
                    series: release.series,
                    version: release.version,
                    installed: installer.is_installed(),
                    selected: release.version == selected.version,
                    support_label: release.support_label,
                    legacy: release.legacy,
                    recommended: release.recommended,
                })
            })
            .collect()
    })
    .await
    .map_err(|error| format!("Redis 版本状态任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn redis_version_select(
    app: AppHandle,
    version: String,
    operation_id: String,
) -> Result<ServiceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_install_task(app, operation_id, "redis".into(), || {
            let root = devbox_root()?;
            let release =
                redis_release(&version).ok_or_else(|| format!("不支持 Redis 版本 {version}"))?;
            let current = service_config(ServiceKind::Redis)?;
            let current_service = RedisService::new(current).map_err(stringify_error)?;
            let status = current_service.status().map_err(stringify_error)?;
            if matches!(status, ServiceStatus::Running { .. }) {
                return Err("请先停止 Redis，再切换运行版本".into());
            }
            current_service
                .prepare_version_data()
                .map_err(stringify_error)?;

            RedisInstaller::for_version(&root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "切换版本", "正在更新 Redis 活动版本配置");
            RedisService::new(redis_service_config(&root, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            info(ServiceKindInput::Redis)
        })
    })
    .await
    .map_err(|error| format!("Redis 版本切换任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn mysql_versions() -> Result<Vec<MysqlVersionInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let root = devbox_root()?;
        let selected = selected_mysql_release(&root);
        MYSQL_RELEASES
            .iter()
            .map(|release| {
                let installer =
                    MysqlInstaller::for_version(&root, release.version).map_err(stringify_error)?;
                Ok(MysqlVersionInfo {
                    series: release.series,
                    version: release.version,
                    installed: installer.is_installed(),
                    selected: release.version == selected.version,
                    support_label: release.support_label,
                    legacy: release.legacy,
                    recommended: release.recommended,
                })
            })
            .collect()
    })
    .await
    .map_err(|error| format!("MySQL 版本状态任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn mysql_version_select(
    app: AppHandle,
    version: String,
    operation_id: String,
) -> Result<ServiceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_install_task(app, operation_id, "mysql".into(), || {
            let root = devbox_root()?;
            let release =
                mysql_release(&version).ok_or_else(|| format!("不支持 MySQL 版本 {version}"))?;
            let current = service_config(ServiceKind::Mysql)?;
            let current_service = MysqlService::new(current).map_err(stringify_error)?;
            let status = current_service.status().map_err(stringify_error)?;
            if matches!(status, ServiceStatus::Running { .. }) {
                return Err("请先停止 MySQL，再切换运行版本".into());
            }
            current_service
                .prepare_version_data()
                .map_err(stringify_error)?;

            let installer =
                MysqlInstaller::for_version(&root, release.version).map_err(stringify_error)?;
            installer.install().map_err(stringify_error)?;
            report_install_progress(92, "切换版本", "正在更新 MySQL 活动版本配置");
            let service =
                MysqlService::new(mysql_service_config(&root, release)).map_err(stringify_error)?;
            service.install().map_err(stringify_error)?;
            installer
                .initialize(&service.data_dir())
                .map_err(stringify_error)?;
            info(ServiceKindInput::Mysql)
        })
    })
    .await
    .map_err(|error| format!("MySQL 版本切换任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn postgres_versions() -> Result<Vec<PostgresVersionInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let root = devbox_root()?;
        let selected = selected_postgres_release(&root);
        POSTGRES_RELEASES
            .iter()
            .map(|release| {
                let installer = PostgresInstaller::for_version(&root, release.version)
                    .map_err(stringify_error)?;
                Ok(PostgresVersionInfo {
                    series: release.series,
                    version: release.version,
                    installed: installer.is_installed(),
                    selected: release.version == selected.version,
                    support_label: release.support_label,
                    legacy: release.legacy,
                    recommended: release.recommended,
                })
            })
            .collect()
    })
    .await
    .map_err(|error| format!("PostgreSQL 版本状态任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn postgres_version_select(
    app: AppHandle,
    version: String,
    operation_id: String,
) -> Result<ServiceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_install_task(app, operation_id, "postgres".into(), || {
            let root = devbox_root()?;
            let release = postgres_release(&version)
                .ok_or_else(|| format!("不支持 PostgreSQL 版本 {version}"))?;
            let current = service_config(ServiceKind::Postgres)?;
            let current_service = PostgresService::new(current).map_err(stringify_error)?;
            let status = current_service.status().map_err(stringify_error)?;
            if matches!(status, ServiceStatus::Running { .. }) {
                return Err("请先停止 PostgreSQL，再切换运行版本".into());
            }
            current_service
                .prepare_version_data()
                .map_err(stringify_error)?;

            let installer =
                PostgresInstaller::for_version(&root, release.version).map_err(stringify_error)?;
            installer.install().map_err(stringify_error)?;
            report_install_progress(92, "切换版本", "正在更新 PostgreSQL 活动版本配置");
            let service = PostgresService::new(postgres_service_config(&root, release))
                .map_err(stringify_error)?;
            service.install().map_err(stringify_error)?;
            installer
                .initialize(&service.data_dir())
                .map_err(stringify_error)?;
            info(ServiceKindInput::Postgres)
        })
    })
    .await
    .map_err(|error| format!("PostgreSQL 版本切换任务异常结束: {error}"))?
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
    let installation_bytes = path_disk_size(&root.join("installations").join(kind.as_str()))?;
    let instance = root.join("instances").join(kind.as_str());
    let data_bytes = path_disk_size(&instance.join("default/data"))?;
    let logs_bytes = path_disk_size(&instance.join("default/logs"))?;
    let config_bytes = path_disk_size(&instance.join("default/conf"))?;
    let instance_bytes = path_disk_size(&instance)?;
    let cache_bytes = download_cache_size(&root.join("downloads"), kind)?
        .saturating_add(download_cache_size(&root.join("tmp"), kind)?);
    let backup_bytes = path_disk_size(&root.join("backups").join(kind.as_str()))?;
    let other_bytes = instance_bytes.saturating_sub(
        data_bytes
            .saturating_add(logs_bytes)
            .saturating_add(config_bytes),
    );
    let total_bytes = installation_bytes
        .saturating_add(instance_bytes)
        .saturating_add(cache_bytes)
        .saturating_add(backup_bytes);

    Ok(ServiceDiskUsage {
        total_bytes,
        installation_bytes,
        data_bytes,
        logs_bytes,
        config_bytes,
        cache_bytes,
        backup_bytes,
        other_bytes,
    })
}

pub(crate) fn stopped_service_instance(kind: ServiceKindInput) -> Result<PathBuf, String> {
    let config = service_config(kind.into())?;
    match with_service(kind, |service| service.status())? {
        ServiceStatus::NotInstalled => Err(format!("{} 尚未安装，无法备份或恢复", config.name)),
        ServiceStatus::Running { .. } => Err(format!("请先停止 {}，再执行备份或恢复", config.name)),
        ServiceStatus::Stopped | ServiceStatus::StalePid { .. } => Ok(config.instance_dir),
    }
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
            total = total.saturating_add(path_disk_size(&entry.path())?);
        }
    }
    Ok(total)
}

pub(crate) fn path_disk_size(path: &Path) -> Result<u64, String> {
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
    use super::{
        mailpit_value_allowed, mysql_service_config, path_disk_size, postgres_service_config,
        redis_service_config, selected_mysql_release, selected_postgres_release,
        selected_redis_release,
    };
    use devbox_core::{ConfigManager, ServiceConfig};
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

        assert!(path_disk_size(&root).unwrap() >= 18);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn redis_selected_version_is_loaded_from_instance_metadata() {
        let root =
            std::env::temp_dir().join(format!("zhiyu-redis-version-test-{}", std::process::id()));
        let release = devbox_core::installer::redis_release("6.2.23").unwrap();
        let mut config: ServiceConfig = redis_service_config(&root, release);
        config.version = "6.2".into();
        ConfigManager.save(&config).unwrap();

        assert_eq!(selected_redis_release(&root).version, "6.2.23");
        assert!(config.executable.ends_with("redis/6.2/bin/redis-server"));
        assert_eq!(
            config.arguments,
            vec![
                root.join("instances/redis/default/conf/redis.conf")
                    .display()
                    .to_string(),
                "--dir".into(),
                root.join("instances/redis/default/data/6.2")
                    .display()
                    .to_string(),
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn mysql_selected_version_is_loaded_from_instance_metadata() {
        let root =
            std::env::temp_dir().join(format!("zhiyu-mysql-version-test-{}", std::process::id()));
        let release = devbox_core::installer::mysql_release("8.0.45").unwrap();
        let config: ServiceConfig = mysql_service_config(&root, release);
        ConfigManager.save(&config).unwrap();

        assert_eq!(selected_mysql_release(&root).version, "8.0.45");
        assert!(config.executable.ends_with("mysql/8.0/bin/mysqld"));
        assert_eq!(
            config.arguments,
            vec![
                format!(
                    "--defaults-file={}",
                    root.join("instances/mysql/default/conf/my.cnf").display()
                ),
                format!(
                    "--basedir={}",
                    root.join("installations/mysql/8.0").display()
                ),
                format!(
                    "--datadir={}",
                    root.join("instances/mysql/default/data/8.0").display()
                ),
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn postgres_selected_version_is_loaded_from_instance_metadata() {
        let root = std::env::temp_dir().join(format!(
            "zhiyu-postgres-version-test-{}",
            std::process::id()
        ));
        let release = devbox_core::installer::postgres_release("16.14").unwrap();
        let config: ServiceConfig = postgres_service_config(&root, release);
        ConfigManager.save(&config).unwrap();

        assert_eq!(selected_postgres_release(&root).version, "16.14");
        assert!(config.executable.ends_with("postgres/16/bin/postgres"));
        assert_eq!(
            config.arguments,
            vec![
                "-D".to_string(),
                root.join("instances/postgres/default/data/16")
                    .display()
                    .to_string(),
                "-c".to_string(),
                format!(
                    "config_file={}",
                    root.join("instances/postgres/default/conf/postgresql.conf")
                        .display()
                ),
                "-c".to_string(),
                format!(
                    "data_directory={}",
                    root.join("instances/postgres/default/data/16").display()
                ),
            ]
        );
        fs::remove_dir_all(root).unwrap();
    }
}
