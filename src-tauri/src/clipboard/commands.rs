use super::ClipboardService;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, State};

pub(crate) struct ClipboardState(pub Mutex<Option<ClipboardService>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Empty {}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClipboardStartResult {
    pub run_state: String,
}

#[tauri::command]
pub async fn clipboard_start(
    state: State<'_, ClipboardState>,
    app: AppHandle,
) -> Result<ClipboardStartResult, String> {
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    if guard.is_none() {
        *guard = Some(ClipboardService::new()?);
    }
    guard.as_ref().unwrap().start_watching(app)?;
    Ok(ClipboardStartResult {
        run_state: guard.as_ref().unwrap().run_state().to_string(),
    })
}

#[tauri::command]
pub async fn clipboard_stop(state: State<'_, ClipboardState>) -> Result<Empty, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    if let Some(svc) = guard.as_ref() {
        svc.stop_watching();
    }
    Ok(Empty {})
}

#[tauri::command]
pub async fn clipboard_pause(state: State<'_, ClipboardState>) -> Result<Empty, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(svc) => {
            svc.pause();
            Ok(Empty {})
        }
        None => Err("剪贴板服务未启动".into()),
    }
}

#[tauri::command]
pub async fn clipboard_resume(state: State<'_, ClipboardState>) -> Result<Empty, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(svc) => {
            svc.resume();
            Ok(Empty {})
        }
        None => Err("剪贴板服务未启动".into()),
    }
}

#[tauri::command]
pub async fn clipboard_status(
    state: State<'_, ClipboardState>,
) -> Result<super::repository::ClipboardStatus, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(svc) => svc.status(),
        None => Ok(super::repository::ClipboardStatus {
            item_count: 0,
            pinned_count: 0,
            db_size_bytes: 0,
            run_state: "stopped".into(),
        }),
    }
}

#[tauri::command]
pub async fn clipboard_list(
    state: State<'_, ClipboardState>,
    search: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<super::repository::ClipboardItem>, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(svc) => svc.list(search.as_deref(), limit.unwrap_or(50), offset.unwrap_or(0)),
        None => Err("剪贴板服务未启动".into()),
    }
}

#[tauri::command]
pub async fn clipboard_copy(state: State<'_, ClipboardState>, id: i64) -> Result<String, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(svc) => svc.copy_item(id),
        None => Err("剪贴板服务未启动".into()),
    }
}

#[tauri::command]
pub async fn clipboard_pin(state: State<'_, ClipboardState>, id: i64) -> Result<Empty, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(svc) => svc.pin(id).map(|_| Empty {}),
        None => Err("剪贴板服务未启动".into()),
    }
}

#[tauri::command]
pub async fn clipboard_delete(state: State<'_, ClipboardState>, id: i64) -> Result<Empty, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(svc) => svc.delete(id).map(|_| Empty {}),
        None => Err("剪贴板服务未启动".into()),
    }
}

#[tauri::command]
pub async fn clipboard_clear(state: State<'_, ClipboardState>) -> Result<u32, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(svc) => svc.clear(),
        None => Err("剪贴板服务未启动".into()),
    }
}

#[tauri::command]
pub async fn clipboard_settings_get(
    state: State<'_, ClipboardState>,
) -> Result<super::repository::ClipboardSettings, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(svc) => svc.settings_get(),
        None => Err("剪贴板服务未启动".into()),
    }
}

#[tauri::command]
pub async fn clipboard_settings_save(
    state: State<'_, ClipboardState>,
    settings: super::repository::ClipboardSettings,
) -> Result<super::repository::ClipboardStatus, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(svc) => {
            svc.settings_save(settings)?;
            svc.status()
        }
        None => Err("剪贴板服务未启动".into()),
    }
}
