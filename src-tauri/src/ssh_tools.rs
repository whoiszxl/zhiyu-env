use base64::{engine::general_purpose::STANDARD, engine::general_purpose::STANDARD_NO_PAD, Engine};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

const MAX_PROFILES: usize = 100;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshProfile {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub identity_file: String,
    #[serde(default = "default_auth_method")]
    pub auth_method: String,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
}

fn default_auth_method() -> String {
    "key".into()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshHostKey {
    host: String,
    key_type: String,
    fingerprint: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshCommandResult {
    success: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    elapsed_millis: u128,
    truncated: bool,
    timed_out: bool,
}

struct ProcessOutput {
    status_code: Option<i32>,
    success: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    truncated: bool,
    timed_out: bool,
    elapsed_millis: u128,
}

pub struct SshTerminalState(Mutex<HashMap<String, TerminalSession>>);

impl Default for SshTerminalState {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

struct TerminalSession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SshTerminalEvent {
    session_id: String,
    event: String,
    data: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SshTerminalConnection {
    session_id: String,
}

fn ssh_directory(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|path| path.join("ssh"))
        .map_err(|error| format!("无法确定 SSH 配置目录: {error}"))
}

fn profiles_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(ssh_directory(app)?.join("profiles.json"))
}

fn known_hosts_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(ssh_directory(app)?.join("known_hosts"))
}

fn set_owner_only(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("无法设置安全文件权限: {error}"))?;
    }
    Ok(())
}

fn ensure_private_directory(directory: &Path) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|error| format!("无法创建 SSH 配置目录: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("无法设置 SSH 目录权限: {error}"))?;
    }
    Ok(())
}

fn load_profiles(app: &AppHandle) -> Result<Vec<SshProfile>, String> {
    let path = profiles_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let profiles: Vec<SshProfile> = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("无法读取 SSH 连接配置: {error}"))?,
    )
    .map_err(|error| format!("SSH 连接配置已损坏: {error}"))?;
    Ok(profiles)
}

fn persist_profiles(app: &AppHandle, profiles: &[SshProfile]) -> Result<(), String> {
    let path = profiles_path(app)?;
    let directory = path
        .parent()
        .ok_or_else(|| "SSH 配置路径无效".to_string())?;
    ensure_private_directory(directory)?;
    let temporary = path.with_extension("tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(profiles).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("无法保存 SSH 连接配置: {error}"))?;
    set_owner_only(&temporary)?;
    fs::rename(&temporary, &path).map_err(|error| format!("无法更新 SSH 连接配置: {error}"))?;
    set_owner_only(&path)
}

fn validate_profile(profile: &SshProfile) -> Result<(), String> {
    let name = profile.name.trim();
    if name.is_empty() || name.chars().count() > 50 || name.chars().any(char::is_control) {
        return Err("连接名称必须为 1 到 50 个可见字符".into());
    }
    let host = profile.host.trim();
    if host.is_empty()
        || host.len() > 255
        || host.starts_with('-')
        || !host
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".:-[]%_".contains(character))
    {
        return Err("SSH 主机地址格式不正确".into());
    }
    let username = profile.username.trim();
    if username.is_empty()
        || username.len() > 64
        || !username
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character))
    {
        return Err("SSH 用户名只能包含字母、数字、点、下划线和连字符".into());
    }
    if profile.port == 0 {
        return Err("SSH 端口必须在 1 到 65535 之间".into());
    }
    if !matches!(profile.auth_method.as_str(), "key" | "password") {
        return Err("SSH 认证方式只支持密钥或密码".into());
    }
    if profile.auth_method == "key" && !profile.identity_file.trim().is_empty() {
        validate_identity_file(Path::new(profile.identity_file.trim()))?;
    }
    Ok(())
}

fn validate_identity_file(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("私钥路径必须是绝对路径".into());
    }
    let metadata = fs::metadata(path).map_err(|error| format!("无法读取私钥文件: {error}"))?;
    if !metadata.is_file() {
        return Err("私钥路径不是普通文件".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err("私钥权限过于宽松，请先执行 chmod 600 <私钥路径>".into());
        }
    }
    Ok(())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn new_profile_id() -> String {
    format!("ssh-{:x}", now_millis())
}

fn profile_by_id(app: &AppHandle, id: &str) -> Result<SshProfile, String> {
    load_profiles(app)?
        .into_iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| "SSH 连接配置不存在".to_string())
}

#[tauri::command]
pub fn ssh_profiles_list(app: AppHandle) -> Result<Vec<SshProfile>, String> {
    load_profiles(&app)
}

#[tauri::command]
pub fn ssh_profile_save(app: AppHandle, mut profile: SshProfile) -> Result<SshProfile, String> {
    profile.name = profile.name.trim().to_string();
    profile.host = profile.host.trim().to_string();
    profile.username = profile.username.trim().to_string();
    profile.identity_file = profile.identity_file.trim().to_string();
    if profile.auth_method == "password" {
        profile.identity_file.clear();
    }
    validate_profile(&profile)?;

    let mut profiles = load_profiles(&app)?;
    let timestamp = now_millis();
    if profile.id.trim().is_empty() {
        if profiles.len() >= MAX_PROFILES {
            return Err("最多保存 100 个 SSH 连接".into());
        }
        profile.id = new_profile_id();
        profile.created_at_millis = timestamp;
        profile.updated_at_millis = timestamp;
        profiles.push(profile.clone());
    } else if let Some(existing) = profiles.iter_mut().find(|item| item.id == profile.id) {
        profile.created_at_millis = existing.created_at_millis;
        profile.updated_at_millis = timestamp;
        *existing = profile.clone();
    } else {
        return Err("要更新的 SSH 连接不存在".into());
    }
    profiles.sort_by(|left, right| right.updated_at_millis.cmp(&left.updated_at_millis));
    persist_profiles(&app, &profiles)?;
    Ok(profile)
}

#[tauri::command]
pub fn ssh_profile_delete(app: AppHandle, id: String) -> Result<(), String> {
    let mut profiles = load_profiles(&app)?;
    let original_len = profiles.len();
    profiles.retain(|profile| profile.id != id);
    if profiles.len() == original_len {
        return Err("SSH 连接配置不存在".into());
    }
    persist_profiles(&app, &profiles)
}

fn scan_host_key(profile: &SshProfile) -> Result<(SshHostKey, String), String> {
    validate_profile(profile)?;
    let mut command = Command::new("ssh-keyscan");
    command
        .arg("-T")
        .arg("8")
        .arg("-p")
        .arg(profile.port.to_string())
        .arg(&profile.host)
        .stdin(Stdio::null())
        .env("LC_ALL", "C");
    let output = run_with_limit(command, Duration::from_secs(10), 256 * 1024)?;
    if output.timed_out {
        return Err("获取主机指纹超时".into());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut candidates = stdout
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let host = parts.next()?;
            let key_type = parts.next()?;
            let encoded = parts.next()?;
            let decoded = STANDARD.decode(encoded).ok()?;
            let fingerprint = format!("SHA256:{}", STANDARD_NO_PAD.encode(Sha256::digest(decoded)));
            let priority = match key_type {
                "ssh-ed25519" => 0,
                value if value.starts_with("ecdsa-") => 1,
                "ssh-rsa" => 2,
                _ => 3,
            };
            Some((
                priority,
                SshHostKey {
                    host: host.to_string(),
                    key_type: key_type.to_string(),
                    fingerprint,
                },
                line.to_string(),
            ))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| candidate.0);
    candidates
        .into_iter()
        .next()
        .map(|(_, key, line)| (key, line))
        .ok_or_else(|| {
            let detail = String::from_utf8_lossy(&output.stderr);
            if detail.trim().is_empty() {
                "服务器没有返回可用的 SSH 主机密钥".into()
            } else {
                format!("无法获取 SSH 主机密钥: {}", detail.trim())
            }
        })
}

#[tauri::command]
pub async fn ssh_host_key_preview(
    app: AppHandle,
    profile_id: String,
) -> Result<SshHostKey, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let profile = profile_by_id(&app, &profile_id)?;
        scan_host_key(&profile).map(|(key, _)| key)
    })
    .await
    .map_err(|error| format!("SSH 指纹检查任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn ssh_host_key_trust(
    app: AppHandle,
    profile_id: String,
    expected_fingerprint: String,
) -> Result<SshHostKey, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let profile = profile_by_id(&app, &profile_id)?;
        let (key, key_line) = scan_host_key(&profile)?;
        if key.fingerprint != expected_fingerprint {
            return Err(format!(
                "主机指纹发生变化，已拒绝保存。当前指纹为 {}",
                key.fingerprint
            ));
        }
        let path = known_hosts_path(&app)?;
        let directory = path
            .parent()
            .ok_or_else(|| "known_hosts 路径无效".to_string())?;
        ensure_private_directory(directory)?;
        let existing = fs::read_to_string(&path).unwrap_or_default();
        let mut lines = existing
            .lines()
            .filter(|line| line.split_whitespace().next() != Some(key.host.as_str()))
            .map(str::to_string)
            .collect::<Vec<_>>();
        lines.push(key_line);
        let contents = format!("{}\n", lines.join("\n"));
        let temporary = path.with_extension("tmp");
        fs::write(&temporary, contents).map_err(|error| format!("无法保存主机指纹: {error}"))?;
        set_owner_only(&temporary)?;
        fs::rename(&temporary, &path).map_err(|error| format!("无法更新 known_hosts: {error}"))?;
        set_owner_only(&path)?;
        Ok(key)
    })
    .await
    .map_err(|error| format!("SSH 指纹保存任务异常结束: {error}"))?
}

fn ssh_command(
    profile: &SshProfile,
    known_hosts: &Path,
    remote_command: &str,
    timeout_seconds: u64,
) -> Result<Command, String> {
    validate_profile(profile)?;
    if remote_command.trim().is_empty() || remote_command.len() > 8192 {
        return Err("远程命令不能为空且不能超过 8192 字节".into());
    }
    let known_hosts_option = ssh_config_path(known_hosts)?;
    let mut command = Command::new("ssh");
    command
        .arg("-T")
        .arg("-F")
        .arg(ssh_config_null_device())
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-o")
        .arg("PasswordAuthentication=no")
        .arg("-o")
        .arg("KbdInteractiveAuthentication=no")
        .arg("-o")
        .arg("StrictHostKeyChecking=yes")
        .arg("-o")
        .arg(format!("UserKnownHostsFile={known_hosts_option}"))
        .arg("-o")
        .arg(format!("GlobalKnownHostsFile={}", ssh_config_null_device()))
        .arg("-o")
        .arg(format!("ConnectTimeout={}", timeout_seconds.min(30)))
        .arg("-o")
        .arg("ServerAliveInterval=15")
        .arg("-o")
        .arg("ServerAliveCountMax=2")
        .arg("-p")
        .arg(profile.port.to_string())
        .arg("-l")
        .arg(&profile.username);
    if !profile.identity_file.is_empty() {
        command
            .arg("-o")
            .arg("IdentitiesOnly=yes")
            .arg("-i")
            .arg(&profile.identity_file);
    }
    command
        .arg(&profile.host)
        .arg(remote_command)
        .stdin(Stdio::null())
        .env("LC_ALL", "C");
    Ok(command)
}

fn ssh_config_null_device() -> &'static str {
    if cfg!(target_os = "windows") {
        "NUL"
    } else {
        "/dev/null"
    }
}

fn ssh_config_path(path: &Path) -> Result<String, String> {
    let value = path
        .to_str()
        .ok_or_else(|| "SSH 配置路径不是有效的 UTF-8".to_string())?;
    #[cfg(target_os = "windows")]
    let value = value.replace('\\', "/");
    #[cfg(not(target_os = "windows"))]
    let value = value.to_string();
    let escaped = value.replace('%', "%%").replace('"', "\\\"");
    Ok(format!("\"{escaped}\""))
}

fn terminal_ssh_arguments(profile: &SshProfile, known_hosts: &Path) -> Result<Vec<String>, String> {
    validate_profile(profile)?;
    let known_hosts_option = ssh_config_path(known_hosts)?;
    let mut arguments = vec![
        "-tt".into(),
        "-F".into(),
        ssh_config_null_device().into(),
        "-o".into(),
        "StrictHostKeyChecking=yes".into(),
        "-o".into(),
        format!("UserKnownHostsFile={known_hosts_option}"),
        "-o".into(),
        format!("GlobalKnownHostsFile={}", ssh_config_null_device()),
        "-o".into(),
        "ConnectTimeout=15".into(),
        "-o".into(),
        "ServerAliveInterval=15".into(),
        "-o".into(),
        "ServerAliveCountMax=2".into(),
        "-p".into(),
        profile.port.to_string(),
        "-l".into(),
        profile.username.clone(),
    ];
    if profile.auth_method == "password" {
        arguments.extend([
            "-o".into(),
            "PubkeyAuthentication=no".into(),
            "-o".into(),
            "PreferredAuthentications=password,keyboard-interactive".into(),
        ]);
    } else {
        if !profile.identity_file.is_empty() {
            arguments.extend([
                "-o".into(),
                "IdentitiesOnly=yes".into(),
                "-i".into(),
                profile.identity_file.clone(),
            ]);
        }
        arguments.extend([
            "-o".into(),
            "PasswordAuthentication=no".into(),
            "-o".into(),
            "KbdInteractiveAuthentication=no".into(),
        ]);
    }
    arguments.push(profile.host.clone());
    Ok(arguments)
}

#[tauri::command]
pub fn ssh_terminal_connect(
    app: AppHandle,
    state: State<'_, SshTerminalState>,
    session_id: String,
    profile_id: String,
    columns: u16,
    rows: u16,
) -> Result<SshTerminalConnection, String> {
    if session_id.len() < 12
        || session_id.len() > 80
        || !session_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("SSH 终端会话标识无效".into());
    }
    if !(40..=400).contains(&columns) || !(10..=200).contains(&rows) {
        return Err("终端尺寸无效".into());
    }
    let profile = profile_by_id(&app, &profile_id)?;
    let known_hosts = known_hosts_path(&app)?;
    if !known_hosts.exists() {
        return Err("尚未信任此服务器的主机指纹".into());
    }
    let arguments = terminal_ssh_arguments(&profile, &known_hosts)?;
    let pair = native_pty_system()
        .openpty(PtySize {
            rows,
            cols: columns,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("无法创建 SSH 终端: {error}"))?;
    let mut command = CommandBuilder::new("ssh");
    for argument in arguments {
        command.arg(argument);
    }
    command.env("LC_ALL", "C");
    command.env("TERM", "xterm-256color");
    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("无法启动系统 OpenSSH: {error}"))?;
    drop(pair.slave);
    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("无法读取 SSH 终端: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("无法写入 SSH 终端: {error}"))?;
    let terminal_session = TerminalSession {
        master: pair.master,
        writer,
        child,
    };
    let mut sessions = state
        .0
        .lock()
        .map_err(|_| "SSH 终端状态锁已损坏".to_string())?;
    if sessions.contains_key(&session_id) {
        drop(terminal_session);
        return Err("SSH 终端会话标识重复".into());
    }
    sessions.insert(session_id.clone(), terminal_session);
    drop(sessions);

    let event_app = app.clone();
    let event_session_id = session_id.clone();
    thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    let _ = event_app.emit(
                        "ssh-terminal-event",
                        SshTerminalEvent {
                            session_id: event_session_id.clone(),
                            event: "data".into(),
                            data: STANDARD.encode(&buffer[..read]),
                        },
                    );
                }
                Err(error) => {
                    let _ = event_app.emit(
                        "ssh-terminal-event",
                        SshTerminalEvent {
                            session_id: event_session_id.clone(),
                            event: "error".into(),
                            data: error.to_string(),
                        },
                    );
                    break;
                }
            }
        }
        if let Ok(mut sessions) = event_app.state::<SshTerminalState>().0.lock() {
            sessions.remove(&event_session_id);
        }
        let _ = event_app.emit(
            "ssh-terminal-event",
            SshTerminalEvent {
                session_id: event_session_id,
                event: "closed".into(),
                data: String::new(),
            },
        );
    });
    Ok(SshTerminalConnection { session_id })
}

#[tauri::command]
pub fn ssh_terminal_input(
    state: State<'_, SshTerminalState>,
    session_id: String,
    data: String,
) -> Result<(), String> {
    if data.len() > 64 * 1024 {
        return Err("单次终端输入不能超过 64 KiB".into());
    }
    let mut sessions = state
        .0
        .lock()
        .map_err(|_| "SSH 终端状态锁已损坏".to_string())?;
    let session = sessions
        .get_mut(&session_id)
        .ok_or_else(|| "SSH 终端连接已关闭".to_string())?;
    session
        .writer
        .write_all(data.as_bytes())
        .map_err(|error| format!("无法写入 SSH 终端: {error}"))?;
    session
        .writer
        .flush()
        .map_err(|error| format!("无法刷新 SSH 终端: {error}"))
}

#[tauri::command]
pub fn ssh_terminal_resize(
    state: State<'_, SshTerminalState>,
    session_id: String,
    columns: u16,
    rows: u16,
) -> Result<(), String> {
    if !(40..=400).contains(&columns) || !(10..=200).contains(&rows) {
        return Err("终端尺寸无效".into());
    }
    let sessions = state
        .0
        .lock()
        .map_err(|_| "SSH 终端状态锁已损坏".to_string())?;
    let session = sessions
        .get(&session_id)
        .ok_or_else(|| "SSH 终端连接已关闭".to_string())?;
    session
        .master
        .resize(PtySize {
            rows,
            cols: columns,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("无法调整 SSH 终端尺寸: {error}"))
}

#[tauri::command]
pub fn ssh_terminal_disconnect(
    state: State<'_, SshTerminalState>,
    session_id: String,
) -> Result<(), String> {
    let session = state
        .0
        .lock()
        .map_err(|_| "SSH 终端状态锁已损坏".to_string())?
        .remove(&session_id);
    drop(session);
    Ok(())
}

fn trusted_fingerprints(path: &Path, profile: &SshProfile) -> Result<Vec<String>, String> {
    let contents =
        fs::read_to_string(path).map_err(|error| format!("无法读取主机指纹: {error}"))?;
    let bracketed = format!("[{}]:{}", profile.host, profile.port);
    let fingerprints = contents
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let host = parts.next()?;
            let _key_type = parts.next()?;
            let encoded = parts.next()?;
            if host != profile.host && host != bracketed {
                return None;
            }
            let decoded = STANDARD.decode(encoded).ok()?;
            Some(format!(
                "SHA256:{}",
                STANDARD_NO_PAD.encode(Sha256::digest(decoded))
            ))
        })
        .collect::<Vec<_>>();
    if fingerprints.is_empty() {
        Err("尚未信任此服务器的主机指纹".into())
    } else {
        Ok(fingerprints)
    }
}

#[derive(Clone)]
struct PasswordSshClient {
    trusted_fingerprints: Arc<Vec<String>>,
}

impl russh::client::Handler for PasswordSshClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        let fingerprint = server_public_key
            .fingerprint(Default::default())
            .to_string();
        Ok(self.trusted_fingerprints.contains(&fingerprint))
    }
}

async fn execute_with_password(
    profile: SshProfile,
    known_hosts: &Path,
    remote_command: String,
    password: String,
    timeout_seconds: u64,
) -> Result<SshCommandResult, String> {
    let trusted = trusted_fingerprints(known_hosts, &profile)?;
    let started = Instant::now();
    let future = async move {
        let config = Arc::new(russh::client::Config {
            inactivity_timeout: Some(Duration::from_secs(timeout_seconds)),
            ..Default::default()
        });
        let handler = PasswordSshClient {
            trusted_fingerprints: Arc::new(trusted),
        };
        let mut session =
            russh::client::connect(config, (profile.host.as_str(), profile.port), handler)
                .await
                .map_err(|error| format!("SSH 连接失败: {error}"))?;
        let authentication = session
            .authenticate_password(&profile.username, password)
            .await
            .map_err(|error| format!("SSH 密码认证失败: {error}"))?;
        if !authentication.success() {
            return Err("SSH 密码认证失败，请检查用户名或密码".into());
        }

        let mut channel = session
            .channel_open_session()
            .await
            .map_err(|error| format!("无法打开 SSH 会话: {error}"))?;
        channel
            .exec(true, remote_command)
            .await
            .map_err(|error| format!("无法执行远程命令: {error}"))?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = None;
        let mut truncated = false;
        while let Some(message) = channel.wait().await {
            match message {
                russh::ChannelMsg::Data { data } => {
                    append_capped(&mut stdout, data.as_ref(), &mut truncated);
                }
                russh::ChannelMsg::ExtendedData { data, .. } => {
                    append_capped(&mut stderr, data.as_ref(), &mut truncated);
                }
                russh::ChannelMsg::ExitStatus { exit_status } => {
                    exit_code = i32::try_from(exit_status).ok();
                }
                _ => {}
            }
        }
        let _ = session
            .disconnect(russh::Disconnect::ByApplication, "", "zh-CN")
            .await;
        Ok::<_, String>((stdout, stderr, exit_code, truncated))
    };

    match tokio::time::timeout(Duration::from_secs(timeout_seconds), future).await {
        Ok(Ok((stdout, stderr, exit_code, truncated))) => Ok(SshCommandResult {
            success: exit_code == Some(0),
            exit_code,
            stdout: String::from_utf8_lossy(&stdout).into_owned(),
            stderr: String::from_utf8_lossy(&stderr).into_owned(),
            elapsed_millis: started.elapsed().as_millis(),
            truncated,
            timed_out: false,
        }),
        Ok(Err(error)) => Err(error),
        Err(_) => Ok(SshCommandResult {
            success: false,
            exit_code: None,
            stdout: String::new(),
            stderr: "远程命令执行超时".into(),
            elapsed_millis: started.elapsed().as_millis(),
            truncated: false,
            timed_out: true,
        }),
    }
}

fn append_capped(target: &mut Vec<u8>, source: &[u8], truncated: &mut bool) {
    let remaining = MAX_OUTPUT_BYTES.saturating_sub(target.len());
    if source.len() > remaining {
        *truncated = true;
    }
    target.extend_from_slice(&source[..source.len().min(remaining)]);
}

#[tauri::command]
pub async fn ssh_command_execute(
    app: AppHandle,
    profile_id: String,
    command: String,
    timeout_seconds: u64,
    password: Option<String>,
) -> Result<SshCommandResult, String> {
    if !(1..=300).contains(&timeout_seconds) {
        return Err("命令超时时间必须在 1 到 300 秒之间".into());
    }
    let profile = profile_by_id(&app, &profile_id)?;
    let known_hosts = known_hosts_path(&app)?;
    if !known_hosts.exists() {
        return Err("尚未信任此服务器的主机指纹".into());
    }
    if profile.auth_method == "password" {
        let password = password.filter(|value| !value.is_empty()).ok_or_else(|| {
            "请输入 SSH 密码；密码只保留在当前应用会话，不会写入配置文件".to_string()
        })?;
        if password.len() > 1024 {
            return Err("SSH 密码长度不能超过 1024 字节".into());
        }
        return execute_with_password(profile, &known_hosts, command, password, timeout_seconds)
            .await;
    }
    tauri::async_runtime::spawn_blocking(move || {
        let process = ssh_command(&profile, &known_hosts, &command, timeout_seconds)?;
        let output = run_with_limit(
            process,
            Duration::from_secs(timeout_seconds),
            MAX_OUTPUT_BYTES,
        )?;
        Ok(SshCommandResult {
            success: output.success && !output.timed_out,
            exit_code: output.status_code,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            elapsed_millis: output.elapsed_millis,
            truncated: output.truncated,
            timed_out: output.timed_out,
        })
    })
    .await
    .map_err(|error| format!("SSH 命令任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn ssh_connection_test(
    app: AppHandle,
    profile_id: String,
    password: Option<String>,
) -> Result<SshCommandResult, String> {
    ssh_command_execute(
        app,
        profile_id,
        "printf '__ZHIYU_SSH_OK__\\n'; uname -s; uname -m".into(),
        15,
        password,
    )
    .await
}

fn run_with_limit(
    mut command: Command,
    timeout: Duration,
    max_bytes: usize,
) -> Result<ProcessOutput, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let started = Instant::now();
    let mut child = command
        .spawn()
        .map_err(|error| format!("无法启动系统 OpenSSH: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法读取 SSH 标准输出".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "无法读取 SSH 错误输出".to_string())?;
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take((max_bytes + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr
            .take((max_bytes + 1) as u64)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });

    let (status, timed_out) = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("无法等待 SSH 进程: {error}"))?
        {
            break (status, false);
        }
        if started.elapsed() >= timeout {
            let _ = child.kill();
            let status = child
                .wait()
                .map_err(|error| format!("无法终止超时的 SSH 进程: {error}"))?;
            break (status, true);
        }
        thread::sleep(Duration::from_millis(30));
    };

    let mut stdout = stdout_reader
        .join()
        .map_err(|_| "SSH 标准输出读取线程异常结束".to_string())?
        .map_err(|error| format!("无法读取 SSH 标准输出: {error}"))?;
    let mut stderr = stderr_reader
        .join()
        .map_err(|_| "SSH 错误输出读取线程异常结束".to_string())?
        .map_err(|error| format!("无法读取 SSH 错误输出: {error}"))?;
    let truncated = stdout.len() > max_bytes || stderr.len() > max_bytes;
    stdout.truncate(max_bytes);
    stderr.truncate(max_bytes);
    Ok(ProcessOutput {
        status_code: status.code(),
        success: status.success(),
        stdout,
        stderr,
        truncated,
        timed_out,
        elapsed_millis: started.elapsed().as_millis(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> SshProfile {
        SshProfile {
            id: String::new(),
            name: "本地服务器".into(),
            host: "127.0.0.1".into(),
            port: 22,
            username: "developer".into(),
            identity_file: String::new(),
            auth_method: "key".into(),
            created_at_millis: 0,
            updated_at_millis: 0,
        }
    }

    #[test]
    fn profile_rejects_argument_injection_and_invalid_usernames() {
        let mut value = profile();
        value.host = "-oProxyCommand=evil".into();
        assert!(validate_profile(&value).is_err());
        value.host = "example.com".into();
        value.username = "root;rm".into();
        assert!(validate_profile(&value).is_err());
    }

    #[test]
    fn profile_accepts_hosts_and_bounded_ports() {
        assert!(validate_profile(&profile()).is_ok());
        let mut value = profile();
        value.host = "server.internal".into();
        value.port = 2222;
        value.username = "deploy-user".into();
        assert!(validate_profile(&value).is_ok());
    }

    #[test]
    fn output_is_capped() {
        let mut command = Command::new("printf");
        command.arg("hello");
        let output = run_with_limit(command, Duration::from_secs(2), 3).unwrap();
        assert_eq!(output.stdout, b"hel");
        assert!(output.truncated);
    }

    #[test]
    fn older_profiles_default_to_key_authentication() {
        let profile: SshProfile = serde_json::from_str(
            r#"{
                "id":"ssh-old",
                "name":"旧连接",
                "host":"example.com",
                "port":22,
                "username":"deploy",
                "identityFile":"",
                "createdAtMillis":0,
                "updatedAtMillis":0
            }"#,
        )
        .unwrap();
        assert_eq!(profile.auth_method, "key");
    }

    #[test]
    fn interactive_password_terminal_disables_public_key_fallback() {
        let mut value = profile();
        value.auth_method = "password".into();
        let arguments =
            terminal_ssh_arguments(&value, Path::new("/tmp/zhiyu-known-hosts")).unwrap();
        assert!(arguments
            .windows(2)
            .any(|pair| { pair == ["-o".to_string(), "PubkeyAuthentication=no".to_string()] }));
        assert!(!arguments.iter().any(|argument| argument.contains("secret")));
    }

    #[test]
    fn known_hosts_path_is_quoted_for_openssh_config_parser() {
        let path = Path::new("/Users/developer/Library/Application Support/智屿/known_hosts");
        let quoted = ssh_config_path(path).unwrap();
        assert_eq!(
            quoted,
            "\"/Users/developer/Library/Application Support/智屿/known_hosts\""
        );
        let arguments = terminal_ssh_arguments(&profile(), path).unwrap();
        assert!(arguments.iter().any(|argument| {
            argument
                == "UserKnownHostsFile=\"/Users/developer/Library/Application Support/智屿/known_hosts\""
        }));
    }

    #[test]
    fn interactive_key_terminal_uses_selected_identity() {
        let identity = tempfile::NamedTempFile::new().unwrap();
        let mut value = profile();
        value.identity_file = identity.path().to_string_lossy().into_owned();
        let arguments =
            terminal_ssh_arguments(&value, Path::new("/tmp/zhiyu-known-hosts")).unwrap();
        assert!(arguments
            .windows(2)
            .any(|pair| { pair == ["-i".to_string(), value.identity_file.clone()] }));
        assert!(arguments
            .windows(2)
            .any(|pair| { pair == ["-o".to_string(), "PasswordAuthentication=no".to_string()] }));
    }
}
