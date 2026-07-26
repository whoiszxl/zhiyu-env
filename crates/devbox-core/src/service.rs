use crate::error::Result;
use crate::status::ServiceStatus;

pub trait ServiceManager: Send + Sync {
    fn install(&self) -> Result<()>;
    fn start(&self) -> Result<u32>;
    fn stop(&self) -> Result<()>;
    fn restart(&self) -> Result<u32>;
    fn status(&self) -> Result<ServiceStatus>;
}
