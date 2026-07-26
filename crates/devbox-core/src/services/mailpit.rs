use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct MailpitService {
    inner: ManagedService,
}

impl MailpitService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Mailpit {
            return Err(DevBoxError::InvalidConfig(
                "MailpitService requires kind=mailpit".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for MailpitService {
    fn install(&self) -> Result<()> {
        let config = &self.inner.config;
        let contents = format!(
            "# Mailpit 本地邮件捕获配置\n\
             MP_SMTP_BIND_ADDR=127.0.0.1:1025\n\
             MP_UI_BIND_ADDR=127.0.0.1:8025\n\
             MP_DATABASE={}\n\
             MP_MAX_MESSAGES=500\n\
             MP_MAX_MESSAGE_SIZE=10\n\
             MP_DISABLE_VERSION_CHECK=true\n\
             MP_BLOCK_REMOTE_CSS_AND_FONTS=true\n\
             MP_QUIET=true\n",
            config.data_dir().join("mailpit.db").display(),
        );
        self.inner.install("mailpit.env", &contents)
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
