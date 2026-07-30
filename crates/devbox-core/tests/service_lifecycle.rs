#![cfg(unix)]

use devbox_core::{
    ActivemqService, ConsulService, EtcdService, FtpService, InfluxdbService, KafkaService,
    MailpitService, MeilisearchService, MinioService, MongodbService, MysqlService, NatsService,
    PostgresService, RabbitmqService, RedisService, RnacosService, RustfsService, ServiceConfig,
    ServiceKind, ServiceManager, ServiceStatus,
};
use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Arc, Barrier};
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
        wait_for_port: false,
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
fn kafka_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = KafkaService::new(config(temp.path(), ServiceKind::Kafka)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("kafka/conf/kafka.conf").is_file());
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
fn influxdb_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = InfluxdbService::new(config(temp.path(), ServiceKind::Influxdb)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("influxdb/conf/influxdb.env").is_file());
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
fn consul_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = ConsulService::new(config(temp.path(), ServiceKind::Consul)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("consul/conf/consul.hcl").is_file());
}

#[test]
fn rnacos_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = RnacosService::new(config(temp.path(), ServiceKind::Rnacos)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("rnacos/conf/rnacos.env").is_file());
}

#[test]
fn rabbitmq_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = RabbitmqService::new(config(temp.path(), ServiceKind::Rabbitmq)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("rabbitmq/conf/rabbitmq.conf").is_file());
    assert!(temp.path().join("rabbitmq/conf/enabled_plugins").is_file());
}

#[test]
fn activemq_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("activemq-home");
    let java_home = temp.path().join("java-home");
    std::fs::create_dir_all(home.join("conf")).unwrap();
    std::fs::create_dir_all(java_home.join("bin")).unwrap();
    std::fs::write(
        home.join("conf/activemq.xml"),
        r#"<transportConnector uri="tcp://0.0.0.0:61616"/><storeUsage limit="100 gb"/><tempUsage limit="50 gb"/>"#,
    )
    .unwrap();
    std::fs::write(java_home.join("bin/java"), b"test").unwrap();

    let mut service_config = config(temp.path(), ServiceKind::Activemq);
    service_config
        .environment
        .insert("ACTIVEMQ_HOME".into(), home.display().to_string());
    service_config
        .environment
        .insert("JAVA_HOME".into(), java_home.display().to_string());
    let service = ActivemqService::new(service_config).unwrap();

    assert_lifecycle(&service);
    let broker = std::fs::read_to_string(temp.path().join("activemq/conf/activemq.xml")).unwrap();
    assert!(broker.contains("tcp://127.0.0.1:61616"));
    assert!(broker.contains("storeUsage limit=\"2 gb\""));
}

#[test]
fn ftp_service_lifecycle() {
    let temp = TempDir::new().unwrap();
    let service = FtpService::new(config(temp.path(), ServiceKind::Ftp)).unwrap();
    assert_lifecycle(&service);
    assert!(temp.path().join("ftp/conf/ftp.env").is_file());
    let password = temp.path().join("ftp/conf/ftp.password");
    assert_eq!(
        std::fs::read_to_string(&password).unwrap(),
        "zhiyu-local-ftp-2026"
    );
    assert_eq!(
        std::fs::metadata(password).unwrap().permissions().mode() & 0o777,
        0o600
    );
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
fn concurrent_start_only_creates_one_process() {
    let temp = TempDir::new().unwrap();
    let service = Arc::new(RedisService::new(config(temp.path(), ServiceKind::Redis)).unwrap());
    service.install().unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let handles = (0..2)
        .map(|_| {
            let service = Arc::clone(&service);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                service.start()
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| result
                .as_ref()
                .is_err_and(|error| error.to_string().contains("操作正在执行")))
            .count(),
        1
    );
    service.stop().unwrap();
}

#[test]
fn pid_identity_mismatch_is_never_treated_as_managed() {
    let temp = TempDir::new().unwrap();
    let service_config = config(temp.path(), ServiceKind::Redis);
    let service = RedisService::new(service_config.clone()).unwrap();
    service.install().unwrap();
    let pid = std::process::id();
    let record = serde_json::json!({
        "pid": pid,
        "executable": service_config.executable,
        "start_marker": "a different process start time",
    });
    std::fs::write(
        service_config.pid_path(),
        serde_json::to_vec_pretty(&record).unwrap(),
    )
    .unwrap();

    assert!(!matches!(
        service.status().unwrap(),
        ServiceStatus::Running { .. }
    ));
    assert!(service_config.anomaly_path().is_file());
    assert!(service.stop().is_err());
    assert!(unsafe { libc::kill(pid as libc::pid_t, 0) } == 0);
}

#[test]
fn unexpected_exit_survives_restart_until_repaired() {
    let temp = TempDir::new().unwrap();
    let service_config = config(temp.path(), ServiceKind::Redis);
    let service = RedisService::new(service_config.clone()).unwrap();
    service.install().unwrap();
    let pid = service.start().unwrap();
    unsafe {
        libc::kill(pid as libc::pid_t, libc::SIGKILL);
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    let status = loop {
        let status = service.status().unwrap();
        if matches!(
            status,
            ServiceStatus::Crashed { pid: crashed_pid } if crashed_pid == pid
        ) || Instant::now() >= deadline
        {
            break status;
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(status, ServiceStatus::Crashed { pid });

    let reopened = RedisService::new(service_config).unwrap();
    assert_eq!(reopened.status().unwrap(), ServiceStatus::Crashed { pid });
    reopened.repair().unwrap();
    assert_eq!(reopened.status().unwrap(), ServiceStatus::Stopped);
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
