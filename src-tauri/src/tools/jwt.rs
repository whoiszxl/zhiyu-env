//! JWT 解码、HMAC 签名验证与测试 Token 生成。
//!
//! 所有运算都在本机完成，Token 与密钥不会发往任何网络服务。

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Sha256, Sha384, Sha512};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretEncoding {
    /// 密钥按普通文本处理
    Utf8,
    /// 密钥是 base64 编码后的字节
    Base64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HmacAlgorithm {
    Hs256,
    Hs384,
    Hs512,
}

impl HmacAlgorithm {
    fn name(self) -> &'static str {
        match self {
            Self::Hs256 => "HS256",
            Self::Hs384 => "HS384",
            Self::Hs512 => "HS512",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_uppercase().as_str() {
            "HS256" => Some(Self::Hs256),
            "HS384" => Some(Self::Hs384),
            "HS512" => Some(Self::Hs512),
            _ => None,
        }
    }
}

/// 时间型声明的解读结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeClaim {
    pub name: String,
    pub label: String,
    pub description: String,
    /// Unix 秒，前端负责按本地时区格式化
    pub value: i64,
    /// 相对当前时间的秒数：正数表示未来，负数表示已过去
    pub offset_seconds: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredClaim {
    pub name: String,
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenStatus {
    /// 当前时间落在有效区间内
    Active,
    Expired,
    NotYetValid,
    /// 没有 exp，也没有 nbf
    NoTimeLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JwtDecoded {
    pub header: String,
    pub payload: String,
    /// base64url 原文，仅供查看
    pub signature: String,
    pub algorithm: String,
    pub token_type: Option<String>,
    pub key_id: Option<String>,
    pub time_claims: Vec<TimeClaim>,
    pub registered_claims: Vec<RegisteredClaim>,
    pub status: TokenStatus,
    pub status_detail: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JwtVerifyResult {
    pub valid: bool,
    pub algorithm: String,
    pub detail: String,
}

#[tauri::command]
pub async fn jwt_decode(token: String) -> Result<JwtDecoded, String> {
    tauri::async_runtime::spawn_blocking(move || decode(&token, now_seconds()))
        .await
        .map_err(|error| format!("JWT 解码任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn jwt_verify_hmac(
    token: String,
    secret: String,
    encoding: SecretEncoding,
) -> Result<JwtVerifyResult, String> {
    tauri::async_runtime::spawn_blocking(move || verify_hmac(&token, &secret, encoding))
        .await
        .map_err(|error| format!("JWT 验签任务异常结束: {error}"))?
}

#[tauri::command]
pub async fn jwt_sign_hmac(
    payload: String,
    algorithm: HmacAlgorithm,
    secret: String,
    encoding: SecretEncoding,
    key_id: Option<String>,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        sign_hmac(&payload, algorithm, &secret, encoding, key_id.as_deref())
    })
    .await
    .map_err(|error| format!("JWT 签发任务异常结束: {error}"))?
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0)
}

fn decode(token: &str, now: i64) -> Result<JwtDecoded, String> {
    let (header_value, payload_value, signature) = split_token(token)?;

    let algorithm = header_value
        .get("alg")
        .and_then(Value::as_str)
        .unwrap_or("未声明")
        .to_string();

    let mut warnings = Vec::new();
    if algorithm.eq_ignore_ascii_case("none") {
        warnings.push(
            "该 Token 的算法是 none，表示完全没有签名保护，任何人都能伪造，绝不能在生产环境接受"
                .into(),
        );
    }
    if signature.is_empty() && !algorithm.eq_ignore_ascii_case("none") {
        warnings.push("签名段为空，但头部声明了签名算法，这个 Token 无法通过验签".into());
    }

    let (time_claims, status, status_detail) = read_time_claims(&payload_value, now);
    if matches!(status, TokenStatus::NoTimeLimit) {
        warnings.push("该 Token 没有 exp 过期时间，一旦泄露将长期有效，建议签发时补上".into());
    }

    Ok(JwtDecoded {
        header: pretty(&header_value),
        payload: pretty(&payload_value),
        signature,
        algorithm,
        token_type: header_value
            .get("typ")
            .and_then(Value::as_str)
            .map(str::to_string),
        key_id: header_value
            .get("kid")
            .and_then(Value::as_str)
            .map(str::to_string),
        time_claims,
        registered_claims: read_registered_claims(&payload_value),
        status,
        status_detail,
        warnings,
    })
}

fn split_token(token: &str) -> Result<(Value, Value, String), String> {
    let trimmed = strip_bearer(token.trim());
    if trimmed.is_empty() {
        return Err("请粘贴需要解析的 JWT".into());
    }

    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() != 3 {
        return Err(format!(
            "JWT 应当由 header.payload.signature 三段组成，当前有 {} 段",
            parts.len()
        ));
    }

    let header = decode_segment(parts[0], "头部")?;
    let payload = decode_segment(parts[1], "载荷")?;
    Ok((header, payload, parts[2].to_string()))
}

/// 允许直接粘贴 `Authorization: Bearer xxx` 里的内容。
fn strip_bearer(token: &str) -> &str {
    let without_header = token
        .strip_prefix("Authorization:")
        .map(str::trim)
        .unwrap_or(token);
    without_header
        .strip_prefix("Bearer ")
        .or_else(|| without_header.strip_prefix("bearer "))
        .map(str::trim)
        .unwrap_or(without_header)
}

fn decode_segment(segment: &str, label: &str) -> Result<Value, String> {
    let bytes = URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|error| format!("{label} 不是合法的 base64url：{error}"))?;
    let text =
        String::from_utf8(bytes).map_err(|_| format!("{label} 解码后不是合法的 UTF-8 文本"))?;
    serde_json::from_str(&text).map_err(|error| format!("{label} 不是合法的 JSON：{error}"))
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn read_time_claims(payload: &Value, now: i64) -> (Vec<TimeClaim>, TokenStatus, String) {
    const DEFINITIONS: [(&str, &str, &str); 3] = [
        ("iat", "签发时间", "Token 的生成时刻，用于判断它签发了多久"),
        ("nbf", "生效时间", "在这个时刻之前，Token 应当被拒绝"),
        ("exp", "过期时间", "到达这个时刻后，Token 应当被拒绝"),
    ];

    let mut claims = Vec::new();
    for (name, label, description) in DEFINITIONS {
        let Some(seconds) = payload.get(name).and_then(read_timestamp) else {
            continue;
        };
        claims.push(TimeClaim {
            name: name.to_string(),
            label: label.to_string(),
            description: description.to_string(),
            value: seconds,
            offset_seconds: seconds - now,
        });
    }

    let expiry = payload.get("exp").and_then(read_timestamp);
    let not_before = payload.get("nbf").and_then(read_timestamp);

    let (status, detail) = match (expiry, not_before) {
        (Some(exp), _) if exp <= now => (
            TokenStatus::Expired,
            format!("已过期 {}", humanize(now - exp)),
        ),
        (_, Some(nbf)) if nbf > now => (
            TokenStatus::NotYetValid,
            format!("尚未生效，还需等待 {}", humanize(nbf - now)),
        ),
        (Some(exp), _) => (
            TokenStatus::Active,
            format!("有效，{}后过期", humanize(exp - now)),
        ),
        (None, _) => (
            TokenStatus::NoTimeLimit,
            "该 Token 没有设置过期时间".to_string(),
        ),
    };

    (claims, status, detail)
}

/// JWT 的时间声明按规范是数字，但实践中有实现写成字符串，这里都接受。
fn read_timestamp(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_f64().map(|raw| raw as i64))
        .or_else(|| value.as_str().and_then(|raw| raw.parse::<i64>().ok()))
}

fn humanize(seconds: i64) -> String {
    let seconds = seconds.abs();
    if seconds < 60 {
        return format!("{seconds} 秒");
    }
    if seconds < 3600 {
        return format!("{} 分钟", seconds / 60);
    }
    if seconds < 86_400 {
        return format!("{} 小时", seconds / 3600);
    }
    format!("{} 天", seconds / 86_400)
}

fn read_registered_claims(payload: &Value) -> Vec<RegisteredClaim> {
    const DEFINITIONS: [(&str, &str); 5] = [
        ("iss", "签发方"),
        ("sub", "主体"),
        ("aud", "接收方"),
        ("jti", "Token 编号"),
        ("scope", "授权范围"),
    ];

    DEFINITIONS
        .iter()
        .filter_map(|(name, label)| {
            let value = payload.get(*name)?;
            let rendered = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            Some(RegisteredClaim {
                name: (*name).to_string(),
                label: (*label).to_string(),
                value: rendered,
            })
        })
        .collect()
}

fn verify_hmac(
    token: &str,
    secret: &str,
    encoding: SecretEncoding,
) -> Result<JwtVerifyResult, String> {
    let trimmed = strip_bearer(token.trim());
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() != 3 {
        return Err("JWT 格式不正确，无法验签".into());
    }

    let header: Value = decode_segment(parts[0], "头部")?;
    let algorithm_name = header
        .get("alg")
        .and_then(Value::as_str)
        .ok_or("头部没有声明 alg 算法，无法验签")?;

    let Some(algorithm) = HmacAlgorithm::from_name(algorithm_name) else {
        return Err(format!(
            "当前只支持 HMAC 系列算法（HS256/HS384/HS512），该 Token 使用的是 {algorithm_name}。\
             RS/ES 等非对称算法需要公钥验签，暂未支持"
        ));
    };

    let key = decode_secret(secret, encoding)?;
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let expected = sign_bytes(signing_input.as_bytes(), algorithm, &key);
    let actual = URL_SAFE_NO_PAD
        .decode(parts[2])
        .map_err(|error| format!("签名段不是合法的 base64url：{error}"))?;

    // 逐字节比较长度已由 Vec 相等性覆盖，这里用常量时间比较避免时序泄露
    let valid = constant_time_eq(&expected, &actual);
    Ok(JwtVerifyResult {
        valid,
        algorithm: algorithm.name().to_string(),
        detail: if valid {
            format!("签名有效，使用 {} 校验通过", algorithm.name())
        } else {
            "签名不匹配，密钥不正确或 Token 内容被修改过".to_string()
        },
    })
}

fn decode_secret(secret: &str, encoding: SecretEncoding) -> Result<Vec<u8>, String> {
    if secret.is_empty() {
        return Err("请输入用于验签的密钥".into());
    }
    match encoding {
        SecretEncoding::Utf8 => Ok(secret.as_bytes().to_vec()),
        SecretEncoding::Base64 => base64::engine::general_purpose::STANDARD
            .decode(secret)
            .map_err(|error| format!("密钥不是合法的 base64：{error}")),
    }
}

fn sign_bytes(message: &[u8], algorithm: HmacAlgorithm, key: &[u8]) -> Vec<u8> {
    // HMAC 接受任意长度密钥，new_from_slice 对这三种摘要不会失败
    match algorithm {
        HmacAlgorithm::Hs256 => {
            let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC 接受任意长度密钥");
            mac.update(message);
            mac.finalize().into_bytes().to_vec()
        }
        HmacAlgorithm::Hs384 => {
            let mut mac = Hmac::<Sha384>::new_from_slice(key).expect("HMAC 接受任意长度密钥");
            mac.update(message);
            mac.finalize().into_bytes().to_vec()
        }
        HmacAlgorithm::Hs512 => {
            let mut mac = Hmac::<Sha512>::new_from_slice(key).expect("HMAC 接受任意长度密钥");
            mac.update(message);
            mac.finalize().into_bytes().to_vec()
        }
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn sign_hmac(
    payload: &str,
    algorithm: HmacAlgorithm,
    secret: &str,
    encoding: SecretEncoding,
    key_id: Option<&str>,
) -> Result<String, String> {
    let payload_value: Value = serde_json::from_str(payload.trim())
        .map_err(|error| format!("载荷不是合法的 JSON：{error}"))?;
    if !payload_value.is_object() {
        return Err("载荷必须是一个 JSON 对象".into());
    }

    let mut header = Map::new();
    header.insert("alg".into(), Value::String(algorithm.name().into()));
    header.insert("typ".into(), Value::String("JWT".into()));
    if let Some(kid) = key_id.filter(|value| !value.trim().is_empty()) {
        header.insert("kid".into(), Value::String(kid.trim().into()));
    }

    let key = decode_secret(secret, encoding)?;
    let header_segment = encode_segment(&Value::Object(header))?;
    let payload_segment = encode_segment(&payload_value)?;
    let signing_input = format!("{header_segment}.{payload_segment}");
    let signature = URL_SAFE_NO_PAD.encode(sign_bytes(signing_input.as_bytes(), algorithm, &key));

    Ok(format!("{signing_input}.{signature}"))
}

fn encode_segment(value: &Value) -> Result<String, String> {
    let raw = serde_json::to_vec(value).map_err(|error| format!("序列化失败：{error}"))?;
    Ok(URL_SAFE_NO_PAD.encode(raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// {"alg":"HS256","typ":"JWT"} / {"sub":"1234567890","name":"张三","iat":1700000000}
    /// 密钥为 "zhiyu-secret"，由本模块的 sign_hmac 生成，可自洽验证。
    fn sample_token() -> String {
        sign_hmac(
            r#"{"sub":"1234567890","name":"张三","iat":1700000000,"exp":1700003600}"#,
            HmacAlgorithm::Hs256,
            "zhiyu-secret",
            SecretEncoding::Utf8,
            None,
        )
        .expect("签发应当成功")
    }

    #[test]
    fn decodes_header_and_payload() {
        let decoded = decode(&sample_token(), 1_700_000_100).expect("解码应当成功");

        assert_eq!(decoded.algorithm, "HS256");
        assert_eq!(decoded.token_type.as_deref(), Some("JWT"));
        assert!(decoded.payload.contains("张三"));
        assert!(decoded.header.contains("HS256"));
    }

    #[test]
    fn accepts_token_pasted_with_bearer_prefix() {
        let token = format!("Bearer {}", sample_token());
        let decoded = decode(&token, 1_700_000_100).expect("应当容忍 Bearer 前缀");

        assert_eq!(decoded.algorithm, "HS256");
    }

    #[test]
    fn accepts_full_authorization_header() {
        let token = format!("Authorization: Bearer {}", sample_token());
        let decoded = decode(&token, 1_700_000_100).expect("应当容忍完整请求头");

        assert_eq!(decoded.algorithm, "HS256");
    }

    #[test]
    fn explains_time_claims_in_chinese() {
        let decoded = decode(&sample_token(), 1_700_000_100).expect("解码应当成功");

        let labels: Vec<&str> = decoded
            .time_claims
            .iter()
            .map(|claim| claim.label.as_str())
            .collect();
        assert!(labels.contains(&"签发时间"));
        assert!(labels.contains(&"过期时间"));

        let exp = decoded
            .time_claims
            .iter()
            .find(|claim| claim.name == "exp")
            .expect("应当解析出 exp");
        assert_eq!(exp.value, 1_700_003_600);
        assert_eq!(exp.offset_seconds, 3_500);
    }

    #[test]
    fn reports_active_status_before_expiry() {
        let decoded = decode(&sample_token(), 1_700_000_100).expect("解码应当成功");

        assert_eq!(decoded.status, TokenStatus::Active);
        assert!(
            decoded.status_detail.contains("后过期"),
            "{}",
            decoded.status_detail
        );
    }

    #[test]
    fn reports_expired_status_after_expiry() {
        let decoded = decode(&sample_token(), 1_700_010_000).expect("解码应当成功");

        assert_eq!(decoded.status, TokenStatus::Expired);
        assert!(
            decoded.status_detail.contains("已过期"),
            "{}",
            decoded.status_detail
        );
    }

    #[test]
    fn reports_not_yet_valid_when_nbf_is_in_the_future() {
        let token = sign_hmac(
            r#"{"nbf":1700009000,"exp":1700010000}"#,
            HmacAlgorithm::Hs256,
            "k",
            SecretEncoding::Utf8,
            None,
        )
        .expect("签发应当成功");

        let decoded = decode(&token, 1_700_000_000).expect("解码应当成功");
        assert_eq!(decoded.status, TokenStatus::NotYetValid);
    }

    #[test]
    fn warns_when_token_has_no_expiry() {
        let token = sign_hmac(
            r#"{"sub":"a"}"#,
            HmacAlgorithm::Hs256,
            "k",
            SecretEncoding::Utf8,
            None,
        )
        .expect("签发应当成功");

        let decoded = decode(&token, 1_700_000_000).expect("解码应当成功");
        assert_eq!(decoded.status, TokenStatus::NoTimeLimit);
        assert!(decoded.warnings.iter().any(|w| w.contains("没有 exp")));
    }

    #[test]
    fn warns_about_alg_none_tokens() {
        // 手工构造 alg=none 的 Token
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(br#"{"sub":"attacker"}"#);
        let token = format!("{header}.{payload}.");

        let decoded = decode(&token, 1_700_000_000).expect("解码应当成功");
        assert!(
            decoded.warnings.iter().any(|w| w.contains("none")),
            "应当警告 alg=none：{:?}",
            decoded.warnings
        );
    }

    #[test]
    fn extracts_registered_claims() {
        let token = sign_hmac(
            r#"{"iss":"zhiyu","sub":"u-1","aud":"web","jti":"abc"}"#,
            HmacAlgorithm::Hs256,
            "k",
            SecretEncoding::Utf8,
            None,
        )
        .expect("签发应当成功");

        let decoded = decode(&token, 1_700_000_000).expect("解码应当成功");
        let names: Vec<&str> = decoded
            .registered_claims
            .iter()
            .map(|claim| claim.name.as_str())
            .collect();
        assert_eq!(names, vec!["iss", "sub", "aud", "jti"]);
    }

    #[test]
    fn rejects_malformed_token() {
        let error = decode("not-a-jwt", 0).expect_err("格式错误应当报错");
        assert!(error.contains("三段"), "实际错误：{error}");
    }

    #[test]
    fn verifies_signature_with_correct_secret() {
        let result = verify_hmac(&sample_token(), "zhiyu-secret", SecretEncoding::Utf8)
            .expect("验签应当执行成功");

        assert!(result.valid);
        assert_eq!(result.algorithm, "HS256");
    }

    #[test]
    fn rejects_signature_with_wrong_secret() {
        let result = verify_hmac(&sample_token(), "wrong-secret", SecretEncoding::Utf8)
            .expect("验签应当执行成功");

        assert!(!result.valid);
        assert!(result.detail.contains("不匹配"));
    }

    #[test]
    fn detects_tampered_payload() {
        let token = sample_token();
        let parts: Vec<&str> = token.split('.').collect();
        let tampered_payload = URL_SAFE_NO_PAD.encode(br#"{"sub":"admin"}"#);
        let tampered = format!("{}.{}.{}", parts[0], tampered_payload, parts[2]);

        let result =
            verify_hmac(&tampered, "zhiyu-secret", SecretEncoding::Utf8).expect("验签应当执行成功");
        assert!(!result.valid, "被篡改的载荷必须验签失败");
    }

    #[test]
    fn rejects_unsupported_asymmetric_algorithms() {
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
        let payload = URL_SAFE_NO_PAD.encode(br#"{"sub":"a"}"#);
        let token = format!("{header}.{payload}.sig");

        let error = verify_hmac(&token, "k", SecretEncoding::Utf8).expect_err("应当拒绝 RS256");
        assert!(error.contains("RS256"), "实际错误：{error}");
    }

    #[test]
    fn signs_and_verifies_all_hmac_variants() {
        for algorithm in [
            HmacAlgorithm::Hs256,
            HmacAlgorithm::Hs384,
            HmacAlgorithm::Hs512,
        ] {
            let token = sign_hmac(
                r#"{"sub":"round-trip"}"#,
                algorithm,
                "secret",
                SecretEncoding::Utf8,
                None,
            )
            .expect("签发应当成功");

            let result =
                verify_hmac(&token, "secret", SecretEncoding::Utf8).expect("验签应当执行成功");
            assert!(result.valid, "{} 应当自洽", algorithm.name());
            assert_eq!(result.algorithm, algorithm.name());
        }
    }

    #[test]
    fn supports_base64_encoded_secret() {
        let token = sign_hmac(
            r#"{"sub":"a"}"#,
            HmacAlgorithm::Hs256,
            "c2VjcmV0LWtleQ==",
            SecretEncoding::Base64,
            None,
        )
        .expect("签发应当成功");

        let result = verify_hmac(&token, "c2VjcmV0LWtleQ==", SecretEncoding::Base64)
            .expect("验签应当执行成功");
        assert!(result.valid);

        // 同一份密钥按 UTF-8 解释时字节不同，必然验签失败
        let mismatched = verify_hmac(&token, "c2VjcmV0LWtleQ==", SecretEncoding::Utf8)
            .expect("验签应当执行成功");
        assert!(!mismatched.valid);
    }

    #[test]
    fn includes_key_id_in_generated_header() {
        let token = sign_hmac(
            r#"{"sub":"a"}"#,
            HmacAlgorithm::Hs256,
            "k",
            SecretEncoding::Utf8,
            Some("main-key"),
        )
        .expect("签发应当成功");

        let decoded = decode(&token, 0).expect("解码应当成功");
        assert_eq!(decoded.key_id.as_deref(), Some("main-key"));
    }

    #[test]
    fn rejects_non_object_payload_when_signing() {
        let error = sign_hmac(
            "[1,2]",
            HmacAlgorithm::Hs256,
            "k",
            SecretEncoding::Utf8,
            None,
        )
        .expect_err("数组载荷应当报错");

        assert!(error.contains("JSON 对象"), "实际错误：{error}");
    }

    #[test]
    fn rejects_empty_secret() {
        let error =
            verify_hmac(&sample_token(), "", SecretEncoding::Utf8).expect_err("空密钥应当报错");

        assert!(error.contains("密钥"), "实际错误：{error}");
    }
}
