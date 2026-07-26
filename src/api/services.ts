import { invoke } from "@tauri-apps/api/core";
import type {
  CacheCleanupResult,
  DatabaseInfo,
  DatabaseOverview,
  DuckdbQueryResult,
  DuckdbStatus,
  MailDetail,
  MailpitOverview,
  MailSummary,
  MongoCollectionDetail,
  MongoCollectionInfo,
  MongoCommandResult,
  MongoDatabaseInfo,
  MongoOverview,
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
  RestoreResult,
  ServiceBackup,
  SqlResult,
  SqlServiceKind,
  TableDetail,
  TableInfo,
} from "../types";

export function listServices(): Promise<ServiceInfo[]> {
  return invoke<ServiceInfo[]>("service_list");
}

export function runServiceAction(
  action: ServiceAction,
  kind: ServiceKind,
): Promise<ServiceInfo> {
  return invoke<ServiceInfo>(`service_${action}`, { kind });
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

export function selectRedisVersion(version: string): Promise<ServiceInfo> {
  return invoke<ServiceInfo>("redis_version_select", { version });
}

export function listMysqlVersions(): Promise<MysqlVersionInfo[]> {
  return invoke<MysqlVersionInfo[]>("mysql_versions");
}

export function selectMysqlVersion(version: string): Promise<ServiceInfo> {
  return invoke<ServiceInfo>("mysql_version_select", { version });
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

export function getDuckdbStatus(): Promise<DuckdbStatus> {
  return invoke<DuckdbStatus>("duckdb_status");
}

export function installDuckdb(): Promise<DuckdbStatus> {
  return invoke<DuckdbStatus>("duckdb_install");
}

export function queryDuckdbFile(
  path: string,
  sql: string,
): Promise<DuckdbQueryResult> {
  return invoke<DuckdbQueryResult>("duckdb_query", { path, sql });
}
