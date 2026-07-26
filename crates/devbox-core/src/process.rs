use crate::config::ServiceConfig;
use crate::error::{DevBoxError, Result};
use crate::status::ServiceStatus;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct ProcessManager {
    stop_timeout: Duration,
    poll_interval: Duration,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self {
            stop_timeout: Duration::from_secs(10),
            poll_interval: Duration::from_millis(100),
        }
    }
}

impl ProcessManager {
    pub fn new(stop_timeout: Duration) -> Self {
        Self {
            stop_timeout,
            ..Self::default()
        }
    }

    pub fn start(&self, config: &ServiceConfig) -> Result<u32> {
        match self.status(config)? {
            ServiceStatus::Running { pid } => return Err(DevBoxError::AlreadyRunning(pid)),
            ServiceStatus::NotInstalled => {
                return Err(DevBoxError::NotInstalled(config.name.clone()))
            }
            ServiceStatus::Stopped | ServiceStatus::StalePid { .. } => {}
        }

        if !config.executable.is_file() {
            return Err(DevBoxError::ExecutableNotFound(config.executable.clone()));
        }

        fs::create_dir_all(config.logs_dir())?;
        fs::create_dir_all(config.run_dir())?;
        let stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open(config.stdout_log_path())?;
        let stderr = OpenOptions::new()
            .create(true)
            .append(true)
            .open(config.stderr_log_path())?;

        let mut child = Command::new(&config.executable)
            .args(&config.arguments)
            .envs(&config.environment)
            .current_dir(&config.instance_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()?;

        let pid = child.id();
        let startup_deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < startup_deadline {
            if let Some(status) = child.try_wait()? {
                let details = startup_log_details(config);
                return Err(DevBoxError::CommandFailed {
                    command: config.executable.display().to_string(),
                    message: format!("进程在启动阶段退出（{status}）。{details}"),
                });
            }
            thread::sleep(Duration::from_millis(25));
        }

        fs::write(config.pid_path(), pid.to_string())?;
        child_registry()
            .lock()
            .expect("process child registry mutex poisoned")
            .insert(pid, child);
        Ok(pid)
    }

    pub fn stop(&self, config: &ServiceConfig) -> Result<()> {
        let pid = match self.status(config)? {
            ServiceStatus::Running { pid } => pid,
            ServiceStatus::StalePid { .. } | ServiceStatus::Stopped => {
                self.remove_pid_file(config)?;
                return Err(DevBoxError::NotRunning);
            }
            ServiceStatus::NotInstalled => {
                return Err(DevBoxError::NotInstalled(config.name.clone()))
            }
        };

        send_signal(pid, libc::SIGTERM)?;
        let started = Instant::now();
        while started.elapsed() < self.stop_timeout {
            self.reap_child(pid)?;
            if !process_exists(pid) {
                self.remove_pid_file(config)?;
                return Ok(());
            }
            thread::sleep(self.poll_interval);
        }

        Err(DevBoxError::StopTimeout {
            pid,
            timeout_secs: self.stop_timeout.as_secs(),
        })
    }

    pub fn restart(&self, config: &ServiceConfig) -> Result<u32> {
        if matches!(self.status(config)?, ServiceStatus::Running { .. }) {
            self.stop(config)?;
        } else {
            self.remove_pid_file(config)?;
        }
        self.start(config)
    }

    pub fn status(&self, config: &ServiceConfig) -> Result<ServiceStatus> {
        if !config.metadata_path().is_file() || !config.executable.is_file() {
            return Ok(ServiceStatus::NotInstalled);
        }
        if !config.pid_path().is_file() {
            return Ok(ServiceStatus::Stopped);
        }

        let raw_pid = fs::read_to_string(config.pid_path())?;
        let pid = raw_pid
            .trim()
            .parse::<u32>()
            .map_err(|_| DevBoxError::InvalidConfig("invalid pid file".into()))?;

        self.reap_child(pid)?;
        if self.owns_running_child(pid) {
            return Ok(ServiceStatus::Running { pid });
        }
        if process_matches(pid, config) {
            Ok(ServiceStatus::Running { pid })
        } else {
            Ok(ServiceStatus::StalePid { pid })
        }
    }

    fn remove_pid_file(&self, config: &ServiceConfig) -> Result<()> {
        match fs::remove_file(config.pid_path()) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    fn reap_child(&self, pid: u32) -> Result<()> {
        let mut registry = child_registry()
            .lock()
            .expect("process child registry mutex poisoned");
        let should_remove = registry
            .get_mut(&pid)
            .map(|process| process.try_wait())
            .transpose()?
            .flatten()
            .is_some();
        if should_remove {
            registry.remove(&pid);
        }
        Ok(())
    }

    fn owns_running_child(&self, pid: u32) -> bool {
        child_registry()
            .lock()
            .expect("process child registry mutex poisoned")
            .contains_key(&pid)
    }
}

fn startup_log_details(config: &ServiceConfig) -> String {
    for path in [config.stderr_log_path(), config.stdout_log_path()] {
        let Ok(contents) = fs::read(&path) else {
            continue;
        };
        if contents.is_empty() {
            continue;
        }
        let start = contents.len().saturating_sub(4_096);
        let tail = String::from_utf8_lossy(&contents[start..]);
        return format!("请检查日志 {}：{}", path.display(), tail.trim());
    }
    format!("请检查日志目录 {}", config.logs_dir().display())
}

fn child_registry() -> &'static Mutex<HashMap<u32, Child>> {
    static CHILDREN: OnceLock<Mutex<HashMap<u32, Child>>> = OnceLock::new();
    CHILDREN.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(unix)]
fn process_exists(pid: u32) -> bool {
    // SAFETY: kill with signal 0 does not send a signal; it only checks whether
    // the process exists and is visible to the current user.
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(unix)]
fn process_matches(pid: u32, config: &ServiceConfig) -> bool {
    if !process_exists(pid) {
        return false;
    }

    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            let command = String::from_utf8_lossy(&output.stdout);
            let executable = config.executable.to_string_lossy();
            let executable_name = config
                .executable
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            let running_name = command
                .split_whitespace()
                .next()
                .and_then(|path| std::path::Path::new(path).file_name())
                .and_then(|name| name.to_str())
                .unwrap_or_default();

            command.contains(executable.as_ref()) || running_name == executable_name
        }
        _ => false,
    }
}

#[cfg(unix)]
fn send_signal(pid: u32, signal: i32) -> Result<()> {
    // SAFETY: pid comes from a parsed u32 pid file and signal is a libc constant.
    let result = unsafe { libc::kill(pid as libc::pid_t, signal) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error().into())
    }
}

#[cfg(not(unix))]
fn process_exists(_pid: u32) -> bool {
    false
}

#[cfg(not(unix))]
fn process_matches(_pid: u32, _config: &ServiceConfig) -> bool {
    false
}

#[cfg(not(unix))]
fn send_signal(_pid: u32, _signal: i32) -> Result<()> {
    Err(DevBoxError::UnsupportedPlatform(
        "process signals currently require a Unix platform".into(),
    ))
}
