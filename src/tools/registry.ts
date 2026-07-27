import DuckdbTool from "../components/tools/DuckdbTool.vue";
import PortTool from "../components/tools/PortTool.vue";
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
    id: "duckdb",
    navLabel: "DuckDB 查询器",
    navHint: "LOCAL FILE SQL",
    icon: "D",
    component: DuckdbTool,
  },
];

export function findTool(id: ToolId | null): ToolDefinition | null {
  if (!id) return null;
  return TOOLS.find((tool) => tool.id === id) ?? null;
}
