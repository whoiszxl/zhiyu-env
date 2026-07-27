use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct KafkaService {
    inner: ManagedService,
}

impl KafkaService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Kafka {
            return Err(DevBoxError::InvalidConfig(
                "KafkaService requires kind=kafka".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for KafkaService {
    fn install(&self) -> Result<()> {
        let config = &self.inner.config;
        let contents = format!(
            "# 智屿 Kafka Sandbox 本地开发配置\n\
             # Tansu 提供 Kafka API 兼容服务，无需 JVM 与 ZooKeeper。\n\
             cluster_id=zhiyu-local\n\
             listener_url=tcp://127.0.0.1:{}\n\
             advertised_listener_url=tcp://127.0.0.1:{}\n\
             storage_engine=sqlite://{}\n",
            config.port,
            config.port,
            config.data_dir().join("tansu.db").display(),
        );
        self.inner.install("kafka.conf", &contents)
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
