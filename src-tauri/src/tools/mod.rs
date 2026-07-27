//! 内置开发工具。与 `services` 不同，这里的工具不下载常驻二进制、
//! 不受进程生命周期管理，全部在应用内直接完成计算。

pub mod data_format;
pub mod json_diff;
pub mod json_path;
