use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct RustfsService {
    inner: ManagedService,
}

impl RustfsService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Rustfs {
            return Err(DevBoxError::InvalidConfig(
                "RustfsService requires kind=rustfs".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for RustfsService {
    fn install(&self) -> Result<()> {
        self.inner.install(
            "rustfs.env",
            "# 智屿 RustFS 本地开发配置\n\
             RUSTFS_ACCESS_KEY=zhiyuadmin\n\
             RUSTFS_SECRET_KEY=zhiyu-local-rustfs-2026\n\
             RUSTFS_ADDRESS=127.0.0.1:9002\n\
             RUSTFS_CONSOLE_ENABLE=true\n\
             RUSTFS_CONSOLE_ADDRESS=127.0.0.1:7001\n",
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
