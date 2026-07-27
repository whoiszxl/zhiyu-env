#![cfg(unix)]

use devbox_core::{
    EtcdService, MailpitService, MeilisearchService, MinioService, MongodbService, MysqlService,
    NatsService, PostgresService, RedisService, RustfsService, ServiceConfig, ServiceKind,
    ServiceManager, ServiceStatus,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn config(root: &Path, kind: ServiceKind) -> ServiceConfig {
    ServiceConfig {
        name: format!("{}-test", kind.as_str()),
        kind,
        version: "test".into(),
        port: 16_000,
        executable: "/bin/sleep".into(),
        arguments: vec!["30".into()],
        environment: BTreeMap::new(),
        instance_dir: root.join(kind.as_str()),
    }
}

fn assert_lifecycle(service: &dyn ServiceManager) {
    assert_eq!(service.status().unwrap(), ServiceStatus::NotInstalled);

    service.install().unwrap();
    assert_eq!(service.status().unwrap(), ServiceStatus::Stopped);

    let first_pid = service.start().unwrap();
    assert_eq!(
        service.status().unwrap(),
        ServiceStatus::Running { pid: first_pid }
    );

    let second_pid = service.restart().unwrap();
    assert_ne!(first_pid, second_pid);
    assert_eq!(
        service.status().unwrap(),
        ServiceStatus::Running { pid: second_pid }
    );

    service.stop().unwrap();
    assert_eq!(service.status().unwrap(), ServiceStatus::Stopped);
}

#[test]
fn redis_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = RedisService::new(config(temp.path(), ServiceKind::Redis)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("redis/conf/redis.conf").is_file());
}

#[test]
fn mysql_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = MysqlService::new(config(temp.path(), ServiceKind::Mysql)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("mysql/conf/my.cnf").is_file());
}

#[test]
fn postgres_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = PostgresService::new(config(temp.path(), ServiceKind::Postgres)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("postgres/conf/postgresql.conf").is_file());
}

#[test]
fn mongodb_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = MongodbService::new(config(temp.path(), ServiceKind::Mongodb)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("mongodb/conf/mongod.conf").is_file());
}

#[test]
fn mailpit_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = MailpitService::new(config(temp.path(), ServiceKind::Mailpit)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("mailpit/conf/mailpit.env").is_file());
}

#[test]
fn nats_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = NatsService::new(config(temp.path(), ServiceKind::Nats)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("nats/conf/nats.conf").is_file());
}

#[test]
fn meilisearch_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = MeilisearchService::new(config(temp.path(), ServiceKind::Meilisearch)).unwrap();
    assert_lifecycle(&service);
    assert!(temp
        .path()
        .join("meilisearch/conf/meilisearch.toml")
        .is_file());
}

#[test]
fn minio_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = MinioService::new(config(temp.path(), ServiceKind::Minio)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("minio/conf/minio.env").is_file());
}

#[test]
fn rustfs_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = RustfsService::new(config(temp.path(), ServiceKind::Rustfs)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("rustfs/conf/rustfs.env").is_file());
}

#[test]
fn etcd_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = EtcdService::new(config(temp.path(), ServiceKind::Etcd)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("etcd/conf/etcd.yaml").is_file());
}

#[test]
fn stale_pid_is_detected() {
    let temp = TempDir::new().unwrap();
    let service = RedisService::new(config(temp.path(), ServiceKind::Redis)).unwrap();
    service.install().unwrap();
    std::fs::write(temp.path().join("redis/run/service.pid"), "999999").unwrap();

    assert_eq!(
        service.status().unwrap(),
        ServiceStatus::StalePid { pid: 999_999 }
    );
}

#[test]
fn start_rejects_missing_executable() {
    let temp = TempDir::new().unwrap();
    let mut service_config = config(temp.path(), ServiceKind::Redis);
    service_config.executable = temp.path().join("missing");
    let service = RedisService::new(service_config).unwrap();
    service.install().unwrap();

    assert!(service.start().is_err());
    thread::sleep(Duration::from_millis(10));
}

#[test]
fn start_reports_a_process_that_exits_immediately() {
    let temp = TempDir::new().unwrap();
    let mut service_config = config(temp.path(), ServiceKind::Redis);
    service_config.executable = "/usr/bin/false".into();
    service_config.arguments.clear();
    let service = RedisService::new(service_config.clone()).unwrap();
    service.install().unwrap();

    let error = service.start().unwrap_err().to_string();
    assert!(error.contains("进程在启动阶段退出"));
    assert!(!service_config.pid_path().exists());
}

#[test]
fn redis_legacy_data_is_moved_to_the_selected_version() {
    let temp = TempDir::new().unwrap();
    let mut service_config = config(temp.path(), ServiceKind::Redis);
    let data_root = service_config.data_dir();
    let version_data = data_root.join("7.2");
    service_config.arguments = vec![
        service_config
            .config_dir()
            .join("redis.conf")
            .display()
            .to_string(),
        "--dir".into(),
        version_data.display().to_string(),
    ];
    std::fs::create_dir_all(&data_root).unwrap();
    std::fs::write(data_root.join("dump.rdb"), b"existing redis data").unwrap();

    let service = RedisService::new(service_config).unwrap();
    service.install().unwrap();

    assert_eq!(service.data_dir(), version_data);
    assert!(!data_root.join("dump.rdb").exists());
    assert_eq!(
        std::fs::read(version_data.join("dump.rdb")).unwrap(),
        b"existing redis data"
    );
}

#[test]
fn mysql_legacy_data_is_moved_to_the_selected_version() {
    let temp = TempDir::new().unwrap();
    let mut service_config = config(temp.path(), ServiceKind::Mysql);
    let data_root = service_config.data_dir();
    let version_data = data_root.join("8.4");
    service_config.arguments = vec![
        format!(
            "--defaults-file={}",
            service_config.config_dir().join("my.cnf").display()
        ),
        format!("--datadir={}", version_data.display()),
    ];
    std::fs::create_dir_all(data_root.join("mysql")).unwrap();
    std::fs::write(data_root.join("auto.cnf"), b"existing mysql data").unwrap();

    let service = MysqlService::new(service_config).unwrap();
    service.install().unwrap();

    assert_eq!(service.data_dir(), version_data);
    assert!(!data_root.join("auto.cnf").exists());
    assert!(version_data.join("mysql").is_dir());
    assert_eq!(
        std::fs::read(version_data.join("auto.cnf")).unwrap(),
        b"existing mysql data"
    );
}

#[test]
fn postgres_legacy_data_is_moved_to_the_selected_version() {
    let temp = TempDir::new().unwrap();
    let mut service_config = config(temp.path(), ServiceKind::Postgres);
    let data_root = service_config.data_dir();
    let version_data = data_root.join("17");
    service_config.arguments = vec![
        "-D".into(),
        version_data.display().to_string(),
        "-c".into(),
        format!(
            "config_file={}",
            service_config
                .config_dir()
                .join("postgresql.conf")
                .display()
        ),
    ];
    std::fs::create_dir_all(&data_root).unwrap();
    std::fs::write(data_root.join("PG_VERSION"), b"17").unwrap();

    let service = PostgresService::new(service_config).unwrap();
    service.install().unwrap();

    assert_eq!(service.data_dir(), version_data);
    assert!(!data_root.join("PG_VERSION").exists());
    assert_eq!(
        std::fs::read(version_data.join("PG_VERSION")).unwrap(),
        b"17"
    );
}

#[test]
fn a_new_manager_can_stop_and_reap_an_existing_child() {
    let temp = TempDir::new().unwrap();
    let service_config = config(temp.path(), ServiceKind::Redis);

    let first_manager = RedisService::new(service_config.clone()).unwrap();
    first_manager.install().unwrap();
    first_manager.start().unwrap();
    drop(first_manager);

    let second_manager = RedisService::new(service_config).unwrap();
    let started = Instant::now();
    second_manager.stop().unwrap();

    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(second_manager.status().unwrap(), ServiceStatus::Stopped);
}
