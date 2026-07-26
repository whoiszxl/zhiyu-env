<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import {
  executeSql,
  executeRedisCommand,
  executeMongoCommand,
  getMailpitMessageDetail,
  getMailpitOverview,
  getDatabaseOverview,
  getMongoCollectionDetail,
  getMongoOverview,
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
  listPortListeners,
  listServices,
  readServiceConfig,
  runServiceAction,
  saveServiceConfig,
  scanRedisKeys,
} from "./api/services";
import { databaseTypeInfo } from "./databaseTypeInfo";
import type {
  DatabaseInfo,
  DatabaseOverview,
  MailDetail,
  MailpitOverview,
  MailSummary,
  MongoCollectionDetail,
  MongoCollectionInfo,
  MongoDatabaseInfo,
  MongoOverview,
  PortListener,
  RedisKeyDetail,
  RedisOverview,
  ServiceAction,
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
  | "config"
  | "logs";
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
type ActiveTool = "ports";
type MongoConsoleEntry = {
  database: string;
  command: string;
  output: unknown;
  elapsedMs: number;
  error: string;
};

const services = ref<ServiceInfo[]>([]);
const selectedKind = ref<ServiceKind>("redis");
const activeTool = ref<ActiveTool | null>(null);
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
const configContent = ref("");
const configOriginal = ref("");
const configLoading = ref(false);
const configSaving = ref(false);
const logs = ref("暂无日志");
const logsLoading = ref(false);
const redisOverview = ref<RedisOverview | null>(null);
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
const mailMessages = ref<MailSummary[]>([]);
const selectedMailId = ref<string | null>(null);
const mailDetail = ref<MailDetail | null>(null);
const mailLoading = ref(false);
const mailDetailLoading = ref(false);
const portListeners = ref<PortListener[]>([]);
const portQuery = ref("");
const portLoading = ref(false);
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
let serviceTimer: number | undefined;
let metricTimer: number | undefined;
let diskTimer: number | undefined;

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

const filteredPortListeners = computed(() => {
  const query = portQuery.value.trim().toLowerCase();
  if (!query) return portListeners.value;
  return portListeners.value.filter((listener) =>
    [
      listener.port,
      listener.address,
      listener.pid,
      listener.process,
      listener.managedService,
      listener.commonService,
    ]
      .filter((value) => value !== null)
      .some((value) => String(value).toLowerCase().includes(query)),
  );
});

const portProcessCount = computed(
  () => new Set(portListeners.value.map((listener) => listener.pid)).size,
);

const publicPortCount = computed(
  () =>
    portListeners.value.filter((listener) =>
      ["*", "0.0.0.0", "[::]"].includes(listener.address),
    ).length,
);

const configChanged = computed(
  () => configContent.value !== configOriginal.value,
);

const detailTabs = computed<Array<[DetailTab, string]>>(() => {
  if (selectedKind.value === "redis") {
    return [
      ["overview", "概览"],
      ["keys", "数据浏览"],
      ["console", "命令台"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
    ];
  }
  if (selectedKind.value === "mongodb") {
    return [
      ["overview", "概览"],
      ["data", "数据浏览"],
      ["mongoConsole", "JSON 命令台"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
    ];
  }
  if (selectedKind.value === "mailpit") {
    return [
      ["overview", "概览"],
      ["mail", "邮件收件箱"],
      ["config", "配置文件"],
      ["logs", "运行日志"],
    ];
  }
  return [
    ["overview", "概览"],
    ["data", "数据浏览"],
    ["sql", "SQL 命令台"],
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
};

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
    } else if (!databaseOverview.value) {
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
  mailMessages.value = [];
  selectedMailId.value = null;
  mailDetail.value = null;
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
  await Promise.all([refreshMetrics(), refreshDiskUsage(kind)]);
}

async function selectPortTool() {
  activeTool.value = "ports";
  notice.value = "";
  error.value = "";
  await loadPortListeners();
}

async function loadPortListeners(silent = false) {
  if (portLoading.value) return;
  portLoading.value = true;
  try {
    portListeners.value = await listPortListeners();
    error.value = "";
  } catch (cause) {
    if (!silent) error.value = String(cause);
  } finally {
    portLoading.value = false;
  }
}

async function execute(action: ServiceAction) {
  const service = selectedService.value;
  if (!service || pendingAction.value) return;

  pendingAction.value = action;
  notice.value = "";
  error.value = "";
  try {
    const updated = await runServiceAction(action, service.kind);
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
    await Promise.all([
      refreshMetrics(),
      refreshDiskUsage(service.kind),
    ]);
  } catch (cause) {
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
  if (tab === "mail" && mailMessages.value.length === 0) {
    await loadMailMessages();
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

function formatBytes(value: number | null) {
  if (value === null) return "—";
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(0)} KiB`;
  if (value < 1024 * 1024 * 1024) {
    return `${(value / 1024 / 1024).toFixed(1)} MiB`;
  }
  return `${(value / 1024 / 1024 / 1024).toFixed(2)} GiB`;
}

function formatMailDate(value: string) {
  if (!value) return "—";
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString("zh-CN");
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
  await refreshServices();
  await Promise.all([refreshMetrics(), refreshDiskUsage()]);
  serviceTimer = window.setInterval(() => refreshServices(true), 3000);
  metricTimer = window.setInterval(async () => {
    if (activeTool.value === "ports") {
      await loadPortListeners(true);
      return;
    }
    await refreshMetrics();
    if (activeTab.value === "logs") await loadLogs();
  }, 2000);
  diskTimer = window.setInterval(() => refreshDiskUsage(), 60_000);
});

onUnmounted(() => {
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
          :class="{ active: selectedKind === service.kind }"
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
          type="button"
          class="service-nav-item"
          :class="{ active: activeTool === 'ports' }"
          @click="selectPortTool"
        >
          <span class="nav-icon ports">↔</span>
          <span class="nav-copy">
            <strong>端口检查器</strong>
            <small>TCP LISTEN</small>
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
      <div v-if="loading" class="page-loading">正在读取服务状态…</div>

      <template v-else-if="activeTool === 'ports'">
        <header class="detail-header">
          <div class="detail-identity">
            <span class="service-logo ports">↔</span>
            <div>
              <div class="title-line">
                <h1>端口检查器</h1>
                <span>TCP LISTEN</span>
              </div>
              <p>查看本机正在监听的 TCP 端口，不修改任何进程</p>
            </div>
          </div>
          <div class="header-actions">
            <button
              class="primary"
              type="button"
              :disabled="portLoading"
              @click="loadPortListeners()"
            >
              <span v-if="portLoading" class="spinner"></span>
              {{ portLoading ? "检查中" : "重新检查" }}
            </button>
          </div>
        </header>

        <div v-if="error" class="notice danger">
          <span>{{ error }}</span>
          <button type="button" @click="error = ''">×</button>
        </div>

        <section class="port-tool-page">
          <div class="metric-grid">
            <article class="metric-card">
              <p>LISTENERS</p>
              <strong>{{ portListeners.length }}</strong>
              <small>正在监听的地址</small>
            </article>
            <article class="metric-card">
              <p>PROCESSES</p>
              <strong>{{ portProcessCount }}</strong>
              <small>占用端口的进程</small>
            </article>
            <article class="metric-card">
              <p>ZHIYU</p>
              <strong>{{
                portListeners.filter((item) => item.managedService).length
              }}</strong>
              <small>智屿管理的监听地址</small>
            </article>
            <article class="metric-card">
              <p>ALL INTERFACES</p>
              <strong>{{ publicPortCount }}</strong>
              <small>监听全部网络接口</small>
            </article>
          </div>

          <div class="port-panel">
            <div class="port-toolbar">
              <div>
                <p>LOCAL PORTS</p>
                <h2>监听端口</h2>
              </div>
              <label>
                筛选
                <input
                  v-model="portQuery"
                  type="search"
                  placeholder="端口、进程或服务"
                />
              </label>
            </div>

            <div v-if="portLoading && portListeners.length === 0" class="port-empty">
              正在读取本机端口…
            </div>
            <div
              v-else-if="filteredPortListeners.length === 0"
              class="port-empty"
            >
              {{ portQuery ? "没有匹配的监听端口" : "当前没有 TCP 监听端口" }}
            </div>
            <div v-else class="port-table-wrap">
              <table class="port-table">
                <thead>
                  <tr>
                    <th>端口</th>
                    <th>监听地址</th>
                    <th>进程</th>
                    <th>PID</th>
                    <th>归属</th>
                    <th>常见用途</th>
                  </tr>
                </thead>
                <tbody>
                  <tr
                    v-for="listener in filteredPortListeners"
                    :key="`${listener.pid}-${listener.address}-${listener.port}`"
                  >
                    <td><code>{{ listener.port }}</code></td>
                    <td>
                      <code>{{ listener.address }}:{{ listener.port }}</code>
                      <span
                        v-if="
                          ['*', '0.0.0.0', '[::]'].includes(listener.address)
                        "
                        class="network-badge"
                      >
                        全部网卡
                      </span>
                    </td>
                    <td>{{ listener.process || "未知进程" }}</td>
                    <td><code>{{ listener.pid }}</code></td>
                    <td>
                      <span
                        v-if="listener.managedService"
                        class="ownership-badge managed"
                      >
                        智屿 · {{ listener.managedService }}
                      </span>
                      <span v-else class="ownership-badge">外部进程</span>
                    </td>
                    <td>{{ listener.commonService ?? "—" }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p class="port-note">
              这里只显示 TCP 监听端口。监听 <code>127.0.0.1</code> 或
              <code>::1</code> 的服务仅供本机访问；“全部网卡”表示局域网设备也可能连接。
            </p>
          </div>
        </section>
      </template>

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
                    { busy: pendingAction !== null },
                  ]"
                ></i>
                {{
                  pendingAction
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
              :disabled="pendingAction !== null"
              @click="execute('install')"
            >
              <span v-if="pendingAction" class="spinner"></span>
              {{ pendingAction ? "安装中" : "下载并安装" }}
            </button>
            <template v-else-if="selectedService.status === 'running'">
              <button
                type="button"
                :disabled="pendingAction !== null"
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
                :disabled="pendingAction !== null"
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
              :disabled="pendingAction !== null"
              @click="execute('start')"
            >
              <span v-if="pendingAction" class="spinner"></span>
              {{ pendingAction ? "启动中" : "启动服务" }}
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

        <section v-if="activeTab === 'overview'" class="overview">
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
            <div>
              <span>下载缓存</span>
              <strong>{{ formatBytes(selectedDiskUsage.cacheBytes) }}</strong>
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
