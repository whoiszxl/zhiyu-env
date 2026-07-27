use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;
use std::fs;

pub struct RabbitmqService {
    inner: ManagedService,
}

impl RabbitmqService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Rabbitmq {
            return Err(DevBoxError::InvalidConfig(
                "RabbitmqService requires kind=rabbitmq".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for RabbitmqService {
    fn install(&self) -> Result<()> {
        self.inner.install(
            "rabbitmq.conf",
            "# 智屿 RabbitMQ 本地开发配置\n\
             listeners.tcp.1 = 127.0.0.1:5672\n\
             management.tcp.ip = 127.0.0.1\n\
             management.tcp.port = 15672\n\
             default_user = zhiyu\n\
             default_pass = zhiyu-local-rabbitmq-2026\n\
             default_user_tags.administrator = true\n\
             log.console = true\n",
        )?;
        let plugins = self.inner.config.config_dir().join("enabled_plugins");
        if !plugins.exists() {
            fs::write(plugins, "[rabbitmq_management].\n")?;
        }
        Ok(())
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
