use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct ConsulService {
    inner: ManagedService,
}

impl ConsulService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Consul {
            return Err(DevBoxError::InvalidConfig(
                "ConsulService requires kind=consul".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for ConsulService {
    fn install(&self) -> Result<()> {
        let data_dir = self.inner.config.data_dir();
        self.inner.install(
            "consul.hcl",
            &format!(
                "# 智屿 Consul 单节点开发配置\n\
                 server = true\n\
                 bootstrap_expect = 1\n\
                 data_dir = \"{}\"\n\
                 bind_addr = \"127.0.0.1\"\n\
                 client_addr = \"127.0.0.1\"\n\
                 disable_update_check = true\n\
                 log_level = \"INFO\"\n\
                 ui_config {{\n\
                   enabled = true\n\
                 }}\n",
                data_dir.display()
            ),
        )
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
