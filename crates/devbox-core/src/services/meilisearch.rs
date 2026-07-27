use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct MeilisearchService {
    inner: ManagedService,
}

impl MeilisearchService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Meilisearch {
            return Err(DevBoxError::InvalidConfig(
                "MeilisearchService requires kind=meilisearch".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for MeilisearchService {
    fn install(&self) -> Result<()> {
        let config = &self.inner.config;
        let contents = format!(
            "# 智屿 Meilisearch 本地开发配置\n\
             env = \"development\"\n\
             http_addr = \"127.0.0.1:7700\"\n\
             db_path = \"{}\"\n\
             dump_dir = \"{}\"\n\
             snapshot_dir = \"{}\"\n\
             no_analytics = true\n",
            config.data_dir().join("db").display(),
            config.data_dir().join("dumps").display(),
            config.data_dir().join("snapshots").display(),
        );
        self.inner.install("meilisearch.toml", &contents)
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
