import type { AppLocale, ServiceKind } from "../types";
import type { DocChapter } from "./docTypes";

import { buildRedisDocs as buildRedisDocsZh } from "./redis/zh-CN";
import { buildMysqlDocs as buildMysqlDocsZh } from "./mysql/zh-CN";
import { buildPostgresDocs as buildPostgresDocsZh } from "./postgres/zh-CN";
import { buildMongodbDocs as buildMongodbDocsZh } from "./mongodb/zh-CN";
import { buildMailpitDocs as buildMailpitDocsZh } from "./mailpit/zh-CN";
import { buildNatsDocs as buildNatsDocsZh } from "./nats/zh-CN";
import { buildKafkaDocs as buildKafkaDocsZh } from "./kafka/zh-CN";
import { buildMeilisearchDocs as buildMeilisearchDocsZh } from "./meilisearch/zh-CN";
import { buildInfluxdbDocs as buildInfluxdbDocsZh } from "./influxdb/zh-CN";
import { buildMinioDocs as buildMinioDocsZh } from "./minio/zh-CN";
import { buildRustfsDocs as buildRustfsDocsZh } from "./rustfs/zh-CN";
import { buildEtcdDocs as buildEtcdDocsZh } from "./etcd/zh-CN";
import { buildConsulDocs as buildConsulDocsZh } from "./consul/zh-CN";
import { buildRnacosDocs as buildRnacosDocsZh } from "./rnacos/zh-CN";
import { buildRabbitmqDocs as buildRabbitmqDocsZh } from "./rabbitmq/zh-CN";
import { buildActivemqDocs as buildActivemqDocsZh } from "./activemq/zh-CN";
import { buildNginxDocs as buildNginxDocsZh } from "./nginx/zh-CN";
import { buildCaddyDocs as buildCaddyDocsZh } from "./caddy/zh-CN";
import { buildFtpDocs as buildFtpDocsZh } from "./ftp/zh-CN";

import { buildRedisDocs as buildRedisDocsEn } from "./redis/en-US";
import { buildMysqlDocs as buildMysqlDocsEn } from "./mysql/en-US";
import { buildPostgresDocs as buildPostgresDocsEn } from "./postgres/en-US";
import { buildMongodbDocs as buildMongodbDocsEn } from "./mongodb/en-US";
import { buildMailpitDocs as buildMailpitDocsEn } from "./mailpit/en-US";
import { buildNatsDocs as buildNatsDocsEn } from "./nats/en-US";
import { buildKafkaDocs as buildKafkaDocsEn } from "./kafka/en-US";
import { buildMeilisearchDocs as buildMeilisearchDocsEn } from "./meilisearch/en-US";
import { buildInfluxdbDocs as buildInfluxdbDocsEn } from "./influxdb/en-US";
import { buildMinioDocs as buildMinioDocsEn } from "./minio/en-US";
import { buildRustfsDocs as buildRustfsDocsEn } from "./rustfs/en-US";
import { buildEtcdDocs as buildEtcdDocsEn } from "./etcd/en-US";
import { buildConsulDocs as buildConsulDocsEn } from "./consul/en-US";
import { buildRnacosDocs as buildRnacosDocsEn } from "./rnacos/en-US";
import { buildRabbitmqDocs as buildRabbitmqDocsEn } from "./rabbitmq/en-US";
import { buildActivemqDocs as buildActivemqDocsEn } from "./activemq/en-US";
import { buildNginxDocs as buildNginxDocsEn } from "./nginx/en-US";
import { buildCaddyDocs as buildCaddyDocsEn } from "./caddy/en-US";
import { buildFtpDocs as buildFtpDocsEn } from "./ftp/en-US";

export type { DocBlock, DocChapter, DocCodeSample } from "./docTypes";

/** 文档实际渲染的两种语言，`system` 由上层解析后传入这里。 */
export type DocLocale = "zh-CN" | "en-US";

type DocBuilder = (port: number) => DocChapter[];

const BUILDERS: Record<DocLocale, Record<ServiceKind, DocBuilder>> = {
  "zh-CN": {
    redis: buildRedisDocsZh,
    mysql: buildMysqlDocsZh,
    postgres: buildPostgresDocsZh,
    mongodb: buildMongodbDocsZh,
    mailpit: buildMailpitDocsZh,
    nats: buildNatsDocsZh,
    kafka: buildKafkaDocsZh,
    meilisearch: buildMeilisearchDocsZh,
    influxdb: buildInfluxdbDocsZh,
    minio: buildMinioDocsZh,
    rustfs: buildRustfsDocsZh,
    etcd: buildEtcdDocsZh,
    consul: buildConsulDocsZh,
    rnacos: buildRnacosDocsZh,
    rabbitmq: buildRabbitmqDocsZh,
    activemq: buildActivemqDocsZh,
    nginx: buildNginxDocsZh,
    caddy: buildCaddyDocsZh,
    ftp: buildFtpDocsZh,
  },
  "en-US": {
    redis: buildRedisDocsEn,
    mysql: buildMysqlDocsEn,
    postgres: buildPostgresDocsEn,
    mongodb: buildMongodbDocsEn,
    mailpit: buildMailpitDocsEn,
    nats: buildNatsDocsEn,
    kafka: buildKafkaDocsEn,
    meilisearch: buildMeilisearchDocsEn,
    influxdb: buildInfluxdbDocsEn,
    minio: buildMinioDocsEn,
    rustfs: buildRustfsDocsEn,
    etcd: buildEtcdDocsEn,
    consul: buildConsulDocsEn,
    rnacos: buildRnacosDocsEn,
    rabbitmq: buildRabbitmqDocsEn,
    activemq: buildActivemqDocsEn,
    nginx: buildNginxDocsEn,
    caddy: buildCaddyDocsEn,
    ftp: buildFtpDocsEn,
  },
};

/** 每个服务在文档标题栏展示的一句话定位。 */
const TAGLINES: Record<DocLocale, Record<ServiceKind, string>> = {
  "zh-CN": {
    redis: "内存键值数据库 · 缓存与高频读写",
    mysql: "关系型数据库 · 事务与结构化数据",
    postgres: "关系型数据库 · 复杂查询与丰富类型",
    mongodb: "文档数据库 · 灵活结构与快速迭代",
    mailpit: "本地邮件沙箱 · 开发期收信调试",
    nats: "轻量消息服务器 · 发布订阅与 JetStream",
    kafka: "Kafka API 兼容消息沙箱 · 无需 JVM",
    meilisearch: "全文搜索引擎 · 索引与搜索调试",
    influxdb: "时序数据库 · 指标写入与 SQL 查询",
    minio: "S3 兼容对象存储 · 存量项目调试",
    rustfs: "Rust 对象存储 · S3 兼容开发调试",
    etcd: "分布式键值存储 · 配置与服务协调",
    consul: "服务发现与配置 · 内置 Web UI",
    rnacos: "Nacos 兼容服务 · 无需 Java Runtime",
    rabbitmq: "AMQP 消息代理 · 队列与交换机",
    activemq: "JMS 消息代理 · OpenWire、AMQP 与 STOMP",
    nginx: "轻量 Web 服务器 · 静态文件与反向代理",
    caddy: "现代化 Web 服务器 · Caddyfile 配置",
    ftp: "本地 FTP 文件传输 · 账号隔离与断点续传",
  },
  "en-US": {
    redis: "In-memory key-value store · Cache & high-frequency I/O",
    mysql: "Relational database · Transactions & structured data",
    postgres: "Relational database · Rich types & complex queries",
    mongodb: "Document database · Flexible schema, rapid iteration",
    mailpit: "Local mail sandbox · Capture dev-time email",
    nats: "Lightweight messaging · Pub/Sub & JetStream",
    kafka: "Kafka API-compatible sandbox · No JVM required",
    meilisearch: "Full-text search engine · Index & query debug",
    influxdb: "Time-series database · Metrics write & SQL query",
    minio: "S3-compatible object storage · Legacy project debug",
    rustfs: "Rust object storage · S3-compatible dev workflow",
    etcd: "Distributed key-value store · Config & coordination",
    consul: "Service discovery & config · Built-in web UI",
    rnacos: "Nacos-compatible service · No Java runtime",
    rabbitmq: "AMQP message broker · Queues & exchanges",
    activemq: "JMS message broker · OpenWire, AMQP & STOMP",
    nginx: "Lightweight web server · Static files & reverse proxy",
    caddy: "Modern web server · Caddyfile configuration",
    ftp: "Local FTP transfer · Account isolation & resume support",
  },
};

/** locale 归一化为文档层认识的两种值，`system` 在上层已经解析过。 */
function resolveDocLocale(locale: AppLocale | DocLocale): DocLocale {
  return locale === "en-US" ? "en-US" : "zh-CN";
}

export function buildServiceDocs(
  kind: ServiceKind,
  port: number,
  locale: AppLocale | DocLocale = "zh-CN",
): DocChapter[] {
  const resolved = resolveDocLocale(locale);
  return BUILDERS[resolved][kind](port);
}

export function serviceDocTagline(
  kind: ServiceKind,
  locale: AppLocale | DocLocale = "zh-CN",
): string {
  const resolved = resolveDocLocale(locale);
  return TAGLINES[resolved][kind];
}
