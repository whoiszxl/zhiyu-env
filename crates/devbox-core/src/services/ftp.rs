use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;
use std::fs;

pub struct FtpService {
    inner: ManagedService,
}

impl FtpService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Ftp {
            return Err(DevBoxError::InvalidConfig(
                "FtpService requires kind=ftp".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }

    fn ensure_password(&self) -> Result<()> {
        let password_path = self.inner.config.config_dir().join("ftp.password");
        if password_path.is_file() {
            return Ok(());
        }

        fs::write(&password_path, "zhiyu-local-ftp-2026")?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&password_path, fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }
}

impl ServiceManager for FtpService {
    fn install(&self) -> Result<()> {
        self.inner.install(
            "ftp.env",
            "# 智屿 FTP 本地开发服务\n\
             # 默认仅监听 127.0.0.1:2121，禁用主动模式，避免暴露到局域网。\n\
             # 账号密码保存在同目录的 ftp.password，文件权限为 0600。\n\
             SFTPGO_FTPD__BINDINGS__0__ADDRESS=127.0.0.1\n\
             SFTPGO_FTPD__PASSIVE_PORT_RANGE__START=50000\n\
             SFTPGO_FTPD__PASSIVE_PORT_RANGE__END=50009\n\
             SFTPGO_FTPD__DISABLE_ACTIVE_MODE=true\n",
        )?;
        self.ensure_password()
    }

    fn start(&self) -> Result<u32> {
        self.ensure_password()?;
        self.inner.start()
    }

    fn stop(&self) -> Result<()> {
        self.inner.stop()
    }

    fn force_stop(&self) -> Result<()> {
        self.inner.force_stop()
    }

    fn restart(&self) -> Result<u32> {
        self.ensure_password()?;
        self.inner.restart()
    }

    fn status(&self) -> Result<ServiceStatus> {
        self.inner.status()
    }

    fn repair(&self) -> Result<()> {
        self.inner.repair()
    }
}
