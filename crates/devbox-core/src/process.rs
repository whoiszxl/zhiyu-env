use crate::config::ServiceConfig;
use crate::error::{DevBoxError, Result};
use crate::status::ServiceStatus;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub struct ProcessManager {
    stop_timeout: Duration,
    startup_timeout: Duration,
    poll_interval: Duration,
}

#[derive(Debug, Serialize, Deserialize)]
struct PidRecord {
    pid: u32,
    executable: PathBuf,
    start_marker: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnomalyRecord {
    kind: AnomalyKind,
    pid: u32,
}

struct LifecycleGuard {
    key: String,
}

impl Drop for LifecycleGuard {
    fn drop(&mut self) {
        lifecycle_locks()
            .lock()
            .expect("lifecycle lock registry mutex poisoned")
            .remove(&self.key);
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AnomalyKind {
    StalePid,
    Crashed,
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self {
            stop_timeout: Duration::from_secs(10),
            startup_timeout: Duration::from_secs(30),
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
        let _guard = self.try_lifecycle_lock(config)?;
        self.start_unlocked(config)
    }

    fn start_unlocked(&self, config: &ServiceConfig) -> Result<u32> {
        match self.status(config)? {
            ServiceStatus::Running { pid } => return Err(DevBoxError::AlreadyRunning(pid)),
            ServiceStatus::NotInstalled => {
                return Err(DevBoxError::NotInstalled(config.name.clone()))
            }
            ServiceStatus::Stopped
            | ServiceStatus::StalePid { .. }
            | ServiceStatus::Crashed { .. } => {}
        }

        if !config.executable.is_file() {
            return Err(DevBoxError::ExecutableNotFound(config.executable.clone()));
        }
        if config.wait_for_port {
            if let Some((pid, process)) = port_owner(config.port) {
                return Err(DevBoxError::PortOccupied {
                    port: config.port,
                    pid,
                    process,
                });
            }
        }

        fs::create_dir_all(config.logs_dir())?;
        fs::create_dir_all(config.run_dir())?;
        self.remove_runtime_files(config)?;
        let stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open(config.stdout_log_path())?;
        let stderr = OpenOptions::new()
            .create(true)
            .append(true)
            .open(config.stderr_log_path())?;

        let child = Command::new(&config.executable)
            .args(&config.arguments)
            .envs(&config.environment)
            .current_dir(&config.instance_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()?;

        let pid = child.id();
        let start_marker = process_start_marker(pid).unwrap_or_default();
        let mut record = PidRecord {
            pid,
            executable: canonical_or_original(&config.executable),
            start_marker,
        };
        fs::write(config.pid_path(), serde_json::to_vec_pretty(&record)?)?;
        child_registry()
            .lock()
            .expect("process child registry mutex poisoned")
            .insert(pid, child);

        let stability_deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < stability_deadline {
            self.reap_child(pid)?;
            if !child_registry()
                .lock()
                .expect("process child registry mutex poisoned")
                .contains_key(&pid)
            {
                let tail = log_tail(config, 50);
                self.record_anomaly(config, AnomalyKind::Crashed, pid)?;
                self.remove_pid_file(config)?;
                return Err(DevBoxError::StartupFailed {
                    message: "进程在启动阶段退出".into(),
                    log_tail: tail,
                });
            }
            thread::sleep(Duration::from_millis(25));
        }
        record.executable =
            process_executable(pid).unwrap_or_else(|| canonical_or_original(&config.executable));
        record.start_marker = process_start_marker(pid).unwrap_or(record.start_marker);
        fs::write(config.pid_path(), serde_json::to_vec_pretty(&record)?)?;

        if !config.wait_for_port {
            return Ok(pid);
        }
        if port_is_ready(config.port) {
            return Ok(pid);
        }

        let started = Instant::now();
        while started.elapsed() < self.startup_timeout {
            self.reap_child(pid)?;
            if !child_registry()
                .lock()
                .expect("process child registry mutex poisoned")
                .contains_key(&pid)
            {
                let tail = log_tail(config, 50);
                self.record_anomaly(config, AnomalyKind::Crashed, pid)?;
                self.remove_pid_file(config)?;
                return Err(DevBoxError::StartupFailed {
                    message: "进程在端口就绪前退出".into(),
                    log_tail: tail,
                });
            }
            if port_is_ready(config.port) {
                return Ok(pid);
            }
            thread::sleep(self.poll_interval);
        }

        let tail = log_tail(config, 50);
        self.terminate_failed_start(pid)?;
        self.record_anomaly(config, AnomalyKind::Crashed, pid)?;
        self.remove_pid_file(config)?;
        Err(DevBoxError::StartupFailed {
            message: format!("等待端口 {} 就绪超过 30 秒", config.port),
            log_tail: tail,
        })
    }

    pub fn stop(&self, config: &ServiceConfig) -> Result<()> {
        let _guard = self.try_lifecycle_lock(config)?;
        self.stop_unlocked(config)
    }

    fn stop_unlocked(&self, config: &ServiceConfig) -> Result<()> {
        let pid = match self.status(config)? {
            ServiceStatus::Running { pid } => pid,
            ServiceStatus::StalePid { .. }
            | ServiceStatus::Crashed { .. }
            | ServiceStatus::Stopped => {
                self.remove_runtime_files(config)?;
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
                self.remove_runtime_files(config)?;
                return Ok(());
            }
            thread::sleep(self.poll_interval);
        }

        Err(DevBoxError::StopTimeout {
            pid,
            timeout_secs: self.stop_timeout.as_secs(),
        })
    }

    pub fn force_stop(&self, config: &ServiceConfig) -> Result<()> {
        let _guard = self.try_lifecycle_lock(config)?;
        let pid = match self.status(config)? {
            ServiceStatus::Running { pid } => pid,
            _ => {
                self.remove_runtime_files(config)?;
                return Err(DevBoxError::NotRunning);
            }
        };
        send_signal(pid, libc::SIGKILL)?;
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(3) {
            self.reap_child(pid)?;
            if !process_exists(pid) {
                self.remove_runtime_files(config)?;
                return Ok(());
            }
            thread::sleep(self.poll_interval);
        }
        Err(DevBoxError::StopTimeout {
            pid,
            timeout_secs: 3,
        })
    }

    pub fn restart(&self, config: &ServiceConfig) -> Result<u32> {
        let _guard = self.try_lifecycle_lock(config)?;
        if matches!(self.status(config)?, ServiceStatus::Running { .. }) {
            self.stop_unlocked(config)?;
        } else {
            self.remove_runtime_files(config)?;
        }
        self.start_unlocked(config)
    }

    pub fn repair(&self, config: &ServiceConfig) -> Result<()> {
        let _guard = self.try_lifecycle_lock(config)?;
        match self.status(config)? {
            ServiceStatus::Running { .. } => Ok(()),
            _ => self.remove_runtime_files(config),
        }
    }

    pub fn status(&self, config: &ServiceConfig) -> Result<ServiceStatus> {
        if !config.metadata_path().is_file() || !config.executable.is_file() {
            return Ok(ServiceStatus::NotInstalled);
        }
        if !config.pid_path().is_file() {
            return self.persisted_status(config);
        }

        let raw = fs::read_to_string(config.pid_path())?;
        let record = match serde_json::from_str::<PidRecord>(&raw) {
            Ok(record) => record,
            Err(_) => {
                let legacy_pid = raw.trim().parse::<u32>().unwrap_or(0);
                self.record_anomaly(config, AnomalyKind::StalePid, legacy_pid)?;
                self.remove_pid_file(config)?;
                return Ok(ServiceStatus::StalePid { pid: legacy_pid });
            }
        };

        self.reap_child(record.pid)?;
        let current_start_marker = process_start_marker(record.pid);
        let current_executable = process_executable(record.pid);
        if current_executable.as_ref() == Some(&record.executable)
            && current_start_marker.as_deref().is_some_and(|marker| {
                record.start_marker.is_empty() || marker == record.start_marker
            })
        {
            return Ok(ServiceStatus::Running { pid: record.pid });
        }
        if current_start_marker
            .as_deref()
            .is_some_and(|marker| !record.start_marker.is_empty() && marker != record.start_marker)
            || current_executable
                .as_ref()
                .is_some_and(|executable| executable != &record.executable)
        {
            self.record_anomaly(config, AnomalyKind::StalePid, record.pid)?;
            self.remove_pid_file(config)?;
            return Ok(ServiceStatus::StalePid { pid: record.pid });
        }
        if child_registry()
            .lock()
            .expect("process child registry mutex poisoned")
            .contains_key(&record.pid)
        {
            return Ok(ServiceStatus::Running { pid: record.pid });
        }

        let kind = if current_start_marker.is_some() {
            AnomalyKind::StalePid
        } else {
            AnomalyKind::Crashed
        };
        let status = match kind {
            AnomalyKind::StalePid => ServiceStatus::StalePid { pid: record.pid },
            AnomalyKind::Crashed => ServiceStatus::Crashed { pid: record.pid },
        };
        self.record_anomaly(config, kind, record.pid)?;
        self.remove_pid_file(config)?;
        Ok(status)
    }

    fn persisted_status(&self, config: &ServiceConfig) -> Result<ServiceStatus> {
        let Ok(contents) = fs::read(config.anomaly_path()) else {
            return Ok(ServiceStatus::Stopped);
        };
        let Ok(record) = serde_json::from_slice::<AnomalyRecord>(&contents) else {
            let _ = fs::remove_file(config.anomaly_path());
            return Ok(ServiceStatus::Stopped);
        };
        Ok(match record.kind {
            AnomalyKind::StalePid => ServiceStatus::StalePid { pid: record.pid },
            AnomalyKind::Crashed => ServiceStatus::Crashed { pid: record.pid },
        })
    }

    fn record_anomaly(&self, config: &ServiceConfig, kind: AnomalyKind, pid: u32) -> Result<()> {
        fs::create_dir_all(config.run_dir())?;
        fs::write(
            config.anomaly_path(),
            serde_json::to_vec_pretty(&AnomalyRecord { kind, pid })?,
        )?;
        Ok(())
    }

    fn remove_pid_file(&self, config: &ServiceConfig) -> Result<()> {
        remove_if_exists(&config.pid_path())
    }

    fn remove_runtime_files(&self, config: &ServiceConfig) -> Result<()> {
        remove_if_exists(&config.pid_path())?;
        remove_if_exists(&config.anomaly_path())
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

    fn terminate_failed_start(&self, pid: u32) -> Result<()> {
        send_signal(pid, libc::SIGTERM)?;
        let graceful_deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < graceful_deadline {
            self.reap_child(pid)?;
            if !process_exists(pid) {
                return Ok(());
            }
            thread::sleep(self.poll_interval);
        }
        send_signal(pid, libc::SIGKILL)?;
        let force_deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < force_deadline {
            self.reap_child(pid)?;
            if !process_exists(pid) {
                return Ok(());
            }
            thread::sleep(self.poll_interval);
        }
        Err(DevBoxError::StopTimeout {
            pid,
            timeout_secs: 5,
        })
    }

    fn try_lifecycle_lock(&self, config: &ServiceConfig) -> Result<LifecycleGuard> {
        let key = config.instance_dir.display().to_string();
        let mut locks = lifecycle_locks()
            .lock()
            .expect("lifecycle lock registry mutex poisoned");
        if locks.insert(key.clone()) {
            Ok(LifecycleGuard { key })
        } else {
            Err(DevBoxError::OperationInProgress)
        }
    }
}

fn lifecycle_locks() -> &'static Mutex<HashSet<String>> {
    static LOCKS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn child_registry() -> &'static Mutex<HashMap<u32, Child>> {
    static CHILDREN: OnceLock<Mutex<HashMap<u32, Child>>> = OnceLock::new();
    CHILDREN.get_or_init(|| Mutex::new(HashMap::new()))
}

fn remove_if_exists(path: &Path) -> Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn canonical_or_original(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn log_tail(config: &ServiceConfig, lines: usize) -> String {
    let mut collected = Vec::new();
    for path in [config.stderr_log_path(), config.stdout_log_path()] {
        let Ok(contents) = fs::read_to_string(&path) else {
            continue;
        };
        collected.extend(contents.lines().map(str::to_owned));
    }
    if collected.is_empty() {
        return format!("日志目录：{}（暂无日志输出）", config.logs_dir().display());
    }
    let start = collected.len().saturating_sub(lines);
    collected[start..].join("\n")
}

fn port_is_ready(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
        Duration::from_millis(150),
    )
    .is_ok()
}

fn port_owner(port: u16) -> Option<(u32, String)> {
    let output = Command::new("/usr/sbin/lsof")
        .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN", "-Fpc"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut pid = None;
    let mut process = None;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        match line.as_bytes().first() {
            Some(b'p') => pid = line[1..].parse::<u32>().ok(),
            Some(b'c') => process = Some(line[1..].to_string()),
            _ => {}
        }
        if let (Some(pid), Some(process)) = (pid, process.clone()) {
            return Some((pid, process));
        }
    }
    None
}

fn process_start_marker(pid: u32) -> Option<String> {
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "lstart="])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(unix)]
fn process_exists(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    // SAFETY: signal 0 only checks whether the process exists.
    let result = unsafe { libc::kill(pid as libc::pid_t, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(unix)]
fn process_executable(pid: u32) -> Option<PathBuf> {
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let command = String::from_utf8_lossy(&output.stdout);
    command
        .split_whitespace()
        .next()
        .map(PathBuf::from)
        .map(|path| canonical_or_original(&path))
}

#[cfg(unix)]
fn send_signal(pid: u32, signal: libc::c_int) -> Result<()> {
    // SAFETY: PID identity is validated immediately before callers signal it.
    let result = unsafe { libc::kill(pid as libc::pid_t, signal) };
    if result == 0 {
        Ok(())
    } else {
        let error = std::io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            Ok(())
        } else {
            Err(error.into())
        }
    }
}
