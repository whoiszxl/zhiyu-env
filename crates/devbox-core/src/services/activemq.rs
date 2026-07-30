use super::common::ManagedService;
use crate::config::{ConfigManager, ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;
use std::fs;
use std::path::Path;

pub struct ActivemqService {
    inner: ManagedService,
}

impl ActivemqService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Activemq {
            return Err(DevBoxError::InvalidConfig(
                "ActivemqService requires kind=activemq".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }

    fn ensure_java(&self) -> Result<()> {
        let home = self
            .inner
            .config
            .environment
            .get("JAVA_HOME")
            .map(Path::new);
        if home.is_some_and(|path| path.join("bin/java").is_file()) {
            Ok(())
        } else {
            let required = if self.inner.config.version.starts_with("6.3") {
                "Java 25"
            } else {
                "Java 17 或 Java 21"
            };
            Err(DevBoxError::InvalidConfig(format!(
                "ActiveMQ {} 需要 {required}。请先在“Java 开发环境”中安装并选择对应版本",
                self.inner.config.version
            )))
        }
    }
}

impl ServiceManager for ActivemqService {
    fn install(&self) -> Result<()> {
        let config = &self.inner.config;
        fs::create_dir_all(config.config_dir())?;
        fs::create_dir_all(config.data_dir())?;
        fs::create_dir_all(config.logs_dir())?;
        fs::create_dir_all(config.run_dir())?;
        fs::create_dir_all(config.instance_dir.join("tmp"))?;

        let home = config
            .environment
            .get("ACTIVEMQ_HOME")
            .ok_or_else(|| DevBoxError::InvalidConfig("ACTIVEMQ_HOME 未配置".into()))?;
        let source = Path::new(home).join("conf");
        if !source.is_dir() {
            return Err(DevBoxError::InvalidConfig(
                "ActiveMQ 官方包缺少 conf 目录".into(),
            ));
        }
        copy_missing_files(&source, &config.config_dir())?;

        let broker_path = config.config_dir().join("activemq.xml");
        if let Ok(contents) = fs::read_to_string(&broker_path) {
            let local_only = contents
                .replace("tcp://0.0.0.0:", "tcp://127.0.0.1:")
                .replace("amqp://0.0.0.0:", "amqp://127.0.0.1:")
                .replace("stomp://0.0.0.0:", "stomp://127.0.0.1:")
                .replace("mqtt://0.0.0.0:", "mqtt://127.0.0.1:")
                .replace("ws://0.0.0.0:", "ws://127.0.0.1:")
                .replace(
                    "<storeUsage limit=\"100 gb\"/>",
                    "<storeUsage limit=\"2 gb\"/>",
                )
                .replace(
                    "<tempUsage limit=\"50 gb\"/>",
                    "<tempUsage limit=\"512 mb\"/>",
                );
            fs::write(broker_path, local_only)?;
        }
        ConfigManager.save(config)
    }

    fn start(&self) -> Result<u32> {
        self.ensure_java()?;
        self.inner.start()
    }
    fn stop(&self) -> Result<()> {
        self.inner.stop()
    }
    fn force_stop(&self) -> Result<()> {
        self.inner.force_stop()
    }
    fn restart(&self) -> Result<u32> {
        self.ensure_java()?;
        self.inner.restart()
    }
    fn status(&self) -> Result<ServiceStatus> {
        self.inner.status()
    }
    fn repair(&self) -> Result<()> {
        self.inner.repair()
    }
}

fn copy_missing_files(source: &Path, target: &Path) -> Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let destination = target.join(entry.file_name());
        if !destination.exists() {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}
