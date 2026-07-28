import DataFormatTool from "../components/tools/DataFormatTool.vue";
import DuckdbTool from "../components/tools/DuckdbTool.vue";
import JwtTool from "../components/tools/JwtTool.vue";
import PortTool from "../components/tools/PortTool.vue";
import SqliteTool from "../components/tools/SqliteTool.vue";
import ClipboardTool from "../components/tools/ClipboardTool.vue";
import S3BrowserTool from "../components/tools/S3BrowserTool.vue";
import MockApiTool from "../components/tools/MockApiTool.vue";
import HttpRequestTool from "../components/tools/HttpRequestTool.vue";
import type { ToolDefinition, ToolId } from "./types";

/**
 * 内置工具注册表。新增工具只需在此追加一条，并在 types.ts 的 ToolId 中登记，
 * 侧栏入口与面板挂载都会自动生效，无需改动 App.vue。
 */
export const TOOLS: ToolDefinition[] = [
  {
    id: "ports",
    navLabel: "端口检查器",
    navHint: "TCP LISTEN",
    icon: "↔",
    component: PortTool,
  },
  {
    id: "mockapi",
    navLabel: "本地 Mock API",
    navHint: "LOCAL HTTP SERVER",
    icon: "M",
    component: MockApiTool,
  },
  {
    id: "http",
    navLabel: "HTTP 请求调试器",
    navHint: "REST CLIENT",
    icon: "H",
    component: HttpRequestTool,
  },
  {
    id: "dataformat",
    navLabel: "数据格式工具箱",
    navHint: "JSON · YAML · TOML",
    icon: "{ }",
    component: DataFormatTool,
  },
  {
    id: "jwt",
    navLabel: "JWT 调试器",
    navHint: "解码 · 验签 · 签发",
    icon: "J",
    component: JwtTool,
  },
  {
    id: "duckdb",
    navLabel: "DuckDB 查询器",
    navHint: "LOCAL FILE SQL",
    icon: "D",
    component: DuckdbTool,
  },
  {
    id: "sqlite",
    navLabel: "SQLite 数据库",
    navHint: "LOCAL DATABASE",
    icon: "S",
    component: SqliteTool,
  },
  {
    id: "clipboard",
    navLabel: "剪贴板历史",
    navHint: "CLIPBOARD",
    icon: "\u2398",
    component: ClipboardTool,
  },
  {
    id: "s3",
    navLabel: "S3 浏览器",
    navHint: "OBJECT STORAGE",
    icon: "\u2610",
    component: S3BrowserTool,
  },
];

export function findTool(id: ToolId | null): ToolDefinition | null {
  if (!id) return null;
  return TOOLS.find((tool) => tool.id === id) ?? null;
}
