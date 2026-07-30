use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct InfluxdbService {
    inner: ManagedService,
}

impl InfluxdbService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Influxdb {
            return Err(DevBoxError::InvalidConfig(
                "InfluxdbService requires kind=influxdb".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for InfluxdbService {
    fn install(&self) -> Result<()> {
        self.inner.install(
            "influxdb.env",
            "# 智屿 InfluxDB 3 Core 本地开发配置\n\
             # 服务仅监听 127.0.0.1，并关闭认证，仅用于本地开发。\n\
             INFLUXDB3_HTTP_BIND_ADDR=127.0.0.1:8181\n\
             INFLUXDB3_START_WITHOUT_AUTH=true\n",
        )
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
