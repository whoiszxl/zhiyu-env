use base64::Engine;
use hmac::{Hmac, Mac};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::Digest;
use std::collections::BTreeMap;
use time::OffsetDateTime;

type HmacSha1 = Hmac<Sha1>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Config {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub bucket: String,
    #[serde(default = "default_true")]
    pub path_style: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Bucket { pub name: String, pub creation_date: String }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Object { pub key: String, pub size: u64, pub last_modified: String, pub etag: String }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3ObjectContent { pub content_type: String, pub data: String, pub size: u64 }

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct S3PresignedUrl { pub url: String }

fn now_iso() -> String {
    let now = OffsetDateTime::now_utc();
    format!("{:04}{:02}{:02}T{:02}{:02}{:02}Z", now.year(), u8::from(now.month()), now.day(), now.hour(), now.minute(), now.second())
}

fn now_unix() -> u64 {
    OffsetDateTime::now_utc().unix_timestamp() as u64
}

fn obj_key(config: &S3Config, key: &str) -> String {
    if config.path_style { format!("{}/{}", config.bucket, key) } else { key.to_string() }
}

fn sign_sha1(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha1::new_from_slice(key).unwrap();
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

fn sha1_hex(data: &[u8]) -> String {
    let mut h = Sha1::default();
    h.update(data);
    format!("{:x}", h.finalize())
}

fn quote(s: &str) -> String { s.replace('%', "%25") }

/// Tencent COS native signing (q-sign-algorithm=sha1)
fn cos_sign(config: &S3Config, method: &str, uri: &str, query: &str, headers: &BTreeMap<String, String>, header_list: &str) -> String {
    let now = now_unix();
    let expires = now + 3600;
    let key_time = format!("{now};{expires}");
    let sign_key = sign_sha1(config.secret_key.as_bytes(), key_time.as_bytes());

    let header_parts: Vec<String> = header_list.split(';').filter(|h| !h.is_empty()).map(|h| {
        let v = headers.get(h).cloned().unwrap_or_default();
        format!("{}={}", quote(h), quote(&v))
    }).collect();
    let http_headers = header_parts.join("&");

    let http_string = format!("{}\n{}\n{}\n{}\n", method.to_lowercase(), uri, query, http_headers);
    let sha1_http = sha1_hex(http_string.as_bytes());
    let string_to_sign = format!("sha1\n{key_time}\n{sha1_http}\n");

    format!("q-sign-algorithm=sha1&q-ak={}&q-sign-time={}&q-key-time={}&q-header-list={}&q-url-param-list=&q-signature={}",
        config.access_key, key_time, key_time, header_list,
        hex(&sign_sha1(&sign_key, string_to_sign.as_bytes())))
}

fn hex(bytes: &[u8]) -> String { bytes.iter().map(|b| format!("{b:02x}")).collect() }

fn s3_request(
    config: &S3Config, method: &str, object_key: &str, query: &str, body: Option<&[u8]>,
) -> Result<String, String> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(30))
        .build().map_err(|e| e.to_string())?;

    let endpoint = config.endpoint.trim_end_matches('/');
    let bare_host = endpoint.strip_prefix("https://").or_else(|| endpoint.strip_prefix("http://")).unwrap_or(endpoint);

    let (url, host_header, uri) = if config.path_style {
        (format!("{endpoint}/{object_key}"), bare_host.to_string(), format!("/{object_key}"))
    } else if config.bucket.is_empty() {
        (endpoint.to_string(), bare_host.to_string(), "/".to_string())
    } else {
        let bucket_host = format!("{}.{}", config.bucket, bare_host);
        if object_key.is_empty() {
            (format!("https://{bucket_host}/"), bucket_host, "/".to_string())
        } else {
            (format!("https://{bucket_host}/{object_key}"), bucket_host, format!("/{object_key}"))
        }
    };

    let query_str = if query.is_empty() { String::new() } else { format!("?{query}") };
    let url = format!("{url}{query_str}");

    let mut headers = BTreeMap::new();
    headers.insert("host".into(), host_header.clone());
    headers.insert("content-type".into(), "application/octet-stream".into());

    let header_list = "content-type;host";
    let auth = cos_sign(config, method, &uri, query, &headers, header_list);

    let mut req = match method {
        "GET" => client.get(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        _ => return Err("不支持的方法".into()),
    };
    for (k, v) in &headers { req = req.header(k.as_str(), v.as_str()); }
    req = req.header("Authorization", &auth);
    if !body.unwrap_or(b"").is_empty() {
        req = req.body(body.unwrap_or(b"").to_vec());
    }

    let resp = req.send().map_err(|e| format!("请求失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().map_err(|e| e.to_string())?;
    if !status.is_success() { return Err(format!("HTTP {status}: {text}")); }
    Ok(text)
}

fn parse_list_buckets(xml: &str) -> Vec<S3Bucket> {
    let mut b = Vec::new();
    for cap in xml.split("<Bucket>").skip(1) {
        let name = cap.split("<Name>").nth(1).and_then(|s| s.split("</Name>").next()).unwrap_or("").to_string();
        let date = cap.split("<CreationDate>").nth(1).and_then(|s| s.split("</CreationDate>").next()).unwrap_or("").to_string();
        if !name.is_empty() { b.push(S3Bucket { name, creation_date: date }); }
    }
    b
}

fn parse_list_objects(xml: &str) -> Vec<S3Object> {
    let mut o = Vec::new();
    for cap in xml.split("<Contents>").skip(1) {
        let key = cap.split("<Key>").nth(1).and_then(|s| s.split("</Key>").next()).unwrap_or("").to_string();
        let size: u64 = cap.split("<Size>").nth(1).and_then(|s| s.split("</Size>").next()).unwrap_or("0").parse().unwrap_or(0);
        let lm = cap.split("<LastModified>").nth(1).and_then(|s| s.split("</LastModified>").next()).unwrap_or("").to_string();
        let etag = cap.split("<ETag>").nth(1).and_then(|s| s.split("</ETag>").next()).unwrap_or("").trim_matches('"').to_string();
        if !key.is_empty() { o.push(S3Object { key, size, last_modified: lm, etag }); }
    }
    o
}

#[tauri::command] pub async fn s3_list_buckets(config: S3Config) -> Result<Vec<S3Bucket>, String> {
    tauri::async_runtime::spawn_blocking(move || Ok(parse_list_buckets(&s3_request(&config, "GET", "", "", None)?))).await.map_err(|e| format!("S3: {e}"))?
}
#[tauri::command] pub async fn s3_list_objects(config: S3Config, prefix: Option<String>) -> Result<Vec<S3Object>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let q = format!("prefix={}&max-keys=500", prefix.as_deref().unwrap_or(""));
        let k = if config.path_style { config.bucket.clone() } else { String::new() };
        Ok(parse_list_objects(&s3_request(&config, "GET", &k, &q, None)?))
    }).await.map_err(|e| format!("S3: {e}"))?
}
#[tauri::command] pub async fn s3_get_object(config: S3Config, key: String) -> Result<S3ObjectContent, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let k = obj_key(&config, &key);
        let xml = s3_request(&config, "GET", &k, "", None)?;
        Ok(S3ObjectContent { content_type: "application/octet-stream".into(), data: base64::engine::general_purpose::STANDARD.encode(xml.as_bytes()), size: xml.len() as u64 })
    }).await.map_err(|e| format!("S3: {e}"))?
}
#[tauri::command] pub async fn s3_put_object(config: S3Config, key: String, data: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || { s3_request(&config, "PUT", &obj_key(&config, &key), "", Some(data.as_bytes()))?; Ok(()) }).await.map_err(|e| format!("S3: {e}"))?
}
#[tauri::command] pub async fn s3_delete_object(config: S3Config, key: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || { s3_request(&config, "DELETE", &obj_key(&config, &key), "", None)?; Ok(()) }).await.map_err(|e| format!("S3: {e}"))?
}
#[tauri::command] pub async fn s3_presigned_url(config: S3Config, key: String, _expires: Option<u64>) -> Result<S3PresignedUrl, String> {
    Err("暂不支持".into())
}
