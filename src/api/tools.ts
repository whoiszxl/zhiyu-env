import { invoke } from "@tauri-apps/api/core";
import type {
  DataFormat,
  HmacAlgorithm,
  JsonDiffResult,
  JsonPathResult,
  JwkInspection,
  JwtDecoded,
  JwtVerifyResult,
  OutputStyle,
  S3Bucket,
  S3Config,
  S3ListResult,
  S3ObjectContent,
  S3PresignedUrl,
  SecretEncoding,
  SqliteOverview,
  SqliteQueryResult,
  SqliteTable,
  TokenStatus,
  TransformResult,
} from "../types";

export function transformDataFormat(
  input: string,
  source: DataFormat,
  target: DataFormat,
  style: OutputStyle,
): Promise<TransformResult> {
  return invoke<TransformResult>("data_format_transform", {
    input,
    source,
    target,
    style,
  });
}

export function diffJson(left: string, right: string): Promise<JsonDiffResult> {
  return invoke<JsonDiffResult>("data_json_diff", { left, right });
}

export function queryJsonPath(
  input: string,
  path: string,
): Promise<JsonPathResult> {
  return invoke<JsonPathResult>("data_jsonpath_query", { input, path });
}

export function decodeJwt(token: string): Promise<JwtDecoded> {
  return invoke<JwtDecoded>("jwt_decode", { token });
}

export function verifyJwtHmac(
  token: string,
  secret: string,
  encoding: SecretEncoding,
): Promise<JwtVerifyResult> {
  return invoke<JwtVerifyResult>("jwt_verify_hmac", {
    token,
    secret,
    encoding,
  });
}

export function signJwtHmac(
  payload: string,
  algorithm: HmacAlgorithm,
  secret: string,
  encoding: SecretEncoding,
  keyId: string | null,
): Promise<string> {
  return invoke<string>("jwt_sign_hmac", {
    payload,
    algorithm,
    secret,
    encoding,
    keyId,
  });
}

export function inspectJwk(input: string): Promise<JwkInspection> {
  return invoke<JwkInspection>("jwk_inspect", { input });
}

export function createSqliteDatabase(
  filePath: string,
): Promise<SqliteOverview> {
  return invoke<SqliteOverview>("sqlite_create", { filePath });
}

export function getSqliteOverview(
  filePath: string,
): Promise<SqliteOverview> {
  return invoke<SqliteOverview>("sqlite_overview", { filePath });
}

export function listSqliteTables(filePath: string): Promise<SqliteTable[]> {
  return invoke<SqliteTable[]>("sqlite_tables", { filePath });
}

export function executeSqlite(
  filePath: string,
  sql: string,
  confirmed = false,
): Promise<SqliteQueryResult> {
  return invoke<SqliteQueryResult>("sqlite_execute", {
    filePath,
    sql,
    confirmed,
  });
}

// ── S3 对象存储浏览器 ──────────────────────────────────────

export function s3ConfigGet(): Promise<S3Config | null> {
  return invoke<S3Config | null>("s3_config_get");
}

export function s3ConfigSave(config: S3Config): Promise<void> {
  return invoke<void>("s3_config_save", { config });
}

export function s3ListBuckets(config: S3Config): Promise<S3Bucket[]> {
  return invoke<S3Bucket[]>("s3_list_buckets", { config });
}

export function s3ListObjects(
  config: S3Config,
  prefix?: string,
  continuationToken?: string,
  pageSize = 200,
): Promise<S3ListResult> {
  return invoke<S3ListResult>("s3_list_objects", {
    config,
    prefix,
    continuationToken,
    pageSize,
  });
}

export function s3GetObject(config: S3Config, key: string): Promise<S3ObjectContent> {
  return invoke<S3ObjectContent>("s3_get_object", { config, key });
}

export function s3PutObject(config: S3Config, key: string, data: string): Promise<void> {
  return invoke<void>("s3_put_object", { config, key, data });
}

export function s3PutFile(config: S3Config, key: string, path: string): Promise<void> {
  return invoke<void>("s3_put_file", { config, key, path });
}

export function s3DeleteObject(config: S3Config, key: string): Promise<void> {
  return invoke<void>("s3_delete_object", { config, key });
}

export function s3PresignedUrl(
  config: S3Config,
  key: string,
  expires?: number,
): Promise<S3PresignedUrl> {
  return invoke<S3PresignedUrl>("s3_presigned_url", { config, key, expires });
}
