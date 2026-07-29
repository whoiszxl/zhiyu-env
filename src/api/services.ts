import { invoke } from "@tauri-apps/api/core";
import type {
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
