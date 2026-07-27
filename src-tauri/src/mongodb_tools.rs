use bson::{doc, Bson, Document};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicI32, Ordering};
use std::time::{Duration, Instant};

const MONGODB_ADDRESS: ([u8; 4], u16) = ([127, 0, 0, 1], 27017);
const MAX_NAME_LENGTH: usize = 256;
const MAX_COMMAND_LENGTH: usize = 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;
const MAX_PREVIEW_DOCUMENTS: i64 = 100;
const OP_MSG: i32 = 2013;

static REQUEST_ID: AtomicI32 = AtomicI32::new(1);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MongoOverview {
    version: String,
    database_count: u64,
    connection_count: u64,
    data_size_bytes: u64,
    uptime_seconds: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MongoDatabaseInfo {
    name: String,
    size_bytes: u64,
    system: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MongoCollectionInfo {
    name: String,
    collection_type: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MongoFieldInfo {
    name: String,
    bson_type: String,
    occurrences: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MongoCollectionDetail {
    document_count: u64,
    size_bytes: u64,
    fields: Vec<MongoFieldInfo>,
    documents: Vec<Value>,
    truncated: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MongoCommandResult {
    output: Value,
    elapsed_ms: u128,
}

#[tauri::command]
pub async fn mongo_overview() -> Result<MongoOverview, String> {
    tauri::async_runtime::spawn_blocking(read_overview)
        .await
        .map_err(|error| format!("MongoDB 概览任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn mongo_databases() -> Result<Vec<MongoDatabaseInfo>, String> {
    tauri::async_runtime::spawn_blocking(read_databases)
        .await
        .map_err(|error| format!("MongoDB 数据库列表任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn mongo_collections(database: String) -> Result<Vec<MongoCollectionInfo>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_name(&database, "数据库")?;
        read_collections(&database)
    })
    .await
    .map_err(|error| format!("MongoDB 集合列表任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn mongo_collection_detail(
    database: String,
    collection: String,
) -> Result<MongoCollectionDetail, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_name(&database, "数据库")?;
        validate_name(&collection, "集合")?;
        read_collection_detail(&database, &collection)
    })
    .await
    .map_err(|error| format!("MongoDB 集合详情任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn mongo_execute(
    database: String,
    command: String,
    confirmed: bool,
) -> Result<MongoCommandResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        validate_name(&database, "数据库")?;
        execute_command(&database, command, confirmed)
    })
    .await
    .map_err(|error| format!("MongoDB 命令任务异常结束: {error}"))?
}

fn read_overview() -> Result<MongoOverview, String> {
    let build = run_command("admin", doc! { "buildInfo": 1 })?;
    let databases = run_command("admin", doc! { "listDatabases": 1, "nameOnly": false })?;
    let status = run_command("admin", doc! { "serverStatus": 1 })?;

    Ok(MongoOverview {
        version: string_field(&build, "version"),
        database_count: databases
            .get_array("databases")
            .map(|values| values.len() as u64)
            .unwrap_or_default(),
        connection_count: status
            .get_document("connections")
            .ok()
            .and_then(|value| value.get("current"))
            .map(bson_u64)
            .unwrap_or_default(),
        data_size_bytes: databases.get("totalSize").map(bson_u64).unwrap_or_default(),
        uptime_seconds: status.get("uptime").map(bson_u64).unwrap_or_default(),
    })
}

fn read_databases() -> Result<Vec<MongoDatabaseInfo>, String> {
    let result = run_command("admin", doc! { "listDatabases": 1, "nameOnly": false })?;
    let databases = result
        .get_array("databases")
        .map_err(|_| "MongoDB 未返回数据库列表".to_string())?;
    Ok(databases
        .iter()
        .filter_map(Bson::as_document)
        .map(|database| {
            let name = string_field(database, "name");
            MongoDatabaseInfo {
                system: matches!(name.as_str(), "admin" | "config" | "local"),
                size_bytes: database.get("sizeOnDisk").map(bson_u64).unwrap_or_default(),
                name,
            }
        })
        .collect())
}

fn read_collections(database: &str) -> Result<Vec<MongoCollectionInfo>, String> {
    let result = run_command(
        database,
        doc! { "listCollections": 1, "nameOnly": true, "cursor": { "batchSize": 500 } },
    )?;
    let batch = cursor_batch(&result)?;
    Ok(batch
        .iter()
        .filter_map(Bson::as_document)
        .map(|collection| MongoCollectionInfo {
            name: string_field(collection, "name"),
            collection_type: string_field(collection, "type"),
        })
        .collect())
}

fn read_collection_detail(
    database: &str,
    collection: &str,
) -> Result<MongoCollectionDetail, String> {
    let stats = run_command(database, doc! { "collStats": collection })?;
    let result = run_command(
        database,
        doc! {
            "find": collection,
            "filter": {},
            "limit": MAX_PREVIEW_DOCUMENTS,
            "batchSize": MAX_PREVIEW_DOCUMENTS,
        },
    )?;
    let batch = cursor_batch(&result)?;
    let mut field_types = BTreeMap::<String, (BTreeSet<String>, usize)>::new();
    let documents = batch
        .iter()
        .filter_map(Bson::as_document)
        .map(|document| {
            for (name, value) in document {
                let entry = field_types.entry(name.clone()).or_default();
                entry.0.insert(bson_type_name(value).into());
                entry.1 += 1;
            }
            serde_json::to_value(Bson::Document(document.clone()))
                .unwrap_or_else(|_| Value::String("<无法显示该文档>".into()))
        })
        .collect::<Vec<_>>();
    let document_count = stats.get("count").map(bson_u64).unwrap_or_default();

    Ok(MongoCollectionDetail {
        document_count,
        size_bytes: stats.get("size").map(bson_u64).unwrap_or_default(),
        fields: field_types
            .into_iter()
            .map(|(name, (types, occurrences))| MongoFieldInfo {
                name,
                bson_type: types.into_iter().collect::<Vec<_>>().join(" | "),
                occurrences,
            })
            .collect(),
        truncated: document_count > documents.len() as u64,
        documents,
    })
}

fn execute_command(
    database: &str,
    command: String,
    confirmed: bool,
) -> Result<MongoCommandResult, String> {
    if command.len() > MAX_COMMAND_LENGTH {
        return Err("MongoDB 命令不能超过 1 MiB".into());
    }
    let value: Value =
        serde_json::from_str(&command).map_err(|error| format!("命令不是有效的 JSON: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "MongoDB 命令必须是 JSON 对象".to_string())?;
    let command_name = object
        .keys()
        .next()
        .ok_or_else(|| "MongoDB 命令不能为空".to_string())?
        .to_ascii_lowercase();

    if matches!(
        command_name.as_str(),
        "shutdown" | "replsetreconfig" | "setparameter" | "compact" | "fsync"
    ) {
        return Err("智屿已禁用可能阻塞或改变服务器运行方式的管理命令".into());
    }
    if matches!(
        command_name.as_str(),
        "dropdatabase" | "drop" | "delete" | "findandmodify"
    ) && !confirmed
    {
        return Err("该命令可能删除数据，需要确认后执行".into());
    }

    let document =
        bson::to_document(&value).map_err(|error| format!("无法解析 MongoDB 命令: {error}"))?;
    let started = Instant::now();
    let output = run_command(database, document)?;
    Ok(MongoCommandResult {
        output: serde_json::to_value(Bson::Document(output))
            .map_err(|error| format!("无法显示 MongoDB 返回值: {error}"))?,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn run_command(database: &str, mut command: Document) -> Result<Document, String> {
    command.insert("$db", database);
    let payload =
        bson::to_vec(&command).map_err(|error| format!("无法编码 MongoDB 命令: {error}"))?;
    let message_length = 16_usize
        .checked_add(4)
        .and_then(|value| value.checked_add(1))
        .and_then(|value| value.checked_add(payload.len()))
        .ok_or_else(|| "MongoDB 命令过大".to_string())?;
    let message_length_i32 =
        i32::try_from(message_length).map_err(|_| "MongoDB 命令过大".to_string())?;
    let request_id = REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let mut message = Vec::with_capacity(message_length);
    message.extend_from_slice(&message_length_i32.to_le_bytes());
    message.extend_from_slice(&request_id.to_le_bytes());
    message.extend_from_slice(&0_i32.to_le_bytes());
    message.extend_from_slice(&OP_MSG.to_le_bytes());
    message.extend_from_slice(&0_u32.to_le_bytes());
    message.push(0);
    message.extend_from_slice(&payload);

    let address = SocketAddr::from(MONGODB_ADDRESS);
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|error| format!("无法连接 MongoDB（127.0.0.1:27017）: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(8)))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(8)))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(&message)
        .map_err(|error| format!("发送 MongoDB 命令失败: {error}"))?;

    let mut header = [0_u8; 16];
    stream
        .read_exact(&mut header)
        .map_err(|error| format!("读取 MongoDB 响应失败: {error}"))?;
    let response_length = i32::from_le_bytes(header[0..4].try_into().unwrap());
    let opcode = i32::from_le_bytes(header[12..16].try_into().unwrap());
    if response_length < 21 || response_length as usize > MAX_RESPONSE_BYTES {
        return Err("MongoDB 返回了无效的响应长度".into());
    }
    if opcode != OP_MSG {
        return Err(format!("MongoDB 返回了不支持的协议消息: {opcode}"));
    }

    let mut body = vec![0_u8; response_length as usize - 16];
    stream
        .read_exact(&mut body)
        .map_err(|error| format!("读取 MongoDB 响应正文失败: {error}"))?;
    if body.get(4) != Some(&0) {
        return Err("MongoDB 返回了不支持的响应分段".into());
    }
    let response = Document::from_reader(&mut Cursor::new(&body[5..]))
        .map_err(|error| format!("无法解析 MongoDB 响应: {error}"))?;
    if response.get("ok").map(bson_f64).unwrap_or_default() != 1.0 {
        let message = response.get_str("errmsg").unwrap_or("MongoDB 命令执行失败");
        return Err(message.into());
    }
    Ok(response)
}

fn cursor_batch(result: &Document) -> Result<&Vec<Bson>, String> {
    result
        .get_document("cursor")
        .and_then(|cursor| cursor.get_array("firstBatch"))
        .map_err(|_| "MongoDB 未返回游标数据".to_string())
}

fn validate_name(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > MAX_NAME_LENGTH || value.contains('\0') {
        Err(format!("{label}名称无效"))
    } else {
        Ok(())
    }
}

fn string_field(document: &Document, name: &str) -> String {
    document.get_str(name).unwrap_or_default().to_string()
}

fn bson_u64(value: &Bson) -> u64 {
    match value {
        Bson::Int32(value) => (*value).max(0) as u64,
        Bson::Int64(value) => (*value).max(0) as u64,
        Bson::Double(value) => value.max(0.0) as u64,
        _ => 0,
    }
}

fn bson_f64(value: &Bson) -> f64 {
    match value {
        Bson::Int32(value) => *value as f64,
        Bson::Int64(value) => *value as f64,
        Bson::Double(value) => *value,
        _ => 0.0,
    }
}

fn bson_type_name(value: &Bson) -> &'static str {
    match value {
        Bson::Double(_) => "double",
        Bson::String(_) => "string",
        Bson::Array(_) => "array",
        Bson::Document(_) => "document",
        Bson::Boolean(_) => "bool",
        Bson::Null => "null",
        Bson::RegularExpression(_) => "regex",
        Bson::JavaScriptCode(_) | Bson::JavaScriptCodeWithScope(_) => "javascript",
        Bson::Int32(_) => "int",
        Bson::Int64(_) => "long",
        Bson::Timestamp(_) => "timestamp",
        Bson::Binary(_) => "binary",
        Bson::ObjectId(_) => "objectId",
        Bson::DateTime(_) => "date",
        Bson::Symbol(_) => "symbol",
        Bson::Decimal128(_) => "decimal",
        Bson::Undefined => "undefined",
        Bson::MaxKey => "maxKey",
        Bson::MinKey => "minKey",
        Bson::DbPointer(_) => "dbPointer",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use devbox_core::{
        installer::{MONGODB_SERIES, MONGODB_VERSION},
        MongodbService, ServiceConfig, ServiceKind, ServiceManager,
    };
    use std::collections::BTreeMap;
    use std::thread;

    #[test]
    fn reports_common_bson_types() {
        assert_eq!(bson_type_name(&Bson::String("value".into())), "string");
        assert_eq!(bson_type_name(&Bson::Int32(1)), "int");
        assert_eq!(bson_type_name(&Bson::Array(Vec::new())), "array");
        assert_eq!(bson_type_name(&Bson::Document(Document::new())), "document");
    }

    #[test]
    fn rejects_invalid_names() {
        assert!(validate_name("", "数据库").is_err());
        assert!(validate_name("app\0data", "数据库").is_err());
        assert!(validate_name("app", "数据库").is_ok());
    }

    #[test]
    #[ignore = "requires the local MongoDB service"]
    fn live_mongodb_service_and_commands() {
        let root = dirs::home_dir().unwrap().join(".devbox");
        let instance = root.join("instances/mongodb/default");
        let service = MongodbService::new(ServiceConfig {
            name: "MongoDB".into(),
            kind: ServiceKind::Mongodb,
            version: MONGODB_VERSION.into(),
            port: 27017,
            executable: root.join(format!("installations/mongodb/{MONGODB_SERIES}/bin/mongod")),
            arguments: vec![
                "--config".into(),
                instance.join("conf/mongod.conf").display().to_string(),
            ],
            environment: BTreeMap::new(),
            instance_dir: instance,
            wait_for_port: true,
        })
        .unwrap();
        service.install().unwrap();
        service.start().unwrap();

        let mut ping_result = Err("MongoDB 启动超时".to_string());
        for _ in 0..30 {
            ping_result = run_command("admin", doc! { "ping": 1 });
            if ping_result.is_ok() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        let ping = ping_result.unwrap();
        assert_eq!(ping.get("ok").map(bson_f64), Some(1.0));

        let databases = read_databases().unwrap();
        assert!(databases.iter().any(|database| database.name == "admin"));
        service.stop().unwrap();
    }
}
