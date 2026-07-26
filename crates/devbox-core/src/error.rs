use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DevBoxError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("failed to serialize configuration: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("service is not installed: {0}")]
    NotInstalled(String),

    #[error("service is already running with pid {0}")]
    AlreadyRunning(u32),

    #[error("service is not running")]
    NotRunning,

    #[error("executable does not exist: {0}")]
    ExecutableNotFound(PathBuf),

    #[error("download integrity check failed: expected {expected}, got {actual}")]
    IntegrityMismatch { expected: String, actual: String },

    #[error("external command failed: {command}: {message}")]
    CommandFailed { command: String, message: String },

    #[error("process {pid} did not stop within {timeout_secs} seconds")]
    StopTimeout { pid: u32, timeout_secs: u64 },

    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(String),
}

pub type Result<T> = std::result::Result<T, DevBoxError>;
