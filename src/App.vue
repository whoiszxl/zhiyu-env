<script setup lang="ts">
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { computed, onMounted, onUnmounted, provide, ref } from "vue";
import ServiceDocs from "./components/ServiceDocs.vue";
import { findTool, TOOLS } from "./tools/registry";
import { INSTALL_TASK_KEY, type ToolId } from "./tools/types";
import { formatBytes } from "./utils/format";
import {
  cleanServiceCache,
  addMeilisearchDocuments,
  createServiceBackup,
  executeSql,
  executeRedisCommand,
  executeMongoCommand,
  getMailpitMessageDetail,
  getMailpitOverview,
  getMeilisearchOverview,
  getDatabaseOverview,
  getMongoCollectionDetail,
  getMongoOverview,
  getNatsOverview,
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
  listRedisVersions,
  listServiceBackups,
  listServices,
  publishNatsMessage,
  readServiceConfig,
  receiveNatsMessage,
  runServiceAction,
  restoreServiceBackup,
  saveServiceConfig,
  scanRedisKeys,
  searchMeilisearch,
  selectRedisVersion,
  selectMysqlVersion,
  selectPostgresVersion,
} from "./api/services";
import { databaseTypeInfo } from "./databaseTypeInfo";
import type {
  DatabaseInfo,
  DatabaseOverview,
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
  MysqlVersionInfo,
  PostgresVersionInfo,
  RedisKeyDetail,
  RedisOverview,
  RedisVersionInfo,
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
} from "./types";

type DetailTab =
  | "overview"
  | "keys"
  | "console"
  | "data"
  | "sql"
  | "mongoConsole"
  | "mail"
  | "messages"
  | "search"
  | "objectStore"
  | "governance"
  | "backup"
  | "config"
  | "logs"
  | "versions"
  | "docs";
type MetricPoint = { cpu: number; memory: number };
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
type InstallTaskStatus = "running" | "completed" | "failed";
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

const services = ref<ServiceInfo[]>([]);
const selectedKind = ref<ServiceKind>("redis");
const activeTool = ref<ToolId | null>(null);
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
const notice = ref("");
const error = ref("");
const installTask = ref<InstallTask | null>(null);
const installLogExpanded = ref(true);
let serviceTimer: number | undefined;
let metricTimer: number | undefined;
let diskTimer: number | undefined;
let unlistenInstallProgress: UnlistenFn | undefined;

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

const serviceControlBusy = computed(
  () =>
    pendingAction.value !== null ||
    redisVersionChanging.value ||
    mysqlVersionChanging.value ||
    postgresVersionChanging.value,
);
const latestInstallLog = computed(
  () => installTask.value?.logs.at(-1) ?? null,
);

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
  if (!task || task.operationId !== operationId || task.status === "failed") {
    return;
  }
  const message = String(cause);
  task.status = "failed";
  task.stage = "安装失败";
  task.logs.push({
    time: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
    stage: "安装失败",
    message,
  });
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
    time: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
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
    time: new Date().toLocaleTimeString("zh-CN", { hour12: false }),
    stage: payload.stage,
    message: payload.message,
  });
}

const configChanged = computed(
  () => configContent.value !== configOriginal.value,
);

const detailTabs = computed<Array<[DetailTab, string]>>(() => {
  if (selectedKind.value === "redis") {
    return [
      ["overview", "概览"],
      ["keys", "数据浏览"],
      ["console", "命令台"],
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
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "mailpit") {
    return [
      ["overview", "概览"],
      ["mail", "邮件收件箱"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "nats") {
    return [
      ["overview", "概览"],
      ["messages", "消息调试"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "meilisearch") {
    return [
      ["overview", "概览"],
      ["search", "索引与搜索"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "minio" || selectedKind.value === "rustfs") {
    return [
      ["overview", "概览"],
      ["objectStore", "连接与控制台"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["docs", "使用文档"],
    ];
  }
  if (selectedKind.value === "etcd" || selectedKind.value === "consul") {
    return [
      ["overview", "概览"],
      ["governance", "连接与调试"],
      ["backup", "备份恢复"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
      ["docs", "使用文档"],
    ];
  }
  return [
    ["overview", "概览"],
    ["data", "数据浏览"],
    ["sql", "SQL 命令台"],
    ["backup", "备份恢复"],
    ["config", "配置文件"],
    ["logs", "运行日志"],
  ];
});

const statusLabel: Record<ServiceState, string> = {
  not_installed: "未安装",
  stopped: "已停止",
  running: "运行中",
  stale_pid: "状态异常",
};

const iconLetter: Record<ServiceKind, string> = {
  redis: "R",
  mysql: "M",
  postgres: "P",
  mongodb: "M",
  mailpit: "@",
  nats: "N",
  meilisearch: "M",
  minio: "M",
  rustfs: "R",
  etcd: "E",
  consul: "C",
};

const governanceProfile = computed(() =>
  selectedKind.value === "consul"
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

const actionLabel: Record<ServiceAction, string> = {
  install: "安装中",
  start: "启动中",
  stop: "停止中",
  restart: "重启中",
};

async function refreshServices(silent = false) {
  if (!silent) loading.value = true;
  try {
    services.value = await listServices();
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

async function refreshMetrics() {
  if (activeTool.value) return;
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

async function clearInstallCache() {
  const service = selectedService.value;
  if (!service || cacheCleaning.value) return;
  const cacheBytes = selectedDiskUsage.value?.cacheBytes ?? 0;
  if (cacheBytes === 0) {
    notice.value = `${service.name} 没有可清理的安装缓存`;
    return;
  }
  if (
    !window.confirm(
      `将清理 ${service.name} 的下载包和安装临时文件，预计释放 ${formatBytes(cacheBytes)}。已安装程序和数据不会被删除，确定继续吗？`,
    )
  ) {
    return;
  }
  cacheCleaning.value = true;
  try {
    const result = await cleanServiceCache(service.kind);
    await refreshDiskUsage(service.kind);
    notice.value = `已清理 ${result.removedItems} 个缓存项，释放 ${formatBytes(result.freedBytes)}`;
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
    error.value = `请先停止 ${service.name}，再创建一致的数据备份`;
    return;
  }
  backupCreating.value = true;
  try {
    const backup = await createServiceBackup(service.kind);
    await Promise.all([loadBackups(), refreshDiskUsage(service.kind)]);
    notice.value = `备份创建成功：${formatBytes(backup.sizeBytes)}`;
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
    error.value = `请先停止 ${service.name}，再恢复数据`;
    return;
  }
  if (
    !window.confirm(
      `确定将 ${service.name} 恢复到 ${formatBackupDate(backup.createdAtMillis)} 的状态吗？当前 data 和 conf 会先自动备份，然后再替换。`,
    )
  ) {
    return;
  }
  restoringBackupId.value = backup.id;
  try {
    const result = await restoreServiceBackup(service.kind, backup.id);
    await Promise.all([loadBackups(), refreshDiskUsage(service.kind)]);
    notice.value = `恢复成功；恢复前状态已保存为 ${result.safetyBackup.id}`;
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    restoringBackupId.value = null;
  }
}

function selectTool(id: ToolId) {
  activeTool.value = id;
  notice.value = "";
  error.value = "";
}

async function selectService(kind: ServiceKind) {
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
    error.value = "请先停止 Redis，再切换运行版本";
    return;
  }
  if (
    !window.confirm(
      `确定切换到 Redis ${target.version} 吗？各版本使用独立数据目录；切回原版本时会恢复该版本的数据。切换前仍建议创建备份。`,
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
      ? `已切换到 Redis ${target.version}`
      : `Redis ${target.version} 安装并切换成功`;
  } catch (cause) {
    recordInstallFailure(operationId, cause);
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
    error.value = "请先停止 MySQL，再切换运行版本";
    return;
  }
  if (
    !window.confirm(
      `确定切换到 MySQL ${target.version} 吗？每个版本使用独立数据目录，不会自动升级或降级原版本数据。切换前仍建议创建备份。`,
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
      ? `已切换到 MySQL ${target.version}`
      : `MySQL ${target.version} 安装并切换成功`;
  } catch (cause) {
    recordInstallFailure(operationId, cause);
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
    error.value = "请先停止 PostgreSQL，再切换运行版本";
    return;
  }
  if (
    !window.confirm(
      `确定切换到 PostgreSQL ${target.version} 吗？每个主版本使用独立数据目录，不会自动升级或降级原版本数据。切换前仍建议创建备份。`,
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
      ? `已切换到 PostgreSQL ${target.version}`
      : `PostgreSQL ${target.version} 编译安装并切换成功`;
  } catch (cause) {
    recordInstallFailure(operationId, cause);
    error.value = String(cause);
  } finally {
    postgresVersionChanging.value = false;
  }
}

async function execute(action: ServiceAction) {
  const service = selectedService.value;
  if (!service || serviceControlBusy.value) return;

  pendingAction.value = action;
  notice.value = "";
  error.value = "";
  const operationId =
    action === "install"
      ? startInstallTask(service.kind, service.name)
      : undefined;
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
    notice.value = `${service.name} ${
      { install: "安装", start: "启动", stop: "停止", restart: "重启" }[
        action
      ]
    }成功`;
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
            : Promise.resolve(),
    ]);
  } catch (cause) {
    if (operationId) recordInstallFailure(operationId, cause);
    error.value = String(cause);
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
  if (tab === "mail" && mailMessages.value.length === 0) {
    await loadMailMessages();
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
      window.confirm("该命令会清空 Redis 数据，确定继续吗？")
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
      window.confirm("该 MongoDB 命令可能删除数据，确定继续吗？")
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
      window.confirm("该 SQL 会删除数据库对象或数据，确定继续吗？")
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
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN");
}

function formatBackupDate(value: number) {
  return new Date(value).toLocaleString("zh-CN");
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
  try {
    unlistenInstallProgress = await listen<InstallProgressPayload>(
      "install-progress",
      (event) => handleInstallProgress(event.payload),
    );
  } catch {
    // Service management remains usable if the event channel is unavailable.
  }
  await refreshServices();
  await Promise.all([
    refreshMetrics(),
    refreshDiskUsage(),
  ]);
  serviceTimer = window.setInterval(() => refreshServices(true), 3000);
  metricTimer = window.setInterval(async () => {
    if (activeTool.value) return;
    await refreshMetrics();
    if (activeTab.value === "logs") await loadLogs();
  }, 2000);
  diskTimer = window.setInterval(() => refreshDiskUsage(), 60_000);
});

onUnmounted(() => {
  unlistenInstallProgress?.();
  if (serviceTimer) window.clearInterval(serviceTimer);
  if (metricTimer) window.clearInterval(metricTimer);
  if (diskTimer) window.clearInterval(diskTimer);
});
</script>

<template>
  <div class="app-layout">
    <aside class="sidebar">
      <div class="brand">
        <div class="brand-mark"><span></span><span></span><span></span></div>
        <div>
          <strong>智屿</strong>
          <small>轻量本地开发环境</small>
        </div>
      </div>

      <nav class="service-nav">
        <p class="nav-label">SERVICES</p>
        <button
          v-for="service in services"
          :key="service.kind"
          type="button"
          class="service-nav-item"
          :class="{
            active: activeTool === null && selectedKind === service.kind,
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

        <p class="nav-label tool-label">TOOLS</p>
        <button
          v-for="tool in TOOLS"
          :key="tool.id"
          type="button"
          class="service-nav-item"
          :class="{ active: activeTool === tool.id }"
          @click="selectTool(tool.id)"
        >
          <span class="nav-icon" :class="tool.id">{{ tool.icon }}</span>
          <span class="nav-copy">
            <strong>{{ tool.navLabel }}</strong>
            <small>{{ tool.navHint }}</small>
          </span>
        </button>

        <button type="button" class="add-service" disabled>
          <span>＋</span> 扩展更多服务
        </button>
      </nav>

      <div class="sidebar-footer">
        <span class="core-dot"></span>
        <div>
          <strong>智屿 Core</strong>
          <small>运行正常 · ARM64</small>
        </div>
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
          v-if="installTask.status !== 'running'"
          type="button"
          class="install-progress-close"
          aria-label="关闭安装日志"
          @click="installTask = null"
        >
          ×
        </button>
      </section>

      <div v-if="loading" class="page-loading">正在读取服务状态…</div>

      <component
        :is="activeToolDefinition.component"
        v-else-if="activeToolDefinition"
      />

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
            </div>
          </div>

          <div class="header-actions">
            <button
              v-if="selectedService.status === 'not_installed'"
              class="primary"
              type="button"
              :disabled="serviceControlBusy"
              @click="execute('install')"
            >
              <span v-if="serviceControlBusy" class="spinner"></span>
              {{ serviceControlBusy ? "安装中" : "下载并安装" }}
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
                {{ pendingAction === "restart" ? "重启中" : "重启" }}
              </button>
              <button
                class="danger"
                type="button"
                :disabled="serviceControlBusy"
                @click="execute('stop')"
              >
                <span v-if="pendingAction === 'stop'" class="spinner"></span>
                {{ pendingAction === "stop" ? "停止中" : "停止" }}
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
              {{ serviceControlBusy ? "处理中" : "启动服务" }}
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
            {{ tab[1] }}
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
                <h2>Redis 运行版本</h2>
              </div>
              <span>二进制独立安装 · 单个活动版本</span>
            </div>

            <div
              v-if="redisVersionsLoading && redisVersions.length === 0"
              class="redis-version-loading"
            >
              正在读取可用版本…
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
                  <i v-if="release.selected">当前</i>
                  <i v-else-if="release.installed">已安装</i>
                  <i v-if="release.recommended" class="recommended">
                    推荐
                  </i>
                </span>
                <em>{{ release.supportLabel }}</em>
              </button>
            </div>

            <div class="redis-version-footer">
              <p>
                各版本程序和数据相互隔离，数据保存在
                <code>data/版本</code>；基础配置共用。切换前建议备份。
              </p>
              <div>
                <span
                  v-if="
                    selectedService.status === 'running' &&
                    !selectedRedisVersionInfo?.selected
                  "
                >
                  请先停止 Redis
                </span>
                <button
                  type="button"
                  :disabled="
                    !selectedRedisVersionInfo ||
                    selectedRedisVersionInfo.selected ||
                    selectedService.status === 'running' ||
                    serviceControlBusy
                  "
                  @click="changeRedisVersion"
                >
                  <span v-if="redisVersionChanging" class="spinner"></span>
                  {{
                    selectedRedisVersionInfo?.selected
                      ? "当前版本"
                      : redisVersionChanging
                        ? "安装切换中"
                        : selectedRedisVersionInfo?.installed
                          ? "切换版本"
                          : "安装并切换"
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
                <h2>MySQL 运行版本</h2>
              </div>
              <span>二进制与数据独立 · 单个活动版本</span>
            </div>

            <div
              v-if="mysqlVersionsLoading && mysqlVersions.length === 0"
              class="redis-version-loading"
            >
              正在读取可用版本…
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
                  <i v-if="release.selected">当前</i>
                  <i v-else-if="release.installed">已安装</i>
                  <i v-if="release.recommended" class="recommended">
                    推荐
                  </i>
                </span>
                <em>{{ release.supportLabel }}</em>
              </button>
            </div>

            <div class="redis-version-footer">
              <p>
                各版本程序和数据相互隔离，数据保存在
                <code>data/版本</code>。新版本首次切换时会自动初始化空数据库。
              </p>
              <div>
                <span
                  v-if="
                    selectedService.status === 'running' &&
                    !selectedMysqlVersionInfo?.selected
                  "
                >
                  请先停止 MySQL
                </span>
                <button
                  type="button"
                  :disabled="
                    !selectedMysqlVersionInfo ||
                    selectedMysqlVersionInfo.selected ||
                    selectedService.status === 'running' ||
                    serviceControlBusy
                  "
                  @click="changeMysqlVersion"
                >
                  <span v-if="mysqlVersionChanging" class="spinner"></span>
                  {{
                    selectedMysqlVersionInfo?.selected
                      ? "当前版本"
                      : mysqlVersionChanging
                        ? "安装初始化中"
                        : selectedMysqlVersionInfo?.installed
                          ? "切换版本"
                          : "安装并切换"
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
                <h2>PostgreSQL 运行版本</h2>
              </div>
              <span>源码独立构建 · 数据目录隔离</span>
            </div>

            <div
              v-if="
                postgresVersionsLoading && postgresVersions.length === 0
              "
              class="redis-version-loading"
            >
              正在读取可用版本…
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
                  <i v-if="release.selected">当前</i>
                  <i v-else-if="release.installed">已安装</i>
                  <i v-if="release.recommended" class="recommended">
                    推荐
                  </i>
                </span>
                <em>{{ release.supportLabel }}</em>
              </button>
            </div>

            <div class="redis-version-footer">
              <p>
                各主版本使用独立的 <code>data/版本</code> 数据目录。首次切换会编译安装并通过
                <code>initdb</code> 创建空数据库。
              </p>
              <div>
                <span
                  v-if="
                    selectedService.status === 'running' &&
                    !selectedPostgresVersionInfo?.selected
                  "
                >
                  请先停止 PostgreSQL
                </span>
                <button
                  type="button"
                  :disabled="
                    !selectedPostgresVersionInfo ||
                    selectedPostgresVersionInfo.selected ||
                    selectedService.status === 'running' ||
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
                      ? "当前版本"
                      : postgresVersionChanging
                        ? "编译初始化中"
                        : selectedPostgresVersionInfo?.installed
                          ? "切换版本"
                          : "安装并切换"
                  }}
                </button>
              </div>
            </div>
          </div>
        </section>

        <section v-else-if="activeTab === 'overview'" class="overview">
          <div class="metric-grid">
            <article class="metric-card">
              <p>MEMORY</p>
              <strong>{{ formatBytes(metrics.memoryBytes) }}</strong>
              <small>当前进程常驻内存</small>
            </article>
            <article class="metric-card">
              <p>CPU</p>
              <strong>{{
                metrics.cpuPercent === null
                  ? "—"
                  : `${metrics.cpuPercent.toFixed(1)}%`
              }}</strong>
              <small>当前进程使用率</small>
            </article>
            <article class="metric-card">
              <p>UPTIME</p>
              <strong>{{ metrics.uptime ?? "—" }}</strong>
              <small>本次连续运行时间</small>
            </article>
            <article class="metric-card">
              <p>DISK</p>
              <strong>{{
                formatBytes(selectedDiskUsage?.totalBytes ?? null)
              }}</strong>
              <small>程序、数据和下载缓存</small>
            </article>
            <article class="metric-card">
              <p>ENDPOINT</p>
              <strong class="endpoint"
                >127.0.0.1:{{ selectedService.port }}</strong
              >
              <small>仅监听本地连接</small>
            </article>
          </div>

          <div v-if="selectedDiskUsage" class="disk-usage-strip">
            <div>
              <span>程序文件</span>
              <strong>{{
                formatBytes(selectedDiskUsage.installationBytes)
              }}</strong>
            </div>
            <div>
              <span>业务数据</span>
              <strong>{{ formatBytes(selectedDiskUsage.dataBytes) }}</strong>
            </div>
            <div>
              <span>运行日志</span>
              <strong>{{ formatBytes(selectedDiskUsage.logsBytes) }}</strong>
            </div>
            <div>
              <span>配置文件</span>
              <strong>{{ formatBytes(selectedDiskUsage.configBytes) }}</strong>
            </div>
            <div class="cache-usage-cell">
              <span>
                下载缓存
                <button
                  type="button"
                  :disabled="
                    cacheCleaning || selectedDiskUsage.cacheBytes === 0
                  "
                  @click="clearInstallCache"
                >
                  {{ cacheCleaning ? "清理中" : "清理" }}
                </button>
              </span>
              <strong>{{ formatBytes(selectedDiskUsage.cacheBytes) }}</strong>
            </div>
            <div>
              <span>备份文件</span>
              <strong>{{ formatBytes(selectedDiskUsage.backupBytes) }}</strong>
            </div>
            <div>
              <span>其他文件</span>
              <strong>{{ formatBytes(selectedDiskUsage.otherBytes) }}</strong>
            </div>
          </div>

          <div
            v-if="selectedKind === 'redis' && redisOverview"
            class="redis-stat-strip"
          >
            <div>
              <span>Redis 内存</span>
              <strong>{{ formatBytes(redisOverview.usedMemoryBytes) }}</strong>
            </div>
            <div>
              <span>Key 数量</span>
              <strong>{{ redisOverview.totalKeys }}</strong>
            </div>
            <div>
              <span>连接数</span>
              <strong>{{ redisOverview.connectedClients }}</strong>
            </div>
            <div>
              <span>每秒操作</span>
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
              <span>数据库</span>
              <strong>{{ databaseOverview.databaseCount }}</strong>
            </div>
            <div>
              <span>数据表</span>
              <strong>{{ databaseOverview.tableCount }}</strong>
            </div>
            <div>
              <span>当前连接</span>
              <strong>{{ databaseOverview.connectionCount }}</strong>
            </div>
            <div>
              <span>数据大小</span>
              <strong>{{ formatBytes(databaseOverview.dataSizeBytes) }}</strong>
            </div>
          </div>

          <div
            v-if="selectedKind === 'mongodb' && mongoOverview"
            class="redis-stat-strip"
          >
            <div>
              <span>数据库</span>
              <strong>{{ mongoOverview.databaseCount }}</strong>
            </div>
            <div>
              <span>MongoDB 版本</span>
              <strong>{{ mongoOverview.version }}</strong>
            </div>
            <div>
              <span>当前连接</span>
              <strong>{{ mongoOverview.connectionCount }}</strong>
            </div>
            <div>
              <span>数据大小</span>
              <strong>{{ formatBytes(mongoOverview.dataSizeBytes) }}</strong>
            </div>
          </div>

          <div
            v-if="selectedKind === 'mailpit' && mailpitOverview"
            class="redis-stat-strip"
          >
            <div>
              <span>已捕获邮件</span>
              <strong>{{ mailpitOverview.total }}</strong>
            </div>
            <div>
              <span>未读邮件</span>
              <strong>{{ mailpitOverview.unread }}</strong>
            </div>
            <div>
              <span>SMTP 地址</span>
              <strong class="small-value">{{
                mailpitOverview.smtpAddress
              }}</strong>
            </div>
            <div>
              <span>Web 地址</span>
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
              <span>当前连接</span>
              <strong>{{ natsOverview.connections }}</strong>
            </div>
            <div>
              <span>订阅数量</span>
              <strong>{{ natsOverview.subscriptions }}</strong>
            </div>
            <div>
              <span>接收消息</span>
              <strong>{{ natsOverview.inMessages }}</strong>
            </div>
            <div>
              <span>发出消息</span>
              <strong>{{ natsOverview.outMessages }}</strong>
            </div>
          </div>

          <div
            v-if="selectedKind === 'meilisearch' && meilisearchOverview"
            class="redis-stat-strip"
          >
            <div>
              <span>索引数量</span>
              <strong>{{ meilisearchOverview.indexCount }}</strong>
            </div>
            <div>
              <span>文档数量</span>
              <strong>{{ meilisearchOverview.documentCount }}</strong>
            </div>
            <div>
              <span>数据库大小</span>
              <strong>{{
                formatBytes(meilisearchOverview.databaseSizeBytes)
              }}</strong>
            </div>
            <div>
              <span>索引任务</span>
              <strong>{{ meilisearchOverview.indexingCount }}</strong>
            </div>
          </div>

          <div class="overview-columns">
            <article class="panel monitoring-panel">
              <div class="panel-title">
                <div>
                  <p>LIVE MONITORING</p>
                  <h2>实时资源</h2>
                </div>
                <span class="live-badge"><i></i>2 秒刷新</span>
              </div>

              <div
                v-if="selectedService.status !== 'running'"
                class="chart-empty"
              >
                服务启动后显示 CPU 和内存趋势
              </div>
              <div v-else class="charts">
                <div class="chart-block">
                  <div class="chart-label">
                    <span>内存使用</span>
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
                  <h2>服务信息</h2>
                </div>
              </div>
              <dl class="info-list">
                <div>
                  <dt>运行状态</dt>
                  <dd>{{ statusLabel[selectedService.status] }}</dd>
                </div>
                <div>
                  <dt>进程 PID</dt>
                  <dd>{{ selectedService.pid ?? "—" }}</dd>
                </div>
                <div>
                  <dt>配置文件</dt>
                  <dd :title="selectedService.configPath">
                    {{ selectedService.configPath }}
                  </dd>
                </div>
                <div>
                  <dt>数据目录</dt>
                  <dd :title="selectedService.dataPath">
                    {{ selectedService.dataPath }}
                  </dd>
                </div>
                <div>
                  <dt>可执行文件</dt>
                  <dd :title="selectedService.executablePath">
                    {{ selectedService.executablePath }}
                  </dd>
                </div>
              </dl>
            </article>
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
              <h2>命令台</h2>
            </div>
            <label>
              DB
              <select v-model.number="redisDatabase">
                <option v-for="database in 16" :key="database - 1" :value="database - 1">
                  {{ database - 1 }}
                </option>
              </select>
            </label>
            <button
              type="button"
              :disabled="consoleHistory.length === 0"
              @click="consoleHistory = []"
            >
              清空输出
            </button>
          </div>
          <div class="console-output">
            <div v-if="consoleHistory.length === 0" class="console-placeholder">
              输入 Redis 命令，例如 <code>GET user:1</code> 或
              <code>SET greeting "hello world"</code>
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
                  ? '输入 Redis 命令'
                  : '请先启动 Redis'
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
              {{ consoleRunning ? "执行中" : "执行" }}
            </button>
          </form>
          <p class="console-note">
            命令直接发送到本机 Redis；会阻塞服务的命令已禁用，清空数据需要二次确认。
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
              <h2>{{ selectedService.name }} 命令台</h2>
            </div>
            <label>
              数据库
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
            <button
              type="button"
              :disabled="sqlHistory.length === 0"
              @click="sqlHistory = []"
            >
              清空结果
            </button>
          </div>

          <textarea
            v-model="sqlInput"
            class="sql-editor"
            spellcheck="false"
            :disabled="sqlRunning || selectedService.status !== 'running'"
            placeholder="输入 SQL，例如 SELECT * FROM users LIMIT 20;"
            @keydown.meta.enter.prevent="runSqlCommand()"
            @keydown.ctrl.enter.prevent="runSqlCommand()"
          ></textarea>
          <div class="sql-runbar">
            <span>⌘ Enter 执行 · 最多显示 500 行</span>
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
              {{ sqlRunning ? "执行中" : "执行 SQL" }}
            </button>
          </div>

          <div class="sql-results">
            <div v-if="sqlHistory.length === 0" class="console-placeholder">
              SQL 只发送到智屿管理的本地数据库。
            </div>
            <article
              v-for="(entry, entryIndex) in sqlHistory"
              :key="entryIndex"
              :class="{ failed: entry.error }"
            >
              <header>
                <strong>{{ entry.database }} &gt; {{ entry.sql }}</strong>
                <span v-if="entry.result">
                  {{ entry.result.summary }} · {{ entry.result.elapsedMs }} ms
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
                  {{ entry.result.summary }}
                </p>
                <p v-if="entry.result.truncated" class="detail-note">
                  结果超过 500 行，已截断显示。
                </p>
              </template>
            </article>
          </div>
          <p class="console-note">
            删除数据库、Schema、数据表或清空表需要二次确认。
          </p>
        </section>

        <section
          v-else-if="activeTab === 'mongoConsole'"
          class="sql-console-panel"
        >
          <div class="console-head">
            <div>
              <p>MONGODB JSON COMMAND</p>
              <h2>MongoDB 命令台</h2>
            </div>
            <label>
              数据库
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
            <button
              type="button"
              :disabled="mongoCommandHistory.length === 0"
              @click="mongoCommandHistory = []"
            >
              清空结果
            </button>
          </div>

          <textarea
            v-model="mongoCommandInput"
            class="sql-editor"
            spellcheck="false"
            :disabled="
              mongoCommandRunning || selectedService.status !== 'running'
            "
            placeholder='输入 JSON 命令，例如 {"find":"users","filter":{},"limit":20}'
            @keydown.meta.enter.prevent="runMongoCommand()"
            @keydown.ctrl.enter.prevent="runMongoCommand()"
          ></textarea>
          <div class="sql-runbar">
            <span>⌘ Enter 执行 · 只接受一个 JSON 命令对象</span>
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
              {{ mongoCommandRunning ? "执行中" : "执行命令" }}
            </button>
          </div>

          <div class="sql-results">
            <div
              v-if="mongoCommandHistory.length === 0"
              class="console-placeholder"
            >
              示例：<code>{"ping": 1}</code>、
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
            命令直接发送到本机 MongoDB；阻塞服务器的管理命令已禁用，删除数据需要二次确认。
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

        <section v-else-if="activeTab === 'messages'" class="nats-workbench">
          <div
            v-if="selectedService.status !== 'running'"
            class="workbench-empty"
          >
            启动 NATS 后即可发布和订阅本地消息
          </div>
          <template v-else>
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

        <section v-else-if="activeTab === 'backup'" class="backup-panel">
          <div class="backup-head">
            <div>
              <p>DATA SAFETY</p>
              <h2>备份与恢复</h2>
              <span>
                保存数据与配置到
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
                {{ backupLoading ? "读取中" : "刷新" }}
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
                {{ backupCreating ? "备份中" : "创建备份" }}
              </button>
            </div>
          </div>

          <div
            v-if="selectedService.status === 'running'"
            class="backup-warning"
          >
            为保证数据一致性，请先停止 {{ selectedService.name }}，再创建或恢复备份。
          </div>
          <div
            v-else-if="selectedService.status === 'not_installed'"
            class="backup-warning"
          >
            服务安装后才能创建数据备份。
          </div>

          <div class="backup-list">
            <div class="backup-list-head">
              <span>备份时间</span>
              <span>类型</span>
              <span>压缩后大小</span>
              <span>操作</span>
            </div>
            <div v-if="backupLoading && backups.length === 0" class="backup-empty">
              正在读取备份…
            </div>
            <div v-else-if="backups.length === 0" class="backup-empty">
              还没有备份。停止服务后点击“创建备份”。
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
                  {{ backup.automatic ? "恢复前安全备份" : "手动备份" }}
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
                    restoringBackupId === backup.id ? "恢复中" : "恢复"
                  }}
                </button>
              </div>
            </article>
          </div>

          <div class="backup-notes">
            <p>
              <strong>备份范围：</strong>
              <code>data/</code> 与 <code>conf/</code>。日志、PID 和程序文件不会进入备份。
            </p>
            <p>
              <strong>恢复保护：</strong>
              恢复前会自动保存当前状态；压缩包通过路径与文件类型检查后才会替换实例目录。
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
          </div>
          <pre>{{ logs }}</pre>
        </section>
      </template>
    </main>
  </div>
</template>
