use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct MongodbService {
    inner: ManagedService,
}

impl MongodbService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Mongodb {
            return Err(DevBoxError::InvalidConfig(
                "MongodbService requires kind=mongodb".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for MongodbService {
    fn install(&self) -> Result<()> {
        let config = &self.inner.config;
        let contents = format!(
            "storage:\n  dbPath: {}\nsystemLog:\n  destination: file\n  path: {}\n  logAppend: true\nnet:\n  bindIp: 127.0.0.1\n  port: {}\nprocessManagement:\n  fork: false\n",
            config.data_dir().display(),
            config.logs_dir().join("mongodb.log").display(),
            config.port,
        );
        self.inner.install("mongod.conf", &contents)
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
