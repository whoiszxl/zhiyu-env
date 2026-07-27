use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct MinioService {
    inner: ManagedService,
}

impl MinioService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Minio {
            return Err(DevBoxError::InvalidConfig(
                "MinioService requires kind=minio".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for MinioService {
    fn install(&self) -> Result<()> {
        self.inner.install(
            "minio.env",
            "# 智屿 MinIO 本地开发配置\n\
             MINIO_ROOT_USER=zhiyuadmin\n\
             MINIO_ROOT_PASSWORD=zhiyu-local-minio-2026\n\
             MINIO_BROWSER=on\n",
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
