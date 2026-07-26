#![cfg(unix)]

use devbox_core::{
    MailpitService, MongodbService, MysqlService, PostgresService, RedisService, ServiceConfig,
    ServiceKind, ServiceManager, ServiceStatus,
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
