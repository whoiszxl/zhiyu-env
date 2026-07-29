export type ServiceKind =
  | "redis"
  | "mysql"
  | "postgres"
  | "mongodb"
  | "mailpit"
  | "nats"
  | "kafka"
  | "meilisearch"
  | "minio"
  | "rustfs"
  | "etcd"
  | "consul"
  | "rnacos"
  | "rabbitmq"
  | "nginx"
  | "caddy";
export type SqlServiceKind = "mysql" | "postgres";

export type ServiceState =
  | "not_installed"
  | "stopped"
  | "running"
  | "stale_pid"
  | "crashed";

export interface ServiceInfo {
  kind: ServiceKind;
  name: string;
  version: string;
  port: number;
  status: ServiceState;
  pid: number | null;
  installSupported: boolean;
  installSupportLabel: string;
  platformLabel: string;
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

export interface EnvironmentMetrics {
  cpuPercent: number;
  memoryBytes: number;
  runningServiceCount: number;
}

export type DiagnosticStatus = "passed" | "warning" | "error";

export interface DiagnosticItem {
  id: string;
  scope: string;
  title: string;
  status: DiagnosticStatus;
  message: string;
  detail: string | null;
  repairable: boolean;
}

export interface DiagnosticReport {
  generatedAtMillis: number;
  summary: {
    passed: number;
    warnings: number;
    errors: number;
    repairable: number;
  };
  items: DiagnosticItem[];
}

export interface DiagnosticRepairResult {
  repairedCount: number;
  messages: string[];
  report: DiagnosticReport;
}

export type ThemeMode = "system" | "light" | "dark";
export type ColorTheme =
  | "classic"
  | "ocean"
  | "forest"
  | "sand"
  | "twilight"
  | "aurora"
  | "graphite"
  | "coral"
  | "sunset"
  | "neon";
export type BackgroundPattern = "auto" | "none" | "grid" | "dots" | "diagonal";
export type UiScale = 90 | 100 | 110 | 120;
export type BackgroundStyle = "off" | "original" | "frosted" | "blur" | "mist";
export type BackgroundPosition = "center" | "top" | "bottom";

export interface AppSettings {
  themeMode: ThemeMode;
  colorTheme: ColorTheme;
  backgroundPattern: BackgroundPattern;
  uiScale: UiScale;
  backgroundImagePath: string;
  backgroundStyle: BackgroundStyle;
  backgroundPosition: BackgroundPosition;
  backgroundOverlay: number;
  launchAtLogin: boolean;
  keepServicesRunningOnClose: boolean;
  downloadMirror: string;
  publicGithubMirror: boolean;
  downloadConcurrency: number;
  downloadTimeoutSeconds: number;
  installRoot: string;
  logRetentionDays: number;
  backupRetentionCount: number;
  autoCheckUpdates: boolean;
  onboardingCompleted: boolean;
}

export interface UpdateStatus {
  currentVersion: string;
  latestVersion: string | null;
  updateAvailable: boolean;
  releaseUrl: string | null;
  message: string;
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
  installationBytes: number;
}

export interface MysqlVersionInfo {
  series: string;
  version: string;
  installed: boolean;
  selected: boolean;
  supportLabel: string;
  legacy: boolean;
  recommended: boolean;
  installationBytes: number;
}

export interface PostgresVersionInfo {
  series: string;
  version: string;
  installed: boolean;
  selected: boolean;
  supportLabel: string;
  legacy: boolean;
  recommended: boolean;
  installationBytes: number;
}

export interface NginxVersionInfo {
  series: string;
  version: string;
  installed: boolean;
  selected: boolean;
  supportLabel: string;
  legacy: boolean;
  recommended: boolean;
  installationBytes: number;
}

export interface ManagedServiceVersionInfo {
  series: string;
  version: string;
  installed: boolean;
  selected: boolean;
  supportLabel: string;
  legacy: boolean;
  recommended: boolean;
  installationBytes: number;
}

export interface VersionUninstallResult {
  kind: ServiceKind;
  version: string;
  freedBytes: number;
  fallbackVersion: string | null;
  dataPreserved: boolean;
  service: ServiceInfo;
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

export interface SqliteOverview {
  version: string;
  fileSizeBytes: number;
  tableCount: number;
  indexCount: number;
  journalMode: string;
}

export interface SqliteTable {
  name: string;
  tableType: "table" | "view";
}

export interface SqliteQueryResult {
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

export interface NatsOverview {
  version: string;
  connections: number;
  subscriptions: number;
  inMessages: number;
  outMessages: number;
  inBytes: number;
  outBytes: number;
  slowConsumers: number;
}

export interface NatsPublishResult {
  subject: string;
  payloadBytes: number;
  elapsedMs: number;
}

export interface NatsMessage {
  subject: string;
  payload: string;
  payloadBytes: number;
  elapsedMs: number;
}

export interface KafkaOverview {
  version: string;
  broker: string;
  topicCount: number;
  storageEngine: string;
}

export interface KafkaTopic {
  name: string;
}

export interface KafkaPublishResult {
  topic: string;
  payloadBytes: number;
  elapsedMs: number;
}

export interface MeilisearchOverview {
  version: string;
  indexCount: number;
  documentCount: number;
  databaseSizeBytes: number;
  usedDatabaseSizeBytes: number;
  indexingCount: number;
}

export interface MeilisearchIndex {
  uid: string;
  documentCount: number;
  indexing: boolean;
}

export interface MeilisearchTask {
  taskUid: number;
  status: string;
  indexUid: string | null;
}

export interface MeilisearchSearchResult {
  hits: unknown[];
  estimatedTotalHits: number;
  processingTimeMs: number;
  query: string;
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

// ── 内置工具：JSON / YAML / TOML 工具箱 ──────────────────────

export type DataFormat = "json" | "yaml" | "toml" | "auto";
export type OutputStyle = "pretty" | "compact";

export interface TransformResult {
  output: string;
  detectedFormat: string;
  warnings: string[];
  inputBytes: number;
  outputBytes: number;
}

export type CsvDirection = "csvToJson" | "jsonToCsv";
export type CsvDelimiter = "comma" | "tab" | "semicolon" | "pipe";

export interface CsvTransformResult {
  output: string;
  rowCount: number;
  columnCount: number;
  inputBytes: number;
  outputBytes: number;
}

export type JsonDiffKind = "added" | "removed" | "changed";

export interface JsonDiffEntry {
  path: string;
  kind: JsonDiffKind;
  left: string | null;
  right: string | null;
}

export interface JsonDiffResult {
  entries: JsonDiffEntry[];
  added: number;
  removed: number;
  changed: number;
  identical: boolean;
}

export interface JsonPathResult {
  matches: string[];
  count: number;
}

// ── 内置工具：JWT 调试器 ────────────────────────────────────

export type SecretEncoding = "utf8" | "base64";
export type HmacAlgorithm = "HS256" | "HS384" | "HS512";
export type TokenStatus = "active" | "expired" | "notYetValid" | "noTimeLimit";

export interface JwtTimeClaim {
  name: string;
  label: string;
  description: string;
  value: number;
  offsetSeconds: number;
}

export interface JwtRegisteredClaim {
  name: string;
  label: string;
  value: string;
}

export interface JwtDecoded {
  header: string;
  payload: string;
  signature: string;
  algorithm: string;
  tokenType: string | null;
  keyId: string | null;
  timeClaims: JwtTimeClaim[];
  registeredClaims: JwtRegisteredClaim[];
  status: TokenStatus;
  statusDetail: string;
  warnings: string[];
}

export interface JwtVerifyResult {
  valid: boolean;
  algorithm: string;
  detail: string;
}

export interface JwkKeyInfo {
  keyId: string | null;
  keyType: string;
  algorithm: string | null;
  usage: string | null;
  summary: string;
  containsPrivateMaterial: boolean;
}

export interface JwkInspection {
  keys: JwkKeyInfo[];
  count: number;
  source: string;
  warnings: string[];
}

// ── 剪贴板历史 ────────────────────────────────────────────

export interface ClipboardItem {
  id: number;
  content: string;
  contentType: "text" | "url" | "code";
  preview: string;
  charCount: number;
  copiedAtMillis: number;
  lastUsedAtMillis: number;
  useCount: number;
  pinned: boolean;
}

export interface ClipboardStatus {
  itemCount: number;
  pinnedCount: number;
  dbSizeBytes: number;
  runState: "stopped" | "running" | "paused";
}

export interface ClipboardSettings {
  maxItems: number;
  retentionDays: number;
  autoStartMonitoring: boolean;
}

// ── S3 对象存储浏览器 ──────────────────────────────────────

export interface S3Config {
  endpoint: string;
  accessKey: string;
  secretKey: string;
  region: string;
  bucket: string;
  pathStyle: boolean;
}

export interface S3Bucket {
  name: string;
  creationDate: string;
}

export interface S3Object {
  key: string;
  size: number;
  lastModified: string;
  etag: string;
}

export interface S3ListResult {
  folders: string[];
  objects: S3Object[];
  nextContinuationToken: string | null;
  truncated: boolean;
}

export interface S3ObjectContent {
  contentType: string;
  data: string;
  size: number;
}

export interface S3PresignedUrl {
  url: string;
}

// ── 本地 Mock API ─────────────────────────────────────────

export interface MockRoute {
  id: string;
  method: string;
  path: string;
  statusCode: number;
  contentType: string;
  responseBody: string;
  delayMs: number;
  enabled: boolean;
}

export interface MockRequestLog {
  id: number;
  timestampMillis: number;
  method: string;
  path: string;
  statusCode: number;
  matchedRouteId: string | null;
  bodyPreview: string;
}

export interface MockApiState {
  running: boolean;
  port: number;
  baseUrl: string;
  routes: MockRoute[];
  recentRequests: MockRequestLog[];
}

// ── HTTP 请求调试器 ───────────────────────────────────────

export interface HttpHeader {
  name: string;
  value: string;
}

export interface HttpRequestInput {
  method: string;
  url: string;
  headers: HttpHeader[];
  body: string;
  timeoutSeconds: number;
  followRedirects: boolean;
}

export interface HttpResponseOutput {
  statusCode: number;
  statusText: string;
  headers: HttpHeader[];
  body: string;
  contentType: string;
  elapsedMs: number;
  sizeBytes: number;
  truncated: boolean;
  effectiveUrl: string;
}

export interface QrCodeResult {
  svg: string;
  modules: number;
  contentBytes: number;
}

// ── SSH 远程连接 ──────────────────────────────────────────

export interface SshProfile {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  identityFile: string;
  authMethod: "key" | "password";
  createdAtMillis: number;
  updatedAtMillis: number;
}

export interface SshHostKey {
  host: string;
  keyType: string;
  fingerprint: string;
}

export interface SshCommandResult {
  success: boolean;
  exitCode: number | null;
  stdout: string;
  stderr: string;
  elapsedMillis: number;
  truncated: boolean;
  timedOut: boolean;
}

export interface SshTerminalConnection {
  sessionId: string;
}

export interface SshTerminalEvent {
  sessionId: string;
  event: "data" | "closed" | "error";
  data: string;
}
