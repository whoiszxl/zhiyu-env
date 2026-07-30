export type ServiceKind =
  | "redis"
  | "mysql"
  | "postgres"
  | "mongodb"
  | "mailpit"
  | "nats"
  | "kafka"
  | "meilisearch"
  | "influxdb"
  | "minio"
  | "rustfs"
  | "etcd"
  | "consul"
  | "rnacos"
  | "rabbitmq"
  | "activemq"
  | "nginx"
  | "caddy"
  | "ftp";
export type SqlServiceKind = "mysql" | "postgres";
export type OlapEngine = "clickhouse" | "doris";

export interface OlapProfile {
  id: string;
  name: string;
  engine: OlapEngine;
  endpoint: string;
  username: string;
  database: string;
  hasPassword: boolean;
}

export interface OlapProfileInput {
  id?: string | null;
  name: string;
  engine: OlapEngine;
  endpoint: string;
  username: string;
  password?: string | null;
  database: string;
}

export interface OlapConnectionTest {
  version: string;
  elapsedMs: number;
}

export interface OlapDatabaseInfo {
  name: string;
  system: boolean;
}

export interface OlapTableInfo {
  name: string;
  engine: string;
  rows: number | null;
}

export interface OlapQueryResult {
  columns: string[];
  rows: Array<Array<string | null>>;
  summary: string;
  elapsedMs: number;
  truncated: boolean;
}

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

export interface InfluxdbOverview {
  ready: boolean;
  databaseCount: number;
  endpoint: string;
}

export interface InfluxdbDatabase {
  name: string;
}

export interface InfluxdbQueryResult {
  columns: string[];
  rows: unknown[][];
  rowCount: number;
  truncated: boolean;
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
export type AppLocale = "system" | "zh-CN" | "en-US";
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
  | "neon"
  | "nord"
  | "sakura"
  | "coffee"
  | "solarized"
  | "lavender";
export type BackgroundPattern =
  | "auto"
  | "none"
  | "grid"
  | "dots"
  | "diagonal"
  | "crosshatch"
  | "circuit"
  | "rings"
  | "paper"
  | "checker";
export type UiScale = 90 | 100 | 110 | 120;
export type BackgroundStyle = "off" | "original" | "frosted" | "blur" | "mist";
export type BackgroundPosition = "center" | "top" | "bottom";
export type ProxyMode = "system" | "manual" | "disabled";

export interface AppSettings {
  locale: AppLocale;
  themeMode: ThemeMode;
  colorTheme: ColorTheme;
  backgroundPattern: BackgroundPattern;
  uiScale: UiScale;
  backgroundImagePath: string;
  backgroundStyle: BackgroundStyle;
  backgroundPosition: BackgroundPosition;
  backgroundOverlay: number;
  hiddenServices: ServiceKind[];
  serviceOrder: ServiceKind[];
  hiddenTools: string[];
  toolOrder: string[];
  launchAtLogin: boolean;
  keepServicesRunningOnClose: boolean;
  resourceSaverEnabled: boolean;
  resourceSaverMode: "remind" | "stop";
  resourceSaverMinutes: number;
  resourceSaverServices: ServiceKind[];
  proxyMode: ProxyMode;
  proxyUrl: string;
  downloadProxyEnabled: boolean;
  networkProxyEnabled: boolean;
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

export type AiApiProtocol = "openai" | "anthropic";

export interface AiSettings {
  enabled: boolean;
  protocol: AiApiProtocol;
  baseUrl: string;
  model: string;
  timeoutSeconds: number;
  maxOutputTokens: number;
  apiKeyConfigured: boolean;
  userAvatarPath: string;
  assistantAvatarPath: string;
}

export interface AiSettingsInput {
  enabled: boolean;
  protocol: AiApiProtocol;
  baseUrl: string;
  model: string;
  timeoutSeconds: number;
  maxOutputTokens: number;
  userAvatarPath: string;
  assistantAvatarPath: string;
  apiKey: string;
  clearApiKey: boolean;
}

export interface AiConnectionTestResult {
  success: boolean;
  protocol: AiApiProtocol;
  model: string;
  latencyMillis: number;
  message: string;
}

export interface AiChatSession {
  id: string;
  title: string;
  preview: string;
  messageCount: number;
  createdAtMillis: number;
  updatedAtMillis: number;
}

export interface AiChatMessage {
  id: number;
  sessionId: string;
  role: "user" | "assistant";
  content: string;
  createdAtMillis: number;
}

export interface AiChatStreamEvent {
  sessionId: string;
  requestId: string;
  event: "delta" | "done" | "cancelled" | "error";
  content: string;
}

export type AiToolCapability =
  | "database_sql"
  | "database_explain"
  | "database_error"
  | "redis_command"
  | "redis_analysis"
  | "mock_api"
  | "mongodb_command"
  | "mongodb_analysis"
  | "message_design"
  | "service_logs"
  | "web_config"
  | "http_request"
  | "cron"
  | "regex"
  | "ssh";

export interface AiToolStreamEvent {
  requestId: string;
  event: "delta" | "done" | "cancelled" | "error";
  content: string;
}

export interface AiAssistOption {
  id: AiToolCapability;
  label: string;
  hint: string;
  canApply?: boolean;
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

export interface NetworkDiagnosticInput {
  target: string;
  mode: "auto" | "tcp" | "http" | "https";
  port: number | null;
  timeoutSeconds: number;
}

export interface NetworkResolvedAddress {
  address: string;
  family: "IPv4" | "IPv6";
  local: boolean;
}

export interface NetworkTcpAttempt {
  address: string;
  connected: boolean;
  elapsedMillis: number;
  error: string;
}

export interface NetworkHttpProbe {
  url: string;
  statusCode: number;
  statusText: string;
  elapsedMillis: number;
  effectiveUrl: string;
  server: string;
  contentType: string;
  contentLength: number | null;
}

export interface NetworkTlsProbe {
  success: boolean;
  elapsedMillis: number;
  protocol: string;
  cipherSuite: string;
  alpn: string;
  certificateCount: number;
  sha256Fingerprint: string;
  error: string;
}

export interface NetworkProxySetting {
  source: string;
  name: string;
  value: string;
}

export interface NetworkFinding {
  level: "success" | "warning" | "error";
  code: "dns_slow" | "tcp_failed" | "tcp_partial" | "local_no_listener" | "http_error" | "http_slow" | "tls_failed" | "healthy";
  detail: string;
}

export interface NetworkDiagnosticResult {
  target: string;
  host: string;
  port: number;
  mode: "tcp" | "http" | "https";
  dnsMillis: number;
  addresses: NetworkResolvedAddress[];
  tcpAttempts: NetworkTcpAttempt[];
  http: NetworkHttpProbe | null;
  tls: NetworkTlsProbe | null;
  portOwner: PortListener | null;
  proxies: NetworkProxySetting[];
  findings: NetworkFinding[];
}

export interface ZeroMqResult {
  endpoint: string;
  pattern: "PUB/SUB" | "PUSH/PULL";
  direction: "sent" | "received";
  frames: string[];
  bytes: number;
  timestampMillis: number;
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

export interface HttpWorkspaceVariable {
  key: string;
  value: string;
  secret: boolean;
  enabled: boolean;
}

export interface HttpWorkspaceEnvironment {
  id: string;
  name: string;
  variables: HttpWorkspaceVariable[];
}

export interface HttpWorkspaceAuth {
  kind: "none" | "basic" | "bearer" | "apiKey";
  username: string;
  password: string;
  token: string;
  key: string;
  value: string;
  placement: "header" | "query";
}

export interface HttpWorkspaceRequest {
  id: string;
  name: string;
  folder: string;
  method: string;
  url: string;
  queryParams: HttpHeader[];
  headers: HttpHeader[];
  body: string;
  auth: HttpWorkspaceAuth;
  updatedAt: number;
}

export interface HttpWorkspaceState {
  version: number;
  activeEnvironmentId: string;
  environments: HttpWorkspaceEnvironment[];
  requests: HttpWorkspaceRequest[];
}

export interface LocalDomainRoute {
  id: string;
  name: string;
  hostname: string;
  target: string;
  path: string;
  https: boolean;
  enabled: boolean;
}

export interface LocalDomainsState {
  version: number;
  httpPort: number;
  httpsPort: number;
  routes: LocalDomainRoute[];
  lastBackupPath: string;
  lastAppliedAtMillis: number;
}

export interface LocalDomainCheck {
  reachable: boolean;
  latencyMillis: number;
  message: string;
}

export interface TestDataExportField {
  name: string;
  kind: string;
  options: string;
  nullablePercent: number;
  unique: boolean;
  prefix: string;
  suffix: string;
}

export interface TestDataExportInput {
  seed: string;
  count: number;
  format: "json" | "csv" | "sql";
  tableName: string;
  fields: TestDataExportField[];
  path: string;
}

export interface TestDataExportResult {
  path: string;
  rows: number;
  bytes: number;
}

export interface QrCodeResult {
  svg: string;
  modules: number;
  contentBytes: number;
  version: number;
}

export interface QrCodeOptions {
  content: string;
  errorCorrection: "L" | "M" | "Q" | "H";
  size: number;
  foreground: string;
  background: string;
  quietZone: boolean;
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

// ── RSS 订阅 ──────────────────────────────────────────────

export interface RssFeed {
  id: number;
  title: string;
  feedUrl: string;
  siteUrl: string | null;
  description: string | null;
  refreshIntervalMinutes: number;
  enabled: boolean;
  unreadCount: number;
  entryCount: number;
  lastRefreshedAtMillis: number | null;
  lastError: string | null;
}

export interface RssEntry {
  id: number;
  feedId: number;
  feedTitle: string;
  title: string;
  link: string | null;
  author: string | null;
  summary: string;
  content: string;
  contentHtml: string;
  publishedAtMillis: number | null;
  fetchedAtMillis: number;
  isRead: boolean;
  isStarred: boolean;
}

export interface RssRefreshResult {
  feedId: number;
  title: string;
  added: number;
  updated: number;
  notModified: boolean;
}

export interface RssImportResult {
  imported: number;
  skipped: number;
}

export interface RssFeedUpdate {
  title: string;
  refreshIntervalMinutes: number;
  enabled: boolean;
}

export type RssAiAction =
  | "summary"
  | "translate"
  | "key_points"
  | "question";

export interface RssAiResult {
  id: number;
  entryId: number;
  action: RssAiAction;
  question: string;
  outputLanguage: string;
  model: string;
  content: string;
  status: "complete" | "partial";
  createdAtMillis: number;
  updatedAtMillis: number;
}

export interface RssAiStreamEvent {
  entryId: number;
  requestId: string;
  action: RssAiAction;
  event: "delta" | "done" | "cancelled" | "error";
  content: string;
}

// ── 开发运行时 ────────────────────────────────────────────

export type RuntimeKind = "go" | "java" | "rust" | "python" | "node";

export interface RuntimeEnvironmentVariable {
  key: string;
  value: string;
}

export interface RuntimeVersionInfo {
  kind: RuntimeKind;
  series: string;
  version: string;
  supportLabel: string;
  legacy: boolean;
  recommended: boolean;
  installed: boolean;
  selected: boolean;
  compatible: boolean;
  platformLabel: string;
  installationPath: string;
  executablePath: string;
  diskBytes: number;
}

export interface RuntimeOverview {
  kind: RuntimeKind;
  name: string;
  selectedVersion: string | null;
  installedCount: number;
  totalDiskBytes: number;
  platformLabel: string;
  compatible: boolean;
  versions: RuntimeVersionInfo[];
  environment: RuntimeEnvironmentVariable[];
  goProxy: string;
}

export interface RuntimeDiagnostic {
  success: boolean;
  version: string;
  executable: string;
  output: string;
  environment: RuntimeEnvironmentVariable[];
}

export interface RuntimeProject {
  id: string;
  name: string;
  path: string;
  description: string;
  services: ServiceKind[];
  goVersion: string | null;
  javaVersion: string | null;
  rustVersion: string | null;
  pythonVersion: string | null;
  nodeVersion: string | null;
  createdAtMillis: number;
  updatedAtMillis: number;
}

export type ScheduledTaskKind = "cron" | "interval";
export type ScheduledTaskRunStatus =
  | "success"
  | "failed"
  | "timed_out"
  | "cancelled";

export interface ScheduledTask {
  id: number;
  name: string;
  scheduleKind: ScheduledTaskKind;
  cronExpression: string;
  intervalMinutes: number;
  command: string;
  workingDirectory: string;
  timeoutSeconds: number;
  enabled: boolean;
  running: boolean;
  nextRunAtMillis: number | null;
  lastRunAtMillis: number | null;
  lastStatus: ScheduledTaskRunStatus | null;
  runCount: number;
  createdAtMillis: number;
  updatedAtMillis: number;
}

export interface ScheduledTaskInput {
  id?: number | null;
  name: string;
  scheduleKind: ScheduledTaskKind;
  cronExpression: string;
  intervalMinutes: number;
  command: string;
  workingDirectory: string;
  timeoutSeconds: number;
  enabled: boolean;
}

export interface ScheduledTaskRun {
  id: number;
  taskId: number;
  startedAtMillis: number;
  finishedAtMillis: number;
  durationMillis: number;
  status: ScheduledTaskRunStatus;
  exitCode: number | null;
  output: string;
  trigger: "manual" | "scheduled";
}
