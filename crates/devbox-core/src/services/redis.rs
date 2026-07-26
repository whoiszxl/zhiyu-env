use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct RedisService {
    inner: ManagedService,
}

impl RedisService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Redis {
            return Err(DevBoxError::InvalidConfig(
                "RedisService requires kind=redis".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for RedisService {
    fn install(&self) -> Result<()> {
        let config = &self.inner.config;
        let contents = format!(
            "bind 127.0.0.1\nport {}\ndir {}\ndaemonize no\n",
            config.port,
            config.data_dir().display()
        );
        self.inner.install("redis.conf", &contents)
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
