use feed_rs::model::{Entry, Feed};
use reqwest::blocking::{Client, Response};
use reqwest::header::{CONTENT_TYPE, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_REFRESH_MINUTES: u32 = 30;
const MIN_REFRESH_MINUTES: u32 = 5;
const MAX_RESPONSE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_ENTRY_TEXT_BYTES: usize = 512 * 1024;
const MAX_ENTRIES_PER_FEED: usize = 500;
const RETENTION_DAYS: i64 = 90;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RssFeed {
    pub id: i64,
    pub title: String,
    pub feed_url: String,
    pub site_url: Option<String>,
    pub description: Option<String>,
    pub refresh_interval_minutes: u32,
    pub enabled: bool,
    pub unread_count: u32,
    pub entry_count: u32,
    pub last_refreshed_at_millis: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RssEntry {
    pub id: i64,
    pub feed_id: i64,
    pub feed_title: String,
    pub title: String,
    pub link: Option<String>,
    pub author: Option<String>,
    pub summary: String,
    pub content: String,
    pub published_at_millis: Option<u64>,
    pub fetched_at_millis: u64,
    pub is_read: bool,
    pub is_starred: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RssRefreshResult {
    pub feed_id: i64,
    pub title: String,
    pub added: u32,
    pub updated: u32,
    pub not_modified: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RssImportResult {
    pub imported: u32,
    pub skipped: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RssFeedUpdate {
    pub title: String,
    pub refresh_interval_minutes: u32,
    pub enabled: bool,
}

struct StoredFeed {
    title: String,
    feed_url: String,
    etag: Option<String>,
    last_modified: Option<String>,
}

struct FetchResult {
    parsed: Option<Feed>,
    etag: Option<String>,
    last_modified: Option<String>,
}

fn db_path() -> Result<PathBuf, String> {
    let dir = dirs::data_dir()
        .ok_or_else(|| "无法确定应用数据目录".to_string())?
        .join("dev.zhiyu.env")
        .join("rss");
    Ok(dir.join("rss.db"))
}

pub(crate) fn open_repo() -> Result<Connection, String> {
    open_repo_at(&db_path()?)
}

fn open_repo_at(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("无法创建 RSS 数据目录: {e}"))?;
    }
    let conn = Connection::open(path).map_err(|e| format!("无法打开 RSS 数据库: {e}"))?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         CREATE TABLE IF NOT EXISTS rss_feeds (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           title TEXT NOT NULL,
           custom_title TEXT,
           feed_url TEXT NOT NULL UNIQUE,
           site_url TEXT,
           description TEXT,
           refresh_interval_minutes INTEGER NOT NULL DEFAULT 30,
           enabled INTEGER NOT NULL DEFAULT 1,
           etag TEXT,
           last_modified TEXT,
           last_refreshed_at_millis INTEGER,
           last_error TEXT,
           created_at_millis INTEGER NOT NULL,
           updated_at_millis INTEGER NOT NULL
         );
         CREATE TABLE IF NOT EXISTS rss_entries (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           feed_id INTEGER NOT NULL REFERENCES rss_feeds(id) ON DELETE CASCADE,
           external_id TEXT NOT NULL,
           title TEXT NOT NULL,
           link TEXT,
           author TEXT,
           summary TEXT NOT NULL DEFAULT '',
           content TEXT NOT NULL DEFAULT '',
           published_at_millis INTEGER,
           fetched_at_millis INTEGER NOT NULL,
           is_read INTEGER NOT NULL DEFAULT 0,
           is_starred INTEGER NOT NULL DEFAULT 0,
           UNIQUE(feed_id, external_id)
         );
         CREATE INDEX IF NOT EXISTS idx_rss_entries_feed_date
           ON rss_entries(feed_id, published_at_millis DESC, id DESC);
         CREATE INDEX IF NOT EXISTS idx_rss_entries_read
           ON rss_entries(is_read);
         CREATE INDEX IF NOT EXISTS idx_rss_entries_starred
           ON rss_entries(is_starred);
         CREATE TABLE IF NOT EXISTS rss_meta (
           key TEXT PRIMARY KEY,
           value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS rss_ai_results (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           entry_id INTEGER NOT NULL REFERENCES rss_entries(id) ON DELETE CASCADE,
           action TEXT NOT NULL CHECK(action IN ('summary', 'translate', 'key_points', 'question')),
           question TEXT NOT NULL DEFAULT '',
           output_language TEXT NOT NULL,
           model TEXT NOT NULL,
           source_hash TEXT NOT NULL,
           content TEXT NOT NULL,
           status TEXT NOT NULL CHECK(status IN ('complete', 'partial')),
           created_at_millis INTEGER NOT NULL,
           updated_at_millis INTEGER NOT NULL,
           UNIQUE(entry_id, action, question, output_language, model, source_hash)
         );
         CREATE INDEX IF NOT EXISTS idx_rss_ai_results_entry
           ON rss_ai_results(entry_id, updated_at_millis DESC);",
    )
    .map_err(|e| format!("RSS 数据库迁移失败: {e}"))?;
    migrate_columns(&conn)?;
    retire_legacy_default_feeds(&conn)?;
    Ok(conn)
}

fn migrate_columns(conn: &Connection) -> Result<(), String> {
    let mut stmt = conn
        .prepare("PRAGMA table_info(rss_feeds)")
        .map_err(|e| format!("无法检查 RSS 数据表: {e}"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("无法读取 RSS 数据表结构: {e}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("无法读取 RSS 数据表字段: {e}"))?;
    if !columns.iter().any(|column| column == "custom_title") {
        conn.execute("ALTER TABLE rss_feeds ADD COLUMN custom_title TEXT", [])
            .map_err(|e| format!("RSS 数据库升级失败: {e}"))?;
    }
    Ok(())
}

fn retire_legacy_default_feeds(conn: &Connection) -> Result<(), String> {
    let seeded = conn
        .query_row(
            "SELECT value FROM rss_meta WHERE key = 'default_feeds_seeded'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| format!("无法检查旧版默认订阅: {e}"))?
        .is_some();
    if !seeded {
        return Ok(());
    }

    const LEGACY_URLS: [&str; 7] = [
        "https://blog.rust-lang.org/feed.xml",
        "https://github.blog/feed/",
        "https://blog.cloudflare.com/rss/",
        "https://stackoverflow.blog/feed/",
        "https://engineering.fb.com/feed/",
        "https://kubernetes.io/feed.xml",
        "https://www.ruanyifeng.com/blog/atom.xml",
    ];
    let transaction = conn
        .unchecked_transaction()
        .map_err(|e| format!("无法迁移旧版默认订阅: {e}"))?;
    for url in LEGACY_URLS {
        transaction
            .execute(
                "DELETE FROM rss_feeds
                 WHERE feed_url = ?1
                   AND custom_title = title
                   AND NOT EXISTS (
                     SELECT 1 FROM rss_entries
                     WHERE feed_id = rss_feeds.id AND is_starred = 1
                   )",
                [url],
            )
            .map_err(|e| format!("无法清理旧版默认订阅: {e}"))?;
    }
    transaction
        .execute(
            "DELETE FROM rss_meta WHERE key = 'default_feeds_seeded'",
            [],
        )
        .map_err(|e| format!("无法更新 RSS 初始化状态: {e}"))?;
    transaction
        .commit()
        .map_err(|e| format!("无法完成旧版默认订阅迁移: {e}"))
}

fn normalize_feed_url(raw: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(raw.trim()).map_err(|_| "请输入有效的 RSS 地址".to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err("RSS 地址仅支持 http:// 或 https://".into());
    }
    Ok(url.to_string())
}

fn client() -> Result<Client, String> {
    crate::settings::reqwest_client_builder(crate::settings::ProxyScope::Network)?
        .connect_timeout(Duration::from_secs(8))
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(8))
        .user_agent("ZhiyuEnv/0.1 RSS Reader")
        .build()
        .map_err(|e| format!("无法创建 RSS 请求客户端: {e}"))
}

fn download_feed(feed: &StoredFeed) -> Result<FetchResult, String> {
    let mut request = client()?.get(&feed.feed_url);
    if let Some(value) = feed.etag.as_deref() {
        request = request.header(IF_NONE_MATCH, value);
    }
    if let Some(value) = feed.last_modified.as_deref() {
        request = request.header(IF_MODIFIED_SINCE, value);
    }
    let response = request
        .send()
        .map_err(|e| format!("无法访问订阅地址: {e}"))?;
    if response.status() == reqwest::StatusCode::NOT_MODIFIED {
        return Ok(FetchResult {
            parsed: None,
            etag: feed.etag.clone(),
            last_modified: feed.last_modified.clone(),
        });
    }
    if !response.status().is_success() {
        return Err(format!("订阅地址返回 HTTP {}", response.status()));
    }
    read_and_parse(response, &feed.feed_url)
}

fn feed_source_hint(requested_url: &str) -> &'static str {
    if requested_url.trim_end_matches('/') == "https://www.zhihu.com/rss" {
        " 知乎的旧地址目前会返回空内容，请改用推荐列表中的“知乎日报（社区）”。"
    } else {
        ""
    }
}

fn inspect_payload(
    bytes: &[u8],
    content_type: &str,
    requested_url: &str,
    final_url: &str,
) -> Result<(), String> {
    let hint = feed_source_hint(requested_url);
    if bytes.iter().all(u8::is_ascii_whitespace) {
        return Err(format!(
            "订阅地址返回了空内容，站点可能已停用该 RSS 或拦截第三方客户端。{hint}"
        ));
    }

    let preview = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]);
    let normalized = preview
        .trim_start_matches('\u{feff}')
        .trim_start()
        .to_ascii_lowercase();
    let has_html_root = normalized.starts_with("<!doctype html") || normalized.starts_with("<html");
    let is_html = has_html_root
        || (content_type.to_ascii_lowercase().contains("text/html")
            && (normalized.contains("<title>安全验证")
                || normalized.contains("/account/unhuman")
                || normalized.contains("captcha")));
    if is_html {
        let redirected = if requested_url != final_url {
            format!("，最终跳转到 {final_url}")
        } else {
            String::new()
        };
        return Err(format!(
            "订阅地址返回的是网页而不是 RSS/Atom（Content-Type: {content_type}{redirected}），可能需要登录或触发了反爬验证。{hint}"
        ));
    }

    let looks_like_feed =
        normalized.starts_with('<') || normalized.starts_with('{') || normalized.starts_with('[');
    if !looks_like_feed {
        return Err(format!(
            "订阅地址返回的内容不是可识别的 RSS、Atom 或 JSON Feed（Content-Type: {content_type}）。{hint}"
        ));
    }
    Ok(())
}

fn read_and_parse(response: Response, requested_url: &str) -> Result<FetchResult, String> {
    if response
        .content_length()
        .is_some_and(|size| size > MAX_RESPONSE_BYTES)
    {
        return Err("订阅内容超过 5 MiB，已停止读取".into());
    }
    let final_url = response.url().to_string();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let last_modified = response
        .headers()
        .get(LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let mut bytes = Vec::new();
    response
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("读取订阅内容失败: {e}"))?;
    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        return Err("订阅内容超过 5 MiB，已停止读取".into());
    }
    inspect_payload(&bytes, &content_type, requested_url, &final_url)?;
    let parsed = feed_rs::parser::parse(bytes.as_slice())
        .map_err(|e| {
            format!(
                "订阅内容看起来像 Feed，但格式不完整或不符合 RSS/Atom/JSON Feed 标准（Content-Type: {content_type}）：{e}"
            )
        })?;
    Ok(FetchResult {
        parsed: Some(parsed),
        etag,
        last_modified,
    })
}

fn stored_feed(conn: &Connection, id: i64) -> Result<StoredFeed, String> {
    conn.query_row(
        "SELECT id, COALESCE(custom_title, title), feed_url, etag, last_modified
         FROM rss_feeds WHERE id = ?1",
        [id],
        |row| {
            Ok(StoredFeed {
                title: row.get(1)?,
                feed_url: row.get(2)?,
                etag: row.get(3)?,
                last_modified: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())?
    .ok_or_else(|| "订阅不存在".to_string())
}

fn refresh_feed(id: i64) -> Result<RssRefreshResult, String> {
    let conn = open_repo()?;
    let stored = stored_feed(&conn, id)?;
    drop(conn);
    let fetched = match download_feed(&stored) {
        Ok(value) => value,
        Err(error) => {
            if let Ok(conn) = open_repo() {
                let _ = conn.execute(
                    "UPDATE rss_feeds SET last_error = ?1, last_refreshed_at_millis = ?2,
                     updated_at_millis = ?2 WHERE id = ?3",
                    params![error, now_millis(), id],
                );
            }
            return Err(error);
        }
    };
    let mut conn = open_repo()?;
    if fetched.parsed.is_none() {
        conn.execute(
            "UPDATE rss_feeds SET last_error = NULL, last_refreshed_at_millis = ?1,
             updated_at_millis = ?1 WHERE id = ?2",
            params![now_millis(), id],
        )
        .map_err(|e| e.to_string())?;
        return Ok(RssRefreshResult {
            feed_id: id,
            title: stored.title,
            added: 0,
            updated: 0,
            not_modified: true,
        });
    }
    persist_feed(&mut conn, id, fetched)
}

fn persist_feed(
    conn: &mut Connection,
    id: i64,
    fetched: FetchResult,
) -> Result<RssRefreshResult, String> {
    let feed = fetched.parsed.expect("checked above");
    let now = now_millis();
    let title = feed
        .title
        .as_ref()
        .map(|v| clean_text(&v.content))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "未命名订阅".into());
    let site_url = feed.links.first().map(|link| link.href.clone());
    let description = feed
        .description
        .as_ref()
        .map(|v| truncate(clean_text(&v.content), 4096));
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "UPDATE rss_feeds SET title = ?1, site_url = ?2, description = ?3, etag = ?4,
         last_modified = ?5, last_refreshed_at_millis = ?6, last_error = NULL,
         updated_at_millis = ?6 WHERE id = ?7",
        params![
            title,
            site_url,
            description,
            fetched.etag,
            fetched.last_modified,
            now,
            id
        ],
    )
    .map_err(|e| e.to_string())?;
    let mut added = 0;
    let mut updated = 0;
    for entry in feed.entries.iter().take(MAX_ENTRIES_PER_FEED) {
        let item = entry_fields(entry, now);
        let inserted = tx
            .execute(
                "INSERT INTO rss_entries (
                   feed_id, external_id, title, link, author, summary, content,
                   published_at_millis, fetched_at_millis
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(feed_id, external_id) DO NOTHING",
                params![
                    id,
                    item.external_id,
                    item.title,
                    item.link,
                    item.author,
                    item.summary,
                    item.content,
                    item.published_at_millis,
                    now
                ],
            )
            .map_err(|e| e.to_string())?;
        if inserted == 1 {
            added += 1;
        } else {
            tx.execute(
                "UPDATE rss_entries SET title = ?1, link = ?2, author = ?3,
                 summary = ?4, content = ?5, published_at_millis = ?6,
                 fetched_at_millis = ?7 WHERE feed_id = ?8 AND external_id = ?9",
                params![
                    item.title,
                    item.link,
                    item.author,
                    item.summary,
                    item.content,
                    item.published_at_millis,
                    now,
                    id,
                    item.external_id
                ],
            )
            .map_err(|e| e.to_string())?;
            updated += 1;
        }
    }
    let cutoff = now as i64 - RETENTION_DAYS * 86_400_000;
    tx.execute(
        "DELETE FROM rss_entries WHERE feed_id = ?1 AND is_starred = 0
         AND fetched_at_millis < ?2",
        params![id, cutoff],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(RssRefreshResult {
        feed_id: id,
        title,
        added,
        updated,
        not_modified: false,
    })
}

struct EntryFields {
    external_id: String,
    title: String,
    link: Option<String>,
    author: Option<String>,
    summary: String,
    content: String,
    published_at_millis: Option<u64>,
}

fn entry_fields(entry: &Entry, now: u64) -> EntryFields {
    let link = entry.links.first().map(|item| item.href.clone());
    let raw_summary = entry
        .summary
        .as_ref()
        .map(|item| item.content.clone())
        .unwrap_or_default();
    let raw_content = entry
        .content
        .as_ref()
        .and_then(|item| item.body.clone())
        .unwrap_or_else(|| raw_summary.clone());
    let title = entry
        .title
        .as_ref()
        .map(|item| clean_text(&item.content))
        .filter(|item| !item.is_empty())
        .unwrap_or_else(|| "无标题".into());
    let published = entry.published.or(entry.updated);
    let external_id = if !entry.id.trim().is_empty() {
        entry.id.clone()
    } else if let Some(link) = link.as_deref() {
        link.to_string()
    } else {
        let mut hasher = Sha256::new();
        hasher.update(title.as_bytes());
        hasher.update(raw_content.as_bytes());
        format!("{:x}", hasher.finalize())
    };
    EntryFields {
        external_id,
        title,
        link,
        author: entry.authors.first().map(|author| author.name.clone()),
        summary: truncate(clean_text(&raw_summary), 16 * 1024),
        content: truncate(clean_content(&raw_content), MAX_ENTRY_TEXT_BYTES),
        published_at_millis: published
            .and_then(|value| u64::try_from(value.timestamp_millis()).ok())
            .or(Some(now)),
    }
}

fn clean_text(value: &str) -> String {
    strip_markup(value, false)
}

fn clean_content(value: &str) -> String {
    strip_markup(value, true)
}

fn strip_markup(value: &str, preserve_blocks: bool) -> String {
    let mut output = String::with_capacity(value.len());
    let mut in_tag = false;
    let mut tag = String::new();
    for ch in value.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' => {
                in_tag = false;
                let name = tag
                    .trim()
                    .trim_start_matches('/')
                    .split(|ch: char| ch.is_whitespace() || ch == '/')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if preserve_blocks
                    && matches!(
                        name.as_str(),
                        "p" | "div"
                            | "br"
                            | "li"
                            | "article"
                            | "section"
                            | "h1"
                            | "h2"
                            | "h3"
                            | "h4"
                            | "blockquote"
                            | "pre"
                    )
                {
                    output.push('\n');
                } else {
                    output.push(' ');
                }
            }
            _ if !in_tag => output.push(ch),
            _ => tag.push(ch),
        }
    }
    let decoded = output
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&#X27;", "'");
    if preserve_blocks {
        decoded
            .lines()
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
    } else {
        decoded.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

fn truncate(mut value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut boundary = max_bytes;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    value.truncate(boundary);
    value
}

pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn list_feeds(conn: &Connection) -> Result<Vec<RssFeed>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT f.id, COALESCE(f.custom_title, f.title), f.feed_url, f.site_url, f.description,
                    f.refresh_interval_minutes, f.enabled,
                    SUM(CASE WHEN e.is_read = 0 THEN 1 ELSE 0 END),
                    COUNT(e.id), f.last_refreshed_at_millis, f.last_error
             FROM rss_feeds f LEFT JOIN rss_entries e ON e.feed_id = f.id
             GROUP BY f.id ORDER BY lower(f.title), f.id",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok(RssFeed {
                id: row.get(0)?,
                title: row.get(1)?,
                feed_url: row.get(2)?,
                site_url: row.get(3)?,
                description: row.get(4)?,
                refresh_interval_minutes: row.get(5)?,
                enabled: row.get(6)?,
                unread_count: row.get(7)?,
                entry_count: row.get(8)?,
                last_refreshed_at_millis: row.get(9)?,
                last_error: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rss_feeds_list() -> Result<Vec<RssFeed>, String> {
    list_feeds(&open_repo()?)
}

#[tauri::command]
pub async fn rss_feed_add(feed_url: String) -> Result<RssRefreshResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let url = normalize_feed_url(&feed_url)?;
        let now = now_millis();
        let conn = open_repo()?;
        conn.execute(
            "INSERT INTO rss_feeds (
               title, feed_url, refresh_interval_minutes, enabled,
               created_at_millis, updated_at_millis
             ) VALUES (?1, ?2, ?3, 1, ?4, ?4)",
            params![url, url, DEFAULT_REFRESH_MINUTES, now],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "这个订阅已经存在".to_string()
            } else {
                e.to_string()
            }
        })?;
        let id = conn.last_insert_rowid();
        drop(conn);
        match refresh_feed(id) {
            Ok(result) => Ok(result),
            Err(error) => {
                if let Ok(conn) = open_repo() {
                    let _ = conn.execute("DELETE FROM rss_feeds WHERE id = ?1", [id]);
                }
                Err(error)
            }
        }
    })
    .await
    .map_err(|e| format!("RSS 添加任务失败: {e}"))?
}

#[tauri::command]
pub fn rss_feed_delete(id: i64) -> Result<(), String> {
    open_repo()?
        .execute("DELETE FROM rss_feeds WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn rss_feed_update(id: i64, update: RssFeedUpdate) -> Result<(), String> {
    let title = update.title.trim();
    if title.is_empty() {
        return Err("订阅名称不能为空".into());
    }
    open_repo()?
        .execute(
            "UPDATE rss_feeds SET custom_title = ?1, refresh_interval_minutes = ?2,
             enabled = ?3, updated_at_millis = ?4 WHERE id = ?5",
            params![
                title,
                update.refresh_interval_minutes.max(MIN_REFRESH_MINUTES),
                update.enabled,
                now_millis(),
                id
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn rss_feed_refresh(id: i64) -> Result<RssRefreshResult, String> {
    tauri::async_runtime::spawn_blocking(move || refresh_feed(id))
        .await
        .map_err(|e| format!("RSS 刷新任务失败: {e}"))?
}

#[tauri::command]
pub async fn rss_refresh_due() -> Result<Vec<RssRefreshResult>, String> {
    tauri::async_runtime::spawn_blocking(refresh_due)
        .await
        .map_err(|e| format!("RSS 定时刷新任务失败: {e}"))?
}

fn refresh_due() -> Result<Vec<RssRefreshResult>, String> {
    let conn = open_repo()?;
    let now = now_millis() as i64;
    let mut stmt = conn
        .prepare(
            "SELECT id FROM rss_feeds WHERE enabled = 1 AND
             (last_refreshed_at_millis IS NULL OR
              last_refreshed_at_millis + refresh_interval_minutes * 60000 <= ?1)
             ORDER BY COALESCE(last_refreshed_at_millis, 0) LIMIT 2",
        )
        .map_err(|e| e.to_string())?;
    let ids = stmt
        .query_map([now], |row| row.get::<_, i64>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);
    drop(conn);
    let mut results = Vec::new();
    for id in ids {
        if let Ok(result) = refresh_feed(id) {
            results.push(result);
        }
    }
    Ok(results)
}

pub fn start_scheduler() {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.tick().await;
        loop {
            interval.tick().await;
            let _ = tauri::async_runtime::spawn_blocking(refresh_due).await;
        }
    });
}

#[tauri::command]
pub fn rss_entries_list(
    feed_id: Option<i64>,
    filter: Option<String>,
    search: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<Vec<RssEntry>, String> {
    let conn = open_repo()?;
    let limit = limit.unwrap_or(100).clamp(1, 500);
    let offset = offset.unwrap_or(0);
    let filter = filter.unwrap_or_else(|| "all".into());
    let search = search.unwrap_or_default();
    let pattern = format!("%{}%", search.trim());
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.feed_id, f.title, e.title, e.link, e.author, e.summary,
                    e.content, e.published_at_millis, e.fetched_at_millis,
                    e.is_read, e.is_starred
             FROM rss_entries e JOIN rss_feeds f ON f.id = e.feed_id
             WHERE (?1 IS NULL OR e.feed_id = ?1)
               AND (?2 = 'all' OR (?2 = 'unread' AND e.is_read = 0)
                    OR (?2 = 'starred' AND e.is_starred = 1))
               AND (?3 = '' OR e.title LIKE ?4 OR e.summary LIKE ?4 OR f.title LIKE ?4)
             ORDER BY COALESCE(e.published_at_millis, e.fetched_at_millis) DESC, e.id DESC
             LIMIT ?5 OFFSET ?6",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            params![feed_id, filter, search.trim(), pattern, limit, offset],
            |row| {
                Ok(RssEntry {
                    id: row.get(0)?,
                    feed_id: row.get(1)?,
                    feed_title: row.get(2)?,
                    title: row.get(3)?,
                    link: row.get(4)?,
                    author: row.get(5)?,
                    summary: row.get(6)?,
                    content: row.get(7)?,
                    published_at_millis: row.get(8)?,
                    fetched_at_millis: row.get(9)?,
                    is_read: row.get(10)?,
                    is_starred: row.get(11)?,
                })
            },
        )
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn rss_entry_read(id: i64, read: bool) -> Result<(), String> {
    open_repo()?
        .execute(
            "UPDATE rss_entries SET is_read = ?1 WHERE id = ?2",
            params![read, id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn rss_entry_star(id: i64, starred: bool) -> Result<(), String> {
    open_repo()?
        .execute(
            "UPDATE rss_entries SET is_starred = ?1 WHERE id = ?2",
            params![starred, id],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn rss_mark_all_read(feed_id: Option<i64>) -> Result<u32, String> {
    let changed = open_repo()?
        .execute(
            "UPDATE rss_entries SET is_read = 1 WHERE is_read = 0
             AND (?1 IS NULL OR feed_id = ?1)",
            [feed_id],
        )
        .map_err(|e| e.to_string())?;
    Ok(changed as u32)
}

#[tauri::command]
pub fn rss_import_opml(content: String) -> Result<RssImportResult, String> {
    let outlines = opml_outlines(&content);
    if outlines.is_empty() {
        return Err("OPML 中没有找到 RSS 订阅".into());
    }
    let conn = open_repo()?;
    let now = now_millis();
    let mut imported = 0;
    let mut skipped = 0;
    for (title, url) in outlines {
        let Ok(url) = normalize_feed_url(&url) else {
            skipped += 1;
            continue;
        };
        let changed = conn
            .execute(
                "INSERT OR IGNORE INTO rss_feeds (
                   title, custom_title, feed_url, refresh_interval_minutes, enabled,
                   created_at_millis, updated_at_millis
                 ) VALUES (?1, ?1, ?2, ?3, 1, ?4, ?4)",
                params![
                    title
                        .filter(|v| !v.trim().is_empty())
                        .unwrap_or_else(|| url.clone()),
                    url,
                    DEFAULT_REFRESH_MINUTES,
                    now
                ],
            )
            .map_err(|e| e.to_string())?;
        if changed == 1 {
            imported += 1;
        } else {
            skipped += 1;
        }
    }
    Ok(RssImportResult { imported, skipped })
}

#[tauri::command]
pub fn rss_export_opml() -> Result<String, String> {
    let feeds = list_feeds(&open_repo()?)?;
    let mut output = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<opml version=\"2.0\">\n<head><title>Zhiyu RSS Subscriptions</title></head>\n<body>\n",
    );
    for feed in feeds {
        output.push_str(&format!(
            "  <outline type=\"rss\" text=\"{}\" title=\"{}\" xmlUrl=\"{}\"{} />\n",
            xml_escape(&feed.title),
            xml_escape(&feed.title),
            xml_escape(&feed.feed_url),
            feed.site_url
                .as_deref()
                .map(|url| format!(" htmlUrl=\"{}\"", xml_escape(url)))
                .unwrap_or_default()
        ));
    }
    output.push_str("</body>\n</opml>\n");
    Ok(output)
}

fn opml_outlines(content: &str) -> Vec<(Option<String>, String)> {
    let mut result = Vec::new();
    for fragment in content.split("<outline").skip(1) {
        let tag = fragment.split('>').next().unwrap_or_default();
        let Some(url) = xml_attribute(tag, "xmlUrl").or_else(|| xml_attribute(tag, "xmlurl"))
        else {
            continue;
        };
        let title = xml_attribute(tag, "title").or_else(|| xml_attribute(tag, "text"));
        result.push((title.map(|value| xml_unescape(&value)), xml_unescape(&url)));
    }
    result
}

fn xml_attribute(tag: &str, name: &str) -> Option<String> {
    let marker = format!("{name}=");
    let start = tag.find(&marker)? + marker.len();
    let quote = tag[start..].chars().next()?;
    if !matches!(quote, '"' | '\'') {
        return None;
    }
    let value_start = start + quote.len_utf8();
    let end = tag[value_start..].find(quote)? + value_start;
    Some(tag[value_start..end].to_string())
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static TEST_ID: AtomicU32 = AtomicU32::new(0);

    fn test_repo() -> Connection {
        let id = TEST_ID.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("zhiyu-rss-test-{}-{id}.db", std::process::id()));
        open_repo_at(&path).unwrap()
    }

    #[test]
    fn opml_round_trip_parser_reads_nested_outlines() {
        let opml = r#"<?xml version="1.0"?><opml><body><outline text="Tech">
          <outline type="rss" text="Example" xmlUrl="https://example.com/feed.xml"/>
        </outline></body></opml>"#;
        let values = opml_outlines(opml);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].0.as_deref(), Some("Example"));
        assert_eq!(values[0].1, "https://example.com/feed.xml");
    }

    #[test]
    fn payload_diagnostics_distinguish_empty_html_and_feed_content() {
        let empty = inspect_payload(
            b"",
            "application/rss+xml",
            "https://www.zhihu.com/rss",
            "https://www.zhihu.com/rss",
        )
        .unwrap_err();
        assert!(empty.contains("空内容"));
        assert!(empty.contains("知乎日报"));

        let html = inspect_payload(
            b"<!doctype html><html><title>Security check</title></html>",
            "text/html",
            "https://example.com/feed",
            "https://example.com/login",
        )
        .unwrap_err();
        assert!(html.contains("网页而不是"));
        assert!(html.contains("login"));

        assert!(inspect_payload(
            br#"<?xml version="1.0"?><rss version="2.0"></rss>"#,
            "application/xml",
            "https://example.com/feed",
            "https://example.com/feed",
        )
        .is_ok());
    }

    #[test]
    fn database_cascades_entries_when_feed_is_deleted() {
        let conn = test_repo();
        let now = now_millis();
        conn.execute(
            "INSERT INTO rss_feeds (title, feed_url, created_at_millis, updated_at_millis)
             VALUES ('Example', 'https://example.com/feed', ?1, ?1)",
            [now],
        )
        .unwrap();
        let feed_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO rss_entries
             (feed_id, external_id, title, fetched_at_millis) VALUES (?1, '1', 'Hello', ?2)",
            params![feed_id, now],
        )
        .unwrap();
        let entry_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO rss_ai_results (
               entry_id, action, output_language, model, source_hash, content,
               status, created_at_millis, updated_at_millis
             ) VALUES (?1, 'summary', 'zh-CN', 'test', 'hash', 'result',
                       'complete', ?2, ?2)",
            params![entry_id, now],
        )
        .unwrap();
        conn.execute("DELETE FROM rss_feeds WHERE id = ?1", [feed_id])
            .unwrap();
        let entry_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM rss_entries", [], |row| row.get(0))
            .unwrap();
        let result_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM rss_ai_results", [], |row| row.get(0))
            .unwrap();
        assert_eq!(entry_count, 0);
        assert_eq!(result_count, 0);
    }

    #[test]
    fn strips_markup_from_preview_text() {
        assert_eq!(
            clean_text("<p>Hello &amp; <strong>world</strong></p>"),
            "Hello & world"
        );
    }

    #[test]
    fn preserves_paragraphs_and_decodes_apostrophes_in_article_content() {
        assert_eq!(
            clean_content("<p>We&#x27;re shipping.</p><p>Second paragraph.</p>"),
            "We're shipping.\n\nSecond paragraph."
        );
    }

    #[test]
    fn upgrades_legacy_database_without_adding_subscriptions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy.db");
        let legacy = Connection::open(&path).unwrap();
        legacy
            .execute_batch(
                "CREATE TABLE rss_feeds (
                   id INTEGER PRIMARY KEY AUTOINCREMENT,
                   title TEXT NOT NULL,
                   feed_url TEXT NOT NULL UNIQUE,
                   site_url TEXT,
                   description TEXT,
                   refresh_interval_minutes INTEGER NOT NULL DEFAULT 30,
                   enabled INTEGER NOT NULL DEFAULT 1,
                   etag TEXT,
                   last_modified TEXT,
                   last_refreshed_at_millis INTEGER,
                   last_error TEXT,
                   created_at_millis INTEGER NOT NULL,
                   updated_at_millis INTEGER NOT NULL
                 );",
            )
            .unwrap();
        drop(legacy);

        let upgraded = open_repo_at(&path).unwrap();
        let custom_title_exists = upgraded
            .prepare("PRAGMA table_info(rss_feeds)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .iter()
            .any(|column| column == "custom_title");
        assert!(custom_title_exists);
        assert!(list_feeds(&upgraded).unwrap().is_empty());
    }

    #[test]
    fn retires_only_untouched_legacy_default_feeds() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("legacy-defaults.db");
        let connection = open_repo_at(&path).unwrap();
        let now = now_millis();
        connection
            .execute(
                "INSERT INTO rss_feeds (
                   title, custom_title, feed_url, created_at_millis, updated_at_millis
                 ) VALUES ('GitHub Blog', 'GitHub Blog', 'https://github.blog/feed/', ?1, ?1)",
                [now],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO rss_meta (key, value) VALUES ('default_feeds_seeded', '1')",
                [],
            )
            .unwrap();
        drop(connection);

        let migrated = open_repo_at(&path).unwrap();
        assert!(list_feeds(&migrated).unwrap().is_empty());
    }
}
