use crate::commands::run_install_task;
use devbox_core::{
    installer::{DUCKDB_SERIES, DUCKDB_VERSION},
    DuckdbInstaller,
};
use serde::Serialize;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;

const QUERY_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_SQL_BYTES: usize = 50 * 1024;
const MAX_RESULT_BYTES: u64 = 16 * 1024 * 1024;
const RESULT_ROW_LIMIT: usize = 500;
const NULL_MARKER: &str = "__ZHIYU_DUCKDB_NULL_6f4e9d__";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuckdbStatus {
    installed: bool,
    version: &'static str,
    executable_path: PathBuf,
    installation_bytes: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuckdbQueryResult {
    columns: Vec<String>,
    rows: Vec<Vec<Option<String>>>,
    elapsed_ms: u128,
    truncated: bool,
    summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalFileKind {
    Csv,
    Json,
    Parquet,
    Database,
}

#[tauri::command]
pub async fn duckdb_status() -> Result<DuckdbStatus, String> {
    tauri::async_runtime::spawn_blocking(read_status)
        .await
        .map_err(|error| format!("DuckDB 状态任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn duckdb_install(app: AppHandle, operation_id: String) -> Result<DuckdbStatus, String> {
    tauri::async_runtime::spawn_blocking(move || {
        run_install_task(app, operation_id, "duckdb".into(), || {
            let root = devbox_root()?;
            DuckdbInstaller::new(&root)
                .install()
                .map_err(|error| error.to_string())?;
            read_status()
        })
    })
    .await
    .map_err(|error| format!("DuckDB 安装任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn duckdb_query(path: String, sql: String) -> Result<DuckdbQueryResult, String> {
    tauri::async_runtime::spawn_blocking(move || run_query(&path, &sql))
        .await
        .map_err(|error| format!("DuckDB 查询任务异常结束: {error}"))?
}

fn read_status() -> Result<DuckdbStatus, String> {
    let root = devbox_root()?;
    let installation = root
        .join("installations")
        .join("duckdb")
        .join(DUCKDB_SERIES);
    let executable = installation.join("bin/duckdb");
    let installed = executable.is_file()
        && Command::new(&executable)
            .arg("--version")
            .output()
            .map(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout).contains(DUCKDB_VERSION)
            })
            .unwrap_or(false);

    Ok(DuckdbStatus {
        installed,
        version: DUCKDB_VERSION,
        executable_path: executable,
        installation_bytes: directory_size(&installation)?,
    })
}

fn run_query(path: &str, sql: &str) -> Result<DuckdbQueryResult, String> {
    validate_query(sql)?;
    let selected_path = validate_file(path)?;
    let file_kind = file_kind(&selected_path)?;
    let root = devbox_root()?;
    let executable = root
        .join("installations")
        .join("duckdb")
        .join(DUCKDB_SERIES)
        .join("bin/duckdb");
    if !executable.is_file() {
        return Err("请先安装 DuckDB CLI".into());
    }

    let query = build_query(&selected_path, file_kind, sql);
    let temp_dir = root.join("tmp");
    fs::create_dir_all(&temp_dir).map_err(|error| error.to_string())?;
    let suffix = unique_suffix();
    let stdout_path = temp_dir.join(format!("duckdb-query-{suffix}.csv"));
    let stderr_path = temp_dir.join(format!("duckdb-query-{suffix}.log"));
    let started = Instant::now();
    let result = execute_cli(
        &executable,
        &selected_path,
        file_kind,
        &query,
        &stdout_path,
        &stderr_path,
    );
    let elapsed_ms = started.elapsed().as_millis();
    let _ = fs::remove_file(&stdout_path);
    let _ = fs::remove_file(&stderr_path);
    let output = result?;
    parse_csv_result(&output, elapsed_ms)
}

fn execute_cli(
    executable: &Path,
    selected_path: &Path,
    file_kind: LocalFileKind,
    query: &str,
    stdout_path: &Path,
    stderr_path: &Path,
) -> Result<Vec<u8>, String> {
    let stdout = File::create(stdout_path).map_err(|error| error.to_string())?;
    let stderr = File::create(stderr_path).map_err(|error| error.to_string())?;
    let mut command = Command::new(executable);
    command
        .args([
            "-batch",
            "-bail",
            "-init",
            "/dev/null",
            "-csv",
            "-header",
            "-nullvalue",
            NULL_MARKER,
        ])
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));

    if file_kind == LocalFileKind::Database {
        command.arg("-safe").arg("-readonly").arg(selected_path);
    } else {
        command.arg(":memory:");
    }
    command.arg("-c").arg(query);

    let mut child = command.spawn().map_err(|error| error.to_string())?;
    let deadline = Instant::now() + QUERY_TIMEOUT;
    let status = loop {
        match child.try_wait().map_err(|error| error.to_string())? {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("查询超过 15 秒，已自动停止。请缩小查询范围或添加过滤条件".into());
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    };

    if !status.success() {
        return Err(read_limited_text(stderr_path, 32 * 1024)?
            .trim()
            .to_string());
    }
    let size = fs::metadata(stdout_path)
        .map_err(|error| error.to_string())?
        .len();
    if size > MAX_RESULT_BYTES {
        return Err("查询结果超过 16 MB，请增加 LIMIT 或只选择必要字段".into());
    }
    fs::read(stdout_path).map_err(|error| error.to_string())
}

fn parse_csv_result(bytes: &[u8], elapsed_ms: u128) -> Result<DuckdbQueryResult, String> {
    if bytes.is_empty() {
        return Ok(DuckdbQueryResult {
            columns: Vec::new(),
            rows: Vec::new(),
            elapsed_ms,
            truncated: false,
            summary: "查询完成，没有返回数据".into(),
        });
    }

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(bytes);
    let columns = reader
        .headers()
        .map_err(|error| format!("无法解析 DuckDB 列名: {error}"))?
        .iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|error| format!("无法解析 DuckDB 结果: {error}"))?;
        rows.push(
            record
                .iter()
                .map(|value| {
                    if value == NULL_MARKER {
                        None
                    } else {
                        Some(value.to_string())
                    }
                })
                .collect(),
        );
    }
    let truncated = rows.len() > RESULT_ROW_LIMIT;
    if truncated {
        rows.truncate(RESULT_ROW_LIMIT);
    }
    let summary = if truncated {
        format!("已显示前 {RESULT_ROW_LIMIT} 行")
    } else {
        format!("返回 {} 行", rows.len())
    };
    Ok(DuckdbQueryResult {
        columns,
        rows,
        elapsed_ms,
        truncated,
        summary,
    })
}

fn validate_file(path: &str) -> Result<PathBuf, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("请先选择本地数据文件".into());
    }
    let canonical = fs::canonicalize(path).map_err(|error| format!("无法读取所选文件: {error}"))?;
    if !canonical.is_file() {
        return Err("所选路径不是文件".into());
    }
    Ok(canonical)
}

fn file_kind(path: &Path) -> Result<LocalFileKind, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "csv" | "tsv" => Ok(LocalFileKind::Csv),
        "json" | "jsonl" | "ndjson" => Ok(LocalFileKind::Json),
        "parquet" => Ok(LocalFileKind::Parquet),
        "duckdb" | "db" => Ok(LocalFileKind::Database),
        _ => Err("暂时只支持 CSV、TSV、JSON、JSONL、Parquet 和 .duckdb/.db 文件".into()),
    }
}

fn build_query(path: &Path, kind: LocalFileKind, sql: &str) -> String {
    let clean_sql = sql.trim().trim_end_matches(';').trim();
    let first = first_keyword(clean_sql).unwrap_or_default();
    let limited_sql = if matches!(first.as_str(), "SELECT" | "WITH" | "FROM") {
        format!(
            "SELECT * FROM ({clean_sql}) AS zhiyu_query_result LIMIT {};",
            RESULT_ROW_LIMIT + 1
        )
    } else {
        format!("{clean_sql};")
    };
    let resource_limits = concat!(
        "SET threads = 2;",
        "SET memory_limit = '512MB';",
        "SET max_temp_directory_size = '256MB';"
    );

    if kind == LocalFileKind::Database {
        // The CLI's safe mode locks configuration before SQL runs, so resource
        // settings cannot be changed for an existing database connection.
        return limited_sql;
    }

    let escaped_path = sql_string(&path.to_string_lossy());
    let source = match kind {
        LocalFileKind::Csv => format!("read_csv_auto('{escaped_path}')"),
        LocalFileKind::Json => format!("read_json_auto('{escaped_path}')"),
        LocalFileKind::Parquet => format!("read_parquet('{escaped_path}')"),
        LocalFileKind::Database => unreachable!(),
    };
    format!(
        "{resource_limits}\
         SET allowed_paths = ['{escaped_path}'];\
         SET enable_external_access = false;\
         SET allow_community_extensions = false;\
         SET lock_configuration = true;\
         CREATE TEMP VIEW selected_file AS SELECT * FROM {source};\
         {limited_sql}"
    )
}

fn validate_query(sql: &str) -> Result<(), String> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err("请输入查询语句".into());
    }
    if trimmed.len() > MAX_SQL_BYTES {
        return Err("SQL 过长，最多支持 50 KB".into());
    }
    if trimmed.starts_with('.') {
        return Err("不支持 DuckDB 点命令，请使用 SQL 查询".into());
    }

    let normalized = normalized_sql(trimmed)?;
    let first = normalized
        .split_whitespace()
        .next()
        .ok_or_else(|| "请输入查询语句".to_string())?;
    if !matches!(
        first,
        "SELECT" | "WITH" | "FROM" | "SHOW" | "DESCRIBE" | "DESC" | "EXPLAIN" | "SUMMARIZE"
    ) {
        return Err("本地文件查询器只允许 SELECT、WITH、FROM、SHOW、DESCRIBE 和 EXPLAIN".into());
    }

    const FORBIDDEN: &[&str] = &[
        "ALTER",
        "ATTACH",
        "CALL",
        "CHECKPOINT",
        "COPY",
        "CREATE",
        "DELETE",
        "DETACH",
        "DROP",
        "EXECUTE",
        "EXPORT",
        "FORCE",
        "IMPORT",
        "INSERT",
        "INSTALL",
        "LOAD",
        "PREPARE",
        "PRAGMA",
        "RESET",
        "SET",
        "TRUNCATE",
        "UPDATE",
        "USE",
        "VACUUM",
    ];
    if let Some(keyword) = normalized
        .split_whitespace()
        .find(|token| FORBIDDEN.contains(token))
    {
        return Err(format!("只读查询中不允许使用 {keyword}"));
    }
    Ok(())
}

fn normalized_sql(sql: &str) -> Result<String, String> {
    let mut output = String::with_capacity(sql.len());
    let mut chars = sql.chars().peekable();
    let mut statement_ended = false;
    while let Some(character) = chars.next() {
        match character {
            '\'' => {
                output.push(' ');
                loop {
                    match chars.next() {
                        Some('\'') if chars.peek() == Some(&'\'') => {
                            chars.next();
                        }
                        Some('\'') => break,
                        Some(_) => {}
                        None => return Err("SQL 字符串没有闭合".into()),
                    }
                }
            }
            '"' => {
                output.push(' ');
                loop {
                    match chars.next() {
                        Some('"') if chars.peek() == Some(&'"') => {
                            chars.next();
                        }
                        Some('"') => break,
                        Some(_) => {}
                        None => return Err("SQL 标识符没有闭合".into()),
                    }
                }
            }
            '-' if chars.peek() == Some(&'-') => {
                chars.next();
                for next in chars.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
                output.push(' ');
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut closed = false;
                while let Some(next) = chars.next() {
                    if next == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    return Err("SQL 注释没有闭合".into());
                }
                output.push(' ');
            }
            ';' => {
                statement_ended = true;
                if chars.clone().any(|next| !next.is_whitespace()) {
                    return Err("一次只能执行一条查询语句".into());
                }
            }
            value if statement_ended && !value.is_whitespace() => {
                return Err("一次只能执行一条查询语句".into())
            }
            value if value.is_ascii_alphanumeric() || value == '_' => {
                output.push(value.to_ascii_uppercase())
            }
            _ => output.push(' '),
        }
    }
    Ok(output)
}

fn first_keyword(sql: &str) -> Option<String> {
    normalized_sql(sql)
        .ok()?
        .split_whitespace()
        .next()
        .map(str::to_string)
}

fn sql_string(value: &str) -> String {
    value.replace('\'', "''")
}

fn read_limited_text(path: &Path, max_bytes: u64) -> Result<String, String> {
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.take(max_bytes)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn directory_size(path: &Path) -> Result<u64, String> {
    if !path.exists() {
        return Ok(0);
    }
    if path.is_file() {
        return fs::metadata(path)
            .map(|metadata| metadata.len())
            .map_err(|error| error.to_string());
    }
    let mut size = 0_u64;
    for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        size = size.saturating_add(directory_size(&entry.path())?);
    }
    Ok(size)
}

fn devbox_root() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".devbox"))
        .ok_or_else(|| "无法确定当前用户目录".to_string())
}

fn unique_suffix() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_single_read_only_queries() {
        assert!(validate_query("SELECT * FROM selected_file LIMIT 10;").is_ok());
        assert!(validate_query("WITH rows AS (SELECT 1) SELECT * FROM rows").is_ok());
        assert!(validate_query("SHOW ALL TABLES").is_ok());
        assert!(validate_query("DESCRIBE selected_file").is_ok());
    }

    #[test]
    fn rejects_mutation_and_multiple_statements() {
        assert!(validate_query("DELETE FROM selected_file").is_err());
        assert!(validate_query("WITH x AS (DELETE FROM t) SELECT * FROM x").is_err());
        assert!(validate_query("SELECT 1; DROP TABLE t").is_err());
        assert!(validate_query(".shell whoami").is_err());
    }

    #[test]
    fn ignores_keywords_and_semicolons_inside_literals_and_comments() {
        assert!(validate_query("SELECT 'DROP; TABLE', \"SET\" -- DELETE\n").is_ok());
        assert!(validate_query("SELECT 1 /* COPY TO file */;").is_ok());
    }

    #[test]
    fn identifies_supported_local_files() {
        assert_eq!(
            file_kind(Path::new("data.CSV")).unwrap(),
            LocalFileKind::Csv
        );
        assert_eq!(
            file_kind(Path::new("events.jsonl")).unwrap(),
            LocalFileKind::Json
        );
        assert_eq!(
            file_kind(Path::new("data.parquet")).unwrap(),
            LocalFileKind::Parquet
        );
        assert_eq!(
            file_kind(Path::new("analytics.duckdb")).unwrap(),
            LocalFileKind::Database
        );
        assert!(file_kind(Path::new("notes.txt")).is_err());
    }

    #[test]
    fn escapes_paths_used_as_sql_strings() {
        assert_eq!(sql_string("/tmp/o'hare.csv"), "/tmp/o''hare.csv");
    }

    #[test]
    fn caps_select_results_but_keeps_metadata_queries() {
        let path = Path::new("/tmp/data.csv");
        let select = build_query(path, LocalFileKind::Csv, "SELECT * FROM selected_file;");
        assert!(select.contains("LIMIT 501"));
        assert!(select.contains("CREATE TEMP VIEW selected_file"));
        assert!(select.contains("SET enable_external_access = false"));

        let show = build_query(path, LocalFileKind::Database, "SHOW ALL TABLES;");
        assert!(show.ends_with("SHOW ALL TABLES;"));
        assert!(!show.contains("zhiyu_query_result"));
    }

    #[test]
    #[ignore = "requires DuckDB installed in ~/.devbox"]
    fn live_queries_csv_and_database_files() {
        let root = devbox_root().unwrap();
        let executable = root
            .join("installations")
            .join("duckdb")
            .join(DUCKDB_SERIES)
            .join("bin/duckdb");
        assert!(executable.is_file());

        let fixture = std::env::temp_dir().join(format!("zhiyu-duckdb-{}", unique_suffix()));
        fs::create_dir_all(&fixture).unwrap();
        let csv_path = fixture.join("scores.csv");
        fs::write(&csv_path, "name,score\nAda,40\nLinus,2\n").unwrap();
        let csv_result = run_query(
            csv_path.to_str().unwrap(),
            "SELECT sum(score) AS total FROM selected_file;",
        )
        .unwrap();
        assert_eq!(csv_result.columns, vec!["total"]);
        assert_eq!(csv_result.rows, vec![vec![Some("42".into())]]);

        let database_path = fixture.join("sample.duckdb");
        let status = Command::new(&executable)
            .arg(&database_path)
            .arg("-c")
            .arg("CREATE TABLE people AS SELECT 'Ada' AS name, 36 AS age;")
            .status()
            .unwrap();
        assert!(status.success());
        let database_result = run_query(
            database_path.to_str().unwrap(),
            "SELECT name, age FROM people;",
        )
        .unwrap();
        assert_eq!(database_result.columns, vec!["name", "age"]);
        assert_eq!(
            database_result.rows,
            vec![vec![Some("Ada".into()), Some("36".into())]]
        );
        fs::remove_dir_all(fixture).unwrap();
    }
}
