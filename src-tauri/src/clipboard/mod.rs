mod privacy;
pub(crate) mod commands;
pub(crate) mod repository;

use repository::{ClipboardItem, ClipboardRepo};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardSettings {
    pub enabled: bool,
    pub max_items: u32,
    pub retention_days: u32,
}

impl Default for ClipboardSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            max_items: 500,
            retention_days: 30,
        }
    }
}

pub(crate) struct ClipboardService {
    repo: Arc<ClipboardRepo>,
    monitoring: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

impl ClipboardService {
    pub(crate) fn new() -> Result<Self, String> {
        let db_path = repository::db_path()?;
        let repo = Arc::new(ClipboardRepo::open(&db_path)?);
        Ok(Self {
            repo,
            monitoring: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
        })
    }

    pub(crate) fn start_watching(&self, app: AppHandle) -> Result<(), String> {
        if self.monitoring.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.monitoring.store(true, Ordering::SeqCst);
        let repo = self.repo.clone();
        let monitoring = self.monitoring.clone();
        let paused = self.paused.clone();

        tauri::async_runtime::spawn(async move {
            watcher::run(repo, monitoring, paused, app).await;
        });
        Ok(())
    }

    pub(crate) fn stop_watching(&self) {
        self.monitoring.store(false, Ordering::SeqCst);
    }

    pub(crate) fn pause(&self) -> bool {
        self.paused.store(true, Ordering::SeqCst);
        true
    }

    pub(crate) fn resume(&self) -> bool {
        self.paused.store(false, Ordering::SeqCst);
        true
    }

    pub(crate) fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub(crate) fn is_monitoring(&self) -> bool {
        self.monitoring.load(Ordering::SeqCst)
    }

    pub(crate) fn list(
        &self,
        search: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ClipboardItem>, String> {
        self.repo.list(search, limit, offset)
    }

    pub(crate) fn copy_item(&self, id: i64) -> Result<String, String> {
        let item = self
            .repo
            .get_by_id(id)?
            .ok_or_else(|| "记录不存在".to_string())?;
        self.repo.mark_used(id)?;
        set_clipboard_text(&item.content)?;
        Ok(item.content)
    }

    pub(crate) fn pin(&self, id: i64) -> Result<(), String> {
        self.repo.toggle_pin(id)
    }

    pub(crate) fn delete(&self, id: i64) -> Result<(), String> {
        self.repo.delete(id)
    }

    pub(crate) fn clear(&self) -> Result<u32, String> {
        self.repo.clear_unpinned()
    }

    pub(crate) fn status(&self) -> Result<repository::ClipboardStatus, String> {
        let mut s = self.repo.status()?;
        s.monitoring =
            self.monitoring.load(Ordering::SeqCst) && !self.paused.load(Ordering::SeqCst);
        Ok(s)
    }
}

fn set_clipboard_text(text: &str) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("无法访问剪贴板: {e}"))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("无法写入剪贴板: {e}"))
}

mod watcher {
    use super::repository::ClipboardRepo;
    use sha2::{Digest, Sha256};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;
    use tauri::{AppHandle, Emitter};

    const POLL_INTERVAL_MS: u64 = 800;
    const PAUSE_POLL_INTERVAL_MS: u64 = 2000;

    pub(super) async fn run(
        repo: Arc<ClipboardRepo>,
        monitoring: Arc<AtomicBool>,
        paused: Arc<AtomicBool>,
        app: AppHandle,
    ) {
        let mut last_hash = String::new();

        loop {
            if !monitoring.load(Ordering::SeqCst) {
                break;
            }
            let delay_ms = if paused.load(Ordering::SeqCst) {
                PAUSE_POLL_INTERVAL_MS
            } else {
                POLL_INTERVAL_MS
            };
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;

            if paused.load(Ordering::SeqCst) {
                continue;
            }

            let text = match read_clipboard_text() {
                Some(t) => t,
                None => continue,
            };
            let hash = hash_str(&text);
            if hash == last_hash {
                continue;
            }
            last_hash = hash;

            if let Some(item) = repo.insert(text).unwrap_or(None) {
                let _ = app.emit("clipboard:changed", &item);
            }
        }
    }

    fn read_clipboard_text() -> Option<String> {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        let text = clipboard.get_text().ok()?;
        if text.is_empty() {
            None
        } else {
            Some(text)
        }
    }

    fn hash_str(s: &str) -> String {
        format!("{:x}", Sha256::digest(s.as_bytes()))
    }
}
