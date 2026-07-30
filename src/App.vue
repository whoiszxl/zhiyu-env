<script setup lang="ts">
import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { computed, nextTick, onMounted, onUnmounted, provide, ref } from "vue";
import { useI18n } from "vue-i18n";
import ServiceDocs from "./components/ServiceDocs.vue";
import ServiceConnectPanel from "./components/ServiceConnectPanel.vue";
import InfluxdbPanel from "./components/InfluxdbPanel.vue";
import ToastViewport from "./components/ToastViewport.vue";
import AiChatModal from "./components/AiChatModal.vue";
import AiAssistDialog from "./components/AiAssistDialog.vue";
import CommandPalette, {
  type CommandPaletteItem,
} from "./components/CommandPalette.vue";
import SshTool from "./components/tools/SshTool.vue";
import { findTool, TOOLS } from "./tools/registry";
import { INSTALL_TASK_KEY, type ToolId } from "./tools/types";
import { formatBytes } from "./utils/format";
import { setColorTheme, setThemeMode } from "./theme";
import { applyUiScale } from "./display";
import { setAppLocale } from "./i18n";
import { dismissToastByKey, showToast } from "./toast";
import {
  checkAppUpdate,
  cleanAllInstallCache,
  cleanServiceCache,
  cancelInstall,
  addMeilisearchDocuments,
  createServiceBackup,
  executeSql,
  executeRedisCommand,
  executeMongoCommand,
  forceStopService,
  getEnvironmentDiskUsage,
  getEnvironmentMetrics,
  getAiSettings,
  getAppSettings,
  importAppBackground,
  importAiAvatar,
  getMailpitMessageDetail,
  getMailpitOverview,
  getMeilisearchOverview,
  getDatabaseOverview,
  getMongoCollectionDetail,
  getMongoOverview,
  getNatsOverview,
  getKafkaOverview,
  getRedisKeyDetail,
  getRedisOverview,
  getServiceLogs,
  getServiceDiskUsage,
  getServiceMetrics,
  getTableDetail,
  listDatabaseTables,
  listDatabases,
  listMongoCollections,
  listMongoDatabases,
  listMailpitMessages,
  listMeilisearchIndexes,
  listMysqlVersions,
  listPostgresVersions,
  listPortListeners,
  listRedisVersions,
  listServices,
  listServiceBackups,
  listNginxVersions,
  listManagedServiceVersions,
  listKafkaTopics,
  createKafkaTopic,
  deleteKafkaTopic,
  publishKafkaMessage,
  publishNatsMessage,
  readServiceConfig,
  receiveNatsMessage,
  repairAppDiagnostics,
  repairServiceState,
  runAppDiagnostics,
  runServiceAction,
  runtimeProjectsList,
  saveAppSettings,
  saveAiSettings,
  removeAppBackground,
  removeAiAvatar,
  restoreServiceBackup,
  saveServiceConfig,
  scanRedisKeys,
  searchMeilisearch,
  selectRedisVersion,
  selectMysqlVersion,
  selectPostgresVersion,
  selectNginxVersion,
  selectManagedServiceVersion,
  stopAllManagedServices,
  testAiConnection,
  uninstallServiceVersion,
} from "./api/services";
import { databaseTypeInfo } from "./databaseTypeInfo";
import type {
  AiApiProtocol,
  AiAssistOption,
  AiToolCapability,
  AiSettings,
  AiSettingsInput,
  AppSettings,
  AppLocale,
  BackgroundPattern,
  BackgroundPosition,
  BackgroundStyle,
  ColorTheme,
  ProxyMode,
  ThemeMode,
  UiScale,
  DatabaseInfo,
  DatabaseOverview,
  DiagnosticReport,
  EnvironmentMetrics,
  MailDetail,
  MailpitOverview,
  MailSummary,
  MeilisearchIndex,
  MeilisearchOverview,
  MeilisearchSearchResult,
  MongoCollectionDetail,
  MongoCollectionInfo,
  MongoDatabaseInfo,
  MongoOverview,
  NatsMessage,
  NatsOverview,
  NatsPublishResult,
  KafkaOverview,
  KafkaPublishResult,
  KafkaTopic,
  MysqlVersionInfo,
  PostgresVersionInfo,
  NginxVersionInfo,
  ManagedServiceVersionInfo,
  PortListener,
  RedisKeyDetail,
  RedisOverview,
  RedisVersionInfo,
  RuntimeProject,
  ServiceAction,
  ServiceBackup,
  ServiceDiskUsage,
  ServiceInfo,
  ServiceKind,
  ServiceMetrics,
  ServiceState,
  SqlResult,
  SqlServiceKind,
  TableDetail,
  TableInfo,
  UpdateStatus,
  VersionUninstallResult,
} from "./types";

const { t, locale: resolvedLocale } = useI18n();

type DetailTab =
  | "overview"
  | "keys"
  | "site"
  | "console"
  | "data"
  | "sql"
  | "mongoConsole"
  | "mail"
  | "messages"
  | "search"
  | "timeseries"
  | "objectStore"
  | "governance"
  | "broker"
  | "connect"
  | "backup"
  | "config"
  | "logs"
  | "versions"
  | "files"
  | "docs";
type MetricPoint = { cpu: number; memory: number };
type ActivityRecord = {
  id: string;
  kind: ServiceKind;
  service: string;
  action: string;
  success: boolean;
  createdAt: number;
  message: string;
};
type TrayNavigationEvent = {
  target: "overview" | "settings" | "service" | "clipboard";
  kind: ServiceKind | null;
};
type TrayServiceActionEvent = {
  success: boolean;
  message: string;
};
const ACTIVITY_STORAGE_KEY = "zhiyu.environment.activity.v1";
type ConsoleEntry = {
  database: number;
  command: string;
  output: string;
  elapsedMs: number;
  error: boolean;
};
type SqlConsoleEntry = {
  database: string;
  sql: string;
  result: SqlResult | null;
  error: string;
};
type MongoConsoleEntry = {
  database: string;
  command: string;
  output: unknown;
  elapsedMs: number;
  error: string;
};
type InstallTaskStatus = "running" | "completed" | "failed" | "cancelled";
type InstallProgressPayload = {
  operationId: string;
  kind: string;
  percent: number | null;
  stage: string;
  message: string;
  status: InstallTaskStatus;
};
type InstallLogEntry = {
  time: string;
  stage: string;
  message: string;
};
type InstallTask = {
  operationId: string;
  kind: string;
  title: string;
  percent: number;
  stage: string;
  status: InstallTaskStatus;
  logs: InstallLogEntry[];
};
type ManagedVersionInfo =
  | RedisVersionInfo
  | MysqlVersionInfo
  | PostgresVersionInfo
  | NginxVersionInfo
  | ManagedServiceVersionInfo;
type VersionUninstallTarget = {
  kind: ServiceKind;
  serviceName: string;
  release: ManagedVersionInfo;
  fallbackVersion: string | null;
};

const services = ref<ServiceInfo[]>([]);
const selectedKind = ref<ServiceKind>("redis");
const activeTool = ref<ToolId | null>(null);
const sshToolMounted = ref(false);
const dashboardActive = ref(true);
const settingsActive = ref(false);
const aiChatOpen = ref(false);
const commandPaletteOpen = ref(false);
const commandBusyId = ref("");
const commandProjects = ref<RuntimeProject[]>([]);
type SettingsTab =
  | "appearance"
  | "sidebar"
  | "ai"
  | "application"
  | "network"
  | "storage"
  | "about";
const activeSettingsTab = ref<SettingsTab>("appearance");
const settingsTabs: SettingsTab[] = [
  "appearance",
  "sidebar",
  "ai",
  "application",
  "network",
  "storage",
  "about",
];
const appVersion = ref("0.1.0");
const PROJECT_REPOSITORY = "https://github.com/whoiszxl/zhiyu-env";
const PROJECT_ISSUES = `${PROJECT_REPOSITORY}/issues`;
const PROJECT_AUTHOR = "https://github.com/whoiszxl";
const localeOptions: Array<{
  value: AppLocale;
  labelKey: string;
  hintKey: string;
}> = [
  {
    value: "system",
    labelKey: "languageSystem",
    hintKey: "languageSystemHint",
  },
  {
    value: "zh-CN",
    labelKey: "languageChinese",
    hintKey: "languageChineseHint",
  },
  {
    value: "en-US",
    labelKey: "languageEnglish",
    hintKey: "languageEnglishHint",
  },
];
type AiProviderId = "openai" | "anthropic" | "deepseek" | "qwen" | "custom";
type AiProviderPreset = {
  id: Exclude<AiProviderId, "custom">;
  name: string;
  badge: string;
  descriptionKey: string;
  baseUrls: Partial<Record<AiApiProtocol, string>>;
  models: string[];
  keyUrl: string;
};
const aiProviderPresets: AiProviderPreset[] = [
  {
    id: "deepseek",
    name: "DeepSeek",
    badge: "D",
    descriptionKey: "settings.ai.providers.deepseek",
    baseUrls: {
      openai: "https://api.deepseek.com",
      anthropic: "https://api.deepseek.com/anthropic",
    },
    models: ["deepseek-v4-flash", "deepseek-v4-pro"],
    keyUrl: "https://platform.deepseek.com/api_keys",
  },
  {
    id: "openai",
    name: "OpenAI",
    badge: "O",
    descriptionKey: "settings.ai.providers.openai",
    baseUrls: {
      openai: "https://api.openai.com/v1",
    },
    models: [
      "gpt-5-mini",
      "gpt-5",
      "gpt-4.1-mini",
      "gpt-4.1",
      "gpt-4o-mini",
      "gpt-4o",
    ],
    keyUrl: "https://platform.openai.com/api-keys",
  },
  {
    id: "anthropic",
    name: "Anthropic",
    badge: "A",
    descriptionKey: "settings.ai.providers.anthropic",
    baseUrls: {
      anthropic: "https://api.anthropic.com/v1",
    },
    models: [
      "claude-sonnet-5",
      "claude-opus-5",
      "claude-fable-5",
      "claude-sonnet-4-6",
      "claude-opus-4-6",
      "claude-haiku-4-5-20251001",
    ],
    keyUrl: "https://console.anthropic.com/settings/keys",
  },
  {
    id: "qwen",
    name: "通义千问",
    badge: "Q",
    descriptionKey: "settings.ai.providers.qwen",
    baseUrls: {
      openai: "https://dashscope.aliyuncs.com/compatible-mode/v1",
      anthropic: "https://dashscope.aliyuncs.com/apps/anthropic",
    },
    models: [
      "qwen3.7-flash",
      "qwen3.7-plus",
      "qwen3.7-max",
      "qwen3.6-flash",
      "qwen3.6-plus",
      "qwen-long",
      "qwen-mt-plus",
    ],
    keyUrl: "https://help.aliyun.com/zh/model-studio/get-api-key",
  },
];
const aiProviderId = ref<AiProviderId>("openai");
const aiCustomModel = ref(false);
const activeAiProvider = computed(() =>
  aiProviderPresets.find((provider) => provider.id === aiProviderId.value),
);
const aiModelSuggestions = computed(
  () => activeAiProvider.value?.models ?? [],
);
const activeToolDefinition = computed(() => findTool(activeTool.value));
const activeTab = ref<DetailTab>("overview");
const loading = ref(true);
const pendingAction = ref<ServiceAction | null>(null);
const metrics = ref<ServiceMetrics>({
  running: false,
  cpuPercent: null,
  memoryBytes: null,
  uptime: null,
});
const environmentMetrics = ref<EnvironmentMetrics>({
  cpuPercent: 0,
  memoryBytes: 0,
  runningServiceCount: 0,
});
const environmentDiskBytes = ref(0);
const portListeners = ref<PortListener[]>([]);
const activityRecords = ref<ActivityRecord[]>(loadActivityRecords());
const stoppingAll = ref(false);
const repairingServices = ref(false);
const diagnosticsOpen = ref(false);
const diagnosticsRunning = ref(false);
const diagnosticsRepairing = ref(false);
const diagnosticReport = ref<DiagnosticReport | null>(null);
const appSettings = ref<AppSettings>({
  locale: "zh-CN",
  themeMode: "system",
  colorTheme: "classic",
  backgroundPattern: "auto",
  uiScale: 100,
  backgroundImagePath: "",
  backgroundStyle: "off",
  backgroundPosition: "center",
  backgroundOverlay: 58,
  hiddenServices: [],
  serviceOrder: [],
  hiddenTools: [],
  toolOrder: [],
  launchAtLogin: false,
  keepServicesRunningOnClose: true,
  resourceSaverEnabled: false,
  resourceSaverMode: "remind",
  resourceSaverMinutes: 60,
  resourceSaverServices: [],
  proxyMode: "system",
  proxyUrl: "",
  downloadProxyEnabled: true,
  networkProxyEnabled: true,
  downloadMirror: "",
  publicGithubMirror: true,
  downloadConcurrency: 2,
  downloadTimeoutSeconds: 180,
  installRoot: "",
  logRetentionDays: 14,
  backupRetentionCount: 10,
  autoCheckUpdates: true,
  onboardingCompleted: false,
});
const settingsDraft = ref<AppSettings>({ ...appSettings.value });
const aiSettings = ref<AiSettings>({
  enabled: false,
  protocol: "openai",
  baseUrl: "https://api.openai.com/v1",
  model: "",
  timeoutSeconds: 60,
  maxOutputTokens: 2048,
  apiKeyConfigured: false,
  userAvatarPath: "",
  assistantAvatarPath: "",
});
const aiDraft = ref<AiSettingsInput>({
  enabled: false,
  protocol: "openai",
  baseUrl: "https://api.openai.com/v1",
  model: "",
  timeoutSeconds: 60,
  maxOutputTokens: 2048,
  userAvatarPath: "",
  assistantAvatarPath: "",
  apiKey: "",
  clearApiKey: false,
});
const aiSettingsLoading = ref(false);
const aiSettingsSaving = ref(false);
const aiConnectionTesting = ref(false);
const aiAvatarImporting = ref<"user" | "assistant" | null>(null);
const aiTestResult = ref<{
  success: boolean;
  message: string;
  latencyMillis?: number;
} | null>(null);
const aiUserAvatarUrl = computed(() =>
  aiSettings.value.userAvatarPath
    ? convertFileSrc(aiSettings.value.userAvatarPath)
    : "",
);
const aiAssistantAvatarUrl = computed(() =>
  aiSettings.value.assistantAvatarPath
    ? convertFileSrc(aiSettings.value.assistantAvatarPath)
    : "",
);
const uiScaleOptions: Array<{ value: UiScale; label: string }> = [
  { value: 90, label: "小" },
  { value: 100, label: "标准" },
  { value: 110, label: "大" },
  { value: 120, label: "特大" },
];
const colorThemeOptions: Array<{
  value: ColorTheme;
  label: string;
  description: string;
  colors: [string, string, string, string, string];
}> = [
  {
    value: "classic",
    label: "智屿经典",
    description: "品牌墨绿与暖橙",
    colors: ["#20231e", "#292c25", "#edf0e8", "#dd5633", "#6eae7d"],
  },
  {
    value: "ocean",
    label: "深海终端",
    description: "冷静、专注、高对比",
    colors: ["#0d1b2a", "#1b263b", "#e0e1dd", "#5fa8d3", "#778da9"],
  },
  {
    value: "forest",
    label: "松林",
    description: "自然、柔和、耐久看",
    colors: ["#1f2925", "#2f3e46", "#edf3ef", "#84a98c", "#52796f"],
  },
  {
    value: "sand",
    label: "暖沙",
    description: "明亮、温和、低刺激",
    colors: ["#f5ebe0", "#fff9f3", "#302a26", "#a56a43", "#d6ccc2"],
  },
  {
    value: "twilight",
    label: "暮光",
    description: "低饱和紫灰氛围",
    colors: ["#22223b", "#303149", "#f2e9e4", "#b59bac", "#4a4e69"],
  },
  {
    value: "aurora",
    label: "极光青",
    description: "深蓝与清透薄荷",
    colors: ["#0b132b", "#1c2541", "#3a506b", "#5bc0be", "#6fffe9"],
  },
  {
    value: "graphite",
    label: "石墨红",
    description: "冷灰与克制红色",
    colors: ["#2b2d42", "#8d99ae", "#edf2f4", "#ef233c", "#d90429"],
  },
  {
    value: "coral",
    label: "薄荷珊瑚",
    description: "柔和青绿与暖珊瑚",
    colors: ["#006d77", "#83c5be", "#edf6f9", "#ffddd2", "#e29578"],
  },
  {
    value: "sunset",
    label: "落日琥珀",
    description: "海军蓝与金橙",
    colors: ["#003049", "#d62828", "#f77f00", "#fcbf49", "#eae2b7"],
  },
  {
    value: "neon",
    label: "霓虹波普",
    description: "高能撞色与深色底",
    colors: ["#ffbe0b", "#fb5607", "#ff006e", "#8338ec", "#3a86ff"],
  },
  {
    value: "nord",
    label: "北境冰川",
    description: "克制蓝灰与冰川青",
    colors: ["#2e3440", "#3b4252", "#d8dee9", "#88c0d0", "#a3be8c"],
  },
  {
    value: "sakura",
    label: "樱雾",
    description: "柔和梅紫与樱花粉",
    colors: ["#6d597a", "#b56576", "#e56b6f", "#eaac8b", "#fff0f3"],
  },
  {
    value: "coffee",
    label: "深焙咖啡",
    description: "温暖棕褐与奶油色",
    colors: ["#2b2118", "#5e4632", "#a98467", "#dbc1ac", "#f3e9dc"],
  },
  {
    value: "solarized",
    label: "日光终端",
    description: "经典低对比开发配色",
    colors: ["#002b36", "#073642", "#839496", "#2aa198", "#b58900"],
  },
  {
    value: "lavender",
    label: "薰衣草",
    description: "清透紫蓝与柔雾白",
    colors: ["#352f44", "#5c5470", "#b9b4c7", "#faf0e6", "#a594f9"],
  },
];
const backgroundPatternOptions: Array<{
  value: BackgroundPattern;
  label: string;
  description: string;
}> = [
  { value: "auto", label: "跟随主题", description: "自动匹配当前配色" },
  { value: "none", label: "无纹理", description: "纯净、简洁" },
  { value: "grid", label: "方格", description: "开发者网格" },
  { value: "dots", label: "点阵", description: "轻盈、现代" },
  { value: "diagonal", label: "斜线", description: "细腻、利落" },
  { value: "crosshatch", label: "交叉线", description: "细密工程草图" },
  { value: "circuit", label: "电路", description: "节点与线路" },
  { value: "rings", label: "涟漪", description: "柔和同心圆" },
  { value: "paper", label: "横线纸", description: "轻量书写节奏" },
  { value: "checker", label: "棋盘", description: "低对比方块" },
];
const backgroundStyleOptions: Array<{
  value: Exclude<BackgroundStyle, "off">;
  label: string;
  description: string;
}> = [
  { value: "original", label: "原图", description: "保留图片细节与色彩" },
  { value: "frosted", label: "磨砂", description: "轻柔玻璃质感，推荐" },
  { value: "blur", label: "高斯模糊", description: "弱化细节，专注内容" },
  { value: "mist", label: "雾气", description: "低饱和柔雾氛围" },
];
const backgroundPositionOptions: Array<{
  value: BackgroundPosition;
  label: string;
}> = [
  { value: "top", label: "顶部" },
  { value: "center", label: "居中" },
  { value: "bottom", label: "底部" },
];
const backgroundImporting = ref(false);
const visualSettings = computed(() =>
  settingsActive.value ? settingsDraft.value : appSettings.value,
);
function orderByPreference<T>(
  items: readonly T[],
  getId: (item: T) => string,
  order: readonly string[],
): T[] {
  const positions = new Map(order.map((id, index) => [id, index]));
  return items
    .map((item, index) => ({ item, index }))
    .sort((left, right) => {
      const leftPosition = positions.get(getId(left.item));
      const rightPosition = positions.get(getId(right.item));
      if (leftPosition !== undefined && rightPosition !== undefined) {
        return leftPosition - rightPosition;
      }
      if (leftPosition !== undefined) return -1;
      if (rightPosition !== undefined) return 1;
      return left.index - right.index;
    })
    .map(({ item }) => item);
}
const sidebarServices = computed(() => {
  const hidden = new Set(visualSettings.value.hiddenServices);
  return orderByPreference(
    services.value,
    (service) => service.kind,
    visualSettings.value.serviceOrder,
  ).filter((service) => !hidden.has(service.kind));
});
const orderedVisibleTools = computed(() => {
  const hidden = new Set(visualSettings.value.hiddenTools);
  return orderByPreference(
    TOOLS,
    (tool) => tool.id,
    visualSettings.value.toolOrder,
  ).filter((tool) => !hidden.has(tool.id));
});
const sidebarDevelopment = computed(() =>
  orderedVisibleTools.value.filter((tool) => tool.group === "development"),
);
const sidebarTools = computed(() =>
  orderedVisibleTools.value.filter((tool) => tool.group !== "development"),
);
const commandPaletteItems = computed<CommandPaletteItem[]>(() => {
  const navigationGroup = t("commandPalette.groups.navigation");
  const serviceGroup = t("commandPalette.groups.services");
  const toolGroup = t("commandPalette.groups.tools");
  const projectGroup = t("commandPalette.groups.projects");
  const actionGroup = t("commandPalette.groups.actions");
  const items: CommandPaletteItem[] = [
    {
      id: "nav:dashboard",
      label: t("nav.dashboardTitle"),
      hint: t("nav.dashboardHint"),
      group: navigationGroup,
      icon: "⌂",
      keywords: "overview dashboard home 全局 概览 首页",
    },
    {
      id: "nav:settings",
      label: t("settings.title"),
      hint: t("nav.settingsHint"),
      group: navigationGroup,
      icon: "⚙",
      keywords: "settings preference 设置 配置",
    },
  ];
  for (const service of services.value) {
    items.push({
      id: `service:${service.kind}`,
      label: service.name,
      hint: `v${service.version} · ${t(`workspace.state.${service.status}`)}`,
      group: serviceGroup,
      icon: iconLetter[service.kind],
      keywords: `${service.kind} ${service.port}`,
    });
  }
  for (const tool of TOOLS) {
    items.push({
      id: `tool:${tool.id}`,
      label: t(`tools.${tool.id}.label`),
      hint: t(`tools.${tool.id}.hint`),
      group: toolGroup,
      icon: tool.icon,
      keywords: `${tool.navLabel} ${tool.navHint}`,
    });
  }
  for (const project of commandProjects.value) {
    items.push({
      id: `project:${project.id}`,
      label: project.name,
      hint: project.path,
      group: projectGroup,
      icon: project.name.slice(0, 1).toUpperCase(),
      keywords: `${project.description} ${project.services.join(" ")}`,
    });
  }
  for (const service of services.value) {
    if (service.status === "stopped") {
      items.push({
        id: `action:${service.kind}:start`,
        label: t("commandPalette.startService", { service: service.name }),
        hint: `${service.name} · ${service.port}`,
        group: actionGroup,
        icon: "▶",
        keywords: `start 启动 ${service.kind}`,
      });
    } else if (service.status === "running") {
      items.push(
        {
          id: `action:${service.kind}:restart`,
          label: t("commandPalette.restartService", { service: service.name }),
          hint: `${service.name} · PID ${service.pid ?? "—"}`,
          group: actionGroup,
          icon: "↻",
          keywords: `restart 重启 ${service.kind}`,
        },
        {
          id: `action:${service.kind}:stop`,
          label: t("commandPalette.stopService", { service: service.name }),
          hint: `${service.name} · PID ${service.pid ?? "—"}`,
          group: actionGroup,
          icon: "■",
          keywords: `stop 停止 ${service.kind}`,
          danger: true,
        },
      );
    }
  }
  return items;
});
const settingsOrderedServices = computed(() =>
  orderByPreference(
    services.value,
    (service) => service.kind,
    settingsDraft.value.serviceOrder,
  ),
);
const settingsOrderedTools = computed(() =>
  orderByPreference(
    TOOLS,
    (tool) => tool.id,
    settingsDraft.value.toolOrder,
  ),
);
const draggingSidebarItem = ref<{
  group: "services" | "tools";
  id: string;
} | null>(null);
const sidebarDropTarget = ref("");
const sidebarPointerDrop = ref<{
  group: "services" | "tools";
  id: string;
  after: boolean;
} | null>(null);
const hasCustomBackground = computed(
  () =>
    Boolean(visualSettings.value.backgroundImagePath) &&
    visualSettings.value.backgroundStyle !== "off",
);
const resolvedBackgroundPattern = computed<
  Exclude<BackgroundPattern, "auto">
>(() => {
  const pattern = visualSettings.value.backgroundPattern;
  if (pattern !== "auto") return pattern;
  if (hasCustomBackground.value) return "none";
  const defaults: Record<ColorTheme, Exclude<BackgroundPattern, "auto">> = {
    classic: "grid",
    ocean: "grid",
    forest: "dots",
    sand: "none",
    twilight: "dots",
    aurora: "dots",
    graphite: "grid",
    coral: "dots",
    sunset: "diagonal",
    neon: "grid",
    nord: "circuit",
    sakura: "rings",
    coffee: "paper",
    solarized: "crosshatch",
    lavender: "checker",
  };
  return defaults[visualSettings.value.colorTheme];
});
const backgroundImageUrl = computed(() =>
  visualSettings.value.backgroundImagePath
    ? convertFileSrc(visualSettings.value.backgroundImagePath)
    : "",
);
const backgroundShellStyle = computed<Record<string, string>>(() => ({
  "--app-background-image": backgroundImageUrl.value
    ? `url("${backgroundImageUrl.value}")`
    : "none",
  "--app-background-position": visualSettings.value.backgroundPosition,
  "--app-background-overlay": String(
    visualSettings.value.backgroundOverlay / 100,
  ),
}));
const settingsSaving = ref(false);
let settingsSaveQueued = false;
const allCacheCleaning = ref(false);
const updateChecking = ref(false);
const updateStatus = ref<UpdateStatus | null>(null);
const onboardingOpen = ref(false);
const onboardingStep = ref(0);

function applyAppTheme(
  mode: ThemeMode,
  palette: ColorTheme,
  persist = true,
) {
  setColorTheme(palette, persist);
  setThemeMode(mode, persist);
  void getCurrentWindow()
    .setTheme(mode === "system" ? null : mode)
    .catch(() => {
      // 浏览器预览环境没有原生窗口时，仅应用 Web 主题。
    });
}
const diskUsageByKind = ref<
  Partial<Record<ServiceKind, ServiceDiskUsage>>
>({});
const metricHistory = ref<MetricPoint[]>([]);
const cacheCleaning = ref(false);
const backups = ref<ServiceBackup[]>([]);
const backupLoading = ref(false);
const backupCreating = ref(false);
const restoringBackupId = ref<string | null>(null);
const configContent = ref("");
const configOriginal = ref("");
const configLoading = ref(false);
const configSaving = ref(false);
const logs = ref("暂无日志");
const logsLoading = ref(false);
const redisOverview = ref<RedisOverview | null>(null);
const redisVersions = ref<RedisVersionInfo[]>([]);
const redisVersionTarget = ref("");
const redisVersionsLoading = ref(false);
const redisVersionChanging = ref(false);
const mysqlVersions = ref<MysqlVersionInfo[]>([]);
const mysqlVersionTarget = ref("");
const mysqlVersionsLoading = ref(false);
const mysqlVersionChanging = ref(false);
const postgresVersions = ref<PostgresVersionInfo[]>([]);
const postgresVersionTarget = ref("");
const postgresVersionsLoading = ref(false);
const postgresVersionChanging = ref(false);
const nginxVersions = ref<NginxVersionInfo[]>([]);
const nginxVersionTarget = ref("");
const nginxVersionsLoading = ref(false);
const nginxVersionChanging = ref(false);
const managedVersions = ref<ManagedServiceVersionInfo[]>([]);
const managedVersionTarget = ref("");
const managedVersionsLoading = ref(false);
const managedVersionChanging = ref(false);
const versionUninstallTarget = ref<VersionUninstallTarget | null>(null);
const versionUninstalling = ref(false);

interface NginxFileEntry {
  name: string;
  path: string;
  isDir: boolean;
  sizeBytes: number;
}

const nginxFiles = ref<NginxFileEntry[]>([]);
const nginxFilesDir = ref("");
const htmlServicePrefix = computed(() => selectedKind.value === "caddy" ? "caddy" : "nginx");
const nginxFilesLoading = ref(false);
const nginxEditingFile = ref("");
const nginxEditingContent = ref("");
const nginxEditingOriginal = ref("");
const nginxEditingSaving = ref(false);
const nginxNewFileName = ref("");

async function loadNginxFiles(subdir?: string) {
  nginxFilesLoading.value = true;
  try {
    nginxFiles.value = await invoke<NginxFileEntry[]>(`${htmlServicePrefix.value}_html_list`, {
      subdir: subdir || null,
    });
    nginxFilesDir.value = subdir || "";
  } catch (e: any) {
    error.value = String(e);
  } finally {
    nginxFilesLoading.value = false;
  }
}

async function editNginxFile(path: string) {
  nginxEditingFile.value = path;
  try {
    const content = await invoke<string>(`${htmlServicePrefix.value}_html_read`, { path });
    nginxEditingContent.value = content;
    nginxEditingOriginal.value = content;
  } catch (e: any) {
    error.value = String(e);
  }
}

function closeNginxEditor() {
  nginxEditingFile.value = "";
  nginxEditingContent.value = "";
  nginxEditingOriginal.value = "";
}

const nginxFileModified = computed(
  () => nginxEditingContent.value !== nginxEditingOriginal.value,
);

async function saveNginxFile() {
  if (!nginxFileModified.value) return;
  nginxEditingSaving.value = true;
  try {
    await invoke(`${htmlServicePrefix.value}_html_write`, {
      path: nginxEditingFile.value,
      content: nginxEditingContent.value,
    });
    nginxEditingOriginal.value = nginxEditingContent.value;
  } catch (e: any) {
    error.value = String(e);
  } finally {
    nginxEditingSaving.value = false;
  }
}

async function createNginxFile() {
  const name = nginxNewFileName.value.trim();
  if (!name) return;
  const path = nginxFilesDir.value ? `${nginxFilesDir.value}/${name}` : name;
  try {
    await invoke(`${htmlServicePrefix.value}_html_write`, { path, content: "" });
    nginxNewFileName.value = "";
    await loadNginxFiles(nginxFilesDir.value || undefined);
    editNginxFile(path);
  } catch (e: any) {
    error.value = String(e);
  }
}

async function deleteNginxFile(path: string, isDir: boolean) {
  const label = isDir ? `目录 "${path}" 及其所有内容` : `文件 "${path}"`;
  if (!confirm(`确定删除 ${label}？此操作不可恢复。`)) return;
  try {
    await invoke(`${htmlServicePrefix.value}_html_delete`, { path });
    if (path === nginxEditingFile.value) closeNginxEditor();
    await loadNginxFiles(nginxFilesDir.value || undefined);
  } catch (e: any) {
    error.value = String(e);
  }
}
const redisDatabase = ref(0);
const redisPattern = ref("*");
const redisCursor = ref("0");
const redisKeys = ref<string[]>([]);
const redisKeysLoading = ref(false);
const selectedRedisKey = ref<string | null>(null);
const redisKeyDetail = ref<RedisKeyDetail | null>(null);
const redisKeyLoading = ref(false);
const consoleInput = ref("");
const consoleHistory = ref<ConsoleEntry[]>([]);
const consoleRunning = ref(false);
const databaseOverview = ref<DatabaseOverview | null>(null);
const mongoOverview = ref<MongoOverview | null>(null);
const mongoDatabases = ref<MongoDatabaseInfo[]>([]);
const selectedMongoDatabase = ref("");
const mongoCollections = ref<MongoCollectionInfo[]>([]);
const selectedMongoCollection = ref<MongoCollectionInfo | null>(null);
const mongoCollectionDetail = ref<MongoCollectionDetail | null>(null);
const mongoLoading = ref(false);
const mongoDetailLoading = ref(false);
const mongoCommandInput = ref('{"ping": 1}');
const mongoCommandHistory = ref<MongoConsoleEntry[]>([]);
const mongoCommandRunning = ref(false);
const mailpitOverview = ref<MailpitOverview | null>(null);
const natsOverview = ref<NatsOverview | null>(null);
const natsPublishSubject = ref("dev.events");
const natsPublishPayload = ref('{"message":"Hello NATS"}');
const natsSubscribeSubject = ref("dev.>");
const natsPublishResult = ref<NatsPublishResult | null>(null);
const natsMessage = ref<NatsMessage | null>(null);
const natsPublishing = ref(false);
const natsReceiving = ref(false);
const kafkaOverview = ref<KafkaOverview | null>(null);
const kafkaTopics = ref<KafkaTopic[]>([]);
const kafkaTopicName = ref("dev.events");
const kafkaPartitions = ref(3);
const kafkaSelectedTopic = ref("");
const kafkaMessageKey = ref("");
const kafkaMessagePayload = ref('{"message":"Hello Kafka"}');
const kafkaPublishResult = ref<KafkaPublishResult | null>(null);
const kafkaLoading = ref(false);
const kafkaPublishing = ref(false);
const meilisearchOverview = ref<MeilisearchOverview | null>(null);
const meilisearchIndexes = ref<MeilisearchIndex[]>([]);
const meilisearchIndex = ref("movies");
const meilisearchPrimaryKey = ref("id");
const meilisearchDocuments = ref(
  '[\n  {"id": 1, "title": "智屿开发环境", "category": "tools"}\n]',
);
const meilisearchQuery = ref("");
const meilisearchResult = ref<MeilisearchSearchResult | null>(null);
const meilisearchLoading = ref(false);
const meilisearchImporting = ref(false);
const meilisearchSearching = ref(false);
const mailMessages = ref<MailSummary[]>([]);
const selectedMailId = ref<string | null>(null);
const mailDetail = ref<MailDetail | null>(null);
const mailLoading = ref(false);
const mailDetailLoading = ref(false);
const databases = ref<DatabaseInfo[]>([]);
const selectedDatabase = ref("");
const tables = ref<TableInfo[]>([]);
const selectedTable = ref<TableInfo | null>(null);
const tableDetail = ref<TableDetail | null>(null);
const databaseLoading = ref(false);
const tableLoading = ref(false);
const sqlInput = ref("SELECT 1;");
const sqlHistory = ref<SqlConsoleEntry[]>([]);
const sqlRunning = ref(false);
const aiAssistOpen = ref(false);
const aiAssistTitle = ref("");
const aiAssistContext = ref("");
const aiAssistOptions = ref<AiAssistOption[]>([]);
const notice = ref("");
const error = ref("");
const installTask = ref<InstallTask | null>(null);
const installLogExpanded = ref(true);
const installCancelling = ref(false);
let serviceTimer: number | undefined;
let metricTimer: number | undefined;
let diskTimer: number | undefined;
let portTimer: number | undefined;
let resourceSaverTimer: number | undefined;
let unlistenInstallProgress: UnlistenFn | undefined;
let unlistenCloseRequested: UnlistenFn | undefined;
let unlistenTrayNavigation: UnlistenFn | undefined;
let unlistenTrayAction: UnlistenFn | undefined;
let hidingWindow = false;
let lastUserActivityAt = Date.now();
let resourceSaverHandled = false;

const selectedService = computed(
  () => activeTool.value
    ? null
    :
    services.value.find((service) => service.kind === selectedKind.value) ??
    null,
);

const selectedDiskUsage = computed(
  () => diskUsageByKind.value[selectedKind.value] ?? null,
);

const selectedRedisVersionInfo = computed(
  () =>
    redisVersions.value.find(
      (release) => release.version === redisVersionTarget.value,
    ) ?? null,
);

const selectedMysqlVersionInfo = computed(
  () =>
    mysqlVersions.value.find(
      (release) => release.version === mysqlVersionTarget.value,
    ) ?? null,
);

const selectedPostgresVersionInfo = computed(
  () =>
    postgresVersions.value.find(
      (release) => release.version === postgresVersionTarget.value,
    ) ?? null,
);

const selectedNginxVersionInfo = computed(
  () =>
    nginxVersions.value.find(
      (release) => release.version === nginxVersionTarget.value,
    ) ?? null,
);
const selectedManagedVersionInfo = computed(
  () =>
    managedVersions.value.find(
      (release) => release.version === managedVersionTarget.value,
    ) ?? null,
);
const genericMultiVersionKinds: ServiceKind[] = [
  "mongodb",
  "mailpit",
  "nats",
  "meilisearch",
  "influxdb",
  "minio",
  "rustfs",
  "etcd",
  "consul",
  "rnacos",
  "rabbitmq",
  "activemq",
  "caddy",
  "ftp",
];

const serviceControlBusy = computed(
  () =>
    pendingAction.value !== null ||
    redisVersionChanging.value ||
    mysqlVersionChanging.value ||
    postgresVersionChanging.value ||
    nginxVersionChanging.value ||
    managedVersionChanging.value ||
    versionUninstalling.value,
);
const latestInstallLog = computed(
  () => installTask.value?.logs.at(-1) ?? null,
);
const runningServices = computed(() =>
  services.value.filter((service) => service.status === "running"),
);
const installedServiceCount = computed(
  () =>
    services.value.filter((service) => service.status !== "not_installed")
      .length,
);
const stoppedServiceCount = computed(
  () =>
    services.value.filter((service) => service.status === "stopped").length,
);
const notInstalledServiceCount = computed(
  () =>
    services.value.filter((service) => service.status === "not_installed")
      .length,
);
const dashboardCacheBytes = computed(() =>
  Object.values(diskUsageByKind.value).reduce(
    (total, usage) => total + (usage?.cacheBytes ?? 0),
    0,
  ),
);
const dashboardBackupBytes = computed(() =>
  Object.values(diskUsageByKind.value).reduce(
    (total, usage) => total + (usage?.backupBytes ?? 0),
    0,
  ),
);
const dashboardLogsBytes = computed(() =>
  Object.values(diskUsageByKind.value).reduce(
    (total, usage) => total + (usage?.logsBytes ?? 0),
    0,
  ),
);
const dashboardDiskRanking = computed(() =>
  services.value
    .map((service) => ({
      service,
      bytes: diskUsageByKind.value[service.kind]?.totalBytes ?? 0,
    }))
    .filter((item) => item.bytes > 0)
    .sort((left, right) => right.bytes - left.bytes)
    .slice(0, 6),
);
const dashboardPortListeners = computed(() => {
  const servicePorts = new Set(services.value.map((service) => service.port));
  return portListeners.value
    .filter(
      (listener) =>
        listener.managedService !== null || servicePorts.has(listener.port),
    )
    .slice(0, 8);
});
const dashboardAlerts = computed(() => {
  const alerts: Array<{ level: "warning" | "danger"; message: string }> = [];
  for (const service of services.value) {
    if (service.status === "crashed") {
      alerts.push({
        level: "danger",
        message: `${service.name} 进程已意外退出（原 PID ${service.pid ?? "未知"}），可一键修复状态后重新启动`,
      });
      continue;
    }
    if (service.status === "stale_pid") {
      alerts.push({
        level: "danger",
        message: `${service.name} 的 PID 身份校验失败，已清理过期 PID 文件`,
      });
      continue;
    }
    if (service.status !== "running") {
      const occupied = portListeners.value.find(
        (listener) =>
          listener.port === service.port && listener.managedService === null,
      );
      if (occupied) {
        alerts.push({
          level: "warning",
          message: `${service.name} 默认端口 ${service.port} 已被 ${occupied.process}（PID ${occupied.pid}）占用`,
        });
      }
    }
  }
  const latestFailure =
    activityRecords.value[0] && !activityRecords.value[0].success
      ? activityRecords.value[0]
      : null;
  if (latestFailure) {
    alerts.push({
      level: "danger",
      message: `最近一次失败：${latestFailure.service} ${latestFailure.action}，${latestFailure.message}`,
    });
  }
  return alerts.slice(0, 6);
});

function loadActivityRecords(): ActivityRecord[] {
  try {
    const parsed = JSON.parse(
      globalThis.localStorage?.getItem(ACTIVITY_STORAGE_KEY) ?? "[]",
    );
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(
        (item) =>
          item &&
          typeof item.id === "string" &&
          typeof item.service === "string" &&
          typeof item.action === "string" &&
          typeof item.createdAt === "number",
      )
      .slice(0, 30) as ActivityRecord[];
  } catch {
    return [];
  }
}

function recordActivity(
  service: ServiceInfo,
  action: string,
  success: boolean,
  message: string,
) {
  activityRecords.value = [
    {
      id: newOperationId(),
      kind: service.kind,
      service: service.name,
      action,
      success,
      createdAt: Date.now(),
      message,
    },
    ...activityRecords.value,
  ].slice(0, 30);
  try {
    globalThis.localStorage?.setItem(
      ACTIVITY_STORAGE_KEY,
      JSON.stringify(activityRecords.value),
    );
  } catch {
    // Activity history remains available for the current session.
  }
}

function formatActivityTime(value: number) {
  return new Date(value).toLocaleString(resolvedLocale.value, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  });
}

function newOperationId() {
  return globalThis.crypto?.randomUUID?.() ??
    `install-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function startInstallTask(kind: string, title: string) {
  const operationId = newOperationId();
  installTask.value = {
    operationId,
    kind,
    title,
    percent: 0,
    stage: "等待开始",
    status: "running",
    logs: [],
  };
  installLogExpanded.value = true;
  return operationId;
}

function recordInstallFailure(operationId: string, cause: unknown) {
  const task = installTask.value;
  if (
    !task ||
    task.operationId !== operationId ||
    task.status === "failed" ||
    task.status === "cancelled"
  ) {
    return;
  }
  const message = String(cause);
  task.status = "failed";
  task.stage = "安装失败";
  task.logs.push({
    time: new Date().toLocaleTimeString(resolvedLocale.value, { hour12: false }),
    stage: "安装失败",
    message,
  });
}

async function cancelCurrentInstall() {
  const task = installTask.value;
  if (!task || task.status !== "running" || installCancelling.value) return;
  installCancelling.value = true;
  task.stage = "正在取消";
  try {
    await cancelInstall(task.operationId);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    installCancelling.value = false;
  }
}

function recordInstallSuccess(operationId: string) {
  const task = installTask.value;
  if (
    !task ||
    task.operationId !== operationId ||
    task.status === "completed"
  ) {
    return;
  }
  task.status = "completed";
  task.percent = 100;
  task.stage = "安装完成";
  task.logs.push({
    time: new Date().toLocaleTimeString(resolvedLocale.value, { hour12: false }),
    stage: "安装完成",
    message: "安装、配置和初始化均已完成",
  });
}

// 安装进度条由 App 持有，需要下载资源的工具组件通过 inject 复用
provide(INSTALL_TASK_KEY, {
  start: startInstallTask,
  succeed: recordInstallSuccess,
  fail: recordInstallFailure,
});

function handleInstallProgress(payload: InstallProgressPayload) {
  const task = installTask.value;
  if (!task || task.operationId !== payload.operationId) return;
  if (payload.percent !== null) {
    task.percent = Math.max(task.percent, payload.percent);
  }
  task.stage = payload.stage;
  task.status = payload.status;
  task.logs.push({
    time: new Date().toLocaleTimeString(resolvedLocale.value, { hour12: false }),
    stage: payload.stage,
    message: payload.message,
  });
}

const configChanged = computed(
  () => configContent.value !== configOriginal.value,
);

const detailTabs = computed<Array<[DetailTab, string]>>(() => {
  if (selectedKind.value === "ftp") {
    return [
      ["overview", "概览"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "rabbitmq") {
    return [
      ["overview", "概览"],
      ["broker", "连接与控制台"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "redis") {
    return [
      ["overview", "概览"],
      ["keys", "数据浏览"],
      ["console", "命令台"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "mysql") {
    return [
      ["overview", "概览"],
      ["data", "数据浏览"],
      ["sql", "SQL 命令台"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "postgres") {
    return [
      ["overview", "概览"],
      ["data", "数据浏览"],
      ["sql", "SQL 命令台"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "mongodb") {
    return [
      ["overview", "概览"],
      ["data", "数据浏览"],
      ["mongoConsole", "JSON 命令台"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "mailpit") {
    return [
      ["overview", "概览"],
      ["mail", "邮件收件箱"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "nats") {
    return [
      ["overview", "概览"],
      ["messages", "消息调试"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "kafka") {
    return [
      ["overview", "概览"],
      ["messages", "主题与消息"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "meilisearch") {
    return [
      ["overview", "概览"],
      ["search", "索引与搜索"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "influxdb") {
    return [
      ["overview", "概览"],
      ["timeseries", "时序数据"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "minio" || selectedKind.value === "rustfs") {
    return [
      ["overview", "概览"],
      ["objectStore", "连接与控制台"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (
    selectedKind.value === "etcd" ||
    selectedKind.value === "consul" ||
    selectedKind.value === "rnacos"
  ) {
    return [
      ["overview", "概览"],
      ["governance", "连接与调试"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "nginx") {
    return [
      ["overview", "概览"],
      ["site", "站点"],
      ["files", "文件管理"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "caddy") {
    return [
      ["overview", "概览"],
      ["site", "站点"],
      ["files", "文件管理"],
      ["connect", "连接"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["versions", "版本管理"],
      ["docs", "使用文档"],
    ];
  }
  return [
    ["overview", "概览"],
    ["data", "数据浏览"],
    ["sql", "SQL 命令台"],
    ["connect", "连接"],
    ["backup", "备份恢复"],
    ["config", "配置文件"],
    ["logs", "运行日志"],
    ["versions", "版本管理"],
    ["docs", "使用文档"],
  ];
});

const statusLabel = computed<Record<ServiceState, string>>(() => ({
  not_installed: t("status.notInstalled"),
  stopped: t("status.stopped"),
  running: t("status.running"),
  stale_pid: t("status.stalePid"),
  crashed: t("status.crashed"),
}));

const iconLetter: Record<ServiceKind, string> = {
  redis: "R",
  mysql: "M",
  postgres: "P",
  mongodb: "M",
  mailpit: "@",
  nats: "N",
  kafka: "K",
  meilisearch: "M",
  influxdb: "I",
  minio: "M",
  rustfs: "R",
  etcd: "E",
  consul: "C",
  rnacos: "R",
  rabbitmq: "Q",
  activemq: "A",
  nginx: "N",
  caddy: "C",
  ftp: "F",
};

function openExternal(url: string) {
  invoke("open_url", { url }).catch(() => {});
}

function openPath(path: string) {
  invoke("open_path", { path }).catch(() => {});
}

const governanceProfile = computed(() =>
  selectedKind.value === "rnacos"
    ? {
        name: "rnacos",
        description: "无需 Java 的 Nacos 兼容配置中心与服务注册中心",
        badge: "单节点 · 无需 Java",
        primaryLabel: "NACOS HTTP",
        primary: "http://127.0.0.1:8848",
        primaryHint: "1.x OpenAPI 与客户端",
        secondaryLabel: "WEB CONSOLE",
        secondary: "http://127.0.0.1:10848/rnacos/",
        secondaryHint: "默认 admin / admin",
        command:
          "curl 'http://127.0.0.1:8848/nacos/v1/cs/configs?dataId=demo&group=DEFAULT_GROUP'",
        note: "OpenAPI 鉴权默认关闭，控制台使用开发账号 admin / admin；不要暴露到公网。",
      }
    : selectedKind.value === "consul"
    ? {
        name: "Consul",
        description: "适合服务注册、健康检查、KV 配置和 DNS 服务发现",
        badge: "单节点 Server · 仅本机",
        primaryLabel: "HTTP API / WEB UI",
        primary: "http://127.0.0.1:8500",
        primaryHint: "API 与 /ui/ 管理界面",
        secondaryLabel: "DNS",
        secondary: "127.0.0.1:8600",
        secondaryHint: "DNS 服务发现",
        command:
          "CONSUL_HTTP_ADDR=http://127.0.0.1:8500 \\\n  ~/.devbox/installations/consul/1.22/bin/consul members",
        note: "智屿只运行本机单节点 Server Agent，不启用 ACL，也不模拟生产集群。",
      }
    : {
        name: "etcd",
        description: "适合配置读取、服务协调、分布式锁和客户端兼容调试",
        badge: "单节点 · 仅本机",
        primaryLabel: "CLIENT ENDPOINT",
        primary: "http://127.0.0.1:2379",
        primaryHint: "应用和 etcdctl 使用",
        secondaryLabel: "PEER ENDPOINT",
        secondary: "http://127.0.0.1:2380",
        secondaryHint: "单节点内部通信",
        command:
          "ETCDCTL_API=3 ~/.devbox/installations/etcd/3.6/bin/etcdctl \\\n  --endpoints=http://127.0.0.1:2379 put hello zhiyu\n\nETCDCTL_API=3 ~/.devbox/installations/etcd/3.6/bin/etcdctl \\\n  --endpoints=http://127.0.0.1:2379 get hello",
        note: "智屿只启用本机单节点模式，不开放远程监听，也不模拟生产集群。",
      },
);

const objectStoreProfile = computed(() =>
  selectedKind.value === "rustfs"
    ? {
        name: "RustFS",
        apiEndpoint: "http://127.0.0.1:9002",
        consoleEndpoint: "http://127.0.0.1:7001",
        accessKey: "zhiyuadmin",
        secretKey: "zhiyu-local-rustfs-2026",
        badge: "推荐尝鲜 · Beta",
        note: "RustFS 当前仍处于 Beta 阶段，适合本地开发验证，不建议保存唯一副本或生产数据。",
      }
    : {
        name: "MinIO",
        apiEndpoint: "http://127.0.0.1:9000",
        consoleEndpoint: "http://127.0.0.1:9001",
        accessKey: "zhiyuadmin",
        secretKey: "zhiyu-local-minio-2026",
        badge: "存量兼容 · 官方仓库已归档",
        note: "MinIO 社区仓库已归档，本模块用于兼容已有开发项目；新项目建议选择 RustFS。",
      },
);

const actionLabel = computed<Record<ServiceAction, string>>(() => ({
  install: t("common.installing"),
  start: t("common.starting"),
  stop: t("common.stopping"),
  restart: t("common.restarting"),
}));

async function refreshServices(silent = false) {
  if (!silent) loading.value = true;
  try {
    services.value = await listServices();
    if (!silent) error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

async function refreshMetrics() {
  if (activeTool.value || dashboardActive.value) return;
  const service = selectedService.value;
  if (!service || service.status !== "running") {
    metrics.value = {
      running: false,
      cpuPercent: null,
      memoryBytes: null,
      uptime: null,
    };
    redisOverview.value = null;
    databaseOverview.value = null;
    mongoOverview.value = null;
    mailpitOverview.value = null;
    natsOverview.value = null;
    kafkaOverview.value = null;
    meilisearchOverview.value = null;
    return;
  }

  try {
    metrics.value = await getServiceMetrics(service.kind);
    if (metrics.value.running) {
      metricHistory.value.push({
        cpu: metrics.value.cpuPercent ?? 0,
        memory: metrics.value.memoryBytes ?? 0,
      });
      if (metricHistory.value.length > 30) metricHistory.value.shift();
    }
    if (service.kind === "redis") {
      try {
        redisOverview.value = await getRedisOverview();
      } catch {
        redisOverview.value = null;
      }
    } else if (service.kind === "mongodb") {
      if (!mongoOverview.value) {
        try {
          mongoOverview.value = await getMongoOverview();
        } catch {
          mongoOverview.value = null;
        }
      }
    } else if (service.kind === "mailpit") {
      try {
        mailpitOverview.value = await getMailpitOverview();
      } catch {
        mailpitOverview.value = null;
      }
    } else if (service.kind === "nats") {
      try {
        natsOverview.value = await getNatsOverview();
      } catch {
        natsOverview.value = null;
      }
    } else if (service.kind === "kafka") {
      try {
        kafkaOverview.value = await getKafkaOverview();
      } catch {
        kafkaOverview.value = null;
      }
    } else if (service.kind === "meilisearch") {
      try {
        meilisearchOverview.value = await getMeilisearchOverview();
      } catch {
        meilisearchOverview.value = null;
      }
    } else if (
      (service.kind === "mysql" || service.kind === "postgres") &&
      !databaseOverview.value
    ) {
      try {
        databaseOverview.value = await getDatabaseOverview(service.kind);
      } catch {
        databaseOverview.value = null;
      }
    }
  } catch {
    // Monitoring is best-effort and must never interrupt service controls.
  }
}

async function refreshDiskUsage(kind?: ServiceKind) {
  const kinds = kind ? [kind] : services.value.map((service) => service.kind);
  await Promise.all(
    kinds.map(async (serviceKind) => {
      try {
        const usage = await getServiceDiskUsage(serviceKind);
        diskUsageByKind.value = {
          ...diskUsageByKind.value,
          [serviceKind]: usage,
        };
      } catch {
        // Disk usage is best-effort and refreshes independently.
      }
    }),
  );
}

async function refreshEnvironmentMetrics() {
  try {
    environmentMetrics.value = await getEnvironmentMetrics();
  } catch {
    // The brand summary is best-effort and must not interrupt service controls.
  }
}

async function refreshEnvironmentDiskUsage() {
  try {
    environmentDiskBytes.value = await getEnvironmentDiskUsage();
  } catch {
    // Disk usage can be temporarily unavailable while files are being moved.
  }
}

async function refreshPortListeners() {
  try {
    portListeners.value = await listPortListeners();
  } catch {
    // Port inspection is best-effort on the global dashboard.
  }
}

async function openDashboard() {
  dashboardActive.value = true;
  settingsActive.value = false;
  activeTool.value = null;
  notice.value = "";
  error.value = "";
  await Promise.all([
    refreshServices(true),
    refreshEnvironmentMetrics(),
    refreshEnvironmentDiskUsage(),
    refreshPortListeners(),
    refreshDiskUsage(),
    refreshPortListeners(),
  ]);
}

async function openDiagnostics() {
  diagnosticsOpen.value = true;
  await runDiagnostics();
}

async function runDiagnostics() {
  if (diagnosticsRunning.value || diagnosticsRepairing.value) return;
  diagnosticsRunning.value = true;
  try {
    diagnosticReport.value = await runAppDiagnostics();
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    diagnosticsRunning.value = false;
  }
}

async function repairDiagnostics() {
  if (
    diagnosticsRunning.value ||
    diagnosticsRepairing.value ||
    !diagnosticReport.value?.summary.repairable
  ) {
    return;
  }
  diagnosticsRepairing.value = true;
  try {
    const result = await repairAppDiagnostics();
    diagnosticReport.value = result.report;
    notice.value =
      result.repairedCount > 0
        ? `诊断修复完成，共处理 ${result.repairedCount} 项`
        : "诊断完成，没有需要自动修复的项目";
    error.value = "";
    await Promise.all([
      refreshServices(true),
      refreshPortListeners(),
      refreshEnvironmentDiskUsage(),
      refreshDiskUsage(),
    ]);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    diagnosticsRepairing.value = false;
  }
}

async function copyDiagnosticReport() {
  const report = diagnosticReport.value;
  if (!report) return;
  const homePrefix = appSettings.value.installRoot.replace(
    /\/?\.devbox\/?$/,
    "",
  );
  const redact = (value: string) =>
    (homePrefix && homePrefix !== value
      ? value.replaceAll(homePrefix, "~")
      : value
    )
      .replace(/\/Users\/[^/\s]+/g, "~")
      .replace(/[A-Za-z]:\\Users\\[^\\\s]+/g, "~");
  const lines = [
    `智屿诊断报告 ${new Date(report.generatedAtMillis).toLocaleString()}`,
    `通过 ${report.summary.passed} · 警告 ${report.summary.warnings} · 错误 ${report.summary.errors}`,
    "",
    ...report.items.flatMap((item) => [
      `[${item.status.toUpperCase()}] ${item.scope} / ${item.title}: ${redact(item.message)}`,
      ...(item.detail &&
      !item.id.endsWith("-crashed") &&
      !item.id.endsWith("-port-not-ready")
        ? [redact(item.detail)]
        : []),
    ]),
  ];
  try {
    await navigator.clipboard.writeText(lines.join("\n"));
    notice.value = "诊断报告已复制，用户目录已脱敏";
  } catch (cause) {
    error.value = `复制诊断报告失败：${String(cause)}`;
  }
}

async function loadAppSettings() {
  try {
    appSettings.value = await getAppSettings();
    settingsDraft.value = { ...appSettings.value };
    await setAppLocale(appSettings.value.locale);
    applyAppTheme(
      appSettings.value.themeMode,
      appSettings.value.colorTheme,
    );
    applyUiScale(appSettings.value.uiScale);
  } catch (cause) {
    error.value = String(cause);
  }
}

function createAiInput(settings: AiSettings): AiSettingsInput {
  return {
    enabled: settings.enabled,
    protocol: settings.protocol,
    baseUrl: settings.baseUrl,
    model: settings.model,
    timeoutSeconds: settings.timeoutSeconds,
    maxOutputTokens: settings.maxOutputTokens,
    userAvatarPath: settings.userAvatarPath,
    assistantAvatarPath: settings.assistantAvatarPath,
    apiKey: "",
    clearApiKey: false,
  };
}

function applyAiAvatarSettings(settings: AiSettings) {
  aiSettings.value = settings;
  aiDraft.value.userAvatarPath = settings.userAvatarPath;
  aiDraft.value.assistantAvatarPath = settings.assistantAvatarPath;
}

async function chooseAiAvatar(role: "user" | "assistant") {
  if (aiAvatarImporting.value) return;
  const selected = await open({
    multiple: false,
    title: role === "user"
      ? t("settings.ai.chooseUserAvatar")
      : t("settings.ai.chooseAssistantAvatar"),
    filters: [{
      name: t("settings.ai.avatarImage"),
      extensions: ["png", "jpg", "jpeg", "webp"],
    }],
  });
  if (typeof selected !== "string") return;
  aiAvatarImporting.value = role;
  try {
    applyAiAvatarSettings(await importAiAvatar(role, selected));
    showToast({
      intent: "success",
      title: t("settings.ai.avatarUpdated"),
      message: t("settings.ai.avatarUpdatedHint"),
    });
  } catch (cause) {
    showToast({
      intent: "error",
      title: t("settings.ai.avatarFailed"),
      message: String(cause),
    });
  } finally {
    aiAvatarImporting.value = null;
  }
}

async function clearAiAvatar(role: "user" | "assistant") {
  if (aiAvatarImporting.value) return;
  aiAvatarImporting.value = role;
  try {
    applyAiAvatarSettings(await removeAiAvatar(role));
  } catch (cause) {
    showToast({
      intent: "error",
      title: t("settings.ai.avatarFailed"),
      message: String(cause),
    });
  } finally {
    aiAvatarImporting.value = null;
  }
}

async function loadAiSettings() {
  aiSettingsLoading.value = true;
  try {
    aiSettings.value = await getAiSettings();
    aiDraft.value = createAiInput(aiSettings.value);
    const matchedProvider = aiProviderPresets.find((provider) =>
      Object.values(provider.baseUrls).some(
        (baseUrl) => baseUrl === aiSettings.value.baseUrl,
      ),
    );
    aiProviderId.value = matchedProvider?.id ?? "custom";
    aiCustomModel.value = Boolean(
      matchedProvider &&
        !matchedProvider.models.includes(aiSettings.value.model),
    );
    aiTestResult.value = null;
  } catch (cause) {
    error.value = String(cause);
  } finally {
    aiSettingsLoading.value = false;
  }
}

function selectAiProvider(providerId: AiProviderId) {
  aiProviderId.value = providerId;
  aiCustomModel.value = providerId === "custom";
  aiTestResult.value = null;
  if (providerId === "custom") return;
  const provider = aiProviderPresets.find((item) => item.id === providerId);
  if (!provider) return;
  const protocol = provider.baseUrls[aiDraft.value.protocol]
    ? aiDraft.value.protocol
    : (Object.keys(provider.baseUrls)[0] as AiApiProtocol);
  aiDraft.value.protocol = protocol;
  aiDraft.value.baseUrl = provider.baseUrls[protocol] ?? "";
  aiDraft.value.model = provider.models[0] ?? "";
}

function selectAiProtocol(protocol: AiApiProtocol) {
  const provider = activeAiProvider.value;
  if (provider) {
    const providerUrl = provider.baseUrls[protocol];
    if (!providerUrl) return;
    aiDraft.value.protocol = protocol;
    aiDraft.value.baseUrl = providerUrl;
    aiTestResult.value = null;
    return;
  }
  const oldDefault =
    aiDraft.value.protocol === "anthropic"
      ? "https://api.anthropic.com/v1"
      : "https://api.openai.com/v1";
  const nextDefault =
    protocol === "anthropic"
      ? "https://api.anthropic.com/v1"
      : "https://api.openai.com/v1";
  if (!aiDraft.value.baseUrl || aiDraft.value.baseUrl === oldDefault) {
    aiDraft.value.baseUrl = nextDefault;
  }
  aiDraft.value.protocol = protocol;
  aiTestResult.value = null;
}

function aiProtocolSupported(protocol: AiApiProtocol): boolean {
  return !activeAiProvider.value || Boolean(activeAiProvider.value.baseUrls[protocol]);
}

function selectAiModel(event: Event) {
  const model = (event.currentTarget as HTMLSelectElement).value;
  if (model === "__custom__") {
    aiCustomModel.value = true;
    aiDraft.value.model = "";
  } else {
    aiCustomModel.value = false;
    aiDraft.value.model = model;
  }
  aiTestResult.value = null;
}

function useRecommendedAiModel() {
  const model = aiModelSuggestions.value[0];
  if (!model) return;
  aiDraft.value.model = model;
  aiCustomModel.value = false;
  aiTestResult.value = null;
}

function openAiKeyPage() {
  const url = activeAiProvider.value?.keyUrl;
  if (url) openExternal(url);
}

async function saveAiConfiguration() {
  aiSettingsSaving.value = true;
  aiTestResult.value = null;
  try {
    const saved = await saveAiSettings({ ...aiDraft.value });
    aiSettings.value = saved;
    aiDraft.value = createAiInput(saved);
    showToast({
      intent: "success",
      title: t("settings.ai.saved"),
      message: t("settings.ai.savedHint"),
    });
  } catch (cause) {
    aiTestResult.value = {
      success: false,
      message: String(cause),
    };
  } finally {
    aiSettingsSaving.value = false;
  }
}

async function runAiConnectionTest() {
  aiConnectionTesting.value = true;
  aiTestResult.value = null;
  try {
    const result = await testAiConnection({ ...aiDraft.value });
    aiTestResult.value = {
      success: true,
      message: result.message,
      latencyMillis: result.latencyMillis,
    };
  } catch (cause) {
    aiTestResult.value = {
      success: false,
      message: String(cause),
    };
  } finally {
    aiConnectionTesting.value = false;
  }
}

function clearAiApiKey() {
  aiDraft.value.apiKey = "";
  aiDraft.value.clearApiKey = true;
  aiDraft.value.enabled = false;
  aiTestResult.value = null;
}

async function selectLocale(locale: AppLocale) {
  settingsDraft.value.locale = locale;
  await setAppLocale(locale);
  await saveSettings();
}

function previewTheme(mode: ThemeMode) {
  settingsDraft.value.themeMode = mode;
  applyAppTheme(mode, settingsDraft.value.colorTheme);
  void saveSettings();
}

function previewColorTheme(theme: ColorTheme) {
  settingsDraft.value.colorTheme = theme;
  applyAppTheme(settingsDraft.value.themeMode, theme);
  void saveSettings();
}

function previewBackgroundPattern(pattern: BackgroundPattern) {
  settingsDraft.value.backgroundPattern = pattern;
  void saveSettings();
}

function toggleServiceVisibility(kind: ServiceKind, event: Event) {
  const visible = (event.currentTarget as HTMLInputElement).checked;
  const hidden = new Set(settingsDraft.value.hiddenServices);
  if (visible) hidden.delete(kind);
  else hidden.add(kind);
  settingsDraft.value.hiddenServices = [...hidden];
  void saveSettings();
}

function toggleToolVisibility(id: ToolId, event: Event) {
  const visible = (event.currentTarget as HTMLInputElement).checked;
  const hidden = new Set(settingsDraft.value.hiddenTools);
  if (visible) hidden.delete(id);
  else hidden.add(id);
  settingsDraft.value.hiddenTools = [...hidden];
  void saveSettings();
}

function beginSidebarPointerDrag(
  event: PointerEvent,
  group: "services" | "tools",
  id: string,
) {
  if (event.button !== 0) return;
  event.preventDefault();
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  draggingSidebarItem.value = { group, id };
  sidebarPointerDrop.value = null;
  sidebarDropTarget.value = "";
  document.documentElement.classList.add("sidebar-item-dragging");
  window.addEventListener("pointermove", moveSidebarPointerDrag);
  window.addEventListener("pointerup", finishSidebarPointerDrag, {
    once: true,
  });
  window.addEventListener("pointercancel", cancelSidebarPointerDrag, {
    once: true,
  });
}

function moveSidebarPointerDrag(event: PointerEvent) {
  const dragging = draggingSidebarItem.value;
  if (!dragging) return;
  event.preventDefault();
  const target = document
    .elementFromPoint(event.clientX, event.clientY)
    ?.closest<HTMLElement>(".sidebar-manager-item");
  const group = target?.dataset.sidebarGroup;
  const id = target?.dataset.sidebarId;
  if (
    !target ||
    group !== dragging.group ||
    !id ||
    id === dragging.id
  ) {
    sidebarPointerDrop.value = null;
    sidebarDropTarget.value = "";
    return;
  }
  const bounds = target.getBoundingClientRect();
  const after = event.clientY >= bounds.top + bounds.height / 2;
  sidebarPointerDrop.value = {
    group: dragging.group,
    id,
    after,
  };
  sidebarDropTarget.value = `${dragging.group}:${id}:${after ? "after" : "before"}`;
}

function finishSidebarPointerDrag(event: PointerEvent) {
  event.preventDefault();
  const dragging = draggingSidebarItem.value;
  const drop = sidebarPointerDrop.value;
  if (!dragging || !drop || dragging.group !== drop.group) {
    endSidebarPointerDrag();
    return;
  }
  reorderSidebarItem(dragging.group, dragging.id, drop.id, drop.after);
  endSidebarPointerDrag();
  void saveSettings();
}

function reorderSidebarItem(
  group: "services" | "tools",
  sourceId: string,
  targetId: string,
  after: boolean,
) {
  const current: string[] =
    group === "services"
      ? settingsOrderedServices.value.map((service) => service.kind)
      : settingsOrderedTools.value.map((tool) => tool.id);
  const next = current.filter((id) => id !== sourceId);
  const targetIndex = next.indexOf(targetId);
  const insertAt =
    targetIndex < 0 ? next.length : targetIndex + (after ? 1 : 0);
  next.splice(insertAt, 0, sourceId);
  if (group === "services") {
    settingsDraft.value.serviceOrder = next as ServiceKind[];
  } else {
    settingsDraft.value.toolOrder = next;
  }
}

function cancelSidebarPointerDrag() {
  endSidebarPointerDrag();
}

function endSidebarPointerDrag() {
  window.removeEventListener("pointermove", moveSidebarPointerDrag);
  window.removeEventListener("pointerup", finishSidebarPointerDrag);
  window.removeEventListener("pointercancel", cancelSidebarPointerDrag);
  document.documentElement.classList.remove("sidebar-item-dragging");
  draggingSidebarItem.value = null;
  sidebarPointerDrop.value = null;
  sidebarDropTarget.value = "";
}

function previewUiScale(scale: UiScale) {
  settingsDraft.value.uiScale = scale;
  applyUiScale(scale);
  void saveSettings();
}

function selectProxyMode(mode: ProxyMode) {
  settingsDraft.value.proxyMode = mode;
  if (mode === "manual" && !settingsDraft.value.proxyUrl) {
    settingsDraft.value.proxyUrl = "http://127.0.0.1:7890";
  }
  void saveSettings();
}

async function chooseAppBackground() {
  if (backgroundImporting.value) return;
  const selected = await open({
    multiple: false,
    title: "选择智屿背景图",
    filters: [
      {
        name: "图片",
        extensions: ["png", "jpg", "jpeg", "webp"],
      },
    ],
  });
  if (typeof selected !== "string") return;

  backgroundImporting.value = true;
  error.value = "";
  try {
    settingsDraft.value.backgroundImagePath =
      await importAppBackground(selected);
    if (settingsDraft.value.backgroundStyle === "off") {
      settingsDraft.value.backgroundStyle = "frosted";
    }
    await saveSettings();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    backgroundImporting.value = false;
  }
}

function selectBackgroundStyle(style: BackgroundStyle) {
  settingsDraft.value.backgroundStyle = style;
  void saveSettings();
}

function selectBackgroundPosition(position: BackgroundPosition) {
  settingsDraft.value.backgroundPosition = position;
  void saveSettings();
}

async function clearAppBackground() {
  if (!settingsDraft.value.backgroundImagePath) return;
  error.value = "";
  try {
    await removeAppBackground();
    settingsDraft.value.backgroundImagePath = "";
    settingsDraft.value.backgroundStyle = "off";
    await saveSettings();
  } catch (cause) {
    error.value = String(cause);
  }
}

async function openSettings() {
  if (settingsActive.value) return;
  dashboardActive.value = false;
  settingsActive.value = true;
  activeTool.value = null;
  notice.value = "";
  error.value = "";
  if (settingsSaving.value) return;
  await Promise.all([loadAppSettings(), loadAiSettings()]);
  if (appSettings.value.autoCheckUpdates && !updateStatus.value) {
    void checkForUpdates();
  }
}

function openCommandPalette() {
  commandPaletteOpen.value = true;
  void runtimeProjectsList()
    .then((projects) => {
      commandProjects.value = projects;
    })
    .catch(() => {
      commandProjects.value = [];
    });
}

async function handleCommandSelect(item: CommandPaletteItem) {
  if (commandBusyId.value) return;
  commandBusyId.value = item.id;
  try {
    const [type, id, action] = item.id.split(":");
    if (type === "nav" && id === "dashboard") {
      commandPaletteOpen.value = false;
      await openDashboard();
    } else if (type === "nav" && id === "settings") {
      commandPaletteOpen.value = false;
      await openSettings();
    } else if (type === "tool") {
      commandPaletteOpen.value = false;
      selectTool(id as ToolId);
    } else if (type === "service") {
      commandPaletteOpen.value = false;
      await selectService(id as ServiceKind);
    } else if (type === "project") {
      commandPaletteOpen.value = false;
      selectTool("workspace");
      await nextTick();
      window.dispatchEvent(
        new CustomEvent("zhiyu:project-open", { detail: { id } }),
      );
    } else if (type === "action" && action) {
      commandPaletteOpen.value = false;
      await selectService(id as ServiceKind);
      await execute(action as ServiceAction);
    }
  } finally {
    commandBusyId.value = "";
  }
}

function handleGlobalKeydown(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
    event.preventDefault();
    if (commandPaletteOpen.value) {
      commandPaletteOpen.value = false;
    } else {
      openCommandPalette();
    }
  }
}

function markUserActivity() {
  lastUserActivityAt = Date.now();
  resourceSaverHandled = false;
  dismissToastByKey("resource-saver");
}

async function stopResourceSaverTargets(targets?: ServiceInfo[]) {
  const selected =
    targets ??
    services.value.filter(
      (service) =>
        service.status === "running" &&
        appSettings.value.resourceSaverServices.includes(service.kind),
    );
  if (!selected.length) return;
  const failed: string[] = [];
  for (const service of selected) {
    try {
      await runServiceAction("stop", service.kind);
    } catch {
      failed.push(service.name);
    }
  }
  await Promise.all([refreshServices(true), refreshEnvironmentMetrics()]);
  showToast({
    key: "resource-saver",
    intent: failed.length ? "warning" : "success",
    title: failed.length
      ? t("resourceSaver.partialStop")
      : t("resourceSaver.stopped", { count: selected.length }),
    message: failed.length
      ? t("resourceSaver.failedServices", { services: failed.join("、") })
      : undefined,
  });
}

async function checkResourceSaver() {
  const settings = appSettings.value;
  if (
    !settings.resourceSaverEnabled ||
    resourceSaverHandled ||
    Date.now() - lastUserActivityAt < settings.resourceSaverMinutes * 60_000
  ) {
    return;
  }
  const targets = services.value.filter(
    (service) =>
      service.status === "running" &&
      settings.resourceSaverServices.includes(service.kind),
  );
  if (!targets.length) return;
  resourceSaverHandled = true;
  if (settings.resourceSaverMode === "stop") {
    await stopResourceSaverTargets(targets);
  } else {
    showToast({
      key: "resource-saver",
      intent: "warning",
      title: t("resourceSaver.idleTitle", {
        minutes: settings.resourceSaverMinutes,
      }),
      message: t("resourceSaver.idleMessage", {
        services: targets.map((service) => service.name).join("、"),
      }),
      duration: 0,
      actionLabel: t("resourceSaver.stopNow"),
      onAction: () => void stopResourceSaverTargets(targets),
    });
  }
}

function handleWorkspaceNavigation(event: Event) {
  const detail = (
    event as CustomEvent<{ type?: string; id?: ServiceKind | ToolId }>
  ).detail;
  if (detail?.type === "service" && detail.id) {
    void selectService(detail.id as ServiceKind);
  } else if (detail?.type === "tool" && detail.id) {
    selectTool(detail.id as ToolId);
  }
}

async function openAiSettingsFromChat() {
  aiChatOpen.value = false;
  await openSettings();
  activeSettingsTab.value = "ai";
}

function handleOpenAiSettingsRequest() {
  void openAiSettingsFromChat();
}

async function chooseInstallRoot() {
  const selected = await open({
    directory: true,
    multiple: false,
    defaultPath: settingsDraft.value.installRoot || undefined,
    title: "选择智屿安装目录",
  });
  if (typeof selected === "string") {
    settingsDraft.value.installRoot = selected;
    void saveSettings();
  }
}

async function chooseOnboardingRoot() {
  const selected = await open({
    directory: true,
    multiple: false,
    defaultPath: settingsDraft.value.installRoot || undefined,
    title: "选择智屿环境目录",
  });
  if (typeof selected !== "string") return;
  settingsDraft.value.installRoot = selected;
  await saveSettings();
}

function showOnboarding() {
  onboardingStep.value = 0;
  onboardingOpen.value = true;
}

async function finishOnboarding(kind?: ServiceKind) {
  settingsDraft.value.onboardingCompleted = true;
  onboardingOpen.value = false;
  onboardingStep.value = 0;
  await saveSettings();
  if (kind) await selectService(kind);
}

async function saveSettings() {
  if (settingsSaving.value) {
    settingsSaveQueued = true;
    return;
  }
  settingsSaving.value = true;
  error.value = "";
  try {
    do {
      settingsSaveQueued = false;
      const snapshot = { ...settingsDraft.value };
      const rootChanged = snapshot.installRoot !== appSettings.value.installRoot;
      if (rootChanged && runningServices.value.length > 0) {
        settingsDraft.value.installRoot = appSettings.value.installRoot;
        throw new Error(
          `请先停止当前运行的 ${runningServices.value.length} 个服务，再更换安装目录`,
        );
      }
      if (
        rootChanged &&
        !window.confirm(
          "更换安装目录后，智屿会切换到一个新的环境。旧目录中的服务和数据不会自动迁移，确定保存吗？",
        )
      ) {
        settingsDraft.value.installRoot = appSettings.value.installRoot;
        return;
      }

      const saved = await saveAppSettings(snapshot);
      appSettings.value = saved;
      if (
        saved.themeMode === settingsDraft.value.themeMode &&
        saved.colorTheme === settingsDraft.value.colorTheme
      ) {
        applyAppTheme(saved.themeMode, saved.colorTheme);
      }
      if (saved.uiScale === settingsDraft.value.uiScale) {
        applyUiScale(saved.uiScale);
      }
      if (rootChanged) {
        notice.value = "已自动保存，并切换到新的安装目录";
        diskUsageByKind.value = {};
        await Promise.all([
          refreshServices(true),
          refreshEnvironmentDiskUsage(),
          refreshDiskUsage(),
          refreshPortListeners(),
        ]);
      }
    } while (settingsSaveQueued);
  } catch (cause) {
    error.value = String(cause);
    settingsDraft.value = { ...appSettings.value };
    applyAppTheme(
      appSettings.value.themeMode,
      appSettings.value.colorTheme,
    );
    applyUiScale(appSettings.value.uiScale);
    await setAppLocale(appSettings.value.locale);
  } finally {
    settingsSaving.value = false;
  }
}

async function cleanAllCaches() {
  if (allCacheCleaning.value) return;
  if (
    !window.confirm(
      "将删除所有服务的下载包和安装临时文件。已安装程序、配置、数据和备份不会被删除，确定继续吗？",
    )
  ) {
    return;
  }
  allCacheCleaning.value = true;
  notice.value = "";
  error.value = "";
  try {
    const result = await cleanAllInstallCache();
    await Promise.all([refreshDiskUsage(), refreshEnvironmentDiskUsage()]);
    notice.value = `已清理 ${result.removedItems} 项缓存，释放 ${formatBytes(result.freedBytes)}`;
  } catch (cause) {
    error.value = String(cause);
  } finally {
    allCacheCleaning.value = false;
  }
}

async function checkForUpdates() {
  if (updateChecking.value) return;
  updateChecking.value = true;
  try {
    updateStatus.value = await checkAppUpdate();
  } catch (cause) {
    updateStatus.value = {
      currentVersion: "0.1.0",
      latestVersion: null,
      updateAvailable: false,
      releaseUrl: null,
      message: String(cause),
    };
  } finally {
    updateChecking.value = false;
  }
}

async function stopAllServices() {
  const targets = runningServices.value;
  if (targets.length === 0 || stoppingAll.value) return;
  if (
    !window.confirm(
      `确定停止当前运行的 ${targets.length} 个服务吗？服务数据不会被删除。`,
    )
  ) {
    return;
  }

  stoppingAll.value = true;
  notice.value = "";
  error.value = "";
  let failed = 0;
  for (const service of targets) {
    try {
      await runServiceAction("stop", service.kind);
      recordActivity(service, "停止", true, "已通过全局概览停止");
    } catch (cause) {
      failed += 1;
      recordActivity(service, "停止", false, String(cause));
    }
  }
  await Promise.all([
    refreshServices(true),
    refreshEnvironmentMetrics(),
    refreshPortListeners(),
  ]);
  stoppingAll.value = false;
  if (failed > 0) {
    error.value = `${targets.length - failed} 个服务已停止，${failed} 个服务停止失败`;
  } else {
    notice.value = `已停止 ${targets.length} 个服务`;
  }
}

async function repairAbnormalServices() {
  if (repairingServices.value) return;
  const targets = services.value.filter(
    (service) =>
      service.status === "stale_pid" || service.status === "crashed",
  );
  if (targets.length === 0) return;
  repairingServices.value = true;
  notice.value = "";
  error.value = "";
  const failures: string[] = [];
  for (const service of targets) {
    try {
      const updated = await repairServiceState(service.kind);
      const index = services.value.findIndex(
        (item) => item.kind === updated.kind,
      );
      if (index >= 0) services.value[index] = updated;
      recordActivity(updated, "修复状态", true, "已清理异常运行记录");
    } catch (cause) {
      failures.push(`${service.name}: ${String(cause)}`);
      recordActivity(service, "修复状态", false, String(cause));
    }
  }
  repairingServices.value = false;
  await refreshServices(true);
  if (failures.length > 0) {
    error.value = `部分服务修复失败：${failures.join("；")}`;
  } else {
    notice.value = `已修复 ${targets.length} 个异常服务状态`;
  }
}

async function clearInstallCache() {
  const service = selectedService.value;
  if (!service || cacheCleaning.value) return;
  const cacheBytes = selectedDiskUsage.value?.cacheBytes ?? 0;
  if (cacheBytes === 0) {
    notice.value = t("serviceOverview.noCache", { service: service.name });
    return;
  }
  if (
    !window.confirm(
      t("serviceOverview.cleanConfirm", {
        service: service.name,
        size: formatBytes(cacheBytes),
      }),
    )
  ) {
    return;
  }
  cacheCleaning.value = true;
  try {
    const result = await cleanServiceCache(service.kind);
    await refreshDiskUsage(service.kind);
    notice.value = t("serviceOverview.cleaned", {
      count: result.removedItems,
      size: formatBytes(result.freedBytes),
    });
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    cacheCleaning.value = false;
  }
}

async function loadBackups() {
  const service = selectedService.value;
  if (!service || backupLoading.value) return;
  backupLoading.value = true;
  try {
    backups.value = await listServiceBackups(service.kind);
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    backupLoading.value = false;
  }
}

async function createBackup() {
  const service = selectedService.value;
  if (!service || backupCreating.value) return;
  if (service.status === "running") {
    error.value = t("backup.stopBeforeCreate", { service: service.name });
    return;
  }
  backupCreating.value = true;
  try {
    const backup = await createServiceBackup(service.kind);
    await Promise.all([loadBackups(), refreshDiskUsage(service.kind)]);
    notice.value = t("backup.created", {
      size: formatBytes(backup.sizeBytes),
    });
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    backupCreating.value = false;
  }
}

async function restoreBackup(backup: ServiceBackup) {
  const service = selectedService.value;
  if (!service || restoringBackupId.value) return;
  if (service.status === "running") {
    error.value = t("backup.stopBeforeRestore", { service: service.name });
    return;
  }
  if (
    !window.confirm(
      t("backup.restoreConfirm", {
        service: service.name,
        time: formatBackupDate(backup.createdAtMillis),
      }),
    )
  ) {
    return;
  }
  restoringBackupId.value = backup.id;
  try {
    const result = await restoreServiceBackup(service.kind, backup.id);
    await Promise.all([loadBackups(), refreshDiskUsage(service.kind)]);
    notice.value = t("backup.restored", { id: result.safetyBackup.id });
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    restoringBackupId.value = null;
  }
}

function selectTool(id: ToolId) {
  dashboardActive.value = false;
  settingsActive.value = false;
  if (id === "ssh") {
    sshToolMounted.value = true;
  }
  activeTool.value = id;
  notice.value = "";
  error.value = "";
}

async function selectService(kind: ServiceKind) {
  dashboardActive.value = false;
  settingsActive.value = false;
  activeTool.value = null;
  selectedKind.value = kind;
  activeTab.value = "overview";
  metricHistory.value = [];
  metrics.value = {
    running: false,
    cpuPercent: null,
    memoryBytes: null,
    uptime: null,
  };
  redisOverview.value = null;
  redisCursor.value = "0";
  redisKeys.value = [];
  selectedRedisKey.value = null;
  redisKeyDetail.value = null;
  databaseOverview.value = null;
  mongoOverview.value = null;
  mailpitOverview.value = null;
  natsOverview.value = null;
  kafkaOverview.value = null;
  kafkaTopics.value = [];
  meilisearchOverview.value = null;
  meilisearchIndexes.value = [];
  mailMessages.value = [];
  selectedMailId.value = null;
  mailDetail.value = null;
  backups.value = [];
  mongoDatabases.value = [];
  selectedMongoDatabase.value = "";
  mongoCollections.value = [];
  selectedMongoCollection.value = null;
  mongoCollectionDetail.value = null;
  databases.value = [];
  selectedDatabase.value = "";
  tables.value = [];
  selectedTable.value = null;
  tableDetail.value = null;
  managedVersions.value = [];
  managedVersionTarget.value = "";
  await Promise.all([
    refreshMetrics(),
    refreshDiskUsage(kind),
  ]);
}

async function loadRedisVersions() {
  if (redisVersionsLoading.value) return;
  redisVersionsLoading.value = true;
  try {
    redisVersions.value = await listRedisVersions();
    redisVersionTarget.value =
      redisVersions.value.find((release) => release.selected)?.version ??
      selectedService.value?.version ??
      "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    redisVersionsLoading.value = false;
  }
}

async function changeRedisVersion() {
  const service = selectedService.value;
  const target = selectedRedisVersionInfo.value;
  if (
    !service ||
    service.kind !== "redis" ||
    !target ||
    target.selected ||
    serviceControlBusy.value
  ) {
    return;
  }
  if (service.status === "running") {
    error.value = t("versions.stopSwitch", { service: "Redis" });
    return;
  }
  if (
    !window.confirm(
      t("versions.redisConfirm", { version: target.version }),
    )
  ) {
    return;
  }

  redisVersionChanging.value = true;
  notice.value = "";
  error.value = "";
  const operationId = startInstallTask(
    "redis",
    `Redis ${target.version}`,
  );
  try {
    const wasInstalled = target.installed;
    const updated = await selectRedisVersion(target.version, operationId);
    recordInstallSuccess(operationId);
    const index = services.value.findIndex(
      (item) => item.kind === updated.kind,
    );
    if (index >= 0) services.value[index] = updated;
    await Promise.all([
      loadRedisVersions(),
      refreshDiskUsage("redis"),
    ]);
    redisOverview.value = null;
    notice.value = wasInstalled
      ? t("versions.switched", { service: "Redis", version: target.version })
      : t("versions.installedSwitched", { service: "Redis", version: target.version });
    recordActivity(
      updated,
      wasInstalled ? t("versions.activitySwitch") : t("versions.activityInstall"),
      true,
      notice.value,
    );
  } catch (cause) {
    recordInstallFailure(operationId, cause);
    recordActivity(service, t("versions.activitySwitch"), false, String(cause));
    error.value = String(cause);
  } finally {
    redisVersionChanging.value = false;
  }
}

async function loadMysqlVersions() {
  if (mysqlVersionsLoading.value) return;
  mysqlVersionsLoading.value = true;
  try {
    mysqlVersions.value = await listMysqlVersions();
    mysqlVersionTarget.value =
      mysqlVersions.value.find((release) => release.selected)?.version ??
      selectedService.value?.version ??
      "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    mysqlVersionsLoading.value = false;
  }
}

async function changeMysqlVersion() {
  const service = selectedService.value;
  const target = selectedMysqlVersionInfo.value;
  if (
    !service ||
    service.kind !== "mysql" ||
    !target ||
    target.selected ||
    serviceControlBusy.value
  ) {
    return;
  }
  if (service.status === "running") {
    error.value = t("versions.stopSwitch", { service: "MySQL" });
    return;
  }
  if (
    !window.confirm(
      t("versions.databaseConfirm", { service: "MySQL", version: target.version }),
    )
  ) {
    return;
  }

  mysqlVersionChanging.value = true;
  notice.value = "";
  error.value = "";
  const operationId = startInstallTask(
    "mysql",
    `MySQL ${target.version}`,
  );
  try {
    const wasInstalled = target.installed;
    const updated = await selectMysqlVersion(target.version, operationId);
    recordInstallSuccess(operationId);
    const index = services.value.findIndex(
      (item) => item.kind === updated.kind,
    );
    if (index >= 0) services.value[index] = updated;
    await Promise.all([
      loadMysqlVersions(),
      refreshDiskUsage("mysql"),
    ]);
    databaseOverview.value = null;
    databases.value = [];
    selectedDatabase.value = "";
    tables.value = [];
    selectedTable.value = null;
    notice.value = wasInstalled
      ? t("versions.switched", { service: "MySQL", version: target.version })
      : t("versions.installedSwitched", { service: "MySQL", version: target.version });
    recordActivity(
      updated,
      wasInstalled ? t("versions.activitySwitch") : t("versions.activityInstall"),
      true,
      notice.value,
    );
  } catch (cause) {
    recordInstallFailure(operationId, cause);
    recordActivity(service, t("versions.activitySwitch"), false, String(cause));
    error.value = String(cause);
  } finally {
    mysqlVersionChanging.value = false;
  }
}

async function loadPostgresVersions() {
  if (postgresVersionsLoading.value) return;
  postgresVersionsLoading.value = true;
  try {
    postgresVersions.value = await listPostgresVersions();
    postgresVersionTarget.value =
      postgresVersions.value.find((release) => release.selected)?.version ??
      selectedService.value?.version ??
      "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    postgresVersionsLoading.value = false;
  }
}

async function changePostgresVersion() {
  const service = selectedService.value;
  const target = selectedPostgresVersionInfo.value;
  if (
    !service ||
    service.kind !== "postgres" ||
    !target ||
    target.selected ||
    serviceControlBusy.value
  ) {
    return;
  }
  if (service.status === "running") {
    error.value = t("versions.stopSwitch", { service: "PostgreSQL" });
    return;
  }
  if (
    !window.confirm(
      t("versions.databaseConfirm", { service: "PostgreSQL", version: target.version }),
    )
  ) {
    return;
  }

  postgresVersionChanging.value = true;
  notice.value = "";
  error.value = "";
  const operationId = startInstallTask(
    "postgres",
    `PostgreSQL ${target.version}`,
  );
  try {
    const wasInstalled = target.installed;
    const updated = await selectPostgresVersion(
      target.version,
      operationId,
    );
    recordInstallSuccess(operationId);
    const index = services.value.findIndex(
      (item) => item.kind === updated.kind,
    );
    if (index >= 0) services.value[index] = updated;
    await Promise.all([
      loadPostgresVersions(),
      refreshDiskUsage("postgres"),
    ]);
    databaseOverview.value = null;
    databases.value = [];
    selectedDatabase.value = "";
    tables.value = [];
    selectedTable.value = null;
    notice.value = wasInstalled
      ? t("versions.switched", { service: "PostgreSQL", version: target.version })
      : t("versions.compiledSwitched", { service: "PostgreSQL", version: target.version });
    recordActivity(
      updated,
      wasInstalled ? t("versions.activitySwitch") : t("versions.activityInstall"),
      true,
      notice.value,
    );
  } catch (cause) {
    recordInstallFailure(operationId, cause);
    recordActivity(service, t("versions.activitySwitch"), false, String(cause));
    error.value = String(cause);
  } finally {
    postgresVersionChanging.value = false;
  }
}

function installedVersionsFor(
  kind: ServiceKind,
): ManagedVersionInfo[] {
  if (kind === "redis") return redisVersions.value;
  if (kind === "mysql") return mysqlVersions.value;
  if (kind === "postgres") return postgresVersions.value;
  if (kind === "nginx") return nginxVersions.value;
  if (genericMultiVersionKinds.includes(kind)) return managedVersions.value;
  return [];
}

function requestVersionUninstall(
  kind: VersionUninstallTarget["kind"],
  serviceName: string,
  release: ManagedVersionInfo,
) {
  const service = selectedService.value;
  if (
    versionUninstalling.value ||
    !release.installed ||
    !service ||
    service.kind !== kind
  ) {
    return;
  }
  if (release.selected && service.status === "running") {
    error.value = t("versions.stopUninstall", { service: serviceName });
    return;
  }
  const fallback =
    installedVersionsFor(kind).find(
      (candidate) =>
        candidate.installed &&
        candidate.version !== release.version &&
        candidate.recommended,
    ) ??
    installedVersionsFor(kind).find(
      (candidate) =>
        candidate.installed && candidate.version !== release.version,
    );
  versionUninstallTarget.value = {
    kind,
    serviceName,
    release,
    fallbackVersion: release.selected ? (fallback?.version ?? null) : null,
  };
}

function requestCurrentProgramUninstall() {
  const service = selectedService.value;
  const usage = selectedDiskUsage.value;
  if (
    !service ||
    !usage ||
    service.status === "not_installed" ||
    usage.installationBytes === 0
  ) {
    return;
  }
  const knownRelease = installedVersionsFor(service.kind).find(
    (release) => release.version === service.version,
  );
  requestVersionUninstall(
    service.kind,
    service.name,
    knownRelease ?? {
      series: service.version,
      version: service.version,
      installed: true,
      selected: true,
      supportLabel: "",
      legacy: false,
      recommended: false,
      installationBytes: usage.installationBytes,
    },
  );
}

async function confirmVersionUninstall() {
  const target = versionUninstallTarget.value;
  const service = selectedService.value;
  if (!target || !service || versionUninstalling.value) return;
  versionUninstalling.value = true;
  error.value = "";
  notice.value = "";
  try {
    const result: VersionUninstallResult = await uninstallServiceVersion(
      target.kind,
      target.release.version,
    );
    const index = services.value.findIndex(
      (item) => item.kind === result.service.kind,
    );
    if (index >= 0) services.value[index] = result.service;
    versionUninstallTarget.value = null;
    const versionRefresh =
      target.kind === "redis"
        ? loadRedisVersions()
        : target.kind === "mysql"
          ? loadMysqlVersions()
          : target.kind === "postgres"
            ? loadPostgresVersions()
            : target.kind === "nginx"
              ? loadNginxVersions()
              : genericMultiVersionKinds.includes(target.kind)
                ? loadManagedVersions()
                : Promise.resolve();
    await Promise.all([
      versionRefresh,
      refreshDiskUsage(target.kind),
      refreshEnvironmentDiskUsage(),
    ]);
    notice.value = result.fallbackVersion
      ? t("versions.uninstallFallback", {
          service: target.serviceName,
          version: result.version,
          fallback: result.fallbackVersion,
        })
      : t("versions.uninstalled", {
          service: target.serviceName,
          version: result.version,
          size: formatBytes(result.freedBytes),
        });
    recordActivity(
      result.service,
      t("versions.activityUninstall"),
      true,
      notice.value,
    );
  } catch (cause) {
    error.value = String(cause);
    recordActivity(service, t("versions.activityUninstall"), false, error.value);
  } finally {
    versionUninstalling.value = false;
  }
}

async function loadNginxVersions() {
  if (nginxVersionsLoading.value) return;
  nginxVersionsLoading.value = true;
  try {
    nginxVersions.value = await listNginxVersions();
    nginxVersionTarget.value =
      nginxVersions.value.find((release) => release.selected)?.version ??
      selectedService.value?.version ??
      "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    nginxVersionsLoading.value = false;
  }
}

async function changeNginxVersion() {
  const service = selectedService.value;
  const target = selectedNginxVersionInfo.value;
  if (
    !service ||
    !target ||
    target.selected ||
    serviceControlBusy.value
  ) {
    return;
  }
  if (service.status === "running") {
    error.value = t("versions.stopSwitch", { service: "Nginx" });
    return;
  }
  nginxVersionChanging.value = true;
  notice.value = "";
  error.value = "";
  const operationId = startInstallTask("nginx", `Nginx ${target.version}`);
  try {
    const wasInstalled = target.installed;
    const updated = await selectNginxVersion(target.version, operationId);
    recordInstallSuccess(operationId);
    const index = services.value.findIndex(
      (item) => item.kind === updated.kind,
    );
    if (index >= 0) services.value[index] = updated;
    await Promise.all([
      loadNginxVersions(),
      refreshDiskUsage("nginx"),
      refreshEnvironmentDiskUsage(),
    ]);
    notice.value = wasInstalled
      ? t("versions.switched", { service: "Nginx", version: target.version })
      : t("versions.compiledSwitched", { service: "Nginx", version: target.version });
    recordActivity(
      updated,
      wasInstalled ? t("versions.activitySwitch") : t("versions.activityInstall"),
      true,
      notice.value,
    );
  } catch (cause) {
    recordInstallFailure(operationId, cause);
    error.value = String(cause);
    recordActivity(service, t("versions.activitySwitch"), false, error.value);
  } finally {
    nginxVersionChanging.value = false;
  }
}

async function loadManagedVersions() {
  const service = selectedService.value;
  if (
    !service ||
    !genericMultiVersionKinds.includes(service.kind) ||
    managedVersionsLoading.value
  ) {
    return;
  }
  managedVersionsLoading.value = true;
  try {
    managedVersions.value = await listManagedServiceVersions(service.kind);
    managedVersionTarget.value =
      managedVersions.value.find((release) => release.selected)?.version ??
      service.version;
  } catch (cause) {
    error.value = String(cause);
  } finally {
    managedVersionsLoading.value = false;
  }
}

async function changeManagedVersion() {
  const service = selectedService.value;
  const target = selectedManagedVersionInfo.value;
  if (
    !service ||
    !target ||
    target.selected ||
    serviceControlBusy.value ||
    !genericMultiVersionKinds.includes(service.kind)
  ) {
    return;
  }
  if (service.status === "running") {
    error.value = t("versions.stopSwitch", { service: service.name });
    return;
  }
  managedVersionChanging.value = true;
  notice.value = "";
  error.value = "";
  const operationId = startInstallTask(
    service.kind,
    `${service.name} ${target.version}`,
  );
  try {
    const wasInstalled = target.installed;
    const updated = await selectManagedServiceVersion(
      service.kind,
      target.version,
      operationId,
    );
    recordInstallSuccess(operationId);
    const index = services.value.findIndex(
      (item) => item.kind === updated.kind,
    );
    if (index >= 0) services.value[index] = updated;
    await Promise.all([
      loadManagedVersions(),
      refreshDiskUsage(service.kind),
      refreshEnvironmentDiskUsage(),
    ]);
    notice.value = wasInstalled
      ? t("versions.switched", { service: service.name, version: target.version })
      : t("versions.installedSwitched", { service: service.name, version: target.version });
    recordActivity(
      updated,
      wasInstalled ? t("versions.activitySwitch") : t("versions.activityInstall"),
      true,
      notice.value,
    );
  } catch (cause) {
    recordInstallFailure(operationId, cause);
    error.value = String(cause);
    recordActivity(service, t("versions.activitySwitch"), false, error.value);
  } finally {
    managedVersionChanging.value = false;
  }
}

async function execute(action: ServiceAction) {
  const service = selectedService.value;
  if (!service || serviceControlBusy.value) return;

  const lifecycleToastKey = `service:${service.kind}:lifecycle`;
  pendingAction.value = action;
  notice.value = "";
  error.value = "";
  const operationId =
    action === "install"
      ? startInstallTask(service.kind, service.name)
      : undefined;
  const activityAction = {
    install: "安装",
    start: "启动",
    stop: "停止",
    restart: "重启",
  }[action];
  if (action !== "install") {
    showToast({
      key: lifecycleToastKey,
      intent: "progress",
      title: t(`serviceToast.${action}Progress`, { service: service.name }),
      duration: 0,
    });
  }
  try {
    const updated = await runServiceAction(
      action,
      service.kind,
      operationId,
    );
    if (operationId) recordInstallSuccess(operationId);
    const index = services.value.findIndex(
      (item) => item.kind === updated.kind,
    );
    if (index >= 0) services.value[index] = updated;
    const successMessage = t(`serviceToast.${action}Success`, {
      service: service.name,
    });
    showToast({
      key: lifecycleToastKey,
      intent: "success",
      title: successMessage,
    });
    recordActivity(updated, activityAction, true, successMessage);
    databaseOverview.value = null;
    mongoOverview.value = null;
    mailpitOverview.value = null;
    natsOverview.value = null;
    meilisearchOverview.value = null;
    await Promise.all([
      refreshMetrics(),
      refreshDiskUsage(service.kind),
      service.kind === "redis" && activeTab.value === "versions"
        ? loadRedisVersions()
        : service.kind === "mysql" && activeTab.value === "versions"
          ? loadMysqlVersions()
          : service.kind === "postgres" && activeTab.value === "versions"
            ? loadPostgresVersions()
            : service.kind === "nginx" && activeTab.value === "versions"
              ? loadNginxVersions()
            : genericMultiVersionKinds.includes(service.kind) &&
                activeTab.value === "versions"
              ? loadManagedVersions()
            : Promise.resolve(),
    ]);
  } catch (cause) {
    const message = String(cause);
    if (
      action === "stop" &&
      message.includes("did not stop within") &&
      window.confirm(
        `${service.name} 未能在正常停止时限内退出，是否强制停止？\n\n强制停止可能导致尚未落盘的数据丢失。`,
      )
    ) {
      try {
        const updated = await forceStopService(service.kind);
        const index = services.value.findIndex(
          (item) => item.kind === updated.kind,
        );
        if (index >= 0) services.value[index] = updated;
        const successMessage = t("serviceToast.forceStopSuccess", {
          service: service.name,
        });
        showToast({
          key: lifecycleToastKey,
          intent: "success",
          title: successMessage,
        });
        recordActivity(updated, "强制停止", true, successMessage);
        await Promise.all([
          refreshMetrics(),
          refreshEnvironmentMetrics(),
          refreshPortListeners(),
        ]);
        return;
      } catch (forceCause) {
        const forceMessage = String(forceCause);
        showToast({
          key: lifecycleToastKey,
          intent: "error",
          title: t("serviceToast.forceStopFailed", { service: service.name }),
          message: forceMessage,
          duration: 0,
        });
        recordActivity(service, "强制停止", false, forceMessage);
        return;
      }
    }
    if (operationId) recordInstallFailure(operationId, cause);
    recordActivity(service, activityAction, false, message);
    if (action === "install") {
      dismissToastByKey(lifecycleToastKey);
      error.value = message;
    } else {
      showToast({
        key: lifecycleToastKey,
        intent: "error",
        title: t(`serviceToast.${action}Failed`, { service: service.name }),
        message,
        duration: 0,
      });
    }
    if (action === "start" && message.includes("最后 50 行日志")) {
      activeTab.value = "logs";
      await loadLogs();
    }
  } finally {
    pendingAction.value = null;
  }
}

async function openTab(tab: DetailTab) {
  activeTab.value = tab;
  if (tab === "keys" && redisKeys.value.length === 0) {
    await loadRedisKeys(true);
  }
  if (
    (tab === "data" || tab === "sql") &&
    selectedKind.value !== "mongodb" &&
    databases.value.length === 0
  ) {
    await loadDatabaseCatalog();
  }
  if (
    (tab === "data" || tab === "mongoConsole") &&
    selectedKind.value === "mongodb" &&
    mongoDatabases.value.length === 0
  ) {
    await loadMongoCatalog();
  }
  if (tab === "config") await loadConfig();
  if (tab === "logs") await loadLogs();
  if (tab === "versions" && selectedKind.value === "redis") {
    await loadRedisVersions();
  }
  if (tab === "versions" && selectedKind.value === "mysql") {
    await loadMysqlVersions();
  }
  if (tab === "versions" && selectedKind.value === "postgres") {
    await loadPostgresVersions();
  }
  if (tab === "versions" && selectedKind.value === "nginx") {
    await loadNginxVersions();
  }
  if (
    tab === "versions" &&
    genericMultiVersionKinds.includes(selectedKind.value)
  ) {
    await loadManagedVersions();
  }
  if (tab === "mail" && mailMessages.value.length === 0) {
    await loadMailMessages();
  }
  if (tab === "messages" && selectedKind.value === "kafka") {
    await loadKafkaTopics();
  }
  if (tab === "backup") await loadBackups();
  if (tab === "search" && meilisearchIndexes.value.length === 0) {
    await loadMeilisearchIndexes();
  }
}

async function loadMeilisearchIndexes() {
  if (
    selectedKind.value !== "meilisearch" ||
    selectedService.value?.status !== "running" ||
    meilisearchLoading.value
  ) {
    return;
  }
  meilisearchLoading.value = true;
  try {
    meilisearchIndexes.value = await listMeilisearchIndexes();
    if (
      meilisearchIndexes.value.length > 0 &&
      !meilisearchIndexes.value.some(
        (index) => index.uid === meilisearchIndex.value,
      )
    ) {
      meilisearchIndex.value = meilisearchIndexes.value[0].uid;
    }
  } catch (cause) {
    error.value = String(cause);
  } finally {
    meilisearchLoading.value = false;
  }
}

async function importMeilisearchDocuments() {
  if (meilisearchImporting.value) return;
  meilisearchImporting.value = true;
  error.value = "";
  try {
    const task = await addMeilisearchDocuments(
      meilisearchIndex.value,
      meilisearchPrimaryKey.value,
      meilisearchDocuments.value,
    );
    notice.value = `文档导入任务 #${task.taskUid} 已进入队列`;
    window.setTimeout(() => void loadMeilisearchIndexes(), 500);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    meilisearchImporting.value = false;
  }
}

async function runMeilisearchSearch() {
  if (meilisearchSearching.value || !meilisearchIndex.value) return;
  meilisearchSearching.value = true;
  error.value = "";
  try {
    meilisearchResult.value = await searchMeilisearch(
      meilisearchIndex.value,
      meilisearchQuery.value,
    );
  } catch (cause) {
    meilisearchResult.value = null;
    error.value = String(cause);
  } finally {
    meilisearchSearching.value = false;
  }
}

async function publishNats() {
  if (
    selectedKind.value !== "nats" ||
    selectedService.value?.status !== "running" ||
    natsPublishing.value
  ) {
    return;
  }
  natsPublishing.value = true;
  error.value = "";
  try {
    natsPublishResult.value = await publishNatsMessage(
      natsPublishSubject.value,
      natsPublishPayload.value,
    );
    notice.value = `消息已发布到 ${natsPublishResult.value.subject}`;
    await refreshMetrics();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    natsPublishing.value = false;
  }
}

async function receiveNats() {
  if (
    selectedKind.value !== "nats" ||
    selectedService.value?.status !== "running" ||
    natsReceiving.value
  ) {
    return;
  }
  natsReceiving.value = true;
  natsMessage.value = null;
  notice.value = "正在等待一条匹配的 NATS 消息，最多等待 8 秒";
  error.value = "";
  try {
    natsMessage.value = await receiveNatsMessage(natsSubscribeSubject.value);
    notice.value = `已收到 ${natsMessage.value.subject}`;
  } catch (cause) {
    error.value = String(cause);
    notice.value = "";
  } finally {
    natsReceiving.value = false;
  }
}

async function loadKafkaTopics() {
  if (
    selectedKind.value !== "kafka" ||
    selectedService.value?.status !== "running" ||
    kafkaLoading.value
  ) {
    return;
  }
  kafkaLoading.value = true;
  error.value = "";
  try {
    kafkaTopics.value = await listKafkaTopics();
    if (
      kafkaSelectedTopic.value &&
      !kafkaTopics.value.some((topic) => topic.name === kafkaSelectedTopic.value)
    ) {
      kafkaSelectedTopic.value = "";
    }
    kafkaSelectedTopic.value ||= kafkaTopics.value[0]?.name ?? "";
    kafkaOverview.value = await getKafkaOverview();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    kafkaLoading.value = false;
  }
}

async function addKafkaTopic() {
  if (kafkaLoading.value || !kafkaTopicName.value.trim()) return;
  kafkaLoading.value = true;
  error.value = "";
  try {
    kafkaTopics.value = await createKafkaTopic(
      kafkaTopicName.value.trim(),
      kafkaPartitions.value,
    );
    kafkaSelectedTopic.value = kafkaTopicName.value.trim();
    notice.value = `主题 ${kafkaSelectedTopic.value} 已创建`;
    kafkaOverview.value = await getKafkaOverview();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    kafkaLoading.value = false;
  }
}

async function removeKafkaTopic(name: string) {
  if (
    kafkaLoading.value ||
    !window.confirm(`确定删除 Kafka 主题 ${name} 吗？主题内消息会一并删除。`)
  ) {
    return;
  }
  kafkaLoading.value = true;
  error.value = "";
  try {
    kafkaTopics.value = await deleteKafkaTopic(name);
    if (kafkaSelectedTopic.value === name) {
      kafkaSelectedTopic.value = kafkaTopics.value[0]?.name ?? "";
    }
    notice.value = `主题 ${name} 已删除`;
    kafkaOverview.value = await getKafkaOverview();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    kafkaLoading.value = false;
  }
}

async function publishKafka() {
  if (
    !kafkaSelectedTopic.value ||
    kafkaPublishing.value ||
    selectedService.value?.status !== "running"
  ) {
    return;
  }
  kafkaPublishing.value = true;
  error.value = "";
  try {
    kafkaPublishResult.value = await publishKafkaMessage(
      kafkaSelectedTopic.value,
      kafkaMessageKey.value,
      kafkaMessagePayload.value,
    );
    notice.value = `测试消息已发送到 ${kafkaSelectedTopic.value}`;
  } catch (cause) {
    error.value = String(cause);
  } finally {
    kafkaPublishing.value = false;
  }
}

async function loadMailMessages() {
  if (
    selectedKind.value !== "mailpit" ||
    selectedService.value?.status !== "running" ||
    mailLoading.value
  ) {
    return;
  }
  mailLoading.value = true;
  try {
    mailMessages.value = await listMailpitMessages();
    if (
      selectedMailId.value &&
      !mailMessages.value.some((message) => message.id === selectedMailId.value)
    ) {
      selectedMailId.value = null;
      mailDetail.value = null;
    }
    mailpitOverview.value = await getMailpitOverview();
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    mailLoading.value = false;
  }
}

async function selectMail(message: MailSummary) {
  selectedMailId.value = message.id;
  mailDetailLoading.value = true;
  try {
    mailDetail.value = await getMailpitMessageDetail(message.id);
    error.value = "";
  } catch (cause) {
    mailDetail.value = null;
    error.value = String(cause);
  } finally {
    mailDetailLoading.value = false;
  }
}

async function loadRedisKeys(reset = false) {
  if (
    selectedKind.value !== "redis" ||
    selectedService.value?.status !== "running" ||
    redisKeysLoading.value
  ) {
    return;
  }
  redisKeysLoading.value = true;
  try {
    const cursor = reset ? "0" : redisCursor.value;
    const result = await scanRedisKeys(
      redisDatabase.value,
      cursor,
      redisPattern.value.trim() || "*",
    );
    redisCursor.value = result.nextCursor;
    redisKeys.value = reset
      ? result.keys
      : [...new Set([...redisKeys.value, ...result.keys])];
    if (reset) {
      selectedRedisKey.value = null;
      redisKeyDetail.value = null;
    }
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    redisKeysLoading.value = false;
  }
}

async function selectRedisKey(key: string) {
  selectedRedisKey.value = key;
  redisKeyLoading.value = true;
  try {
    redisKeyDetail.value = await getRedisKeyDetail(
      redisDatabase.value,
      key,
    );
    error.value = "";
  } catch (cause) {
    redisKeyDetail.value = null;
    error.value = String(cause);
  } finally {
    redisKeyLoading.value = false;
  }
}

function parseCommand(input: string) {
  const arguments_: string[] = [];
  const pattern = /"((?:\\.|[^"])*)"|'((?:\\.|[^'])*)'|(\S+)/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(input)) !== null) {
    const value = match[1] ?? match[2] ?? match[3] ?? "";
    arguments_.push(value.replace(/\\(["'\\])/g, "$1"));
  }
  return arguments_;
}

async function runConsoleCommand(confirmed = false) {
  const command = consoleInput.value.trim();
  if (!command || consoleRunning.value) return;
  const arguments_ = parseCommand(command);
  consoleRunning.value = true;
  try {
    const result = await executeRedisCommand(
      redisDatabase.value,
      arguments_,
      confirmed,
    );
    consoleHistory.value.push({
      database: redisDatabase.value,
      command,
      output: result.output.trimEnd() || "(nil)",
      elapsedMs: result.elapsedMs,
      error: false,
    });
    consoleInput.value = "";
    if (consoleHistory.value.length > 50) consoleHistory.value.shift();
  } catch (cause) {
    const message = String(cause);
    if (
      message.includes("CONFIRM_REQUIRED:") &&
      window.confirm(t("console.redisConfirm"))
    ) {
      consoleRunning.value = false;
      await runConsoleCommand(true);
      return;
    }
    consoleHistory.value.push({
      database: redisDatabase.value,
      command,
      output: message,
      elapsedMs: 0,
      error: true,
    });
  } finally {
    consoleRunning.value = false;
  }
}

async function changeRedisDatabase() {
  redisCursor.value = "0";
  redisKeys.value = [];
  selectedRedisKey.value = null;
  redisKeyDetail.value = null;
  if (activeTab.value === "keys") await loadRedisKeys(true);
}

function sqlKind(): SqlServiceKind | null {
  return selectedKind.value === "mysql" || selectedKind.value === "postgres"
    ? selectedKind.value
    : null;
}

async function loadMongoCatalog() {
  if (
    selectedKind.value !== "mongodb" ||
    selectedService.value?.status !== "running" ||
    mongoLoading.value
  ) {
    return;
  }
  mongoLoading.value = true;
  try {
    mongoDatabases.value = await listMongoDatabases();
    if (
      !mongoDatabases.value.some(
        (database) => database.name === selectedMongoDatabase.value,
      )
    ) {
      selectedMongoDatabase.value =
        mongoDatabases.value.find((database) => !database.system)?.name ??
        mongoDatabases.value[0]?.name ??
        "";
    }
    await loadMongoCollections();
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    mongoLoading.value = false;
  }
}

async function loadMongoCollections() {
  if (!selectedMongoDatabase.value) {
    mongoCollections.value = [];
    return;
  }
  mongoDetailLoading.value = true;
  try {
    mongoCollections.value = await listMongoCollections(
      selectedMongoDatabase.value,
    );
    selectedMongoCollection.value = null;
    mongoCollectionDetail.value = null;
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    mongoDetailLoading.value = false;
  }
}

async function selectMongoCollection(collection: MongoCollectionInfo) {
  if (!selectedMongoDatabase.value) return;
  selectedMongoCollection.value = collection;
  mongoDetailLoading.value = true;
  try {
    mongoCollectionDetail.value = await getMongoCollectionDetail(
      selectedMongoDatabase.value,
      collection.name,
    );
    error.value = "";
  } catch (cause) {
    mongoCollectionDetail.value = null;
    error.value = String(cause);
  } finally {
    mongoDetailLoading.value = false;
  }
}

async function changeMongoDatabase() {
  await loadMongoCollections();
}

async function runMongoCommand(confirmed = false) {
  const command = mongoCommandInput.value.trim();
  if (
    !selectedMongoDatabase.value ||
    !command ||
    mongoCommandRunning.value
  ) {
    return;
  }
  mongoCommandRunning.value = true;
  try {
    const result = await executeMongoCommand(
      selectedMongoDatabase.value,
      command,
      confirmed,
    );
    mongoCommandHistory.value.push({
      database: selectedMongoDatabase.value,
      command,
      output: result.output,
      elapsedMs: result.elapsedMs,
      error: "",
    });
    if (mongoCommandHistory.value.length > 30) {
      mongoCommandHistory.value.shift();
    }
    mongoOverview.value = null;
  } catch (cause) {
    const message = String(cause);
    if (
      message.includes("需要确认后执行") &&
      window.confirm(t("console.mongoConfirm"))
    ) {
      mongoCommandRunning.value = false;
      await runMongoCommand(true);
      return;
    }
    mongoCommandHistory.value.push({
      database: selectedMongoDatabase.value,
      command,
      output: null,
      elapsedMs: 0,
      error: message,
    });
  } finally {
    mongoCommandRunning.value = false;
  }
}

async function loadDatabaseCatalog() {
  const kind = sqlKind();
  if (
    !kind ||
    selectedService.value?.status !== "running" ||
    databaseLoading.value
  ) {
    return;
  }
  databaseLoading.value = true;
  try {
    databases.value = await listDatabases(kind);
    if (
      !databases.value.some(
        (database) => database.name === selectedDatabase.value,
      )
    ) {
      selectedDatabase.value =
        databases.value.find((database) => !database.system)?.name ??
        databases.value[0]?.name ??
        "";
    }
    await loadTables();
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    databaseLoading.value = false;
  }
}

async function loadTables() {
  const kind = sqlKind();
  if (!kind || !selectedDatabase.value) return;
  tableLoading.value = true;
  try {
    tables.value = await listDatabaseTables(
      kind,
      selectedDatabase.value,
    );
    selectedTable.value = null;
    tableDetail.value = null;
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    tableLoading.value = false;
  }
}

async function selectTable(table: TableInfo) {
  const kind = sqlKind();
  if (!kind || !selectedDatabase.value) return;
  selectedTable.value = table;
  tableLoading.value = true;
  try {
    tableDetail.value = await getTableDetail(
      kind,
      selectedDatabase.value,
      table.schema,
      table.name,
    );
    error.value = "";
  } catch (cause) {
    tableDetail.value = null;
    error.value = String(cause);
  } finally {
    tableLoading.value = false;
  }
}

async function changeSqlDatabase() {
  await loadTables();
}

async function runSqlCommand(confirmed = false) {
  const kind = sqlKind();
  const sql = sqlInput.value.trim();
  if (
    !kind ||
    !selectedDatabase.value ||
    !sql ||
    sqlRunning.value
  ) {
    return;
  }
  sqlRunning.value = true;
  try {
    const result = await executeSql(
      kind,
      selectedDatabase.value,
      sql,
      confirmed,
    );
    sqlHistory.value.push({
      database: selectedDatabase.value,
      sql,
      result,
      error: "",
    });
    if (sqlHistory.value.length > 30) sqlHistory.value.shift();
    databaseOverview.value = null;
  } catch (cause) {
    const message = String(cause);
    if (
      message.includes("CONFIRM_REQUIRED:") &&
      window.confirm(t("console.sqlConfirm"))
    ) {
      sqlRunning.value = false;
      await runSqlCommand(true);
      return;
    }
    sqlHistory.value.push({
      database: selectedDatabase.value,
      sql,
      result: null,
      error: message,
    });
  } finally {
    sqlRunning.value = false;
  }
}

function openSqlAiAssistant() {
  const latest = sqlHistory.value.at(-1);
  aiAssistTitle.value = `${selectedService.value?.name ?? "Database"} AI 助手`;
  aiAssistOptions.value = [
    { id: "database_sql", label: "生成 SQL", hint: "描述希望查询或处理的数据", canApply: true },
    { id: "database_explain", label: "分析与优化", hint: "分析当前 SQL 或粘贴 EXPLAIN 结果" },
    { id: "database_error", label: "修复错误", hint: "说明遇到的问题，AI 会结合最近一次错误诊断" },
  ];
  aiAssistContext.value = JSON.stringify({
    engine: selectedKind.value,
    database: selectedDatabase.value,
    databases: databases.value.map((item) => item.name),
    tables: tables.value.map((item) => item.name),
    selectedTable: tableDetail.value
      ? {
          name: selectedTable.value?.name,
          schema: selectedTable.value?.schema,
          columns: tableDetail.value.columns.map((column) => ({
            name: column.name,
            type: column.dataType,
            nullable: column.nullable,
            key: column.key,
          })),
        }
      : null,
    currentSql: sqlInput.value,
    latestResult: latest
      ? { sql: latest.sql, error: latest.error, summary: latest.result?.summary }
      : null,
  }, null, 2);
  aiAssistOpen.value = true;
}

function openRedisAiAssistant() {
  const latest = consoleHistory.value.at(-1);
  aiAssistTitle.value = "Redis AI 助手";
  aiAssistOptions.value = [
    { id: "redis_command", label: "生成命令", hint: "描述希望完成的 Redis 操作", canApply: true },
    { id: "redis_analysis", label: "分析与建议", hint: "分析当前 Key、命令输出、内存、TTL 或慢查询问题" },
  ];
  aiAssistContext.value = JSON.stringify({
    database: redisDatabase.value,
    currentCommand: consoleInput.value,
    selectedKey: redisKeyDetail.value,
    overview: redisOverview.value,
    latestCommand: latest ?? null,
  }, null, 2);
  aiAssistOpen.value = true;
}

function openMongoAiAssistant() {
  const latest = mongoCommandHistory.value.at(-1);
  aiAssistTitle.value = "MongoDB AI 助手";
  aiAssistOptions.value = [
    { id: "mongodb_command", label: "生成命令", hint: "描述希望查询、聚合或修改的文档", canApply: true },
    { id: "mongodb_analysis", label: "分析与优化", hint: "分析当前命令、聚合管道、索引或最近错误" },
  ];
  aiAssistContext.value = JSON.stringify({
    database: selectedMongoDatabase.value,
    collection: selectedMongoCollection.value?.name ?? null,
    collectionDetail: mongoCollectionDetail.value,
    currentCommand: mongoCommandInput.value,
    latestCommand: latest ?? null,
  }, null, 2);
  aiAssistOpen.value = true;
}

function openMessageAiAssistant() {
  const broker = selectedKind.value;
  aiAssistTitle.value = `${selectedService.value?.name ?? "消息系统"} AI 设计助手`;
  aiAssistOptions.value = [{
    id: "message_design",
    label: "设计消息",
    hint: "描述业务事件、生产者、消费者和期望的消息字段",
    canApply: broker === "nats" || broker === "kafka",
  }];
  aiAssistContext.value = JSON.stringify({
    broker,
    nats: broker === "nats" ? {
      publishSubject: natsPublishSubject.value,
      subscribeSubject: natsSubscribeSubject.value,
      payload: natsPublishPayload.value,
      overview: natsOverview.value,
    } : null,
    kafka: broker === "kafka" ? {
      topics: kafkaTopics.value,
      selectedTopic: kafkaSelectedTopic.value,
      draftTopic: kafkaTopicName.value,
      key: kafkaMessageKey.value,
      payload: kafkaMessagePayload.value,
      overview: kafkaOverview.value,
    } : null,
    rabbitmq: broker === "rabbitmq" ? {
      endpoint: "amqp://127.0.0.1:5672/",
      mode: "local single node",
    } : null,
  }, null, 2);
  aiAssistOpen.value = true;
}

function openLogAiAssistant() {
  aiAssistTitle.value = `${selectedService.value?.name ?? "Service"} 日志诊断`;
  aiAssistOptions.value = [{
    id: "service_logs",
    label: "诊断日志",
    hint: "说明当前现象，AI 会结合最近的运行日志分析",
  }];
  aiAssistContext.value = [
    `服务：${selectedService.value?.name ?? ""}`,
    `状态：${selectedService.value?.status ?? ""}`,
    "最近日志：",
    logs.value.slice(-24_000),
  ].join("\n");
  aiAssistOpen.value = true;
}

function openConfigAiAssistant() {
  aiAssistTitle.value = `${selectedService.value?.name ?? "Web"} 配置助手`;
  aiAssistOptions.value = [{
    id: "web_config",
    label: "生成配置",
    hint: "描述反向代理、静态目录、端口或本地开发需求",
    canApply: true,
  }];
  aiAssistContext.value = `服务：${selectedService.value?.name}\n当前配置：\n${configContent.value}`;
  aiAssistOpen.value = true;
}

function applyAiAssist(content: string, capability: AiToolCapability) {
  if (capability === "database_sql") {
    sqlInput.value = content.replace(/^```sql\s*|```$/gi, "").trim();
  } else if (capability === "redis_command") {
    const command = content.split(/\r?\n/)[0].replace(/^```.*?\s*|```$/g, "").trim();
    if (/^(FLUSHALL|FLUSHDB|SHUTDOWN|DEBUG|MODULE|CONFIG\s+SET)\b/i.test(command)
      || /^KEYS\s+\*$/i.test(command)) {
      showToast({
        intent: "error",
        title: "已拦截高风险命令",
        message: "AI 生成的 Redis 命令包含危险操作，未写入命令行。",
      });
      return;
    }
    consoleInput.value = command;
  } else if (capability === "mongodb_command") {
    try {
      const value = JSON.parse(content.trim().replace(/^```(?:json)?\s*/i, "").replace(/\s*```$/, ""));
      const commandName = value && typeof value === "object"
        ? Object.keys(value)[0]?.toLowerCase()
        : "";
      const blockedCommands = new Set([
        "shutdown",
        "setparameter",
        "eval",
        "createuser",
        "updateuser",
        "dropuser",
        "dropdatabase",
      ]);
      if (!commandName || blockedCommands.has(commandName)) {
        showToast({
          intent: "error",
          title: "已拦截高风险 MongoDB 命令",
          message: "AI 结果包含实例管理、用户权限或高风险操作，未写入命令台。",
        });
        return;
      }
      mongoCommandInput.value = JSON.stringify(value, null, 2);
    } catch {
      showToast({ intent: "error", title: "MongoDB 命令格式无效", message: "AI 没有返回合法 JSON，未写入命令台。" });
      return;
    }
  } else if (capability === "message_design") {
    try {
      const value = JSON.parse(content.trim().replace(/^```(?:json)?\s*/i, "").replace(/\s*```$/, ""));
      const broker = String(value.broker || "").toLowerCase();
      if (broker !== selectedKind.value) {
        throw new Error("AI 返回的消息系统与当前服务不一致");
      }
      const payload = typeof value.payload === "string"
        ? value.payload
        : JSON.stringify(value.payload ?? {}, null, 2);
      if (new TextEncoder().encode(payload).length > 1024 * 1024) {
        throw new Error("AI 生成的 Payload 超过 1 MiB");
      }
      if (selectedKind.value === "nats") {
        const subject = String(value.destination || "").trim();
        if (!subject || /\s|[*>]/.test(subject)) {
          throw new Error("AI 生成的 NATS 发布 Subject 无效");
        }
        natsPublishSubject.value = subject;
        natsSubscribeSubject.value = String(value.subscription || natsSubscribeSubject.value);
        natsPublishPayload.value = payload;
      } else if (selectedKind.value === "kafka") {
        const topic = String(value.destination || "").trim();
        if (!/^[A-Za-z0-9._-]{1,249}$/.test(topic) || topic === "." || topic === "..") {
          throw new Error("AI 生成的 Kafka Topic 名称无效");
        }
        kafkaTopicName.value = topic;
        kafkaPartitions.value = Math.min(12, Math.max(1, Number(value.partitions) || 3));
        kafkaMessageKey.value = String(value.key || "");
        kafkaMessagePayload.value = payload;
        if (kafkaTopics.value.some((item) => item.name === topic)) {
          kafkaSelectedTopic.value = topic;
        }
      }
    } catch (cause) {
      showToast({
        intent: "error",
        title: "消息设计格式无效",
        message: cause instanceof Error
          ? cause.message
          : "AI 没有返回合法 JSON，未修改当前表单。",
      });
      return;
    }
  } else if (capability === "web_config") {
    configContent.value = content.replace(/^```[\w-]*\s*|```$/g, "").trim();
  }
  aiAssistOpen.value = false;
}

function tableIdentity(table: TableInfo) {
  return `${table.schema}.${table.name}`;
}

function displayCell(value: string | null) {
  return value === null ? "NULL" : value;
}

function previewColumnType(columnName: string) {
  const column = tableDetail.value?.columns.find(
    (item) => item.name === columnName,
  );
  return column ? databaseTypeInfo(column.dataType) : null;
}

async function loadConfig() {
  const service = selectedService.value;
  if (!service) return;
  configLoading.value = true;
  try {
    const content = await readServiceConfig(service.kind);
    configContent.value = content;
    configOriginal.value = content;
  } catch (cause) {
    configContent.value = "";
    configOriginal.value = "";
    error.value = String(cause);
  } finally {
    configLoading.value = false;
  }
}

async function saveConfig() {
  const service = selectedService.value;
  if (!service || !configChanged.value) return;
  configSaving.value = true;
  try {
    await saveServiceConfig(service.kind, configContent.value);
    configOriginal.value = configContent.value;
    notice.value = "配置已保存；服务运行中时需要重启后生效";
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    configSaving.value = false;
  }
}

async function loadLogs() {
  const service = selectedService.value;
  if (!service) return;
  logsLoading.value = true;
  try {
    logs.value = await getServiceLogs(service.kind);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    logsLoading.value = false;
  }
}

function formatMailDate(value: string) {
  if (!value) return "—";
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? value
    : date.toLocaleString(resolvedLocale.value);
}

function formatBackupDate(value: number) {
  return new Date(value).toLocaleString(resolvedLocale.value);
}

function localizedSupportLabel(label: string) {
  if (resolvedLocale.value !== "en-US") return label;
  if (label === "旧版 · 已停止维护") return t("versions.support.legacy");
  if (label === "创新版本 · 无长期支持") return t("versions.support.innovation");
  if (label === "历史稳定版") return t("versions.support.historical");
  if (label === "当前稳定版") return t("versions.support.current");
  if (label === "兼容稳定版") return t("versions.support.compatible");
  if (label === "最新稳定版") return t("versions.support.latest");
  const legacyMaintenance = label.match(/^旧版 · 维护期至 (.+)$/);
  if (legacyMaintenance) {
    return t("versions.support.legacyUntil", { date: legacyMaintenance[1] });
  }
  const maintenance = label.match(/^维护期至 (.+)$/);
  if (maintenance) {
    return t("versions.support.maintenanceUntil", { date: maintenance[1] });
  }
  return label;
}

function localizedInstallSupportLabel(label: string) {
  if (resolvedLocale.value !== "en-US") return label;
  const supported = label.match(/^支持当前平台：(.+)$/);
  if (supported) {
    return t("versions.platformSupported", { platform: supported[1] });
  }
  const unsupported = label.match(/^当前版本暂不支持在 (.+) 自动安装$/);
  return unsupported
    ? t("versions.platformUnsupported", { platform: unsupported[1] })
    : label;
}

function localizedSqlSummary(summary: string) {
  if (resolvedLocale.value !== "en-US") return summary;
  if (summary === "执行完成") return t("console.completed");
  const rows = summary.match(/^返回 (\d+) 行$/);
  return rows ? t("console.returnedRows", { count: rows[1] }) : summary;
}

function chartPoints(values: number[], width = 560, height = 112) {
  if (values.length < 2) return "";
  const maximum = Math.max(...values, 1);
  return values
    .map((value, index) => {
      const x = (index / (values.length - 1)) * width;
      const y = height - (value / maximum) * (height - 12) - 6;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(" ");
}

onMounted(async () => {
  void getVersion().then((version) => {
    appVersion.value = version;
  }).catch(() => undefined);
  window.addEventListener("zhiyu:open-ai-settings", handleOpenAiSettingsRequest);
  window.addEventListener("keydown", handleGlobalKeydown);
  window.addEventListener("keydown", markUserActivity);
  window.addEventListener("pointerdown", markUserActivity);
  window.addEventListener("zhiyu:navigate", handleWorkspaceNavigation);
  await loadAppSettings();
  try {
    unlistenCloseRequested = await getCurrentWindow().onCloseRequested(
      async (event) => {
        event.preventDefault();
        if (hidingWindow) {
          return;
        }
        hidingWindow = true;
        try {
          if (!appSettings.value.keepServicesRunningOnClose) {
            await stopAllManagedServices();
          }
        } finally {
          await getCurrentWindow().hide();
          hidingWindow = false;
        }
      },
    );
  } catch {
    // Window closing still works when lifecycle interception is unavailable.
  }
  try {
    unlistenInstallProgress = await listen<InstallProgressPayload>(
      "install-progress",
      (event) => handleInstallProgress(event.payload),
    );
  } catch {
    // Service management remains usable if the event channel is unavailable.
  }
  try {
    unlistenTrayNavigation = await listen<TrayNavigationEvent>(
      "tray:navigate",
      async (event) => {
        if (event.payload.target === "overview") {
          await openDashboard();
        } else if (event.payload.target === "settings") {
          await openSettings();
        } else if (event.payload.target === "clipboard") {
          selectTool("clipboard");
        } else if (event.payload.kind) {
          await selectService(event.payload.kind);
        }
      },
    );
    unlistenTrayAction = await listen<TrayServiceActionEvent>(
      "tray:service-action",
      async (event) => {
        if (event.payload.success) {
          notice.value = event.payload.message;
          error.value = "";
        } else {
          error.value = event.payload.message;
          notice.value = "";
        }
        await Promise.all([
          refreshServices(true),
          refreshEnvironmentMetrics(),
          refreshPortListeners(),
          loadAppSettings(),
        ]);
      },
    );
  } catch {
    // Tray actions remain available even if the frontend event bridge is unavailable.
  }
  await refreshServices();
  if (!appSettings.value.onboardingCompleted) {
    onboardingOpen.value = true;
  }
  await Promise.all([
    refreshMetrics(),
    refreshDiskUsage(),
    refreshEnvironmentMetrics(),
    refreshEnvironmentDiskUsage(),
  ]);
  serviceTimer = window.setInterval(() => {
    void refreshServices(true);
    void refreshEnvironmentMetrics();
  }, 3000);
  metricTimer = window.setInterval(async () => {
    if (activeTool.value || dashboardActive.value) return;
    await refreshMetrics();
    if (activeTab.value === "logs") await loadLogs();
  }, 2000);
  diskTimer = window.setInterval(() => {
    void refreshDiskUsage();
    void refreshEnvironmentDiskUsage();
  }, 60_000);
  portTimer = window.setInterval(() => {
    if (dashboardActive.value) void refreshPortListeners();
  }, 10_000);
  resourceSaverTimer = window.setInterval(() => {
    void checkResourceSaver();
  }, 60_000);
  if (appSettings.value.autoCheckUpdates) {
    void checkForUpdates();
  }
});

onUnmounted(() => {
  window.removeEventListener("zhiyu:open-ai-settings", handleOpenAiSettingsRequest);
  window.removeEventListener("keydown", handleGlobalKeydown);
  window.removeEventListener("keydown", markUserActivity);
  window.removeEventListener("pointerdown", markUserActivity);
  window.removeEventListener("zhiyu:navigate", handleWorkspaceNavigation);
  endSidebarPointerDrag();
  unlistenInstallProgress?.();
  unlistenCloseRequested?.();
  unlistenTrayNavigation?.();
  unlistenTrayAction?.();
  if (serviceTimer) window.clearInterval(serviceTimer);
  if (metricTimer) window.clearInterval(metricTimer);
  if (diskTimer) window.clearInterval(diskTimer);
  if (portTimer) window.clearInterval(portTimer);
  if (resourceSaverTimer) window.clearInterval(resourceSaverTimer);
});
</script>

<template>
  <div
    v-tool-i18n
    class="app-layout"
    :class="[
      `background-${visualSettings.backgroundStyle}`,
      `pattern-${resolvedBackgroundPattern}`,
      { 'has-custom-background': hasCustomBackground },
    ]"
    :style="backgroundShellStyle"
  >
    <ToastViewport />
    <CommandPalette
      :open="commandPaletteOpen"
      :items="commandPaletteItems"
      :busy-id="commandBusyId"
      @close="commandPaletteOpen = false"
      @select="handleCommandSelect"
    />
    <AiChatModal
      v-if="aiChatOpen"
      @close="aiChatOpen = false"
      @configure="openAiSettingsFromChat"
    />
    <AiAssistDialog
      :open="aiAssistOpen"
      :title="aiAssistTitle"
      :context="aiAssistContext"
      :options="aiAssistOptions"
      @close="aiAssistOpen = false"
      @settings="openAiSettingsFromChat"
      @apply="applyAiAssist"
    />
    <div v-if="hasCustomBackground" class="app-background-image"></div>
    <div v-if="hasCustomBackground" class="app-background-veil"></div>
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-mark"><span></span><span></span><span></span></div>
        <div class="brand-copy">
          <strong>{{ t("brand.name") }}</strong>
          <small>{{ t("brand.tagline") }}</small>
          <div class="brand-resource-row">
            <span
              :title="t('brand.memoryDetail', { count: environmentMetrics.runningServiceCount })"
            >
              {{ t("brand.memory") }} {{ formatBytes(environmentMetrics.memoryBytes) }}
            </span>
            <span
              :title="t('brand.diskDetail', { root: appSettings.installRoot || '~/.devbox' })"
            >
              {{ t("brand.disk") }} {{ formatBytes(environmentDiskBytes) }}
            </span>
          </div>
        </div>
      </div>

      <nav class="service-nav">
        <p class="nav-label">{{ t("nav.overview").toUpperCase() }}</p>
        <button
          type="button"
          class="service-nav-item dashboard-nav-item"
          :class="{ active: dashboardActive }"
          @click="openDashboard"
        >
          <span class="nav-icon dashboard">⌂</span>
          <span class="nav-copy">
            <strong>{{ t("nav.dashboardTitle") }}</strong>
            <small>{{ t("nav.dashboardHint") }}</small>
          </span>
        </button>

        <p class="nav-label">{{ t("nav.services").toUpperCase() }}</p>
        <button
          v-for="service in sidebarServices"
          :key="service.kind"
          type="button"
          class="service-nav-item"
          :class="{
            active:
              !dashboardActive &&
              !settingsActive &&
              activeTool === null &&
              selectedKind === service.kind,
          }"
          @click="selectService(service.kind)"
        >
          <span class="nav-icon" :class="service.kind">{{
            iconLetter[service.kind]
          }}</span>
          <span class="nav-copy">
            <strong>{{ service.name }}</strong>
            <small>
              v{{ service.version }} ·
              {{
                formatBytes(
                  diskUsageByKind[service.kind]?.totalBytes ?? null,
                )
              }}
            </small>
          </span>
          <i class="nav-state" :class="service.status"></i>
        </button>

        <p
          v-if="sidebarDevelopment.length"
          class="nav-label tool-label"
        >
          {{ t("nav.development").toUpperCase() }}
        </p>
        <button
          v-for="tool in sidebarDevelopment"
          :key="tool.id"
          type="button"
          class="service-nav-item"
          :class="{
            active:
              !dashboardActive &&
              !settingsActive &&
              activeTool === tool.id,
          }"
          @click="selectTool(tool.id)"
        >
          <span class="nav-icon" :class="tool.id">{{ tool.icon }}</span>
          <span class="nav-copy">
            <strong>{{ t(`tools.${tool.id}.label`) }}</strong>
            <small>{{ t(`tools.${tool.id}.hint`) }}</small>
          </span>
        </button>

        <p class="nav-label tool-label">{{ t("nav.tools").toUpperCase() }}</p>
        <button
          v-for="tool in sidebarTools"
          :key="tool.id"
          type="button"
          class="service-nav-item"
          :class="{
            active:
              !dashboardActive &&
              !settingsActive &&
              activeTool === tool.id,
          }"
          @click="selectTool(tool.id)"
        >
          <span class="nav-icon" :class="tool.id">{{ tool.icon }}</span>
          <span class="nav-copy">
            <strong>{{ t(`tools.${tool.id}.label`) }}</strong>
            <small>{{ t(`tools.${tool.id}.hint`) }}</small>
          </span>
        </button>

        <button type="button" class="add-service" disabled>
          <span>＋</span> 扩展更多服务
        </button>

        <p class="nav-label tool-label">{{ t("nav.system").toUpperCase() }}</p>
        <button
          type="button"
          class="service-nav-item"
          :class="{ active: settingsActive }"
          @click="openSettings"
        >
          <span class="nav-icon settings">⚙</span>
          <span class="nav-copy">
            <strong>{{ t("settings.title") }}</strong>
            <small>{{ t("nav.settingsHint") }}</small>
          </span>
        </button>
      </nav>

      <div class="sidebar-footer">
        <div class="sidebar-footer-core">
          <span class="core-dot"></span>
          <div>
            <strong>Core</strong>
            <small>{{ t("brand.coreReady") }}</small>
          </div>
        </div>
        <button
          type="button"
          class="sidebar-command-button"
          :title="t('commandPalette.open')"
          @click="openCommandPalette"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="10.5" cy="10.5" r="5.7" />
            <path d="m14.8 14.8 4.2 4.2" />
          </svg>
          <span>⌘K</span>
        </button>
        <button
          type="button"
          class="sidebar-ai-button"
          :title="t('aiChat.open')"
          @click="aiChatOpen = true"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M12 3.5a8.5 8.5 0 1 0 8.5 8.5" />
            <path d="M12 7.4a4.6 4.6 0 1 0 4.6 4.6" />
            <path d="M18.4 3.5v4.1h4.1M12 10.4V12l1.2 1.2" />
          </svg>
          <span>AI</span>
        </button>
      </div>
    </aside>

    <main class="content">
      <section
        v-if="installTask"
        class="install-progress-panel"
        :class="[installTask.status, { expanded: installLogExpanded }]"
      >
        <button
          type="button"
          class="install-progress-summary"
          :aria-expanded="installLogExpanded"
          @click="installLogExpanded = !installLogExpanded"
        >
          <span class="install-progress-state"></span>
          <strong>{{ installTask.title }}</strong>
          <span class="install-progress-stage">{{ installTask.stage }}</span>
          <span class="install-progress-value">
            {{
              installTask.status === "completed"
                ? "完成"
                : installTask.status === "failed"
                  ? "失败"
                  : installTask.status === "cancelled"
                    ? "已取消"
                    : `${installTask.percent}%`
            }}
          </span>
          <span class="install-progress-toggle">
            {{ installLogExpanded ? "收起" : "展开" }}
          </span>
        </button>
        <div class="install-progress-track">
          <span :style="{ width: `${installTask.percent}%` }"></span>
        </div>
        <p v-if="latestInstallLog" class="install-log-preview">
          <time>{{ latestInstallLog.time }}</time>
          <strong>{{ latestInstallLog.stage }}</strong>
          <span>{{ latestInstallLog.message }}</span>
        </p>
        <div v-if="installLogExpanded" class="install-log-full">
          <p v-if="installTask.logs.length === 0">等待安装器输出…</p>
          <template v-else>
            <p
              v-for="(entry, index) in installTask.logs"
              :key="`${entry.time}-${index}`"
            >
              <time>{{ entry.time }}</time>
              <strong>{{ entry.stage }}</strong>
              <span>{{ entry.message }}</span>
            </p>
          </template>
        </div>
        <button
          v-if="installTask.status === 'running'"
          type="button"
          class="install-progress-cancel"
          :disabled="installCancelling"
          @click="cancelCurrentInstall"
        >
          {{ installCancelling ? "取消中…" : "取消安装" }}
        </button>
        <button
          v-if="installTask.status !== 'running'"
          type="button"
          class="install-progress-close"
          aria-label="关闭安装日志"
          @click="installTask = null"
        >
          ×
        </button>
      </section>

      <div
        v-if="sshToolMounted"
        v-show="activeTool === 'ssh' && !loading"
        v-tool-i18n
        class="persistent-tool-host"
      >
        <SshTool :visible="activeTool === 'ssh' && !loading" />
      </div>

      <div v-if="loading" class="page-loading">{{ t("common.loading") }}…</div>

      <section v-else-if="settingsActive" class="settings-page">
        <header class="settings-header">
          <div>
            <span class="dashboard-eyebrow">PREFERENCES</span>
            <h1>{{ t("settings.title") }}</h1>
            <p>{{ t("settings.subtitle") }}</p>
          </div>
        </header>

        <nav class="settings-tabs" role="tablist" :aria-label="t('settings.title')">
          <button
            v-for="tab in settingsTabs"
            :id="`settings-tab-${tab}`"
            :key="tab"
            type="button"
            role="tab"
            :class="{ active: activeSettingsTab === tab }"
            :aria-selected="activeSettingsTab === tab"
            :aria-controls="`settings-panel-${tab}`"
            @click="activeSettingsTab = tab"
          >
            <strong>{{ t(`settings.tabs.${tab}.label`) }}</strong>
            <small>{{ t(`settings.tabs.${tab}.hint`) }}</small>
          </button>
        </nav>

        <div v-if="notice" class="notice settings-notice">
          <span>{{ notice }}</span>
          <button type="button" @click="notice = ''">×</button>
        </div>
        <div v-if="error" class="notice danger settings-notice">
          <span>{{ error }}</span>
          <button type="button" @click="error = ''">×</button>
        </div>

        <div class="settings-body">
          <section
            v-show="activeSettingsTab === 'appearance'"
            id="settings-panel-appearance"
            class="settings-section settings-appearance"
            role="tabpanel"
            aria-labelledby="settings-tab-appearance"
          >
            <div class="settings-section-title">
              <div>
                <h2>{{ t("settings.appearance.title") }}</h2>
                <p>{{ t("settings.appearance.subtitle") }}</p>
              </div>
            </div>
            <div class="language-setting">
              <div class="color-theme-heading">
                <strong>{{ t("settings.appearance.languageTitle") }}</strong>
                <small>{{ t("settings.appearance.languageHint") }}</small>
              </div>
              <div class="language-options" role="radiogroup" :aria-label="t('settings.appearance.languageTitle')">
                <button
                  v-for="option in localeOptions"
                  :key="option.value"
                  type="button"
                  :class="{ selected: settingsDraft.locale === option.value }"
                  role="radio"
                  :aria-checked="settingsDraft.locale === option.value"
                  @click="selectLocale(option.value)"
                >
                  <span>{{ option.value === "system" ? "◐" : option.value === "zh-CN" ? "中" : "EN" }}</span>
                  <strong>{{ t(`settings.appearance.${option.labelKey}`) }}</strong>
                  <small>{{ t(`settings.appearance.${option.hintKey}`) }}</small>
                  <em>✓</em>
                </button>
              </div>
            </div>
            <div class="theme-options" role="radiogroup" :aria-label="t('settings.appearance.themeTitle')">
              <button
                type="button"
                :class="{ selected: settingsDraft.themeMode === 'system' }"
                role="radio"
                :aria-checked="settingsDraft.themeMode === 'system'"
                @click="previewTheme('system')"
              >
                <span class="theme-preview system"><i></i><i></i></span>
                <strong>{{ t("settings.appearance.themeSystem") }}</strong>
                <small>{{ t("settings.appearance.themeSystemHint") }}</small>
              </button>
              <button
                type="button"
                :class="{ selected: settingsDraft.themeMode === 'light' }"
                role="radio"
                :aria-checked="settingsDraft.themeMode === 'light'"
                @click="previewTheme('light')"
              >
                <span class="theme-preview light"><i></i></span>
                <strong>{{ t("settings.appearance.themeLight") }}</strong>
                <small>{{ t("settings.appearance.themeLightHint") }}</small>
              </button>
              <button
                type="button"
                :class="{ selected: settingsDraft.themeMode === 'dark' }"
                role="radio"
                :aria-checked="settingsDraft.themeMode === 'dark'"
                @click="previewTheme('dark')"
              >
                <span class="theme-preview dark"><i></i></span>
                <strong>{{ t("settings.appearance.themeDark") }}</strong>
                <small>{{ t("settings.appearance.themeDarkHint") }}</small>
              </button>
            </div>
            <div class="color-theme-setting">
              <div class="color-theme-heading">
                <strong>{{ t("settings.appearance.colorTitle") }}</strong>
                <small>{{ t("settings.appearance.colorHint") }}</small>
              </div>
              <div
                class="color-theme-options"
                role="radiogroup"
                aria-label="配色主题"
              >
                <button
                  v-for="option in colorThemeOptions"
                  :key="option.value"
                  type="button"
                  :class="{
                    selected: settingsDraft.colorTheme === option.value,
                  }"
                  role="radio"
                  :aria-checked="settingsDraft.colorTheme === option.value"
                  @click="previewColorTheme(option.value)"
                >
                  <span class="color-theme-swatches" aria-hidden="true">
                    <i
                      v-for="color in option.colors"
                      :key="color"
                      :style="{ backgroundColor: color }"
                    ></i>
                  </span>
                  <strong>{{ t(`settings.appearance.colors.${option.value}.label`) }}</strong>
                  <small>{{ t(`settings.appearance.colors.${option.value}.description`) }}</small>
                  <em>✓</em>
                </button>
              </div>
            </div>
            <div class="background-pattern-setting">
              <div class="color-theme-heading">
                <strong>{{ t("settings.appearance.patternTitle") }}</strong>
                <small>{{ t("settings.appearance.patternHint") }}</small>
              </div>
              <div
                class="background-pattern-options"
                role="radiogroup"
                aria-label="背景纹理"
              >
                <button
                  v-for="option in backgroundPatternOptions"
                  :key="option.value"
                  type="button"
                  :class="{
                    selected:
                      settingsDraft.backgroundPattern === option.value,
                  }"
                  role="radio"
                  :aria-checked="
                    settingsDraft.backgroundPattern === option.value
                  "
                  @click="previewBackgroundPattern(option.value)"
                >
                  <span
                    class="background-pattern-preview"
                    :class="`pattern-preview-${option.value}`"
                    aria-hidden="true"
                  ></span>
                  <strong>{{ t(`settings.appearance.patterns.${option.value}.label`) }}</strong>
                  <small>{{ t(`settings.appearance.patterns.${option.value}.description`) }}</small>
                  <em>✓</em>
                </button>
              </div>
            </div>
            <div class="ui-scale-setting">
              <div>
                <strong>{{ t("settings.appearance.scaleTitle") }}</strong>
                <small>{{ t("settings.appearance.scaleHint") }}</small>
              </div>
              <div
                class="ui-scale-options"
                role="radiogroup"
                aria-label="界面字号"
              >
                <button
                  v-for="option in uiScaleOptions"
                  :key="option.value"
                  type="button"
                  :class="{
                    selected: settingsDraft.uiScale === option.value,
                  }"
                  role="radio"
                  :aria-checked="settingsDraft.uiScale === option.value"
                  @click="previewUiScale(option.value)"
                >
                  <span
                    :style="{ fontSize: `${option.value / 10}px` }"
                  >
                    字
                  </span>
                  <strong>{{ t(`settings.appearance.sizes.${option.value}`) }}</strong>
                  <small>{{ option.value }}%</small>
                </button>
              </div>
            </div>
            <div class="background-setting">
              <div class="background-setting-head">
                <div>
                  <strong>{{ t("settings.appearance.backgroundTitle") }}</strong>
                  <small>{{ t("settings.appearance.backgroundHint") }}</small>
                </div>
                <div class="background-actions">
                  <button
                    v-if="settingsDraft.backgroundImagePath"
                    type="button"
                    class="background-remove"
                    @click="clearAppBackground"
                  >
                    {{ t("common.remove") }}
                  </button>
                  <button
                    type="button"
                    class="background-upload"
                    :disabled="backgroundImporting"
                    @click="chooseAppBackground"
                  >
                    <span v-if="backgroundImporting" class="spinner"></span>
                    {{
                      settingsDraft.backgroundImagePath
                        ? t("settings.appearance.replaceImage")
                        : t("settings.appearance.chooseImage")
                    }}
                  </button>
                </div>
              </div>

              <div
                class="background-preview"
                :class="[
                  `preview-${settingsDraft.backgroundStyle}`,
                  { empty: !settingsDraft.backgroundImagePath },
                ]"
                :style="{
                  backgroundImage: settingsDraft.backgroundImagePath
                    ? `url('${backgroundImageUrl}')`
                    : undefined,
                  backgroundPosition: settingsDraft.backgroundPosition,
                }"
              >
                <div v-if="!settingsDraft.backgroundImagePath" class="background-empty">
                  <span>▧</span>
                  <strong>{{ t("settings.appearance.emptyImage") }}</strong>
                  <small>{{ t("settings.appearance.imageSupport") }}</small>
                </div>
                <template v-else>
                  <div class="background-preview-veil"></div>
                  <div class="background-preview-window">
                    <i></i><i></i><i></i>
                    <span></span>
                    <b></b>
                  </div>
                </template>
              </div>

              <div class="background-style-options">
                <button
                  v-for="option in backgroundStyleOptions"
                  :key="option.value"
                  type="button"
                  :disabled="!settingsDraft.backgroundImagePath"
                  :class="{
                    selected: settingsDraft.backgroundStyle === option.value,
                  }"
                  @click="selectBackgroundStyle(option.value)"
                >
                  <span :class="`style-swatch ${option.value}`"></span>
                  <strong>{{ t(`settings.appearance.backgroundStyles.${option.value}.label`) }}</strong>
                  <small>{{ t(`settings.appearance.backgroundStyles.${option.value}.description`) }}</small>
                </button>
              </div>

              <div
                v-if="settingsDraft.backgroundImagePath"
                class="background-fine-tuning"
              >
                <div class="background-position-control">
                  <span>{{ t("settings.appearance.imagePosition") }}</span>
                  <div role="radiogroup" aria-label="背景图片位置">
                    <button
                      v-for="option in backgroundPositionOptions"
                      :key="option.value"
                      type="button"
                      :class="{
                        selected:
                          settingsDraft.backgroundPosition === option.value,
                      }"
                      @click="selectBackgroundPosition(option.value)"
                    >
                      {{ t(`settings.appearance.positions.${option.value}`) }}
                    </button>
                  </div>
                </div>
                <label class="background-overlay-control">
                  <span>
                    <strong>{{ t("settings.appearance.contentOverlay") }}</strong>
                    <small>{{ settingsDraft.backgroundOverlay }}%</small>
                  </span>
                  <input
                    v-model.number="settingsDraft.backgroundOverlay"
                    type="range"
                    min="20"
                    max="90"
                    step="1"
                    @change="saveSettings"
                  />
                </label>
                <button
                  type="button"
                  class="background-disable"
                  @click="
                    selectBackgroundStyle(
                      settingsDraft.backgroundStyle === 'off'
                        ? 'frosted'
                        : 'off',
                    )
                  "
                >
                  {{
                    settingsDraft.backgroundStyle === "off"
                      ? t("settings.appearance.showBackground")
                      : t("settings.appearance.disableBackground")
                  }}
                </button>
              </div>
            </div>
          </section>

          <section
            v-show="activeSettingsTab === 'sidebar'"
            id="settings-panel-sidebar"
            class="settings-section sidebar-settings-section"
            role="tabpanel"
            aria-labelledby="settings-tab-sidebar"
          >
            <div class="settings-section-title">
              <div>
                <h2>{{ t("settings.sidebar.title") }}</h2>
                <p>{{ t("settings.sidebar.subtitle") }}</p>
              </div>
            </div>
            <div class="sidebar-manager-grid">
              <div class="sidebar-manager">
                <div class="sidebar-manager-head">
                  <div>
                    <strong>{{ t("common.services") }}</strong>
                    <small>SERVICES</small>
                  </div>
                  <span>
                    {{
                      services.length -
                      settingsDraft.hiddenServices.length
                    }}/{{ services.length }} {{ t("common.show") }}
                  </span>
                </div>
                <div class="sidebar-manager-list">
                  <div
                    v-for="service in settingsOrderedServices"
                    :key="service.kind"
                    class="sidebar-manager-item"
                    data-sidebar-group="services"
                    :data-sidebar-id="service.kind"
                    :class="{
                      dragging:
                        draggingSidebarItem?.group === 'services' &&
                        draggingSidebarItem.id === service.kind,
                      'drop-before':
                        sidebarDropTarget ===
                        `services:${service.kind}:before`,
                      'drop-after':
                        sidebarDropTarget ===
                        `services:${service.kind}:after`,
                    }"
                  >
                    <span
                      class="sidebar-drag-handle"
                      :title="t('settings.sidebar.drag')"
                      @pointerdown="
                        beginSidebarPointerDrag(
                          $event,
                          'services',
                          service.kind,
                        )
                      "
                    >
                      <svg viewBox="0 0 12 18" aria-hidden="true">
                        <circle cx="3" cy="3" r="1.2"></circle>
                        <circle cx="9" cy="3" r="1.2"></circle>
                        <circle cx="3" cy="9" r="1.2"></circle>
                        <circle cx="9" cy="9" r="1.2"></circle>
                        <circle cx="3" cy="15" r="1.2"></circle>
                        <circle cx="9" cy="15" r="1.2"></circle>
                      </svg>
                    </span>
                    <span class="nav-icon" :class="service.kind">
                      {{ iconLetter[service.kind] }}
                    </span>
                    <span class="sidebar-manager-copy">
                      <strong>{{ service.name }}</strong>
                      <small>v{{ service.version }} · :{{ service.port }}</small>
                    </span>
                    <label
                      class="sidebar-visibility-switch"
                      :title="
                        settingsDraft.hiddenServices.includes(service.kind)
                          ? t('settings.sidebar.showInSidebar')
                          : t('settings.sidebar.hideFromSidebar')
                      "
                    >
                      <input
                        type="checkbox"
                        :checked="
                          !settingsDraft.hiddenServices.includes(service.kind)
                        "
                        :aria-label="`在侧栏显示 ${service.name}`"
                        @change="
                          toggleServiceVisibility(service.kind, $event)
                        "
                      />
                      <i></i>
                    </label>
                  </div>
                </div>
              </div>

              <div class="sidebar-manager">
                <div class="sidebar-manager-head">
                  <div>
                    <strong>{{ t("common.tools") }}</strong>
                    <small>TOOLS</small>
                  </div>
                  <span>
                    {{
                      TOOLS.length - settingsDraft.hiddenTools.length
                    }}/{{ TOOLS.length }} {{ t("common.show") }}
                  </span>
                </div>
                <div class="sidebar-manager-list">
                  <div
                    v-for="tool in settingsOrderedTools"
                    :key="tool.id"
                    class="sidebar-manager-item"
                    data-sidebar-group="tools"
                    :data-sidebar-id="tool.id"
                    :class="{
                      dragging:
                        draggingSidebarItem?.group === 'tools' &&
                        draggingSidebarItem.id === tool.id,
                      'drop-before':
                        sidebarDropTarget === `tools:${tool.id}:before`,
                      'drop-after':
                        sidebarDropTarget === `tools:${tool.id}:after`,
                    }"
                  >
                    <span
                      class="sidebar-drag-handle"
                      :title="t('settings.sidebar.drag')"
                      @pointerdown="
                        beginSidebarPointerDrag($event, 'tools', tool.id)
                      "
                    >
                      <svg viewBox="0 0 12 18" aria-hidden="true">
                        <circle cx="3" cy="3" r="1.2"></circle>
                        <circle cx="9" cy="3" r="1.2"></circle>
                        <circle cx="3" cy="9" r="1.2"></circle>
                        <circle cx="9" cy="9" r="1.2"></circle>
                        <circle cx="3" cy="15" r="1.2"></circle>
                        <circle cx="9" cy="15" r="1.2"></circle>
                      </svg>
                    </span>
                    <span class="nav-icon" :class="tool.id">
                      {{ tool.icon }}
                    </span>
                    <span class="sidebar-manager-copy">
                    <strong>{{ t(`tools.${tool.id}.label`) }}</strong>
                    <small>{{ t(`tools.${tool.id}.hint`) }}</small>
                    </span>
                    <label
                      class="sidebar-visibility-switch"
                      :title="
                        settingsDraft.hiddenTools.includes(tool.id)
                          ? t('settings.sidebar.showInSidebar')
                          : t('settings.sidebar.hideFromSidebar')
                      "
                    >
                      <input
                        type="checkbox"
                        :checked="
                          !settingsDraft.hiddenTools.includes(tool.id)
                        "
                        :aria-label="`${t('settings.sidebar.showInSidebar')} ${t(`tools.${tool.id}.label`)}`"
                        @change="toggleToolVisibility(tool.id, $event)"
                      />
                      <i></i>
                    </label>
                  </div>
                </div>
              </div>
            </div>
          </section>

          <section
            v-show="activeSettingsTab === 'ai'"
            id="settings-panel-ai"
            class="settings-section ai-settings-section"
            role="tabpanel"
            aria-labelledby="settings-tab-ai"
          >
            <div class="settings-section-title">
              <div>
                <h2>{{ t("settings.ai.title") }}</h2>
                <p>{{ t("settings.ai.subtitle") }}</p>
              </div>
              <span
                class="ai-config-status"
                :class="{ ready: aiSettings.apiKeyConfigured }"
              >
                {{
                  aiSettings.apiKeyConfigured
                    ? t("settings.ai.keyConfigured")
                    : t("settings.ai.keyMissing")
                }}
              </span>
            </div>

            <div v-if="aiSettingsLoading" class="ai-settings-loading">
              <span class="spinner"></span>
              {{ t("common.loading") }}…
            </div>

            <template v-else>
              <label class="settings-toggle-row">
                <span>
                  <strong>{{ t("settings.ai.enableTitle") }}</strong>
                  <small>{{ t("settings.ai.enableHint") }}</small>
                </span>
                <input v-model="aiDraft.enabled" type="checkbox" />
                <i></i>
              </label>

              <div class="ai-avatar-setting">
                <div class="ai-field-heading">
                  <strong>{{ t("settings.ai.avatarTitle") }}</strong>
                  <small>{{ t("settings.ai.avatarHint") }}</small>
                </div>
                <div class="ai-avatar-options">
                  <article>
                    <span class="ai-avatar-preview user">
                      <img
                        v-if="aiUserAvatarUrl"
                        :src="aiUserAvatarUrl"
                        alt=""
                      />
                      <svg v-else viewBox="0 0 28 28" aria-hidden="true">
                        <circle cx="14" cy="10" r="4.2" />
                        <path d="M6.5 23c.8-4.3 3.3-6.5 7.5-6.5s6.7 2.2 7.5 6.5" />
                      </svg>
                    </span>
                    <div>
                      <strong>{{ t("settings.ai.userAvatar") }}</strong>
                      <small>{{ t("settings.ai.userAvatarHint") }}</small>
                    </div>
                    <button
                      type="button"
                      :disabled="Boolean(aiAvatarImporting)"
                      @click="chooseAiAvatar('user')"
                    >
                      {{
                        aiAvatarImporting === "user"
                          ? t("common.loading")
                          : t("settings.ai.chooseAvatar")
                      }}
                    </button>
                    <button
                      v-if="aiSettings.userAvatarPath"
                      type="button"
                      class="subtle"
                      :disabled="Boolean(aiAvatarImporting)"
                      @click="clearAiAvatar('user')"
                    >
                      {{ t("settings.ai.restoreDefault") }}
                    </button>
                  </article>
                  <article>
                    <span class="ai-avatar-preview assistant">
                      <img
                        v-if="aiAssistantAvatarUrl"
                        :src="aiAssistantAvatarUrl"
                        alt=""
                      />
                      <svg v-else viewBox="0 0 28 28" aria-hidden="true">
                        <path d="M14 3.5a10.5 10.5 0 1 0 10.5 10.5" />
                        <path d="M14 8a6 6 0 1 0 6 6" />
                        <path d="M20.7 3.7v5.1h5.1M14 11.2V14l2 2" />
                      </svg>
                    </span>
                    <div>
                      <strong>{{ t("settings.ai.assistantAvatar") }}</strong>
                      <small>{{ t("settings.ai.assistantAvatarHint") }}</small>
                    </div>
                    <button
                      type="button"
                      :disabled="Boolean(aiAvatarImporting)"
                      @click="chooseAiAvatar('assistant')"
                    >
                      {{
                        aiAvatarImporting === "assistant"
                          ? t("common.loading")
                          : t("settings.ai.chooseAvatar")
                      }}
                    </button>
                    <button
                      v-if="aiSettings.assistantAvatarPath"
                      type="button"
                      class="subtle"
                      :disabled="Boolean(aiAvatarImporting)"
                      @click="clearAiAvatar('assistant')"
                    >
                      {{ t("settings.ai.restoreDefault") }}
                    </button>
                  </article>
                </div>
              </div>

              <div class="ai-provider-setting">
                <div class="ai-field-heading">
                  <strong>{{ t("settings.ai.providerTitle") }}</strong>
                  <small>{{ t("settings.ai.providerHint") }}</small>
                </div>
                <div class="ai-provider-options">
                  <button
                    v-for="provider in aiProviderPresets"
                    :key="provider.id"
                    type="button"
                    :class="{ selected: aiProviderId === provider.id }"
                    @click="selectAiProvider(provider.id)"
                  >
                    <span>{{ provider.badge }}</span>
                    <strong>{{ provider.name }}</strong>
                    <small>{{ t(provider.descriptionKey) }}</small>
                    <i v-if="aiProviderId === provider.id">✓</i>
                  </button>
                  <button
                    type="button"
                    :class="{ selected: aiProviderId === 'custom' }"
                    @click="selectAiProvider('custom')"
                  >
                    <span>+</span>
                    <strong>{{ t("settings.ai.customProvider") }}</strong>
                    <small>{{ t("settings.ai.providers.custom") }}</small>
                    <i v-if="aiProviderId === 'custom'">✓</i>
                  </button>
                </div>
              </div>

              <div class="ai-protocol-setting">
                <div class="ai-field-heading">
                  <strong>{{ t("settings.ai.protocolTitle") }}</strong>
                  <small>{{ t("settings.ai.protocolHint") }}</small>
                </div>
                <div class="ai-protocol-options">
                  <button
                    type="button"
                    :disabled="!aiProtocolSupported('openai')"
                    :class="{ selected: aiDraft.protocol === 'openai' }"
                    @click="selectAiProtocol('openai')"
                  >
                    <span>O</span>
                    <strong>OpenAI Compatible</strong>
                    <small>POST /chat/completions</small>
                  </button>
                  <button
                    type="button"
                    :disabled="!aiProtocolSupported('anthropic')"
                    :class="{ selected: aiDraft.protocol === 'anthropic' }"
                    @click="selectAiProtocol('anthropic')"
                  >
                    <span>A</span>
                    <strong>Anthropic Compatible</strong>
                    <small>POST /messages</small>
                  </button>
                </div>
              </div>

              <div class="ai-form-grid">
                <label class="ai-form-field ai-form-field-wide">
                  <span>{{ t("settings.ai.baseUrl") }}</span>
                  <input
                    v-model.trim="aiDraft.baseUrl"
                    type="url"
                    spellcheck="false"
                    :readonly="aiProviderId !== 'custom'"
                    :placeholder="
                      aiDraft.protocol === 'anthropic'
                        ? 'https://api.anthropic.com/v1'
                        : 'https://api.openai.com/v1'
                    "
                    @input="aiTestResult = null"
                  />
                  <small>
                    {{
                      aiProviderId === "custom"
                        ? t("settings.ai.baseUrlHint")
                        : t("settings.ai.baseUrlPresetHint")
                    }}
                  </small>
                </label>

                <label class="ai-form-field">
                  <span class="ai-field-label">{{ t("settings.ai.model") }}</span>
                  <select
                    v-if="aiModelSuggestions.length && !aiCustomModel"
                    class="ai-model-select"
                    :value="aiDraft.model"
                    @change="selectAiModel"
                  >
                    <option
                      v-for="model in aiModelSuggestions"
                      :key="model"
                      :value="model"
                    >
                      {{ model }}
                    </option>
                    <option value="__custom__">
                      {{ t("settings.ai.customModel") }}
                    </option>
                  </select>
                  <div v-else class="ai-model-custom-input">
                    <input
                      v-model.trim="aiDraft.model"
                      type="text"
                      spellcheck="false"
                      :placeholder="t('settings.ai.modelPlaceholder')"
                      @input="aiTestResult = null"
                    />
                    <button
                      v-if="aiModelSuggestions.length"
                      type="button"
                      @click="useRecommendedAiModel"
                    >
                      {{ t("settings.ai.recommendedModels") }}
                    </button>
                  </div>
                  <small v-if="aiModelSuggestions.length">
                    {{ t("settings.ai.modelHint") }}
                  </small>
                </label>

                <label class="ai-form-field">
                  <span class="ai-api-key-label">
                    {{ t("settings.ai.apiKey") }}
                    <button
                      v-if="activeAiProvider"
                      type="button"
                      @click="openAiKeyPage"
                    >
                      {{ t("settings.ai.getKey") }} ↗
                    </button>
                  </span>
                  <div class="ai-secret-input">
                    <input
                      v-model="aiDraft.apiKey"
                      type="password"
                      autocomplete="new-password"
                      spellcheck="false"
                      :placeholder="
                        aiSettings.apiKeyConfigured && !aiDraft.clearApiKey
                          ? t('settings.ai.keyRetained')
                          : t('settings.ai.keyPlaceholder')
                      "
                      @input="
                        aiDraft.clearApiKey = false;
                        aiTestResult = null;
                      "
                    />
                    <button
                      v-if="aiSettings.apiKeyConfigured && !aiDraft.clearApiKey"
                      type="button"
                      @click="clearAiApiKey"
                    >
                      {{ t("settings.ai.clearKey") }}
                    </button>
                  </div>
                  <small>{{ t("settings.ai.apiKeyHint") }}</small>
                </label>

                <label class="ai-form-field">
                  <span>{{ t("settings.ai.timeout") }}</span>
                  <div class="settings-number">
                    <input
                      v-model.number="aiDraft.timeoutSeconds"
                      type="number"
                      min="5"
                      max="600"
                    />
                    <em>{{ t("settings.storage.seconds") }}</em>
                  </div>
                </label>

                <label class="ai-form-field">
                  <span>{{ t("settings.ai.maxTokens") }}</span>
                  <input
                    v-model.number="aiDraft.maxOutputTokens"
                    type="number"
                    min="16"
                    max="65536"
                    step="128"
                  />
                </label>
              </div>

              <div
                v-if="aiTestResult"
                class="ai-test-result"
                :class="{ success: aiTestResult.success, danger: !aiTestResult.success }"
              >
                <span>{{ aiTestResult.success ? "✓" : "!" }}</span>
                <div>
                  <strong>
                    {{
                      aiTestResult.success
                        ? t("settings.ai.testSucceeded")
                        : t("settings.ai.testFailed")
                    }}
                  </strong>
                  <small>
                    {{ aiTestResult.message }}
                    <template v-if="aiTestResult.latencyMillis !== undefined">
                      · {{ aiTestResult.latencyMillis }} ms
                    </template>
                  </small>
                </div>
              </div>

              <div class="ai-settings-actions">
                <p>{{ t("settings.ai.localStorageHint") }}</p>
                <div>
                  <button
                    type="button"
                    class="secondary"
                    :disabled="aiConnectionTesting || aiSettingsSaving"
                    @click="runAiConnectionTest"
                  >
                    <span v-if="aiConnectionTesting" class="spinner"></span>
                    {{
                      aiConnectionTesting
                        ? t("settings.ai.testing")
                        : t("settings.ai.test")
                    }}
                  </button>
                  <button
                    type="button"
                    class="primary"
                    :disabled="aiSettingsSaving || aiConnectionTesting"
                    @click="saveAiConfiguration"
                  >
                    <span v-if="aiSettingsSaving" class="spinner"></span>
                    {{
                      aiSettingsSaving
                        ? t("settings.ai.saving")
                        : t("settings.ai.save")
                    }}
                  </button>
                </div>
              </div>
            </template>
          </section>

          <section
            v-show="activeSettingsTab === 'application'"
            id="settings-panel-application"
            class="settings-section"
            role="tabpanel"
            aria-labelledby="settings-tab-application"
          >
            <div class="settings-section-title">
              <div>
                <h2>{{ t("settings.application.behaviorTitle") }}</h2>
                <p>{{ t("settings.application.behaviorHint") }}</p>
              </div>
            </div>
            <label class="settings-toggle-row">
              <span>
                <strong>{{ t("settings.application.launchTitle") }}</strong>
                <small>{{ t("settings.application.launchHint") }}</small>
              </span>
              <input
                v-model="settingsDraft.launchAtLogin"
                type="checkbox"
                @change="saveSettings"
              />
              <i></i>
            </label>
            <label class="settings-toggle-row">
              <span>
                <strong>{{ t("settings.application.keepRunningTitle") }}</strong>
                <small>{{ t("settings.application.keepRunningHint") }}</small>
              </span>
              <input
                v-model="settingsDraft.keepServicesRunningOnClose"
                type="checkbox"
                @change="saveSettings"
              />
              <i></i>
            </label>
            <label class="settings-toggle-row">
              <span>
                <strong>{{ t("settings.application.resourceSaverTitle") }}</strong>
                <small>{{ t("settings.application.resourceSaverHint") }}</small>
              </span>
              <input
                v-model="settingsDraft.resourceSaverEnabled"
                type="checkbox"
                @change="saveSettings"
              />
              <i></i>
            </label>
            <div
              v-if="settingsDraft.resourceSaverEnabled"
              class="resource-saver-settings"
            >
              <div class="resource-saver-fields">
                <label>
                  <span>{{ t("settings.application.idleAfter") }}</span>
                  <div>
                    <input
                      v-model.number="settingsDraft.resourceSaverMinutes"
                      type="number"
                      min="15"
                      max="1440"
                      @change="saveSettings"
                    />
                    <em>{{ t("settings.application.minutes") }}</em>
                  </div>
                </label>
                <label>
                  <span>{{ t("settings.application.idleAction") }}</span>
                  <select
                    v-model="settingsDraft.resourceSaverMode"
                    @change="saveSettings"
                  >
                    <option value="remind">
                      {{ t("settings.application.remindOnly") }}
                    </option>
                    <option value="stop">
                      {{ t("settings.application.autoStop") }}
                    </option>
                  </select>
                </label>
              </div>
              <div class="resource-saver-services">
                <strong>{{ t("settings.application.managedServices") }}</strong>
                <small>{{ t("settings.application.managedServicesHint") }}</small>
                <div>
                  <label v-for="service in services" :key="service.kind">
                    <input
                      v-model="settingsDraft.resourceSaverServices"
                      type="checkbox"
                      :value="service.kind"
                      @change="saveSettings"
                    />
                    <span>{{ service.name }}</span>
                  </label>
                </div>
              </div>
            </div>
            <div class="settings-guide-row">
              <span>
                <strong>{{ t("settings.application.onboardingTitle") }}</strong>
                <small>{{ t("settings.application.onboardingHint") }}</small>
              </span>
              <button type="button" @click="showOnboarding">{{ t("settings.application.viewAgain") }}</button>
            </div>
          </section>

          <section
            v-show="activeSettingsTab === 'network'"
            id="settings-panel-network"
            class="settings-section proxy-settings-section"
            role="tabpanel"
            aria-labelledby="settings-tab-network"
          >
            <div class="settings-section-title">
              <div>
                <h2>{{ t("settings.network.title") }}</h2>
                <p>{{ t("settings.network.subtitle") }}</p>
              </div>
            </div>

            <div class="proxy-mode-setting">
              <div class="proxy-mode-heading">
                <strong>{{ t("settings.network.modeTitle") }}</strong>
                <small>{{ t("settings.network.modeHint") }}</small>
              </div>
              <div class="proxy-mode-options" role="radiogroup" :aria-label="t('settings.network.modeTitle')">
                <button
                  v-for="mode in (['system', 'manual', 'disabled'] as ProxyMode[])"
                  :key="mode"
                  type="button"
                  role="radio"
                  :aria-checked="settingsDraft.proxyMode === mode"
                  :class="{ selected: settingsDraft.proxyMode === mode }"
                  @click="selectProxyMode(mode)"
                >
                  <span>{{ mode === "system" ? "◐" : mode === "manual" ? "↗" : "×" }}</span>
                  <strong>{{ t(`settings.network.modes.${mode}.label`) }}</strong>
                  <small>{{ t(`settings.network.modes.${mode}.hint`) }}</small>
                  <em>✓</em>
                </button>
              </div>
            </div>

            <div v-if="settingsDraft.proxyMode === 'manual'" class="settings-field proxy-url-field">
              <label for="proxy-url">{{ t("settings.network.proxyUrl") }}</label>
              <div>
                <input
                  id="proxy-url"
                  v-model.trim="settingsDraft.proxyUrl"
                  type="url"
                  placeholder="http://127.0.0.1:7890"
                  @change="saveSettings"
                />
                <small>{{ t("settings.network.proxyUrlHint") }}</small>
              </div>
            </div>

            <div class="proxy-scope-heading">
              <strong>{{ t("settings.network.scopeTitle") }}</strong>
              <small>{{ t("settings.network.scopeHint") }}</small>
            </div>
            <label class="settings-toggle-row">
              <span>
                <strong>{{ t("settings.network.downloadTitle") }}</strong>
                <small>{{ t("settings.network.downloadHint") }}</small>
              </span>
              <input
                v-model="settingsDraft.downloadProxyEnabled"
                type="checkbox"
                :disabled="settingsDraft.proxyMode === 'disabled'"
                @change="saveSettings"
              />
              <i></i>
            </label>
            <label class="settings-toggle-row">
              <span>
                <strong>{{ t("settings.network.requestTitle") }}</strong>
                <small>{{ t("settings.network.requestHint") }}</small>
              </span>
              <input
                v-model="settingsDraft.networkProxyEnabled"
                type="checkbox"
                :disabled="settingsDraft.proxyMode === 'disabled'"
                @change="saveSettings"
              />
              <i></i>
            </label>
            <div class="proxy-local-note">
              <span>LOCAL BYPASS</span>
              <p>{{ t("settings.network.localBypass") }}</p>
            </div>
          </section>

          <section
            v-show="activeSettingsTab === 'storage'"
            id="settings-panel-storage"
            class="settings-section"
            role="tabpanel"
            aria-labelledby="settings-tab-storage"
          >
            <div class="settings-section-title">
              <div>
                <h2>{{ t("settings.storage.downloadTitle") }}</h2>
                <p>{{ t("settings.storage.downloadHint") }}</p>
              </div>
            </div>
            <div class="settings-field">
              <label for="download-mirror">{{ t("settings.storage.customMirror") }}</label>
              <div>
                <input
                  id="download-mirror"
                  v-model.trim="settingsDraft.downloadMirror"
                  type="url"
                  placeholder="https://your-cdn.example.com/zhiyu-packages"
                  @change="saveSettings"
                />
                <small>{{ t("settings.storage.customMirrorHint") }}</small>
              </div>
            </div>
            <label class="settings-toggle-row">
              <span>
                <strong>{{ t("settings.storage.githubMirror") }}</strong>
                <small>{{ t("settings.storage.githubMirrorHint") }}</small>
              </span>
              <input
                v-model="settingsDraft.publicGithubMirror"
                type="checkbox"
                @change="saveSettings"
              />
              <i></i>
            </label>
            <div class="settings-field-grid">
              <label>
                <span>{{ t("settings.storage.concurrency") }}</span>
                <select
                  v-model.number="settingsDraft.downloadConcurrency"
                  @change="saveSettings"
                >
                  <option v-for="count in 4" :key="count" :value="count">{{ count }} {{ t("settings.storage.itemUnit") }}</option>
                </select>
              </label>
              <label>
                <span>{{ t("settings.storage.timeout") }}</span>
                <div class="settings-number">
                  <input
                    v-model.number="settingsDraft.downloadTimeoutSeconds"
                    type="number"
                    min="15"
                    max="600"
                    @change="saveSettings"
                  />
                  <em>{{ t("settings.storage.seconds") }}</em>
                </div>
              </label>
            </div>
          </section>

          <section
            v-show="activeSettingsTab === 'storage'"
            class="settings-section"
            aria-labelledby="settings-tab-storage"
          >
            <div class="settings-section-title">
              <div>
                <h2>{{ t("settings.storage.policyTitle") }}</h2>
                <p>{{ t("settings.storage.policyHint") }}</p>
              </div>
            </div>
            <div class="settings-field">
              <label>{{ t("settings.storage.installRoot") }}</label>
              <div class="settings-path">
                <input
                  v-model="settingsDraft.installRoot"
                  type="text"
                  readonly
                />
                <button type="button" @click="chooseInstallRoot">{{ t("settings.storage.choose") }}</button>
                <small>{{ t("settings.storage.installRootHint") }}</small>
              </div>
            </div>
            <div class="settings-field-grid">
              <label>
                <span>{{ t("settings.storage.logRetention") }}</span>
                <div class="settings-number">
                  <input
                    v-model.number="settingsDraft.logRetentionDays"
                    type="number"
                    min="1"
                    max="365"
                    @change="saveSettings"
                  />
                  <em>{{ t("settings.storage.days") }}</em>
                </div>
              </label>
              <label>
                <span>{{ t("settings.storage.backupRetention") }}</span>
                <div class="settings-number">
                  <input
                    v-model.number="settingsDraft.backupRetentionCount"
                    type="number"
                    min="1"
                    max="100"
                    @change="saveSettings"
                  />
                  <em>{{ t("settings.storage.copies") }}</em>
                </div>
              </label>
            </div>
            <div class="settings-maintenance">
              <span>
                <strong>{{ t("settings.storage.cacheTitle") }}</strong>
                <small>{{ t("settings.storage.cacheHint") }}</small>
              </span>
              <button
                type="button"
                :disabled="allCacheCleaning"
                @click="cleanAllCaches"
              >
                <span v-if="allCacheCleaning" class="spinner"></span>
                {{ allCacheCleaning ? t("settings.storage.cleaning") : t("settings.storage.cleanNow") }}
              </button>
            </div>
          </section>

          <section
            v-show="activeSettingsTab === 'application'"
            class="settings-section"
            aria-labelledby="settings-tab-application"
          >
            <div class="settings-section-title">
              <div>
                <h2>{{ t("settings.application.updateTitle") }}</h2>
                <p>{{ t("settings.application.updateHint") }}</p>
              </div>
            </div>
            <label class="settings-toggle-row">
              <span>
                <strong>{{ t("settings.application.autoUpdateTitle") }}</strong>
                <small>{{ t("settings.application.autoUpdateHint") }}</small>
              </span>
              <input
                v-model="settingsDraft.autoCheckUpdates"
                type="checkbox"
                @change="saveSettings"
              />
              <i></i>
            </label>
            <div class="settings-update-row">
              <span>
                <strong>{{ updateStatus?.message ?? t("settings.application.notChecked") }}</strong>
                <small v-if="updateStatus">
                  {{ t("settings.application.currentVersion") }} {{ updateStatus.currentVersion }}
                  <template v-if="updateStatus.latestVersion">
                    · {{ t("settings.application.latestVersion") }} {{ updateStatus.latestVersion }}
                  </template>
                </small>
              </span>
              <button
                type="button"
                :disabled="updateChecking"
                @click="checkForUpdates"
              >
                <span v-if="updateChecking" class="spinner"></span>
                {{ updateChecking ? t("settings.application.checking") : t("settings.application.checkUpdates") }}
              </button>
            </div>
          </section>

          <section
            v-show="activeSettingsTab === 'about'"
            id="settings-panel-about"
            class="settings-section about-settings-section"
            role="tabpanel"
            aria-labelledby="settings-tab-about"
          >
            <div class="about-hero">
              <div class="about-mark" aria-hidden="true">
                <svg viewBox="0 0 48 48" role="img">
                  <rect x="3" y="3" width="42" height="42" rx="11" />
                  <path d="M13 15h23L20 33h16" />
                  <circle cx="13" cy="11" r="1.6" />
                  <circle cx="18" cy="11" r="1.6" />
                </svg>
              </div>
              <div>
                <span>ZHIYU ENVIRONMENT</span>
                <h2>{{ t("settings.about.productName") }}</h2>
                <p>{{ t("settings.about.tagline") }}</p>
              </div>
              <div class="about-version">
                <small>{{ t("settings.about.version") }}</small>
                <strong>v{{ appVersion }}</strong>
              </div>
            </div>

            <div class="about-introduction">
              <div>
                <span>ABOUT</span>
                <h3>{{ t("settings.about.introTitle") }}</h3>
              </div>
              <p>{{ t("settings.about.intro") }}</p>
            </div>

            <div class="about-capabilities">
              <article>
                <span>01</span>
                <div>
                  <strong>{{ t("settings.about.capabilities.services.title") }}</strong>
                  <small>{{ t("settings.about.capabilities.services.hint") }}</small>
                </div>
              </article>
              <article>
                <span>02</span>
                <div>
                  <strong>{{ t("settings.about.capabilities.tools.title") }}</strong>
                  <small>{{ t("settings.about.capabilities.tools.hint") }}</small>
                </div>
              </article>
              <article>
                <span>03</span>
                <div>
                  <strong>{{ t("settings.about.capabilities.local.title") }}</strong>
                  <small>{{ t("settings.about.capabilities.local.hint") }}</small>
                </div>
              </article>
            </div>

            <div class="about-links">
              <div>
                <span>OPEN SOURCE</span>
                <h3>{{ t("settings.about.contactTitle") }}</h3>
                <p>{{ t("settings.about.contactHint") }}</p>
              </div>
              <div class="about-link-list">
                <button type="button" @click="openExternal(PROJECT_REPOSITORY)">
                  <span>
                    <strong>{{ t("settings.about.repository") }}</strong>
                    <small>github.com/whoiszxl/zhiyu-env</small>
                  </span>
                  <em>↗</em>
                </button>
                <button type="button" @click="openExternal(PROJECT_ISSUES)">
                  <span>
                    <strong>{{ t("settings.about.feedback") }}</strong>
                    <small>{{ t("settings.about.feedbackHint") }}</small>
                  </span>
                  <em>↗</em>
                </button>
                <button type="button" @click="openExternal(PROJECT_AUTHOR)">
                  <span>
                    <strong>{{ t("settings.about.author") }}</strong>
                    <small>@whoiszxl</small>
                  </span>
                  <em>↗</em>
                </button>
              </div>
            </div>

            <footer class="about-footer">
              <span>Apache License 2.0</span>
              <p>{{ t("settings.about.footer") }}</p>
            </footer>
          </section>
        </div>
      </section>

      <section v-else-if="dashboardActive" class="dashboard-page">
        <header class="dashboard-header">
          <div>
            <span class="dashboard-eyebrow">{{ t("dashboard.eyebrow") }}</span>
            <h1>{{ t("dashboard.title") }}</h1>
            <p>{{ t("dashboard.subtitle") }}</p>
          </div>
          <div class="dashboard-header-actions">
            <button
              type="button"
              class="dashboard-diagnostics"
              :disabled="diagnosticsRunning"
              @click="openDiagnostics"
            >
              <span v-if="diagnosticsRunning" class="spinner"></span>
              {{ diagnosticsRunning ? "正在诊断" : "诊断与修复" }}
            </button>
            <button
              type="button"
              class="dashboard-stop-all"
              :disabled="runningServices.length === 0 || stoppingAll"
              @click="stopAllServices"
            >
              <span v-if="stoppingAll" class="spinner"></span>
              {{
                stoppingAll
                  ? "正在停止"
                  : `${t("dashboard.stopAll")}${runningServices.length ? ` (${runningServices.length})` : ""}`
              }}
            </button>
          </div>
        </header>

        <div v-if="notice" class="notice dashboard-notice">
          <span>{{ notice }}</span>
          <button type="button" @click="notice = ''">×</button>
        </div>
        <div v-if="error" class="notice danger dashboard-notice">
          <span>{{ error }}</span>
          <button type="button" @click="error = ''">×</button>
        </div>

        <div class="dashboard-body">
          <div class="dashboard-metrics">
            <article class="metric-accent-success">
              <span>{{ t("dashboard.runningServices") }}</span>
              <strong>{{ runningServices.length }}</strong>
              <small>{{ t("dashboard.totalServices", { count: services.length }) }}</small>
            </article>
            <article class="metric-accent-muted">
              <span>{{ t("dashboard.stopped") }}</span>
              <strong>{{ stoppedServiceCount }}</strong>
              <small>{{ t("dashboard.stoppedHint") }}</small>
            </article>
            <article>
              <span>{{ t("dashboard.installed") }}</span>
              <strong>{{ installedServiceCount }}</strong>
              <small>{{ t("dashboard.installedHint") }}</small>
            </article>
            <article class="metric-accent-muted">
              <span>{{ t("dashboard.notInstalled") }}</span>
              <strong>{{ notInstalledServiceCount }}</strong>
              <small>{{ t("dashboard.notInstalledHint") }}</small>
            </article>
            <article>
              <span>{{ t("dashboard.ports") }}</span>
              <strong>{{ dashboardPortListeners.length }}</strong>
              <small>{{ t("dashboard.portsHint") }}</small>
            </article>
            <article
              :class="{
                healthy: dashboardAlerts.length === 0,
                danger: dashboardAlerts.length > 0,
              }"
            >
              <span>{{ t("dashboard.exceptionsMetric") }}</span>
              <strong>{{ dashboardAlerts.length }}</strong>
              <small>{{
                dashboardAlerts.length === 0
                  ? t("dashboard.exceptionsHealthy")
                  : t("dashboard.exceptionsAttention")
              }}</small>
            </article>
            <article>
              <span>{{ t("dashboard.totalCpu") }}</span>
              <strong>{{ environmentMetrics.cpuPercent.toFixed(1) }}%</strong>
              <small>{{ t("dashboard.cpuHint") }}</small>
            </article>
            <article>
              <span>{{ t("dashboard.totalMemory") }}</span>
              <strong>{{ formatBytes(environmentMetrics.memoryBytes) }}</strong>
              <small>{{ t("dashboard.memoryHint") }}</small>
            </article>
            <article>
              <span>{{ t("dashboard.totalDisk") }}</span>
              <strong>{{ formatBytes(environmentDiskBytes) }}</strong>
              <small>{{ t("dashboard.diskHint") }}</small>
            </article>
            <article class="metric-accent-muted">
              <span>{{ t("dashboard.backupSize") }}</span>
              <strong>{{ formatBytes(dashboardBackupBytes) }}</strong>
              <small>{{ t("dashboard.backupHint") }}</small>
            </article>
            <article class="metric-accent-muted">
              <span>{{ t("dashboard.logsSize") }}</span>
              <strong>{{ formatBytes(dashboardLogsBytes) }}</strong>
              <small>{{ t("dashboard.logsHint") }}</small>
            </article>
            <article class="metric-accent-warning">
              <span>{{ t("dashboard.cacheSize") }}</span>
              <strong>{{ formatBytes(dashboardCacheBytes) }}</strong>
              <small>{{ t("dashboard.cacheHint") }}</small>
            </article>
          </div>

          <section
            v-if="dashboardAlerts.length > 0"
            class="dashboard-panel dashboard-alerts"
          >
            <div class="dashboard-panel-title">
              <div>
                <h2>{{ t("dashboard.exceptions") }}</h2>
                <p>PID、端口和最近操作</p>
              </div>
              <div class="dashboard-alert-actions">
                <button
                  v-if="services.some((service) => service.status === 'stale_pid' || service.status === 'crashed')"
                  type="button"
                  :disabled="repairingServices"
                  @click="repairAbnormalServices"
                >
                  <span v-if="repairingServices" class="spinner"></span>
                  {{ repairingServices ? "修复中" : "一键修复" }}
                </button>
                <span>{{ dashboardAlerts.length }}</span>
              </div>
            </div>
            <ul>
              <li
                v-for="alert in dashboardAlerts"
                :key="alert.message"
                :class="alert.level"
              >
                <i></i>
                <span>{{ alert.message }}</span>
              </li>
            </ul>
          </section>

          <div class="dashboard-grid">
            <section class="dashboard-panel service-status-panel">
              <div class="dashboard-panel-title">
                <div>
                  <h2>{{ t("dashboard.serviceStatus") }}</h2>
                  <p>点击服务进入详情</p>
                </div>
                <div
                  class="dashboard-service-summary"
                  :aria-label="`${runningServices.length} 个运行中，${services.length - runningServices.length} 个未运行`"
                >
                  <span title="运行中">
                    <i class="running"></i>{{ runningServices.length }}
                  </span>
                  <span title="未运行">
                    <i></i>{{ services.length - runningServices.length }}
                  </span>
                </div>
              </div>
              <div class="dashboard-service-grid">
                <button
                  v-for="service in services"
                  :key="service.kind"
                  type="button"
                  class="dashboard-service-row"
                  :aria-label="`${service.name}，${statusLabel[service.status]}，端口 ${service.port}`"
                  @click="selectService(service.kind)"
                >
                  <span class="nav-icon" :class="service.kind">
                    {{ iconLetter[service.kind] }}
                  </span>
                  <em
                    :class="service.status"
                    :title="statusLabel[service.status]"
                  ></em>
                  <strong>{{ service.name }}</strong>
                  <small>v{{ service.version }} · :{{ service.port }}</small>
                </button>
              </div>
            </section>

            <section class="dashboard-panel">
              <div class="dashboard-panel-title">
                <div>
                  <h2>{{ t("dashboard.portUsage") }}</h2>
                  <p>智屿服务与默认端口</p>
                </div>
                <span>{{ dashboardPortListeners.length }}</span>
              </div>
              <p
                v-if="dashboardPortListeners.length === 0"
                class="dashboard-empty"
              >
                暂无相关监听端口
              </p>
              <div v-else class="dashboard-port-list">
                <div
                  v-for="listener in dashboardPortListeners"
                  :key="`${listener.pid}-${listener.address}-${listener.port}`"
                >
                  <code>{{ listener.port }}</code>
                  <span>
                    <strong>{{
                      listener.managedService ?? listener.process
                    }}</strong>
                    <small>{{ listener.address }} · PID {{ listener.pid }}</small>
                  </span>
                  <em :class="{ managed: listener.managedService }">
                    {{ listener.managedService ? "智屿" : "占用" }}
                  </em>
                </div>
              </div>
            </section>

            <section class="dashboard-panel">
              <div class="dashboard-panel-title">
                <div>
                  <h2>{{ t("dashboard.diskRanking") }}</h2>
                  <p>程序、数据、日志与缓存</p>
                </div>
              </div>
              <p
                v-if="dashboardDiskRanking.length === 0"
                class="dashboard-empty"
              >
                暂无磁盘数据
              </p>
              <div v-else class="dashboard-disk-list">
                <div
                  v-for="item in dashboardDiskRanking"
                  :key="item.service.kind"
                >
                  <span>
                    <strong>{{ item.service.name }}</strong>
                    <em>{{ formatBytes(item.bytes) }}</em>
                  </span>
                  <i>
                    <b
                      :style="{
                        width: `${Math.max(
                          4,
                          (item.bytes / dashboardDiskRanking[0].bytes) * 100,
                        )}%`,
                      }"
                    ></b>
                  </i>
                </div>
              </div>
            </section>

            <section class="dashboard-panel">
              <div class="dashboard-panel-title">
                <div>
                  <h2>{{ t("dashboard.recentActivity") }}</h2>
                  <p>安装与生命周期记录</p>
                </div>
              </div>
              <p v-if="activityRecords.length === 0" class="dashboard-empty">
                {{ t("dashboard.noActivity") }}
              </p>
              <div v-else class="dashboard-activity-list">
                <div
                  v-for="record in activityRecords.slice(0, 8)"
                  :key="record.id"
                >
                  <i :class="{ failed: !record.success }"></i>
                  <span>
                    <strong>{{ record.service }} · {{ record.action }}</strong>
                    <small>{{ record.message }}</small>
                  </span>
                  <time>{{ formatActivityTime(record.createdAt) }}</time>
                </div>
              </div>
            </section>
          </div>
        </div>
      </section>

      <template v-else-if="activeTool === 'ssh'"></template>

      <div
        v-else-if="activeToolDefinition"
        v-tool-i18n
        class="tool-i18n-host"
      >
        <component :is="activeToolDefinition.component" />
      </div>

      <template v-else-if="selectedService">
        <header class="detail-header">
          <div class="detail-identity">
            <span class="service-logo" :class="selectedService.kind">
              {{ iconLetter[selectedService.kind] }}
            </span>
            <div>
              <div class="title-line">
                <h1>{{ selectedService.name }}</h1>
                <span>v{{ selectedService.version }}</span>
              </div>
              <p>
                <i
                  class="status-dot"
                  :class="[
                    selectedService.status,
                    { busy: serviceControlBusy },
                  ]"
                ></i>
                {{
                  redisVersionChanging
                    ? "版本安装与切换中"
                    : pendingAction
                    ? actionLabel[pendingAction]
                    : statusLabel[selectedService.status]
                }}
                <template v-if="selectedService.pid">
                  · PID {{ selectedService.pid }}
                </template>
              </p>
              <small
                v-if="
                  selectedService.status === 'not_installed' &&
                  !selectedService.installSupported
                "
                class="platform-unsupported"
              >
                {{ localizedInstallSupportLabel(selectedService.installSupportLabel) }}
              </small>
            </div>
          </div>

          <div class="header-actions">
            <button
              v-if="selectedService.status === 'not_installed'"
              class="primary"
              type="button"
              :disabled="
                serviceControlBusy || !selectedService.installSupported
              "
              :title="localizedInstallSupportLabel(selectedService.installSupportLabel)"
              @click="execute('install')"
            >
              <template v-if="pendingAction === 'install'">
                <span class="spinner"></span>
                <span>{{ t("common.installing") }}</span>
              </template>
              <span v-else>{{ t("common.install") }}</span>
            </button>
            <template v-else-if="selectedService.status === 'running'">
              <button
                type="button"
                :disabled="serviceControlBusy"
                @click="execute('restart')"
              >
                <span
                  v-if="pendingAction === 'restart'"
                  class="spinner"
                ></span>
                {{ pendingAction === "restart" ? t("common.restarting") : t("common.restart") }}
              </button>
              <button
                class="danger"
                type="button"
                :disabled="serviceControlBusy"
                @click="execute('stop')"
              >
                <span v-if="pendingAction === 'stop'" class="spinner"></span>
                {{ pendingAction === "stop" ? t("common.stopping") : t("common.stop") }}
              </button>
            </template>
            <button
              v-else
              class="primary"
              type="button"
              :disabled="serviceControlBusy"
              @click="execute('start')"
            >
              <span v-if="serviceControlBusy" class="spinner"></span>
              {{ serviceControlBusy ? t("common.processing") : t("common.startService") }}
            </button>
          </div>
        </header>

        <div v-if="notice || error" class="notice" :class="{ danger: error }">
          <span>{{ error || notice }}</span>
          <button type="button" @click="notice = error = ''">×</button>
        </div>

        <nav class="detail-tabs">
          <button
            v-for="tab in detailTabs"
            :key="tab[0]"
            type="button"
            :class="{ active: activeTab === tab[0] }"
            @click="openTab(tab[0] as DetailTab)"
          >
            {{ t(`serviceTabs.${tab[0]}`) }}
          </button>
        </nav>

        <section
          v-if="activeTab === 'versions' && selectedKind === 'redis'"
          class="version-panel"
        >
          <div class="redis-version-manager">
            <div class="redis-version-head">
              <div>
                <p>VERSION MANAGER</p>
                <h2>{{ t("versions.title", { service: "Redis" }) }}</h2>
              </div>
              <span>{{ t("versions.isolatedBinary") }}</span>
            </div>

            <div
              v-if="redisVersionsLoading && redisVersions.length === 0"
              class="redis-version-loading"
            >
              {{ t("versions.loading") }}
            </div>
            <div v-else class="redis-version-grid">
              <button
                v-for="release in redisVersions"
                :key="release.version"
                type="button"
                :class="{
                  selected: redisVersionTarget === release.version,
                  active: release.selected,
                  legacy: release.legacy,
                }"
                :disabled="redisVersionChanging"
                @click="redisVersionTarget = release.version"
              >
                <span class="redis-version-radio"></span>
                <span class="redis-version-copy">
                  <strong>Redis {{ release.series }}</strong>
                  <small>v{{ release.version }}</small>
                </span>
                <span class="redis-version-badges">
                  <i v-if="release.selected">{{ t("versions.current") }}</i>
                  <i v-else-if="release.installed">{{ t("versions.installed") }}</i>
                  <i v-if="release.recommended" class="recommended">
                    {{ t("versions.recommended") }}
                  </i>
                </span>
                <em>
                  {{ localizedSupportLabel(release.supportLabel) }}
                  <template v-if="!selectedService.installSupported">
                    · {{ t("versions.unsupported") }}
                  </template>
                </em>
              </button>
            </div>

            <div class="redis-version-footer">
              <p>
                {{ t("versions.redisNote") }}
              </p>
              <div>
                <span
                  v-if="
                    selectedService.status === 'running' &&
                    !selectedRedisVersionInfo?.selected
                  "
                >
                  {{ t("versions.stopFirst", { service: "Redis" }) }}
                </span>
                <button
                  v-if="selectedRedisVersionInfo?.installed"
                  type="button"
                  class="version-remove-button"
                  :disabled="
                    serviceControlBusy ||
                    (selectedRedisVersionInfo.selected &&
                      selectedService.status === 'running')
                  "
                  @click="
                    requestVersionUninstall(
                      'redis',
                      'Redis',
                      selectedRedisVersionInfo,
                    )
                  "
                >
                  {{ t("versions.uninstallProgram") }}
                </button>
                <button
                  type="button"
                  :disabled="
                    !selectedRedisVersionInfo ||
                    selectedRedisVersionInfo.selected ||
                    selectedService.status === 'running' ||
                    !selectedService.installSupported ||
                    serviceControlBusy
                  "
                  @click="changeRedisVersion"
                >
                  <span v-if="redisVersionChanging" class="spinner"></span>
                  {{
                    selectedRedisVersionInfo?.selected
                      ? t("versions.currentVersion")
                      : redisVersionChanging
                        ? t("versions.switching")
                        : selectedRedisVersionInfo?.installed
                          ? t("versions.switch")
                          : t("versions.installAndSwitch")
                  }}
                </button>
              </div>
            </div>
          </div>
        </section>

        <section
          v-else-if="activeTab === 'versions' && selectedKind === 'mysql'"
          class="version-panel"
        >
          <div class="redis-version-manager">
            <div class="redis-version-head">
              <div>
                <p>VERSION MANAGER</p>
                <h2>{{ t("versions.title", { service: "MySQL" }) }}</h2>
              </div>
              <span>{{ t("versions.isolatedData") }}</span>
            </div>

            <div
              v-if="mysqlVersionsLoading && mysqlVersions.length === 0"
              class="redis-version-loading"
            >
              {{ t("versions.loading") }}
            </div>
            <div v-else class="redis-version-grid">
              <button
                v-for="release in mysqlVersions"
                :key="release.version"
                type="button"
                :class="{
                  selected: mysqlVersionTarget === release.version,
                  active: release.selected,
                  legacy: release.legacy,
                }"
                :disabled="mysqlVersionChanging"
                @click="mysqlVersionTarget = release.version"
              >
                <span class="redis-version-radio"></span>
                <span class="redis-version-copy">
                  <strong>MySQL {{ release.series }}</strong>
                  <small>v{{ release.version }}</small>
                </span>
                <span class="redis-version-badges">
                  <i v-if="release.selected">{{ t("versions.current") }}</i>
                  <i v-else-if="release.installed">{{ t("versions.installed") }}</i>
                  <i v-if="release.recommended" class="recommended">
                    {{ t("versions.recommended") }}
                  </i>
                </span>
                <em>
                  {{ localizedSupportLabel(release.supportLabel) }}
                  <template v-if="!selectedService.installSupported">
                    · {{ t("versions.unsupported") }}
                  </template>
                </em>
              </button>
            </div>

            <div class="redis-version-footer">
              <p>
                {{ t("versions.mysqlNote") }}
              </p>
              <div>
                <span
                  v-if="
                    selectedService.status === 'running' &&
                    !selectedMysqlVersionInfo?.selected
                  "
                >
                  {{ t("versions.stopFirst", { service: "MySQL" }) }}
                </span>
                <button
                  v-if="selectedMysqlVersionInfo?.installed"
                  type="button"
                  class="version-remove-button"
                  :disabled="
                    serviceControlBusy ||
                    (selectedMysqlVersionInfo.selected &&
                      selectedService.status === 'running')
                  "
                  @click="
                    requestVersionUninstall(
                      'mysql',
                      'MySQL',
                      selectedMysqlVersionInfo,
                    )
                  "
                >
                  {{ t("versions.uninstallProgram") }}
                </button>
                <button
                  type="button"
                  :disabled="
                    !selectedMysqlVersionInfo ||
                    selectedMysqlVersionInfo.selected ||
                    selectedService.status === 'running' ||
                    !selectedService.installSupported ||
                    serviceControlBusy
                  "
                  @click="changeMysqlVersion"
                >
                  <span v-if="mysqlVersionChanging" class="spinner"></span>
                  {{
                    selectedMysqlVersionInfo?.selected
                      ? t("versions.currentVersion")
                      : mysqlVersionChanging
                        ? t("versions.initializing")
                        : selectedMysqlVersionInfo?.installed
                          ? t("versions.switch")
                          : t("versions.installAndSwitch")
                  }}
                </button>
              </div>
            </div>
          </div>
        </section>

        <section
          v-else-if="activeTab === 'versions' && selectedKind === 'postgres'"
          class="version-panel"
        >
          <div class="redis-version-manager">
            <div class="redis-version-head">
              <div>
                <p>VERSION MANAGER</p>
                <h2>{{ t("versions.title", { service: "PostgreSQL" }) }}</h2>
              </div>
              <span>{{ t("versions.isolatedSource") }}</span>
            </div>

            <div
              v-if="
                postgresVersionsLoading && postgresVersions.length === 0
              "
              class="redis-version-loading"
            >
              {{ t("versions.loading") }}
            </div>
            <div v-else class="redis-version-grid">
              <button
                v-for="release in postgresVersions"
                :key="release.version"
                type="button"
                :class="{
                  selected: postgresVersionTarget === release.version,
                  active: release.selected,
                  legacy: release.legacy,
                }"
                :disabled="postgresVersionChanging"
                @click="postgresVersionTarget = release.version"
              >
                <span class="redis-version-radio"></span>
                <span class="redis-version-copy">
                  <strong>PostgreSQL {{ release.series }}</strong>
                  <small>v{{ release.version }}</small>
                </span>
                <span class="redis-version-badges">
                  <i v-if="release.selected">{{ t("versions.current") }}</i>
                  <i v-else-if="release.installed">{{ t("versions.installed") }}</i>
                  <i v-if="release.recommended" class="recommended">
                    {{ t("versions.recommended") }}
                  </i>
                </span>
                <em>
                  {{ localizedSupportLabel(release.supportLabel) }}
                  <template v-if="!selectedService.installSupported">
                    · {{ t("versions.unsupported") }}
                  </template>
                </em>
              </button>
            </div>

            <div class="redis-version-footer">
              <p>
                {{ t("versions.postgresNote") }}
              </p>
              <div>
                <span
                  v-if="
                    selectedService.status === 'running' &&
                    !selectedPostgresVersionInfo?.selected
                  "
                >
                  {{ t("versions.stopFirst", { service: "PostgreSQL" }) }}
                </span>
                <button
                  v-if="selectedPostgresVersionInfo?.installed"
                  type="button"
                  class="version-remove-button"
                  :disabled="
                    serviceControlBusy ||
                    (selectedPostgresVersionInfo.selected &&
                      selectedService.status === 'running')
                  "
                  @click="
                    requestVersionUninstall(
                      'postgres',
                      'PostgreSQL',
                      selectedPostgresVersionInfo,
                    )
                  "
                >
                  {{ t("versions.uninstallProgram") }}
                </button>
                <button
                  type="button"
                  :disabled="
                    !selectedPostgresVersionInfo ||
                    selectedPostgresVersionInfo.selected ||
                    selectedService.status === 'running' ||
                    !selectedService.installSupported ||
                    serviceControlBusy
                  "
                  @click="changePostgresVersion"
                >
                  <span
                    v-if="postgresVersionChanging"
                    class="spinner"
                  ></span>
                  {{
                    selectedPostgresVersionInfo?.selected
                      ? t("versions.currentVersion")
                      : postgresVersionChanging
                        ? t("versions.compiling")
                        : selectedPostgresVersionInfo?.installed
                          ? t("versions.switch")
                          : t("versions.installAndSwitch")
                  }}
                </button>
              </div>
            </div>
          </div>
        </section>

        <section
          v-else-if="
            activeTab === 'versions' &&
            !['redis', 'mysql', 'postgres', 'nginx'].includes(selectedKind)
          "
          class="version-panel"
        >
          <div class="redis-version-manager">
            <div class="redis-version-head">
              <div>
                <p>VERSION MANAGER</p>
                <h2>{{ t("versions.title", { service: selectedService.name }) }}</h2>
              </div>
              <span>{{ t("versions.verifiedPackages") }}</span>
            </div>

            <div
              v-if="
                genericMultiVersionKinds.includes(selectedKind) &&
                managedVersionsLoading
              "
              class="panel-state"
            >
              {{ t("versions.loadingVerified") }}
            </div>

            <template
              v-else-if="genericMultiVersionKinds.includes(selectedKind)"
            >
              <div
                class="redis-version-grid"
                :class="{ 'two-columns': managedVersions.length === 2 }"
              >
                <button
                  v-for="release in managedVersions"
                  :key="release.version"
                  type="button"
                  :class="{
                    selected: managedVersionTarget === release.version,
                    active: release.selected,
                    legacy: release.legacy,
                  }"
                  :disabled="managedVersionChanging"
                  @click="managedVersionTarget = release.version"
                >
                  <span class="redis-version-radio"></span>
                  <span class="redis-version-copy">
                    <strong>{{ selectedService.name }} {{ release.series }}</strong>
                    <small>v{{ release.version }}</small>
                  </span>
                  <span class="redis-version-badges">
                    <i v-if="release.selected">{{ t("versions.current") }}</i>
                    <i v-else-if="release.installed">{{ t("versions.installed") }}</i>
                    <i v-if="release.recommended" class="recommended">{{ t("versions.recommended") }}</i>
                  </span>
                  <em>
                    {{ localizedSupportLabel(release.supportLabel) }}
                    <template v-if="release.installationBytes > 0">
                      · {{ formatBytes(release.installationBytes) }}
                    </template>
                  </em>
                </button>
              </div>

              <footer class="redis-version-footer">
                <p>
                  {{ t("versions.genericNote") }}
                </p>
                <div>
                  <button
                    v-if="selectedManagedVersionInfo?.installed"
                    type="button"
                    class="version-remove-button"
                    :disabled="
                      serviceControlBusy ||
                      (selectedManagedVersionInfo.selected &&
                        selectedService.status === 'running')
                    "
                    @click="
                      requestVersionUninstall(
                        selectedService.kind,
                        selectedService.name,
                        selectedManagedVersionInfo,
                      )
                    "
                  >
                    {{ t("versions.uninstallVersion") }}
                  </button>
                  <button
                    type="button"
                    :disabled="
                      !selectedManagedVersionInfo ||
                      selectedManagedVersionInfo.selected ||
                      selectedService.status === 'running' ||
                      !selectedService.installSupported ||
                      serviceControlBusy
                    "
                    @click="changeManagedVersion"
                  >
                    <span
                      v-if="managedVersionChanging"
                      class="spinner"
                    ></span>
                    {{
                      selectedManagedVersionInfo?.selected
                        ? t("versions.currentVersion")
                        : managedVersionChanging
                          ? t("versions.switching")
                          : selectedManagedVersionInfo?.installed
                            ? t("versions.switch")
                            : t("versions.installAndSwitch")
                    }}
                  </button>
                </div>
              </footer>
            </template>

            <template v-else>
              <div class="redis-version-grid">
                <button
                  type="button"
                  class="selected active"
                  disabled
                >
                  <span class="redis-version-radio"></span>
                  <span class="redis-version-copy">
                    <strong>
                      {{ selectedService.name }} {{ selectedService.version }}
                    </strong>
                    <small>v{{ selectedService.version }}</small>
                  </span>
                  <span class="redis-version-badges">
                    <i>{{ t("versions.current") }}</i>
                    <i
                      v-if="selectedService.status !== 'not_installed'"
                    >
                      {{ t("versions.installed") }}
                    </i>
                  </span>
                  <em>
                    {{ localizedInstallSupportLabel(selectedService.installSupportLabel) }}
                    <template
                      v-if="
                        (selectedDiskUsage?.installationBytes ?? 0) > 0
                      "
                    >
                      ·
                      {{
                        formatBytes(
                          selectedDiskUsage?.installationBytes ?? 0,
                        )
                      }}
                    </template>
                  </em>
                </button>
              </div>

              <footer class="redis-version-footer">
                <p>
                  {{ t("versions.singleNote") }}
                </p>
                <div>
                  <span v-if="selectedService.status === 'running'">
                    {{ t("versions.stopBeforeUninstall") }}
                  </span>
                  <button
                    v-if="selectedService.status !== 'not_installed'"
                    type="button"
                    class="version-remove-button"
                    :disabled="
                      serviceControlBusy ||
                      selectedService.status === 'running'
                    "
                    @click="requestCurrentProgramUninstall"
                  >
                    {{ t("versions.uninstallProgram") }}
                  </button>
                  <button
                    v-else
                    type="button"
                    :disabled="
                      serviceControlBusy ||
                      !selectedService.installSupported
                    "
                    @click="execute('install')"
                  >
                    <span
                      v-if="pendingAction === 'install'"
                      class="spinner"
                    ></span>
                    {{
                      pendingAction === "install"
                        ? t("common.installing")
                        : t("common.install")
                    }}
                  </button>
                </div>
              </footer>
            </template>
          </div>
        </section>

        <section v-else-if="activeTab === 'overview'" class="overview">
          <div class="metric-grid">
            <article class="metric-card">
              <p>MEMORY</p>
              <strong>{{ formatBytes(metrics.memoryBytes) }}</strong>
              <small>{{ t("serviceOverview.memoryHint") }}</small>
            </article>
            <article class="metric-card">
              <p>CPU</p>
              <strong>{{
                metrics.cpuPercent === null
                  ? "—"
                  : `${metrics.cpuPercent.toFixed(1)}%`
              }}</strong>
              <small>{{ t("serviceOverview.cpuHint") }}</small>
            </article>
            <article class="metric-card">
              <p>UPTIME</p>
              <strong>{{ metrics.uptime ?? "—" }}</strong>
              <small>{{ t("serviceOverview.uptimeHint") }}</small>
            </article>
            <article class="metric-card">
              <p>DISK</p>
              <strong>{{
                formatBytes(selectedDiskUsage?.totalBytes ?? null)
              }}</strong>
              <small>{{ t("serviceOverview.diskHint") }}</small>
            </article>
            <article class="metric-card">
              <p>ENDPOINT</p>
              <strong class="endpoint"
                >127.0.0.1:{{ selectedService.port }}</strong
              >
              <small>{{ t("serviceOverview.endpointHint") }}</small>
            </article>
          </div>

          <div v-if="selectedDiskUsage" class="disk-usage-strip">
            <div class="program-usage-cell">
              <span>
                {{ t("serviceOverview.programFiles") }}
                <button
                  v-if="selectedService.status !== 'not_installed'"
                  type="button"
                  :disabled="
                    serviceControlBusy ||
                    selectedService.status === 'running' ||
                    selectedDiskUsage.installationBytes === 0
                  "
                  @click="requestCurrentProgramUninstall"
                >
                  {{
                    selectedService.status === "running"
                      ? t("status.running")
                      : t("serviceOverview.uninstall")
                  }}
                </button>
              </span>
              <strong>{{
                formatBytes(selectedDiskUsage.installationBytes)
              }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.businessData") }}</span>
              <strong>{{ formatBytes(selectedDiskUsage.dataBytes) }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.runtimeLogs") }}</span>
              <strong>{{ formatBytes(selectedDiskUsage.logsBytes) }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.configFiles") }}</span>
              <strong>{{ formatBytes(selectedDiskUsage.configBytes) }}</strong>
            </div>
            <div class="cache-usage-cell">
              <span>
                {{ t("serviceOverview.downloadCache") }}
                <button
                  type="button"
                  :disabled="
                    cacheCleaning || selectedDiskUsage.cacheBytes === 0
                  "
                  @click="clearInstallCache"
                >
                  {{ cacheCleaning ? t("serviceOverview.cleaning") : t("serviceOverview.clean") }}
                </button>
              </span>
              <strong>{{ formatBytes(selectedDiskUsage.cacheBytes) }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.backupFiles") }}</span>
              <strong>{{ formatBytes(selectedDiskUsage.backupBytes) }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.otherFiles") }}</span>
              <strong>{{ formatBytes(selectedDiskUsage.otherBytes) }}</strong>
            </div>
          </div>

          <div
            v-if="selectedKind === 'redis' && redisOverview"
            class="redis-stat-strip"
          >
            <div>
              <span>{{ t("serviceOverview.redisMemory") }}</span>
              <strong>{{ formatBytes(redisOverview.usedMemoryBytes) }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.keyCount") }}</span>
              <strong>{{ redisOverview.totalKeys }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.connections") }}</span>
              <strong>{{ redisOverview.connectedClients }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.operationsPerSecond") }}</span>
              <strong>{{ redisOverview.operationsPerSecond }}</strong>
            </div>
          </div>

          <div
            v-if="
              (selectedKind === 'mysql' || selectedKind === 'postgres') &&
              databaseOverview
            "
            class="redis-stat-strip"
          >
            <div>
              <span>{{ t("serviceOverview.databases") }}</span>
              <strong>{{ databaseOverview.databaseCount }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.tables") }}</span>
              <strong>{{ databaseOverview.tableCount }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.activeConnections") }}</span>
              <strong>{{ databaseOverview.connectionCount }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.dataSize") }}</span>
              <strong>{{ formatBytes(databaseOverview.dataSizeBytes) }}</strong>
            </div>
          </div>

          <div
            v-if="selectedKind === 'mongodb' && mongoOverview"
            class="redis-stat-strip"
          >
            <div>
              <span>{{ t("serviceOverview.databases") }}</span>
              <strong>{{ mongoOverview.databaseCount }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.mongoVersion") }}</span>
              <strong>{{ mongoOverview.version }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.activeConnections") }}</span>
              <strong>{{ mongoOverview.connectionCount }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.dataSize") }}</span>
              <strong>{{ formatBytes(mongoOverview.dataSizeBytes) }}</strong>
            </div>
          </div>

          <div
            v-if="selectedKind === 'mailpit' && mailpitOverview"
            class="redis-stat-strip"
          >
            <div>
              <span>{{ t("serviceOverview.capturedMail") }}</span>
              <strong>{{ mailpitOverview.total }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.unreadMail") }}</span>
              <strong>{{ mailpitOverview.unread }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.smtpAddress") }}</span>
              <strong class="small-value">{{
                mailpitOverview.smtpAddress
              }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.webAddress") }}</span>
              <strong class="small-value">{{
                mailpitOverview.webAddress
              }}</strong>
            </div>
          </div>

          <div
            v-if="selectedKind === 'nats' && natsOverview"
            class="redis-stat-strip"
          >
            <div>
              <span>{{ t("serviceOverview.activeConnections") }}</span>
              <strong>{{ natsOverview.connections }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.subscriptions") }}</span>
              <strong>{{ natsOverview.subscriptions }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.incomingMessages") }}</span>
              <strong>{{ natsOverview.inMessages }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.outgoingMessages") }}</span>
              <strong>{{ natsOverview.outMessages }}</strong>
            </div>
          </div>

          <div
            v-if="selectedKind === 'kafka' && kafkaOverview"
            class="redis-stat-strip"
          >
            <div>
              <span>{{ t("serviceOverview.compatibleProtocol") }}</span>
              <strong>Kafka API</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.topics") }}</span>
              <strong>{{ kafkaOverview.topicCount }}</strong>
            </div>
            <div>
              <span>Broker</span>
              <strong class="small-value">127.0.0.1:9092</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.storageEngine") }}</span>
              <strong>{{ kafkaOverview.storageEngine }}</strong>
            </div>
          </div>

          <div
            v-if="selectedKind === 'meilisearch' && meilisearchOverview"
            class="redis-stat-strip"
          >
            <div>
              <span>{{ t("serviceOverview.indexes") }}</span>
              <strong>{{ meilisearchOverview.indexCount }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.documents") }}</span>
              <strong>{{ meilisearchOverview.documentCount }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.databaseSize") }}</span>
              <strong>{{
                formatBytes(meilisearchOverview.databaseSizeBytes)
              }}</strong>
            </div>
            <div>
              <span>{{ t("serviceOverview.indexingTasks") }}</span>
              <strong>{{ meilisearchOverview.indexingCount }}</strong>
            </div>
          </div>

          <div class="overview-columns">
            <article class="panel monitoring-panel">
              <div class="panel-title">
                <div>
                  <p>LIVE MONITORING</p>
                  <h2>{{ t("serviceOverview.liveResources") }}</h2>
                </div>
                <span class="live-badge"><i></i>{{ t("serviceOverview.refreshInterval") }}</span>
              </div>

              <div
                v-if="selectedService.status !== 'running'"
                class="chart-empty"
              >
                {{ t("serviceOverview.chartEmpty") }}
              </div>
              <div v-else class="charts">
                <div class="chart-block">
                  <div class="chart-label">
                    <span>{{ t("serviceOverview.memoryUsage") }}</span>
                    <strong>{{ formatBytes(metrics.memoryBytes) }}</strong>
                  </div>
                  <svg viewBox="0 0 560 112" preserveAspectRatio="none">
                    <line x1="0" y1="106" x2="560" y2="106"></line>
                    <polyline
                      :points="
                        chartPoints(
                          metricHistory.map((point) => point.memory),
                        )
                      "
                      class="memory-line"
                    ></polyline>
                  </svg>
                </div>
                <div class="chart-block compact">
                  <div class="chart-label">
                    <span>CPU</span>
                    <strong
                      >{{ (metrics.cpuPercent ?? 0).toFixed(1) }}%</strong
                    >
                  </div>
                  <svg viewBox="0 0 560 72" preserveAspectRatio="none">
                    <line x1="0" y1="66" x2="560" y2="66"></line>
                    <polyline
                      :points="
                        chartPoints(
                          metricHistory.map((point) => point.cpu),
                          560,
                          72,
                        )
                      "
                      class="cpu-line"
                    ></polyline>
                  </svg>
                </div>
              </div>
            </article>

            <article class="panel paths-panel">
              <div class="panel-title">
                <div>
                  <p>RUNTIME</p>
                  <h2>{{ t("serviceOverview.serviceInfo") }}</h2>
                </div>
              </div>
              <dl class="info-list">
                <div>
                  <dt>{{ t("serviceOverview.runtimeStatus") }}</dt>
                  <dd>{{ statusLabel[selectedService.status] }}</dd>
                </div>
                <div>
                  <dt>{{ t("serviceOverview.processPid") }}</dt>
                  <dd>{{ selectedService.pid ?? "—" }}</dd>
                </div>
                <div>
                  <dt>{{ t("serviceOverview.configFile") }}</dt>
                  <dd :title="selectedService.configPath">
                    {{ selectedService.configPath }}
                  </dd>
                </div>
                <div>
                  <dt>{{ t("serviceOverview.dataDirectory") }}</dt>
                  <dd :title="selectedService.dataPath">
                    {{ selectedService.dataPath }}
                  </dd>
                </div>
                <div>
                  <dt>{{ t("serviceOverview.executable") }}</dt>
                  <dd :title="selectedService.executablePath">
                    {{ selectedService.executablePath }}
                  </dd>
                </div>
              </dl>
            </article>
          </div>
        </section>

        <section
          v-else-if="activeTab === 'site' && (selectedKind === 'nginx' || selectedKind === 'caddy')"
          class="nginx-site-panel"
        >
          <div class="ns-site-head">
            <div>
              <p>SITE</p>
              <h2>站点管理</h2>
            </div>
          </div>
          <div class="ns-site-grid">
            <div class="ns-site-card">
              <strong>本地访问地址</strong>
              <div class="ns-code-row">
                <code>http://127.0.0.1:{{ selectedService.port }}</code>
                <button class="ns-icon-btn" title="在浏览器打开" @click="openExternal('http://127.0.0.1:' + selectedService.port)">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
                </button>
              </div>
              <span>仅监听本地，不可公网访问</span>
            </div>
            <div class="ns-site-card">
              <strong>静态站点目录</strong>
              <div class="ns-code-row">
                <code>{{ selectedService.instanceDir }}/html</code>
                <button class="ns-icon-btn" title="在 Finder 中打开" @click="openPath(selectedService.instanceDir + '/html')">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/></svg>
                </button>
              </div>
              <span>index.html 为默认首页</span>
            </div>
            <div class="ns-site-card">
              <strong>Access Log</strong>
              <div class="ns-code-row">
                <code>{{ selectedService.instanceDir }}/logs/access.log</code>
                <button class="ns-icon-btn" title="在 Finder 中打开" @click="openPath(selectedService.instanceDir + '/logs')">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/></svg>
                </button>
              </div>
              <span>HTTP 请求记录</span>
            </div>
            <div class="ns-site-card">
              <strong>Error Log</strong>
              <div class="ns-code-row">
                <code>{{ selectedService.instanceDir }}/logs/error.log</code>
                <button class="ns-icon-btn" title="在 Finder 中打开" @click="openPath(selectedService.instanceDir + '/logs')">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/></svg>
                </button>
              </div>
              <span>错误与警告信息</span>
            </div>
          </div>
          <p class="ns-site-note">
            修改端口和反向代理请在「配置文件」标签页编辑
            {{ selectedKind === 'nginx' ? 'nginx.conf' : 'Caddyfile' }}，
            {{ selectedKind === 'nginx' ? '保存后使用 nginx -t 自动校验。' : 'Caddy 会在启动时自动校验配置。' }}
          </p>
        </section>

        <section
          v-else-if="activeTab === 'files' && (selectedKind === 'nginx' || selectedKind === 'caddy')"
          class="nginx-files-panel"
        >
          <div class="nf-head">
            <div>
              <p>FILES</p>
              <h2>文件管理</h2>
            </div>
            <button type="button" @click="loadNginxFiles(nginxFilesDir || undefined)" :disabled="nginxFilesLoading">刷新</button>
          </div>
          <div class="nf-toolbar">
            <input v-model="nginxNewFileName" type="text" placeholder="新文件名，例如 style.css" spellcheck="false" @keydown.enter.prevent="createNginxFile" />
            <button type="button" class="primary" @click="createNginxFile" :disabled="!nginxNewFileName.trim()">新建</button>
            <button type="button" class="ns-outline-btn" @click="openPath((selectedService?.dataPath ?? '') + '/html')">在 Finder 打开</button>
          </div>
          <div v-if="nginxFilesLoading" class="panel-state">加载中…</div>
          <div v-else-if="nginxFiles.length === 0" class="nf-empty">
            <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#b4b6ae" stroke-width="1.2"><path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z"/></svg>
            <p>此目录暂无文件</p>
            <span>通过上方工具栏新建，或在 Finder 中拖入文件</span>
          </div>
          <div v-else class="nf-table">
            <div class="nf-row nf-row-head">
              <span>名称</span>
              <span>大小</span>
              <span>操作</span>
            </div>
            <button
              v-if="nginxFilesDir"
              type="button"
              class="nf-row nf-folder-row"
              @click="loadNginxFiles(nginxFilesDir.split('/').slice(0, -1).join('/') || undefined)"
            >📁 ..</button>
            <template v-for="f in nginxFiles" :key="f.path">
              <button
                v-if="f.isDir"
                type="button"
                class="nf-row nf-folder-row"
                @click="loadNginxFiles(f.path)"
              >
                <span>📁 {{ f.name }}</span>
                <span>—</span>
                <span></span>
              </button>
              <div v-else class="nf-row">
                <span class="nf-name" :title="f.path">{{ f.name }}</span>
                <span>{{ formatBytes(f.sizeBytes) }}</span>
                <span class="nf-actions">
                  <button type="button" class="clip-btn" @click="editNginxFile(f.path)">编辑</button>
                  <button type="button" class="clip-btn remove" @click="deleteNginxFile(f.path, false)">删除</button>
                </span>
              </div>
            </template>
          </div>
          <div v-if="nginxEditingFile" class="nf-editor">
            <div class="nf-editor-head">
              <strong>{{ nginxEditingFile }}</strong>
              <div>
                <button type="button" @click="saveNginxFile" :disabled="!nginxFileModified || nginxEditingSaving">
                  {{ nginxEditingSaving ? "保存中…" : "保存" }}
                </button>
                <button type="button" @click="closeNginxEditor">关闭</button>
              </div>
            </div>
            <textarea
              v-model="nginxEditingContent"
              class="nf-editor-textarea"
              spellcheck="false"
            ></textarea>
          </div>
        </section>

        <section
          v-else-if="activeTab === 'versions' && selectedKind === 'nginx'"
          class="version-panel"
        >
          <div class="redis-version-manager">
          <div class="redis-version-head">
            <div>
              <p>VERSION MANAGER</p>
              <h2>{{ t("versions.title", { service: "Nginx" }) }}</h2>
            </div>
            <span>{{ t("versions.nginxSource") }}</span>
          </div>
          <div v-if="nginxVersionsLoading" class="panel-state">
            {{ t("versions.loadingVerified") }}
          </div>
          <template v-else>
            <div
              class="redis-version-grid"
              :class="{ 'two-columns': nginxVersions.length === 2 }"
            >
              <button
                v-for="release in nginxVersions"
                :key="release.version"
                type="button"
                :class="{
                  selected: nginxVersionTarget === release.version,
                  active: release.selected,
                  legacy: release.legacy,
                }"
                :disabled="nginxVersionChanging"
                @click="nginxVersionTarget = release.version"
              >
                <span class="redis-version-radio"></span>
                <span class="redis-version-copy">
                  <strong>Nginx {{ release.series }}</strong>
                  <small>v{{ release.version }}</small>
                </span>
                <span class="redis-version-badges">
                  <i v-if="release.selected">{{ t("versions.current") }}</i>
                  <i v-else-if="release.installed">{{ t("versions.installed") }}</i>
                  <i v-if="release.recommended" class="recommended">{{ t("versions.recommended") }}</i>
                </span>
                <em>
                  {{ localizedSupportLabel(release.supportLabel) }}
                  <template v-if="release.installationBytes > 0">
                    · {{ formatBytes(release.installationBytes) }}
                  </template>
                </em>
              </button>
            </div>
            <p class="ns-ver-note">
              {{ t("versions.nginxVerifiedNote") }}
            </p>
            <div class="redis-version-footer">
              <p>
                {{ t("versions.nginxNote") }}
              </p>
              <div>
              <span v-if="selectedService.status === 'running'">
                {{ t("versions.nginxStop") }}
              </span>
              <button
                v-if="selectedNginxVersionInfo?.installed"
                type="button"
                class="version-remove-button"
                :disabled="
                  serviceControlBusy ||
                  (selectedNginxVersionInfo.selected &&
                    selectedService.status === 'running')
                "
                @click="
                  requestVersionUninstall(
                    'nginx',
                    'Nginx',
                    selectedNginxVersionInfo,
                  )
                "
              >
                {{ t("versions.uninstallVersion") }}
              </button>
              <button
                type="button"
                :disabled="
                  !selectedNginxVersionInfo ||
                  selectedNginxVersionInfo.selected ||
                  selectedService.status === 'running' ||
                  !selectedService.installSupported ||
                  serviceControlBusy
                "
                @click="changeNginxVersion"
              >
                <span v-if="nginxVersionChanging" class="spinner"></span>
                {{
                  selectedNginxVersionInfo?.selected
                    ? t("versions.currentVersion")
                    : nginxVersionChanging
                      ? t("versions.compileSwitching")
                      : selectedNginxVersionInfo?.installed
                        ? t("versions.switch")
                        : t("versions.installAndSwitch")
                }}
              </button>
              </div>
            </div>
          </template>
          </div>
        </section>

        <section v-else-if="activeTab === 'keys'" class="redis-workbench">
          <div class="redis-toolbar">
            <label>
              数据库
              <select
                v-model.number="redisDatabase"
                @change="changeRedisDatabase"
              >
                <option v-for="database in 16" :key="database - 1" :value="database - 1">
                  DB {{ database - 1 }}
                </option>
              </select>
            </label>
            <label class="key-search">
              Key
              <input
                v-model="redisPattern"
                type="search"
                placeholder="例如 user:*"
                @keydown.enter.prevent="loadRedisKeys(true)"
              />
            </label>
            <button
              type="button"
              :disabled="
                redisKeysLoading || selectedService.status !== 'running'
              "
              @click="loadRedisKeys(true)"
            >
              {{ redisKeysLoading ? "查询中" : "查询" }}
            </button>
            <button type="button" @click="openRedisAiAssistant">
              ✦ AI 分析
            </button>
          </div>

          <div
            v-if="selectedService.status !== 'running'"
            class="workbench-empty"
          >
            启动 Redis 后即可浏览本地数据
          </div>
          <div v-else class="key-browser">
            <aside class="key-list">
              <div class="key-list-head">
                <strong>KEYS</strong>
                <span>{{ redisKeys.length }}</span>
              </div>
              <div v-if="redisKeys.length === 0" class="key-list-empty">
                {{ redisKeysLoading ? "正在读取…" : "没有匹配的 Key" }}
              </div>
              <button
                v-for="key in redisKeys"
                :key="key"
                type="button"
                :class="{ active: selectedRedisKey === key }"
                :title="key"
                @click="selectRedisKey(key)"
              >
                {{ key }}
              </button>
              <button
                v-if="redisCursor !== '0'"
                type="button"
                class="load-more"
                :disabled="redisKeysLoading"
                @click="loadRedisKeys(false)"
              >
                {{ redisKeysLoading ? "读取中…" : "加载更多" }}
              </button>
            </aside>

            <article class="key-detail">
              <div v-if="redisKeyLoading" class="workbench-empty">
                正在读取 Key…
              </div>
              <template v-else-if="redisKeyDetail">
                <div class="key-detail-head">
                  <div>
                    <span>{{ redisKeyDetail.keyType }}</span>
                    <h2>{{ redisKeyDetail.key }}</h2>
                  </div>
                  <dl>
                    <div>
                      <dt>TTL</dt>
                      <dd>
                        {{
                          redisKeyDetail.ttlSeconds === -1
                            ? "永久"
                            : `${redisKeyDetail.ttlSeconds}s`
                        }}
                      </dd>
                    </div>
                    <div>
                      <dt>大小</dt>
                      <dd>{{ formatBytes(redisKeyDetail.memoryBytes) }}</dd>
                    </div>
                  </dl>
                </div>
                <pre>{{ JSON.stringify(redisKeyDetail.value, null, 2) }}</pre>
                <p v-if="redisKeyDetail.truncated" class="detail-note">
                  为保持轻量，仅显示前 100 条数据。
                </p>
              </template>
              <div v-else class="workbench-empty">
                从左侧选择一个 Key 查看内容
              </div>
            </article>
          </div>
        </section>

        <section v-else-if="activeTab === 'console'" class="console-panel">
          <div class="console-head">
            <div>
              <p>REDIS CLI</p>
              <h2>{{ t("console.title") }}</h2>
            </div>
            <label>
              DB
              <select v-model.number="redisDatabase">
                <option v-for="database in 16" :key="database - 1" :value="database - 1">
                  {{ database - 1 }}
                </option>
              </select>
            </label>
            <button type="button" @click="openRedisAiAssistant">
              ✦ AI 助手
            </button>
            <button
              type="button"
              :disabled="consoleHistory.length === 0"
              @click="consoleHistory = []"
            >
              {{ t("console.clearOutput") }}
            </button>
          </div>
          <div class="console-output">
            <div v-if="consoleHistory.length === 0" class="console-placeholder">
              {{ t("console.redisEmpty") }}
            </div>
            <article
              v-for="(entry, index) in consoleHistory"
              :key="index"
              :class="{ failed: entry.error }"
            >
              <header>
                <strong>DB{{ entry.database }} &gt; {{ entry.command }}</strong>
                <span>{{ entry.elapsedMs }} ms</span>
              </header>
              <pre>{{ entry.output }}</pre>
            </article>
          </div>
          <form class="console-input" @submit.prevent="runConsoleCommand()">
            <span>DB{{ redisDatabase }} &gt;</span>
            <input
              v-model="consoleInput"
              type="text"
              autocomplete="off"
              spellcheck="false"
              :disabled="
                consoleRunning || selectedService.status !== 'running'
              "
              :placeholder="
                selectedService.status === 'running'
                  ? t('console.redisInput')
                  : t('console.redisStartFirst')
              "
            />
            <button
              type="submit"
              :disabled="
                !consoleInput.trim() ||
                consoleRunning ||
                selectedService.status !== 'running'
              "
            >
              {{ consoleRunning ? t("console.executing") : t("console.execute") }}
            </button>
          </form>
          <p class="console-note">
            {{ t("console.redisNote") }}
          </p>
        </section>

        <section
          v-else-if="activeTab === 'data' && selectedKind === 'mongodb'"
          class="redis-workbench"
        >
          <div class="redis-toolbar">
            <label class="database-select">
              数据库
              <select
                v-model="selectedMongoDatabase"
                :disabled="mongoLoading"
                @change="changeMongoDatabase"
              >
                <option
                  v-for="database in mongoDatabases"
                  :key="database.name"
                  :value="database.name"
                >
                  {{ database.name }}{{ database.system ? " · 系统" : "" }}
                </option>
              </select>
            </label>
            <span class="toolbar-summary">
              {{ mongoCollections.length }} 个集合
            </span>
            <button
              type="button"
              :disabled="
                mongoLoading || selectedService.status !== 'running'
              "
              @click="loadMongoCatalog"
            >
              {{ mongoLoading ? "读取中" : "刷新" }}
            </button>
          </div>

          <div
            v-if="selectedService.status !== 'running'"
            class="workbench-empty"
          >
            启动 MongoDB 后即可浏览本地文档
          </div>
          <div v-else class="key-browser database-browser">
            <aside class="key-list table-list">
              <div class="key-list-head">
                <strong>COLLECTIONS</strong>
                <span>{{ mongoCollections.length }}</span>
              </div>
              <div v-if="mongoCollections.length === 0" class="key-list-empty">
                {{
                  mongoDetailLoading
                    ? "正在读取…"
                    : "当前数据库没有集合"
                }}
              </div>
              <button
                v-for="collection in mongoCollections"
                :key="collection.name"
                type="button"
                :class="{
                  active:
                    selectedMongoCollection?.name === collection.name,
                }"
                :title="collection.name"
                @click="selectMongoCollection(collection)"
              >
                <strong>{{ collection.name }}</strong>
                <small>{{ collection.collectionType || "collection" }}</small>
              </button>
            </aside>

            <article class="key-detail table-detail">
              <div v-if="mongoDetailLoading" class="workbench-empty">
                正在读取集合…
              </div>
              <template
                v-else-if="
                  selectedMongoCollection && mongoCollectionDetail
                "
              >
                <div class="key-detail-head">
                  <div>
                    <span>COLLECTION</span>
                    <h2>
                      {{ selectedMongoDatabase }}.{{
                        selectedMongoCollection.name
                      }}
                    </h2>
                  </div>
                  <dl>
                    <div>
                      <dt>文档数</dt>
                      <dd>{{ mongoCollectionDetail.documentCount }}</dd>
                    </div>
                    <div>
                      <dt>数据大小</dt>
                      <dd>
                        {{ formatBytes(mongoCollectionDetail.sizeBytes) }}
                      </dd>
                    </div>
                  </dl>
                </div>

                <div class="schema-section">
                  <h3>字段结构 · 从预览文档自动识别</h3>
                  <div class="result-table-wrap">
                    <table>
                      <thead>
                        <tr>
                          <th>字段</th>
                          <th>BSON 类型</th>
                          <th>出现次数</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr
                          v-for="field in mongoCollectionDetail.fields"
                          :key="field.name"
                        >
                          <td>{{ field.name }}</td>
                          <td class="type-description-cell">
                            <code class="raw-data-type">{{
                              field.bsonType
                            }}</code>
                            <span
                              class="type-help type-badge"
                              tabindex="0"
                              :title="
                                databaseTypeInfo(field.bsonType).description
                              "
                              :aria-label="`${databaseTypeInfo(field.bsonType).label}：${databaseTypeInfo(field.bsonType).description}`"
                            >
                              i
                              <span class="type-tooltip" role="tooltip">
                                <strong>{{
                                  databaseTypeInfo(field.bsonType).label
                                }}</strong>
                                {{
                                  databaseTypeInfo(field.bsonType).description
                                }}
                              </span>
                            </span>
                          </td>
                          <td>{{ field.occurrences }}</td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </div>

                <div class="schema-section preview-section">
                  <h3>
                    文档预览 · {{ mongoCollectionDetail.documents.length }} 条
                  </h3>
                  <pre class="mongo-documents">{{
                    JSON.stringify(mongoCollectionDetail.documents, null, 2)
                  }}</pre>
                </div>
                <p
                  v-if="mongoCollectionDetail.truncated"
                  class="detail-note"
                >
                  为保持轻量，仅显示前 100 条文档。
                </p>
              </template>
              <div v-else class="workbench-empty">
                从左侧选择一个集合查看字段和文档
              </div>
            </article>
          </div>
        </section>

        <section v-else-if="activeTab === 'data'" class="redis-workbench">
          <div class="redis-toolbar">
            <label class="database-select">
              数据库
              <select
                v-model="selectedDatabase"
                :disabled="databaseLoading"
                @change="changeSqlDatabase"
              >
                <option
                  v-for="database in databases"
                  :key="database.name"
                  :value="database.name"
                >
                  {{ database.name }}{{ database.system ? " · 系统" : "" }}
                </option>
              </select>
            </label>
            <span class="toolbar-summary">
              {{ tables.length }} 张表
            </span>
            <button
              type="button"
              :disabled="
                databaseLoading || selectedService.status !== 'running'
              "
              @click="loadDatabaseCatalog"
            >
              {{ databaseLoading ? "读取中" : "刷新" }}
            </button>
          </div>

          <div
            v-if="selectedService.status !== 'running'"
            class="workbench-empty"
          >
            启动 {{ selectedService.name }} 后即可浏览本地数据
          </div>
          <div v-else class="key-browser database-browser">
            <aside class="key-list table-list">
              <div class="key-list-head">
                <strong>TABLES</strong>
                <span>{{ tables.length }}</span>
              </div>
              <div v-if="tables.length === 0" class="key-list-empty">
                {{ tableLoading ? "正在读取…" : "当前数据库没有数据表" }}
              </div>
              <button
                v-for="table in tables"
                :key="tableIdentity(table)"
                type="button"
                :class="{
                  active:
                    selectedTable &&
                    tableIdentity(selectedTable) === tableIdentity(table),
                }"
                :title="tableIdentity(table)"
                @click="selectTable(table)"
              >
                <strong>{{ table.name }}</strong>
                <small>
                  {{ table.schema }} · {{ table.rowCount }} 行 ·
                  {{ formatBytes(table.sizeBytes) }}
                </small>
              </button>
            </aside>

            <article class="key-detail table-detail">
              <div v-if="tableLoading" class="workbench-empty">
                正在读取数据表…
              </div>
              <template v-else-if="selectedTable && tableDetail">
                <div class="key-detail-head">
                  <div>
                    <span>TABLE</span>
                    <h2>{{ tableIdentity(selectedTable) }}</h2>
                  </div>
                  <dl>
                    <div>
                      <dt>预估行数</dt>
                      <dd>{{ selectedTable.rowCount }}</dd>
                    </div>
                    <div>
                      <dt>大小</dt>
                      <dd>{{ formatBytes(selectedTable.sizeBytes) }}</dd>
                    </div>
                  </dl>
                </div>

                <div class="schema-section">
                  <h3>字段结构</h3>
                  <div class="result-table-wrap">
                    <table>
                      <thead>
                        <tr>
                          <th>字段</th>
                          <th>数据库类型</th>
                          <th>可空</th>
                          <th>键</th>
                          <th>默认值</th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr
                          v-for="column in tableDetail.columns"
                          :key="column.name"
                        >
                          <td>{{ column.name }}</td>
                          <td class="type-description-cell">
                            <code class="raw-data-type">{{
                              column.dataType
                            }}</code>
                            <span
                              class="type-help type-badge"
                              tabindex="0"
                              :title="
                                databaseTypeInfo(column.dataType).description
                              "
                              :aria-label="`${databaseTypeInfo(column.dataType).label}：${databaseTypeInfo(column.dataType).description}`"
                            >
                              i
                              <span class="type-tooltip" role="tooltip">
                                <strong>{{
                                  databaseTypeInfo(column.dataType).label
                                }}</strong>
                                {{
                                  databaseTypeInfo(column.dataType).description
                                }}
                              </span>
                            </span>
                          </td>
                          <td>{{ column.nullable ? "YES" : "NO" }}</td>
                          <td>{{ column.key || "—" }}</td>
                          <td>{{ column.defaultValue ?? "NULL" }}</td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </div>

                <div class="schema-section preview-section">
                  <h3>数据预览 · 前 100 行</h3>
                  <div
                    v-if="tableDetail.preview.columns.length === 0"
                    class="table-empty"
                  >
                    暂无数据
                  </div>
                  <div v-else class="result-table-wrap">
                    <table>
                      <thead>
                        <tr>
                          <th
                            v-for="column in tableDetail.preview.columns"
                            :key="column"
                          >
                            <span class="preview-column-name">
                              {{ column }}
                              <span
                                v-if="previewColumnType(column)"
                                class="type-help compact type-badge"
                                tabindex="0"
                                :title="
                                  previewColumnType(column)?.description
                                "
                                :aria-label="`${previewColumnType(column)?.label}：${previewColumnType(column)?.description}`"
                              >
                                i
                                <span class="type-tooltip" role="tooltip">
                                  <strong>{{
                                    previewColumnType(column)?.label
                                  }}</strong>
                                  {{
                                    previewColumnType(column)?.description
                                  }}
                                </span>
                              </span>
                            </span>
                          </th>
                        </tr>
                      </thead>
                      <tbody>
                        <tr
                          v-for="(row, rowIndex) in tableDetail.preview.rows"
                          :key="rowIndex"
                        >
                          <td
                            v-for="(value, columnIndex) in row"
                            :key="columnIndex"
                            :class="{ null: value === null }"
                          >
                            {{ displayCell(value) }}
                          </td>
                        </tr>
                      </tbody>
                    </table>
                  </div>
                </div>
              </template>
              <div v-else class="workbench-empty">
                从左侧选择一张表查看结构和数据
              </div>
            </article>
          </div>
        </section>

        <section v-else-if="activeTab === 'sql'" class="sql-console-panel">
          <div class="console-head">
            <div>
              <p>SQL CONSOLE</p>
              <h2>{{ t("console.sqlTitle", { service: selectedService.name }) }}</h2>
            </div>
            <label>
              {{ t("console.database") }}
              <select
                v-model="selectedDatabase"
                :disabled="databaseLoading || sqlRunning"
              >
                <option
                  v-for="database in databases"
                  :key="database.name"
                  :value="database.name"
                >
                  {{ database.name }}
                </option>
              </select>
            </label>
            <button type="button" @click="openSqlAiAssistant">
              ✦ AI 助手
            </button>
            <button
              type="button"
              :disabled="sqlHistory.length === 0"
              @click="sqlHistory = []"
            >
              {{ t("console.clearResults") }}
            </button>
          </div>

          <textarea
            v-model="sqlInput"
            class="sql-editor"
            spellcheck="false"
            :disabled="sqlRunning || selectedService.status !== 'running'"
            :placeholder="t('console.sqlPlaceholder')"
            @keydown.meta.enter.prevent="runSqlCommand()"
            @keydown.ctrl.enter.prevent="runSqlCommand()"
          ></textarea>
          <div class="sql-runbar">
            <span>{{ t("console.sqlShortcut") }}</span>
            <button
              type="button"
              :disabled="
                !sqlInput.trim() ||
                !selectedDatabase ||
                sqlRunning ||
                selectedService.status !== 'running'
              "
              @click="runSqlCommand()"
            >
              {{ sqlRunning ? t("console.executing") : t("console.executeSql") }}
            </button>
          </div>

          <div class="sql-results">
            <div v-if="sqlHistory.length === 0" class="console-placeholder">
              {{ t("console.sqlEmpty") }}
            </div>
            <article
              v-for="(entry, entryIndex) in sqlHistory"
              :key="entryIndex"
              :class="{ failed: entry.error }"
            >
              <header>
                <strong>{{ entry.database }} &gt; {{ entry.sql }}</strong>
                <span v-if="entry.result">
                  {{ localizedSqlSummary(entry.result.summary) }} · {{ entry.result.elapsedMs }} ms
                </span>
              </header>
              <pre v-if="entry.error">{{ entry.error }}</pre>
              <template v-else-if="entry.result">
                <div
                  v-if="entry.result.columns.length > 0"
                  class="result-table-wrap"
                >
                  <table>
                    <thead>
                      <tr>
                        <th
                          v-for="column in entry.result.columns"
                          :key="column"
                        >
                          {{ column }}
                        </th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr
                        v-for="(row, rowIndex) in entry.result.rows"
                        :key="rowIndex"
                      >
                        <td
                          v-for="(value, columnIndex) in row"
                          :key="columnIndex"
                          :class="{ null: value === null }"
                        >
                          {{ displayCell(value) }}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
                <p v-else class="sql-summary">
                  {{ localizedSqlSummary(entry.result.summary) }}
                </p>
                <p v-if="entry.result.truncated" class="detail-note">
                  {{ t("console.truncated") }}
                </p>
              </template>
            </article>
          </div>
          <p class="console-note">
            {{ t("console.sqlNote") }}
          </p>
        </section>

        <section
          v-else-if="activeTab === 'mongoConsole'"
          class="sql-console-panel"
        >
          <div class="console-head">
            <div>
              <p>MONGODB JSON COMMAND</p>
              <h2>{{ t("console.mongoTitle") }}</h2>
            </div>
            <label>
              {{ t("console.database") }}
              <select
                v-model="selectedMongoDatabase"
                :disabled="mongoLoading || mongoCommandRunning"
              >
                <option
                  v-for="database in mongoDatabases"
                  :key="database.name"
                  :value="database.name"
                >
                  {{ database.name }}
                </option>
              </select>
            </label>
            <button type="button" @click="openMongoAiAssistant">
              ✦ AI 助手
            </button>
            <button
              type="button"
              :disabled="mongoCommandHistory.length === 0"
              @click="mongoCommandHistory = []"
            >
              {{ t("console.clearResults") }}
            </button>
          </div>

          <textarea
            v-model="mongoCommandInput"
            class="sql-editor"
            spellcheck="false"
            :disabled="
              mongoCommandRunning || selectedService.status !== 'running'
            "
            :placeholder="t('console.mongoPlaceholder')"
            @keydown.meta.enter.prevent="runMongoCommand()"
            @keydown.ctrl.enter.prevent="runMongoCommand()"
          ></textarea>
          <div class="sql-runbar">
            <span>{{ t("console.mongoShortcut") }}</span>
            <button
              type="button"
              :disabled="
                !mongoCommandInput.trim() ||
                !selectedMongoDatabase ||
                mongoCommandRunning ||
                selectedService.status !== 'running'
              "
              @click="runMongoCommand()"
            >
              {{ mongoCommandRunning ? t("console.executing") : t("console.executeCommand") }}
            </button>
          </div>

          <div class="sql-results">
            <div
              v-if="mongoCommandHistory.length === 0"
              class="console-placeholder"
            >
              {{ t("console.mongoExample") }}：<code>{"ping": 1}</code>、
              <code>{"find": "users", "limit": 20}</code>
            </div>
            <article
              v-for="(entry, entryIndex) in mongoCommandHistory"
              :key="entryIndex"
              :class="{ failed: entry.error }"
            >
              <header>
                <strong>{{ entry.database }} &gt; {{ entry.command }}</strong>
                <span v-if="!entry.error">{{ entry.elapsedMs }} ms</span>
              </header>
              <pre>{{
                entry.error || JSON.stringify(entry.output, null, 2)
              }}</pre>
            </article>
          </div>
          <p class="console-note">
            {{ t("console.mongoNote") }}
          </p>
        </section>

        <section v-else-if="activeTab === 'mail'" class="mail-workbench">
          <div class="redis-toolbar mail-toolbar">
            <div>
              <strong>本地邮件收件箱</strong>
              <span>
                应用 SMTP 配置：
                <code>127.0.0.1:1025</code>
              </span>
            </div>
            <span class="toolbar-summary">
              最多保留 500 封 · 不向外部投递
            </span>
            <button
              type="button"
              :disabled="
                mailLoading || selectedService.status !== 'running'
              "
              @click="loadMailMessages"
            >
              {{ mailLoading ? "读取中" : "刷新邮件" }}
            </button>
          </div>

          <div
            v-if="selectedService.status !== 'running'"
            class="workbench-empty"
          >
            启动 Mailpit 后，即可捕获本机应用发出的测试邮件
          </div>
          <div v-else class="key-browser mail-browser">
            <aside class="key-list mail-list">
              <div class="key-list-head">
                <strong>MESSAGES</strong>
                <span>{{ mailMessages.length }}</span>
              </div>
              <div v-if="mailMessages.length === 0" class="key-list-empty">
                {{ mailLoading ? "正在读取…" : "还没有捕获到邮件" }}
              </div>
              <button
                v-for="message in mailMessages"
                :key="message.id"
                type="button"
                :class="{
                  active: selectedMailId === message.id,
                  unread: !message.read,
                }"
                @click="selectMail(message)"
              >
                <strong>{{ message.subject || "（无主题）" }}</strong>
                <small>{{ message.from || "未知发件人" }}</small>
                <span>
                  {{ formatMailDate(message.created) }}
                  <i v-if="message.attachmentCount">
                    · {{ message.attachmentCount }} 个附件
                  </i>
                </span>
              </button>
            </aside>

            <article class="key-detail mail-detail">
              <div v-if="mailDetailLoading" class="workbench-empty">
                正在读取邮件…
              </div>
              <template v-else-if="mailDetail">
                <header class="mail-detail-head">
                  <p>MESSAGE</p>
                  <h2>{{ mailDetail.subject || "（无主题）" }}</h2>
                  <dl>
                    <div>
                      <dt>发件人</dt>
                      <dd>{{ mailDetail.from || "—" }}</dd>
                    </div>
                    <div>
                      <dt>收件人</dt>
                      <dd>{{ mailDetail.to.join(", ") || "—" }}</dd>
                    </div>
                    <div v-if="mailDetail.cc.length">
                      <dt>抄送</dt>
                      <dd>{{ mailDetail.cc.join(", ") }}</dd>
                    </div>
                    <div>
                      <dt>时间</dt>
                      <dd>{{ formatMailDate(mailDetail.created) }}</dd>
                    </div>
                  </dl>
                </header>

                <section v-if="mailDetail.text" class="mail-body">
                  <h3>纯文本内容</h3>
                  <pre>{{ mailDetail.text }}</pre>
                </section>
                <section v-if="mailDetail.html" class="mail-body">
                  <h3>HTML 源码（安全文本预览）</h3>
                  <pre>{{ mailDetail.html }}</pre>
                </section>
                <section
                  v-if="!mailDetail.text && !mailDetail.html"
                  class="mail-body-empty"
                >
                  这封邮件没有可显示的正文
                </section>
              </template>
              <div v-else class="workbench-empty">
                从左侧选择一封邮件查看内容
              </div>
            </article>
          </div>
          <p class="console-note">
            邮件仅保存在
            <code>{{ selectedService.dataPath }}/mailpit.db</code>；
            HTML 不会直接渲染，避免测试邮件中的脚本或远程资源影响桌面端。
          </p>
        </section>

        <section
          v-else-if="activeTab === 'broker'"
          class="object-store-panel"
        >
          <div class="object-store-hero">
            <div>
              <p>AMQP MESSAGE BROKER</p>
              <h2>RabbitMQ 本地消息代理</h2>
              <span>适合调试队列、交换机、路由键、确认和重试</span>
            </div>
            <span class="legacy-badge">内置 Erlang/OTP 27</span>
          </div>
          <div class="message-ai-toolbar">
            <div>
              <p>AI MESSAGE DESIGN</p>
              <span>生成 Exchange、Queue、Routing Key 与消息结构建议</span>
            </div>
            <button type="button" @click="openMessageAiAssistant">
              ✦ AI 设计消息
            </button>
          </div>
          <div
            v-if="selectedService.status !== 'running'"
            class="workbench-empty"
          >
            启动 RabbitMQ 后可连接 AMQP 并访问 Management UI
          </div>
          <template v-else>
            <div class="object-store-grid">
              <article class="panel object-store-card">
                <p>AMQP</p>
                <strong>127.0.0.1:5672</strong>
                <small>应用客户端连接</small>
              </article>
              <article class="panel object-store-card">
                <p>MANAGEMENT UI</p>
                <strong>http://127.0.0.1:15672</strong>
                <small>队列、交换机和连接管理</small>
              </article>
              <article class="panel object-store-card">
                <p>USERNAME</p>
                <strong>zhiyu</strong>
                <small>本地开发管理员</small>
              </article>
              <article class="panel object-store-card">
                <p>PASSWORD</p>
                <strong>zhiyu-local-rabbitmq-2026</strong>
                <small>仅用于本机开发</small>
              </article>
            </div>
            <article class="panel object-store-snippet">
              <div class="panel-title">
                <div>
                  <p>CONNECTION URL</p>
                  <h2>应用连接配置</h2>
                </div>
              </div>
              <pre>AMQP_URL=amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:5672/</pre>
            </article>
            <p class="console-note">
              Erlang/OTP 与 RabbitMQ 都安装在 ~/.devbox，不读取或修改系统全局运行时。
            </p>
          </template>
        </section>

        <section
          v-else-if="activeTab === 'governance'"
          class="object-store-panel"
        >
          <div class="object-store-hero">
            <div>
              <p>SERVICE COORDINATION</p>
              <h2>{{ governanceProfile.name }} 本地单节点</h2>
              <span>{{ governanceProfile.description }}</span>
            </div>
            <span class="legacy-badge">{{ governanceProfile.badge }}</span>
          </div>
          <div
            v-if="selectedService.status !== 'running'"
            class="workbench-empty"
          >
            启动 {{ governanceProfile.name }} 后可通过客户端或内置命令行连接
          </div>
          <template v-else>
            <div class="object-store-grid">
              <article class="panel object-store-card">
                <p>{{ governanceProfile.primaryLabel }}</p>
                <strong>{{ governanceProfile.primary }}</strong>
                <small>{{ governanceProfile.primaryHint }}</small>
              </article>
              <article class="panel object-store-card">
                <p>{{ governanceProfile.secondaryLabel }}</p>
                <strong>{{ governanceProfile.secondary }}</strong>
                <small>{{ governanceProfile.secondaryHint }}</small>
              </article>
            </div>
            <article class="panel object-store-snippet">
              <div class="panel-title">
                <div>
                  <p>CLI</p>
                  <h2>快速读写验证</h2>
                </div>
              </div>
              <pre>{{ governanceProfile.command }}</pre>
            </article>
            <p class="console-note">
              {{ governanceProfile.note }}
            </p>
          </template>
        </section>

        <section
          v-else-if="activeTab === 'objectStore'"
          class="object-store-panel"
        >
          <div class="object-store-hero">
            <div>
              <p>S3-COMPATIBLE OBJECT STORAGE</p>
              <h2>{{ objectStoreProfile.name }} 本地对象存储</h2>
              <span>适合验证 S3 SDK、文件上传、Bucket 和预签名 URL</span>
            </div>
            <span class="legacy-badge">{{ objectStoreProfile.badge }}</span>
          </div>
          <div
            v-if="selectedService.status !== 'running'"
            class="workbench-empty"
          >
            启动 {{ objectStoreProfile.name }} 后可访问 S3 API 与 Web Console
          </div>
          <template v-else>
            <div class="object-store-grid">
              <article class="panel object-store-card">
                <p>S3 API</p>
                <strong>{{ objectStoreProfile.apiEndpoint }}</strong>
                <small>应用和 AWS SDK 使用</small>
              </article>
              <article class="panel object-store-card">
                <p>WEB CONSOLE</p>
                <strong>{{ objectStoreProfile.consoleEndpoint }}</strong>
                <small>在浏览器打开管理 Bucket 和对象</small>
              </article>
              <article class="panel object-store-card">
                <p>ACCESS KEY</p>
                <strong>{{ objectStoreProfile.accessKey }}</strong>
                <small>仅用于本机开发</small>
              </article>
              <article class="panel object-store-card">
                <p>SECRET KEY</p>
                <strong>{{ objectStoreProfile.secretKey }}</strong>
                <small>可在配置文件中查看</small>
              </article>
            </div>
            <article class="panel object-store-snippet">
              <div class="panel-title">
                <div>
                  <p>ENVIRONMENT</p>
                  <h2>应用连接配置</h2>
                </div>
              </div>
              <pre>S3_ENDPOINT={{ objectStoreProfile.apiEndpoint }}
AWS_ACCESS_KEY_ID={{ objectStoreProfile.accessKey }}
AWS_SECRET_ACCESS_KEY={{ objectStoreProfile.secretKey }}
AWS_REGION=us-east-1
S3_FORCE_PATH_STYLE=true</pre>
            </article>
            <p class="console-note">
              {{ objectStoreProfile.note }}
            </p>
          </template>
        </section>

        <section
          v-else-if="activeTab === 'timeseries' && selectedKind === 'influxdb'"
        >
          <InfluxdbPanel :running="selectedService.status === 'running'" />
        </section>

        <section
          v-else-if="activeTab === 'search'"
          class="meilisearch-workbench"
        >
          <div
            v-if="selectedService.status !== 'running'"
            class="workbench-empty"
          >
            启动 Meilisearch 后即可创建索引、导入文档和测试搜索
          </div>
          <template v-else>
            <div class="meili-index-strip">
              <button
                v-for="index in meilisearchIndexes"
                :key="index.uid"
                type="button"
                :class="{ active: meilisearchIndex === index.uid }"
                @click="meilisearchIndex = index.uid"
              >
                <strong>{{ index.uid }}</strong>
                <small>
                  {{ index.documentCount }} 文档
                  <template v-if="index.indexing"> · 索引中</template>
                </small>
              </button>
              <button type="button" @click="loadMeilisearchIndexes">
                {{ meilisearchLoading ? "读取中…" : "刷新索引" }}
              </button>
            </div>

            <div class="nats-console-grid">
              <article class="panel nats-console-card meili-card">
                <div class="panel-title">
                  <div>
                    <p>DOCUMENTS</p>
                    <h2>导入 JSON 文档</h2>
                  </div>
                  <span>自动创建索引</span>
                </div>
                <div class="meili-fields">
                  <label>
                    <span>索引 UID</span>
                    <input v-model="meilisearchIndex" placeholder="movies" />
                  </label>
                  <label>
                    <span>主键</span>
                    <input v-model="meilisearchPrimaryKey" placeholder="id" />
                  </label>
                </div>
                <label>
                  <span>JSON 数组</span>
                  <textarea
                    v-model="meilisearchDocuments"
                    spellcheck="false"
                  ></textarea>
                </label>
                <button
                  class="primary"
                  type="button"
                  :disabled="
                    meilisearchImporting ||
                    !meilisearchIndex.trim() ||
                    !meilisearchPrimaryKey.trim()
                  "
                  @click="importMeilisearchDocuments"
                >
                  <span v-if="meilisearchImporting" class="spinner"></span>
                  {{ meilisearchImporting ? "提交中" : "导入文档" }}
                </button>
              </article>

              <article class="panel nats-console-card meili-card">
                <div class="panel-title">
                  <div>
                    <p>SEARCH</p>
                    <h2>搜索调试</h2>
                  </div>
                  <span v-if="meilisearchResult">
                    {{ meilisearchResult.processingTimeMs }} ms
                  </span>
                </div>
                <label>
                  <span>索引 UID</span>
                  <input v-model="meilisearchIndex" placeholder="movies" />
                </label>
                <label>
                  <span>搜索内容（留空可预览全部）</span>
                  <input
                    v-model="meilisearchQuery"
                    placeholder="输入关键词"
                    @keydown.enter="runMeilisearchSearch"
                  />
                </label>
                <button
                  class="primary"
                  type="button"
                  :disabled="meilisearchSearching || !meilisearchIndex.trim()"
                  @click="runMeilisearchSearch"
                >
                  <span v-if="meilisearchSearching" class="spinner"></span>
                  {{ meilisearchSearching ? "搜索中" : "执行搜索" }}
                </button>
                <div v-if="meilisearchResult" class="meili-result">
                  <header>
                    <strong>
                      约 {{ meilisearchResult.estimatedTotalHits }} 条结果
                    </strong>
                    <span>最多展示 50 条</span>
                  </header>
                  <pre>{{
                    JSON.stringify(meilisearchResult.hits, null, 2)
                  }}</pre>
                </div>
                <div v-else class="nats-message-empty">
                  选择索引并输入关键词后执行搜索
                </div>
              </article>
            </div>
          </template>
        </section>

        <section
          v-else-if="activeTab === 'messages' && selectedKind === 'kafka'"
          class="nats-workbench"
        >
          <div
            v-if="selectedService.status !== 'running'"
            class="workbench-empty"
          >
            启动 Kafka Sandbox 后即可创建主题和发送测试消息
          </div>
          <template v-else>
            <div class="message-ai-toolbar">
              <div>
                <p>AI MESSAGE DESIGN</p>
                <span>根据业务事件生成 Topic、Key、分区数与 Payload</span>
              </div>
              <button type="button" @click="openMessageAiAssistant">
                ✦ AI 设计消息
              </button>
            </div>
            <div class="nats-endpoint-strip">
              <div>
                <p>KAFKA BROKER</p>
                <strong>127.0.0.1:9092</strong>
              </div>
              <div>
                <p>RUNTIME</p>
                <strong>Tansu {{ selectedService.version }}</strong>
              </div>
              <div>
                <p>STORAGE</p>
                <strong>SQLite · 本地持久化</strong>
              </div>
            </div>

            <div class="kafka-console-grid">
              <article class="panel kafka-topic-card">
                <div class="panel-title">
                  <div>
                    <p>TOPICS</p>
                    <h2>主题管理</h2>
                  </div>
                  <button type="button" :disabled="kafkaLoading" @click="loadKafkaTopics">
                    {{ kafkaLoading ? "读取中" : "刷新" }}
                  </button>
                </div>
                <div class="kafka-topic-create">
                  <input v-model="kafkaTopicName" placeholder="orders.created" />
                  <input
                    v-model.number="kafkaPartitions"
                    type="number"
                    min="1"
                    max="32"
                    title="分区数"
                  />
                  <button
                    class="primary"
                    type="button"
                    :disabled="kafkaLoading || !kafkaTopicName.trim()"
                    @click="addKafkaTopic"
                  >
                    创建
                  </button>
                </div>
                <div v-if="kafkaTopics.length" class="kafka-topic-list">
                  <button
                    v-for="topic in kafkaTopics"
                    :key="topic.name"
                    type="button"
                    :class="{ active: kafkaSelectedTopic === topic.name }"
                    @click="kafkaSelectedTopic = topic.name"
                  >
                    <span>{{ topic.name }}</span>
                    <i
                      title="删除主题"
                      @click.stop="removeKafkaTopic(topic.name)"
                    >×</i>
                  </button>
                </div>
                <div v-else class="nats-message-empty">
                  暂无主题，创建一个主题即可开始调试。
                </div>
              </article>

              <article class="panel nats-console-card">
                <div class="panel-title">
                  <div>
                    <p>PRODUCE</p>
                    <h2>发送测试消息</h2>
                  </div>
                  <span v-if="kafkaPublishResult">
                    {{ kafkaPublishResult.elapsedMs }} ms
                  </span>
                </div>
                <label>
                  <span>Topic</span>
                  <select v-model="kafkaSelectedTopic">
                    <option value="">请选择主题</option>
                    <option
                      v-for="topic in kafkaTopics"
                      :key="topic.name"
                      :value="topic.name"
                    >
                      {{ topic.name }}
                    </option>
                  </select>
                </label>
                <label>
                  <span>Key（可选）</span>
                  <input v-model="kafkaMessageKey" placeholder="order-1001" />
                </label>
                <label>
                  <span>Payload</span>
                  <textarea
                    v-model="kafkaMessagePayload"
                    spellcheck="false"
                    placeholder='{"id": 1001}'
                  ></textarea>
                </label>
                <button
                  class="primary"
                  type="button"
                  :disabled="kafkaPublishing || !kafkaSelectedTopic"
                  @click="publishKafka"
                >
                  <span v-if="kafkaPublishing" class="spinner"></span>
                  {{ kafkaPublishing ? "发送中" : "发送消息" }}
                </button>
                <p v-if="kafkaPublishResult" class="nats-result-note">
                  已发送 {{ kafkaPublishResult.payloadBytes }} 字节到
                  <code>{{ kafkaPublishResult.topic }}</code>
                </p>
              </article>
            </div>
            <p class="console-note">
              面向本地开发调试：兼容常用 Kafka 客户端，不包含集群、副本和生产运维能力。
            </p>
          </template>
        </section>

        <section
          v-else-if="activeTab === 'messages' && selectedKind === 'nats'"
          class="nats-workbench"
        >
          <div
            v-if="selectedService.status !== 'running'"
            class="workbench-empty"
          >
            启动 NATS 后即可发布和订阅本地消息
          </div>
          <template v-else>
            <div class="message-ai-toolbar">
              <div>
                <p>AI MESSAGE DESIGN</p>
                <span>根据业务事件生成 Subject、订阅模式与 Payload</span>
              </div>
              <button type="button" @click="openMessageAiAssistant">
                ✦ AI 设计消息
              </button>
            </div>
            <div class="nats-endpoint-strip">
              <div>
                <p>CLIENT ENDPOINT</p>
                <strong>nats://127.0.0.1:4222</strong>
              </div>
              <div>
                <p>MONITORING</p>
                <strong>http://127.0.0.1:8222</strong>
              </div>
              <div>
                <p>JETSTREAM</p>
                <strong>已启用 · 本地文件存储</strong>
              </div>
            </div>

            <div class="nats-console-grid">
              <article class="panel nats-console-card">
                <div class="panel-title">
                  <div>
                    <p>PUBLISH</p>
                    <h2>发布消息</h2>
                  </div>
                  <span v-if="natsPublishResult">
                    {{ natsPublishResult.elapsedMs }} ms
                  </span>
                </div>
                <label>
                  <span>Subject</span>
                  <input
                    v-model="natsPublishSubject"
                    placeholder="orders.created"
                  />
                </label>
                <label>
                  <span>Payload</span>
                  <textarea
                    v-model="natsPublishPayload"
                    spellcheck="false"
                    placeholder='{"id": 42}'
                  ></textarea>
                </label>
                <button
                  class="primary"
                  type="button"
                  :disabled="natsPublishing || !natsPublishSubject.trim()"
                  @click="publishNats"
                >
                  <span v-if="natsPublishing" class="spinner"></span>
                  {{ natsPublishing ? "发布中" : "发布消息" }}
                </button>
                <p v-if="natsPublishResult" class="nats-result-note">
                  已发送 {{ natsPublishResult.payloadBytes }} 字节到
                  <code>{{ natsPublishResult.subject }}</code>
                </p>
              </article>

              <article class="panel nats-console-card">
                <div class="panel-title">
                  <div>
                    <p>SUBSCRIBE ONCE</p>
                    <h2>等待一条消息</h2>
                  </div>
                  <span>支持 * 和 &gt;</span>
                </div>
                <label>
                  <span>Subject</span>
                  <input
                    v-model="natsSubscribeSubject"
                    placeholder="orders.>"
                  />
                </label>
                <button
                  class="primary"
                  type="button"
                  :disabled="natsReceiving || !natsSubscribeSubject.trim()"
                  @click="receiveNats"
                >
                  <span v-if="natsReceiving" class="spinner"></span>
                  {{ natsReceiving ? "等待中…" : "开始等待" }}
                </button>
                <div v-if="natsMessage" class="nats-message-result">
                  <header>
                    <strong>{{ natsMessage.subject }}</strong>
                    <span>
                      {{ natsMessage.payloadBytes }} B ·
                      {{ natsMessage.elapsedMs }} ms
                    </span>
                  </header>
                  <pre>{{ natsMessage.payload }}</pre>
                </div>
                <div v-else class="nats-message-empty">
                  先点击“开始等待”，再从应用或左侧发布面板发送消息。
                </div>
              </article>
            </div>

            <p class="console-note">
              消息只发送到智屿管理的本机 NATS；单条 Payload 限制为 1 MiB，等待订阅最多持续 8 秒。
            </p>
          </template>
        </section>

        <section v-else-if="activeTab === 'connect'" class="connect-panel-section">
          <ServiceConnectPanel :kind="selectedKind" />
        </section>

        <section v-else-if="activeTab === 'backup'" class="backup-panel">
          <div class="backup-head">
            <div>
              <p>DATA SAFETY</p>
              <h2>{{ t("backup.title") }}</h2>
              <span>
                {{ t("backup.location") }}
                <code>~/.devbox/backups/{{ selectedKind }}/</code>
              </span>
            </div>
            <div class="backup-actions">
              <button
                type="button"
                :disabled="
                  backupLoading ||
                  backupCreating ||
                  restoringBackupId !== null
                "
                @click="loadBackups"
              >
                {{ backupLoading ? t("backup.reading") : t("backup.refresh") }}
              </button>
              <button
                class="primary"
                type="button"
                :disabled="
                  selectedService.status === 'running' ||
                  selectedService.status === 'not_installed' ||
                  backupCreating ||
                  restoringBackupId !== null
                "
                @click="createBackup"
              >
                <span v-if="backupCreating" class="spinner"></span>
                {{ backupCreating ? t("backup.creating") : t("backup.create") }}
              </button>
            </div>
          </div>

          <div
            v-if="selectedService.status === 'running'"
            class="backup-warning"
          >
            {{ t("backup.stopWarning", { service: selectedService.name }) }}
          </div>
          <div
            v-else-if="selectedService.status === 'not_installed'"
            class="backup-warning"
          >
            {{ t("backup.installWarning") }}
          </div>

          <div class="backup-list">
            <div class="backup-list-head">
              <span>{{ t("backup.time") }}</span>
              <span>{{ t("backup.type") }}</span>
              <span>{{ t("backup.size") }}</span>
              <span>{{ t("backup.action") }}</span>
            </div>
            <div v-if="backupLoading && backups.length === 0" class="backup-empty">
              {{ t("backup.loading") }}
            </div>
            <div v-else-if="backups.length === 0" class="backup-empty">
              {{ t("backup.empty") }}
            </div>
            <article v-for="backup in backups" :key="backup.id">
              <div>
                <strong>{{ formatBackupDate(backup.createdAtMillis) }}</strong>
                <small>{{ backup.id }}</small>
              </div>
              <div>
                <span
                  class="backup-type"
                  :class="{ automatic: backup.automatic }"
                >
                  {{ backup.automatic ? t("backup.safety") : t("backup.manual") }}
                </span>
              </div>
              <div>
                <strong>{{ formatBytes(backup.sizeBytes) }}</strong>
              </div>
              <div>
                <button
                  type="button"
                  :disabled="
                    selectedService.status === 'running' ||
                    selectedService.status === 'not_installed' ||
                    restoringBackupId !== null ||
                    backupCreating
                  "
                  @click="restoreBackup(backup)"
                >
                  <span
                    v-if="restoringBackupId === backup.id"
                    class="spinner"
                  ></span>
                  {{
                    restoringBackupId === backup.id ? t("backup.restoring") : t("backup.restore")
                  }}
                </button>
              </div>
            </article>
          </div>

          <div class="backup-notes">
            <p>
              <strong>{{ t("backup.scopeTitle") }}</strong>
              {{ t("backup.scope") }}
            </p>
            <p>
              <strong>{{ t("backup.protectionTitle") }}</strong>
              {{ t("backup.protection") }}
            </p>
          </div>
        </section>

        <ServiceDocs
          v-else-if="activeTab === 'docs'"
          :kind="selectedService.kind"
          :port="selectedService.port"
          :service-name="selectedService.name"
        />

        <section v-else-if="activeTab === 'config'" class="editor-panel">
          <div class="editor-head">
            <div>
              <p>CONFIGURATION</p>
              <h2>{{ selectedService.configPath }}</h2>
            </div>
            <div class="editor-actions">
              <span v-if="configChanged">有未保存修改</span>
              <button
                v-if="selectedKind === 'nginx' || selectedKind === 'caddy'"
                type="button"
                @click="openConfigAiAssistant"
              >
                ✦ AI 配置
              </button>
              <button
                type="button"
                :disabled="!configChanged || configSaving"
                @click="saveConfig"
              >
                <span v-if="configSaving" class="spinner"></span>
                {{ configSaving ? "保存中" : "保存配置" }}
              </button>
            </div>
          </div>
          <textarea
            v-model="configContent"
            spellcheck="false"
            :disabled="configLoading"
            aria-label="服务配置文件"
          ></textarea>
          <p class="editor-note">
            保存前会自动生成 <code>.bak</code> 备份；部分配置需要重启服务后生效。
          </p>
        </section>

        <section v-else class="logs-panel">
          <div class="editor-head">
            <div>
              <p>RUNTIME LOGS</p>
              <h2>{{ selectedService.logPath }}</h2>
            </div>
            <button type="button" :disabled="logsLoading" @click="loadLogs">
              {{ logsLoading ? "读取中" : "刷新日志" }}
            </button>
            <button type="button" @click="openLogAiAssistant">
              ✦ AI 诊断
            </button>
          </div>
          <pre>{{ logs }}</pre>
        </section>
      </template>

      <div
        v-if="versionUninstallTarget"
        class="version-uninstall-backdrop"
      >
        <section
          class="version-uninstall-modal"
          role="dialog"
          aria-modal="true"
          aria-labelledby="version-uninstall-title"
        >
          <header>
            <span>{{ t("versions.safeUninstall") }}</span>
            <h2 id="version-uninstall-title">
              {{ t("versions.uninstallTitle", {
                service: versionUninstallTarget.serviceName,
                version: versionUninstallTarget.release.version,
              }) }}
            </h2>
            <p>{{ t("versions.uninstallSubtitle") }}</p>
          </header>

          <div class="version-uninstall-size">
            <span>{{ t("versions.space") }}</span>
            <strong>{{
              formatBytes(
                versionUninstallTarget.release.installationBytes,
              )
            }}</strong>
          </div>

          <div class="version-uninstall-preserved">
            <strong>{{ t("versions.preserved") }}</strong>
            <div>
              <span>{{ t("versions.dataDirectory") }}</span>
              <span>{{ t("versions.configFiles") }}</span>
              <span>{{ t("versions.runtimeLogs") }}</span>
              <span>{{ t("versions.localBackups") }}</span>
            </div>
          </div>

          <p
            v-if="versionUninstallTarget.fallbackVersion"
            class="version-uninstall-fallback"
          >
            {{ t("versions.fallback", { version: versionUninstallTarget.fallbackVersion }) }}
          </p>
          <p v-else class="version-uninstall-note">
            {{
              versionUninstallTarget.release.selected
                ? t("versions.currentNote")
                : t("versions.otherNote")
            }}
          </p>

          <footer>
            <button
              type="button"
              :disabled="versionUninstalling"
              @click="versionUninstallTarget = null"
            >
              {{ t("common.cancel") }}
            </button>
            <button
              type="button"
              class="confirm"
              :disabled="versionUninstalling"
              @click="confirmVersionUninstall"
            >
              <span v-if="versionUninstalling" class="spinner"></span>
              {{ versionUninstalling ? t("versions.uninstalling") : t("versions.confirmUninstall") }}
            </button>
          </footer>
        </section>
      </div>

      <div v-if="onboardingOpen" class="onboarding-backdrop">
        <section
          class="onboarding-modal"
          role="dialog"
          aria-modal="true"
          aria-labelledby="onboarding-title"
        >
          <div class="onboarding-progress">
            <i
              v-for="index in 3"
              :key="index"
              :class="{ active: index - 1 <= onboardingStep }"
            ></i>
          </div>

          <template v-if="onboardingStep === 0">
            <div class="onboarding-content onboarding-welcome">
              <span class="onboarding-mark">Z</span>
              <p>WELCOME TO ZHIYU ENVIRONMENT</p>
              <h2 id="onboarding-title">轻量管理你的本地开发环境</h2>
              <p>
                不使用 Docker，不启动虚拟机。Redis、MySQL、PostgreSQL
                等服务直接运行在用户目录中。
              </p>
              <div class="onboarding-features">
                <span><i>01</i>独立安装，不污染系统环境</span>
                <span><i>02</i>支持多个服务版本共存</span>
                <span><i>03</i>随用随开，保持低资源占用</span>
              </div>
            </div>
          </template>

          <template v-else-if="onboardingStep === 1">
            <div class="onboarding-content">
              <p>STEP 02 · LOCAL WORKSPACE</p>
              <h2 id="onboarding-title">所有文件都由你掌控</h2>
              <p>
                程序、配置、数据、日志和备份统一保存在下面的目录。卸载智屿不会自动删除你的服务数据。
              </p>
              <div class="onboarding-path">
                <span>环境目录</span>
                <code>{{ settingsDraft.installRoot }}</code>
                <button type="button" @click="chooseOnboardingRoot">
                  更换目录
                </button>
              </div>
              <div class="onboarding-directory-tree">
                <span>installations/ <em>官方程序</em></span>
                <span>instances/ <em>配置、数据与日志</em></span>
                <span>backups/ <em>本地备份</em></span>
                <span>downloads/ <em>可清理的安装缓存</em></span>
              </div>
            </div>
          </template>

          <template v-else>
            <div class="onboarding-content">
              <p>STEP 03 · FIRST SERVICE</p>
              <h2 id="onboarding-title">从一个常用服务开始</h2>
              <p>
                选择后会进入服务详情页，你可以查看版本并点击“下载并安装”。智屿不会自动下载安装任何内容。
              </p>
              <div class="onboarding-services">
                <button type="button" @click="finishOnboarding('redis')">
                  <span class="nav-icon redis">R</span>
                  <strong>Redis</strong>
                  <small>缓存与 Key-Value</small>
                  <em>{{
                    services.find((item) => item.kind === "redis")?.status !==
                    "not_installed"
                      ? "已安装"
                      : "推荐入门"
                  }}</em>
                </button>
                <button type="button" @click="finishOnboarding('mysql')">
                  <span class="nav-icon mysql">M</span>
                  <strong>MySQL</strong>
                  <small>关系型数据库</small>
                  <em>{{
                    services.find((item) => item.kind === "mysql")?.status !==
                    "not_installed"
                      ? "已安装"
                      : "按需安装"
                  }}</em>
                </button>
                <button type="button" @click="finishOnboarding('postgres')">
                  <span class="nav-icon postgres">P</span>
                  <strong>PostgreSQL</strong>
                  <small>关系型数据库</small>
                  <em>{{
                    services.find((item) => item.kind === "postgres")?.status !==
                    "not_installed"
                      ? "已安装"
                      : "按需安装"
                  }}</em>
                </button>
              </div>
            </div>
          </template>

          <footer class="onboarding-footer">
            <button
              v-if="onboardingStep === 0"
              type="button"
              class="onboarding-skip"
              @click="finishOnboarding()"
            >
              暂时跳过
            </button>
            <button
              v-else
              type="button"
              class="onboarding-skip"
              @click="onboardingStep--"
            >
              上一步
            </button>
            <span>{{ onboardingStep + 1 }} / 3</span>
            <button
              v-if="onboardingStep < 2"
              type="button"
              class="onboarding-next"
              @click="onboardingStep++"
            >
              下一步
            </button>
            <button
              v-else
              type="button"
              class="onboarding-next"
              @click="finishOnboarding()"
            >
              稍后再安装
            </button>
          </footer>
        </section>
      </div>

      <Teleport to="body">
        <div
          v-if="diagnosticsOpen"
          v-tool-i18n
          class="diagnostics-backdrop"
          role="presentation"
          @click.self="diagnosticsOpen = false"
        >
          <section
            class="diagnostics-modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="diagnostics-title"
          >
            <header class="diagnostics-header">
              <div>
                <span>SYSTEM HEALTH</span>
                <h2 id="diagnostics-title">一键诊断与修复</h2>
                <p>检查安装目录、服务文件、PID、端口和安装残留</p>
              </div>
              <button
                type="button"
                class="diagnostics-close"
                aria-label="关闭诊断"
                @click="diagnosticsOpen = false"
              >
                ×
              </button>
            </header>

            <div
              v-if="diagnosticsRunning && !diagnosticReport"
              class="diagnostics-loading"
            >
              <span class="spinner"></span>
              <strong>正在检查本地环境</strong>
              <small>通常只需要几秒钟</small>
            </div>

            <template v-else-if="diagnosticReport">
              <div class="diagnostics-summary">
                <article class="passed">
                  <span>通过</span>
                  <strong>{{ diagnosticReport.summary.passed }}</strong>
                </article>
                <article class="warning">
                  <span>警告</span>
                  <strong>{{ diagnosticReport.summary.warnings }}</strong>
                </article>
                <article class="error">
                  <span>错误</span>
                  <strong>{{ diagnosticReport.summary.errors }}</strong>
                </article>
                <article class="repairable">
                  <span>可自动修复</span>
                  <strong>{{ diagnosticReport.summary.repairable }}</strong>
                </article>
              </div>

              <div class="diagnostics-results">
                <article
                  v-for="item in diagnosticReport.items"
                  :key="item.id"
                  class="diagnostics-item"
                  :class="item.status"
                >
                  <i></i>
                  <div>
                    <div class="diagnostics-item-title">
                      <span>{{ item.scope }}</span>
                      <strong>{{ item.title }}</strong>
                      <em v-if="item.repairable">可修复</em>
                    </div>
                    <p>{{ item.message }}</p>
                    <details v-if="item.detail">
                      <summary>查看详细信息</summary>
                      <pre>{{ item.detail }}</pre>
                    </details>
                  </div>
                </article>
              </div>

              <footer class="diagnostics-footer">
                <span>
                  {{
                    new Date(
                      diagnosticReport.generatedAtMillis,
                    ).toLocaleTimeString()
                  }}
                  完成
                </span>
                <div>
                  <button type="button" @click="copyDiagnosticReport">
                    复制报告
                  </button>
                  <button
                    type="button"
                    :disabled="diagnosticsRunning || diagnosticsRepairing"
                    @click="runDiagnostics"
                  >
                    <span v-if="diagnosticsRunning" class="spinner"></span>
                    重新诊断
                  </button>
                  <button
                    type="button"
                    class="diagnostics-repair"
                    :disabled="
                      diagnosticsRepairing ||
                      diagnosticsRunning ||
                      diagnosticReport.summary.repairable === 0
                    "
                    @click="repairDiagnostics"
                  >
                    <span v-if="diagnosticsRepairing" class="spinner"></span>
                    {{
                      diagnosticsRepairing
                        ? "正在修复"
                        : `一键修复 (${diagnosticReport.summary.repairable})`
                    }}
                  </button>
                </div>
              </footer>
            </template>
          </section>
        </div>
      </Teleport>
    </main>
  </div>
</template>
