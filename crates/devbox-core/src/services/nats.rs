use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct NatsService {
    inner: ManagedService,
}

impl NatsService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Nats {
            return Err(DevBoxError::InvalidConfig(
                "NatsService requires kind=nats".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for NatsService {
    fn install(&self) -> Result<()> {
        let config = &self.inner.config;
        let contents = format!(
            "# 智屿 NATS 本地开发配置\n\
             server_name: zhiyu-local\n\
             host: 127.0.0.1\n\
             port: 4222\n\
             http: 127.0.0.1:8222\n\
             max_payload: 8MB\n\
             jetstream {{\n\
               store_dir: \"{}\"\n\
               max_memory_store: 128MB\n\
               max_file_store: 1GB\n\
             }}\n",
            config.data_dir().display(),
        );
        self.inner.install("nats.conf", &contents)
    }

    fn start(&self) -> Result<u32> {
        self.inner.start()
    }

    fn stop(&self) -> Result<()> {
        self.inner.stop()
    }

    fn force_stop(&self) -> Result<()> {
        self.inner.force_stop()
    }

    fn restart(&self) -> Result<u32> {
        self.inner.restart()
    }

    fn status(&self) -> Result<ServiceStatus> {
        self.inner.status()
    }

    fn repair(&self) -> Result<()> {
        self.inner.repair()
    }
}
