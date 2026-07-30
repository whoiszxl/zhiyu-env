use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use time::{OffsetDateTime, UtcOffset};

const MAX_OUTPUT_BYTES: usize = 64 * 1024;
const MAX_HISTORY_ROWS: usize = 200;
const MIN_INTERVAL_MINUTES: u32 = 1;
const MAX_INTERVAL_MINUTES: u32 = 30 * 24 * 60;

static SCHEDULER_STARTED: OnceLock<()> = OnceLock::new();
static RUNNING_TASKS: OnceLock<Mutex<HashSet<i64>>> = OnceLock::new();
static CANCELLED_TASKS: OnceLock<Mutex<HashSet<i64>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTask {
    pub id: i64,
    pub name: String,
    pub schedule_kind: String,
    pub cron_expression: String,
    pub interval_minutes: u32,
    pub command: String,
    pub working_directory: String,
    pub timeout_seconds: u32,
    pub enabled: bool,
    pub running: bool,
    pub next_run_at_millis: Option<u64>,
    pub last_run_at_millis: Option<u64>,
    pub last_status: Option<String>,
    pub run_count: u32,
    pub created_at_millis: u64,
    pub updated_at_millis: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTaskInput {
    pub id: Option<i64>,
    pub name: String,
    pub schedule_kind: String,
    pub cron_expression: String,
    pub interval_minutes: u32,
    pub command: String,
    pub working_directory: String,
    pub timeout_seconds: u32,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTaskRun {
    pub id: i64,
    pub task_id: i64,
    pub started_at_millis: u64,
    pub finished_at_millis: u64,
    pub duration_millis: u64,
    pub status: String,
    pub exit_code: Option<i32>,
    pub output: String,
    pub trigger: String,
}

#[derive(Clone)]
struct ExecutableTask {
    id: i64,
    command: String,
    working_directory: String,
    timeout_seconds: u32,
}

#[derive(Clone)]
struct ParsedCron {
    minute: HashSet<u8>,
    hour: HashSet<u8>,
    day: HashSet<u8>,
    month: HashSet<u8>,
    weekday: HashSet<u8>,
    day_wildcard: bool,
    weekday_wildcard: bool,
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
        .join("scheduler");
    Ok(directory.join("scheduler.db"))
}

fn open_repo() -> Result<Connection, String> {
    open_repo_at(&db_path()?)
}

fn open_repo_at(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("无法创建定时任务数据目录: {error}"))?;
    }
    let connection =
        Connection::open(path).map_err(|error| format!("无法打开定时任务数据库: {error}"))?;
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=3000;
             CREATE TABLE IF NOT EXISTS scheduled_tasks (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               name TEXT NOT NULL,
               schedule_kind TEXT NOT NULL CHECK(schedule_kind IN ('cron', 'interval')),
               cron_expression TEXT NOT NULL DEFAULT '',
               interval_minutes INTEGER NOT NULL DEFAULT 60,
               command TEXT NOT NULL,
               working_directory TEXT NOT NULL DEFAULT '',
               timeout_seconds INTEGER NOT NULL DEFAULT 60,
               enabled INTEGER NOT NULL DEFAULT 1,
               next_run_at_millis INTEGER,
               last_run_at_millis INTEGER,
               last_status TEXT,
               run_count INTEGER NOT NULL DEFAULT 0,
               created_at_millis INTEGER NOT NULL,
               updated_at_millis INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_scheduled_tasks_due
               ON scheduled_tasks(enabled, next_run_at_millis);
             CREATE TABLE IF NOT EXISTS scheduled_task_runs (
               id INTEGER PRIMARY KEY AUTOINCREMENT,
               task_id INTEGER NOT NULL REFERENCES scheduled_tasks(id) ON DELETE CASCADE,
               started_at_millis INTEGER NOT NULL,
               finished_at_millis INTEGER NOT NULL,
               duration_millis INTEGER NOT NULL,
               status TEXT NOT NULL,
               exit_code INTEGER,
               output TEXT NOT NULL DEFAULT '',
               trigger TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_scheduled_task_runs_task
               ON scheduled_task_runs(task_id, started_at_millis DESC);
             PRAGMA foreign_keys=ON;",
        )
        .map_err(|error| format!("定时任务数据库初始化失败: {error}"))?;
    Ok(connection)
}

fn task_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScheduledTask> {
    Ok(ScheduledTask {
        id: row.get(0)?,
        name: row.get(1)?,
        schedule_kind: row.get(2)?,
        cron_expression: row.get(3)?,
        interval_minutes: row.get(4)?,
        command: row.get(5)?,
        working_directory: row.get(6)?,
        timeout_seconds: row.get(7)?,
        enabled: row.get(8)?,
        running: false,
        next_run_at_millis: row.get(9)?,
        last_run_at_millis: row.get(10)?,
        last_status: row.get(11)?,
        run_count: row.get(12)?,
        created_at_millis: row.get(13)?,
        updated_at_millis: row.get(14)?,
    })
}

fn select_task(connection: &Connection, id: i64) -> Result<ScheduledTask, String> {
    let mut task = connection
        .query_row(
            "SELECT id, name, schedule_kind, cron_expression, interval_minutes, command,
                    working_directory, timeout_seconds, enabled, next_run_at_millis,
                    last_run_at_millis, last_status, run_count, created_at_millis,
                    updated_at_millis
             FROM scheduled_tasks WHERE id = ?1",
            [id],
            task_from_row,
        )
        .optional()
        .map_err(|error| format!("无法读取定时任务: {error}"))?
        .ok_or_else(|| "定时任务不存在".to_string())?;
    task.running = running_tasks().lock().unwrap().contains(&task.id);
    Ok(task)
}

fn normalize_name(value: &str, names: &[(&str, u8)]) -> String {
    let upper = value.to_ascii_uppercase();
    names.iter().fold(upper, |current, (name, number)| {
        current.replace(name, &number.to_string())
    })
}

fn parse_field(
    raw: &str,
    minimum: u8,
    maximum: u8,
    names: &[(&str, u8)],
    weekday: bool,
) -> Result<HashSet<u8>, String> {
    let normalized = normalize_name(raw.trim(), names);
    let mut values = HashSet::new();
    for item in normalized.split(',') {
        if item.is_empty() {
            return Err("Cron 字段不能为空".into());
        }
        let (base, step, has_step) = match item.split_once('/') {
            Some((base, step)) => {
                let parsed = step
                    .parse::<u8>()
                    .map_err(|_| format!("Cron 步长无效: {item}"))?;
                if parsed == 0 {
                    return Err("Cron 步长不能为 0".into());
                }
                (base, parsed, true)
            }
            None => (item, 1, false),
        };
        let (start, end) = if base == "*" {
            (minimum, maximum)
        } else if let Some((start, end)) = base.split_once('-') {
            (
                start
                    .parse::<u8>()
                    .map_err(|_| format!("Cron 范围无效: {item}"))?,
                end.parse::<u8>()
                    .map_err(|_| format!("Cron 范围无效: {item}"))?,
            )
        } else {
            let value = base
                .parse::<u8>()
                .map_err(|_| format!("Cron 数值无效: {item}"))?;
            (value, if has_step { maximum } else { value })
        };
        if start < minimum || end > maximum || start > end {
            return Err(format!("Cron 数值超出范围: {item}"));
        }
        for value in start..=end {
            if (value - start) % step == 0 {
                values.insert(if weekday && value == 7 { 0 } else { value });
            }
        }
    }
    Ok(values)
}

fn parse_cron(expression: &str) -> Result<ParsedCron, String> {
    let fields = expression.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 5 {
        return Err("Cron 必须是 5 段格式：分 时 日 月 星期".into());
    }
    let month_names = [
        ("JAN", 1),
        ("FEB", 2),
        ("MAR", 3),
        ("APR", 4),
        ("MAY", 5),
        ("JUN", 6),
        ("JUL", 7),
        ("AUG", 8),
        ("SEP", 9),
        ("OCT", 10),
        ("NOV", 11),
        ("DEC", 12),
    ];
    let weekday_names = [
        ("SUN", 0),
        ("MON", 1),
        ("TUE", 2),
        ("WED", 3),
        ("THU", 4),
        ("FRI", 5),
        ("SAT", 6),
    ];
    Ok(ParsedCron {
        minute: parse_field(fields[0], 0, 59, &[], false)?,
        hour: parse_field(fields[1], 0, 23, &[], false)?,
        day: parse_field(fields[2], 1, 31, &[], false)?,
        month: parse_field(fields[3], 1, 12, &month_names, false)?,
        weekday: parse_field(fields[4], 0, 7, &weekday_names, true)?,
        day_wildcard: fields[2] == "*",
        weekday_wildcard: fields[4] == "*",
    })
}

fn cron_matches(cron: &ParsedCron, date: OffsetDateTime) -> bool {
    if !cron.minute.contains(&date.minute())
        || !cron.hour.contains(&date.hour())
        || !cron.month.contains(&(date.month() as u8))
    {
        return false;
    }
    let day_matches = cron.day.contains(&date.day());
    let weekday_matches = cron
        .weekday
        .contains(&(date.weekday().number_days_from_sunday()));
    match (cron.day_wildcard, cron.weekday_wildcard) {
        (true, true) => true,
        (true, false) => weekday_matches,
        (false, true) => day_matches,
        (false, false) => day_matches || weekday_matches,
    }
}

fn next_cron_millis(expression: &str, after_millis: u64) -> Result<u64, String> {
    let cron = parse_cron(expression)?;
    let after_seconds = (after_millis / 1000) as i64;
    let mut timestamp = after_seconds - after_seconds.rem_euclid(60) + 60;
    let limit = timestamp + 366 * 24 * 60 * 60;
    while timestamp <= limit {
        let utc = OffsetDateTime::from_unix_timestamp(timestamp)
            .map_err(|error| format!("无法计算下次执行时间: {error}"))?;
        let offset = UtcOffset::local_offset_at(utc).unwrap_or(UtcOffset::UTC);
        if cron_matches(&cron, utc.to_offset(offset)) {
            return Ok((timestamp as u64) * 1000);
        }
        timestamp += 60;
    }
    Err("未来一年内没有匹配的执行时间".into())
}

fn next_run_millis(
    schedule_kind: &str,
    cron_expression: &str,
    interval_minutes: u32,
    after_millis: u64,
) -> Result<u64, String> {
    match schedule_kind {
        "cron" => next_cron_millis(cron_expression, after_millis),
        "interval" => Ok(after_millis + u64::from(interval_minutes) * 60_000),
        _ => Err("调度类型只支持 cron 或 interval".into()),
    }
}

fn validate_input(input: &mut ScheduledTaskInput) -> Result<(), String> {
    input.name = input.name.trim().to_string();
    input.command = input.command.trim().to_string();
    input.cron_expression = input.cron_expression.trim().to_string();
    input.working_directory = input.working_directory.trim().to_string();
    if input.name.is_empty() || input.name.chars().count() > 80 {
        return Err("任务名称不能为空且不能超过 80 个字符".into());
    }
    if input.command.is_empty() || input.command.len() > 16_384 {
        return Err("执行命令不能为空且不能超过 16 KiB".into());
    }
    if !(1..=3600).contains(&input.timeout_seconds) {
        return Err("超时时间必须在 1 到 3600 秒之间".into());
    }
    if input.schedule_kind == "interval" {
        if !(MIN_INTERVAL_MINUTES..=MAX_INTERVAL_MINUTES).contains(&input.interval_minutes) {
            return Err("固定间隔必须在 1 分钟到 30 天之间".into());
        }
    } else if input.schedule_kind == "cron" {
        parse_cron(&input.cron_expression)?;
    } else {
        return Err("调度类型只支持 cron 或 interval".into());
    }
    if !input.working_directory.is_empty() {
        let path = Path::new(&input.working_directory);
        if !path.is_absolute() || !path.is_dir() {
            return Err("工作目录必须是已存在的绝对目录".into());
        }
    }
    Ok(())
}

#[tauri::command]
pub fn scheduled_tasks_list() -> Result<Vec<ScheduledTask>, String> {
    let connection = open_repo()?;
    let mut statement = connection
        .prepare(
            "SELECT id, name, schedule_kind, cron_expression, interval_minutes, command,
                    working_directory, timeout_seconds, enabled, next_run_at_millis,
                    last_run_at_millis, last_status, run_count, created_at_millis,
                    updated_at_millis
             FROM scheduled_tasks ORDER BY enabled DESC, name COLLATE NOCASE, id",
        )
        .map_err(|error| format!("无法读取定时任务: {error}"))?;
    let mut tasks = statement
        .query_map([], task_from_row)
        .map_err(|error| format!("无法读取定时任务: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("无法读取定时任务: {error}"))?;
    let running = running_tasks().lock().unwrap();
    for task in &mut tasks {
        task.running = running.contains(&task.id);
    }
    Ok(tasks)
}

#[tauri::command]
pub fn scheduled_task_save(mut input: ScheduledTaskInput) -> Result<ScheduledTask, String> {
    validate_input(&mut input)?;
    let connection = open_repo()?;
    let now = now_millis();
    let next_run = input.enabled.then(|| {
        next_run_millis(
            &input.schedule_kind,
            &input.cron_expression,
            input.interval_minutes,
            now,
        )
    });
    let next_run = next_run.transpose()?;
    let id = if let Some(id) = input.id {
        let changed = connection
            .execute(
                "UPDATE scheduled_tasks SET name = ?1, schedule_kind = ?2,
                   cron_expression = ?3, interval_minutes = ?4, command = ?5,
                   working_directory = ?6, timeout_seconds = ?7, enabled = ?8,
                   next_run_at_millis = ?9, updated_at_millis = ?10 WHERE id = ?11",
                params![
                    input.name,
                    input.schedule_kind,
                    input.cron_expression,
                    input.interval_minutes,
                    input.command,
                    input.working_directory,
                    input.timeout_seconds,
                    input.enabled,
                    next_run,
                    now,
                    id
                ],
            )
            .map_err(|error| format!("无法保存定时任务: {error}"))?;
        if changed == 0 {
            return Err("定时任务不存在".into());
        }
        id
    } else {
        connection
            .execute(
                "INSERT INTO scheduled_tasks (
                   name, schedule_kind, cron_expression, interval_minutes, command,
                   working_directory, timeout_seconds, enabled, next_run_at_millis,
                   created_at_millis, updated_at_millis
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                params![
                    input.name,
                    input.schedule_kind,
                    input.cron_expression,
                    input.interval_minutes,
                    input.command,
                    input.working_directory,
                    input.timeout_seconds,
                    input.enabled,
                    next_run,
                    now
                ],
            )
            .map_err(|error| format!("无法创建定时任务: {error}"))?;
        connection.last_insert_rowid()
    };
    select_task(&connection, id)
}

#[tauri::command]
pub fn scheduled_task_delete(id: i64) -> Result<(), String> {
    if running_tasks().lock().unwrap().contains(&id) {
        return Err("任务正在执行，暂时不能删除".into());
    }
    let connection = open_repo()?;
    connection
        .execute("DELETE FROM scheduled_tasks WHERE id = ?1", [id])
        .map_err(|error| format!("无法删除定时任务: {error}"))?;
    Ok(())
}

#[tauri::command]
pub fn scheduled_task_toggle(id: i64, enabled: bool) -> Result<ScheduledTask, String> {
    let connection = open_repo()?;
    let current = select_task(&connection, id)?;
    let next_run = if enabled {
        Some(next_run_millis(
            &current.schedule_kind,
            &current.cron_expression,
            current.interval_minutes,
            now_millis(),
        )?)
    } else {
        None
    };
    connection
        .execute(
            "UPDATE scheduled_tasks SET enabled = ?1, next_run_at_millis = ?2,
             updated_at_millis = ?3 WHERE id = ?4",
            params![enabled, next_run, now_millis(), id],
        )
        .map_err(|error| format!("无法更新定时任务状态: {error}"))?;
    select_task(&connection, id)
}

#[tauri::command]
pub fn scheduled_task_cancel(id: i64) -> Result<(), String> {
    if !running_tasks().lock().unwrap().contains(&id) {
        return Err("任务当前没有运行".into());
    }
    cancelled_tasks().lock().unwrap().insert(id);
    Ok(())
}

#[tauri::command]
pub fn scheduled_task_history(
    task_id: Option<i64>,
    limit: Option<u32>,
) -> Result<Vec<ScheduledTaskRun>, String> {
    let connection = open_repo()?;
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let sql = if task_id.is_some() {
        "SELECT id, task_id, started_at_millis, finished_at_millis, duration_millis,
                status, exit_code, output, trigger
         FROM scheduled_task_runs WHERE task_id = ?1
         ORDER BY started_at_millis DESC, id DESC LIMIT ?2"
    } else {
        "SELECT id, task_id, started_at_millis, finished_at_millis, duration_millis,
                status, exit_code, output, trigger
         FROM scheduled_task_runs
         ORDER BY started_at_millis DESC, id DESC LIMIT ?2"
    };
    let mut statement = connection
        .prepare(sql)
        .map_err(|error| format!("无法读取任务历史: {error}"))?;
    let mapper = |row: &rusqlite::Row<'_>| {
        Ok(ScheduledTaskRun {
            id: row.get(0)?,
            task_id: row.get(1)?,
            started_at_millis: row.get(2)?,
            finished_at_millis: row.get(3)?,
            duration_millis: row.get(4)?,
            status: row.get(5)?,
            exit_code: row.get(6)?,
            output: row.get(7)?,
            trigger: row.get(8)?,
        })
    };
    let rows = if let Some(task_id) = task_id {
        statement.query_map(params![task_id, limit], mapper)
    } else {
        statement.query_map(params![rusqlite::types::Null, limit], mapper)
    }
    .map_err(|error| format!("无法读取任务历史: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("无法读取任务历史: {error}"))
}

fn executable_task(id: i64) -> Result<ExecutableTask, String> {
    let connection = open_repo()?;
    connection
        .query_row(
            "SELECT id, command, working_directory, timeout_seconds
             FROM scheduled_tasks WHERE id = ?1",
            [id],
            |row| {
                Ok(ExecutableTask {
                    id: row.get(0)?,
                    command: row.get(1)?,
                    working_directory: row.get(2)?,
                    timeout_seconds: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|error| format!("无法读取定时任务: {error}"))?
        .ok_or_else(|| "定时任务不存在".into())
}

fn running_tasks() -> &'static Mutex<HashSet<i64>> {
    RUNNING_TASKS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn cancelled_tasks() -> &'static Mutex<HashSet<i64>> {
    CANCELLED_TASKS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn truncate_output(bytes: Vec<u8>) -> String {
    let slice = if bytes.len() > MAX_OUTPUT_BYTES {
        &bytes[bytes.len() - MAX_OUTPUT_BYTES..]
    } else {
        &bytes
    };
    String::from_utf8_lossy(slice).into_owned()
}

fn execute_task(task: ExecutableTask, trigger: &str) -> Result<ScheduledTaskRun, String> {
    {
        let mut running = running_tasks().lock().unwrap();
        if !running.insert(task.id) {
            return Err("任务已经在执行中".into());
        }
    }
    cancelled_tasks().lock().unwrap().remove(&task.id);
    let result = execute_task_inner(&task, trigger);
    running_tasks().lock().unwrap().remove(&task.id);
    cancelled_tasks().lock().unwrap().remove(&task.id);
    result
}

fn execute_task_inner(task: &ExecutableTask, trigger: &str) -> Result<ScheduledTaskRun, String> {
    let started_at = now_millis();
    let started = Instant::now();
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-NonInteractive", "-Command", &task.command]);
        command
    };
    #[cfg(not(target_os = "windows"))]
    let mut command = {
        let mut command = Command::new("/bin/sh");
        command.args(["-lc", &task.command]);
        command
    };
    if !task.working_directory.is_empty() {
        command.current_dir(&task.working_directory);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("无法启动定时任务命令: {error}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut stdout) = stdout {
            let _ = stdout.read_to_end(&mut bytes);
        }
        bytes
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut stderr) = stderr {
            let _ = stderr.read_to_end(&mut bytes);
        }
        bytes
    });
    let deadline = Instant::now() + Duration::from_secs(u64::from(task.timeout_seconds));
    let (status, exit_code) = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("无法读取定时任务进程状态: {error}"))?
        {
            break (
                if status.success() {
                    "success"
                } else {
                    "failed"
                },
                status.code(),
            );
        }
        let cancelled = cancelled_tasks().lock().unwrap().remove(&task.id);
        if cancelled || Instant::now() >= deadline {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.kill();
            let _ = child.wait();
            break (if cancelled { "cancelled" } else { "timed_out" }, None);
        }
        thread::sleep(Duration::from_millis(100));
    };
    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    let mut combined = stdout;
    if !stderr.is_empty() {
        if !combined.is_empty() {
            combined.extend_from_slice(b"\n");
        }
        combined.extend_from_slice(&stderr);
    }
    let output = truncate_output(combined);
    let finished_at = now_millis();
    let duration = started.elapsed().as_millis() as u64;
    let connection = open_repo()?;
    connection
        .execute(
            "INSERT INTO scheduled_task_runs (
               task_id, started_at_millis, finished_at_millis, duration_millis,
               status, exit_code, output, trigger
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                task.id,
                started_at,
                finished_at,
                duration,
                status,
                exit_code,
                output,
                trigger
            ],
        )
        .map_err(|error| format!("无法保存任务运行记录: {error}"))?;
    let run_id = connection.last_insert_rowid();
    connection
        .execute(
            "UPDATE scheduled_tasks SET last_run_at_millis = ?1, last_status = ?2,
             run_count = run_count + 1, updated_at_millis = ?1 WHERE id = ?3",
            params![finished_at, status, task.id],
        )
        .map_err(|error| format!("无法更新任务状态: {error}"))?;
    connection
        .execute(
            "DELETE FROM scheduled_task_runs WHERE id IN (
               SELECT id FROM scheduled_task_runs ORDER BY started_at_millis DESC
               LIMIT -1 OFFSET ?1
             )",
            [MAX_HISTORY_ROWS],
        )
        .map_err(|error| format!("无法清理旧任务记录: {error}"))?;
    Ok(ScheduledTaskRun {
        id: run_id,
        task_id: task.id,
        started_at_millis: started_at,
        finished_at_millis: finished_at,
        duration_millis: duration,
        status: status.into(),
        exit_code,
        output,
        trigger: trigger.into(),
    })
}

#[tauri::command]
pub async fn scheduled_task_run(id: i64) -> Result<ScheduledTaskRun, String> {
    let task = executable_task(id)?;
    tokio::task::spawn_blocking(move || execute_task(task, "manual"))
        .await
        .map_err(|error| format!("定时任务执行线程失败: {error}"))?
}

fn claim_due_tasks() -> Result<Vec<ExecutableTask>, String> {
    let mut connection = open_repo()?;
    let now = now_millis();
    let transaction = connection
        .transaction()
        .map_err(|error| format!("无法检查到期任务: {error}"))?;
    let due = {
        let mut statement = transaction
            .prepare(
                "SELECT id, schedule_kind, cron_expression, interval_minutes, command,
                        working_directory, timeout_seconds
                 FROM scheduled_tasks
                 WHERE enabled = 1 AND next_run_at_millis IS NOT NULL
                   AND next_run_at_millis <= ?1
                 ORDER BY next_run_at_millis LIMIT 8",
            )
            .map_err(|error| format!("无法检查到期任务: {error}"))?;
        let rows = statement
            .query_map([now], |row| {
                Ok((
                    ExecutableTask {
                        id: row.get(0)?,
                        command: row.get(4)?,
                        working_directory: row.get(5)?,
                        timeout_seconds: row.get(6)?,
                    },
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, u32>(3)?,
                ))
            })
            .map_err(|error| format!("无法检查到期任务: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("无法检查到期任务: {error}"))?;
        rows
    };
    let mut tasks = Vec::new();
    for (task, kind, cron, interval) in due {
        let next = next_run_millis(&kind, &cron, interval, now)?;
        transaction
            .execute(
                "UPDATE scheduled_tasks SET next_run_at_millis = ?1,
                 updated_at_millis = ?2 WHERE id = ?3",
                params![next, now, task.id],
            )
            .map_err(|error| format!("无法更新下次执行时间: {error}"))?;
        tasks.push(task);
    }
    transaction
        .commit()
        .map_err(|error| format!("无法提交定时任务调度: {error}"))?;
    Ok(tasks)
}

pub fn start_scheduler() {
    SCHEDULER_STARTED.get_or_init(|| {
        thread::spawn(|| loop {
            match claim_due_tasks() {
                Ok(tasks) => {
                    for task in tasks {
                        if !running_tasks().lock().unwrap().insert(task.id) {
                            continue;
                        }
                        cancelled_tasks().lock().unwrap().remove(&task.id);
                        thread::spawn(move || {
                            let id = task.id;
                            if let Err(error) = execute_task_inner(&task, "scheduled") {
                                eprintln!("定时任务执行失败: {error}");
                            }
                            running_tasks().lock().unwrap().remove(&id);
                            cancelled_tasks().lock().unwrap().remove(&id);
                        });
                    }
                }
                Err(error) => eprintln!("定时任务调度检查失败: {error}"),
            }
            thread::sleep(Duration::from_secs(1));
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_parser_supports_ranges_steps_and_names() {
        let cron = parse_cron("5/15 9-18 * JAN,MAR MON-FRI").unwrap();
        assert!(cron.minute.contains(&5));
        assert!(cron.minute.contains(&20));
        assert!(cron.minute.contains(&50));
        assert!(cron.hour.contains(&18));
        assert!(cron.month.contains(&1));
        assert!(cron.month.contains(&3));
        assert!(cron.weekday.contains(&1));
        assert!(cron.weekday.contains(&5));
    }

    #[test]
    fn repository_round_trip_works() {
        let directory = tempfile::tempdir().unwrap();
        let connection = open_repo_at(&directory.path().join("scheduler.db")).unwrap();
        connection
            .execute(
                "INSERT INTO scheduled_tasks (
                   name, schedule_kind, cron_expression, interval_minutes, command,
                   working_directory, timeout_seconds, enabled, next_run_at_millis,
                   created_at_millis, updated_at_millis
                 ) VALUES ('health', 'interval', '', 5, 'echo ok', '', 10, 1, 1, 1, 1)",
                [],
            )
            .unwrap();
        let task = select_task(&connection, connection.last_insert_rowid()).unwrap();
        assert_eq!(task.name, "health");
        assert_eq!(task.interval_minutes, 5);
        assert!(task.enabled);
    }
}
