use crate::ai_runtime::{stream_completion, ModelMessage};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

const STREAM_EVENT: &str = "ai-chat-stream";
const MAX_MESSAGE_BYTES: usize = 32 * 1024;
const MAX_CONTEXT_MESSAGES: usize = 24;
const MAX_CONTEXT_BYTES: usize = 96 * 1024;
static SESSION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
pub struct AiChatState(Mutex<HashMap<String, Arc<AtomicBool>>>);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChatSession {
    id: String,
    title: String,
    preview: String,
    message_count: u32,
    created_at_millis: u64,
    updated_at_millis: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChatMessage {
    id: i64,
    session_id: String,
    role: String,
    content: String,
    created_at_millis: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiChatSendInput {
    session_id: String,
    request_id: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AiChatStreamEvent {
    session_id: String,
    request_id: String,
    event: String,
    content: String,
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn db_path() -> Result<PathBuf, String> {
    let directory = dirs::data_dir()
        .ok_or_else(|| "无法确定应用数据目录".to_string())?
        .join("dev.zhiyu.env")
        .join("ai");
    Ok(directory.join("chat.db"))
}

fn open_repo() -> Result<Connection, String> {
    open_repo_at(&db_path()?)
}

fn open_repo_at(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("无法创建 AI 会话目录: {error}"))?;
    }
    let connection =
        Connection::open(path).map_err(|error| format!("无法打开 AI 会话数据库: {error}"))?;
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             CREATE TABLE IF NOT EXISTS ai_chat_sessions (
               id TEXT PRIMARY KEY,
               title TEXT NOT NULL,
               created_at_millis INTEGER NOT NULL,
               updated_at_millis INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS ai_chat_messages (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               session_id TEXT NOT NULL REFERENCES ai_chat_sessions(id) ON DELETE CASCADE,
               role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
               content TEXT NOT NULL,
               created_at_millis INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_ai_chat_messages_session
               ON ai_chat_messages(session_id, id);",
        )
        .map_err(|error| format!("AI 会话数据库初始化失败: {error}"))?;
    Ok(connection)
}

fn new_session_id() -> String {
    format!(
        "chat-{}-{}",
        now_millis(),
        SESSION_COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

fn clean_title(content: &str) -> String {
    let compact = content.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut title = compact.chars().take(24).collect::<String>();
    if compact.chars().count() > 24 {
        title.push('…');
    }
    if title.is_empty() {
        "新对话".into()
    } else {
        title
    }
}

fn create_session() -> Result<AiChatSession, String> {
    let connection = open_repo()?;
    let id = new_session_id();
    let now = now_millis();
    connection
        .execute(
            "INSERT INTO ai_chat_sessions (id, title, created_at_millis, updated_at_millis)
             VALUES (?1, '新对话', ?2, ?2)",
            params![id, now],
        )
        .map_err(|error| format!("无法创建 AI 会话: {error}"))?;
    Ok(AiChatSession {
        id,
        title: "新对话".into(),
        preview: String::new(),
        message_count: 0,
        created_at_millis: now,
        updated_at_millis: now,
    })
}

fn list_sessions() -> Result<Vec<AiChatSession>, String> {
    let connection = open_repo()?;
    let mut statement = connection
        .prepare(
            "SELECT s.id, s.title,
                    COALESCE((SELECT content FROM ai_chat_messages m
                              WHERE m.session_id = s.id ORDER BY m.id DESC LIMIT 1), ''),
                    (SELECT COUNT(*) FROM ai_chat_messages m WHERE m.session_id = s.id),
                    s.created_at_millis, s.updated_at_millis
             FROM ai_chat_sessions s
             ORDER BY s.updated_at_millis DESC",
        )
        .map_err(|error| format!("无法读取 AI 会话: {error}"))?;
    let sessions = statement
        .query_map([], |row| {
            Ok(AiChatSession {
                id: row.get(0)?,
                title: row.get(1)?,
                preview: row.get(2)?,
                message_count: row.get(3)?,
                created_at_millis: row.get(4)?,
                updated_at_millis: row.get(5)?,
            })
        })
        .map_err(|error| format!("无法查询 AI 会话: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("无法读取 AI 会话记录: {error}"))?;
    Ok(sessions)
}

fn list_messages(session_id: &str) -> Result<Vec<AiChatMessage>, String> {
    let connection = open_repo()?;
    let mut statement = connection
        .prepare(
            "SELECT id, session_id, role, content, created_at_millis
             FROM ai_chat_messages WHERE session_id = ?1 ORDER BY id",
        )
        .map_err(|error| format!("无法读取 AI 消息: {error}"))?;
    let messages = statement
        .query_map(params![session_id], |row| {
            Ok(AiChatMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at_millis: row.get(4)?,
            })
        })
        .map_err(|error| format!("无法查询 AI 消息: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("无法读取 AI 消息记录: {error}"))?;
    Ok(messages)
}

fn insert_message(session_id: &str, role: &str, content: &str) -> Result<i64, String> {
    let connection = open_repo()?;
    let now = now_millis();
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM ai_chat_sessions WHERE id = ?1)",
            params![session_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("无法检查 AI 会话: {error}"))?;
    if !exists {
        return Err("AI 会话不存在".into());
    }
    connection
        .execute(
            "INSERT INTO ai_chat_messages (session_id, role, content, created_at_millis)
             VALUES (?1, ?2, ?3, ?4)",
            params![session_id, role, content, now],
        )
        .map_err(|error| format!("无法保存 AI 消息: {error}"))?;
    if role == "user" {
        connection
            .execute(
                "UPDATE ai_chat_sessions
                 SET title = CASE WHEN title = '新对话' THEN ?2 ELSE title END,
                     updated_at_millis = ?3
                 WHERE id = ?1",
                params![session_id, clean_title(content), now],
            )
            .map_err(|error| format!("无法更新 AI 会话: {error}"))?;
    } else {
        connection
            .execute(
                "UPDATE ai_chat_sessions SET updated_at_millis = ?2 WHERE id = ?1",
                params![session_id, now],
            )
            .map_err(|error| format!("无法更新 AI 会话时间: {error}"))?;
    }
    Ok(connection.last_insert_rowid())
}

fn context_messages(session_id: &str) -> Result<Vec<ModelMessage>, String> {
    let connection = open_repo()?;
    let mut statement = connection
        .prepare(
            "SELECT role, content FROM ai_chat_messages
             WHERE session_id = ?1 ORDER BY id DESC LIMIT ?2",
        )
        .map_err(|error| format!("无法准备 AI 上下文: {error}"))?;
    let mut messages = statement
        .query_map(params![session_id, MAX_CONTEXT_MESSAGES as i64], |row| {
            Ok(ModelMessage {
                role: row.get(0)?,
                content: row.get(1)?,
            })
        })
        .map_err(|error| format!("无法查询 AI 上下文: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("无法读取 AI 上下文: {error}"))?;
    messages.reverse();
    let mut bytes = messages
        .iter()
        .map(|item| item.content.len())
        .sum::<usize>();
    while bytes > MAX_CONTEXT_BYTES && messages.len() > 2 {
        bytes = bytes.saturating_sub(messages[0].content.len());
        messages.remove(0);
    }
    Ok(messages)
}

fn stream_request(
    app: &AppHandle,
    session_id: &str,
    request_id: &str,
    cancel: &AtomicBool,
) -> Result<(String, bool), String> {
    let messages = context_messages(session_id)?;
    let completion = stream_completion(&messages, cancel, |delta| {
        let _ = app.emit(
            STREAM_EVENT,
            AiChatStreamEvent {
                session_id: session_id.into(),
                request_id: request_id.into(),
                event: "delta".into(),
                content: delta.into(),
            },
        );
    })?;
    Ok((completion.answer, completion.cancelled))
}

#[tauri::command]
pub async fn ai_chat_sessions_list() -> Result<Vec<AiChatSession>, String> {
    tauri::async_runtime::spawn_blocking(list_sessions)
        .await
        .map_err(|error| format!("AI 会话读取任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn ai_chat_session_create() -> Result<AiChatSession, String> {
    tauri::async_runtime::spawn_blocking(create_session)
        .await
        .map_err(|error| format!("AI 会话创建任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn ai_chat_session_delete(session_id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        open_repo()?
            .execute(
                "DELETE FROM ai_chat_sessions WHERE id = ?1",
                params![session_id],
            )
            .map_err(|error| format!("无法删除 AI 会话: {error}"))?;
        Ok(())
    })
    .await
    .map_err(|error| format!("AI 会话删除任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn ai_chat_messages_list(session_id: String) -> Result<Vec<AiChatMessage>, String> {
    tauri::async_runtime::spawn_blocking(move || list_messages(&session_id))
        .await
        .map_err(|error| format!("AI 消息读取任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn ai_chat_send(
    app: AppHandle,
    state: State<'_, AiChatState>,
    input: AiChatSendInput,
) -> Result<(), String> {
    let content = input.content.trim().to_string();
    if content.is_empty() {
        return Err("请输入消息".into());
    }
    if content.len() > MAX_MESSAGE_BYTES {
        return Err("单条消息不能超过 32 KiB".into());
    }
    if input.request_id.len() > 100 || input.session_id.len() > 100 {
        return Err("AI 会话标识无效".into());
    }
    insert_message(&input.session_id, "user", &content)?;
    let cancellation = Arc::new(AtomicBool::new(false));
    {
        let mut requests = state
            .0
            .lock()
            .map_err(|_| "AI 请求状态锁已损坏".to_string())?;
        requests.insert(input.request_id.clone(), cancellation.clone());
    }
    let request_id = input.request_id.clone();
    let session_id = input.session_id.clone();
    let task_app = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        stream_request(&task_app, &session_id, &request_id, &cancellation)
    })
    .await
    .map_err(|error| format!("AI 流式任务异常结束: {error}"))?;
    if let Ok(mut requests) = state.0.lock() {
        requests.remove(&input.request_id);
    }
    match result {
        Ok((answer, cancelled)) => {
            if !answer.is_empty() {
                insert_message(&input.session_id, "assistant", &answer)?;
            }
            let _ = app.emit(
                STREAM_EVENT,
                AiChatStreamEvent {
                    session_id: input.session_id,
                    request_id: input.request_id,
                    event: if cancelled { "cancelled" } else { "done" }.into(),
                    content: String::new(),
                },
            );
            Ok(())
        }
        Err(error) => {
            let _ = app.emit(
                STREAM_EVENT,
                AiChatStreamEvent {
                    session_id: input.session_id,
                    request_id: input.request_id,
                    event: "error".into(),
                    content: error.clone(),
                },
            );
            Err(error)
        }
    }
}

#[tauri::command]
pub fn ai_chat_cancel(state: State<'_, AiChatState>, request_id: String) -> Result<(), String> {
    let requests = state
        .0
        .lock()
        .map_err(|_| "AI 请求状态锁已损坏".to_string())?;
    let request = requests
        .get(&request_id)
        .ok_or_else(|| "AI 请求已经结束".to_string())?;
    request.store(true, Ordering::Relaxed);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn chat_repository_keeps_sessions_and_messages() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("chat.db");
        let connection = open_repo_at(&path).unwrap();
        let now = now_millis();
        connection
            .execute(
                "INSERT INTO ai_chat_sessions VALUES ('one', '新对话', ?1, ?1)",
                params![now],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO ai_chat_messages
                 (session_id, role, content, created_at_millis)
                 VALUES ('one', 'user', 'hello', ?1)",
                params![now],
            )
            .unwrap();
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM ai_chat_messages WHERE session_id = 'one'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn titles_are_short_and_readable() {
        assert_eq!(clean_title("  hello   world  "), "hello world");
        assert!(clean_title(&"a".repeat(40)).ends_with('…'));
    }
}
