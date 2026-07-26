export type ServiceKind =
  | "redis"
  | "mysql"
  | "postgres"
  | "mongodb"
  | "mailpit";
export type SqlServiceKind = "mysql" | "postgres";

export type ServiceState =
  | "not_installed"
  | "stopped"
  | "running"
  | "stale_pid";

export interface ServiceInfo {
  kind: ServiceKind;
  name: string;
  version: string;
  port: number;
  status: ServiceState;
  pid: number | null;
  instanceDir: string;
  configPath: string;
  dataPath: string;
  logPath: string;
  executablePath: string;
}

export type ServiceAction = "install" | "start" | "stop" | "restart";

export interface ServiceMetrics {
  running: boolean;
  cpuPercent: number | null;
  memoryBytes: number | null;
  uptime: string | null;
}

export interface ServiceDiskUsage {
  totalBytes: number;
  installationBytes: number;
  dataBytes: number;
  logsBytes: number;
  configBytes: number;
  cacheBytes: number;
  backupBytes: number;
  otherBytes: number;
}

export interface CacheCleanupResult {
  removedItems: number;
  freedBytes: number;
}

export interface ServiceBackup {
  id: string;
  createdAtMillis: number;
  sizeBytes: number;
  automatic: boolean;
}

export interface RestoreResult {
  safetyBackup: ServiceBackup;
}

export interface RedisOverview {
  version: string;
  usedMemoryBytes: number;
  connectedClients: number;
  operationsPerSecond: number;
  totalKeys: number;
  hitRatePercent: number;
}

export interface RedisVersionInfo {
  series: string;
  version: string;
  installed: boolean;
  selected: boolean;
  supportLabel: string;
  legacy: boolean;
  recommended: boolean;
}

export interface MysqlVersionInfo {
  series: string;
  version: string;
  installed: boolean;
  selected: boolean;
  supportLabel: string;
  legacy: boolean;
  recommended: boolean;
}

export interface PostgresVersionInfo {
  series: string;
  version: string;
  installed: boolean;
  selected: boolean;
  supportLabel: string;
  legacy: boolean;
  recommended: boolean;
}

export interface RedisScanResult {
  nextCursor: string;
  keys: string[];
}

export interface RedisKeyDetail {
  key: string;
  keyType: string;
  ttlSeconds: number;
  memoryBytes: number | null;
  value: unknown;
  truncated: boolean;
}

export interface RedisCommandResult {
  output: string;
  elapsedMs: number;
}

export interface DatabaseOverview {
  version: string;
  databaseCount: number;
  tableCount: number;
  connectionCount: number;
  dataSizeBytes: number;
}

export interface DatabaseInfo {
  name: string;
  sizeBytes: number;
  system: boolean;
}

export interface TableInfo {
  schema: string;
  name: string;
  rowCount: number;
  sizeBytes: number;
}

export interface ColumnInfo {
  name: string;
  dataType: string;
  nullable: boolean;
  key: string;
  defaultValue: string | null;
}

export interface SqlResult {
  columns: string[];
  rows: Array<Array<string | null>>;
  summary: string;
  elapsedMs: number;
  truncated: boolean;
}

export interface TableDetail {
  columns: ColumnInfo[];
  preview: SqlResult;
}

export interface MongoOverview {
  version: string;
  databaseCount: number;
  connectionCount: number;
  dataSizeBytes: number;
  uptimeSeconds: number;
}

export interface MongoDatabaseInfo {
  name: string;
  sizeBytes: number;
  system: boolean;
}

export interface MongoCollectionInfo {
  name: string;
  collectionType: string;
}

export interface MongoFieldInfo {
  name: string;
  bsonType: string;
  occurrences: number;
}

export interface MongoCollectionDetail {
  documentCount: number;
  sizeBytes: number;
  fields: MongoFieldInfo[];
  documents: unknown[];
  truncated: boolean;
}

export interface MongoCommandResult {
  output: unknown;
  elapsedMs: number;
}

export interface MailpitOverview {
  total: number;
  unread: number;
  smtpAddress: string;
  webAddress: string;
}

export interface MailSummary {
  id: string;
  from: string;
  to: string[];
  subject: string;
  created: string;
  sizeBytes: number;
  read: boolean;
  snippet: string;
  attachmentCount: number;
}

export interface MailHeader {
  name: string;
  value: string;
}

export interface MailDetail {
  id: string;
  from: string;
  to: string[];
  cc: string[];
  subject: string;
  created: string;
  text: string;
  html: string;
  headers: MailHeader[];
  attachmentCount: number;
}

export interface PortListener {
  port: number;
  address: string;
  pid: number;
  process: string;
  managedService: string | null;
  commonService: string | null;
}

export interface DuckdbStatus {
  installed: boolean;
  version: string;
  executablePath: string;
  installationBytes: number;
}

export interface DuckdbQueryResult {
  columns: string[];
  rows: Array<Array<string | null>>;
  elapsedMs: number;
  truncated: boolean;
  summary: string;
}
