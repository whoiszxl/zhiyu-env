use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct RnacosService {
    inner: ManagedService,
}

impl RnacosService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Rnacos {
            return Err(DevBoxError::InvalidConfig(
                "RnacosService requires kind=rnacos".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for RnacosService {
    fn install(&self) -> Result<()> {
        let data_dir = self.inner.config.data_dir();
        self.inner.install(
            "rnacos.env",
            &format!(
                "# 智屿 rnacos 单节点开发配置\n\
                 RUST_LOG=warn\n\
                 RNACOS_HTTP_PORT=8848\n\
                 RNACOS_GRPC_PORT=9848\n\
                 RNACOS_HTTP_CONSOLE_PORT=10848\n\
                 RNACOS_CONFIG_DB_DIR={}\n\
                 RNACOS_ENABLE_OPEN_API_AUTH=false\n",
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
