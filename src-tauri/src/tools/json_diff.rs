//! 两份 JSON 的结构化差异比较。

use serde::Serialize;
use serde_json::Value;

/// 单条差异过长时截断，避免把整棵子树塞进界面。
const VALUE_PREVIEW_LIMIT: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffKind {
    /// 右侧新增
    Added,
    /// 右侧缺失
    Removed,
    /// 两侧都有但值不同
    Changed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffEntry {
    pub path: String,
    pub kind: DiffKind,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffResult {
    pub entries: Vec<DiffEntry>,
    pub added: usize,
    pub removed: usize,
    pub changed: usize,
    pub identical: bool,
}

#[tauri::command]
pub async fn data_json_diff(left: String, right: String) -> Result<DiffResult, String> {
    tauri::async_runtime::spawn_blocking(move || diff_documents(&left, &right))
        .await
        .map_err(|error| format!("差异比较任务异常结束: {error}"))?
}

fn diff_documents(left: &str, right: &str) -> Result<DiffResult, String> {
    let left_value = parse_side(left, "左侧")?;
    let right_value = parse_side(right, "右侧")?;

    let mut entries = Vec::new();
    walk(&left_value, &right_value, "$", &mut entries);

    let added = entries.iter().filter(|e| e.kind == DiffKind::Added).count();
    let removed = entries
        .iter()
        .filter(|e| e.kind == DiffKind::Removed)
        .count();
    let changed = entries
        .iter()
        .filter(|e| e.kind == DiffKind::Changed)
        .count();

    Ok(DiffResult {
        identical: entries.is_empty(),
        entries,
        added,
        removed,
        changed,
    })
}

fn parse_side(input: &str, side: &str) -> Result<Value, String> {
    if input.trim().is_empty() {
        return Err(format!("{side} JSON 不能为空"));
    }
    serde_json::from_str(input).map_err(|error| {
        format!(
            "{side} JSON 解析失败：第 {} 行第 {} 列 {error}",
            error.line(),
            error.column()
        )
    })
}

fn walk(left: &Value, right: &Value, path: &str, entries: &mut Vec<DiffEntry>) {
    if left == right {
        return;
    }

    match (left, right) {
        (Value::Object(left_map), Value::Object(right_map)) => {
            for (key, left_child) in left_map {
                let child_path = format!("{path}.{key}");
                match right_map.get(key) {
                    Some(right_child) => walk(left_child, right_child, &child_path, entries),
                    None => entries.push(DiffEntry {
                        path: child_path,
                        kind: DiffKind::Removed,
                        left: Some(preview(left_child)),
                        right: None,
                    }),
                }
            }
            for (key, right_child) in right_map {
                if !left_map.contains_key(key) {
                    entries.push(DiffEntry {
                        path: format!("{path}.{key}"),
                        kind: DiffKind::Added,
                        left: None,
                        right: Some(preview(right_child)),
                    });
                }
            }
        }
        (Value::Array(left_items), Value::Array(right_items)) => {
            let shared = left_items.len().min(right_items.len());
            for index in 0..shared {
                walk(
                    &left_items[index],
                    &right_items[index],
                    &format!("{path}[{index}]"),
                    entries,
                );
            }
            for (offset, item) in left_items.iter().skip(shared).enumerate() {
                entries.push(DiffEntry {
                    path: format!("{path}[{}]", shared + offset),
                    kind: DiffKind::Removed,
                    left: Some(preview(item)),
                    right: None,
                });
            }
            for (offset, item) in right_items.iter().skip(shared).enumerate() {
                entries.push(DiffEntry {
                    path: format!("{path}[{}]", shared + offset),
                    kind: DiffKind::Added,
                    left: None,
                    right: Some(preview(item)),
                });
            }
        }
        _ => entries.push(DiffEntry {
            path: path.to_string(),
            kind: DiffKind::Changed,
            left: Some(preview(left)),
            right: Some(preview(right)),
        }),
    }
}

fn preview(value: &Value) -> String {
    let rendered = match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    if rendered.chars().count() <= VALUE_PREVIEW_LIMIT {
        return rendered;
    }
    let truncated: String = rendered.chars().take(VALUE_PREVIEW_LIMIT).collect();
    format!("{truncated}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diff(left: &str, right: &str) -> DiffResult {
        diff_documents(left, right).expect("比较应当成功")
    }

    #[test]
    fn reports_identical_documents() {
        let result = diff(r#"{"a":1,"b":[1,2]}"#, r#"{"a":1,"b":[1,2]}"#);

        assert!(result.identical);
        assert!(result.entries.is_empty());
    }

    #[test]
    fn ignores_key_order() {
        let result = diff(r#"{"a":1,"b":2}"#, r#"{"b":2,"a":1}"#);

        assert!(result.identical, "键顺序不同不应算作差异");
    }

    #[test]
    fn detects_changed_value_with_path() {
        let result = diff(r#"{"user":{"age":28}}"#, r#"{"user":{"age":30}}"#);

        assert_eq!(result.changed, 1);
        assert_eq!(result.entries[0].path, "$.user.age");
        assert_eq!(result.entries[0].kind, DiffKind::Changed);
        assert_eq!(result.entries[0].left.as_deref(), Some("28"));
        assert_eq!(result.entries[0].right.as_deref(), Some("30"));
    }

    #[test]
    fn detects_added_and_removed_keys() {
        let result = diff(r#"{"keep":1,"gone":2}"#, r#"{"keep":1,"fresh":3}"#);

        assert_eq!(result.removed, 1);
        assert_eq!(result.added, 1);
        let paths: Vec<&str> = result.entries.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"$.gone"));
        assert!(paths.contains(&"$.fresh"));
    }

    #[test]
    fn detects_array_length_difference() {
        let result = diff(r#"{"list":[1,2,3]}"#, r#"{"list":[1,2]}"#);

        assert_eq!(result.removed, 1);
        assert_eq!(result.entries[0].path, "$.list[2]");
    }

    #[test]
    fn detects_type_change_as_changed() {
        let result = diff(r#"{"a":1}"#, r#"{"a":"1"}"#);

        assert_eq!(result.changed, 1);
        assert_eq!(result.entries[0].kind, DiffKind::Changed);
    }

    #[test]
    fn truncates_overly_long_values() {
        let long = "x".repeat(500);
        let result = diff(&format!(r#"{{"a":"{long}"}}"#), r#"{"a":"short"}"#);

        let left = result.entries[0].left.as_ref().expect("应有左值");
        assert!(left.ends_with('…'));
        assert_eq!(left.chars().count(), VALUE_PREVIEW_LIMIT + 1);
    }

    #[test]
    fn rejects_invalid_json_with_side_label() {
        let error = diff_documents("{", r#"{"a":1}"#).expect_err("非法 JSON 应当报错");

        assert!(error.contains("左侧"), "实际错误：{error}");
    }
}
