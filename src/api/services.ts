import { invoke } from "@tauri-apps/api/core";
import type {
  AiConnectionTestResult,
  AiSettings,
  AiSettingsInput,
  AppSettings,
  CacheCleanupResult,
  DatabaseInfo,
  DatabaseOverview,
  DiagnosticRepairResult,
  DiagnosticReport,
  DuckdbQueryResult,
  DuckdbStatus,
  EnvironmentMetrics,
  MailDetail,
  MailpitOverview,
  MailSummary,
  MeilisearchIndex,
  MeilisearchOverview,
  MeilisearchSearchResult,
  MeilisearchTask,
  MongoCollectionDetail,
  MongoCollectionInfo,
  MongoCommandResult,
  MongoDatabaseInfo,
  MongoOverview,
  NatsMessage,
  NatsOverview,
  NatsPublishResult,
  KafkaOverview,
  KafkaPublishResult,
  KafkaTopic,
  PortListener,
  ServiceAction,
  ServiceDiskUsage,
  ServiceInfo,
  ServiceKind,
  ServiceMetrics,
  RedisCommandResult,
  RedisKeyDetail,
  RedisOverview,
  RedisScanResult,
  RedisVersionInfo,
  MysqlVersionInfo,
  PostgresVersionInfo,
  NginxVersionInfo,
  ManagedServiceVersionInfo,
  RestoreResult,
  ServiceBackup,
  SqlResult,
  SqlServiceKind,
  TableDetail,
  TableInfo,
  UpdateStatus,
  VersionUninstallResult,
} from "../types";

export function listServices(): Promise<ServiceInfo[]> {
  return invoke<ServiceInfo[]>("service_list");
}

export function runServiceAction(
  action: ServiceAction,
  kind: ServiceKind,
  operationId?: string,
): Promise<ServiceInfo> {
  return invoke<ServiceInfo>(`service_${action}`, {
    kind,
    ...(operationId ? { operationId } : {}),
  });
}

export function cancelInstall(operationId: string): Promise<void> {
  return invoke<void>("service_install_cancel", { operationId });
}

export function getServiceMetrics(
  kind: ServiceKind,
): Promise<ServiceMetrics> {
  return invoke<ServiceMetrics>("service_metrics", { kind });
}

export function getServiceDiskUsage(
  kind: ServiceKind,
): Promise<ServiceDiskUsage> {
  return invoke<ServiceDiskUsage>("service_disk_usage", { kind });
}

export function getEnvironmentMetrics(): Promise<EnvironmentMetrics> {
  return invoke<EnvironmentMetrics>("environment_metrics");
}

export function getEnvironmentDiskUsage(): Promise<number> {
  return invoke<number>("environment_disk_usage");
}

export function runAppDiagnostics(): Promise<DiagnosticReport> {
  return invoke<DiagnosticReport>("app_diagnostics_run");
}

export function repairAppDiagnostics(): Promise<DiagnosticRepairResult> {
  return invoke<DiagnosticRepairResult>("app_diagnostics_repair");
}

export function getAppSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("app_settings_get");
}

export function getAiSettings(): Promise<AiSettings> {
  return invoke<AiSettings>("ai_settings_get");
}

export function saveAiSettings(input: AiSettingsInput): Promise<AiSettings> {
  return invoke<AiSettings>("ai_settings_save", { input });
}

export function testAiConnection(
  input: AiSettingsInput,
): Promise<AiConnectionTestResult> {
  return invoke<AiConnectionTestResult>("ai_connection_test", { input });
}

export function importAiAvatar(
  role: "user" | "assistant",
  sourcePath: string,
): Promise<AiSettings> {
  return invoke<AiSettings>("ai_avatar_import", { role, sourcePath });
}

export function removeAiAvatar(
  role: "user" | "assistant",
): Promise<AiSettings> {
  return invoke<AiSettings>("ai_avatar_remove", { role });
}

export function saveAppSettings(settings: AppSettings): Promise<AppSettings> {
  return invoke<AppSettings>("app_settings_save", { settings });
}

export function importAppBackground(sourcePath: string): Promise<string> {
  return invoke<string>("app_background_import", { sourcePath });
}

export function removeAppBackground(): Promise<void> {
  return invoke<void>("app_background_remove");
}

export function checkAppUpdate(): Promise<UpdateStatus> {
  return invoke<UpdateStatus>("app_update_check");
}

export function cleanAllInstallCache(): Promise<CacheCleanupResult> {
  return invoke<CacheCleanupResult>("app_cache_clean_all");
}

export function stopAllManagedServices(): Promise<ServiceInfo[]> {
  return invoke<ServiceInfo[]>("service_stop_all");
}

export function forceStopService(kind: ServiceKind): Promise<ServiceInfo> {
  return invoke<ServiceInfo>("service_force_stop", { kind });
}

export function repairServiceState(kind: ServiceKind): Promise<ServiceInfo> {
  return invoke<ServiceInfo>("service_repair", { kind });
}

export function cleanServiceCache(
  kind: ServiceKind,
): Promise<CacheCleanupResult> {
  return invoke<CacheCleanupResult>("service_cache_clean", { kind });
}

export function listServiceBackups(
  kind: ServiceKind,
): Promise<ServiceBackup[]> {
  return invoke<ServiceBackup[]>("service_backup_list", { kind });
}

export function createServiceBackup(
  kind: ServiceKind,
): Promise<ServiceBackup> {
  return invoke<ServiceBackup>("service_backup_create", { kind });
}

export function restoreServiceBackup(
  kind: ServiceKind,
  backupId: string,
): Promise<RestoreResult> {
  return invoke<RestoreResult>("service_backup_restore", {
    kind,
    backupId,
  });
}

export function readServiceConfig(kind: ServiceKind): Promise<string> {
  return invoke<string>("service_config_read", { kind });
}

export function saveServiceConfig(
  kind: ServiceKind,
  content: string,
): Promise<void> {
  return invoke<void>("service_config_save", { kind, content });
}

export function getServiceLogs(kind: ServiceKind): Promise<string> {
  return invoke<string>("service_logs", { kind });
}

export function getRedisOverview(): Promise<RedisOverview> {
  return invoke<RedisOverview>("redis_overview");
}

export function listRedisVersions(): Promise<RedisVersionInfo[]> {
  return invoke<RedisVersionInfo[]>("redis_versions");
}

export function selectRedisVersion(
  version: string,
  operationId: string,
): Promise<ServiceInfo> {
  return invoke<ServiceInfo>("redis_version_select", {
    version,
    operationId,
  });
}

export function listMysqlVersions(): Promise<MysqlVersionInfo[]> {
  return invoke<MysqlVersionInfo[]>("mysql_versions");
}

export function selectMysqlVersion(
  version: string,
  operationId: string,
): Promise<ServiceInfo> {
  return invoke<ServiceInfo>("mysql_version_select", {
    version,
    operationId,
  });
}

export function listPostgresVersions(): Promise<PostgresVersionInfo[]> {
  return invoke<PostgresVersionInfo[]>("postgres_versions");
}

export function selectPostgresVersion(
  version: string,
  operationId: string,
): Promise<ServiceInfo> {
  return invoke<ServiceInfo>("postgres_version_select", {
    version,
    operationId,
  });
}

export function listNginxVersions(): Promise<NginxVersionInfo[]> {
  return invoke<NginxVersionInfo[]>("nginx_versions");
}

export function selectNginxVersion(
  version: string,
  operationId: string,
): Promise<ServiceInfo> {
  return invoke<ServiceInfo>("nginx_version_select", {
    version,
    operationId,
  });
}

export function listManagedServiceVersions(
  kind: ServiceKind,
): Promise<ManagedServiceVersionInfo[]> {
  return invoke<ManagedServiceVersionInfo[]>("service_versions", { kind });
}

export function selectManagedServiceVersion(
  kind: ServiceKind,
  version: string,
  operationId: string,
): Promise<ServiceInfo> {
  return invoke<ServiceInfo>("service_version_select", {
    kind,
    version,
    operationId,
  });
}

export function uninstallServiceVersion(
  kind: ServiceKind,
  version: string,
): Promise<VersionUninstallResult> {
  return invoke<VersionUninstallResult>("service_version_uninstall", {
    kind,
    version,
  });
}

export function scanRedisKeys(
  database: number,
  cursor: string,
  pattern: string,
): Promise<RedisScanResult> {
  return invoke<RedisScanResult>("redis_scan_keys", {
    database,
    cursor,
    pattern,
  });
}

export function getRedisKeyDetail(
  database: number,
  key: string,
): Promise<RedisKeyDetail> {
  return invoke<RedisKeyDetail>("redis_key_detail", { database, key });
}

export function executeRedisCommand(
  database: number,
  arguments_: string[],
  confirmed = false,
): Promise<RedisCommandResult> {
  return invoke<RedisCommandResult>("redis_execute", {
    database,
    arguments: arguments_,
    confirmed,
  });
}

export function getDatabaseOverview(
  kind: SqlServiceKind,
): Promise<DatabaseOverview> {
  return invoke<DatabaseOverview>("database_overview", { kind });
}

export function listDatabases(
  kind: SqlServiceKind,
): Promise<DatabaseInfo[]> {
  return invoke<DatabaseInfo[]>("database_list", { kind });
}

export function listDatabaseTables(
  kind: SqlServiceKind,
  database: string,
): Promise<TableInfo[]> {
  return invoke<TableInfo[]>("database_tables", { kind, database });
}

export function getTableDetail(
  kind: SqlServiceKind,
  database: string,
  schema: string,
  table: string,
): Promise<TableDetail> {
  return invoke<TableDetail>("database_table_detail", {
    kind,
    database,
    schema,
    table,
  });
}

export function executeSql(
  kind: SqlServiceKind,
  database: string,
  sql: string,
  confirmed = false,
): Promise<SqlResult> {
  return invoke<SqlResult>("database_execute", {
    kind,
    database,
    sql,
    confirmed,
  });
}

export function getMongoOverview(): Promise<MongoOverview> {
  return invoke<MongoOverview>("mongo_overview");
}

export function listMongoDatabases(): Promise<MongoDatabaseInfo[]> {
  return invoke<MongoDatabaseInfo[]>("mongo_databases");
}

export function listMongoCollections(
  database: string,
): Promise<MongoCollectionInfo[]> {
  return invoke<MongoCollectionInfo[]>("mongo_collections", { database });
}

export function getMongoCollectionDetail(
  database: string,
  collection: string,
): Promise<MongoCollectionDetail> {
  return invoke<MongoCollectionDetail>("mongo_collection_detail", {
    database,
    collection,
  });
}

export function executeMongoCommand(
  database: string,
  command: string,
  confirmed = false,
): Promise<MongoCommandResult> {
  return invoke<MongoCommandResult>("mongo_execute", {
    database,
    command,
    confirmed,
  });
}

export function listPortListeners(): Promise<PortListener[]> {
  return invoke<PortListener[]>("port_listeners");
}

export function getMailpitOverview(): Promise<MailpitOverview> {
  return invoke<MailpitOverview>("mailpit_overview");
}

export function listMailpitMessages(): Promise<MailSummary[]> {
  return invoke<MailSummary[]>("mailpit_messages");
}

export function getMailpitMessageDetail(id: string): Promise<MailDetail> {
  return invoke<MailDetail>("mailpit_message_detail", { id });
}

export function getNatsOverview(): Promise<NatsOverview> {
  return invoke<NatsOverview>("nats_overview");
}

export function publishNatsMessage(
  subject: string,
  payload: string,
): Promise<NatsPublishResult> {
  return invoke<NatsPublishResult>("nats_publish", { subject, payload });
}

export function receiveNatsMessage(subject: string): Promise<NatsMessage> {
  return invoke<NatsMessage>("nats_receive", { subject });
}

export function getKafkaOverview(): Promise<KafkaOverview> {
  return invoke<KafkaOverview>("kafka_overview");
}

export function listKafkaTopics(): Promise<KafkaTopic[]> {
  return invoke<KafkaTopic[]>("kafka_topics");
}

export function createKafkaTopic(
  name: string,
  partitions: number,
): Promise<KafkaTopic[]> {
  return invoke<KafkaTopic[]>("kafka_topic_create", { name, partitions });
}

export function deleteKafkaTopic(name: string): Promise<KafkaTopic[]> {
  return invoke<KafkaTopic[]>("kafka_topic_delete", { name });
}

export function publishKafkaMessage(
  topic: string,
  key: string,
  payload: string,
): Promise<KafkaPublishResult> {
  return invoke<KafkaPublishResult>("kafka_publish", {
    topic,
    key: key || null,
    payload,
  });
}

export function getMeilisearchOverview(): Promise<MeilisearchOverview> {
  return invoke<MeilisearchOverview>("meilisearch_overview");
}

export function listMeilisearchIndexes(): Promise<MeilisearchIndex[]> {
  return invoke<MeilisearchIndex[]>("meilisearch_indexes");
}

export function addMeilisearchDocuments(
  indexUid: string,
  primaryKey: string,
  documents: string,
): Promise<MeilisearchTask> {
  return invoke<MeilisearchTask>("meilisearch_add_documents", {
    indexUid,
    primaryKey,
    documents,
  });
}

export function searchMeilisearch(
  indexUid: string,
  query: string,
): Promise<MeilisearchSearchResult> {
  return invoke<MeilisearchSearchResult>("meilisearch_search", {
    indexUid,
    query,
  });
}

export function getDuckdbStatus(): Promise<DuckdbStatus> {
  return invoke<DuckdbStatus>("duckdb_status");
}

export function testServiceConnection(kind: ServiceKind): Promise<void> {
  return invoke<void>("service_test_connection", { kind });
}

// ── 剪贴板历史 ────────────────────────────────────────────

import type { ClipboardItem, ClipboardStatus, ClipboardSettings } from "../types";

export function clipboardStart(): Promise<{ runState: string }> {
  return invoke<{ runState: string }>("clipboard_start");
}

export function clipboardStop(): Promise<void> {
  return invoke<void>("clipboard_stop");
}

export function clipboardPause(): Promise<void> {
  return invoke<void>("clipboard_pause");
}

export function clipboardResume(): Promise<void> {
  return invoke<void>("clipboard_resume");
}

export function clipboardStatus(): Promise<ClipboardStatus> {
  return invoke<ClipboardStatus>("clipboard_status");
}

export function clipboardList(
  search?: string,
  limit?: number,
  offset?: number,
): Promise<ClipboardItem[]> {
  return invoke<ClipboardItem[]>("clipboard_list", { search, limit, offset });
}

export function clipboardCopy(id: number): Promise<string> {
  return invoke<string>("clipboard_copy", { id });
}

export function clipboardPin(id: number): Promise<void> {
  return invoke<void>("clipboard_pin", { id });
}

export function clipboardDelete(id: number): Promise<void> {
  return invoke<void>("clipboard_delete", { id });
}

export function clipboardClear(): Promise<number> {
  return invoke<number>("clipboard_clear");
}

export function clipboardSettingsGet(): Promise<ClipboardSettings> {
  return invoke<ClipboardSettings>("clipboard_settings_get");
}

export function clipboardSettingsSave(settings: ClipboardSettings): Promise<ClipboardStatus> {
  return invoke<ClipboardStatus>("clipboard_settings_save", { settings });
}

export function installDuckdb(operationId: string): Promise<DuckdbStatus> {
  return invoke<DuckdbStatus>("duckdb_install", { operationId });
}

export function queryDuckdbFile(
  path: string,
  sql: string,
): Promise<DuckdbQueryResult> {
  return invoke<DuckdbQueryResult>("duckdb_query", { path, sql });
}

// ── RSS 订阅 ──────────────────────────────────────────────

import type {
  RssEntry,
  RssAiAction,
  RssAiResult,
  RssFeed,
  RssFeedUpdate,
  RssImportResult,
  RssRefreshResult,
} from "../types";

export function rssFeedsList(): Promise<RssFeed[]> {
  return invoke<RssFeed[]>("rss_feeds_list");
}

export function rssFeedAdd(feedUrl: string): Promise<RssRefreshResult> {
  return invoke<RssRefreshResult>("rss_feed_add", { feedUrl });
}

export function rssFeedDelete(id: number): Promise<void> {
  return invoke<void>("rss_feed_delete", { id });
}

export function rssFeedUpdate(id: number, update: RssFeedUpdate): Promise<void> {
  return invoke<void>("rss_feed_update", { id, update });
}

export function rssFeedRefresh(id: number): Promise<RssRefreshResult> {
  return invoke<RssRefreshResult>("rss_feed_refresh", { id });
}

export function rssRefreshDue(): Promise<RssRefreshResult[]> {
  return invoke<RssRefreshResult[]>("rss_refresh_due");
}

export function rssEntriesList(
  feedId?: number,
  filter = "all",
  search?: string,
  limit = 200,
  offset = 0,
): Promise<RssEntry[]> {
  return invoke<RssEntry[]>("rss_entries_list", {
    feedId,
    filter,
    search,
    limit,
    offset,
  });
}

export function rssEntryRead(id: number, read: boolean): Promise<void> {
  return invoke<void>("rss_entry_read", { id, read });
}

export function rssEntryStar(id: number, starred: boolean): Promise<void> {
  return invoke<void>("rss_entry_star", { id, starred });
}

export function rssMarkAllRead(feedId?: number): Promise<number> {
  return invoke<number>("rss_mark_all_read", { feedId });
}

export function rssImportOpml(content: string): Promise<RssImportResult> {
  return invoke<RssImportResult>("rss_import_opml", { content });
}

export function rssExportOpml(): Promise<string> {
  return invoke<string>("rss_export_opml");
}

export function rssAiResultsList(entryId: number): Promise<RssAiResult[]> {
  return invoke<RssAiResult[]>("rss_ai_results_list", { entryId });
}

export function rssAiGenerate(input: {
  entryId: number;
  requestId: string;
  action: RssAiAction;
  question: string;
  outputLanguage: string;
}): Promise<void> {
  return invoke<void>("rss_ai_generate", { input });
}

export function rssAiCancel(requestId: string): Promise<void> {
  return invoke<void>("rss_ai_cancel", { requestId });
}

export function rssAiResultDelete(id: number): Promise<void> {
  return invoke<void>("rss_ai_result_delete", { id });
}

// ── 开发运行时 ────────────────────────────────────────────

import type {
  RuntimeDiagnostic,
  RuntimeKind,
  RuntimeOverview,
  RuntimeProject,
} from "../types";

export function runtimeOverview(kind: RuntimeKind): Promise<RuntimeOverview> {
  return invoke<RuntimeOverview>("runtime_overview", { kind });
}

export function runtimeInstall(
  kind: RuntimeKind,
  version: string,
  operationId: string,
): Promise<RuntimeOverview> {
  return invoke<RuntimeOverview>("runtime_install", {
    kind,
    version,
    operationId,
  });
}

export function runtimeSelect(
  kind: RuntimeKind,
  version: string,
): Promise<RuntimeOverview> {
  return invoke<RuntimeOverview>("runtime_select", { kind, version });
}

export function runtimeUninstall(
  kind: RuntimeKind,
  version: string,
): Promise<RuntimeOverview> {
  return invoke<RuntimeOverview>("runtime_uninstall", { kind, version });
}

export function runtimeDiagnose(
  kind: RuntimeKind,
  version?: string,
): Promise<RuntimeDiagnostic> {
  return invoke<RuntimeDiagnostic>("runtime_diagnose", { kind, version });
}

export function runtimeSetGoProxy(proxy: string): Promise<RuntimeOverview> {
  return invoke<RuntimeOverview>("runtime_go_proxy_set", { proxy });
}

export function runtimeProjectsList(): Promise<RuntimeProject[]> {
  return invoke<RuntimeProject[]>("runtime_projects_list");
}

export function runtimeProjectSave(
  project: RuntimeProject,
): Promise<RuntimeProject[]> {
  return invoke<RuntimeProject[]>("runtime_project_save", { project });
}

export function runtimeProjectDelete(id: string): Promise<RuntimeProject[]> {
  return invoke<RuntimeProject[]>("runtime_project_delete", { id });
}

export function runtimeProjectManifestExport(id: string): Promise<string> {
  return invoke<string>("runtime_project_manifest_export", { id });
}

export function runtimeProjectManifestImport(
  path: string,
): Promise<RuntimeProject[]> {
  return invoke<RuntimeProject[]>("runtime_project_manifest_import", { path });
}

export function runtimeOpenTerminal(
  kind: RuntimeKind,
  projectPath?: string,
  version?: string,
): Promise<void> {
  return invoke<void>("runtime_open_terminal", { kind, projectPath, version });
}
