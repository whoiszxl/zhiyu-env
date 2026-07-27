pub(crate) mod commands;
mod privacy;
pub(crate) mod repository;

use repository::{ClipboardItem, ClipboardRepo, ClipboardSettings};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;

const STATE_STOPPED: u8 = 0;
const STATE_RUNNING: u8 = 1;
const STATE_PAUSED: u8 = 2;

pub(crate) struct ClipboardService {
    repo: Arc<ClipboardRepo>,
    state: Arc<AtomicU8>,
    generation: Arc<AtomicU64>,
    last_self_hash: Arc<Mutex<Option<String>>>,
}

impl ClipboardService {
    pub(crate) fn new() -> Result<Self, String> {
        let db_path = repository::db_path()?;
        let repo = Arc::new(ClipboardRepo::open(&db_path)?);
        Ok(Self {
            repo,
            state: Arc::new(AtomicU8::new(STATE_STOPPED)),
            generation: Arc::new(AtomicU64::new(0)),
            last_self_hash: Arc::new(Mutex::new(None)),
        })
    }

    pub(crate) fn start_watching(&self, app: AppHandle) -> Result<(), String> {
        let current = self.state.load(Ordering::SeqCst);
        if current == STATE_RUNNING {
            return Ok(());
        }
        self.state.store(STATE_RUNNING, Ordering::SeqCst);
        let gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;

        let repo = self.repo.clone();
        let state = self.state.clone();
        let generation = self.generation.clone();
        let last_self_hash = self.last_self_hash.clone();

        tauri::async_runtime::spawn(async move {
            watcher::run(repo, state, generation, last_self_hash, gen, app).await;
        });
        Ok(())
    }

    pub(crate) fn stop_watching(&self) {
        self.state.store(STATE_STOPPED, Ordering::SeqCst);
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub(crate) fn pause(&self) {
        self.state.store(STATE_PAUSED, Ordering::SeqCst);
    }

    pub(crate) fn resume(&self) {
        self.state.store(STATE_RUNNING, Ordering::SeqCst);
    }

    pub(crate) fn run_state(&self) -> &'static str {
        match self.state.load(Ordering::SeqCst) {
            STATE_RUNNING => "running",
            STATE_PAUSED => "paused",
            _ => "stopped",
        }
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
        let h = hash_str(&item.content);
        *self.last_self_hash.lock().map_err(|e| e.to_string())? = Some(h);
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
        s.run_state = self.run_state().to_string();
        Ok(s)
    }

    pub(crate) fn settings_get(&self) -> Result<ClipboardSettings, String> {
        self.repo.load_settings()
    }

    pub(crate) fn settings_save(&self, settings: ClipboardSettings) -> Result<(), String> {
        self.repo.save_settings(&settings)?;
        self.repo.evict()?;
        Ok(())
    }
}

fn set_clipboard_text(text: &str) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("无法访问剪贴板: {e}"))?;
    clipboard
        .set_text(text)
        .map_err(|e| format!("无法写入剪贴板: {e}"))
}

fn hash_str(s: &str) -> String {
    format!("{:x}", Sha256::digest(s.as_bytes()))
}

mod watcher {
    use super::repository::ClipboardRepo;
    use sha2::{Digest, Sha256};
    use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tauri::{AppHandle, Emitter};

    use super::{STATE_PAUSED, STATE_STOPPED};

    const POLL_INTERVAL_MS: u64 = 800;
    const PAUSE_POLL_INTERVAL_MS: u64 = 2000;

    pub(super) async fn run(
        repo: Arc<ClipboardRepo>,
        state: Arc<AtomicU8>,
        generation: Arc<AtomicU64>,
        last_self_hash: Arc<Mutex<Option<String>>>,
        my_gen: u64,
        app: AppHandle,
    ) {
        let mut last_clipboard_hash = String::new();

        loop {
            if generation.load(Ordering::SeqCst) != my_gen {
                break;
            }
            let current_state = state.load(Ordering::SeqCst);
            if current_state == STATE_STOPPED {
                break;
            }
            let delay_ms = if current_state == STATE_PAUSED {
                PAUSE_POLL_INTERVAL_MS
            } else {
                POLL_INTERVAL_MS
            };
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;

            if generation.load(Ordering::SeqCst) != my_gen {
                break;
            }
            if state.load(Ordering::SeqCst) == STATE_PAUSED {
                continue;
            }

            let text = match read_clipboard_text() {
                Some(t) => t,
                None => continue,
            };
            let hash = format!("{:x}", Sha256::digest(text.as_bytes()));
            if hash == last_clipboard_hash {
                continue;
            }
            last_clipboard_hash = hash.clone();

            // Skip content written by ourselves (copy_item)
            let skip = {
                let mut guard = last_self_hash.lock().unwrap_or_else(|e| e.into_inner());
                let matches = guard.as_ref() == Some(&hash);
                if matches {
                    *guard = None;
                }
                matches
            };
            if skip {
                continue;
            }

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
}
