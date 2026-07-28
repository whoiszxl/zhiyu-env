use crate::error::{DevBoxError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServiceKind {
    Redis,
    Mysql,
    Postgres,
    Mongodb,
    Mailpit,
    Nats,
    Kafka,
    Meilisearch,
    Minio,
    Rustfs,
    Etcd,
    Consul,
    Rnacos,
    Rabbitmq,
    Nginx,
}

impl ServiceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Redis => "redis",
            Self::Mysql => "mysql",
            Self::Postgres => "postgres",
            Self::Mongodb => "mongodb",
            Self::Mailpit => "mailpit",
            Self::Nats => "nats",
            Self::Kafka => "kafka",
            Self::Meilisearch => "meilisearch",
            Self::Minio => "minio",
            Self::Rustfs => "rustfs",
            Self::Etcd => "etcd",
            Self::Consul => "consul",
            Self::Rnacos => "rnacos",
            Self::Rabbitmq => "rabbitmq",
            Self::Nginx => "nginx",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub kind: ServiceKind,
    pub version: String,
    pub port: u16,
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub instance_dir: PathBuf,
    #[serde(default = "default_wait_for_port")]
    pub wait_for_port: bool,
}

fn default_wait_for_port() -> bool {
    true
}

impl ServiceConfig {
    pub fn validate(&self) -> Result<()> {
        if self.name.trim().is_empty() {
            return Err(DevBoxError::InvalidConfig(
                "service name cannot be empty".into(),
            ));
        }
        if self.version.trim().is_empty() {
            return Err(DevBoxError::InvalidConfig(
                "service version cannot be empty".into(),
            ));
        }
        if self.port == 0 {
            return Err(DevBoxError::InvalidConfig(
                "service port cannot be zero".into(),
            ));
        }
        if self.executable.as_os_str().is_empty() {
            return Err(DevBoxError::InvalidConfig(
                "service executable cannot be empty".into(),
            ));
        }
        if self.instance_dir.as_os_str().is_empty() {
            return Err(DevBoxError::InvalidConfig(
                "instance directory cannot be empty".into(),
            ));
        }
        Ok(())
    }

    pub fn config_dir(&self) -> PathBuf {
        self.instance_dir.join("conf")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.instance_dir.join("data")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.instance_dir.join("logs")
    }

    pub fn run_dir(&self) -> PathBuf {
        self.instance_dir.join("run")
    }

    pub fn metadata_path(&self) -> PathBuf {
        self.instance_dir.join("service.json")
    }

    pub fn pid_path(&self) -> PathBuf {
        self.run_dir().join("service.pid")
    }

    pub fn anomaly_path(&self) -> PathBuf {
        self.run_dir().join("service.anomaly.json")
    }

    pub fn stdout_log_path(&self) -> PathBuf {
        self.logs_dir().join("stdout.log")
    }

    pub fn stderr_log_path(&self) -> PathBuf {
        self.logs_dir().join("stderr.log")
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ConfigManager;

impl ConfigManager {
    pub fn save(&self, config: &ServiceConfig) -> Result<()> {
        config.validate()?;
        fs::create_dir_all(&config.instance_dir)?;

        let target = config.metadata_path();
        let temporary = target.with_extension("json.tmp");
        let contents = serde_json::to_vec_pretty(config)?;
        fs::write(&temporary, contents)?;
        fs::rename(temporary, target)?;
        Ok(())
    }

    pub fn load(&self, path: impl AsRef<Path>) -> Result<ServiceConfig> {
        let contents = fs::read(path)?;
        let config = serde_json::from_slice::<ServiceConfig>(&contents)?;
        config.validate()?;
        Ok(config)
    }
}
