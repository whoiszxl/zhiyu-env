//! JSONPath 查询（遵循 RFC 9535）。

use serde::Serialize;
use serde_json::Value;
use serde_json_path::JsonPath;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonPathResult {
    /// 每条匹配结果的美化 JSON
    pub matches: Vec<String>,
    pub count: usize,
}

#[tauri::command]
pub async fn data_jsonpath_query(input: String, path: String) -> Result<JsonPathResult, String> {
    tauri::async_runtime::spawn_blocking(move || query(&input, &path))
        .await
        .map_err(|error| format!("JSONPath 查询任务异常结束: {error}"))?
}

fn query(input: &str, path: &str) -> Result<JsonPathResult, String> {
    if input.trim().is_empty() {
        return Err("请输入需要查询的 JSON".into());
    }
    if path.trim().is_empty() {
        return Err("请输入 JSONPath 表达式，例如 $.store.book[0].title".into());
    }

    let value: Value = serde_json::from_str(input).map_err(|error| {
        format!(
            "JSON 解析失败：第 {} 行第 {} 列 {error}",
            error.line(),
            error.column()
        )
    })?;

    let expression =
        JsonPath::parse(path.trim()).map_err(|error| format!("JSONPath 表达式无效：{error}"))?;

    let matches: Vec<String> = expression
        .query(&value)
        .all()
        .into_iter()
        .map(|matched| {
            serde_json::to_string_pretty(matched).unwrap_or_else(|_| matched.to_string())
        })
        .collect();

    Ok(JsonPathResult {
        count: matches.len(),
        matches,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "store": {
            "book": [
                { "title": "入门指南", "price": 39 },
                { "title": "进阶手册", "price": 88 }
            ]
        }
    }"#;

    #[test]
    fn selects_a_single_field() {
        let result = query(SAMPLE, "$.store.book[0].title").expect("查询应当成功");

        assert_eq!(result.count, 1);
        assert_eq!(result.matches[0], "\"入门指南\"");
    }

    #[test]
    fn selects_all_matches_with_wildcard() {
        let result = query(SAMPLE, "$.store.book[*].price").expect("查询应当成功");

        assert_eq!(result.count, 2);
        assert_eq!(result.matches, vec!["39", "88"]);
    }

    #[test]
    fn supports_filter_expressions() {
        let result = query(SAMPLE, "$.store.book[?(@.price > 50)].title").expect("查询应当成功");

        assert_eq!(result.count, 1);
        assert_eq!(result.matches[0], "\"进阶手册\"");
    }

    #[test]
    fn returns_empty_result_when_nothing_matches() {
        let result = query(SAMPLE, "$.store.magazine").expect("查询应当成功");

        assert_eq!(result.count, 0);
        assert!(result.matches.is_empty());
    }

    #[test]
    fn rejects_invalid_expression() {
        let error = query(SAMPLE, "$.store[").expect_err("非法表达式应当报错");

        assert!(error.contains("JSONPath 表达式无效"), "实际错误：{error}");
    }

    #[test]
    fn rejects_invalid_json() {
        let error = query("{", "$.a").expect_err("非法 JSON 应当报错");

        assert!(error.contains("JSON 解析失败"), "实际错误：{error}");
    }

    #[test]
    fn rejects_empty_path() {
        let error = query(SAMPLE, "  ").expect_err("空表达式应当报错");

        assert!(error.contains("JSONPath 表达式"));
    }
}
