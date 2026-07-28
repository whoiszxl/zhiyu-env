use base64::Engine;
use hmac::{Hmac, Mac};
use reqwest::blocking::{Client, Response};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;
use time::OffsetDateTime;

type HmacSha1 = Hmac<Sha1>;
type HmacSha256 = Hmac<Sha256>;

const LIST_RESPONSE_LIMIT: usize = 4 * 1024 * 1024;
const PREVIEW_RESPONSE_LIMIT: usize = 3 * 1024 * 1024;
const ERROR_RESPONSE_LIMIT: usize = 256 * 1024;
const DEFAULT_PAGE_SIZE: u16 = 200;
const MAX_UPLOAD_SIZE: u64 = 256 * 1024 * 1024;

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

fn s3_config_path() -> Result<PathBuf, String> {
    let directory = dirs::config_dir()
        .ok_or_else(|| "无法确定应用配置目录".to_string())?
        .join("zhiyu-env");
    fs::create_dir_all(&directory).map_err(|error| format!("创建对象存储配置目录失败: {error}"))?;
    Ok(directory.join("s3-config.json"))
}

#[tauri::command]
pub fn s3_config_get() -> Result<Option<S3Config>, String> {
    let path = s3_config_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let config: S3Config = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("读取对象存储配置失败: {error}"))?,
    )
    .map_err(|error| format!("解析对象存储配置失败: {error}"))?;
    Ok(Some(config))
}

#[tauri::command]
pub fn s3_config_save(config: S3Config) -> Result<(), String> {
    validate_config(&config)?;
    let path = s3_config_path()?;
    let bytes = serde_json::to_vec_pretty(&config)
        .map_err(|error| format!("序列化对象存储配置失败: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes).map_err(|error| format!("写入对象存储配置失败: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("设置对象存储配置权限失败: {error}"))?;
    }
    fs::rename(&temporary, &path).map_err(|error| format!("保存对象存储配置失败: {error}"))?;
    Ok(())
}

fn default_true() -> bool {
    true
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
pub struct S3ListResult {
    pub folders: Vec<String>,
    pub objects: Vec<S3Object>,
    pub next_continuation_token: Option<String>,
    pub truncated: bool,
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

struct S3Response {
    content_type: String,
    body: Vec<u8>,
}

struct RequestTarget {
    url_prefix: String,
    host: String,
    // COS 的签名字符串使用原始 UTF-8 请求路径；实际 HTTP URL 仍使用编码后的路径。
    cos_uri: String,
    canonical_uri: String,
}

fn now_iso() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

fn now_date() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "{:04}{:02}{:02}",
        now.year(),
        u8::from(now.month()),
        now.day()
    )
}

fn now_unix() -> u64 {
    OffsetDateTime::now_utc().unix_timestamp() as u64
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hmac_sha1(key: &[u8], message: &str) -> Vec<u8> {
    let mut mac = HmacSha1::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(message.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn hmac_sha256(key: &[u8], message: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts keys of any length");
    mac.update(message.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

fn sha1_hex(data: &str) -> String {
    hex(&Sha1::digest(data.as_bytes()))
}

fn sha256_hex(data: &str) -> String {
    hex(&Sha256::digest(data.as_bytes()))
}

fn sha256_hex_bytes(data: &[u8]) -> String {
    hex(&Sha256::digest(data))
}

fn percent_encode(value: &str, encode_slash: bool) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_' | b'.' | b'~')
            || (!encode_slash && byte == b'/')
        {
            encoded.push(char::from(byte));
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn canonical_query(pairs: &[(String, String)]) -> String {
    let mut encoded = pairs
        .iter()
        .map(|(key, value)| (percent_encode(key, true), percent_encode(value, true)))
        .collect::<Vec<_>>();
    encoded.sort();
    encoded
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn is_cos_endpoint(host: &str) -> bool {
    host == "myqcloud.com" || host.ends_with(".myqcloud.com")
}

fn requires_virtual_host(host: &str) -> bool {
    is_cos_endpoint(host) || host == "aliyuncs.com" || host.ends_with(".aliyuncs.com")
}

fn validate_config(config: &S3Config) -> Result<reqwest::Url, String> {
    if config.endpoint.trim().is_empty()
        || config.access_key.trim().is_empty()
        || config.secret_key.is_empty()
        || config.region.trim().is_empty()
    {
        return Err("Endpoint、Access Key、Secret Key 和 Region 均不能为空".into());
    }
    let endpoint = reqwest::Url::parse(config.endpoint.trim())
        .map_err(|error| format!("Endpoint 格式无效: {error}"))?;
    if !matches!(endpoint.scheme(), "http" | "https") {
        return Err("Endpoint 只支持 HTTP 或 HTTPS".into());
    }
    if endpoint.path() != "/" || endpoint.query().is_some() || endpoint.fragment().is_some() {
        return Err("Endpoint 只能填写协议和域名，不能包含路径、查询参数或片段".into());
    }
    if endpoint.host_str().is_some_and(is_cos_endpoint) && config.bucket.trim().is_empty() {
        return Err("腾讯云 COS 必须填写包含 APPID 的 Bucket，例如 bucket-name-123456".into());
    }
    Ok(endpoint)
}

fn request_target(
    config: &S3Config,
    endpoint: &reqwest::Url,
    object_key: &str,
) -> Result<RequestTarget, String> {
    let scheme = endpoint.scheme();
    let base_host = endpoint
        .host_str()
        .ok_or_else(|| "Endpoint 缺少主机名".to_string())?;
    let port_suffix = endpoint
        .port()
        .map(|port| format!(":{port}"))
        .unwrap_or_default();
    let base_authority = format!("{base_host}{port_suffix}");
    let path_style = config.path_style && !requires_virtual_host(base_host);

    let (host, raw_uri) = if path_style {
        (base_authority, format!("/{object_key}"))
    } else if config.bucket.is_empty() {
        (base_authority, "/".into())
    } else {
        let host = if base_host.starts_with(&format!("{}.", config.bucket)) {
            base_authority
        } else {
            format!("{}.{base_host}{port_suffix}", config.bucket)
        };
        let key = object_key
            .strip_prefix(&format!("{}/", config.bucket))
            .unwrap_or(object_key);
        (host, format!("/{key}"))
    };
    Ok(RequestTarget {
        url_prefix: format!("{scheme}://{host}"),
        host,
        cos_uri: raw_uri.clone(),
        canonical_uri: percent_encode(&raw_uri, false),
    })
}

fn object_key(config: &S3Config, endpoint: &reqwest::Url, key: &str) -> String {
    let path_style = config.path_style && !endpoint.host_str().is_some_and(requires_virtual_host);
    if path_style {
        format!("{}/{}", config.bucket, key)
    } else {
        key.into()
    }
}

fn cos_authorization(
    config: &S3Config,
    method: &str,
    uri: &str,
    query: &str,
    host: &str,
    start: u64,
    expires: u64,
) -> String {
    let key_time = format!("{start};{}", start.saturating_add(expires));
    let query_keys = query
        .split('&')
        .filter_map(|part| {
            part.split_once('=')
                .map(|(key, _)| key.to_ascii_lowercase())
        })
        .collect::<Vec<_>>()
        .join(";");
    let http_string = format!(
        "{}\n{uri}\n{query}\nhost={}\n",
        method.to_ascii_lowercase(),
        percent_encode(host, true)
    );
    let string_to_sign = format!("sha1\n{key_time}\n{}\n", sha1_hex(&http_string));
    // COS defines SignKey as the lowercase hex string produced by the first
    // HMAC, then uses that string (not the raw 20-byte digest) as the key for
    // the second HMAC.
    let sign_key = hex(&hmac_sha1(config.secret_key.as_bytes(), &key_time));
    let signature = hex(&hmac_sha1(sign_key.as_bytes(), &string_to_sign));
    format!(
        "q-sign-algorithm=sha1&q-ak={}&q-sign-time={key_time}&q-key-time={key_time}&\
         q-header-list=host&q-url-param-list={query_keys}&q-signature={signature}",
        percent_encode(&config.access_key, true),
    )
}

fn aws_authorization(
    config: &S3Config,
    method: &str,
    uri: &str,
    query: &str,
    host: &str,
    payload_hash: &str,
    amz_date: &str,
    date: &str,
) -> String {
    let canonical_headers =
        format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n");
    let signed_headers = "host;x-amz-content-sha256;x-amz-date";
    let canonical_request =
        format!("{method}\n{uri}\n{query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}");
    let scope = format!("{date}/{}/s3/aws4_request", config.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        sha256_hex(&canonical_request)
    );
    let date_key = hmac_sha256(format!("AWS4{}", config.secret_key).as_bytes(), date);
    let region_key = hmac_sha256(&date_key, &config.region);
    let service_key = hmac_sha256(&region_key, "s3");
    let signing_key = hmac_sha256(&service_key, "aws4_request");
    let signature = hex(&hmac_sha256(&signing_key, &string_to_sign));
    format!(
        "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
        config.access_key
    )
}

fn read_response(mut response: Response, limit: usize) -> Result<S3Response, String> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    if status.is_success() {
        if response
            .content_length()
            .is_some_and(|length| length > limit as u64)
        {
            return Err(format!(
                "对象超过预览上限 {} MB，请使用预签名链接流式查看或下载",
                limit / 1024 / 1024
            ));
        }
        let mut body = Vec::new();
        response
            .by_ref()
            .take(limit as u64 + 1)
            .read_to_end(&mut body)
            .map_err(|error| format!("读取对象响应失败: {error}"))?;
        if body.len() > limit {
            return Err(format!(
                "对象超过预览上限 {} MB，请使用预签名链接流式查看或下载",
                limit / 1024 / 1024
            ));
        }
        return Ok(S3Response { content_type, body });
    }

    let mut body = Vec::new();
    response
        .by_ref()
        .take(ERROR_RESPONSE_LIMIT as u64)
        .read_to_end(&mut body)
        .map_err(|error| format!("读取错误响应失败: {error}"))?;
    Err(format!("HTTP {status}: {}", String::from_utf8_lossy(&body)))
}

fn s3_request(
    config: &S3Config,
    method: &str,
    object_key: &str,
    query_pairs: &[(String, String)],
    body: Option<&[u8]>,
    response_limit: usize,
) -> Result<S3Response, String> {
    let endpoint = validate_config(config)?;
    let target = request_target(config, &endpoint, object_key)?;
    let query = canonical_query(query_pairs);
    let url = if query.is_empty() {
        format!("{}{}", target.url_prefix, target.canonical_uri)
    } else {
        format!("{}{}?{query}", target.url_prefix, target.canonical_uri)
    };
    let payload = body.unwrap_or_default();
    let payload_hash = sha256_hex_bytes(payload);
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("创建 HTTP 客户端失败: {error}"))?;
    let mut request = match method {
        "GET" => client.get(&url),
        "PUT" => client.put(&url),
        "DELETE" => client.delete(&url),
        _ => return Err("不支持的对象存储请求方法".into()),
    }
    .header("host", &target.host);

    if endpoint.host_str().is_some_and(is_cos_endpoint) {
        request = request.header(
            "authorization",
            cos_authorization(
                config,
                method,
                &target.cos_uri,
                &query,
                &target.host,
                now_unix(),
                3_600,
            ),
        );
    } else {
        let amz_date = now_iso();
        let date = now_date();
        request = request
            .header("x-amz-content-sha256", &payload_hash)
            .header("x-amz-date", &amz_date)
            .header(
                "authorization",
                aws_authorization(
                    config,
                    method,
                    &target.canonical_uri,
                    &query,
                    &target.host,
                    &payload_hash,
                    &amz_date,
                    &date,
                ),
            );
    }
    if body.is_some() {
        request = request
            .header("content-type", "application/octet-stream")
            .body(payload.to_vec());
    }
    let response = request
        .send()
        .map_err(|error| format!("对象存储请求失败: {error}"))?;
    read_response(response, response_limit)
}

fn xml_value(section: &str, tag: &str) -> Option<String> {
    let start = format!("<{tag}>");
    let end = format!("</{tag}>");
    section
        .split_once(&start)
        .and_then(|(_, rest)| rest.split_once(&end))
        .map(|(value, _)| xml_unescape(value))
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn parse_list_buckets(xml: &str) -> Vec<S3Bucket> {
    xml.split("<Bucket>")
        .skip(1)
        .filter_map(|section| {
            let name = xml_value(section, "Name")?;
            Some(S3Bucket {
                name,
                creation_date: xml_value(section, "CreationDate").unwrap_or_default(),
            })
        })
        .collect()
}

fn parse_list_objects(xml: &str) -> S3ListResult {
    let folders = xml
        .split("<CommonPrefixes>")
        .skip(1)
        .filter_map(|section| xml_value(section, "Prefix"))
        .collect();
    let objects = xml
        .split("<Contents>")
        .skip(1)
        .filter_map(|section| {
            let key = xml_value(section, "Key")?;
            Some(S3Object {
                key,
                size: xml_value(section, "Size")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
                last_modified: xml_value(section, "LastModified").unwrap_or_default(),
                etag: xml_value(section, "ETag")
                    .unwrap_or_default()
                    .trim_matches('"')
                    .to_string(),
            })
        })
        .collect();
    let next_continuation_token = xml_value(xml, "NextContinuationToken");
    S3ListResult {
        folders,
        objects,
        truncated: xml_value(xml, "IsTruncated").as_deref() == Some("true"),
        next_continuation_token,
    }
}

fn response_xml(response: S3Response) -> Result<String, String> {
    String::from_utf8(response.body).map_err(|error| format!("对象存储返回了无效 XML: {error}"))
}

#[tauri::command]
pub async fn s3_list_buckets(config: S3Config) -> Result<Vec<S3Bucket>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let response = s3_request(&config, "GET", "", &[], None, LIST_RESPONSE_LIMIT)?;
        Ok(parse_list_buckets(&response_xml(response)?))
    })
    .await
    .map_err(|error| format!("对象存储任务异常: {error}"))?
}

#[tauri::command]
pub async fn s3_list_objects(
    config: S3Config,
    prefix: Option<String>,
    continuation_token: Option<String>,
    page_size: Option<u16>,
) -> Result<S3ListResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let endpoint = validate_config(&config)?;
        let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, 500);
        let mut query = vec![
            ("list-type".into(), "2".into()),
            ("delimiter".into(), "/".into()),
            ("max-keys".into(), page_size.to_string()),
            ("prefix".into(), prefix.unwrap_or_default()),
        ];
        if let Some(token) = continuation_token.filter(|token| !token.is_empty()) {
            query.push(("continuation-token".into(), token));
        }
        let key = if config.path_style && !endpoint.host_str().is_some_and(requires_virtual_host) {
            config.bucket.clone()
        } else {
            String::new()
        };
        let response = s3_request(&config, "GET", &key, &query, None, LIST_RESPONSE_LIMIT)?;
        Ok(parse_list_objects(&response_xml(response)?))
    })
    .await
    .map_err(|error| format!("对象存储任务异常: {error}"))?
}

#[tauri::command]
pub async fn s3_get_object(config: S3Config, key: String) -> Result<S3ObjectContent, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let endpoint = validate_config(&config)?;
        let key = object_key(&config, &endpoint, &key);
        let response = s3_request(&config, "GET", &key, &[], None, PREVIEW_RESPONSE_LIMIT)?;
        let size = response.body.len() as u64;
        Ok(S3ObjectContent {
            content_type: response.content_type,
            data: base64::engine::general_purpose::STANDARD.encode(response.body),
            size,
        })
    })
    .await
    .map_err(|error| format!("对象存储任务异常: {error}"))?
}

#[tauri::command]
pub async fn s3_put_object(config: S3Config, key: String, data: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let endpoint = validate_config(&config)?;
        let key = object_key(&config, &endpoint, &key);
        s3_request(
            &config,
            "PUT",
            &key,
            &[],
            Some(data.as_bytes()),
            ERROR_RESPONSE_LIMIT,
        )?;
        Ok(())
    })
    .await
    .map_err(|error| format!("对象存储任务异常: {error}"))?
}

#[tauri::command]
pub async fn s3_put_file(config: S3Config, key: String, path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let metadata =
            fs::metadata(&path).map_err(|error| format!("读取待上传文件失败: {error}"))?;
        if !metadata.is_file() {
            return Err("选择的路径不是文件".to_string());
        }
        if metadata.len() > MAX_UPLOAD_SIZE {
            return Err(format!("文件超过 256 MB 上传限制: {}", metadata.len()));
        }
        let data = fs::read(&path).map_err(|error| format!("读取待上传文件失败: {error}"))?;
        let endpoint = validate_config(&config)?;
        let key = object_key(&config, &endpoint, &key);
        s3_request(&config, "PUT", &key, &[], Some(&data), ERROR_RESPONSE_LIMIT)?;
        Ok(())
    })
    .await
    .map_err(|error| format!("对象存储上传任务异常: {error}"))?
}

#[tauri::command]
pub async fn s3_delete_object(config: S3Config, key: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let endpoint = validate_config(&config)?;
        let key = object_key(&config, &endpoint, &key);
        s3_request(&config, "DELETE", &key, &[], None, ERROR_RESPONSE_LIMIT)?;
        Ok(())
    })
    .await
    .map_err(|error| format!("对象存储任务异常: {error}"))?
}

#[tauri::command]
pub async fn s3_presigned_url(
    config: S3Config,
    key: String,
    expires: Option<u64>,
) -> Result<S3PresignedUrl, String> {
    let endpoint = validate_config(&config)?;
    let expires = expires.unwrap_or(3_600).clamp(1, 604_800);
    let key = object_key(&config, &endpoint, &key);
    let target = request_target(&config, &endpoint, &key)?;

    if endpoint.host_str().is_some_and(is_cos_endpoint) {
        let authorization = cos_authorization(
            &config,
            "GET",
            &target.cos_uri,
            "",
            &target.host,
            now_unix(),
            expires,
        );
        return Ok(S3PresignedUrl {
            url: format!(
                "{}{}?{authorization}",
                target.url_prefix, target.canonical_uri
            ),
        });
    }

    let amz_date = now_iso();
    let date = now_date();
    let scope = format!("{date}/{}/s3/aws4_request", config.region);
    let mut query = vec![
        ("X-Amz-Algorithm".into(), "AWS4-HMAC-SHA256".into()),
        (
            "X-Amz-Credential".into(),
            format!("{}/{scope}", config.access_key),
        ),
        ("X-Amz-Date".into(), amz_date.clone()),
        ("X-Amz-Expires".into(), expires.to_string()),
        ("X-Amz-SignedHeaders".into(), "host".into()),
    ];
    let canonical = canonical_query(&query);
    let canonical_request = format!(
        "GET\n{}\n{canonical}\nhost:{}\n\nhost\nUNSIGNED-PAYLOAD",
        target.canonical_uri, target.host
    );
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        sha256_hex(&canonical_request)
    );
    let date_key = hmac_sha256(format!("AWS4{}", config.secret_key).as_bytes(), &date);
    let region_key = hmac_sha256(&date_key, &config.region);
    let service_key = hmac_sha256(&region_key, "s3");
    let signing_key = hmac_sha256(&service_key, "aws4_request");
    query.push((
        "X-Amz-Signature".into(),
        hex(&hmac_sha256(&signing_key, &string_to_sign)),
    ));
    Ok(S3PresignedUrl {
        url: format!(
            "{}{}?{}",
            target.url_prefix,
            target.canonical_uri,
            canonical_query(&query)
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_parameters_are_encoded_and_sorted() {
        let query = vec![
            ("prefix".into(), "音乐/中文 & demo/".into()),
            ("max-keys".into(), "200".into()),
            ("delimiter".into(), "/".into()),
        ];
        assert_eq!(
            canonical_query(&query),
            "delimiter=%2F&max-keys=200&prefix=%E9%9F%B3%E4%B9%90%2F%E4%B8%AD%E6%96%87%20%26%20demo%2F"
        );
    }

    #[test]
    fn list_response_groups_folders_and_keeps_page_token() {
        let xml = r#"<ListBucketResult>
          <IsTruncated>true</IsTruncated>
          <Contents><Key>readme.txt</Key><Size>12</Size><LastModified>2026-07-27</LastModified><ETag>"abc"</ETag></Contents>
          <CommonPrefixes><Prefix>audio/</Prefix></CommonPrefixes>
          <CommonPrefixes><Prefix>视频/</Prefix></CommonPrefixes>
          <NextContinuationToken>next&amp;token</NextContinuationToken>
        </ListBucketResult>"#;
        let result = parse_list_objects(xml);
        assert_eq!(result.folders, ["audio/", "视频/"]);
        assert_eq!(result.objects[0].key, "readme.txt");
        assert_eq!(
            result.next_continuation_token.as_deref(),
            Some("next&token")
        );
        assert!(result.truncated);
    }

    #[test]
    fn cos_signs_query_keys_and_virtual_host_is_detected() {
        let config = S3Config {
            endpoint: "https://cos.ap-guangzhou.myqcloud.com".into(),
            access_key: "id".into(),
            secret_key: "secret".into(),
            region: "ap-guangzhou".into(),
            bucket: "demo-123".into(),
            path_style: true,
        };
        let auth = cos_authorization(
            &config,
            "GET",
            "/",
            "delimiter=%2F&list-type=2&max-keys=200&prefix=",
            "demo-123.cos.ap-guangzhou.myqcloud.com",
            100,
            3_600,
        );
        assert!(auth.contains("q-url-param-list=delimiter;list-type;max-keys;prefix"));
        assert!(auth.contains("q-signature=769db06df8b07e5a74a9125b3ec0e8b8ad85bd33"));
        assert!(is_cos_endpoint("cos.ap-guangzhou.myqcloud.com"));
        assert!(requires_virtual_host("oss-cn-shenzhen.aliyuncs.com"));
    }

    #[test]
    fn cos_format_string_matches_reported_server_hash() {
        let format_string = "get\n/\ndelimiter=%2F&list-type=2&max-keys=200&prefix=\n\
                             host=demo-123.cos.ap-guangzhou.myqcloud.com\n";
        assert_eq!(
            sha1_hex(format_string),
            "007c6f44fe87a10a5487f798e2fe8cc644598dd0"
        );
    }

    #[test]
    fn cos_signing_keeps_raw_unicode_uri_but_encodes_http_uri() {
        let config = S3Config {
            endpoint: "https://cos.ap-guangzhou.myqcloud.com".into(),
            access_key: "id".into(),
            secret_key: "secret".into(),
            region: "ap-guangzhou".into(),
            bucket: "demo-123".into(),
            path_style: false,
        };
        let endpoint = reqwest::Url::parse(&config.endpoint).unwrap();
        let target = request_target(
            &config,
            &endpoint,
            "creative-materials/林一航_-*Java*高级开发工程师简历.html",
        )
        .unwrap();
        assert!(target.cos_uri.contains("林一航"));
        assert!(target.canonical_uri.contains("%E6%9E%97"));
        assert!(target.canonical_uri.contains("%2A"));
    }
}
