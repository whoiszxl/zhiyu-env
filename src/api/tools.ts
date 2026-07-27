import { invoke } from "@tauri-apps/api/core";
import type {
  DataFormat,
  JsonDiffResult,
  JsonPathResult,
  OutputStyle,
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
