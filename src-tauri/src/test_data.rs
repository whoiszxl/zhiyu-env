use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

const MAX_ROWS: u64 = 1_000_000;
const MAX_FIELDS: usize = 100;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDataField {
    name: String,
    kind: String,
    #[serde(default)]
    options: String,
    #[serde(default)]
    nullable_percent: u8,
    #[serde(default)]
    unique: bool,
    #[serde(default)]
    prefix: String,
    #[serde(default)]
    suffix: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDataExportInput {
    seed: String,
    count: u64,
    format: String,
    table_name: String,
    fields: Vec<TestDataField>,
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestDataExportResult {
    path: String,
    rows: u64,
    bytes: u64,
}

#[tauri::command]
pub async fn test_data_export(input: TestDataExportInput) -> Result<TestDataExportResult, String> {
    tauri::async_runtime::spawn_blocking(move || export(input))
        .await
        .map_err(|error| format!("测试数据导出任务异常：{error}"))?
}

fn export(input: TestDataExportInput) -> Result<TestDataExportResult, String> {
    validate_input(&input)?;
    let path = PathBuf::from(&input.path);
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| "导出路径无效".to_string())?;
    if !parent.is_dir() {
        return Err("导出目录不存在".into());
    }
    let temporary = temporary_path(&path);
    let file =
        fs::File::create(&temporary).map_err(|error| format!("创建导出文件失败：{error}"))?;
    let mut writer = BufWriter::new(file);
    let mut random = Random::new(hash_seed(&input.seed));
    let field_names = input
        .fields
        .iter()
        .map(|field| safe_identifier(&field.name))
        .collect::<Vec<_>>();

    let result = match input.format.as_str() {
        "json" => write_json(&mut writer, &input, &mut random),
        "csv" => write_csv(&mut writer, &input, &field_names, &mut random),
        "sql" => write_sql(&mut writer, &input, &field_names, &mut random),
        _ => Err("仅支持 JSON、CSV 与 SQL 格式".into()),
    };
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    writer.flush().map_err(|error| error.to_string())?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|error| error.to_string())?;
    drop(writer);

    if path.exists() {
        fs::remove_file(&path).map_err(|error| format!("覆盖旧文件失败：{error}"))?;
    }
    fs::rename(&temporary, &path).map_err(|error| format!("完成导出失败：{error}"))?;
    let bytes = fs::metadata(&path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    Ok(TestDataExportResult {
        path: path.display().to_string(),
        rows: input.count,
        bytes,
    })
}

fn validate_input(input: &TestDataExportInput) -> Result<(), String> {
    if input.count == 0 || input.count > MAX_ROWS {
        return Err(format!("生成数量必须在 1 到 {MAX_ROWS} 之间"));
    }
    if input.fields.is_empty() || input.fields.len() > MAX_FIELDS {
        return Err(format!("字段数量必须在 1 到 {MAX_FIELDS} 之间"));
    }
    if input.seed.len() > 256 || input.table_name.len() > 128 {
        return Err("种子或表名过长".into());
    }
    let mut names = std::collections::HashSet::new();
    for field in &input.fields {
        if safe_identifier(&field.name).is_empty()
            || field.options.len() > 4096
            || field.prefix.len() > 1024
            || field.suffix.len() > 1024
            || field.nullable_percent > 100
        {
            return Err(format!("字段“{}”配置无效", field.name));
        }
        if !matches!(
            field.kind.as_str(),
            "id" | "name"
                | "email"
                | "phone"
                | "integer"
                | "decimal"
                | "boolean"
                | "date"
                | "uuid"
                | "text"
                | "enum"
        ) {
            return Err(format!("字段“{}”的数据类型不支持", field.name));
        }
        if !names.insert(safe_identifier(&field.name)) {
            return Err(format!("字段名重复：{}", field.name));
        }
    }
    Ok(())
}

fn write_json(
    writer: &mut BufWriter<fs::File>,
    input: &TestDataExportInput,
    random: &mut Random,
) -> Result<(), String> {
    writer
        .write_all(b"[\n")
        .map_err(|error| error.to_string())?;
    for index in 0..input.count {
        if index > 0 {
            writer
                .write_all(b",\n")
                .map_err(|error| error.to_string())?;
        }
        let row = make_row(&input.fields, index, random)
            .into_iter()
            .collect::<serde_json::Map<String, Value>>();
        serde_json::to_writer(&mut *writer, &row).map_err(|error| error.to_string())?;
    }
    writer
        .write_all(b"\n]\n")
        .map_err(|error| error.to_string())
}

fn write_csv(
    writer: &mut BufWriter<fs::File>,
    input: &TestDataExportInput,
    field_names: &[String],
    random: &mut Random,
) -> Result<(), String> {
    writeln!(
        writer,
        "{}",
        field_names
            .iter()
            .map(|name| csv_cell(name))
            .collect::<Vec<_>>()
            .join(",")
    )
    .map_err(|error| error.to_string())?;
    for index in 0..input.count {
        let row = make_row(&input.fields, index, random);
        let line = row
            .iter()
            .map(|(_, value)| csv_cell(&plain_value(value)))
            .collect::<Vec<_>>()
            .join(",");
        writeln!(writer, "{line}").map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn write_sql(
    writer: &mut BufWriter<fs::File>,
    input: &TestDataExportInput,
    field_names: &[String],
    random: &mut Random,
) -> Result<(), String> {
    let table = safe_identifier(&input.table_name);
    for index in 0..input.count {
        let row = make_row(&input.fields, index, random);
        let values = row
            .iter()
            .map(|(_, value)| sql_value(value))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(
            writer,
            "INSERT INTO {table} ({}) VALUES ({values});",
            field_names.join(", ")
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn make_row(fields: &[TestDataField], index: u64, random: &mut Random) -> Vec<(String, Value)> {
    fields
        .iter()
        .map(|field| {
            let value = if field.nullable_percent > 0
                && random.range(1, 100) <= u64::from(field.nullable_percent)
            {
                Value::Null
            } else {
                decorate(field, generated_value(field, index, random))
            };
            (safe_identifier(&field.name), value)
        })
        .collect()
}

fn generated_value(field: &TestDataField, index: u64, random: &mut Random) -> Value {
    const SURNAMES: &[&str] = &[
        "林", "陈", "周", "吴", "徐", "孙", "胡", "朱", "高", "何", "郭", "马",
    ];
    const GIVEN: &[&str] = &[
        "一航", "子墨", "雨桐", "浩然", "思远", "清扬", "若溪", "嘉宁", "云舟", "星野",
    ];
    const WORDS: &[&str] = &[
        "轻量开发环境",
        "本地测试数据",
        "接口联调样本",
        "智屿生成内容",
        "示例业务记录",
    ];
    let unique_tail = if field.unique {
        format!("-{}", index + 1)
    } else {
        String::new()
    };
    match field.kind.as_str() {
        "id" => Value::from(index + 1),
        "name" => Value::from(format!(
            "{}{}{}",
            SURNAMES[random.index(SURNAMES.len())],
            GIVEN[random.index(GIVEN.len())],
            unique_tail
        )),
        "email" => Value::from(format!(
            "dev{}_{}{}@example.com",
            index + 1,
            random.range(100, 999),
            unique_tail
        )),
        "phone" => Value::from(format!(
            "1{}{:09}",
            [3, 5, 6, 7, 8, 9][random.index(6)],
            random.range(0, 999_999_999)
        )),
        "integer" => {
            let (min, max) = parse_range(&field.options, (1, 100));
            Value::from(random.range_i64(min, max))
        }
        "decimal" => {
            let (min, max) = parse_range(&field.options, (0, 1000));
            let basis = ((min as f64 + random.unit() * (max - min) as f64) * 100.0).round() / 100.0;
            Value::from(basis)
        }
        "boolean" => Value::from(random.range(0, 1) == 1),
        "date" => {
            let base = 1_767_225_600_000u64; // 2026-01-01T00:00:00Z
            let millis = base.saturating_sub(random.range(0, 365 * 86_400_000));
            let formatted =
                time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000)
                    .ok()
                    .map(|date| {
                        format!(
                            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
                            date.year(),
                            date.month() as u8,
                            date.day(),
                            date.hour(),
                            date.minute(),
                            date.second(),
                            millis % 1000
                        )
                    })
                    .unwrap_or_default();
            Value::from(formatted)
        }
        "uuid" => Value::from(random.uuid()),
        "enum" => {
            let values = field
                .options
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            Value::from(if values.is_empty() {
                "A".to_string()
            } else {
                format!("{}{}", values[random.index(values.len())], unique_tail)
            })
        }
        "text" => Value::from(if field.options.trim().is_empty() {
            format!("{}{}", WORDS[random.index(WORDS.len())], unique_tail)
        } else {
            format!("{}{}", field.options.trim(), unique_tail)
        }),
        _ => Value::Null,
    }
}

fn decorate(field: &TestDataField, value: Value) -> Value {
    if field.prefix.is_empty() && field.suffix.is_empty() {
        return value;
    }
    let raw = plain_value(&value);
    Value::from(format!("{}{}{}", field.prefix, raw, field.suffix))
}

fn parse_range(value: &str, fallback: (i64, i64)) -> (i64, i64) {
    let numbers = value
        .split([',', '~', '-'])
        .filter_map(|part| part.trim().parse::<i64>().ok())
        .collect::<Vec<_>>();
    if numbers.len() >= 2 {
        (numbers[0].min(numbers[1]), numbers[0].max(numbers[1]))
    } else {
        fallback
    }
}

fn csv_cell(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn plain_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}

fn sql_value(value: &Value) -> String {
    match value {
        Value::Null => "NULL".into(),
        Value::Bool(value) => {
            if *value {
                "TRUE".into()
            } else {
                "FALSE".into()
            }
        }
        Value::Number(value) => value.to_string(),
        Value::String(value) => format!("'{}'", value.replace('\'', "''")),
        other => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}

fn safe_identifier(value: &str) -> String {
    let result = value
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if result.is_empty() {
        "sample_data".into()
    } else {
        result
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("zhiyu-test-data");
    path.with_file_name(format!(".{file_name}.zhiyu.tmp"))
}

fn hash_seed(seed: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in seed.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    if hash == 0 {
        1
    } else {
        hash
    }
}

struct Random {
    state: u64,
}

impl Random {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn range(&mut self, min: u64, max: u64) -> u64 {
        min + (self.unit() * (max.saturating_sub(min) + 1) as f64).floor() as u64
    }

    fn range_i64(&mut self, min: i64, max: i64) -> i64 {
        min + (self.unit() * (max.saturating_sub(min) as f64 + 1.0)).floor() as i64
    }

    fn index(&mut self, length: usize) -> usize {
        (self.unit() * length.max(1) as f64).floor() as usize
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / 9_007_199_254_740_992.0
    }

    fn uuid(&mut self) -> String {
        let mut bytes = [0u8; 16];
        for byte in &mut bytes {
            *byte = self.range(0, 255) as u8;
        }
        bytes[6] = (bytes[6] & 0x0f) | 0x40;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_rows_are_reproducible() {
        let fields = vec![TestDataField {
            name: "email".into(),
            kind: "email".into(),
            options: String::new(),
            nullable_percent: 0,
            unique: true,
            prefix: String::new(),
            suffix: String::new(),
        }];
        let mut first = Random::new(hash_seed("demo"));
        let mut second = Random::new(hash_seed("demo"));
        assert_eq!(
            make_row(&fields, 0, &mut first),
            make_row(&fields, 0, &mut second)
        );
    }

    #[test]
    fn validates_row_limit() {
        let input = TestDataExportInput {
            seed: "demo".into(),
            count: MAX_ROWS + 1,
            format: "json".into(),
            table_name: "users".into(),
            fields: Vec::new(),
            path: "/tmp/test.json".into(),
        };
        assert!(validate_input(&input).is_err());
    }

    #[test]
    fn exports_json_objects_without_buffering_all_rows() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("rows.json");
        let input = TestDataExportInput {
            seed: "demo".into(),
            count: 3,
            format: "json".into(),
            table_name: "users".into(),
            fields: vec![
                TestDataField {
                    name: "id".into(),
                    kind: "id".into(),
                    options: String::new(),
                    nullable_percent: 0,
                    unique: true,
                    prefix: String::new(),
                    suffix: String::new(),
                },
                TestDataField {
                    name: "status".into(),
                    kind: "enum".into(),
                    options: "active,pending".into(),
                    nullable_percent: 0,
                    unique: false,
                    prefix: String::new(),
                    suffix: String::new(),
                },
            ],
            path: path.display().to_string(),
        };
        export(input).unwrap();
        let value: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(value.as_array().unwrap().len(), 3);
        assert_eq!(value[0]["id"], 1);
        assert!(value[0].get("status").is_some());
    }
}
