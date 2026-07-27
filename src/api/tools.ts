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
