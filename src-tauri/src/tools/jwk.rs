//! JWK / JWKS 查看：解析密钥集合并给出中文摘要，同时提示私钥泄露风险。

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Serialize;
use serde_json::Value;

/// 出现这些字段说明 JWK 里含有私钥材料，绝不应出现在公开的 JWKS 端点。
const PRIVATE_FIELDS: [&str; 7] = ["d", "p", "q", "dp", "dq", "qi", "k"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JwkKeyInfo {
    pub key_id: Option<String>,
    pub key_type: String,
    pub algorithm: Option<String>,
    pub usage: Option<String>,
    /// 中文摘要，例如「RSA 公钥 · 2048 位」
    pub summary: String,
    pub contains_private_material: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JwkInspection {
    pub keys: Vec<JwkKeyInfo>,
    pub count: usize,
    /// jwks 表示密钥集合，jwk 表示单个密钥
    pub source: String,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub async fn jwk_inspect(input: String) -> Result<JwkInspection, String> {
    tauri::async_runtime::spawn_blocking(move || inspect(&input))
        .await
        .map_err(|error| format!("JWK 解析任务异常结束: {error}"))?
}

fn inspect(input: &str) -> Result<JwkInspection, String> {
    if input.trim().is_empty() {
        return Err("请粘贴 JWK 或 JWKS 的 JSON 内容".into());
    }

    let value: Value =
        serde_json::from_str(input.trim()).map_err(|error| format!("不是合法的 JSON：{error}"))?;

    let (entries, source) = match value.get("keys") {
        Some(Value::Array(items)) => (items.clone(), "jwks"),
        Some(_) => return Err("keys 字段必须是数组".into()),
        None => (vec![value.clone()], "jwk"),
    };

    if entries.is_empty() {
        return Err("密钥集合是空的，keys 数组里没有任何密钥".into());
    }

    let keys: Vec<JwkKeyInfo> = entries.iter().map(describe_key).collect();

    let mut warnings = Vec::new();
    let private_count = keys
        .iter()
        .filter(|key| key.contains_private_material)
        .count();
    if private_count > 0 {
        warnings.push(format!(
            "检测到 {private_count} 个密钥包含私钥材料（d/p/q/k 等字段）。\
             公开的 JWKS 端点只应发布公钥，请确认这份内容没有被对外暴露"
        ));
    }

    let missing_kid = keys.iter().filter(|key| key.key_id.is_none()).count();
    if missing_kid > 0 && keys.len() > 1 {
        warnings.push(format!(
            "有 {missing_kid} 个密钥没有 kid，多密钥场景下验签方将无法确定该用哪一个"
        ));
    }

    Ok(JwkInspection {
        count: keys.len(),
        keys,
        source: source.to_string(),
        warnings,
    })
}

fn describe_key(entry: &Value) -> JwkKeyInfo {
    let key_type = entry
        .get("kty")
        .and_then(Value::as_str)
        .unwrap_or("未知")
        .to_string();

    let contains_private_material = PRIVATE_FIELDS
        .iter()
        .any(|field| entry.get(*field).is_some());

    JwkKeyInfo {
        key_id: entry.get("kid").and_then(Value::as_str).map(str::to_string),
        algorithm: entry.get("alg").and_then(Value::as_str).map(str::to_string),
        usage: entry
            .get("use")
            .and_then(Value::as_str)
            .map(translate_usage),
        summary: summarize(entry, &key_type, contains_private_material),
        key_type,
        contains_private_material,
    }
}

fn translate_usage(usage: &str) -> String {
    match usage {
        "sig" => "签名验签".to_string(),
        "enc" => "加密解密".to_string(),
        other => other.to_string(),
    }
}

fn summarize(entry: &Value, key_type: &str, is_private: bool) -> String {
    let role = if is_private { "私钥" } else { "公钥" };

    match key_type {
        "RSA" => match rsa_modulus_bits(entry) {
            Some(bits) => format!("RSA {role} · {bits} 位"),
            None => format!("RSA {role} · 无法读取模数长度"),
        },
        "EC" => {
            let curve = entry
                .get("crv")
                .and_then(Value::as_str)
                .unwrap_or("未知曲线");
            format!("椭圆曲线 {role} · {curve}")
        }
        "OKP" => {
            let curve = entry
                .get("crv")
                .and_then(Value::as_str)
                .unwrap_or("未知曲线");
            format!("Edwards 曲线 {role} · {curve}")
        }
        "oct" => match symmetric_key_bits(entry) {
            Some(bits) => format!("对称密钥 · {bits} 位（本身就是私密材料）"),
            None => "对称密钥（本身就是私密材料）".to_string(),
        },
        other => format!("{other} 类型 {role}"),
    }
}

/// RSA 模数 n 是 base64url 编码的大端字节串，字节数 × 8 即位长。
fn rsa_modulus_bits(entry: &Value) -> Option<usize> {
    let modulus = entry.get("n").and_then(Value::as_str)?;
    let bytes = URL_SAFE_NO_PAD.decode(modulus).ok()?;
    // 去掉前导零字节，避免把 2048 位算成 2056 位
    let significant = bytes.iter().skip_while(|byte| **byte == 0).count();
    (significant > 0).then_some(significant * 8)
}

fn symmetric_key_bits(entry: &Value) -> Option<usize> {
    let key = entry.get("k").and_then(Value::as_str)?;
    let bytes = URL_SAFE_NO_PAD.decode(key).ok()?;
    (!bytes.is_empty()).then_some(bytes.len() * 8)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个 2048 位的 RSA 公钥 JWK（模数用 256 个非零字节填充）。
    fn rsa_public_jwk() -> String {
        let modulus = URL_SAFE_NO_PAD.encode(vec![0xABu8; 256]);
        format!(
            r#"{{"kty":"RSA","kid":"main","alg":"RS256","use":"sig","n":"{modulus}","e":"AQAB"}}"#
        )
    }

    #[test]
    fn parses_a_single_jwk() {
        let result = inspect(&rsa_public_jwk()).expect("解析应当成功");

        assert_eq!(result.source, "jwk");
        assert_eq!(result.count, 1);
        assert_eq!(result.keys[0].key_id.as_deref(), Some("main"));
        assert_eq!(result.keys[0].algorithm.as_deref(), Some("RS256"));
    }

    #[test]
    fn translates_usage_into_chinese() {
        let result = inspect(&rsa_public_jwk()).expect("解析应当成功");

        assert_eq!(result.keys[0].usage.as_deref(), Some("签名验签"));
    }

    #[test]
    fn reports_rsa_modulus_size() {
        let result = inspect(&rsa_public_jwk()).expect("解析应当成功");

        assert_eq!(result.keys[0].summary, "RSA 公钥 · 2048 位");
    }

    #[test]
    fn ignores_leading_zero_bytes_in_modulus() {
        // 前面补两个零字节，位长仍应算作 2048 而不是 2064
        let mut modulus = vec![0u8, 0u8];
        modulus.extend(vec![0xABu8; 256]);
        let encoded = URL_SAFE_NO_PAD.encode(modulus);
        let jwk = format!(r#"{{"kty":"RSA","n":"{encoded}","e":"AQAB"}}"#);

        let result = inspect(&jwk).expect("解析应当成功");
        assert_eq!(result.keys[0].summary, "RSA 公钥 · 2048 位");
    }

    #[test]
    fn parses_a_jwks_with_multiple_keys() {
        let jwks = format!(
            r#"{{"keys":[{},{{"kty":"EC","kid":"ec-1","crv":"P-256","x":"a","y":"b"}}]}}"#,
            rsa_public_jwk()
        );

        let result = inspect(&jwks).expect("解析应当成功");
        assert_eq!(result.source, "jwks");
        assert_eq!(result.count, 2);
        assert_eq!(result.keys[1].summary, "椭圆曲线 公钥 · P-256");
    }

    #[test]
    fn flags_private_key_material() {
        let jwk = r#"{"kty":"EC","kid":"ec-1","crv":"P-256","x":"a","y":"b","d":"secret"}"#;

        let result = inspect(jwk).expect("解析应当成功");
        assert!(result.keys[0].contains_private_material);
        assert!(result.keys[0].summary.contains("私钥"));
        assert!(
            result.warnings.iter().any(|w| w.contains("私钥材料")),
            "应当警告私钥泄露：{:?}",
            result.warnings
        );
    }

    #[test]
    fn treats_symmetric_keys_as_private() {
        let key = URL_SAFE_NO_PAD.encode(vec![0x11u8; 32]);
        let jwk = format!(r#"{{"kty":"oct","k":"{key}"}}"#);

        let result = inspect(&jwk).expect("解析应当成功");
        assert!(result.keys[0].contains_private_material);
        assert_eq!(
            result.keys[0].summary,
            "对称密钥 · 256 位（本身就是私密材料）"
        );
    }

    #[test]
    fn warns_when_multiple_keys_lack_kid() {
        let jwks = r#"{"keys":[{"kty":"EC","crv":"P-256"},{"kty":"EC","crv":"P-384"}]}"#;

        let result = inspect(jwks).expect("解析应当成功");
        assert!(
            result.warnings.iter().any(|w| w.contains("kid")),
            "应当警告缺少 kid：{:?}",
            result.warnings
        );
    }

    #[test]
    fn does_not_warn_about_missing_kid_for_single_key() {
        let jwk = r#"{"kty":"EC","crv":"P-256"}"#;

        let result = inspect(jwk).expect("解析应当成功");
        assert!(result.warnings.is_empty(), "单密钥不应报 kid 警告");
    }

    #[test]
    fn rejects_empty_key_set() {
        let error = inspect(r#"{"keys":[]}"#).expect_err("空集合应当报错");
        assert!(error.contains("空的"), "实际错误：{error}");
    }

    #[test]
    fn rejects_invalid_json() {
        let error = inspect("{").expect_err("非法 JSON 应当报错");
        assert!(error.contains("不是合法的 JSON"), "实际错误：{error}");
    }
}
