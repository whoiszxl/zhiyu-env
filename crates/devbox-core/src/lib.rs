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
    report_install_progress, with_install_reporter, DuckdbInstaller, InstallOutcome,
    InstallReporter, InstallUpdate, MailpitInstaller, MongodbInstaller, MysqlInstaller,
    PostgresInstaller, RedisInstaller,
};
pub use process::ProcessManager;
pub use service::ServiceManager;
pub use services::{MailpitService, MongodbService, MysqlService, PostgresService, RedisService};
pub use status::ServiceStatus;
