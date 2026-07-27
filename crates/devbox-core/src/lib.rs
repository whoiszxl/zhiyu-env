pub mod config;
pub mod error;
pub mod installer;
pub mod process;
pub mod service;
pub mod services;
pub mod status;

pub use config::{ConfigManager, ServiceConfig, ServiceKind};
pub use error::{DevBoxError, Result};
pub use installer::{
    report_install_progress, with_install_reporter, ConsulInstaller, DuckdbInstaller,
    EtcdInstaller, InstallOutcome, InstallReporter, InstallUpdate, MailpitInstaller,
    MeilisearchInstaller, MinioInstaller, MongodbInstaller, MysqlInstaller, NatsInstaller,
    PostgresInstaller, RabbitmqInstaller, RedisInstaller, RnacosInstaller, RustfsInstaller,
};
pub use process::ProcessManager;
pub use service::ServiceManager;
pub use services::{
    ConsulService, EtcdService, MailpitService, MeilisearchService, MinioService, MongodbService,
    MysqlService, NatsService, PostgresService, RabbitmqService, RedisService, RnacosService,
    RustfsService,
};
pub use status::ServiceStatus;
