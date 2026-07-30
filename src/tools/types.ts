import type { Component, InjectionKey } from "vue";

/**
 * 内置工具的标识。新增工具时在这里追加，并在 registry.ts 中登记。
 * 工具与开发环境都不属于「托管服务」：它们不参与服务进程生命周期管理。
 * 开发环境可以通过 Runtime Core 下载按版本隔离的 SDK。
 */
export type ToolId =
  | "go"
  | "java"
  | "rust"
  | "python"
  | "node"
  | "workspace"
  | "templates"
  | "domains"
  | "ports"
  | "network"
  | "zeromq"
  | "clickhouse"
  | "doris"
  | "mockapi"
  | "http"
  | "dbdev"
  | "testdata"
  | "realtime"
  | "time"
  | "regex"
  | "cron"
  | "tasks"
  | "qrcode"
  | "ssh"
  | "duckdb"
  | "sqlite"
  | "dataformat"
  | "jwt"
  | "clipboard"
  | "rss"
  | "s3";

export interface ToolDefinition {
  id: ToolId;
  /** 侧栏分组；开发环境与常规工具分开呈现。 */
  group?: "development" | "tools";
  /** 侧栏主标题 */
  navLabel: string;
  /** 侧栏副标题 */
  navHint: string;
  /** 侧栏图标字符，配色取 .nav-icon.<id> */
  icon: string;
  /** 工具面板组件，自行渲染 detail-header 及全部内容 */
  component: Component;
}

/**
 * 安装进度条由 App 统一持有，需要下载资源的工具通过 inject 使用。
 * 纯本地工具（JSON、JWT 等）不需要注入。
 */
export interface InstallTaskApi {
  start(kind: string, title: string): string;
  succeed(operationId: string): void;
  fail(operationId: string, cause: unknown): void;
}

export const INSTALL_TASK_KEY: InjectionKey<InstallTaskApi> =
  Symbol("zhiyu:install-task");
