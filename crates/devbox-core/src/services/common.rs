use crate::config::{ConfigManager, ServiceConfig};
use crate::error::Result;
use crate::process::ProcessManager;
use crate::status::ServiceStatus;
use std::fs;

pub(crate) struct ManagedService {
    pub(crate) config: ServiceConfig,
    process: ProcessManager,
    config_manager: ConfigManager,
}

impl ManagedService {
    pub(crate) fn new(config: ServiceConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            config,
            process: ProcessManager::default(),
            config_manager: ConfigManager,
        })
    }

    pub(crate) fn install(&self, native_config_name: &str, contents: &str) -> Result<()> {
        fs::create_dir_all(self.config.config_dir())?;
        fs::create_dir_all(self.config.data_dir())?;
        fs::create_dir_all(self.config.logs_dir())?;
        fs::create_dir_all(self.config.run_dir())?;

        let native_config_path = self.config.config_dir().join(native_config_name);
        if !native_config_path.exists() {
            fs::write(native_config_path, contents)?;
        }
        self.config_manager.save(&self.config)
    }

    pub(crate) fn start(&self) -> Result<u32> {
        self.process.start(&self.config)
    }

    pub(crate) fn stop(&self) -> Result<()> {
        self.process.stop(&self.config)
    }

    pub(crate) fn force_stop(&self) -> Result<()> {
        self.process.force_stop(&self.config)
    }

    pub(crate) fn restart(&self) -> Result<u32> {
        self.process.restart(&self.config)
    }

    pub(crate) fn status(&self) -> Result<ServiceStatus> {
        self.process.status(&self.config)
    }

    pub(crate) fn repair(&self) -> Result<()> {
        self.process.repair(&self.config)
    }
}
