use crate::commands::ServiceKindInput;
use devbox_core::installer::{MYSQL_SERIES, POSTGRES_SERIES};
use devbox_core::{ConfigManager, ServiceKind};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Instant;

const POSTGRES_PORT: u16 = 5432;
const MAX_NAME_LENGTH: usize = 256;
const MAX_SQL_LENGTH: usize = 1024 * 1024;
const MAX_RESULT_ROWS: usize = 500;
const MYSQL_DATABASES_SQL: &str = concat!(
    "SELECT SCHEMA_NAME, ",
    "COALESCE((SELECT SUM(DATA_LENGTH + INDEX_LENGTH) ",
    "FROM information_schema.TABLES t ",
    "WHERE t.TABLE_SCHEMA = s.SCHEMA_NAME), 0), ",
    "CASE WHEN SCHEMA_NAME IN ",
    "('information_schema','mysql','performance_schema','sys') ",
    "THEN 1 ELSE 0 END ",
    "FROM information_schema.SCHEMATA s ORDER BY 3, 1"
);
const POSTGRES_DATABASES_SQL: &str = concat!(
    "SELECT datname, pg_database_size(datname), ",
    "CASE WHEN datname = 'postgres' THEN 1 ELSE 0 END ",
    "FROM pg_database WHERE datistemplate = false ORDER BY 3, 1"
);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseOverview {
    version: String,
    database_count: u64,
    table_count: u64,
    connection_count: u64,
    data_size_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    name: String,
    size_bytes: u64,
    system: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableInfo {
    schema: String,
    name: String,
    row_count: u64,
    size_bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnInfo {
    name: String,
    data_type: String,
    nullable: bool,
    key: String,
    default_value: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SqlResult {
    columns: Vec<String>,
    rows: Vec<Vec<Option<String>>>,
    summary: String,
    elapsed_ms: u128,
    truncated: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableDetail {
    columns: Vec<ColumnInfo>,
    preview: SqlResult,
}

#[derive(Clone, Copy)]
enum DatabaseEngine {
    Mysql,
    Postgres,
}

impl TryFrom<ServiceKindInput> for DatabaseEngine {
    type Error = String;

    fn try_from(value: ServiceKindInput) -> Result<Self, Self::Error> {
        match value {
            ServiceKindInput::Mysql => Ok(Self::Mysql),
            ServiceKindInput::Postgres => Ok(Self::Postgres),
            ServiceKindInput::Redis => Err("Redis 不使用 SQL 数据库接口".into()),
            ServiceKindInput::Mongodb => Err("MongoDB 不使用 SQL 数据库接口".into()),
            ServiceKindInput::Mailpit => Err("Mailpit 不使用 SQL 数据库接口".into()),
            ServiceKindInput::Nats => Err("NATS 不使用 SQL 数据库接口".into()),
            ServiceKindInput::Meilisearch => Err("Meilisearch 不使用 SQL 数据库接口".into()),
            ServiceKindInput::Minio => Err("MinIO 不使用 SQL 数据库接口".into()),
            ServiceKindInput::Rustfs => Err("RustFS 不使用 SQL 数据库接口".into()),
            ServiceKindInput::Etcd => Err("etcd 不使用 SQL 数据库接口".into()),
            ServiceKindInput::Consul => Err("Consul 不使用 SQL 数据库接口".into()),
            ServiceKindInput::Rnacos => Err("rnacos 不使用 SQL 数据库接口".into()),
        }
    }
}

#[tauri::command]
pub async fn database_overview(kind: ServiceKindInput) -> Result<DatabaseOverview, String> {
    tauri::async_runtime::spawn_blocking(move || read_overview(kind.try_into()?))
        .await
        .map_err(|error| format!("数据库概览任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn database_list(kind: ServiceKindInput) -> Result<Vec<DatabaseInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || read_databases(kind.try_into()?))
        .await
        .map_err(|error| format!("数据库列表任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn database_tables(
    kind: ServiceKindInput,
    database: String,
) -> Result<Vec<TableInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_name(&database, "数据库")?;
        read_tables(kind.try_into()?, &database)
    })
    .await
    .map_err(|error| format!("数据表列表任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn database_table_detail(
    kind: ServiceKindInput,
    database: String,
    schema: String,
    table: String,
) -> Result<TableDetail, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_name(&database, "数据库")?;
        validate_name(&schema, "Schema")?;
        validate_name(&table, "数据表")?;
        read_table_detail(kind.try_into()?, &database, &schema, &table)
    })
    .await
    .map_err(|error| format!("数据表详情任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn database_execute(
    kind: ServiceKindInput,
    database: String,
    sql: String,
    confirmed: bool,
) -> Result<SqlResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_name(&database, "数据库")?;
        execute_sql(kind.try_into()?, &database, sql, confirmed)
    })
    .await
    .map_err(|error| format!("SQL 执行任务异常结束: {error}"))?
}

fn read_overview(engine: DatabaseEngine) -> Result<DatabaseOverview, String> {
    let sql = match engine {
        DatabaseEngine::Mysql => {
            "SELECT VERSION(),\
             (SELECT COUNT(*) FROM information_schema.SCHEMATA),\
             (SELECT COUNT(*) FROM information_schema.TABLES WHERE TABLE_TYPE='BASE TABLE'),\
             (SELECT COUNT(*) FROM information_schema.PROCESSLIST),\
             COALESCE((SELECT SUM(DATA_LENGTH + INDEX_LENGTH) FROM information_schema.TABLES), 0)"
        }
        DatabaseEngine::Postgres => {
            "SELECT current_setting('server_version'),\
             (SELECT COUNT(*) FROM pg_database WHERE datistemplate = false),\
             (SELECT COUNT(*) FROM pg_stat_user_tables),\
             (SELECT COUNT(*) FROM pg_stat_activity),\
             COALESCE((SELECT SUM(pg_database_size(datname)) FROM pg_database WHERE datistemplate = false), 0)"
        }
    };
    let result = run_query(engine, default_database(engine), sql)?;
    let row = result
        .rows
        .first()
        .ok_or_else(|| "数据库没有返回概览信息".to_string())?;
    Ok(DatabaseOverview {
        version: cell(row, 0),
        database_count: cell_u64(row, 1),
        table_count: cell_u64(row, 2),
        connection_count: cell_u64(row, 3),
        data_size_bytes: cell_u64(row, 4),
    })
}

fn read_databases(engine: DatabaseEngine) -> Result<Vec<DatabaseInfo>, String> {
    let sql = match engine {
        DatabaseEngine::Mysql => MYSQL_DATABASES_SQL,
        DatabaseEngine::Postgres => POSTGRES_DATABASES_SQL,
    };
    let result = run_query(engine, default_database(engine), sql)?;
    Ok(result
        .rows
        .into_iter()
        .map(|row| DatabaseInfo {
            name: cell(&row, 0),
            size_bytes: cell_u64(&row, 1),
            system: cell_u64(&row, 2) == 1,
        })
        .collect())
}

fn read_tables(engine: DatabaseEngine, database: &str) -> Result<Vec<TableInfo>, String> {
    let sql = match engine {
        DatabaseEngine::Mysql => format!(
            "SELECT TABLE_SCHEMA, TABLE_NAME, COALESCE(TABLE_ROWS, 0), \
             COALESCE(DATA_LENGTH + INDEX_LENGTH, 0) \
             FROM information_schema.TABLES \
             WHERE TABLE_SCHEMA = '{}' AND TABLE_TYPE = 'BASE TABLE' ORDER BY TABLE_NAME",
            sql_literal(database)
        ),
        DatabaseEngine::Postgres => "SELECT n.nspname, c.relname, COALESCE(s.n_live_tup, 0), \
             pg_total_relation_size(c.oid) \
             FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace \
             LEFT JOIN pg_stat_user_tables s ON s.relid = c.oid \
             WHERE c.relkind IN ('r','p') \
             AND n.nspname NOT IN ('pg_catalog','information_schema') \
             ORDER BY n.nspname, c.relname"
            .into(),
    };
    let result = run_query(engine, database, &sql)?;
    Ok(result
        .rows
        .into_iter()
        .map(|row| TableInfo {
            schema: cell(&row, 0),
            name: cell(&row, 1),
            row_count: cell_u64(&row, 2),
            size_bytes: cell_u64(&row, 3),
        })
        .collect())
}

fn read_table_detail(
    engine: DatabaseEngine,
    database: &str,
    schema: &str,
    table: &str,
) -> Result<TableDetail, String> {
    let columns_sql = match engine {
        DatabaseEngine::Mysql => format!(
            "SELECT COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE, COLUMN_KEY, COLUMN_DEFAULT \
             FROM information_schema.COLUMNS \
             WHERE TABLE_SCHEMA = '{}' AND TABLE_NAME = '{}' ORDER BY ORDINAL_POSITION",
            sql_literal(database),
            sql_literal(table)
        ),
        DatabaseEngine::Postgres => format!(
            "SELECT column_name, data_type, is_nullable, '', column_default \
             FROM information_schema.columns \
             WHERE table_schema = '{}' AND table_name = '{}' ORDER BY ordinal_position",
            sql_literal(schema),
            sql_literal(table)
        ),
    };
    let result = run_query(engine, database, &columns_sql)?;
    let columns = result
        .rows
        .into_iter()
        .map(|row| ColumnInfo {
            name: cell(&row, 0),
            data_type: cell(&row, 1),
            nullable: cell(&row, 2).eq_ignore_ascii_case("yes"),
            key: cell(&row, 3),
            default_value: row.get(4).cloned().flatten(),
        })
        .collect();
    let preview_sql = match engine {
        DatabaseEngine::Mysql => format!(
            "SELECT * FROM {}.{} LIMIT 100",
            mysql_identifier(database),
            mysql_identifier(table)
        ),
        DatabaseEngine::Postgres => format!(
            "SELECT * FROM {}.{} LIMIT 100",
            postgres_identifier(schema),
            postgres_identifier(table)
        ),
    };
    let preview = run_query(engine, database, &preview_sql)?;
    Ok(TableDetail { columns, preview })
}

fn execute_sql(
    engine: DatabaseEngine,
    database: &str,
    sql: String,
    confirmed: bool,
) -> Result<SqlResult, String> {
    let sql = sql.trim();
    if sql.is_empty() {
        return Err("请输入 SQL".into());
    }
    if sql.len() > MAX_SQL_LENGTH || sql.contains('\0') {
        return Err("SQL 不能超过 1 MiB，且不能包含 NUL 字符".into());
    }
    let normalized = sql.to_ascii_uppercase();
    for blocked in [
        "SHUTDOWN",
        "SET GLOBAL",
        "INSTALL PLUGIN",
        "UNINSTALL PLUGIN",
    ] {
        if normalized.contains(blocked) {
            return Err(format!("智屿 SQL 命令台不执行 {blocked}"));
        }
    }
    for destructive in ["DROP DATABASE", "DROP SCHEMA", "DROP TABLE", "TRUNCATE"] {
        if normalized.contains(destructive) && !confirmed {
            return Err(format!("CONFIRM_REQUIRED:{destructive}"));
        }
    }
    run_query(engine, database, sql)
}

fn run_query(engine: DatabaseEngine, database: &str, sql: &str) -> Result<SqlResult, String> {
    let started = Instant::now();
    let output = match engine {
        DatabaseEngine::Mysql => mysql_command(database)?.arg("--execute").arg(sql).output(),
        DatabaseEngine::Postgres => postgres_command(database)?
            .arg("--command")
            .arg(sql)
            .output(),
    }
    .map_err(|error| format!("数据库客户端执行失败: {error}"))?;
    let elapsed_ms = started.elapsed().as_millis();
    parse_query_output(engine, output, elapsed_ms)
}

fn mysql_command(database: &str) -> Result<Command, String> {
    let root = devbox_root()?;
    let metadata = root.join("instances/mysql/default/service.json");
    let executable = ConfigManager
        .load(metadata)
        .ok()
        .filter(|config| config.kind == ServiceKind::Mysql)
        .map(|config| config.executable.with_file_name("mysql"))
        .unwrap_or_else(|| {
            root.join("installations/mysql")
                .join(MYSQL_SERIES)
                .join("bin/mysql")
        });
    ensure_executable(&executable)?;
    let mut command = Command::new(executable);
    command.args([
        "--no-defaults",
        "--no-login-paths",
        "--batch",
        "--default-character-set=utf8mb4",
        "--connect-timeout=2",
        "--protocol=TCP",
        "--host=127.0.0.1",
        "--port=3306",
        "--user=root",
    ]);
    if !database.is_empty() {
        command.arg(format!("--database={database}"));
    }
    Ok(command)
}

fn postgres_command(database: &str) -> Result<Command, String> {
    let root = devbox_root()?;
    let metadata = root.join("instances/postgres/default/service.json");
    let executable = ConfigManager
        .load(metadata)
        .ok()
        .filter(|config| config.kind == ServiceKind::Postgres)
        .map(|config| config.executable.with_file_name("psql"))
        .unwrap_or_else(|| {
            root.join("installations/postgres")
                .join(POSTGRES_SERIES)
                .join("bin/psql")
        });
    ensure_executable(&executable)?;
    let mut command = Command::new(executable);
    command
        .args([
            "--no-psqlrc",
            "--no-password",
            "--csv",
            "--username=postgres",
            "--host=127.0.0.1",
        ])
        .arg(format!("--port={POSTGRES_PORT}"))
        .arg(format!("--dbname={database}"));
    Ok(command)
}

fn parse_query_output(
    engine: DatabaseEngine,
    output: Output,
    elapsed_ms: u128,
) -> Result<SqlResult, String> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(if stderr.is_empty() { stdout } else { stderr });
    }
    match engine {
        DatabaseEngine::Mysql => parse_mysql_output(&output.stdout, elapsed_ms),
        DatabaseEngine::Postgres => parse_postgres_output(&output.stdout, elapsed_ms),
    }
}

fn parse_mysql_output(bytes: &[u8], elapsed_ms: u128) -> Result<SqlResult, String> {
    let text = String::from_utf8_lossy(bytes);
    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        return Ok(empty_result(elapsed_ms));
    };
    let columns = header.split('\t').map(mysql_unescape).collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut truncated = false;
    for line in lines {
        if rows.len() == MAX_RESULT_ROWS {
            truncated = true;
            break;
        }
        rows.push(
            line.split('\t')
                .map(|value| {
                    if value == "NULL" {
                        None
                    } else {
                        Some(mysql_unescape(value))
                    }
                })
                .collect(),
        );
    }
    let summary = format!("返回 {} 行", rows.len());
    Ok(SqlResult {
        columns,
        rows,
        summary,
        elapsed_ms,
        truncated,
    })
}

fn parse_postgres_output(bytes: &[u8], elapsed_ms: u128) -> Result<SqlResult, String> {
    let text = String::from_utf8_lossy(bytes);
    if text.trim().is_empty() {
        return Ok(empty_result(elapsed_ms));
    }
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(text.as_bytes());
    let columns = reader
        .headers()
        .map_err(|error| format!("PostgreSQL CSV 表头解析失败: {error}"))?
        .iter()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut truncated = false;
    for record in reader.records() {
        if rows.len() == MAX_RESULT_ROWS {
            truncated = true;
            break;
        }
        let record = record.map_err(|error| format!("PostgreSQL CSV 解析失败: {error}"))?;
        rows.push(
            record
                .iter()
                .map(|value| {
                    if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    }
                })
                .collect(),
        );
    }
    let summary = if rows.is_empty() && columns.len() == 1 {
        columns[0].clone()
    } else {
        format!("返回 {} 行", rows.len())
    };
    Ok(SqlResult {
        columns,
        rows,
        summary,
        elapsed_ms,
        truncated,
    })
}

fn empty_result(elapsed_ms: u128) -> SqlResult {
    SqlResult {
        columns: Vec::new(),
        rows: Vec::new(),
        summary: "执行完成".into(),
        elapsed_ms,
        truncated: false,
    }
}

fn mysql_unescape(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }
        match chars.next() {
            Some('n') => result.push('\n'),
            Some('t') => result.push('\t'),
            Some('r') => result.push('\r'),
            Some('0') => result.push('\0'),
            Some('Z') => result.push('\u{001a}'),
            Some('\\') => result.push('\\'),
            Some(other) => {
                result.push('\\');
                result.push(other);
            }
            None => result.push('\\'),
        }
    }
    result
}

fn default_database(engine: DatabaseEngine) -> &'static str {
    match engine {
        DatabaseEngine::Mysql => "",
        DatabaseEngine::Postgres => "postgres",
    }
}

fn devbox_root() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".devbox"))
        .ok_or_else(|| "无法确定当前用户目录".to_string())
}

fn ensure_executable(path: &Path) -> Result<(), String> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("数据库客户端不存在: {}", path.display()))
    }
}

fn validate_name(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > MAX_NAME_LENGTH || value.contains('\0') {
        Err(format!("{label}名称无效"))
    } else {
        Ok(())
    }
}

fn sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn mysql_identifier(value: &str) -> String {
    format!("`{}`", value.replace('`', "``"))
}

fn postgres_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn cell(row: &[Option<String>], index: usize) -> String {
    row.get(index).cloned().flatten().unwrap_or_default()
}

fn cell_u64(row: &[Option<String>], index: usize) -> u64 {
    cell(row, index).parse().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::ExitStatus;

    #[cfg(unix)]
    fn success_output(stdout: &str) -> Output {
        use std::os::unix::process::ExitStatusExt;

        Output {
            status: ExitStatus::from_raw(0),
            stdout: stdout.as_bytes().to_vec(),
            stderr: Vec::new(),
        }
    }

    #[test]
    fn mysql_batch_values_are_unescaped() {
        assert_eq!(mysql_unescape(r"hello\nworld\t\\"), "hello\nworld\t\\");
    }

    #[test]
    fn identifiers_are_quoted_for_each_engine() {
        assert_eq!(mysql_identifier("odd`name"), "`odd``name`");
        assert_eq!(postgres_identifier("odd\"name"), "\"odd\"\"name\"");
        assert_eq!(sql_literal("it's"), "it''s");
    }

    #[test]
    fn database_list_queries_keep_keyword_boundaries() {
        assert!(MYSQL_DATABASES_SQL.contains("END FROM"));
        assert!(POSTGRES_DATABASES_SQL.contains("END FROM"));
        assert!(!MYSQL_DATABASES_SQL.contains("ENDFROM"));
        assert!(!POSTGRES_DATABASES_SQL.contains("ENDFROM"));
    }

    #[test]
    fn destructive_sql_requires_confirmation_before_client_lookup() {
        let error = execute_sql(
            DatabaseEngine::Mysql,
            "test",
            "DROP TABLE users".into(),
            false,
        )
        .unwrap_err();
        assert_eq!(error, "CONFIRM_REQUIRED:DROP TABLE");
    }

    #[test]
    #[cfg(unix)]
    fn parses_mysql_and_postgres_tabular_results() {
        let mysql = parse_query_output(
            DatabaseEngine::Mysql,
            success_output("name\tvalue\nhello\\nworld\tNULL\n"),
            3,
        )
        .unwrap();
        assert_eq!(mysql.columns, ["name", "value"]);
        assert_eq!(mysql.rows[0][0].as_deref(), Some("hello\nworld"));
        assert_eq!(mysql.rows[0][1], None);

        let postgres = parse_query_output(
            DatabaseEngine::Postgres,
            success_output("name,value\nhello,42\n"),
            4,
        )
        .unwrap();
        assert_eq!(postgres.columns, ["name", "value"]);
        assert_eq!(postgres.rows[0][1].as_deref(), Some("42"));
    }
}
