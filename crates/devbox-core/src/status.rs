use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    NotInstalled,
    Stopped,
    Running { pid: u32 },
    StalePid { pid: u32 },
    Crashed { pid: u32 },
}
