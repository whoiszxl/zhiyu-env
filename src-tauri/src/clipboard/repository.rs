use rusqlite::{params, Connection};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use super::privacy;

pub(crate) const DEFAULT_MAX_ITEMS: u32 = 500;
pub(crate) const DEFAULT_RETENTION_DAYS: u32 = 30;
const MAX_CONTENT_BYTES: usize = 256 * 1024;
const MAX_DB_SIZE_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardItem {
    pub id: i64,
    pub content: String,
    pub content_type: String,
    pub preview: String,
    pub char_count: u32,
    pub copied_at_millis: u64,
    pub last_used_at_millis: u64,
    pub use_count: u32,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardStatus {
    pub item_count: u32,
    pub pinned_count: u32,
    pub db_size_bytes: u64,
    pub monitoring: bool,
}

pub(crate) struct ClipboardRepo {
    conn: Mutex<Connection>,
}

impl ClipboardRepo {
    pub(crate) fn open(db_path: &PathBuf) -> Result<Self, String> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("无法创建剪贴板目录: {e}"))?;
        }
        let conn = Connection::open(db_path).map_err(|e| format!("无法打开剪贴板数据库: {e}"))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
            .map_err(|e| format!("SQLite 配置失败: {e}"))?;
        let repo = Self {
            conn: Mutex::new(conn),
        };
        repo.migrate()?;
        Ok(repo)
    }

    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clipboard_items (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                content         TEXT    NOT NULL,
                content_hash    TEXT    NOT NULL UNIQUE,
                content_type    TEXT    NOT NULL DEFAULT 'text',
                preview         TEXT    NOT NULL DEFAULT '',
                char_count      INTEGER NOT NULL DEFAULT 0,
                copied_at_millis    INTEGER NOT NULL,
                last_used_at_millis INTEGER NOT NULL,
                use_count       INTEGER NOT NULL DEFAULT 1,
                pinned          INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_clipboard_copied_at
                ON clipboard_items(copied_at_millis DESC);
            CREATE INDEX IF NOT EXISTS idx_clipboard_pinned
                ON clipboard_items(pinned);
            CREATE INDEX IF NOT EXISTS idx_clipboard_content_type
                ON clipboard_items(content_type);",
        )
        .map_err(|e| format!("剪贴板数据库迁移失败: {e}"))?;
        Ok(())
    }

    pub(crate) fn insert(&self, content: String) -> Result<Option<ClipboardItem>, String> {
        if content.len() > MAX_CONTENT_BYTES {
            return Ok(None);
        }
        if privacy::is_sensitive(&content) {
            return Ok(None);
        }

        let content_type = classify(&content);
        let preview = make_preview(&content);
        let char_count = content.chars().count() as u32;
        let content_hash = hash_hex(Sha256::digest(content.as_bytes()));
        let now_ms = now_millis();

        let conn = self.conn.lock().map_err(|e| e.to_string())?;

        // 去重：内容已存在时只更新时间，不新增记录
        let updated = conn
            .execute(
                "UPDATE clipboard_items
                    SET last_used_at_millis = ?1,
                        copied_at_millis = ?1,
                        use_count = use_count + 1
                  WHERE content_hash = ?2",
                params![now_ms as i64, content_hash],
            )
            .map_err(|e| e.to_string())?;

        if updated > 0 {
            let mut stmt = conn
                .prepare("SELECT id FROM clipboard_items WHERE content_hash = ?1")
                .map_err(|e| e.to_string())?;
            let id: i64 = stmt
                .query_row(params![content_hash], |row| row.get(0))
                .map_err(|e| e.to_string())?;
            return Ok(Some(ClipboardItem {
                id,
                content,
                content_type,
                preview,
                char_count,
                copied_at_millis: now_ms,
                last_used_at_millis: now_ms,
                use_count: 0,
                pinned: false,
            }));
        }

        conn.execute(
            "INSERT INTO clipboard_items (content, content_hash, content_type, preview, char_count, copied_at_millis, last_used_at_millis, use_count, pinned)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6, 1, 0)",
            params![content, content_hash, content_type, preview, char_count, now_ms as i64],
        )
        .map_err(|e| e.to_string())?;

        let id = conn.last_insert_rowid();

        // 淘汰：删除超出数量限制的非置顶记录 + 过期记录
        drop(conn);
        self.evict()?;

        Ok(Some(ClipboardItem {
            id,
            content,
            content_type,
            preview,
            char_count,
            copied_at_millis: now_ms,
            last_used_at_millis: now_ms,
            use_count: 1,
            pinned: false,
        }))
    }

    pub(crate) fn list(
        &self,
        search: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ClipboardItem>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let (sql, params_vec): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = if let Some(q) = search {
            let pattern = format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"));
            (
                "SELECT id, content, content_type, preview, char_count, copied_at_millis, last_used_at_millis, use_count, pinned
                   FROM clipboard_items
                  WHERE content LIKE ?1 ESCAPE '\\'
                  ORDER BY pinned DESC, copied_at_millis DESC
                  LIMIT ?2 OFFSET ?3"
                    .into(),
                vec![
                    Box::new(pattern) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(limit),
                    Box::new(offset),
                ],
            )
        } else {
            (
                "SELECT id, content, content_type, preview, char_count, copied_at_millis, last_used_at_millis, use_count, pinned
                   FROM clipboard_items
                  ORDER BY pinned DESC, copied_at_millis DESC
                  LIMIT ?1 OFFSET ?2"
                    .into(),
                vec![
                    Box::new(limit) as Box<dyn rusqlite::types::ToSql>,
                    Box::new(offset),
                ],
            )
        };

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let param_refs: Vec<&dyn rusqlite::types::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        let rows = stmt
            .query_map(param_refs.as_slice(), |row| {
                Ok(ClipboardItem {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    content_type: row.get(2)?,
                    preview: row.get(3)?,
                    char_count: row.get(4)?,
                    copied_at_millis: row.get::<_, i64>(5)? as u64,
                    last_used_at_millis: row.get::<_, i64>(6)? as u64,
                    use_count: row.get(7)?,
                    pinned: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut items = Vec::new();
        for row in rows {
            items.push(row.map_err(|e| e.to_string())?);
        }
        Ok(items)
    }

    pub(crate) fn get_by_id(&self, id: i64) -> Result<Option<ClipboardItem>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, content, content_type, preview, char_count, copied_at_millis, last_used_at_millis, use_count, pinned
                   FROM clipboard_items WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let result = stmt
            .query_row(params![id], |row| {
                Ok(ClipboardItem {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    content_type: row.get(2)?,
                    preview: row.get(3)?,
                    char_count: row.get(4)?,
                    copied_at_millis: row.get::<_, i64>(5)? as u64,
                    last_used_at_millis: row.get::<_, i64>(6)? as u64,
                    use_count: row.get(7)?,
                    pinned: row.get(8)?,
                })
            })
            .ok();
        Ok(result)
    }

    pub(crate) fn mark_used(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE clipboard_items SET last_used_at_millis = ?1, use_count = use_count + 1 WHERE id = ?2",
            params![now_millis() as i64, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub(crate) fn toggle_pin(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "UPDATE clipboard_items SET pinned = CASE WHEN pinned = 0 THEN 1 ELSE 0 END WHERE id = ?1",
            params![id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub(crate) fn delete(&self, id: i64) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM clipboard_items WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub(crate) fn clear_unpinned(&self) -> Result<u32, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let deleted = conn
            .execute("DELETE FROM clipboard_items WHERE pinned = 0", [])
            .map_err(|e| e.to_string())?;
        Ok(deleted as u32)
    }

    fn evict(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let cutoff_ms = (now_millis() as i64)
            .saturating_sub((DEFAULT_RETENTION_DAYS as i64) * 24 * 60 * 60 * 1000);

        // 1. 删除过期记录（pinned 的不删）
        conn.execute(
            "DELETE FROM clipboard_items WHERE pinned = 0 AND copied_at_millis < ?1",
            params![cutoff_ms],
        )
        .map_err(|e| e.to_string())?;

        // 2. 超出数量限制时，删除最旧的非置顶记录
        conn.execute(
            "DELETE FROM clipboard_items WHERE id IN (
                SELECT id FROM clipboard_items
                 WHERE pinned = 0
                 ORDER BY copied_at_millis ASC
                 LIMIT MAX(0, (SELECT COUNT(*) FROM clipboard_items WHERE pinned = 0) - ?1)
             )",
            params![DEFAULT_MAX_ITEMS],
        )
        .map_err(|e| e.to_string())?;

        // 3. 数据库文件超过上限时压缩
        drop(conn);
        self.vacuum_if_needed()?;

        Ok(())
    }

    fn vacuum_if_needed(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let page_count: i64 = conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .unwrap_or(0);
        let page_size: i64 = conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))
            .unwrap_or(4096);
        let db_bytes = (page_count * page_size) as u64;
        if db_bytes > MAX_DB_SIZE_BYTES {
            conn.execute("VACUUM", []).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub(crate) fn status(&self) -> Result<ClipboardStatus, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let item_count: u32 = conn
            .query_row("SELECT COUNT(*) FROM clipboard_items", [], |row| row.get(0))
            .unwrap_or(0);
        let pinned_count: u32 = conn
            .query_row(
                "SELECT COUNT(*) FROM clipboard_items WHERE pinned = 1",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0);
        let page_count: i64 = conn
            .query_row("PRAGMA page_count", [], |row| row.get(0))
            .unwrap_or(0);
        let page_size: i64 = conn
            .query_row("PRAGMA page_size", [], |row| row.get(0))
            .unwrap_or(4096);
        Ok(ClipboardStatus {
            item_count,
            pinned_count,
            db_size_bytes: (page_count * page_size) as u64,
            monitoring: false,
        })
    }
}

pub(crate) fn db_path() -> Result<PathBuf, String> {
    let dir = dirs::data_dir()
        .ok_or_else(|| "无法确定应用数据目录".to_string())?
        .join("dev.zhiyu.env")
        .join("clipboard");
    Ok(dir.join("clipboard.db"))
}

fn classify(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return "url".into();
    }
    if trimmed.contains('\n') || trimmed.contains('\t') {
        let brief = trimmed.lines().take(2).collect::<Vec<_>>().join(" ");
        if brief.contains("SELECT ") || brief.contains("function ") || brief.contains("def ") || brief.contains("import ") {
            return "code".into();
        }
    }
    "text".into()
}

fn make_preview(content: &str) -> String {
    let first_line = content.lines().next().unwrap_or("");
    if first_line.chars().count() <= 120 {
        first_line.to_string()
    } else {
        first_line.chars().take(117).collect::<String>() + "..."
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn hash_hex(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_ID: AtomicU32 = AtomicU32::new(0);

    fn test_repo() -> ClipboardRepo {
        let id = TEST_ID.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("zhiyu-clipboard-test-{}-{}", std::process::id(), id));
        std::fs::create_dir_all(&dir).ok();
        ClipboardRepo::open(&dir.join("test.db")).unwrap()
    }

    #[test]
    fn insert_and_list() {
        let repo = test_repo();
        repo.insert("hello world".into()).unwrap();
        repo.insert("SELECT * FROM users".into()).unwrap();
        let items = repo.list(None, 10, 0).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn dedup_same_content() {
        let repo = test_repo();
        repo.insert("same content".into()).unwrap();
        repo.insert("same content".into()).unwrap();
        let items = repo.list(None, 10, 0).unwrap();
        assert_eq!(items.len(), 1);
    }

    #[test]
    fn skips_sensitive() {
        let repo = test_repo();
        assert!(repo.insert("-----BEGIN PRIVATE KEY-----".into()).unwrap().is_none());
        assert!(repo.insert("123456".into()).unwrap().is_none());
    }

    #[test]
    fn classify_types() {
        assert_eq!(classify("http://localhost:8080"), "url");
        assert_eq!(classify("https://example.com"), "url");
        assert_eq!(classify("hello\nworld"), "text");
        assert_eq!(
            classify("SELECT *\nFROM users\nWHERE id = 1"),
            "code"
        );
    }
}
