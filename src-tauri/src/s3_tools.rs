use base64::Engine;
use hmac::{Hmac, Mac};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Config {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub bucket: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Bucket {
    pub name: String,
    pub creation_date: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Object {
    pub key: String,
    pub size: u64,
    pub last_modified: String,
    pub etag: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3ObjectContent {
    pub content_type: String,
    pub data: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3PresignedUrl {
    pub url: String,
}

fn now_iso() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // epoch 基准: 1970-01-01 对应 days=719468
    let total_days = (secs / 86400) + 719468;
    let secs_of_day = secs % 86400;
    let h = secs_of_day / 3600;
    let m = (secs_of_day % 3600) / 60;
    let s = secs_of_day % 60;
    // YMD 转换 (proleptic Gregorian)
    let (y, mo, d) = days_to_ymd(total_days as i64);
    format!("{y:04}{mo:02}{d:02}T{h:02}{m:02}{s:02}Z")
}

fn now_date() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let total_days = (secs / 86400) + 719468;
    let (y, mo, d) = days_to_ymd(total_days as i64);
    format!("{y:04}{mo:02}{d:02}")
}

fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    let mut d = days;
    let mut y = (d as f64 / 365.2425) as i64;
    loop {
        let era_start = y * 365 + y / 4 - y / 100 + y / 400;
        if era_start >= d {
            y -= 1;
        } else {
            d -= era_start;
            break;
        }
    }
    let days_in_month = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut mo = 1;
    for &dim in &days_in_month {
        if d <= dim {
            break;
        }
        d -= dim;
        mo += 1;
    }
    (y, mo, d)
}

fn sign(key: &[u8], msg: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(msg.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha256_hex(data: &str) -> String {
    hex(&Sha256::digest(data.as_bytes()))
}

fn sign_v4(
    secret_key: &str,
    date: &str,
    region: &str,
    service: &str,
    string_to_sign: &str,
) -> String {
    let k_date = sign(format!("AWS4{secret_key}").as_bytes(), date);
    let k_region = sign(&k_date, region);
    let k_service = sign(&k_region, service);
    let k_signing = sign(&k_service, "aws4_request");
    hex(&sign(&k_signing, string_to_sign))
}

fn build_canonical_request(
    method: &str,
    uri: &str,
    query: &str,
    headers: &BTreeMap<String, String>,
    signed_headers: &str,
    payload_hash: &str,
) -> String {
    let canonical_headers: String = headers.iter().map(|(k, v)| format!("{k}:{v}\n")).collect();
    format!("{method}\n{uri}\n{query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}")
}

fn build_auth_header(
    access_key: &str,
    date: &str,
    region: &str,
    service: &str,
    signed_headers: &str,
    signature: &str,
) -> String {
    format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{date}/{region}/{service}/aws4_request, \
         SignedHeaders={signed_headers}, Signature={signature}"
    )
}

fn s3_request(
    config: &S3Config,
    method: &str,
    path: &str,
    query: &str,
    body: Option<&[u8]>,
) -> Result<String, String> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let endpoint = config.endpoint.trim_end_matches('/');
    let query_str = if query.is_empty() {
        String::new()
    } else {
        format!("?{query}")
    };
    let url = format!("{endpoint}/{path}{query_str}");
    let payload = body.unwrap_or(b"");
    let payload_hash = sha256_hex(std::str::from_utf8(payload).unwrap_or(""));
    let amz_date = now_iso();
    let date_stamp = now_date();
    let host = endpoint
        .strip_prefix("https://")
        .or_else(|| endpoint.strip_prefix("http://"))
        .unwrap_or(endpoint);

    let mut headers = BTreeMap::new();
    headers.insert("host".into(), host.to_string());
    headers.insert("x-amz-content-sha256".into(), payload_hash.clone());
    headers.insert("x-amz-date".into(), amz_date.clone());
    if !payload.is_empty() {
        headers.insert("content-type".into(), "application/octet-stream".into());
    }

    let signed_headers = "host;x-amz-content-sha256;x-amz-date";
    let canonical_request = build_canonical_request(
        method,
        &format!("/{path}"),
        query,
        &headers,
        signed_headers,
        &payload_hash,
    );
    let scope = format!("{date_stamp}/{}/{}/aws4_request", config.region, "s3");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        scope,
        sha256_hex(&canonical_request)
    );
    let signature = sign_v4(
        &config.secret_key,
        &date_stamp,
        &config.region,
        "s3",
        &string_to_sign,
    );
    let auth = build_auth_header(
        &config.access_key,
        &date_stamp,
        &config.region,
        "s3",
        signed_headers,
        &signature,
    );
    headers.insert("authorization".into(), auth);

    let mut req = match method {
        "GET" => client.get(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        "HEAD" => client.head(&url),
        _ => return Err("不支持的方法".into()),
    };

    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    if !payload.is_empty() {
        req = req.body(payload.to_vec());
    }

    let resp = req.send().map_err(|e| format!("请求失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}: {text}"));
    }
    Ok(text)
}

fn parse_list_buckets(xml: &str) -> Vec<S3Bucket> {
    let mut buckets = Vec::new();
    for cap in xml.split("<Bucket>").skip(1) {
        let name = cap
            .split("<Name>")
            .nth(1)
            .and_then(|s| s.split("</Name>").next())
            .unwrap_or("")
            .to_string();
        let date = cap
            .split("<CreationDate>")
            .nth(1)
            .and_then(|s| s.split("</CreationDate>").next())
            .unwrap_or("")
            .to_string();
        if !name.is_empty() {
            buckets.push(S3Bucket {
                name,
                creation_date: date,
            });
        }
    }
    buckets
}

fn parse_list_objects(xml: &str) -> (Vec<S3Object>, bool) {
    let mut objects = Vec::new();
    for cap in xml.split("<Contents>").skip(1) {
        let key = cap
            .split("<Key>")
            .nth(1)
            .and_then(|s| s.split("</Key>").next())
            .unwrap_or("")
            .to_string();
        let size: u64 = cap
            .split("<Size>")
            .nth(1)
            .and_then(|s| s.split("</Size>").next())
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        let last_modified = cap
            .split("<LastModified>")
            .nth(1)
            .and_then(|s| s.split("</LastModified>").next())
            .unwrap_or("")
            .to_string();
        let etag = cap
            .split("<ETag>")
            .nth(1)
            .and_then(|s| s.split("</ETag>").next())
            .unwrap_or("")
            .trim_matches('"')
            .to_string();
        if !key.is_empty() {
            objects.push(S3Object {
                key,
                size,
                last_modified,
                etag,
            });
        }
    }
    let truncated = xml.contains("<IsTruncated>true</IsTruncated>");
    (objects, truncated)
}

#[tauri::command]
pub async fn s3_list_buckets(config: S3Config) -> Result<Vec<S3Bucket>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let xml = s3_request(&config, "GET", "", "", None)?;
        Ok(parse_list_buckets(&xml))
    })
    .await
    .map_err(|e| format!("S3 任务异常: {e}"))?
}

#[tauri::command]
pub async fn s3_list_objects(
    config: S3Config,
    prefix: Option<String>,
) -> Result<Vec<S3Object>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let query = format!(
            "list-type=2&prefix={}&max-keys=500",
            prefix.as_deref().unwrap_or("")
        );
        let xml = s3_request(&config, "GET", &config.bucket, &query, None)?;
        Ok(parse_list_objects(&xml).0)
    })
    .await
    .map_err(|e| format!("S3 任务异常: {e}"))?
}

#[tauri::command]
pub async fn s3_get_object(config: S3Config, key: String) -> Result<S3ObjectContent, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let xml = s3_request(
            &config,
            "GET",
            &format!("{}/{}", config.bucket, key),
            "",
            None,
        )?;
        // Return as base64 for binary safety; frontend decodes for text preview
        let encoded = base64::engine::general_purpose::STANDARD.encode(xml.as_bytes());
        Ok(S3ObjectContent {
            content_type: "application/octet-stream".into(),
            data: encoded,
            size: xml.len() as u64,
        })
    })
    .await
    .map_err(|e| format!("S3 任务异常: {e}"))?
}

#[tauri::command]
pub async fn s3_put_object(config: S3Config, key: String, data: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        s3_request(
            &config,
            "PUT",
            &format!("{}/{}", config.bucket, key),
            "",
            Some(data.as_bytes()),
        )?;
        Ok(())
    })
    .await
    .map_err(|e| format!("S3 任务异常: {e}"))?
}

#[tauri::command]
pub async fn s3_delete_object(config: S3Config, key: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        s3_request(
            &config,
            "DELETE",
            &format!("{}/{}", config.bucket, key),
            "",
            None,
        )?;
        Ok(())
    })
    .await
    .map_err(|e| format!("S3 任务异常: {e}"))?
}

#[tauri::command]
pub async fn s3_presigned_url(
    config: S3Config,
    key: String,
    expires: Option<u64>,
) -> Result<S3PresignedUrl, String> {
    let expires = expires.unwrap_or(3600);
    let amz_date = now_iso();
    let date_stamp = now_date();
    let host = config
        .endpoint
        .trim_end_matches('/')
        .strip_prefix("https://")
        .or_else(|| config.endpoint.strip_prefix("http://"))
        .unwrap_or(&config.endpoint);
    let credential = format!(
        "{}/{}/{}/s3/aws4_request",
        config.access_key, date_stamp, config.region
    );
    let canonical_uri = format!("/{}/{}", config.bucket, key);
    let query = format!(
        "X-Amz-Algorithm=AWS4-HMAC-SHA256&\
         X-Amz-Credential={}&\
         X-Amz-Date={}&\
         X-Amz-Expires={}&\
         X-Amz-SignedHeaders=host",
        urlencoding(&credential),
        amz_date,
        expires,
    );
    let canonical_request = format!(
        "GET\n{}\n{}\nhost:{}\n\nhost\nUNSIGNED-PAYLOAD",
        canonical_uri, query, host
    );
    let scope = format!("{date_stamp}/{}/{}/aws4_request", config.region, "s3");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        sha256_hex(&canonical_request)
    );
    let signature = sign_v4(
        &config.secret_key,
        &date_stamp,
        &config.region,
        "s3",
        &string_to_sign,
    );
    let endpoint = config.endpoint.trim_end_matches('/');
    let url = format!(
        "{endpoint}{}?{}&X-Amz-Signature={}",
        canonical_uri, query, signature
    );
    Ok(S3PresignedUrl { url })
}

fn urlencoding(s: &str) -> String {
    s.replace('/', "%2F")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now_formats() {
        let iso = now_iso();
        let date = now_date();
        assert_eq!(iso.len(), 16); // YYYYMMDDTHHMMSSZ
        assert_eq!(date.len(), 8); // YYYYMMDD
        assert!(iso.ends_with('Z'));
        assert!(date.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_sign_v4_known_vector() {
        // AWS SigV4 test suite — get-vanilla-query-unreserved
        let secret = "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY";
        let sig = sign_v4(secret, "20150830", "us-east-1", "iam", "AWS4-HMAC-SHA256\n20150830T123600Z\n20150830/us-east-1/iam/aws4_request\nf536975d06c0309214f805bb90ccff089219ecd68b2577efef23edd43b7e1a59");
        assert_eq!(
            sig,
            "5d672d79c15b13162d9279b0855cfba6789a8edb4c82c400e06b5924a6f2b5d7"
        );
    }
}
