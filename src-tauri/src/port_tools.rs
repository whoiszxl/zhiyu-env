use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

const LSOF_PATH: &str = "/usr/sbin/lsof";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortListener {
    port: u16,
    address: String,
    pid: u32,
    process: String,
    managed_service: Option<String>,
    common_service: Option<&'static str>,
}

#[tauri::command]
pub async fn port_listeners() -> Result<Vec<PortListener>, String> {
    tauri::async_runtime::spawn_blocking(read_port_listeners)
        .await
        .map_err(|error| format!("端口检查任务异常结束: {error}"))?
}

fn read_port_listeners() -> Result<Vec<PortListener>, String> {
    if !Path::new(LSOF_PATH).is_file() {
        return Err("当前系统没有可用的 lsof 端口检查工具".into());
    }

    let output = Command::new(LSOF_PATH)
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-Fpcn"])
        .output()
        .map_err(|error| format!("无法执行端口检查: {error}"))?;
    if !output.status.success() && output.stdout.is_empty() {
        return Err(format!(
            "端口检查失败: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let managed = managed_service_pids();
    Ok(parse_lsof(
        &String::from_utf8_lossy(&output.stdout),
        &managed,
    ))
}

fn parse_lsof(input: &str, managed: &BTreeMap<u32, String>) -> Vec<PortListener> {
    let mut current_pid = None;
    let mut current_process = String::new();
    let mut seen = BTreeSet::new();
    let mut listeners = Vec::new();

    for line in input.lines() {
        let Some((field, value)) = line.split_at_checked(1) else {
            continue;
        };
        match field {
            "p" => {
                current_pid = value.parse::<u32>().ok();
                current_process.clear();
            }
            "c" => current_process = value.to_string(),
            "n" => {
                let (Some(pid), Some((address, port))) = (current_pid, parse_address(value)) else {
                    continue;
                };
                if !seen.insert((pid, address.clone(), port)) {
                    continue;
                }
                listeners.push(PortListener {
                    port,
                    address,
                    pid,
                    process: current_process.clone(),
                    managed_service: managed.get(&pid).cloned(),
                    common_service: common_service(port),
                });
            }
            _ => {}
        }
    }

    listeners.sort_by(|left, right| {
        left.port
            .cmp(&right.port)
            .then_with(|| left.pid.cmp(&right.pid))
            .then_with(|| left.address.cmp(&right.address))
    });
    listeners
}

fn parse_address(value: &str) -> Option<(String, u16)> {
    let (address, port) = value.rsplit_once(':')?;
    let port = port.parse::<u16>().ok()?;
    Some((address.to_string(), port))
}

fn managed_service_pids() -> BTreeMap<u32, String> {
    let Ok(devbox_root) = crate::settings::devbox_root() else {
        return BTreeMap::new();
    };
    let root = devbox_root.join("instances");
    [
        ("redis", "Redis"),
        ("mysql", "MySQL"),
        ("postgres", "PostgreSQL"),
        ("mongodb", "MongoDB"),
        ("mailpit", "Mailpit"),
        ("nats", "NATS"),
        ("meilisearch", "Meilisearch"),
        ("minio", "MinIO"),
        ("rustfs", "RustFS"),
        ("etcd", "etcd"),
        ("consul", "Consul"),
        ("rnacos", "rnacos"),
        ("rabbitmq", "RabbitMQ"),
    ]
    .into_iter()
    .filter_map(|(directory, name)| {
        let raw = fs::read_to_string(root.join(directory).join("default/run/service.pid")).ok()?;
        raw.trim()
            .parse::<u32>()
            .ok()
            .map(|pid| (pid, name.to_string()))
    })
    .collect()
}

fn common_service(port: u16) -> Option<&'static str> {
    match port {
        80 => Some("HTTP"),
        443 => Some("HTTPS"),
        1025 => Some("Mailpit SMTP"),
        2379 => Some("etcd Client"),
        2380 => Some("etcd Peer"),
        3000 => Some("常用前端开发端口"),
        3306 => Some("MySQL"),
        4222 => Some("NATS"),
        5000 => Some("常用应用开发端口"),
        5432 => Some("PostgreSQL"),
        5672 => Some("RabbitMQ AMQP"),
        6379 => Some("Redis"),
        7700 => Some("Meilisearch"),
        8000 | 8080 => Some("常用 HTTP 开发端口"),
        8025 => Some("Mailpit Web"),
        8222 => Some("NATS Monitoring"),
        8300 => Some("Consul Server"),
        8301 => Some("Consul LAN Serf"),
        8302 => Some("Consul WAN Serf"),
        8500 => Some("Consul HTTP / UI"),
        8502 => Some("Consul gRPC"),
        8600 => Some("Consul DNS"),
        8848 => Some("Nacos HTTP"),
        9848 => Some("Nacos gRPC"),
        10848 => Some("rnacos Console"),
        15672 => Some("RabbitMQ Management"),
        9000 => Some("S3 Object Storage"),
        9001 => Some("Object Storage Console"),
        9002 => Some("RustFS S3 API"),
        7001 => Some("RustFS Console"),
        27017 => Some("MongoDB"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_deduplicates_lsof_machine_output() {
        let mut managed = BTreeMap::new();
        managed.insert(42, "Redis".to_string());
        let output = concat!(
            "p42\n",
            "credis-server\n",
            "f10\n",
            "n127.0.0.1:6379\n",
            "f11\n",
            "n127.0.0.1:6379\n",
            "p99\n",
            "cnode\n",
            "n[::1]:3000\n",
        );

        let listeners = parse_lsof(output, &managed);

        assert_eq!(listeners.len(), 2);
        assert_eq!(listeners[0].port, 3000);
        assert_eq!(listeners[0].address, "[::1]");
        assert_eq!(listeners[1].managed_service.as_deref(), Some("Redis"));
    }

    #[test]
    fn ignores_non_numeric_ports() {
        assert_eq!(parse_address("127.0.0.1:http"), None);
        assert_eq!(parse_address("*:8080"), Some(("*".to_string(), 8080)));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn reads_live_macos_listeners() {
        let listeners = read_port_listeners().unwrap();
        assert!(listeners.iter().all(|listener| listener.port > 0));
        assert!(listeners.iter().all(|listener| listener.pid > 0));
    }
}
