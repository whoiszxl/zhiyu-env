use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock, RwLock,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_REQUEST_BYTES: usize = 1024 * 1024;
const MAX_REQUEST_LOGS: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MockRoute {
    pub id: String,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub content_type: String,
    pub response_body: String,
    pub delay_ms: u64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockRequestLog {
    pub id: u128,
    pub timestamp_millis: u128,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub matched_route_id: Option<String>,
    pub body_preview: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MockApiState {
    pub running: bool,
    pub port: u16,
    pub base_url: String,
    pub routes: Vec<MockRoute>,
    pub recent_requests: Vec<MockRequestLog>,
}

struct MockServer {
    port: u16,
    stop: Arc<AtomicBool>,
}

struct MockRuntime {
    server: Option<MockServer>,
    preferred_port: u16,
    routes: Arc<RwLock<Vec<MockRoute>>>,
    logs: Arc<Mutex<VecDeque<MockRequestLog>>>,
}

static RUNTIME: OnceLock<Mutex<MockRuntime>> = OnceLock::new();

fn runtime() -> &'static Mutex<MockRuntime> {
    RUNTIME.get_or_init(|| {
        Mutex::new(MockRuntime {
            server: None,
            preferred_port: 9321,
            routes: Arc::new(RwLock::new(load_routes())),
            logs: Arc::new(Mutex::new(VecDeque::new())),
        })
    })
}

fn routes_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("zhiyu-env/mock-api-routes.json"))
}

fn default_routes() -> Vec<MockRoute> {
    vec![MockRoute {
        id: "hello".into(),
        method: "GET".into(),
        path: "/api/hello".into(),
        status_code: 200,
        content_type: "application/json; charset=utf-8".into(),
        response_body: "{\n  \"message\": \"你好，智屿 Mock API\"\n}".into(),
        delay_ms: 0,
        enabled: true,
    }]
}

fn load_routes() -> Vec<MockRoute> {
    read_persisted_routes().unwrap_or_else(default_routes)
}

fn read_persisted_routes() -> Option<Vec<MockRoute>> {
    routes_path()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

fn validate_routes(routes: &mut [MockRoute]) -> Result<(), String> {
    for route in routes {
        route.method = route.method.trim().to_ascii_uppercase();
        route.path = route.path.trim().to_string();
        route.content_type = route.content_type.trim().to_string();
        if route.id.trim().is_empty() {
            return Err("接口 ID 不能为空".into());
        }
        if !matches!(
            route.method.as_str(),
            "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
        ) {
            return Err(format!("不支持的请求方法：{}", route.method));
        }
        if !route.path.starts_with('/') || route.path.contains('?') {
            return Err(format!(
                "接口路径必须以 / 开头且不包含查询参数：{}",
                route.path
            ));
        }
        if !(100..=599).contains(&route.status_code) {
            return Err("HTTP 状态码必须在 100 到 599 之间".into());
        }
        if route.content_type.is_empty() {
            return Err("响应 Content-Type 不能为空".into());
        }
        if route.delay_ms > 10_000 {
            return Err("响应延迟不能超过 10000 毫秒".into());
        }
        if route.response_body.len() > MAX_REQUEST_BYTES {
            return Err("单个 Mock 响应不能超过 1 MiB".into());
        }
    }
    Ok(())
}

fn persist_routes(routes: &[MockRoute]) -> Result<(), String> {
    let path = routes_path().ok_or_else(|| "无法确定应用配置目录".to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "配置文件路径无效".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(routes).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

fn snapshot(runtime: &MockRuntime) -> MockApiState {
    let routes = runtime
        .routes
        .read()
        .map(|value| value.clone())
        .unwrap_or_default();
    let recent_requests = runtime
        .logs
        .lock()
        .map(|value| value.iter().rev().cloned().collect())
        .unwrap_or_default();
    let port = runtime
        .server
        .as_ref()
        .map(|server| server.port)
        .unwrap_or(runtime.preferred_port);
    MockApiState {
        running: runtime.server.is_some(),
        port,
        base_url: format!("http://127.0.0.1:{port}"),
        routes,
        recent_requests,
    }
}

#[tauri::command]
pub fn mock_api_state() -> Result<MockApiState, String> {
    let runtime = runtime().lock().map_err(|_| "Mock 服务状态不可用")?;
    Ok(snapshot(&runtime))
}

#[tauri::command]
pub fn mock_api_save_routes(mut routes: Vec<MockRoute>) -> Result<MockApiState, String> {
    validate_routes(&mut routes)?;
    persist_routes(&routes)?;
    let runtime = runtime().lock().map_err(|_| "Mock 服务状态不可用")?;
    *runtime.routes.write().map_err(|_| "Mock 路由状态不可用")? = routes;
    Ok(snapshot(&runtime))
}

#[tauri::command]
pub fn mock_api_start(port: u16, mut routes: Vec<MockRoute>) -> Result<MockApiState, String> {
    if port < 1024 {
        return Err("Mock 服务端口必须在 1024 到 65535 之间".into());
    }
    validate_routes(&mut routes)?;
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|error| format!("无法监听 127.0.0.1:{port}，端口可能已被占用：{error}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("无法设置 Mock 服务监听模式：{error}"))?;

    let mut runtime = runtime().lock().map_err(|_| "Mock 服务状态不可用")?;
    if runtime.server.is_some() {
        return Err("Mock API 服务已经在运行".into());
    }
    persist_routes(&routes)?;
    *runtime.routes.write().map_err(|_| "Mock 路由状态不可用")? = routes;
    runtime.preferred_port = port;

    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker_routes = Arc::clone(&runtime.routes);
    let worker_logs = Arc::clone(&runtime.logs);
    thread::Builder::new()
        .name("zhiyu-mock-api".into())
        .spawn(move || {
            while !worker_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        // 监听器为了支持停止检查使用非阻塞模式。macOS 上 accept
                        // 得到的连接可能同样处于非阻塞状态，浏览器建立连接后稍晚
                        // 才写入请求时，首次 read 会得到 WouldBlock。业务连接必须
                        // 恢复阻塞读取，并由 read_timeout 控制最长等待时间。
                        let _ = stream.set_nonblocking(false);
                        handle_connection(stream, &worker_routes, &worker_logs);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(25));
                    }
                    Err(_) => break,
                }
            }
        })
        .map_err(|error| format!("无法启动 Mock 服务线程：{error}"))?;

    runtime.server = Some(MockServer { port, stop });
    Ok(snapshot(&runtime))
}

#[tauri::command]
pub fn mock_api_stop() -> Result<MockApiState, String> {
    let mut runtime = runtime().lock().map_err(|_| "Mock 服务状态不可用")?;
    if let Some(server) = runtime.server.take() {
        server.stop.store(true, Ordering::Relaxed);
        let _ = TcpStream::connect(("127.0.0.1", server.port));
    }
    Ok(snapshot(&runtime))
}

#[tauri::command]
pub fn mock_api_clear_requests() -> Result<MockApiState, String> {
    let runtime = runtime().lock().map_err(|_| "Mock 服务状态不可用")?;
    runtime.logs.lock().map_err(|_| "请求日志不可用")?.clear();
    Ok(snapshot(&runtime))
}

fn handle_connection(
    mut stream: TcpStream,
    routes: &Arc<RwLock<Vec<MockRoute>>>,
    logs: &Arc<Mutex<VecDeque<MockRequestLog>>>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let mut request = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                request.extend_from_slice(&buffer[..count]);
                if request.len() >= MAX_REQUEST_BYTES || request_complete(&request) {
                    break;
                }
            }
            Err(_) => break,
        }
    }

    let request_text = String::from_utf8_lossy(&request);
    let mut lines = request_text.lines();
    let first_line = lines.next().unwrap_or_default();
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_ascii_uppercase();
    let target = parts.next().unwrap_or("/");
    let path = normalize_request_path(target);
    if method.is_empty() {
        // 浏览器可能建立预连接但不立即发送 HTTP 请求。空连接直接关闭，
        // 不能返回 404，否则浏览器会把该响应误认为实际页面请求的结果。
        return;
    }
    let body = request_text
        .split_once("\r\n\r\n")
        .map(|(_, value)| value)
        .unwrap_or_default();

    if method == "OPTIONS" {
        write_response(&mut stream, 204, "text/plain", "", false);
        push_log(logs, method, target, 204, None, body);
        return;
    }

    let mut matched = find_matching_route(routes, &method, path);
    if matched.is_none() {
        // 如果监听端口的进程与保存配置的窗口不是同一个实例，内存中的规则可能
        // 暂时落后。仅在未命中时从磁盘同步一次，不增加正常请求的文件读取开销。
        if let Some(mut persisted) = read_persisted_routes() {
            if validate_routes(&mut persisted).is_ok() {
                matched = persisted
                    .iter()
                    .find(|route| route_matches(route, &method, path))
                    .cloned();
                if matched.is_some() {
                    if let Ok(mut active) = routes.write() {
                        *active = persisted;
                    }
                }
            }
        }
    }
    let (status, matched_id) = if let Some(route) = matched {
        if route.delay_ms > 0 {
            thread::sleep(Duration::from_millis(route.delay_ms));
        }
        let head_only = method == "HEAD";
        write_response(
            &mut stream,
            route.status_code,
            &route.content_type,
            &route.response_body,
            head_only,
        );
        (route.status_code, Some(route.id))
    } else {
        let available_routes = routes
            .read()
            .map(|routes| {
                routes
                    .iter()
                    .filter(|route| route.enabled)
                    .map(|route| format!("{} {}", route.method, route.path))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let diagnostic = serde_json::json!({
            "error": "No matching mock route",
            "requestMethod": method,
            "requestPath": path,
            "availableRoutes": available_routes,
        })
        .to_string();
        write_response(
            &mut stream,
            404,
            "application/json; charset=utf-8",
            &diagnostic,
            false,
        );
        (404, None)
    };
    push_log(logs, method, target, status, matched_id, body);
}

fn find_matching_route(
    routes: &Arc<RwLock<Vec<MockRoute>>>,
    method: &str,
    path: &str,
) -> Option<MockRoute> {
    routes.read().ok().and_then(|routes| {
        routes
            .iter()
            .find(|route| route_matches(route, method, path))
            .cloned()
    })
}

fn route_matches(route: &MockRoute, method: &str, request_path: &str) -> bool {
    route.enabled
        && route.method.eq_ignore_ascii_case(method)
        && normalized_path_for_match(&route.path) == normalized_path_for_match(request_path)
}

fn normalized_path_for_match(path: &str) -> &str {
    let path = normalize_request_path(path);
    if path.len() > 1 {
        path.strip_suffix('/').unwrap_or(path)
    } else {
        path
    }
}

/// 浏览器直连通常使用 `/api/path`，经过系统代理时可能使用
/// `http://127.0.0.1:9321/api/path`。两种都是合法的 HTTP 请求目标。
fn normalize_request_path(target: &str) -> &str {
    let without_query = target.split('?').next().unwrap_or("/");
    let scheme_length = if without_query
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
    {
        Some(7)
    } else if without_query
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
    {
        Some(8)
    } else {
        None
    };
    if let Some(scheme_length) = scheme_length {
        let after_scheme = &without_query[scheme_length..];
        after_scheme
            .find('/')
            .map(|index| &after_scheme[index..])
            .unwrap_or("/")
    } else {
        without_query
    }
}

fn request_complete(request: &[u8]) -> bool {
    let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") else {
        return false;
    };
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    request.len() >= header_end + 4 + content_length
}

fn push_log(
    logs: &Arc<Mutex<VecDeque<MockRequestLog>>>,
    method: String,
    path: &str,
    status_code: u16,
    matched_route_id: Option<String>,
    body: &str,
) {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    if let Ok(mut logs) = logs.lock() {
        logs.push_back(MockRequestLog {
            id: timestamp,
            timestamp_millis: timestamp,
            method,
            path: path.to_string(),
            status_code,
            matched_route_id,
            body_preview: body.chars().take(500).collect(),
        });
        while logs.len() > MAX_REQUEST_LOGS {
            logs.pop_front();
        }
    }
}

fn write_response(
    stream: &mut TcpStream,
    status_code: u16,
    content_type: &str,
    body: &str,
    head_only: bool,
) {
    let reason = status_reason(status_code);
    let response = format!(
        "HTTP/1.1 {status_code} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store, no-cache, must-revalidate\r\nPragma: no-cache\r\nExpires: 0\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: *\r\nAccess-Control-Allow-Methods: GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    if !head_only {
        let _ = stream.write_all(body.as_bytes());
    }
    let _ = stream.flush();
}

fn status_reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Mock Response",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_route_path_and_delay() {
        let mut routes = default_routes();
        assert!(validate_routes(&mut routes).is_ok());
        routes[0].path = "api/hello".into();
        assert!(validate_routes(&mut routes).is_err());
    }

    #[test]
    fn detects_complete_request_with_body() {
        let request = b"POST /api HTTP/1.1\r\nContent-Length: 4\r\n\r\ntest";
        assert!(request_complete(request));
        assert!(!request_complete(&request[..request.len() - 1]));
    }

    #[test]
    fn normalizes_direct_and_proxy_request_targets() {
        assert_eq!(
            normalize_request_path("/api/hello?from=browser"),
            "/api/hello"
        );
        assert_eq!(
            normalize_request_path("http://127.0.0.1:9321/api/hello?from=proxy"),
            "/api/hello"
        );
        assert_eq!(
            normalize_request_path("HTTP://127.0.0.1:9321/api/hello"),
            "/api/hello"
        );
        assert_eq!(normalize_request_path("https://example.com"), "/");
        assert_eq!(normalized_path_for_match("/api/hello/"), "/api/hello");
    }

    #[test]
    #[ignore = "requires permission to bind a localhost TCP port"]
    fn serves_a_route_and_records_the_request() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let routes = Arc::new(RwLock::new(default_routes()));
        let logs = Arc::new(Mutex::new(VecDeque::new()));
        let worker_routes = Arc::clone(&routes);
        let worker_logs = Arc::clone(&logs);
        let worker = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_connection(stream, &worker_routes, &worker_logs);
        });

        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .write_all(b"GET /api/hello HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        worker.join().unwrap();

        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("你好，智屿 Mock API"));
        let logs = logs.lock().unwrap();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].matched_route_id.as_deref(), Some("hello"));
    }
}
