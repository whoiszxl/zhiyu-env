import { invoke } from "@tauri-apps/api/core";
import type {
  DataFormat,
  CsvDelimiter,
  CsvDirection,
  CsvTransformResult,
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
  MockApiState,
  MockRoute,
  HttpRequestInput,
  HttpResponseOutput,
  QrCodeResult,
  SshCommandResult,
  SshHostKey,
  SshProfile,
  SshTerminalConnection,
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

export function transformCsv(
  input: string,
  direction: CsvDirection,
  delimiter: CsvDelimiter,
): Promise<CsvTransformResult> {
  return invoke<CsvTransformResult>("data_csv_transform", {
    input,
    direction,
    delimiter,
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

// ── 本地 Mock API ─────────────────────────────────────────

export function mockApiState(): Promise<MockApiState> {
  return invoke<MockApiState>("mock_api_state");
}

export function mockApiSaveRoutes(routes: MockRoute[]): Promise<MockApiState> {
  return invoke<MockApiState>("mock_api_save_routes", { routes });
}

export function mockApiStart(port: number, routes: MockRoute[]): Promise<MockApiState> {
  return invoke<MockApiState>("mock_api_start", { port, routes });
}

export function mockApiStop(): Promise<MockApiState> {
  return invoke<MockApiState>("mock_api_stop");
}

export function mockApiClearRequests(): Promise<MockApiState> {
  return invoke<MockApiState>("mock_api_clear_requests");
}

// ── HTTP 请求调试器 ───────────────────────────────────────

export function executeHttpRequest(
  request: HttpRequestInput,
): Promise<HttpResponseOutput> {
  return invoke<HttpResponseOutput>("http_request_execute", { request });
}

export function generateQrCode(
  content: string,
  errorCorrection: string,
  size: number,
): Promise<QrCodeResult> {
  return invoke<QrCodeResult>("qr_code_generate", {
    content,
    errorCorrection,
    size,
  });
}

export function listSshProfiles(): Promise<SshProfile[]> {
  return invoke<SshProfile[]>("ssh_profiles_list");
}

export function saveSshProfile(profile: SshProfile): Promise<SshProfile> {
  return invoke<SshProfile>("ssh_profile_save", { profile });
}

export function deleteSshProfile(id: string): Promise<void> {
  return invoke<void>("ssh_profile_delete", { id });
}

export function previewSshHostKey(profileId: string): Promise<SshHostKey> {
  return invoke<SshHostKey>("ssh_host_key_preview", { profileId });
}

export function trustSshHostKey(
  profileId: string,
  expectedFingerprint: string,
): Promise<SshHostKey> {
  return invoke<SshHostKey>("ssh_host_key_trust", {
    profileId,
    expectedFingerprint,
  });
}

export function testSshConnection(
  profileId: string,
  password?: string,
): Promise<SshCommandResult> {
  return invoke<SshCommandResult>("ssh_connection_test", {
    profileId,
    password: password || null,
  });
}

export function executeSshCommand(
  profileId: string,
  command: string,
  timeoutSeconds: number,
  password?: string,
): Promise<SshCommandResult> {
  return invoke<SshCommandResult>("ssh_command_execute", {
    profileId,
    command,
    timeoutSeconds,
    password: password || null,
  });
}

export function connectSshTerminal(
  sessionId: string,
  profileId: string,
  columns: number,
  rows: number,
): Promise<SshTerminalConnection> {
  return invoke<SshTerminalConnection>("ssh_terminal_connect", {
    sessionId,
    profileId,
    columns,
    rows,
  });
}

export function writeSshTerminal(
  sessionId: string,
  data: string,
): Promise<void> {
  return invoke<void>("ssh_terminal_input", { sessionId, data });
}

export function resizeSshTerminal(
  sessionId: string,
  columns: number,
  rows: number,
): Promise<void> {
  return invoke<void>("ssh_terminal_resize", {
    sessionId,
    columns,
    rows,
  });
}

export function disconnectSshTerminal(sessionId: string): Promise<void> {
  return invoke<void>("ssh_terminal_disconnect", { sessionId });
}
