import DataFormatTool from "../components/tools/DataFormatTool.vue";
import DuckdbTool from "../components/tools/DuckdbTool.vue";
import JwtTool from "../components/tools/JwtTool.vue";
import PortTool from "../components/tools/PortTool.vue";
import NetworkTool from "../components/tools/NetworkTool.vue";
import ClickHouseTool from "../components/tools/ClickHouseTool.vue";
import DorisTool from "../components/tools/DorisTool.vue";
import SqliteTool from "../components/tools/SqliteTool.vue";
import ClipboardTool from "../components/tools/ClipboardTool.vue";
import S3BrowserTool from "../components/tools/S3BrowserTool.vue";
import MockApiTool from "../components/tools/MockApiTool.vue";
import HttpRequestTool from "../components/tools/HttpRequestTool.vue";
import RealtimeTool from "../components/tools/RealtimeTool.vue";
import TimeTool from "../components/tools/TimeTool.vue";
import RegexTool from "../components/tools/RegexTool.vue";
import CronTool from "../components/tools/CronTool.vue";
import ScheduledTasksTool from "../components/tools/ScheduledTasksTool.vue";
import QrCodeTool from "../components/tools/QrCodeTool.vue";
import SshTool from "../components/tools/SshTool.vue";
import RssTool from "../components/tools/RssTool.vue";
import GoRuntimeTool from "../components/runtimes/GoRuntimeTool.vue";
import JavaRuntimeTool from "../components/runtimes/JavaRuntimeTool.vue";
import RustRuntimeTool from "../components/runtimes/RustRuntimeTool.vue";
import PythonRuntimeTool from "../components/runtimes/PythonRuntimeTool.vue";
import NodeRuntimeTool from "../components/runtimes/NodeRuntimeTool.vue";
import ProjectWorkspaceTool from "../components/tools/ProjectWorkspaceTool.vue";
import EnvironmentTemplatesTool from "../components/tools/EnvironmentTemplatesTool.vue";
import LocalDomainsTool from "../components/tools/LocalDomainsTool.vue";
import DatabaseDevTool from "../components/tools/DatabaseDevTool.vue";
import TestDataTool from "../components/tools/TestDataTool.vue";
import ZeroMqTool from "../components/tools/ZeroMqTool.vue";
import type { ToolDefinition, ToolId } from "./types";

/**
 * 内置工具注册表。新增工具只需在此追加一条，并在 types.ts 的 ToolId 中登记，
 * 侧栏入口与面板挂载都会自动生效，无需改动 App.vue。
 */
export const TOOLS: ToolDefinition[] = [
  {
    id: "go",
    group: "development",
    navLabel: "Go 开发环境",
    navHint: "GOROOT · GOPATH",
    icon: "G",
    component: GoRuntimeTool,
  },
  {
    id: "java",
    group: "development",
    navLabel: "Java 开发环境",
    navHint: "TEMURIN JDK",
    icon: "J",
    component: JavaRuntimeTool,
  },
  {
    id: "rust",
    group: "development",
    navLabel: "Rust 开发环境",
    navHint: "RUSTC · CARGO",
    icon: "R",
    component: RustRuntimeTool,
  },
  {
    id: "python",
    group: "development",
    navLabel: "Python 开发环境",
    navHint: "PYTHON · PIP",
    icon: "Py",
    component: PythonRuntimeTool,
  },
  {
    id: "node",
    group: "development",
    navLabel: "Node.js 开发环境",
    navHint: "NODE · NPM",
    icon: "N",
    component: NodeRuntimeTool,
  },
  {
    id: "workspace",
    group: "development",
    navLabel: "项目工作区",
    navHint: "PROJECT STACK",
    icon: "W",
    component: ProjectWorkspaceTool,
  },
  {
    id: "templates",
    group: "development",
    navLabel: "环境模板",
    navHint: "LOCAL RECIPES",
    icon: "T",
    component: EnvironmentTemplatesTool,
  },
  {
    id: "domains",
    navLabel: "本地域名",
    navHint: "LOCAL GATEWAY",
    icon: "⌁",
    component: LocalDomainsTool,
  },
  {
    id: "ports",
    navLabel: "端口检查器",
    navHint: "TCP LISTEN",
    icon: "↔",
    component: PortTool,
  },
  {
    id: "network",
    navLabel: "网络诊断工具箱",
    navHint: "DNS · TCP · TLS · HTTP",
    icon: "◎",
    component: NetworkTool,
  },
  {
    id: "zeromq",
    navLabel: "ZeroMQ 调试器",
    navHint: "PUB/SUB · PUSH/PULL",
    icon: "Ø",
    component: ZeroMqTool,
  },
  {
    id: "clickhouse",
    navLabel: "ClickHouse 数据库",
    navHint: "LOCAL · REMOTE OLAP",
    icon: "C",
    component: ClickHouseTool,
  },
  {
    id: "doris",
    navLabel: "Apache Doris",
    navHint: "REMOTE OLAP",
    icon: "D",
    component: DorisTool,
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
    id: "dbdev",
    navLabel: "数据库开发辅助",
    navHint: "DATABASE LAB",
    icon: "DB",
    component: DatabaseDevTool,
  },
  {
    id: "testdata",
    navLabel: "测试数据生成器",
    navHint: "DATA FACTORY",
    icon: "D",
    component: TestDataTool,
  },
  {
    id: "realtime",
    navLabel: "WebSocket / SSE",
    navHint: "REALTIME CLIENT",
    icon: "↯",
    component: RealtimeTool,
  },
  {
    id: "time",
    navLabel: "时间与时间戳",
    navHint: "TIME CONVERTER",
    icon: "T",
    component: TimeTool,
  },
  {
    id: "regex",
    navLabel: "正则表达式调试器",
    navHint: "REGEX TESTER",
    icon: ".*",
    component: RegexTool,
  },
  {
    id: "cron",
    navLabel: "Cron 表达式",
    navHint: "SCHEDULE",
    icon: "C",
    component: CronTool,
  },
  {
    id: "tasks",
    navLabel: "定时任务",
    navHint: "LOCAL SCHEDULER",
    icon: "⌁",
    component: ScheduledTasksTool,
  },
  {
    id: "qrcode",
    navLabel: "QR Code 工具",
    navHint: "LOCAL QR",
    icon: "▦",
    component: QrCodeTool,
  },
  {
    id: "ssh",
    navLabel: "SSH 连接管理",
    navHint: "SECURE SHELL",
    icon: ">_",
    component: SshTool,
  },
  {
    id: "dataformat",
    navLabel: "数据格式工具箱",
    navHint: "JSON · CSV · ENCODE",
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
    id: "rss",
    navLabel: "RSS 订阅",
    navHint: "RSS · ATOM",
    icon: "R",
    component: RssTool,
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
