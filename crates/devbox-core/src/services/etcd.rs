use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct EtcdService {
    inner: ManagedService,
}

impl EtcdService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Etcd {
            return Err(DevBoxError::InvalidConfig(
                "EtcdService requires kind=etcd".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for EtcdService {
    fn install(&self) -> Result<()> {
        let data_dir = self.inner.config.data_dir();
        self.inner.install(
            "etcd.yaml",
            &format!(
                "# 智屿 etcd 单节点开发配置\n\
                 name: zhiyu-local\n\
                 data-dir: {}\n\
                 listen-client-urls: http://127.0.0.1:2379\n\
                 advertise-client-urls: http://127.0.0.1:2379\n\
                 listen-peer-urls: http://127.0.0.1:2380\n\
                 initial-advertise-peer-urls: http://127.0.0.1:2380\n\
                 initial-cluster: zhiyu-local=http://127.0.0.1:2380\n\
                 initial-cluster-state: new\n\
                 initial-cluster-token: zhiyu-local-etcd\n",
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
