use rusqlite::{types::ValueRef, Connection, OpenFlags};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const MAX_SQL_LENGTH: usize = 1024 * 1024;
const MAX_RESULT_ROWS: usize = 500;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteOverview {
    version: String,
    file_size_bytes: u64,
    table_count: u64,
    index_count: u64,
    journal_mode: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteTable {
    name: String,
    table_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqliteQueryResult {
    columns: Vec<String>,
    rows: Vec<Vec<Option<String>>>,
    summary: String,
    elapsed_ms: u128,
    truncated: bool,
}

#[tauri::command]
pub async fn sqlite_create(file_path: String) -> Result<SqliteOverview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = validate_path(&file_path, false)?;
        if path.exists() {
            return Err("目标文件已经存在，请选择另一个文件名".into());
        }
        let parent = path
            .parent()
            .ok_or_else(|| "数据库文件路径无效".to_string())?;
        if !parent.is_dir() {
            return Err("数据库文件所在目录不存在".into());
        }
        Connection::open(&path).map_err(sqlite_error)?;
        read_overview(&path)
    })
    .await
    .map_err(|error| format!("SQLite 建库任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn sqlite_overview(file_path: String) -> Result<SqliteOverview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = validate_path(&file_path, true)?;
        read_overview(&path)
    })
    .await
    .map_err(|error| format!("SQLite 概览任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn sqlite_tables(file_path: String) -> Result<Vec<SqliteTable>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = validate_path(&file_path, true)?;
        let connection = open_readonly(&path)?;
        let mut statement = connection
            .prepare(
                "SELECT name, type FROM sqlite_schema \
                 WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' \
                 ORDER BY type, name",
            )
            .map_err(sqlite_error)?;
        let tables = statement
            .query_map([], |row| {
                Ok(SqliteTable {
                    name: row.get(0)?,
                    table_type: row.get(1)?,
                })
            })
            .map_err(sqlite_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sqlite_error)?;
        Ok(tables)
    })
    .await
    .map_err(|error| format!("SQLite 表结构任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn sqlite_execute(
    file_path: String,
    sql: String,
    confirmed: bool,
) -> Result<SqliteQueryResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = validate_path(&file_path, true)?;
        execute_sql(&path, &sql, confirmed)
    })
    .await
    .map_err(|error| format!("SQLite 查询任务异常结束: {error}"))?
}

fn read_overview(path: &Path) -> Result<SqliteOverview, String> {
    let connection = open_readonly(path)?;
    let table_count = connection
        .query_row(
            "SELECT count(*) FROM sqlite_schema \
             WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )
        .map_err(sqlite_error)?;
    let index_count = connection
        .query_row(
            "SELECT count(*) FROM sqlite_schema \
             WHERE type = 'index' AND name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )
        .map_err(sqlite_error)?;
    let journal_mode = connection
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .map_err(sqlite_error)?;
    let file_size_bytes = fs::metadata(path)
        .map_err(|error| format!("无法读取数据库文件信息: {error}"))?
        .len();

    Ok(SqliteOverview {
        version: rusqlite::version().into(),
        file_size_bytes,
        table_count,
        index_count,
        journal_mode,
    })
}

fn execute_sql(path: &Path, sql: &str, confirmed: bool) -> Result<SqliteQueryResult, String> {
    let sql = sql.trim();
    if sql.is_empty() {
        return Err("请输入 SQL".into());
    }
    if sql.len() > MAX_SQL_LENGTH || sql.contains('\0') {
        return Err("SQL 不能超过 1 MiB，且不能包含 NUL 字符".into());
    }

    let normalized = sql.to_ascii_uppercase();
    if normalized.contains("ATTACH DATABASE") {
        return Err("智屿 SQLite 命令台暂不允许 ATTACH DATABASE".into());
    }
    if requires_confirmation(&normalized) && !confirmed {
        return Err("CONFIRM_REQUIRED:该 SQL 会删除或清空数据".into());
    }

    let started = Instant::now();
    let connection = Connection::open(path).map_err(sqlite_error)?;
    let mut statement = connection.prepare(sql).map_err(sqlite_error)?;
    let column_count = statement.column_count();

    if column_count == 0 {
        drop(statement);
        let affected = connection.execute(sql, []).map_err(sqlite_error)?;
        return Ok(SqliteQueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            summary: format!("执行完成，影响 {affected} 行"),
            elapsed_ms: started.elapsed().as_millis(),
            truncated: false,
        });
    }

    let columns = statement
        .column_names()
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut query = statement.query([]).map_err(sqlite_error)?;
    let mut rows = Vec::new();
    let mut truncated = false;
    while let Some(row) = query.next().map_err(sqlite_error)? {
        if rows.len() == MAX_RESULT_ROWS {
            truncated = true;
            break;
        }
        let mut values = Vec::with_capacity(column_count);
        for index in 0..column_count {
            values.push(value_to_string(row.get_ref(index).map_err(sqlite_error)?));
        }
        rows.push(values);
    }

    Ok(SqliteQueryResult {
        columns,
        summary: format!("返回 {} 行", rows.len()),
        rows,
        elapsed_ms: started.elapsed().as_millis(),
        truncated,
    })
}

fn open_readonly(path: &Path) -> Result<Connection, String> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(sqlite_error)
}

fn validate_path(file_path: &str, must_exist: bool) -> Result<PathBuf, String> {
    if file_path.is_empty() || file_path.contains('\0') {
        return Err("SQLite 文件路径无效".into());
    }
    let path = PathBuf::from(file_path);
    if !path.is_absolute() {
        return Err("SQLite 文件必须使用绝对路径".into());
    }
    if must_exist && !path.is_file() {
        return Err(format!("SQLite 文件不存在: {}", path.display()));
    }
    Ok(path)
}

fn requires_confirmation(normalized: &str) -> bool {
    ["DROP TABLE", "DROP VIEW", "DELETE FROM", "VACUUM"]
        .iter()
        .any(|keyword| normalized.contains(keyword))
}

fn value_to_string(value: ValueRef<'_>) -> Option<String> {
    match value {
        ValueRef::Null => None,
        ValueRef::Integer(value) => Some(value.to_string()),
        ValueRef::Real(value) => Some(value.to_string()),
        ValueRef::Text(value) => Some(String::from_utf8_lossy(value).into_owned()),
        ValueRef::Blob(value) => Some(format!("<BLOB {} bytes>", value.len())),
    }
}

fn sqlite_error(error: rusqlite::Error) -> String {
    format!("SQLite: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_queries_and_describes_database() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("test.sqlite");
        Connection::open(&path)
            .unwrap()
            .execute(
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
                [],
            )
            .unwrap();
        Connection::open(&path)
            .unwrap()
            .execute("INSERT INTO users (name) VALUES ('智屿')", [])
            .unwrap();

        let overview = read_overview(&path).unwrap();
        assert_eq!(overview.table_count, 1);
        let result = execute_sql(&path, "SELECT name FROM users", false).unwrap();
        assert_eq!(result.columns, vec!["name"]);
        assert_eq!(result.rows[0][0].as_deref(), Some("智屿"));
    }

    #[test]
    fn destructive_queries_require_confirmation() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("test.sqlite");
        Connection::open(&path)
            .unwrap()
            .execute("CREATE TABLE users (id INTEGER)", [])
            .unwrap();

        let error = execute_sql(&path, "DELETE FROM users", false).unwrap_err();
        assert!(error.starts_with("CONFIRM_REQUIRED:"));
    }

    #[test]
    fn attach_database_is_blocked() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("test.sqlite");
        Connection::open(&path).unwrap();

        let error =
            execute_sql(&path, "ATTACH DATABASE '/tmp/other.db' AS other", true).unwrap_err();
        assert!(error.contains("ATTACH DATABASE"));
    }
}
