//! JSON / YAML / TOML 的格式化、压缩、校验与互转。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JavaScript Number 能精确表示的最大整数（2^53 - 1）。
/// 超过这个范围的整数传到前端会丢精度，需要提示用户。
const JS_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataFormat {
    Json,
    Yaml,
    Toml,
    /// 自动识别：依次尝试 JSON → TOML → YAML
    Auto,
}

impl DataFormat {
    fn label(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Auto => "auto",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputStyle {
    /// 缩进美化
    Pretty,
    /// 去掉所有空白（仅 JSON 有意义）
    Compact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransformResult {
    pub output: String,
    /// 实际识别出的输入格式
    pub detected_format: String,
    /// 非致命提示，例如大整数精度风险
    pub warnings: Vec<String>,
    pub input_bytes: usize,
    pub output_bytes: usize,
}

#[tauri::command]
pub async fn data_format_transform(
    input: String,
    source: DataFormat,
    target: DataFormat,
    style: OutputStyle,
) -> Result<TransformResult, String> {
    tauri::async_runtime::spawn_blocking(move || transform(&input, source, target, style))
        .await
        .map_err(|error| format!("格式转换任务异常结束: {error}"))?
}

fn transform(
    input: &str,
    source: DataFormat,
    target: DataFormat,
    style: OutputStyle,
) -> Result<TransformResult, String> {
    if input.trim().is_empty() {
        return Err("请输入需要处理的内容".into());
    }

    let (value, detected) = parse(input, source)?;
    let output = serialize(&value, target, style)?;

    Ok(TransformResult {
        detected_format: detected.label().to_string(),
        warnings: collect_warnings(&value, target),
        input_bytes: input.len(),
        output_bytes: output.len(),
        output,
    })
}

/// 解析输入，返回解析结果与实际使用的格式。
fn parse(input: &str, source: DataFormat) -> Result<(Value, DataFormat), String> {
    match source {
        DataFormat::Json => parse_json(input).map(|value| (value, DataFormat::Json)),
        DataFormat::Yaml => parse_yaml(input).map(|value| (value, DataFormat::Yaml)),
        DataFormat::Toml => parse_toml(input).map(|value| (value, DataFormat::Toml)),
        DataFormat::Auto => detect_and_parse(input),
    }
}

/// JSON 是 YAML 的子集，所以必须先试 JSON；TOML 的 `a = 1` 会被 YAML
/// 当成普通字符串，所以 TOML 要排在 YAML 之前。
fn detect_and_parse(input: &str) -> Result<(Value, DataFormat), String> {
    if let Ok(value) = parse_json(input) {
        return Ok((value, DataFormat::Json));
    }
    if let Ok(value) = parse_toml(input) {
        return Ok((value, DataFormat::Toml));
    }
    match parse_yaml(input) {
        Ok(value) => Ok((value, DataFormat::Yaml)),
        Err(_) => Err("无法识别输入格式，请确认它是合法的 JSON、YAML 或 TOML".into()),
    }
}

fn parse_json(input: &str) -> Result<Value, String> {
    serde_json::from_str(input)
        .map_err(|error| format!("JSON 解析失败：第 {} 行第 {} 列 {error}", error.line(), error.column()))
}

fn parse_yaml(input: &str) -> Result<Value, String> {
    serde_norway::from_str(input).map_err(|error| format!("YAML 解析失败：{error}"))
}

fn parse_toml(input: &str) -> Result<Value, String> {
    toml::from_str(input).map_err(|error| format!("TOML 解析失败：{error}"))
}

fn serialize(value: &Value, target: DataFormat, style: OutputStyle) -> Result<String, String> {
    match target {
        DataFormat::Json | DataFormat::Auto => match style {
            OutputStyle::Pretty => serde_json::to_string_pretty(value)
                .map_err(|error| format!("JSON 序列化失败：{error}")),
            OutputStyle::Compact => {
                serde_json::to_string(value).map_err(|error| format!("JSON 序列化失败：{error}"))
            }
        },
        DataFormat::Yaml => {
            serde_norway::to_string(value).map_err(|error| format!("YAML 序列化失败：{error}"))
        }
        DataFormat::Toml => serialize_toml(value, style),
    }
}

/// TOML 不能表达顶层数组/标量，也没有 null，这里给出明确的中文提示。
fn serialize_toml(value: &Value, style: OutputStyle) -> Result<String, String> {
    if !value.is_object() {
        return Err("TOML 顶层必须是键值表，当前内容的顶层是数组或标量，无法转换".into());
    }
    if contains_null(value) {
        return Err("TOML 没有 null 类型，请先移除内容中的 null 字段再转换".into());
    }

    let rendered = match style {
        OutputStyle::Pretty => toml::to_string_pretty(value),
        OutputStyle::Compact => toml::to_string(value),
    };
    rendered.map_err(|error| format!("TOML 序列化失败：{error}"))
}

fn contains_null(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Array(items) => items.iter().any(contains_null),
        Value::Object(entries) => entries.values().any(contains_null),
        _ => false,
    }
}

fn collect_warnings(value: &Value, target: DataFormat) -> Vec<String> {
    let mut paths = Vec::new();
    find_unsafe_integers(value, "$", &mut paths);

    let mut warnings = Vec::new();
    if !paths.is_empty() {
        let preview = paths
            .iter()
            .take(5)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("、");
        let suffix = if paths.len() > 5 {
            format!(" 等 {} 处", paths.len())
        } else {
            String::new()
        };
        warnings.push(format!(
            "以下字段的整数超过 JavaScript 安全范围（2^53-1），在前端或浏览器中解析会丢失精度，建议改用字符串传输：{preview}{suffix}"
        ));
    }

    if matches!(target, DataFormat::Toml) {
        warnings.push("TOML 会重排键的顺序并把嵌套对象提升为表，转换结果与原始顺序可能不同".into());
    }
    warnings
}

fn find_unsafe_integers(value: &Value, path: &str, found: &mut Vec<String>) {
    match value {
        Value::Number(number) => {
            let unsafe_integer = number
                .as_i64()
                .is_some_and(|raw| raw.abs() > JS_SAFE_INTEGER)
                || number.as_u64().is_some_and(|raw| raw > JS_SAFE_INTEGER as u64);
            if unsafe_integer {
                found.push(path.to_string());
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                find_unsafe_integers(item, &format!("{path}[{index}]"), found);
            }
        }
        Value::Object(entries) => {
            for (key, entry) in entries {
                find_unsafe_integers(entry, &format!("{path}.{key}"), found);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(input: &str, source: DataFormat, target: DataFormat) -> TransformResult {
        transform(input, source, target, OutputStyle::Pretty).expect("转换应当成功")
    }

    #[test]
    fn formats_json_with_indentation() {
        let result = run(r#"{"b":1,"a":2}"#, DataFormat::Json, DataFormat::Json);

        assert!(result.output.contains("\n  \"b\": 1"));
        // preserve_order 生效，键顺序保持输入顺序
        assert!(result.output.find("\"b\"").unwrap() < result.output.find("\"a\"").unwrap());
    }

    #[test]
    fn compacts_json_by_removing_whitespace() {
        let result = transform(
            "{\n  \"a\": 1\n}",
            DataFormat::Json,
            DataFormat::Json,
            OutputStyle::Compact,
        )
        .expect("压缩应当成功");

        assert_eq!(result.output, r#"{"a":1}"#);
        assert!(result.output_bytes < result.input_bytes);
    }

    #[test]
    fn converts_json_to_yaml_and_back() {
        let yaml = run(r#"{"name":"张三","tags":["a","b"]}"#, DataFormat::Json, DataFormat::Yaml);
        assert!(yaml.output.contains("name: 张三"));

        let back = run(&yaml.output, DataFormat::Yaml, DataFormat::Json);
        let value: Value = serde_json::from_str(&back.output).expect("应当是合法 JSON");
        assert_eq!(value["name"], "张三");
        assert_eq!(value["tags"][1], "b");
    }

    #[test]
    fn converts_json_to_toml() {
        let result = run(r#"{"server":{"host":"127.0.0.1","port":6379}}"#, DataFormat::Json, DataFormat::Toml);

        assert!(result.output.contains("[server]"));
        assert!(result.output.contains("port = 6379"));
    }

    #[test]
    fn detects_format_automatically() {
        assert_eq!(run("{\"a\":1}", DataFormat::Auto, DataFormat::Json).detected_format, "json");
        assert_eq!(run("a = 1\n", DataFormat::Auto, DataFormat::Json).detected_format, "toml");
        assert_eq!(run("a:\n  - 1\n", DataFormat::Auto, DataFormat::Json).detected_format, "yaml");
    }

    #[test]
    fn reports_json_error_with_position() {
        let error = transform("{\"a\": }", DataFormat::Json, DataFormat::Json, OutputStyle::Pretty)
            .expect_err("非法 JSON 应当报错");

        assert!(error.contains("JSON 解析失败"), "实际错误：{error}");
        assert!(error.contains("行"), "错误信息应包含行号：{error}");
    }

    #[test]
    fn warns_about_integers_beyond_javascript_precision() {
        let result = run(r#"{"id":9007199254740993,"small":42}"#, DataFormat::Json, DataFormat::Json);

        assert_eq!(result.warnings.len(), 1);
        assert!(result.warnings[0].contains("$.id"), "实际提示：{}", result.warnings[0]);
        assert!(!result.warnings[0].contains("$.small"));
    }

    #[test]
    fn rejects_toml_output_for_top_level_array() {
        let error = transform("[1,2,3]", DataFormat::Json, DataFormat::Toml, OutputStyle::Pretty)
            .expect_err("顶层数组不能转 TOML");

        assert!(error.contains("顶层必须是键值表"), "实际错误：{error}");
    }

    #[test]
    fn rejects_toml_output_when_null_present() {
        let error = transform(
            r#"{"a":null}"#,
            DataFormat::Json,
            DataFormat::Toml,
            OutputStyle::Pretty,
        )
        .expect_err("含 null 不能转 TOML");

        assert!(error.contains("null"), "实际错误：{error}");
    }

    #[test]
    fn rejects_empty_input() {
        let error = transform("   ", DataFormat::Auto, DataFormat::Json, OutputStyle::Pretty)
            .expect_err("空输入应当报错");

        assert!(error.contains("请输入"));
    }
}
