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
  SecretEncoding,
  SqliteOverview,
  SqliteQueryResult,
  SqliteTable,
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
