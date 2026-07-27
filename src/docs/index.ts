import type { ServiceKind } from "../types";
import type { DocChapter } from "./docTypes";
import { buildRedisDocs } from "./redisDocs";
import { buildMysqlDocs } from "./mysqlDocs";
import { buildPostgresDocs } from "./postgresDocs";
import { buildMongodbDocs } from "./mongodbDocs";
import { buildMailpitDocs } from "./mailpitDocs";
import { buildNatsDocs } from "./natsDocs";
import { buildMeilisearchDocs } from "./meilisearchDocs";
import { buildMinioDocs } from "./minioDocs";
import { buildRustfsDocs } from "./rustfsDocs";
import { buildEtcdDocs } from "./etcdDocs";
import { buildConsulDocs } from "./consulDocs";
import { buildRnacosDocs } from "./rnacosDocs";

export type { DocBlock, DocChapter, DocCodeSample } from "./docTypes";

const BUILDERS: Record<ServiceKind, (port: number) => DocChapter[]> = {
  redis: buildRedisDocs,
  mysql: buildMysqlDocs,
  postgres: buildPostgresDocs,
  mongodb: buildMongodbDocs,
  mailpit: buildMailpitDocs,
  nats: buildNatsDocs,
  meilisearch: buildMeilisearchDocs,
  minio: buildMinioDocs,
  rustfs: buildRustfsDocs,
  etcd: buildEtcdDocs,
  consul: buildConsulDocs,
  rnacos: buildRnacosDocs,
};

/** 文档标题栏上展示的一句话定位。 */
const TAGLINES: Record<ServiceKind, string> = {
  redis: "内存键值数据库 · 缓存与高频读写",
  mysql: "关系型数据库 · 事务与结构化数据",
  postgres: "关系型数据库 · 复杂查询与丰富类型",
  mongodb: "文档数据库 · 灵活结构与快速迭代",
  mailpit: "本地邮件沙箱 · 开发期收信调试",
  nats: "轻量消息服务器 · 发布订阅与 JetStream",
  meilisearch: "全文搜索引擎 · 索引与搜索调试",
  minio: "S3 兼容对象存储 · 存量项目调试",
  rustfs: "Rust 对象存储 · S3 兼容开发调试",
  etcd: "分布式键值存储 · 配置与服务协调",
  consul: "服务发现与配置 · 内置 Web UI",
  rnacos: "Nacos 兼容服务 · 无需 Java Runtime",
};

export function buildServiceDocs(kind: ServiceKind, port: number): DocChapter[] {
  return BUILDERS[kind](port);
}

export function serviceDocTagline(kind: ServiceKind): string {
  return TAGLINES[kind];
}
