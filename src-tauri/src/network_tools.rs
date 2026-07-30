use reqwest::header::RANGE;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::Read;
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_ADDRESSES: usize = 4;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDiagnosticInput {
    target: String,
    mode: String,
    port: Option<u16>,
    timeout_seconds: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDiagnosticResult {
    target: String,
    host: String,
    port: u16,
    mode: String,
    dns_millis: u128,
    addresses: Vec<ResolvedAddress>,
    tcp_attempts: Vec<TcpAttempt>,
    http: Option<HttpProbe>,
    tls: Option<TlsProbe>,
    port_owner: Option<crate::port_tools::PortListener>,
    proxies: Vec<ProxySetting>,
    findings: Vec<NetworkFinding>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedAddress {
    address: String,
    family: &'static str,
    local: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpAttempt {
    address: String,
    connected: bool,
    elapsed_millis: u128,
    error: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpProbe {
    url: String,
    status_code: u16,
    status_text: String,
    elapsed_millis: u128,
    effective_url: String,
    server: String,
    content_type: String,
    content_length: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsProbe {
    success: bool,
    elapsed_millis: u128,
    protocol: String,
    cipher_suite: String,
    alpn: String,
    certificate_count: usize,
    sha256_fingerprint: String,
    error: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxySetting {
    source: String,
    name: String,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkFinding {
    level: &'static str,
    code: &'static str,
    detail: String,
}

struct Target {
    url: Option<String>,
    host: String,
    port: u16,
    mode: String,
}

#[tauri::command]
pub async fn network_diagnose(
    input: NetworkDiagnosticInput,
) -> Result<NetworkDiagnosticResult, String> {
    tauri::async_runtime::spawn_blocking(move || diagnose(input))
        .await
        .map_err(|error| format!("网络诊断任务异常：{error}"))?
}

#[tauri::command]
pub fn network_proxy_settings() -> Vec<ProxySetting> {
    proxy_settings()
}

fn diagnose(input: NetworkDiagnosticInput) -> Result<NetworkDiagnosticResult, String> {
    if !(1..=15).contains(&input.timeout_seconds) {
        return Err("超时时间必须在 1 到 15 秒之间".into());
    }
    let target = parse_target(&input)?;
    let timeout = Duration::from_secs(input.timeout_seconds);

    let dns_started = Instant::now();
    let mut socket_addresses = (target.host.as_str(), target.port)
        .to_socket_addrs()
        .map_err(|error| format!("DNS 解析失败：{error}"))?
        .collect::<Vec<_>>();
    let dns_millis = dns_started.elapsed().as_millis();
    socket_addresses.sort();
    socket_addresses.dedup();
    if socket_addresses.is_empty() {
        return Err("DNS 没有返回可连接的地址".into());
    }

    let addresses = socket_addresses
        .iter()
        .map(|address| ResolvedAddress {
            address: address.ip().to_string(),
            family: if address.is_ipv4() { "IPv4" } else { "IPv6" },
            local: is_local_ip(address.ip()),
        })
        .collect::<Vec<_>>();
    let tcp_attempts = socket_addresses
        .iter()
        .take(MAX_ADDRESSES)
        .map(|address| tcp_probe(*address, timeout))
        .collect::<Vec<_>>();
    let connected = tcp_attempts.iter().any(|attempt| attempt.connected);

    let tls = if target.mode == "https" && connected {
        Some(tls_probe(
            &target.host,
            &socket_addresses,
            target.port,
            timeout,
        ))
    } else {
        None
    };
    let http = if matches!(target.mode.as_str(), "http" | "https") && connected {
        target
            .url
            .as_deref()
            .map(|url| http_probe(url, timeout))
            .transpose()?
    } else {
        None
    };
    let port_owner = if addresses.iter().any(|address| address.local) {
        crate::port_tools::read_port_listeners()
            .ok()
            .and_then(|listeners| listeners.into_iter().find(|item| item.port == target.port))
    } else {
        None
    };
    let proxies = proxy_settings();
    let findings = findings(
        dns_millis,
        &addresses,
        &tcp_attempts,
        http.as_ref(),
        tls.as_ref(),
        port_owner.as_ref(),
    );

    Ok(NetworkDiagnosticResult {
        target: target
            .url
            .clone()
            .unwrap_or_else(|| format!("{}:{}", display_host(&target.host), target.port)),
        host: target.host,
        port: target.port,
        mode: target.mode,
        dns_millis,
        addresses,
        tcp_attempts,
        http,
        tls,
        port_owner,
        proxies,
        findings,
    })
}

fn parse_target(input: &NetworkDiagnosticInput) -> Result<Target, String> {
    let raw = input.target.trim();
    if raw.is_empty() || raw.len() > 2048 || raw.chars().any(|character| character.is_control()) {
        return Err("请输入有效的域名、IP 地址或 URL".into());
    }
    if !matches!(input.mode.as_str(), "auto" | "tcp" | "http" | "https") {
        return Err("诊断模式无效".into());
    }

    let explicit_url = raw.contains("://");
    let candidate = if explicit_url {
        raw.to_string()
    } else if matches!(input.mode.as_str(), "http" | "https") {
        format!("{}://{raw}", input.mode)
    } else {
        format!("tcp://{raw}")
    };
    let parsed = reqwest::Url::parse(&candidate).map_err(|_| "目标地址格式无效".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https" | "tcp") {
        return Err("仅支持 TCP、HTTP 和 HTTPS 目标".into());
    }
    let host = parsed
        .host_str()
        .filter(|host| !host.is_empty())
        .ok_or_else(|| "目标缺少主机名".to_string())?
        .trim_matches(['[', ']'])
        .to_string();
    let mode = if input.mode == "auto" {
        match parsed.scheme() {
            "http" => "http",
            "https" => "https",
            _ => "tcp",
        }
    } else {
        input.mode.as_str()
    }
    .to_string();
    let default_port = match mode.as_str() {
        "https" => 443,
        "http" => 80,
        _ => 80,
    };
    let port = input
        .port
        .or_else(|| parsed.port())
        .or_else(|| parsed.port_or_known_default())
        .unwrap_or(default_port);
    if port == 0 {
        return Err("端口无效".into());
    }
    let url = if matches!(mode.as_str(), "http" | "https") {
        let mut url = if explicit_url {
            parsed
        } else {
            reqwest::Url::parse(&format!("{mode}://{raw}"))
                .map_err(|_| "HTTP 地址格式无效".to_string())?
        };
        if url.scheme() != mode {
            url.set_scheme(&mode)
                .map_err(|_| "无法切换目标协议".to_string())?;
        }
        if input.port.is_some() {
            url.set_port(Some(port))
                .map_err(|_| "无法设置目标端口".to_string())?;
        }
        Some(url.to_string())
    } else {
        None
    };
    Ok(Target {
        url,
        host,
        port,
        mode,
    })
}

fn tcp_probe(address: SocketAddr, timeout: Duration) -> TcpAttempt {
    let started = Instant::now();
    match TcpStream::connect_timeout(&address, timeout) {
        Ok(stream) => {
            let _ = stream.shutdown(std::net::Shutdown::Both);
            TcpAttempt {
                address: address.to_string(),
                connected: true,
                elapsed_millis: started.elapsed().as_millis(),
                error: String::new(),
            }
        }
        Err(error) => TcpAttempt {
            address: address.to_string(),
            connected: false,
            elapsed_millis: started.elapsed().as_millis(),
            error: error.to_string(),
        },
    }
}

fn http_probe(url: &str, timeout: Duration) -> Result<HttpProbe, String> {
    let client = crate::settings::reqwest_client_builder(crate::settings::ProxyScope::Network)?
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|error| format!("无法创建 HTTP 客户端：{error}"))?;
    let started = Instant::now();
    let mut response = client
        .get(url)
        .header(RANGE, "bytes=0-1023")
        .send()
        .map_err(|error| format!("HTTP 检查失败：{error}"))?;
    let elapsed_millis = started.elapsed().as_millis();
    let status = response.status();
    let effective_url = response.url().to_string();
    let headers = response.headers();
    let server = headers
        .get(reqwest::header::SERVER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let content_type = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let content_length = response.content_length();
    let mut preview = [0u8; 1024];
    let _ = response.read(&mut preview);
    Ok(HttpProbe {
        url: url.into(),
        status_code: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").into(),
        elapsed_millis,
        effective_url,
        server,
        content_type,
        content_length,
    })
}

fn tls_probe(host: &str, addresses: &[SocketAddr], port: u16, timeout: Duration) -> TlsProbe {
    let started = Instant::now();
    let result = (|| -> Result<TlsProbe, String> {
        let address = addresses
            .iter()
            .copied()
            .find(|address| address.port() == port)
            .or_else(|| addresses.first().copied())
            .ok_or_else(|| "没有可用于 TLS 的地址".to_string())?;
        let socket = TcpStream::connect_timeout(&address, timeout)
            .map_err(|error| format!("TCP 连接失败：{error}"))?;
        socket
            .set_read_timeout(Some(timeout))
            .map_err(|error| error.to_string())?;
        socket
            .set_write_timeout(Some(timeout))
            .map_err(|error| error.to_string())?;
        let roots = RootCertStore::from_iter(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let config = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let server_name =
            ServerName::try_from(host.to_string()).map_err(|_| "TLS 主机名无效".to_string())?;
        let connection = ClientConnection::new(Arc::new(config), server_name)
            .map_err(|error| format!("创建 TLS 连接失败：{error}"))?;
        let mut stream = StreamOwned::new(connection, socket);
        while stream.conn.is_handshaking() {
            stream
                .conn
                .complete_io(&mut stream.sock)
                .map_err(|error| format!("TLS 握手失败：{error}"))?;
        }
        let certificates = stream.conn.peer_certificates().unwrap_or_default();
        let fingerprint = certificates
            .first()
            .map(|certificate| {
                Sha256::digest(certificate.as_ref())
                    .iter()
                    .map(|byte| format!("{byte:02X}"))
                    .collect::<Vec<_>>()
                    .join(":")
            })
            .unwrap_or_default();
        Ok(TlsProbe {
            success: true,
            elapsed_millis: started.elapsed().as_millis(),
            protocol: stream
                .conn
                .protocol_version()
                .map(|version| format!("{version:?}"))
                .unwrap_or_default(),
            cipher_suite: stream
                .conn
                .negotiated_cipher_suite()
                .map(|suite| format!("{:?}", suite.suite()))
                .unwrap_or_default(),
            alpn: stream
                .conn
                .alpn_protocol()
                .map(|value| String::from_utf8_lossy(value).into_owned())
                .unwrap_or_default(),
            certificate_count: certificates.len(),
            sha256_fingerprint: fingerprint,
            error: String::new(),
        })
    })();
    result.unwrap_or_else(|error| TlsProbe {
        success: false,
        elapsed_millis: started.elapsed().as_millis(),
        protocol: String::new(),
        cipher_suite: String::new(),
        alpn: String::new(),
        certificate_count: 0,
        sha256_fingerprint: String::new(),
        error,
    })
}

fn proxy_settings() -> Vec<ProxySetting> {
    let mut settings = Vec::new();
    let mut seen = BTreeSet::new();
    for key in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
        "no_proxy",
    ] {
        if let Ok(value) = std::env::var(key) {
            if !value.trim().is_empty() && seen.insert((key.to_ascii_uppercase(), value.clone())) {
                settings.push(ProxySetting {
                    source: "Environment".into(),
                    name: key.to_ascii_uppercase(),
                    value: redact_url_password(&value),
                });
            }
        }
    }
    settings
}

fn redact_url_password(value: &str) -> String {
    let Ok(mut url) = reqwest::Url::parse(value) else {
        return value.chars().take(300).collect();
    };
    if url.password().is_some() {
        let _ = url.set_password(Some("••••••"));
    }
    url.to_string()
}

fn findings(
    dns_millis: u128,
    addresses: &[ResolvedAddress],
    tcp: &[TcpAttempt],
    http: Option<&HttpProbe>,
    tls: Option<&TlsProbe>,
    owner: Option<&crate::port_tools::PortListener>,
) -> Vec<NetworkFinding> {
    let mut result = Vec::new();
    if dns_millis > 1000 {
        result.push(NetworkFinding {
            level: "warning",
            code: "dns_slow",
            detail: dns_millis.to_string(),
        });
    }
    if tcp.iter().all(|attempt| !attempt.connected) {
        result.push(NetworkFinding {
            level: "error",
            code: "tcp_failed",
            detail: String::new(),
        });
    } else if tcp.iter().any(|attempt| !attempt.connected) {
        result.push(NetworkFinding {
            level: "warning",
            code: "tcp_partial",
            detail: String::new(),
        });
    }
    if addresses.iter().any(|address| address.local)
        && owner.is_none()
        && tcp.iter().all(|attempt| !attempt.connected)
    {
        result.push(NetworkFinding {
            level: "warning",
            code: "local_no_listener",
            detail: String::new(),
        });
    }
    if let Some(http) = http {
        if http.status_code >= 400 {
            result.push(NetworkFinding {
                level: "warning",
                code: "http_error",
                detail: http.status_code.to_string(),
            });
        }
        if http.elapsed_millis > 3000 {
            result.push(NetworkFinding {
                level: "warning",
                code: "http_slow",
                detail: http.elapsed_millis.to_string(),
            });
        }
    }
    if let Some(tls) = tls {
        if !tls.success {
            result.push(NetworkFinding {
                level: "error",
                code: "tls_failed",
                detail: tls.error.clone(),
            });
        }
    }
    if result.is_empty() {
        result.push(NetworkFinding {
            level: "success",
            code: "healthy",
            detail: String::new(),
        });
    }
    result
}

fn is_local_ip(ip: IpAddr) -> bool {
    ip.is_loopback() || ip.is_unspecified()
}

fn display_host(host: &str) -> String {
    if host.contains(':') {
        format!("[{host}]")
    } else {
        host.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    fn input(target: &str, mode: &str, port: Option<u16>) -> NetworkDiagnosticInput {
        NetworkDiagnosticInput {
            target: target.into(),
            mode: mode.into(),
            port,
            timeout_seconds: 3,
        }
    }

    #[test]
    fn parses_https_url_and_custom_port() {
        let parsed =
            parse_target(&input("https://example.com/api/health", "auto", Some(8443))).unwrap();
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.mode, "https");
        assert_eq!(parsed.port, 8443);
        assert!(parsed.url.unwrap().contains(":8443/api/health"));
    }

    #[test]
    fn parses_ipv6_tcp_target() {
        let parsed = parse_target(&input("[::1]:6379", "tcp", None)).unwrap();
        assert_eq!(parsed.host, "::1");
        assert_eq!(parsed.port, 6379);
    }

    #[test]
    fn rejects_non_network_schemes() {
        assert!(parse_target(&input("file:///etc/hosts", "auto", None)).is_err());
    }

    #[test]
    fn redacts_proxy_passwords() {
        assert_eq!(
            redact_url_password("http://user:secret@127.0.0.1:7890"),
            "http://user:%E2%80%A2%E2%80%A2%E2%80%A2%E2%80%A2%E2%80%A2%E2%80%A2@127.0.0.1:7890/"
        );
    }

    #[test]
    fn diagnoses_a_local_http_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 1024];
                if stream.read(&mut request).unwrap_or(0) > 0 {
                    stream
                        .write_all(
                            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nServer: test\r\n\r\nOK",
                        )
                        .unwrap();
                }
            }
        });
        let result = diagnose(input(
            &format!("http://127.0.0.1:{port}/health"),
            "auto",
            None,
        ))
        .unwrap();
        server.join().unwrap();
        assert!(result.tcp_attempts.iter().any(|attempt| attempt.connected));
        assert_eq!(result.http.unwrap().status_code, 200);
    }
}
