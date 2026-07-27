use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;
use std::fs;
use std::path::{Path, PathBuf};

pub struct PostgresService {
    inner: ManagedService,
}

impl PostgresService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Postgres {
            return Err(DevBoxError::InvalidConfig(
                "PostgresService requires kind=postgres".into(),
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
                    "无法迁移 PostgreSQL 旧数据：{} 已存在，请先备份并处理冲突文件",
                    destination.display()
                )));
            }
            fs::rename(source, destination)?;
        }
        Ok(())
    }
}

impl ServiceManager for PostgresService {
    fn install(&self) -> Result<()> {
        self.prepare_version_data()?;
        let config = &self.inner.config;
        let contents = format!(
            "listen_addresses = '127.0.0.1'\nport = {}\ndata_directory = '{}'\nunix_socket_directories = '{}'\n",
            config.port,
            self.data_dir().display(),
            config.run_dir().display(),
        );
        self.inner.install("postgresql.conf", &contents)
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
        .windows(2)
        .find(|arguments| arguments[0] == "-D")
        .map(|arguments| PathBuf::from(&arguments[1]))
}

fn is_version_directory(path: &Path) -> bool {
    path.is_dir()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                !name.is_empty() && name.chars().all(|character| character.is_ascii_digit())
            })
}
