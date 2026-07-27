import type { Component, InjectionKey } from "vue";

/**
 * 内置工具的标识。新增工具时在这里追加，并在 registry.ts 中登记。
 * 工具与「托管服务」的区别：工具不下载常驻二进制、不受进程生命周期管理。
 */
export type ToolId = "ports" | "duckdb";

export interface ToolDefinition {
  id: ToolId;
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
