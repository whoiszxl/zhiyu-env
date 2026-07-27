use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;
use std::fs;
use std::path::{Path, PathBuf};

pub struct RedisService {
    inner: ManagedService,
}

impl RedisService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Redis {
            return Err(DevBoxError::InvalidConfig(
                "RedisService requires kind=redis".into(),
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
                    "无法迁移 Redis 旧数据：{} 已存在，请先备份并处理冲突文件",
                    destination.display()
                )));
            }
            fs::rename(source, destination)?;
        }
        Ok(())
    }
}

impl ServiceManager for RedisService {
    fn install(&self) -> Result<()> {
        self.prepare_version_data()?;
        let config = &self.inner.config;
        let contents = format!("bind 127.0.0.1\nport {}\ndaemonize no\n", config.port);
        self.inner.install("redis.conf", &contents)
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
        .find(|arguments| arguments[0] == "--dir")
        .map(|arguments| PathBuf::from(&arguments[1]))
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
