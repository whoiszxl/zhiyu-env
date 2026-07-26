use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct PostgresService {
    inner: ManagedService,
}

impl PostgresService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Postgres {
            return Err(DevBoxError::InvalidConfig(
                "PostgresService requires kind=postgres".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for PostgresService {
    fn install(&self) -> Result<()> {
        let config = &self.inner.config;
        let contents = format!(
            "listen_addresses = '127.0.0.1'\nport = {}\ndata_directory = '{}'\nunix_socket_directories = '{}'\n",
            config.port,
            config.data_dir().display(),
            config.run_dir().display(),
        );
        self.inner.install("postgresql.conf", &contents)
    }

    fn start(&self) -> Result<u32> {
        self.inner.start()
    }

    fn stop(&self) -> Result<()> {
        self.inner.stop()
    }

    fn restart(&self) -> Result<u32> {
        self.inner.restart()
    }

    fn status(&self) -> Result<ServiceStatus> {
        self.inner.status()
    }
}
