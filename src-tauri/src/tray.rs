use crate::commands::{self, LifecycleAction, ServiceInfo, ServiceKindInput};
use crate::settings;
use serde::Serialize;
use std::thread;
use std::time::Duration;
use tauri::{
    menu::{CheckMenuItemBuilder, Menu, MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    App, AppHandle, Emitter, Manager,
};

const TRAY_ID: &str = "zhiyu-main-tray";
const NAVIGATE_EVENT: &str = "tray:navigate";
const ACTION_EVENT: &str = "tray:service-action";
const REFRESH_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone)]
struct TrayService {
    id: &'static str,
    name: &'static str,
    version: String,
    port: u16,
    status: &'static str,
}

#[derive(Clone)]
struct TraySnapshot {
    services: Vec<TrayService>,
    running_count: usize,
    memory_bytes: Option<u64>,
    launch_at_login: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct NavigationEvent {
    target: &'static str,
    kind: Option<&'static str>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ServiceActionEvent {
    success: bool,
    message: String,
}

pub(crate) fn setup(app: &mut App) -> tauri::Result<()> {
    let snapshot = collect_snapshot();
    let menu = build_menu(app.handle(), &snapshot)?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or_else(|| tauri::Error::AssetNotFound("default window icon".into()))?;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .icon_as_template(cfg!(target_os = "macos"))
        .tooltip(tooltip(&snapshot))
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;

    start_refresh_loop(app.handle().clone());
    Ok(())
}

fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id().as_ref();
    match id {
        "tray.open" => show_main_window(app),
        "tray.overview" => navigate(app, "overview", None),
        "tray.settings" => navigate(app, "settings", None),
        "tray.stop_all" => spawn_stop_all(app.clone()),
        "tray.toggle_autostart" => spawn_toggle_autostart(app.clone()),
        "tray.quit" => app.exit(0),
        _ => {
            if let Some(kind) = id.strip_prefix("tray.open_service.") {
                navigate(app, "service", Some(kind));
                return;
            }
            if let Some((kind, action)) = parse_service_action(id) {
                spawn_service_action(app.clone(), kind, action);
            }
        }
    }
}

fn navigate(app: &AppHandle, target: &'static str, kind: Option<&str>) {
    show_main_window(app);
    let kind = kind.and_then(service_kind).map(service_id);
    let _ = app.emit(NAVIGATE_EVENT, NavigationEvent { target, kind });
}

fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn spawn_service_action(app: AppHandle, kind: ServiceKindInput, action: LifecycleAction) {
    thread::spawn(move || {
        let action_name = match action {
            LifecycleAction::Start => "启动",
            LifecycleAction::Stop => "停止",
            LifecycleAction::Restart => "重启",
        };
        let result = commands::lifecycle_action(kind, action);
        let event = match result {
            Ok(service) => ServiceActionEvent {
                success: true,
                message: format!("{}{}成功", service.name, action_name),
            },
            Err(error) => ServiceActionEvent {
                success: false,
                message: error,
            },
        };
        let _ = app.emit(ACTION_EVENT, event);
        schedule_refresh(&app);
    });
}

fn spawn_stop_all(app: AppHandle) {
    thread::spawn(move || {
        let mut stopped = 0;
        let mut failures = Vec::new();
        if let Ok(services) = commands::service_list() {
            for service in services
                .into_iter()
                .filter(|service| service.status == "running")
            {
                let Some(kind) = service_kind(service.kind.as_str()) else {
                    continue;
                };
                match commands::lifecycle_action(kind, LifecycleAction::Stop) {
                    Ok(_) => stopped += 1,
                    Err(error) => failures.push(format!("{}: {error}", service.name)),
                }
            }
        }
        let event = if failures.is_empty() {
            ServiceActionEvent {
                success: true,
                message: format!("已停止 {stopped} 个服务"),
            }
        } else {
            ServiceActionEvent {
                success: false,
                message: format!("部分服务停止失败：{}", failures.join("；")),
            }
        };
        let _ = app.emit(ACTION_EVENT, event);
        schedule_refresh(&app);
    });
}

fn spawn_toggle_autostart(app: AppHandle) {
    thread::spawn(move || {
        let event = match settings::toggle_launch_at_login(&app) {
            Ok(true) => ServiceActionEvent {
                success: true,
                message: "已启用开机启动智屿".into(),
            },
            Ok(false) => ServiceActionEvent {
                success: true,
                message: "已关闭开机启动智屿".into(),
            },
            Err(error) => ServiceActionEvent {
                success: false,
                message: error,
            },
        };
        let _ = app.emit(ACTION_EVENT, event);
        schedule_refresh(&app);
    });
}

fn start_refresh_loop(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(REFRESH_INTERVAL);
        schedule_refresh(&app);
    });
}

fn schedule_refresh(app: &AppHandle) {
    let snapshot = collect_snapshot();
    let handle = app.clone();
    let dispatch = handle.clone();
    let _ = dispatch.run_on_main_thread(move || {
        let _ = apply_snapshot(&handle, &snapshot);
    });
}

fn collect_snapshot() -> TraySnapshot {
    let services = commands::service_list().unwrap_or_default();
    let running_count = services
        .iter()
        .filter(|service| service.status == "running")
        .count();
    let memory_bytes = commands::collect_environment_metrics_from(&services)
        .ok()
        .map(|metrics| metrics.memory_bytes);
    let launch_at_login = settings::load_settings().launch_at_login;
    TraySnapshot {
        services: services.into_iter().filter_map(tray_service).collect(),
        running_count,
        memory_bytes,
        launch_at_login,
    }
}

fn tray_service(service: ServiceInfo) -> Option<TrayService> {
    (service.status != "not_installed").then(|| TrayService {
        id: service_id(service_kind(service.kind.as_str()).expect("registered service kind")),
        name: service.name,
        version: service.version,
        port: service.port,
        status: service.status,
    })
}

fn apply_snapshot(app: &AppHandle, snapshot: &TraySnapshot) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    tray.set_menu(Some(build_menu(app, snapshot)?))?;
    tray.set_tooltip(Some(tooltip(snapshot)))?;
    Ok(())
}

fn build_menu(app: &AppHandle, snapshot: &TraySnapshot) -> tauri::Result<Menu<tauri::Wry>> {
    let summary = MenuItemBuilder::with_id(
        "tray.summary",
        format!(
            "智屿 · {} 个服务运行中 · {}",
            snapshot.running_count,
            snapshot
                .memory_bytes
                .map(format_bytes)
                .unwrap_or_else(|| "内存暂不可用".into())
        ),
    )
    .enabled(false)
    .build(app)?;

    let mut services = SubmenuBuilder::with_id(
        app,
        "tray.services",
        format!("服务管理（{} 个运行）", snapshot.running_count),
    );
    if snapshot.services.is_empty() {
        let empty = MenuItemBuilder::with_id("tray.services.empty", "暂无已安装服务")
            .enabled(false)
            .build(app)?;
        services = services.item(&empty);
    } else {
        for service in &snapshot.services {
            let marker = if service.status == "running" {
                "●"
            } else if matches!(service.status, "crashed" | "stale_pid") {
                "!"
            } else {
                "○"
            };
            let mut submenu = SubmenuBuilder::with_id(
                app,
                format!("tray.service.{}", service.id),
                format!(
                    "{marker} {} {} · {}",
                    service.name, service.version, service.port
                ),
            );
            let status = MenuItemBuilder::with_id(
                format!("tray.status.{}", service.id),
                service_status_label(service.status),
            )
            .enabled(false)
            .build(app)?;
            submenu = submenu.item(&status);
            if service.status == "running" {
                submenu = submenu
                    .text(format!("tray.action.{}.restart", service.id), "重新启动")
                    .text(format!("tray.action.{}.stop", service.id), "停止");
            } else {
                submenu = submenu.text(format!("tray.action.{}.start", service.id), "启动");
            }
            submenu = submenu
                .separator()
                .text(format!("tray.open_service.{}", service.id), "打开详情");
            services = services.item(&submenu.build()?);
        }
    }
    let services = services.build()?;
    let autostart = CheckMenuItemBuilder::with_id("tray.toggle_autostart", "开机启动智屿")
        .checked(snapshot.launch_at_login)
        .build(app)?;
    let stop_all = MenuItemBuilder::with_id("tray.stop_all", "停止全部服务")
        .enabled(snapshot.running_count > 0)
        .build(app)?;
    let quit_text = if snapshot.running_count > 0 {
        "退出智屿（服务继续运行）"
    } else {
        "退出智屿"
    };

    MenuBuilder::with_id(app, "zhiyu-tray-menu")
        .item(&summary)
        .separator()
        .text("tray.open", "打开智屿")
        .text("tray.overview", "打开全局概览")
        .separator()
        .item(&services)
        .item(&stop_all)
        .separator()
        .item(&autostart)
        .text("tray.settings", "设置中心")
        .separator()
        .text("tray.quit", quit_text)
        .build()
}

fn parse_service_action(id: &str) -> Option<(ServiceKindInput, LifecycleAction)> {
    let remainder = id.strip_prefix("tray.action.")?;
    let (kind, action) = remainder.rsplit_once('.')?;
    let kind = service_kind(kind)?;
    let action = match action {
        "start" => LifecycleAction::Start,
        "stop" => LifecycleAction::Stop,
        "restart" => LifecycleAction::Restart,
        _ => return None,
    };
    Some((kind, action))
}

fn service_kind(id: &str) -> Option<ServiceKindInput> {
    Some(match id {
        "redis" => ServiceKindInput::Redis,
        "mysql" => ServiceKindInput::Mysql,
        "postgres" => ServiceKindInput::Postgres,
        "mongodb" => ServiceKindInput::Mongodb,
        "mailpit" => ServiceKindInput::Mailpit,
        "nats" => ServiceKindInput::Nats,
        "meilisearch" => ServiceKindInput::Meilisearch,
        "minio" => ServiceKindInput::Minio,
        "rustfs" => ServiceKindInput::Rustfs,
        "etcd" => ServiceKindInput::Etcd,
        "consul" => ServiceKindInput::Consul,
        "rnacos" => ServiceKindInput::Rnacos,
        "rabbitmq" => ServiceKindInput::Rabbitmq,
        _ => return None,
    })
}

fn service_id(kind: ServiceKindInput) -> &'static str {
    match kind {
        ServiceKindInput::Redis => "redis",
        ServiceKindInput::Mysql => "mysql",
        ServiceKindInput::Postgres => "postgres",
        ServiceKindInput::Mongodb => "mongodb",
        ServiceKindInput::Mailpit => "mailpit",
        ServiceKindInput::Nats => "nats",
        ServiceKindInput::Meilisearch => "meilisearch",
        ServiceKindInput::Minio => "minio",
        ServiceKindInput::Rustfs => "rustfs",
        ServiceKindInput::Etcd => "etcd",
        ServiceKindInput::Consul => "consul",
        ServiceKindInput::Rnacos => "rnacos",
        ServiceKindInput::Rabbitmq => "rabbitmq",
    }
}

fn service_status_label(status: &str) -> &'static str {
    match status {
        "running" => "运行中",
        "stopped" => "已停止",
        "crashed" => "进程意外退出",
        "stale_pid" => "PID 状态异常",
        _ => "状态未知",
    }
}

fn tooltip(snapshot: &TraySnapshot) -> String {
    format!("智屿 · {} 个服务运行中", snapshot.running_count)
}

fn format_bytes(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * MIB;
    if bytes as f64 >= GIB {
        format!("{:.1} GB", bytes as f64 / GIB)
    } else {
        format!("{:.0} MB", bytes as f64 / MIB)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_service_actions() {
        assert!(matches!(
            parse_service_action("tray.action.redis.restart"),
            Some((ServiceKindInput::Redis, LifecycleAction::Restart))
        ));
        assert!(parse_service_action("tray.action.redis.install").is_none());
        assert!(parse_service_action("other.redis.start").is_none());
    }

    #[test]
    fn formats_memory_for_tray_summary() {
        assert_eq!(format_bytes(512 * 1024 * 1024), "512 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
    }
}
