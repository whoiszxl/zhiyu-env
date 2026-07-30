use devbox_core::{
    installer::{
        activemq_release, caddy_release, consul_release, etcd_release, ftp_release,
        influxdb_release, mailpit_release, meilisearch_release, minio_release, mongodb_release,
        mysql_release, nats_release, nginx_release, postgres_release, rabbitmq_release,
        redis_release, rnacos_release, rustfs_release, MysqlRelease, NginxRelease, PostgresRelease,
        RedisRelease, VerifiedBinaryRelease, ACTIVEMQ_RELEASES, ACTIVEMQ_VERSION, CADDY_RELEASES,
        CADDY_VERSION, CONSUL_RELEASES, CONSUL_VERSION, ETCD_RELEASES, ETCD_VERSION, FTP_RELEASES,
        FTP_VERSION, INFLUXDB_RELEASES, INFLUXDB_VERSION, KAFKA_SERIES, KAFKA_VERSION,
        MAILPIT_RELEASES, MAILPIT_VERSION, MEILISEARCH_RELEASES, MEILISEARCH_VERSION,
        MINIO_RELEASES, MINIO_VERSION, MONGODB_RELEASES, MONGODB_VERSION, MYSQL_RELEASES,
        MYSQL_VERSION, NATS_RELEASES, NATS_VERSION, NGINX_RELEASES, NGINX_VERSION,
        POSTGRES_RELEASES, POSTGRES_VERSION, RABBITMQ_RELEASES, RABBITMQ_SERIES, RABBITMQ_VERSION,
        REDIS_RELEASES, REDIS_VERSION, RNACOS_RELEASES, RNACOS_VERSION, RUSTFS_RELEASES,
        RUSTFS_VERSION,
    },
    report_install_progress, with_install_context, ActivemqInstaller, ActivemqService,
    CaddyInstaller, CaddyService, ConfigManager, ConsulInstaller, ConsulService, EtcdInstaller,
    EtcdService, FtpInstaller, FtpService, InfluxdbInstaller, InfluxdbService,
    InstallCancellationToken, InstallReporter, KafkaInstaller, KafkaService, MailpitInstaller,
    MailpitService, MeilisearchInstaller, MeilisearchService, MinioInstaller, MinioService,
    MongodbInstaller, MongodbService, MysqlInstaller, MysqlService, NatsInstaller, NatsService,
    NginxInstaller, NginxService, PostgresInstaller, PostgresService, RabbitmqInstaller,
    RabbitmqService, RedisInstaller, RedisService, RnacosInstaller, RnacosService, RustfsInstaller,
    RustfsService, ServiceConfig, ServiceKind, ServiceManager, ServiceStatus,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

pub(crate) const INSTALL_PROGRESS_EVENT: &str = "install-progress";
#[derive(Clone)]
struct ActiveInstallTask {
    kind: String,
    cancellation: InstallCancellationToken,
}

static INSTALL_TASKS: OnceLock<Mutex<HashMap<String, ActiveInstallTask>>> = OnceLock::new();

fn install_tasks() -> &'static Mutex<HashMap<String, ActiveInstallTask>> {
    INSTALL_TASKS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn has_active_install_tasks() -> bool {
    !install_tasks()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .is_empty()
}

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
    let cancellation = InstallCancellationToken::default();
    {
        let mut tasks = install_tasks()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if tasks.values().any(|task| task.kind == kind) {
            return Err(format!("{kind} 正在安装，请等待当前任务结束"));
        }
        match tasks.entry(operation_id.clone()) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ActiveInstallTask {
                    kind: kind.clone(),
                    cancellation: cancellation.clone(),
                });
            }
            std::collections::hash_map::Entry::Occupied(_) => {
                return Err("安装任务标识正在使用".into());
            }
        }
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

    let result = with_install_context(reporter, cancellation.clone(), operation);
    install_tasks()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&operation_id);
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
        Err(error) => {
            let cancelled = cancellation.is_cancelled();
            emit_install_event(
                &app,
                InstallProgressEvent {
                    operation_id,
                    kind,
                    percent: None,
                    stage: if cancelled {
                        "安装已取消".into()
                    } else {
                        "安装失败".into()
                    },
                    message: if cancelled {
                        "安装任务已停止，可再次安装并从下载断点继续".into()
                    } else {
                        error.clone()
                    },
                    status: if cancelled { "cancelled" } else { "failed" },
                },
            )
        }
    }
    result
}

#[tauri::command]
pub fn service_install_cancel(operation_id: String) -> Result<(), String> {
    let tasks = install_tasks()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let task = tasks
        .get(&operation_id)
        .ok_or_else(|| "安装任务已经结束或不存在".to_string())?;
    task.cancellation.cancel();
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKindInput {
    Redis,
    Mysql,
    Postgres,
    Mongodb,
    Mailpit,
    Nats,
    Kafka,
    Meilisearch,
    Influxdb,
    Minio,
    Rustfs,
    Etcd,
    Consul,
    Rnacos,
    Rabbitmq,
    Activemq,
    Nginx,
    Caddy,
    Ftp,
}

impl From<ServiceKindInput> for ServiceKind {
    fn from(value: ServiceKindInput) -> Self {
        match value {
            ServiceKindInput::Redis => Self::Redis,
            ServiceKindInput::Mysql => Self::Mysql,
            ServiceKindInput::Postgres => Self::Postgres,
            ServiceKindInput::Mongodb => Self::Mongodb,
            ServiceKindInput::Mailpit => Self::Mailpit,
            ServiceKindInput::Nats => Self::Nats,
            ServiceKindInput::Kafka => Self::Kafka,
            ServiceKindInput::Meilisearch => Self::Meilisearch,
            ServiceKindInput::Influxdb => Self::Influxdb,
            ServiceKindInput::Minio => Self::Minio,
            ServiceKindInput::Rustfs => Self::Rustfs,
            ServiceKindInput::Etcd => Self::Etcd,
            ServiceKindInput::Consul => Self::Consul,
            ServiceKindInput::Rnacos => Self::Rnacos,
            ServiceKindInput::Rabbitmq => Self::Rabbitmq,
            ServiceKindInput::Activemq => Self::Activemq,
            ServiceKindInput::Nginx => Self::Nginx,
            ServiceKindInput::Caddy => Self::Caddy,
            ServiceKindInput::Ftp => Self::Ftp,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInfo {
    pub(crate) kind: ServiceKind,
    pub(crate) name: &'static str,
    pub(crate) version: String,
    pub(crate) port: u16,
    pub(crate) status: &'static str,
    pub(crate) pid: Option<u32>,
    install_supported: bool,
    install_support_label: String,
    platform_label: String,
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
pub struct EnvironmentMetrics {
    pub(crate) cpu_percent: f32,
    pub(crate) memory_bytes: u64,
    pub(crate) running_service_count: usize,
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
    installation_bytes: u64,
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
    installation_bytes: u64,
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
    installation_bytes: u64,
}

fn devbox_root() -> Result<PathBuf, String> {
    crate::settings::devbox_root()
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
        wait_for_port: true,
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
        wait_for_port: true,
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
        wait_for_port: true,
    }
}

fn selected_nginx_release(root: &Path) -> &'static NginxRelease {
    let metadata = root.join("instances/nginx/default/service.json");
    ConfigManager
        .load(metadata)
        .ok()
        .filter(|config| config.kind == ServiceKind::Nginx)
        .and_then(|config| nginx_release(&config.version))
        .unwrap_or_else(|| {
            nginx_release(NGINX_VERSION).expect("default Nginx release is registered")
        })
}

fn nginx_service_config(root: &Path, release: &NginxRelease) -> ServiceConfig {
    let instance = root.join("instances/nginx/default");
    ServiceConfig {
        name: "Nginx".into(),
        kind: ServiceKind::Nginx,
        version: release.version.into(),
        port: 8081,
        executable: root
            .join("installations/nginx")
            .join(release.series)
            .join("bin/nginx"),
        arguments: vec![
            "-p".into(),
            instance.display().to_string(),
            "-c".into(),
            "conf/nginx.conf".into(),
        ],
        environment: BTreeMap::new(),
        instance_dir: instance,
        wait_for_port: true,
    }
}

fn selected_verified_release(
    root: &Path,
    kind: ServiceKind,
    resolve: fn(&str) -> Option<&'static VerifiedBinaryRelease>,
    default_version: &str,
) -> &'static VerifiedBinaryRelease {
    let metadata = root
        .join("instances")
        .join(kind.as_str())
        .join("default/service.json");
    ConfigManager
        .load(metadata)
        .ok()
        .filter(|config| config.kind == kind)
        .and_then(|config| resolve(&config.version))
        .unwrap_or_else(|| resolve(default_version).expect("default release is registered"))
}

fn mailpit_service_config(root: &Path, release: &VerifiedBinaryRelease) -> ServiceConfig {
    let instance = root.join("instances/mailpit/default");
    ServiceConfig {
        name: "Mailpit".into(),
        kind: ServiceKind::Mailpit,
        version: release.version.into(),
        port: 1025,
        executable: root
            .join("installations/mailpit")
            .join(release.series)
            .join("bin/mailpit"),
        arguments: Vec::new(),
        environment: mailpit_environment(&instance),
        instance_dir: instance,
        wait_for_port: true,
    }
}

fn nats_service_config(root: &Path, release: &VerifiedBinaryRelease) -> ServiceConfig {
    let instance = root.join("instances/nats/default");
    ServiceConfig {
        name: "NATS".into(),
        kind: ServiceKind::Nats,
        version: release.version.into(),
        port: 4222,
        executable: root
            .join("installations/nats")
            .join(release.series)
            .join("bin/nats-server"),
        arguments: vec![
            "-c".into(),
            instance.join("conf/nats.conf").display().to_string(),
        ],
        environment: BTreeMap::new(),
        instance_dir: instance,
        wait_for_port: true,
    }
}

fn etcd_service_config(root: &Path, release: &VerifiedBinaryRelease) -> ServiceConfig {
    let instance = root.join("instances/etcd/default");
    ServiceConfig {
        name: "etcd".into(),
        kind: ServiceKind::Etcd,
        version: release.version.into(),
        port: 2379,
        executable: root
            .join("installations/etcd")
            .join(release.series)
            .join("bin/etcd"),
        arguments: vec![
            "--config-file".into(),
            instance.join("conf/etcd.yaml").display().to_string(),
        ],
        environment: BTreeMap::new(),
        instance_dir: instance,
        wait_for_port: true,
    }
}

fn caddy_service_config(root: &Path, release: &VerifiedBinaryRelease) -> ServiceConfig {
    let instance = root.join("instances/caddy/default");
    let gateway_port = fs::read(root.join("tools/local-domains.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|value| value.get("httpPort")?.as_u64())
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port > 0)
        .unwrap_or(8082);
    ServiceConfig {
        name: "Caddy".into(),
        kind: ServiceKind::Caddy,
        version: release.version.into(),
        port: gateway_port,
        executable: root
            .join("installations/caddy")
            .join(release.series)
            .join("bin/caddy"),
        arguments: vec![
            "run".into(),
            "--config".into(),
            instance.join("conf/Caddyfile").display().to_string(),
        ],
        environment: BTreeMap::new(),
        instance_dir: instance,
        wait_for_port: true,
    }
}

fn catalog_service_config(
    root: &Path,
    kind: ServiceKind,
    release: &VerifiedBinaryRelease,
) -> ServiceConfig {
    let instance = root.join("instances").join(kind.as_str()).join("default");
    let installation = root
        .join("installations")
        .join(kind.as_str())
        .join(release.series);
    let (name, port, executable, arguments) = match kind {
        ServiceKind::Mongodb => (
            "MongoDB",
            27017,
            installation.join("bin/mongod"),
            vec![
                "--config".into(),
                instance.join("conf/mongod.conf").display().to_string(),
            ],
        ),
        ServiceKind::Meilisearch => (
            "Meilisearch",
            7700,
            installation.join("bin/meilisearch"),
            vec![
                "--config-file-path".into(),
                instance.join("conf/meilisearch.toml").display().to_string(),
            ],
        ),
        ServiceKind::Influxdb => (
            "InfluxDB",
            8181,
            installation.join("bin/influxdb3"),
            vec![
                "serve".into(),
                "--node-id".into(),
                "zhiyu-local".into(),
                "--object-store".into(),
                "file".into(),
                "--data-dir".into(),
                instance
                    .join("data")
                    .join(release.series)
                    .display()
                    .to_string(),
                "--http-bind".into(),
                "127.0.0.1:8181".into(),
                "--without-auth".into(),
            ],
        ),
        ServiceKind::Minio => (
            "MinIO",
            9000,
            installation.join("bin/minio"),
            vec![
                "server".into(),
                instance.join("data").display().to_string(),
                "--address".into(),
                "127.0.0.1:9000".into(),
                "--console-address".into(),
                "127.0.0.1:9001".into(),
            ],
        ),
        ServiceKind::Rustfs => ("RustFS", 9002, installation.join("bin/rustfs"), Vec::new()),
        ServiceKind::Consul => (
            "Consul",
            8500,
            installation.join("bin/consul"),
            vec![
                "agent".into(),
                "-config-file".into(),
                instance.join("conf/consul.hcl").display().to_string(),
            ],
        ),
        ServiceKind::Rnacos => (
            "rnacos",
            8848,
            installation.join("bin/rnacos"),
            vec![
                "-e".into(),
                instance.join("conf/rnacos.env").display().to_string(),
            ],
        ),
        ServiceKind::Rabbitmq => (
            "RabbitMQ",
            5672,
            installation.join("server/sbin/rabbitmq-server"),
            Vec::new(),
        ),
        ServiceKind::Activemq => (
            "ActiveMQ Classic",
            61616,
            installation.join("home/bin/activemq"),
            vec!["console".into()],
        ),
        ServiceKind::Ftp => (
            "FTP Server",
            2121,
            installation.join("bin/sftpgo"),
            vec![
                "portable".into(),
                "--directory".into(),
                instance.join("data").display().to_string(),
                "--username".into(),
                "zhiyu".into(),
                "--password-file".into(),
                instance.join("conf/ftp.password").display().to_string(),
                "--permissions".into(),
                "*".into(),
                "--ftpd-port".into(),
                "2121".into(),
                "--sftpd-port".into(),
                "-1".into(),
                "--httpd-port".into(),
                "-1".into(),
                "--webdav-port".into(),
                "-1".into(),
                "--log-level".into(),
                "info".into(),
                "--grace-time".into(),
                "5".into(),
            ],
        ),
        _ => unreachable!("service does not use the shared binary catalog"),
    };
    let environment = match kind {
        ServiceKind::Minio => BTreeMap::from([
            ("MINIO_ROOT_USER".into(), "zhiyuadmin".into()),
            (
                "MINIO_ROOT_PASSWORD".into(),
                "zhiyu-local-minio-2026".into(),
            ),
            ("MINIO_BROWSER".into(), "on".into()),
        ]),
        ServiceKind::Rustfs => BTreeMap::from([
            ("RUSTFS_ACCESS_KEY".into(), "zhiyuadmin".into()),
            ("RUSTFS_SECRET_KEY".into(), "zhiyu-local-rustfs-2026".into()),
            (
                "RUSTFS_VOLUMES".into(),
                instance.join("data").display().to_string(),
            ),
            ("RUSTFS_ADDRESS".into(), "127.0.0.1:9002".into()),
            ("RUSTFS_CONSOLE_ENABLE".into(), "true".into()),
            ("RUSTFS_CONSOLE_ADDRESS".into(), "127.0.0.1:7001".into()),
        ]),
        ServiceKind::Rabbitmq => BTreeMap::from([
            (
                "ERLANG_HOME".into(),
                installation.join("otp").display().to_string(),
            ),
            (
                "PATH".into(),
                format!("{}:/usr/bin:/bin", installation.join("otp/bin").display()),
            ),
            (
                "RABBITMQ_HOME".into(),
                installation.join("server").display().to_string(),
            ),
            (
                "RABBITMQ_CONFIG_FILE".into(),
                instance.join("conf/rabbitmq").display().to_string(),
            ),
            (
                "RABBITMQ_ENABLED_PLUGINS_FILE".into(),
                instance.join("conf/enabled_plugins").display().to_string(),
            ),
            (
                "RABBITMQ_MNESIA_BASE".into(),
                instance.join("data").display().to_string(),
            ),
            (
                "RABBITMQ_LOG_BASE".into(),
                instance.join("logs").display().to_string(),
            ),
            (
                "RABBITMQ_PID_FILE".into(),
                instance.join("run/rabbitmq-node.pid").display().to_string(),
            ),
            ("RABBITMQ_NODENAME".into(), "rabbit@localhost".into()),
        ]),
        ServiceKind::Activemq => {
            let java_home = managed_java_home(root, release.series)
                .unwrap_or_else(|| root.join("runtimes/java/not-installed/home"));
            BTreeMap::from([
                ("JAVA_HOME".into(), java_home.display().to_string()),
                (
                    "PATH".into(),
                    format!("{}:/usr/bin:/bin", java_home.join("bin").display()),
                ),
                (
                    "ACTIVEMQ_HOME".into(),
                    installation.join("home").display().to_string(),
                ),
                ("ACTIVEMQ_BASE".into(), instance.display().to_string()),
                (
                    "ACTIVEMQ_CONF".into(),
                    instance.join("conf").display().to_string(),
                ),
                (
                    "ACTIVEMQ_DATA".into(),
                    instance.join("data").display().to_string(),
                ),
                (
                    "ACTIVEMQ_TMP".into(),
                    instance.join("tmp").display().to_string(),
                ),
                (
                    "ACTIVEMQ_OUT".into(),
                    instance.join("logs/activemq.out").display().to_string(),
                ),
                (
                    "ACTIVEMQ_PIDFILE".into(),
                    instance.join("run/activemq.pid").display().to_string(),
                ),
                ("ACTIVEMQ_OPTS_MEMORY".into(), "-Xms64M -Xmx256M".into()),
            ])
        }
        ServiceKind::Ftp => ftp_environment(&instance),
        _ => BTreeMap::new(),
    };
    ServiceConfig {
        name: name.into(),
        kind,
        version: release.version.into(),
        port,
        executable,
        arguments,
        environment,
        instance_dir: instance,
        wait_for_port: true,
    }
}

fn managed_java_home(root: &Path, activemq_series: &str) -> Option<PathBuf> {
    let accepts = |version: &str| {
        let major = version.split('.').next()?.parse::<u32>().ok()?;
        Some(if activemq_series == "6.3" {
            major == 25
        } else {
            matches!(major, 17 | 21)
        })
    };
    let settings = fs::read(root.join("runtime-profiles/settings.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    if let Some(version) = settings
        .as_ref()
        .and_then(|value| value.get("defaults"))
        .and_then(|value| value.get("java"))
        .and_then(|value| value.as_str())
        .filter(|version| accepts(version) == Some(true))
    {
        let home = root.join("runtimes/java").join(version).join("home");
        if home.join("bin/java").is_file() {
            return Some(home);
        }
    }
    let directory = root.join("runtimes/java");
    let mut candidates = fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|version| accepts(version) == Some(true))
                && path.join("home/bin/java").is_file()
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.pop().map(|path| path.join("home"))
}

pub(crate) fn activate_installed_version(
    kind: ServiceKindInput,
    version: &str,
) -> Result<(), String> {
    let root = devbox_root()?;
    let config = match kind {
        ServiceKindInput::Redis => redis_service_config(
            &root,
            redis_release(version).ok_or_else(|| format!("不支持 Redis 版本 {version}"))?,
        ),
        ServiceKindInput::Mysql => mysql_service_config(
            &root,
            mysql_release(version).ok_or_else(|| format!("不支持 MySQL 版本 {version}"))?,
        ),
        ServiceKindInput::Postgres => postgres_service_config(
            &root,
            postgres_release(version).ok_or_else(|| format!("不支持 PostgreSQL 版本 {version}"))?,
        ),
        ServiceKindInput::Nginx => nginx_service_config(
            &root,
            nginx_release(version).ok_or_else(|| format!("不支持 Nginx 版本 {version}"))?,
        ),
        ServiceKindInput::Mailpit => mailpit_service_config(
            &root,
            mailpit_release(version).ok_or_else(|| format!("不支持 Mailpit 版本 {version}"))?,
        ),
        ServiceKindInput::Nats => nats_service_config(
            &root,
            nats_release(version).ok_or_else(|| format!("不支持 NATS 版本 {version}"))?,
        ),
        ServiceKindInput::Etcd => etcd_service_config(
            &root,
            etcd_release(version).ok_or_else(|| format!("不支持 etcd 版本 {version}"))?,
        ),
        ServiceKindInput::Caddy => caddy_service_config(
            &root,
            caddy_release(version).ok_or_else(|| format!("不支持 Caddy 版本 {version}"))?,
        ),
        ServiceKindInput::Mongodb => catalog_service_config(
            &root,
            ServiceKind::Mongodb,
            mongodb_release(version).ok_or_else(|| format!("不支持 MongoDB 版本 {version}"))?,
        ),
        ServiceKindInput::Meilisearch => catalog_service_config(
            &root,
            ServiceKind::Meilisearch,
            meilisearch_release(version)
                .ok_or_else(|| format!("不支持 Meilisearch 版本 {version}"))?,
        ),
        ServiceKindInput::Influxdb => catalog_service_config(
            &root,
            ServiceKind::Influxdb,
            influxdb_release(version).ok_or_else(|| format!("不支持 InfluxDB 版本 {version}"))?,
        ),
        ServiceKindInput::Minio => catalog_service_config(
            &root,
            ServiceKind::Minio,
            minio_release(version).ok_or_else(|| format!("不支持 MinIO 版本 {version}"))?,
        ),
        ServiceKindInput::Rustfs => catalog_service_config(
            &root,
            ServiceKind::Rustfs,
            rustfs_release(version).ok_or_else(|| format!("不支持 RustFS 版本 {version}"))?,
        ),
        ServiceKindInput::Consul => catalog_service_config(
            &root,
            ServiceKind::Consul,
            consul_release(version).ok_or_else(|| format!("不支持 Consul 版本 {version}"))?,
        ),
        ServiceKindInput::Rnacos => catalog_service_config(
            &root,
            ServiceKind::Rnacos,
            rnacos_release(version).ok_or_else(|| format!("不支持 rnacos 版本 {version}"))?,
        ),
        ServiceKindInput::Rabbitmq => catalog_service_config(
            &root,
            ServiceKind::Rabbitmq,
            rabbitmq_release(version).ok_or_else(|| format!("不支持 RabbitMQ 版本 {version}"))?,
        ),
        ServiceKindInput::Activemq => catalog_service_config(
            &root,
            ServiceKind::Activemq,
            activemq_release(version).ok_or_else(|| format!("不支持 ActiveMQ 版本 {version}"))?,
        ),
        ServiceKindInput::Ftp => catalog_service_config(
            &root,
            ServiceKind::Ftp,
            ftp_release(version).ok_or_else(|| format!("不支持 FTP Server 版本 {version}"))?,
        ),
        _ => return Err("当前服务不支持版本切换".into()),
    };
    ConfigManager.save(&config).map_err(stringify_error)
}

pub(crate) fn service_config(kind: ServiceKind) -> Result<ServiceConfig, String> {
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
    if kind == ServiceKind::Nginx {
        return Ok(nginx_service_config(&root, selected_nginx_release(&root)));
    }
    if kind == ServiceKind::Mailpit {
        let release = selected_verified_release(&root, kind, mailpit_release, MAILPIT_VERSION);
        return Ok(mailpit_service_config(&root, release));
    }
    if kind == ServiceKind::Nats {
        let release = selected_verified_release(&root, kind, nats_release, NATS_VERSION);
        return Ok(nats_service_config(&root, release));
    }
    if kind == ServiceKind::Etcd {
        let release = selected_verified_release(&root, kind, etcd_release, ETCD_VERSION);
        return Ok(etcd_service_config(&root, release));
    }
    if kind == ServiceKind::Caddy {
        let release = selected_verified_release(&root, kind, caddy_release, CADDY_VERSION);
        return Ok(caddy_service_config(&root, release));
    }
    let catalog: Option<(
        fn(&str) -> Option<&'static VerifiedBinaryRelease>,
        &'static str,
    )> = match kind {
        ServiceKind::Mongodb => Some((
            mongodb_release as fn(&str) -> Option<&'static VerifiedBinaryRelease>,
            MONGODB_VERSION,
        )),
        ServiceKind::Meilisearch => Some((meilisearch_release, MEILISEARCH_VERSION)),
        ServiceKind::Influxdb => Some((influxdb_release, INFLUXDB_VERSION)),
        ServiceKind::Minio => Some((minio_release, MINIO_VERSION)),
        ServiceKind::Rustfs => Some((rustfs_release, RUSTFS_VERSION)),
        ServiceKind::Consul => Some((consul_release, CONSUL_VERSION)),
        ServiceKind::Rnacos => Some((rnacos_release, RNACOS_VERSION)),
        ServiceKind::Rabbitmq => Some((rabbitmq_release, RABBITMQ_VERSION)),
        ServiceKind::Activemq => Some((activemq_release, ACTIVEMQ_VERSION)),
        ServiceKind::Ftp => Some((ftp_release, FTP_VERSION)),
        _ => None,
    };
    if let Some((resolve, default_version)) = catalog {
        let release = selected_verified_release(&root, kind, resolve, default_version);
        return Ok(catalog_service_config(&root, kind, release));
    }
    let (name, version, port, executable, arguments) = match kind {
        ServiceKind::Redis => unreachable!(),
        ServiceKind::Mysql => unreachable!(),
        ServiceKind::Postgres => unreachable!(),
        ServiceKind::Nginx => unreachable!(),
        ServiceKind::Caddy
        | ServiceKind::Mailpit
        | ServiceKind::Nats
        | ServiceKind::Etcd
        | ServiceKind::Mongodb
        | ServiceKind::Meilisearch
        | ServiceKind::Influxdb
        | ServiceKind::Minio
        | ServiceKind::Rustfs
        | ServiceKind::Consul
        | ServiceKind::Rnacos
        | ServiceKind::Rabbitmq => {
            unreachable!()
        }
        ServiceKind::Activemq => unreachable!(),
        ServiceKind::Ftp => unreachable!(),
        ServiceKind::Kafka => {
            let instance = root.join("instances/kafka/default");
            (
                "Kafka Sandbox",
                KAFKA_VERSION,
                9092,
                root.join(format!("installations/kafka/{KAFKA_SERIES}/bin/tansu")),
                vec![
                    "broker".into(),
                    "--cluster-id".into(),
                    "zhiyu-local".into(),
                    "--listener-url".into(),
                    "tcp://127.0.0.1:9092".into(),
                    "--advertised-listener-url".into(),
                    "tcp://127.0.0.1:9092".into(),
                    "--storage-engine".into(),
                    format!("sqlite://{}", instance.join("data/tansu.db").display()),
                ],
            )
        }
    };

    let instance_dir = root.join("instances").join(kind.as_str()).join("default");
    let environment = match kind {
        ServiceKind::Mailpit => mailpit_environment(&instance_dir),
        ServiceKind::Minio => BTreeMap::from([
            ("MINIO_ROOT_USER".into(), "zhiyuadmin".into()),
            (
                "MINIO_ROOT_PASSWORD".into(),
                "zhiyu-local-minio-2026".into(),
            ),
            ("MINIO_BROWSER".into(), "on".into()),
        ]),
        ServiceKind::Rustfs => BTreeMap::from([
            ("RUSTFS_ACCESS_KEY".into(), "zhiyuadmin".into()),
            ("RUSTFS_SECRET_KEY".into(), "zhiyu-local-rustfs-2026".into()),
            (
                "RUSTFS_VOLUMES".into(),
                instance_dir.join("data").display().to_string(),
            ),
            ("RUSTFS_ADDRESS".into(), "127.0.0.1:9002".into()),
            ("RUSTFS_CONSOLE_ENABLE".into(), "true".into()),
            ("RUSTFS_CONSOLE_ADDRESS".into(), "127.0.0.1:7001".into()),
        ]),
        ServiceKind::Rabbitmq => {
            let installation = root.join("installations/rabbitmq").join(RABBITMQ_SERIES);
            BTreeMap::from([
                (
                    "ERLANG_HOME".into(),
                    installation.join("otp").display().to_string(),
                ),
                (
                    "PATH".into(),
                    format!("{}:/usr/bin:/bin", installation.join("otp/bin").display()),
                ),
                (
                    "RABBITMQ_HOME".into(),
                    installation.join("server").display().to_string(),
                ),
                (
                    "RABBITMQ_CONFIG_FILE".into(),
                    instance_dir.join("conf/rabbitmq").display().to_string(),
                ),
                (
                    "RABBITMQ_ENABLED_PLUGINS_FILE".into(),
                    instance_dir
                        .join("conf/enabled_plugins")
                        .display()
                        .to_string(),
                ),
                (
                    "RABBITMQ_MNESIA_BASE".into(),
                    instance_dir.join("data").display().to_string(),
                ),
                (
                    "RABBITMQ_LOG_BASE".into(),
                    instance_dir.join("logs").display().to_string(),
                ),
                (
                    "RABBITMQ_PID_FILE".into(),
                    instance_dir
                        .join("run/rabbitmq-node.pid")
                        .display()
                        .to_string(),
                ),
                ("RABBITMQ_NODENAME".into(), "rabbit@localhost".into()),
            ])
        }
        ServiceKind::Kafka => BTreeMap::from([
            ("RUST_LOG".into(), "tansu=info".into()),
            ("NO_COLOR".into(), "1".into()),
        ]),
        _ => BTreeMap::new(),
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
        wait_for_port: true,
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

fn ftp_environment(instance_dir: &Path) -> BTreeMap<String, String> {
    const ALLOWED_KEYS: &[&str] = &[
        "SFTPGO_FTPD__BINDINGS__0__ADDRESS",
        "SFTPGO_FTPD__PASSIVE_PORT_RANGE__START",
        "SFTPGO_FTPD__PASSIVE_PORT_RANGE__END",
        "SFTPGO_FTPD__DISABLE_ACTIVE_MODE",
    ];
    let mut environment = BTreeMap::from([
        (
            "SFTPGO_FTPD__BINDINGS__0__ADDRESS".into(),
            "127.0.0.1".into(),
        ),
        (
            "SFTPGO_FTPD__PASSIVE_PORT_RANGE__START".into(),
            "50000".into(),
        ),
        (
            "SFTPGO_FTPD__PASSIVE_PORT_RANGE__END".into(),
            "50009".into(),
        ),
        ("SFTPGO_FTPD__DISABLE_ACTIVE_MODE".into(), "true".into()),
    ]);
    if let Ok(contents) = fs::read_to_string(instance_dir.join("conf/ftp.env")) {
        for line in contents.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            let valid = match key {
                "SFTPGO_FTPD__BINDINGS__0__ADDRESS" => value == "127.0.0.1",
                "SFTPGO_FTPD__PASSIVE_PORT_RANGE__START" => value == "50000",
                "SFTPGO_FTPD__PASSIVE_PORT_RANGE__END" => value == "50009",
                "SFTPGO_FTPD__DISABLE_ACTIVE_MODE" => value == "true",
                _ => false,
            };
            if valid && ALLOWED_KEYS.contains(&key) {
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
        ServiceKindInput::Nats => operation(&NatsService::new(config).map_err(stringify_error)?),
        ServiceKindInput::Kafka => operation(&KafkaService::new(config).map_err(stringify_error)?),
        ServiceKindInput::Meilisearch => {
            operation(&MeilisearchService::new(config).map_err(stringify_error)?)
        }
        ServiceKindInput::Influxdb => {
            operation(&InfluxdbService::new(config).map_err(stringify_error)?)
        }
        ServiceKindInput::Minio => operation(&MinioService::new(config).map_err(stringify_error)?),
        ServiceKindInput::Rustfs => {
            operation(&RustfsService::new(config).map_err(stringify_error)?)
        }
        ServiceKindInput::Etcd => operation(&EtcdService::new(config).map_err(stringify_error)?),
        ServiceKindInput::Consul => {
            operation(&ConsulService::new(config).map_err(stringify_error)?)
        }
        ServiceKindInput::Rnacos => {
            operation(&RnacosService::new(config).map_err(stringify_error)?)
        }
        ServiceKindInput::Rabbitmq => {
            operation(&RabbitmqService::new(config).map_err(stringify_error)?)
        }
        ServiceKindInput::Activemq => {
            operation(&ActivemqService::new(config).map_err(stringify_error)?)
        }
        ServiceKindInput::Nginx => operation(&NginxService::new(config).map_err(stringify_error)?),
        ServiceKindInput::Caddy => operation(&CaddyService::new(config).map_err(stringify_error)?),
        ServiceKindInput::Ftp => operation(&FtpService::new(config).map_err(stringify_error)?),
    }
    .map_err(stringify_error)
}

pub(crate) fn service_status(kind: ServiceKindInput) -> Result<ServiceStatus, String> {
    with_service(kind, |service| service.status())
}

pub(crate) fn repair_service(kind: ServiceKindInput) -> Result<(), String> {
    with_service(kind, |service| service.repair())
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
        ServiceStatus::Crashed { pid } => ("crashed", Some(pid)),
    }
}

pub(crate) fn service_info(kind: ServiceKindInput) -> Result<ServiceInfo, String> {
    let config = service_config(kind.into())?;
    let status = with_service(kind, |service| service.status())?;
    let (status, pid) = status_parts(status);
    let name = match kind {
        ServiceKindInput::Redis => "Redis",
        ServiceKindInput::Mysql => "MySQL",
        ServiceKindInput::Postgres => "PostgreSQL",
        ServiceKindInput::Mongodb => "MongoDB",
        ServiceKindInput::Mailpit => "Mailpit",
        ServiceKindInput::Nats => "NATS",
        ServiceKindInput::Kafka => "Kafka Sandbox",
        ServiceKindInput::Meilisearch => "Meilisearch",
        ServiceKindInput::Influxdb => "InfluxDB",
        ServiceKindInput::Minio => "MinIO",
        ServiceKindInput::Rustfs => "RustFS",
        ServiceKindInput::Etcd => "etcd",
        ServiceKindInput::Consul => "Consul",
        ServiceKindInput::Rnacos => "rnacos",
        ServiceKindInput::Rabbitmq => "RabbitMQ",
        ServiceKindInput::Activemq => "ActiveMQ Classic",
        ServiceKindInput::Nginx => "Nginx",
        ServiceKindInput::Caddy => "Caddy",
        ServiceKindInput::Ftp => "FTP Server",
    };
    let (install_supported, install_support_label) = install_compatibility(kind);
    Ok(ServiceInfo {
        kind: kind.into(),
        name,
        version: config.version.clone(),
        port: config.port,
        status,
        pid,
        install_supported,
        install_support_label,
        platform_label: platform_label(),
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

fn platform_label() -> String {
    let os = match std::env::consts::OS {
        "macos" => "macOS",
        "windows" => "Windows",
        "linux" => "Linux",
        value => value,
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" if std::env::consts::OS == "macos" => "Apple Silicon",
        "aarch64" => "ARM64",
        "x86_64" if std::env::consts::OS == "macos" => "Intel",
        "x86_64" => "x64",
        value => value,
    };
    format!("{os} · {arch}")
}

fn install_compatibility(kind: ServiceKindInput) -> (bool, String) {
    let platform = platform_label();
    let supported = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        true
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        matches!(
            kind,
            ServiceKindInput::Redis
                | ServiceKindInput::Postgres
                | ServiceKindInput::Nginx
                | ServiceKindInput::Caddy
        )
    } else {
        false
    };
    let label = if supported {
        format!("支持当前平台：{platform}")
    } else {
        format!("当前版本暂不支持在 {platform} 自动安装")
    };
    (supported, label)
}

fn ensure_install_compatible(kind: ServiceKindInput) -> Result<(), String> {
    let (supported, label) = install_compatibility(kind);
    if supported {
        Ok(())
    } else {
        Err(label)
    }
}

fn run_action(
    kind: ServiceKindInput,
    action: impl FnOnce(&dyn ServiceManager) -> Result<(), devbox_core::DevBoxError>,
) -> Result<ServiceInfo, String> {
    with_service(kind, action)?;
    service_info(kind)
}

#[derive(Clone, Copy)]
pub(crate) enum LifecycleAction {
    Start,
    Stop,
    Restart,
}

pub(crate) fn lifecycle_action(
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
        ServiceKindInput::Nats,
        ServiceKindInput::Kafka,
        ServiceKindInput::Meilisearch,
        ServiceKindInput::Influxdb,
        ServiceKindInput::Minio,
        ServiceKindInput::Rustfs,
        ServiceKindInput::Etcd,
        ServiceKindInput::Consul,
        ServiceKindInput::Rnacos,
        ServiceKindInput::Rabbitmq,
        ServiceKindInput::Activemq,
        ServiceKindInput::Nginx,
        ServiceKindInput::Caddy,
        ServiceKindInput::Ftp,
    ]
    .into_iter()
    .map(service_info)
    .collect()
}

fn install_service(kind: ServiceKindInput) -> Result<ServiceInfo, String> {
    ensure_install_compatible(kind)?;
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
            service_info(kind)
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
            service_info(kind)
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
            service_info(kind)
        }
        ServiceKindInput::Mongodb => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Mongodb)?;
            MongodbInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 MongoDB 实例配置");
            MongodbService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Mailpit => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Mailpit)?;
            MailpitInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 Mailpit 实例配置");
            MailpitService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Nats => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Nats)?;
            NatsInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 NATS 实例配置");
            NatsService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Kafka => {
            KafkaInstaller::new(devbox_root()?)
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 Kafka Sandbox 实例配置");
            run_action(kind, |service| service.install())
        }
        ServiceKindInput::Meilisearch => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Meilisearch)?;
            MeilisearchInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 Meilisearch 实例配置");
            MeilisearchService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Influxdb => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Influxdb)?;
            InfluxdbInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 InfluxDB 实例配置");
            InfluxdbService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Minio => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Minio)?;
            MinioInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 MinIO 实例配置");
            MinioService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Rustfs => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Rustfs)?;
            RustfsInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 RustFS 实例配置");
            RustfsService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Etcd => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Etcd)?;
            EtcdInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 etcd 实例配置");
            EtcdService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Consul => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Consul)?;
            ConsulInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 Consul 实例配置");
            ConsulService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Rnacos => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Rnacos)?;
            RnacosInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 rnacos 实例配置");
            RnacosService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Rabbitmq => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Rabbitmq)?;
            RabbitmqInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 RabbitMQ 实例配置");
            RabbitmqService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Activemq => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Activemq)?;
            ActivemqInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 ActiveMQ 实例配置");
            ActivemqService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Nginx => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Nginx)?;
            NginxInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 Nginx 实例配置");
            NginxService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }

        ServiceKindInput::Caddy => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Caddy)?;
            CaddyInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 Caddy 实例配置");
            CaddyService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
        }
        ServiceKindInput::Ftp => {
            let root = devbox_root()?;
            let config = service_config(ServiceKind::Ftp)?;
            FtpInstaller::for_version(&root, &config.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 FTP Server 实例配置");
            FtpService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(kind)
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
                    installation_bytes: path_disk_size(&installer.installation_dir())
                        .unwrap_or_default(),
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
            ensure_install_compatible(ServiceKindInput::Redis)?;
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
            service_info(ServiceKindInput::Redis)
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
                    installation_bytes: path_disk_size(&installer.installation_dir())
                        .unwrap_or_default(),
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
            ensure_install_compatible(ServiceKindInput::Mysql)?;
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
            service_info(ServiceKindInput::Mysql)
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
                    installation_bytes: path_disk_size(&installer.installation_dir())
                        .unwrap_or_default(),
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
            ensure_install_compatible(ServiceKindInput::Postgres)?;
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
            service_info(ServiceKindInput::Postgres)
        })
    })
    .await
    .map_err(|error| format!("PostgreSQL 版本切换任务异常结束: {error}"))?
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NginxVersionInfo {
    pub series: &'static str,
    pub version: &'static str,
    pub installed: bool,
    pub selected: bool,
    pub support_label: &'static str,
    pub legacy: bool,
    pub recommended: bool,
    pub installation_bytes: u64,
}

#[tauri::command]
pub async fn nginx_versions() -> Result<Vec<NginxVersionInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let root = devbox_root()?;
        let selected = selected_nginx_release(&root);
        NGINX_RELEASES
            .iter()
            .map(|release| {
                let installer =
                    NginxInstaller::for_version(&root, release.version).map_err(stringify_error)?;
                Ok(NginxVersionInfo {
                    series: release.series,
                    version: release.version,
                    installed: installer.is_installed(),
                    selected: release.version == selected.version,
                    support_label: release.support_label,
                    legacy: release.legacy,
                    recommended: release.recommended,
                    installation_bytes: path_disk_size(&installer.installation_dir())
                        .unwrap_or_default(),
                })
            })
            .collect()
    })
    .await
    .map_err(|error| format!("Nginx 版本列表任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn nginx_version_select(
    app: AppHandle,
    version: String,
    operation_id: String,
) -> Result<ServiceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_install_task(app, operation_id, "nginx".into(), || {
            ensure_install_compatible(ServiceKindInput::Nginx)?;
            let root = devbox_root()?;
            let release =
                nginx_release(&version).ok_or_else(|| format!("不支持 Nginx 版本 {version}"))?;
            if matches!(
                service_status(ServiceKindInput::Nginx)?,
                ServiceStatus::Running { .. }
            ) {
                return Err("请先停止 Nginx，再切换运行版本".into());
            }
            let installer =
                NginxInstaller::for_version(&root, release.version).map_err(stringify_error)?;
            installer.install().map_err(stringify_error)?;
            report_install_progress(94, "写入配置", "正在创建 Nginx 实例配置");
            let config = nginx_service_config(&root, release);
            NginxService::new(config)
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
            service_info(ServiceKindInput::Nginx)
        })
    })
    .await
    .map_err(|error| format!("Nginx 版本切换任务异常结束: {error}"))?
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedServiceVersionInfo {
    pub series: &'static str,
    pub version: &'static str,
    pub installed: bool,
    pub selected: bool,
    pub support_label: &'static str,
    pub legacy: bool,
    pub recommended: bool,
    pub installation_bytes: u64,
}

fn verified_releases(kind: ServiceKindInput) -> Result<&'static [VerifiedBinaryRelease], String> {
    match kind {
        ServiceKindInput::Mailpit => Ok(MAILPIT_RELEASES),
        ServiceKindInput::Nats => Ok(NATS_RELEASES),
        ServiceKindInput::Etcd => Ok(ETCD_RELEASES),
        ServiceKindInput::Caddy => Ok(CADDY_RELEASES),
        ServiceKindInput::Mongodb => Ok(MONGODB_RELEASES),
        ServiceKindInput::Meilisearch => Ok(MEILISEARCH_RELEASES),
        ServiceKindInput::Influxdb => Ok(INFLUXDB_RELEASES),
        ServiceKindInput::Minio => Ok(MINIO_RELEASES),
        ServiceKindInput::Rustfs => Ok(RUSTFS_RELEASES),
        ServiceKindInput::Consul => Ok(CONSUL_RELEASES),
        ServiceKindInput::Rnacos => Ok(RNACOS_RELEASES),
        ServiceKindInput::Rabbitmq => Ok(RABBITMQ_RELEASES),
        ServiceKindInput::Activemq => Ok(ACTIVEMQ_RELEASES),
        ServiceKindInput::Ftp => Ok(FTP_RELEASES),
        _ => Err("当前服务暂未接入通用多版本管理".into()),
    }
}

fn verified_service_name(kind: ServiceKindInput) -> &'static str {
    match kind {
        ServiceKindInput::Mailpit => "Mailpit",
        ServiceKindInput::Nats => "NATS",
        ServiceKindInput::Etcd => "etcd",
        ServiceKindInput::Caddy => "Caddy",
        ServiceKindInput::Mongodb => "MongoDB",
        ServiceKindInput::Meilisearch => "Meilisearch",
        ServiceKindInput::Influxdb => "InfluxDB",
        ServiceKindInput::Minio => "MinIO",
        ServiceKindInput::Rustfs => "RustFS",
        ServiceKindInput::Consul => "Consul",
        ServiceKindInput::Rnacos => "rnacos",
        ServiceKindInput::Rabbitmq => "RabbitMQ",
        ServiceKindInput::Activemq => "ActiveMQ",
        ServiceKindInput::Ftp => "FTP Server",
        _ => "服务",
    }
}

fn verified_installer_state(
    root: &Path,
    kind: ServiceKindInput,
    version: &str,
) -> Result<(bool, PathBuf), String> {
    macro_rules! state {
        ($installer:ty) => {{
            let installer = <$installer>::for_version(root, version).map_err(stringify_error)?;
            (installer.is_installed(), installer.installation_dir())
        }};
    }
    Ok(match kind {
        ServiceKindInput::Mailpit => state!(MailpitInstaller),
        ServiceKindInput::Nats => state!(NatsInstaller),
        ServiceKindInput::Etcd => state!(EtcdInstaller),
        ServiceKindInput::Caddy => state!(CaddyInstaller),
        ServiceKindInput::Mongodb => state!(MongodbInstaller),
        ServiceKindInput::Meilisearch => state!(MeilisearchInstaller),
        ServiceKindInput::Influxdb => state!(InfluxdbInstaller),
        ServiceKindInput::Minio => state!(MinioInstaller),
        ServiceKindInput::Rustfs => state!(RustfsInstaller),
        ServiceKindInput::Consul => state!(ConsulInstaller),
        ServiceKindInput::Rnacos => state!(RnacosInstaller),
        ServiceKindInput::Rabbitmq => state!(RabbitmqInstaller),
        ServiceKindInput::Activemq => state!(ActivemqInstaller),
        ServiceKindInput::Ftp => state!(FtpInstaller),
        _ => return Err("当前服务暂未接入通用多版本管理".into()),
    })
}

#[tauri::command]
pub async fn service_versions(
    kind: ServiceKindInput,
) -> Result<Vec<ManagedServiceVersionInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let root = devbox_root()?;
        let selected = service_config(kind.into())?.version;
        verified_releases(kind)?
            .iter()
            .map(|release| {
                let (installed, directory) =
                    verified_installer_state(&root, kind, release.version)?;
                Ok(ManagedServiceVersionInfo {
                    series: release.series,
                    version: release.version,
                    installed,
                    selected: release.version == selected,
                    support_label: release.support_label,
                    legacy: release.legacy,
                    recommended: release.recommended,
                    installation_bytes: path_disk_size(&directory).unwrap_or_default(),
                })
            })
            .collect()
    })
    .await
    .map_err(|error| format!("服务版本列表任务异常结束: {error}"))?
}

fn install_verified_version(
    root: &Path,
    kind: ServiceKindInput,
    version: &str,
) -> Result<ServiceInfo, String> {
    let release = verified_releases(kind)?
        .iter()
        .find(|release| release.version == version || release.series == version)
        .ok_or_else(|| format!("不支持 {} 版本 {version}", verified_service_name(kind)))?;
    match kind {
        ServiceKindInput::Mailpit => {
            MailpitInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            MailpitService::new(mailpit_service_config(root, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        ServiceKindInput::Nats => {
            NatsInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            NatsService::new(nats_service_config(root, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        ServiceKindInput::Etcd => {
            EtcdInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            EtcdService::new(etcd_service_config(root, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        ServiceKindInput::Caddy => {
            CaddyInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            CaddyService::new(caddy_service_config(root, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        ServiceKindInput::Mongodb => {
            MongodbInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            MongodbService::new(catalog_service_config(root, ServiceKind::Mongodb, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        ServiceKindInput::Meilisearch => {
            MeilisearchInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            MeilisearchService::new(catalog_service_config(
                root,
                ServiceKind::Meilisearch,
                release,
            ))
            .and_then(|service| service.install())
            .map_err(stringify_error)?;
        }
        ServiceKindInput::Influxdb => {
            InfluxdbInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            InfluxdbService::new(catalog_service_config(root, ServiceKind::Influxdb, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        ServiceKindInput::Minio => {
            MinioInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            MinioService::new(catalog_service_config(root, ServiceKind::Minio, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        ServiceKindInput::Rustfs => {
            RustfsInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            RustfsService::new(catalog_service_config(root, ServiceKind::Rustfs, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        ServiceKindInput::Consul => {
            ConsulInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            ConsulService::new(catalog_service_config(root, ServiceKind::Consul, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        ServiceKindInput::Rnacos => {
            RnacosInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            RnacosService::new(catalog_service_config(root, ServiceKind::Rnacos, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        ServiceKindInput::Rabbitmq => {
            RabbitmqInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            RabbitmqService::new(catalog_service_config(root, ServiceKind::Rabbitmq, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        ServiceKindInput::Activemq => {
            ActivemqInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            ActivemqService::new(catalog_service_config(root, ServiceKind::Activemq, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        ServiceKindInput::Ftp => {
            FtpInstaller::for_version(root, release.version)
                .map_err(stringify_error)?
                .install()
                .map_err(stringify_error)?;
            FtpService::new(catalog_service_config(root, ServiceKind::Ftp, release))
                .and_then(|service| service.install())
                .map_err(stringify_error)?;
        }
        _ => return Err("当前服务暂未接入通用多版本管理".into()),
    }
    service_info(kind)
}

#[tauri::command]
pub async fn service_version_select(
    app: AppHandle,
    kind: ServiceKindInput,
    version: String,
    operation_id: String,
) -> Result<ServiceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_install_task(
            app,
            operation_id,
            ServiceKind::from(kind).as_str().into(),
            || {
                ensure_install_compatible(kind)?;
                if matches!(service_status(kind)?, ServiceStatus::Running { .. }) {
                    return Err(format!(
                        "请先停止 {}，再切换运行版本",
                        verified_service_name(kind)
                    ));
                }
                let root = devbox_root()?;
                report_install_progress(4, "准备版本", "正在验证目标版本和运行状态");
                install_verified_version(&root, kind, &version)
            },
        )
    })
    .await
    .map_err(|error| format!("服务版本切换任务异常结束: {error}"))?
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
pub async fn service_force_stop(kind: ServiceKindInput) -> Result<ServiceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || run_action(kind, |service| service.force_stop()))
        .await
        .map_err(|error| format!("强制停止任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn service_repair(kind: ServiceKindInput) -> Result<ServiceInfo, String> {
    tauri::async_runtime::spawn_blocking(move || run_action(kind, |service| service.repair()))
        .await
        .map_err(|error| format!("修复服务状态任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn service_stop_all() -> Result<Vec<ServiceInfo>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let mut failures = Vec::new();
        for kind in [
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
            ServiceKindInput::Activemq,
            ServiceKindInput::Nginx,
            ServiceKindInput::Caddy,
            ServiceKindInput::Ftp,
        ] {
            match with_service(kind, |service| service.status()) {
                Ok(ServiceStatus::Running { .. }) => {
                    if let Err(error) = with_service(kind, |service| service.stop()) {
                        failures.push(format!("{}: {error}", ServiceKind::from(kind).as_str()));
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    failures.push(format!("{}: {error}", ServiceKind::from(kind).as_str()))
                }
            }
        }
        if failures.is_empty() {
            service_list()
        } else {
            Err(format!("部分服务停止失败：{}", failures.join("；")))
        }
    })
    .await
    .map_err(|error| format!("停止全部服务任务异常结束: {error}"))?
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
pub async fn environment_metrics() -> Result<EnvironmentMetrics, String> {
    tauri::async_runtime::spawn_blocking(collect_environment_metrics)
        .await
        .map_err(|error| format!("总内存统计任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn environment_disk_usage() -> Result<u64, String> {
    tauri::async_runtime::spawn_blocking(|| devbox_root().and_then(|root| path_disk_size(&root)))
        .await
        .map_err(|error| format!("总磁盘统计任务异常结束: {error}"))?
}

fn nginx_html_dir() -> Result<PathBuf, String> {
    let root = devbox_root()?;
    Ok(root.join("instances/nginx/default/html"))
}

fn nginx_html_path(relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty() || relative.contains("..") || relative.starts_with('/') {
        return Err("无效的文件路径".into());
    }
    let base = nginx_html_dir()?;
    let resolved = base.join(relative);
    let canonical = resolved
        .canonicalize()
        .map_err(|_| "路径不存在".to_string())?;
    if !canonical.starts_with(&base.canonicalize().unwrap_or_else(|_| base.clone())) {
        return Err("路径越界".into());
    }
    Ok(canonical)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NginxFileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size_bytes: u64,
}

#[tauri::command]
pub fn nginx_html_list(subdir: Option<String>) -> Result<Vec<NginxFileEntry>, String> {
    let dir = if let Some(ref sub) = subdir {
        nginx_html_path(sub)?
    } else {
        nginx_html_dir()?
    };
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if let Some(ref sub) = subdir {
            format!("{sub}/{name}")
        } else {
            name.clone()
        };
        entries.push(NginxFileEntry {
            name,
            path: rel,
            is_dir: meta.is_dir(),
            size_bytes: meta.len(),
        });
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(entries)
}

#[tauri::command]
pub fn nginx_html_read(path: String) -> Result<String, String> {
    let resolved = nginx_html_path(&path)?;
    let meta = resolved.metadata().map_err(|e| e.to_string())?;
    if meta.is_dir() {
        return Err("不能读取目录".into());
    }
    if meta.len() > 512 * 1024 {
        return Err("文件超过 512 KiB，不适合在线编辑".into());
    }
    String::from_utf8(std::fs::read(&resolved).map_err(|e| e.to_string())?)
        .map_err(|_| "文件不是有效的 UTF-8 文本".into())
}

#[tauri::command]
pub fn nginx_html_write(path: String, content: String) -> Result<(), String> {
    if content.len() > 512 * 1024 {
        return Err("内容不能超过 512 KiB".into());
    }
    let base = nginx_html_dir()?;
    if path.contains("..") || path.starts_with('/') {
        return Err("无效的文件路径".into());
    }
    let resolved = base.join(&path);
    if let Some(parent) = resolved.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = resolved.with_extension("html.tmp");
    std::fs::write(&tmp, content.as_bytes()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &resolved).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn nginx_html_delete(path: String) -> Result<(), String> {
    let resolved = nginx_html_path(&path)?;
    let meta = resolved.metadata().map_err(|e| e.to_string())?;
    if meta.is_dir() {
        std::fs::remove_dir_all(&resolved).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(&resolved).map_err(|e| e.to_string())
    }
}

fn caddy_html_dir() -> Result<PathBuf, String> {
    let root = devbox_root()?;
    Ok(root.join("instances/caddy/default/html"))
}

fn caddy_html_path(relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty() || relative.contains("..") || relative.starts_with('/') {
        return Err("无效的文件路径".into());
    }
    let base = caddy_html_dir()?;
    let resolved = base.join(relative);
    let canonical = resolved
        .canonicalize()
        .map_err(|_| "路径不存在".to_string())?;
    if !canonical.starts_with(&base.canonicalize().unwrap_or_else(|_| base.clone())) {
        return Err("路径越界".into());
    }
    Ok(canonical)
}

#[tauri::command]
pub fn caddy_html_list(subdir: Option<String>) -> Result<Vec<NginxFileEntry>, String> {
    let dir = if let Some(ref sub) = subdir {
        caddy_html_path(sub)?
    } else {
        caddy_html_dir()?
    };
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let meta = entry.metadata().map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if let Some(ref sub) = subdir {
            format!("{sub}/{name}")
        } else {
            name.clone()
        };
        entries.push(NginxFileEntry {
            name,
            path: rel,
            is_dir: meta.is_dir(),
            size_bytes: meta.len(),
        });
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(entries)
}

#[tauri::command]
pub fn caddy_html_read(path: String) -> Result<String, String> {
    let resolved = caddy_html_path(&path)?;
    let meta = resolved.metadata().map_err(|e| e.to_string())?;
    if meta.is_dir() {
        return Err("不能读取目录".into());
    }
    if meta.len() > 512 * 1024 {
        return Err("文件超过 512 KiB".into());
    }
    String::from_utf8(std::fs::read(&resolved).map_err(|e| e.to_string())?)
        .map_err(|_| "文件不是有效的 UTF-8 文本".into())
}

#[tauri::command]
pub fn caddy_html_write(path: String, content: String) -> Result<(), String> {
    if content.len() > 512 * 1024 {
        return Err("内容不能超过 512 KiB".into());
    }
    let base = caddy_html_dir()?;
    if path.contains("..") || path.starts_with('/') {
        return Err("无效的文件路径".into());
    }
    let resolved = base.join(&path);
    if let Some(parent) = resolved.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = resolved.with_extension("html.tmp");
    std::fs::write(&tmp, content.as_bytes()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &resolved).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn caddy_html_delete(path: String) -> Result<(), String> {
    let resolved = caddy_html_path(&path)?;
    let meta = resolved.metadata().map_err(|e| e.to_string())?;
    if meta.is_dir() {
        std::fs::remove_dir_all(&resolved).map_err(|e| e.to_string())
    } else {
        std::fs::remove_file(&resolved).map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("只允许打开 HTTP 或 HTTPS 链接".into());
    }
    std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|error| format!("无法打开浏览器: {error}"))?;
    Ok(())
}

#[tauri::command]
pub fn open_path(path: String) -> Result<(), String> {
    let p = PathBuf::from(&path);
    let base = devbox_root()?;
    let canonical = p.canonicalize().map_err(|_| "路径不存在".to_string())?;
    if !canonical.starts_with(&base.canonicalize().unwrap_or_else(|_| base.clone())) {
        return Err("不允许打开 devbox 之外的路径".into());
    }
    std::process::Command::new("open")
        .arg(&canonical)
        .spawn()
        .map_err(|error| format!("无法打开: {error}"))?;
    Ok(())
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

    let temporary = path.with_extension("tmp");
    fs::write(&temporary, content.clone()).map_err(|error| error.to_string())?;

    if matches!(config.kind, ServiceKind::Nginx) {
        let output = std::process::Command::new(&config.executable)
            .args([
                "-t",
                "-c",
                &temporary.display().to_string(),
                "-p",
                &config.instance_dir.display().to_string(),
            ])
            .output()
            .map_err(|error| format!("无法验证 Nginx 配置：{error}"))?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr);
            let max_len = 8 * 1024;
            let message = if detail.chars().count() > max_len {
                format!(
                    "配置校验失败：{}…",
                    detail.chars().take(max_len - 6).collect::<String>()
                )
            } else {
                format!("配置校验失败：{detail}")
            };
            let _ = fs::remove_file(&temporary);
            return Err(message);
        }
    }

    if path.is_file() {
        fs::copy(&path, path.with_extension("bak")).map_err(|error| error.to_string())?;
    }
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
        ServiceKind::Redis
        | ServiceKind::Postgres
        | ServiceKind::Mailpit
        | ServiceKind::Nats
        | ServiceKind::Kafka
        | ServiceKind::Meilisearch
        | ServiceKind::Influxdb
        | ServiceKind::Minio => {}
        ServiceKind::Rustfs => {}
        ServiceKind::Etcd => {}
        ServiceKind::Consul => {}
        ServiceKind::Rnacos => {}
        ServiceKind::Rabbitmq => {}
        ServiceKind::Activemq => {}
        ServiceKind::Nginx => {}
        ServiceKind::Caddy => {}
        ServiceKind::Ftp => {}
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
        ServiceKind::Redis
        | ServiceKind::Postgres
        | ServiceKind::Mailpit
        | ServiceKind::Nats
        | ServiceKind::Kafka
        | ServiceKind::Meilisearch
        | ServiceKind::Influxdb
        | ServiceKind::Minio => config.stdout_log_path(),
        ServiceKind::Rustfs => config.stdout_log_path(),
        ServiceKind::Etcd => config.stdout_log_path(),
        ServiceKind::Consul => config.stdout_log_path(),
        ServiceKind::Rnacos => config.stdout_log_path(),
        ServiceKind::Rabbitmq => config.stdout_log_path(),
        ServiceKind::Activemq => config.stdout_log_path(),
        ServiceKind::Nginx => config.stdout_log_path(),
        ServiceKind::Caddy => config.stdout_log_path(),
        ServiceKind::Ftp => config.stdout_log_path(),
    }
}

pub(crate) fn native_config_path(config: &ServiceConfig) -> PathBuf {
    let name = match config.kind {
        ServiceKind::Redis => "redis.conf",
        ServiceKind::Mysql => "my.cnf",
        ServiceKind::Postgres => "postgresql.conf",
        ServiceKind::Mongodb => "mongod.conf",
        ServiceKind::Mailpit => "mailpit.env",
        ServiceKind::Nats => "nats.conf",
        ServiceKind::Kafka => "kafka.conf",
        ServiceKind::Meilisearch => "meilisearch.toml",
        ServiceKind::Influxdb => "influxdb.env",
        ServiceKind::Minio => "minio.env",
        ServiceKind::Rustfs => "rustfs.env",
        ServiceKind::Etcd => "etcd.yaml",
        ServiceKind::Consul => "consul.hcl",
        ServiceKind::Rnacos => "rnacos.env",
        ServiceKind::Rabbitmq => "rabbitmq.conf",
        ServiceKind::Activemq => "activemq.xml",
        ServiceKind::Nginx => "nginx.conf",
        ServiceKind::Caddy => "Caddyfile",
        ServiceKind::Ftp => "ftp.env",
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

pub(crate) fn collect_environment_metrics() -> Result<EnvironmentMetrics, String> {
    let services = service_list()?;
    collect_environment_metrics_from(&services)
}

pub(crate) fn collect_environment_metrics_from(
    services: &[ServiceInfo],
) -> Result<EnvironmentMetrics, String> {
    let running_service_count = services
        .iter()
        .filter(|service| service.status == "running")
        .count();
    let mut pids = BTreeSet::from([std::process::id()]);
    pids.extend(services.iter().filter_map(|service| {
        (service.status == "running")
            .then_some(service.pid)
            .flatten()
    }));
    let pid_list = pids
        .into_iter()
        .map(|pid| pid.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let output = Command::new("/bin/ps")
        .args(["-p", &pid_list, "-o", "rss=", "-o", "%cpu="])
        .output()
        .map_err(|error| format!("无法读取智屿进程指标: {error}"))?;
    if !output.status.success() {
        return Err("无法读取智屿进程指标".into());
    }

    let (memory_bytes, cpu_percent) = sum_process_metrics(&String::from_utf8_lossy(&output.stdout));
    Ok(EnvironmentMetrics {
        cpu_percent,
        memory_bytes,
        running_service_count,
    })
}

fn sum_process_metrics(output: &str) -> (u64, f32) {
    output
        .lines()
        .fold((0_u64, 0_f32), |(memory_bytes, cpu_percent), line| {
            let mut values = line.split_whitespace();
            let rss_kib = values
                .next()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or_default();
            let process_cpu = values
                .next()
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or_default();
            (
                memory_bytes.saturating_add(rss_kib.saturating_mul(1024)),
                cpu_percent + process_cpu,
            )
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
        ServiceStatus::Stopped | ServiceStatus::StalePid { .. } | ServiceStatus::Crashed { .. } => {
            Ok(config.instance_dir)
        }
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
        ServiceKind::Nats => "nats-server",
        ServiceKind::Kafka => "tansu-",
        ServiceKind::Meilisearch => "meilisearch",
        ServiceKind::Influxdb => "influxdb3-core-",
        ServiceKind::Minio => "minio.",
        ServiceKind::Rustfs => "rustfs-",
        ServiceKind::Etcd => "etcd-",
        ServiceKind::Consul => "consul_",
        ServiceKind::Rnacos => "rnacos-",
        ServiceKind::Rabbitmq => "rabbitmq-",
        ServiceKind::Activemq => "apache-activemq-",
        ServiceKind::Nginx => "nginx",
        ServiceKind::Caddy => "caddy",
        ServiceKind::Ftp => "sftpgo_",
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
        install_compatibility, mailpit_value_allowed, mysql_service_config, path_disk_size,
        platform_label, postgres_service_config, redis_service_config, selected_mysql_release,
        selected_postgres_release, selected_redis_release, sum_process_metrics, ServiceKindInput,
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
    fn process_metrics_are_summed_and_rss_is_converted_from_kibibytes() {
        assert_eq!(
            sum_process_metrics(" 1024 1.5\n 2048 2.25\n"),
            (3 * 1024 * 1024, 3.75)
        );
        assert_eq!(sum_process_metrics(""), (0, 0.0));
    }

    #[test]
    fn installer_reports_the_current_platform_and_support_state() {
        let platform = platform_label();
        assert!(!platform.is_empty());
        let (supported, label) = install_compatibility(ServiceKindInput::Redis);
        assert!(label.contains(&platform));
        if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            assert!(supported);
        }
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

#[tauri::command]
pub async fn service_test_connection(kind: ServiceKindInput) -> Result<(), String> {
    let kind: ServiceKind = kind.into();
    let (addr, timeout_secs) = connection_target(&kind);
    tauri::async_runtime::spawn_blocking(move || {
        let socket = addr
            .to_socket_addrs()
            .map_err(|e| format!("地址解析失败: {e}"))?
            .next()
            .ok_or_else(|| "地址无可用 IP".to_string())?;
        TcpStream::connect_timeout(&socket, Duration::from_secs(timeout_secs))
            .map_err(|e| format!("无法连接 {addr}: {e}"))?;
        Ok(())
    })
    .await
    .map_err(|e| format!("连接测试异常: {e}"))?
}

fn connection_target(kind: &ServiceKind) -> (String, u64) {
    match kind {
        ServiceKind::Redis => ("127.0.0.1:6379".into(), 3),
        ServiceKind::Mysql => ("127.0.0.1:3306".into(), 5),
        ServiceKind::Postgres => ("127.0.0.1:5432".into(), 5),
        ServiceKind::Mongodb => ("127.0.0.1:27017".into(), 5),
        ServiceKind::Mailpit => ("127.0.0.1:1025".into(), 3),
        ServiceKind::Nats => ("127.0.0.1:4222".into(), 3),
        ServiceKind::Kafka => ("127.0.0.1:9092".into(), 3),
        ServiceKind::Meilisearch => ("127.0.0.1:7700".into(), 3),
        ServiceKind::Influxdb => ("127.0.0.1:8181".into(), 5),
        ServiceKind::Minio => ("127.0.0.1:9000".into(), 3),
        ServiceKind::Rustfs => ("127.0.0.1:9002".into(), 3),
        ServiceKind::Etcd => ("127.0.0.1:2379".into(), 3),
        ServiceKind::Consul => ("127.0.0.1:8500".into(), 3),
        ServiceKind::Rnacos => ("127.0.0.1:8848".into(), 3),
        ServiceKind::Rabbitmq => ("127.0.0.1:5672".into(), 5),
        ServiceKind::Activemq => ("127.0.0.1:61616".into(), 8),
        ServiceKind::Nginx => ("127.0.0.1:8081".into(), 3),
        ServiceKind::Caddy => ("127.0.0.1:8082".into(), 3),
        ServiceKind::Ftp => ("127.0.0.1:2121".into(), 3),
    }
}
