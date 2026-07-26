use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;
use std::fs;

pub struct MysqlService {
    inner: ManagedService,
}

impl MysqlService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Mysql {
            return Err(DevBoxError::InvalidConfig(
                "MysqlService requires kind=mysql".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for MysqlService {
    fn install(&self) -> Result<()> {
        let config = &self.inner.config;
        let basedir = config
            .executable
            .parent()
            .and_then(|path| path.parent())
            .ok_or_else(|| DevBoxError::InvalidConfig("invalid MySQL executable path".into()))?;
        let contents = format!(
            "[mysqld]\nbasedir={}\ndatadir={}\nbind-address=127.0.0.1\nport={}\nsocket={}\npid-file={}\nlog-error={}\nmysqlx=0\n",
            basedir.display(),
            config.data_dir().display(),
            config.port,
            config.run_dir().join("mysql.sock").display(),
            config.run_dir().join("mysqld.pid").display(),
            config.logs_dir().join("mysql-error.log").display(),
        );
        self.inner.install("my.cnf", &contents)?;

        let path = config.config_dir().join("my.cnf");
        let mut existing = fs::read_to_string(&path)?;
        if !existing
            .lines()
            .any(|line| line.trim_start().starts_with("mysqlx="))
        {
            if !existing.ends_with('\n') {
                existing.push('\n');
            }
            existing.push_str("mysqlx=0\n");
            fs::write(path, existing)?;
        }
        Ok(())
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
