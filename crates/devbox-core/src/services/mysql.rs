use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;
use std::fs;
use std::path::{Path, PathBuf};

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

    pub fn data_dir(&self) -> PathBuf {
        version_data_dir(&self.inner.config).unwrap_or_else(|| self.inner.config.data_dir())
    }

    pub fn prepare_version_data(&self) -> Result<()> {
        let Some(target) = version_data_dir(&self.inner.config) else {
            return Ok(());
        };
        let data_root = self.inner.config.data_dir();
        fs::create_dir_all(&target)?;

        if !data_root.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(&data_root)? {
            let entry = entry?;
            let source = entry.path();
            if source == target || is_version_directory(&source) {
                continue;
            }

            let destination = target.join(entry.file_name());
            if destination.exists() {
                return Err(DevBoxError::InvalidConfig(format!(
                    "无法迁移 MySQL 旧数据：{} 已存在，请先备份并处理冲突文件",
                    destination.display()
                )));
            }
            fs::rename(source, destination)?;
        }
        Ok(())
    }
}

impl ServiceManager for MysqlService {
    fn install(&self) -> Result<()> {
        self.prepare_version_data()?;
        let config = &self.inner.config;
        let basedir = config
            .executable
            .parent()
            .and_then(|path| path.parent())
            .ok_or_else(|| DevBoxError::InvalidConfig("invalid MySQL executable path".into()))?;
        let contents = format!(
            "[mysqld]\nbasedir={}\ndatadir={}\nbind-address=127.0.0.1\nport={}\nsocket={}\npid-file={}\nlog-error={}\nmysqlx=0\n",
            basedir.display(),
            self.data_dir().display(),
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
        match self.inner.status()? {
            ServiceStatus::Stopped
            | ServiceStatus::StalePid { .. }
            | ServiceStatus::Crashed { .. } => {
                self.prepare_version_data()?;
                self.inner.start()
            }
            ServiceStatus::Running { .. } | ServiceStatus::NotInstalled => self.inner.start(),
        }
    }

    fn stop(&self) -> Result<()> {
        self.inner.stop()
    }

    fn force_stop(&self) -> Result<()> {
        self.inner.force_stop()
    }

    fn restart(&self) -> Result<u32> {
        self.prepare_version_data()?;
        self.inner.restart()
    }

    fn status(&self) -> Result<ServiceStatus> {
        self.inner.status()
    }

    fn repair(&self) -> Result<()> {
        self.inner.repair()
    }
}

fn version_data_dir(config: &ServiceConfig) -> Option<PathBuf> {
    config
        .arguments
        .iter()
        .find_map(|argument| argument.strip_prefix("--datadir=").map(PathBuf::from))
}

fn is_version_directory(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.split_once('.'))
        .is_some_and(|(major, minor)| {
            !major.is_empty()
                && !minor.is_empty()
                && major.chars().all(|character| character.is_ascii_digit())
                && minor.chars().all(|character| character.is_ascii_digit())
        })
}
