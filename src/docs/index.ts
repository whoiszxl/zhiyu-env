import type { ServiceKind } from "../types";
import type { DocChapter } from "./docTypes";
import { buildRedisDocs } from "./redisDocs";
import { buildMysqlDocs } from "./mysqlDocs";
import { buildPostgresDocs } from "./postgresDocs";
import { buildMongodbDocs } from "./mongodbDocs";
import { buildMailpitDocs } from "./mailpitDocs";

export type { DocBlock, DocChapter, DocCodeSample } from "./docTypes";

const BUILDERS: Record<ServiceKind, (port: number) => DocChapter[]> = {
  redis: buildRedisDocs,
  mysql: buildMysqlDocs,
  postgres: buildPostgresDocs,
  mongodb: buildMongodbDocs,
  mailpit: buildMailpitDocs,
};

/** 文档标题栏上展示的一句话定位。 */
const TAGLINES: Record<ServiceKind, string> = {
  redis: "内存键值数据库 · 缓存与高频读写",
  mysql: "关系型数据库 · 事务与结构化数据",
  postgres: "关系型数据库 · 复杂查询与丰富类型",
  mongodb: "文档数据库 · 灵活结构与快速迭代",
  mailpit: "本地邮件沙箱 · 开发期收信调试",
};

export function buildServiceDocs(kind: ServiceKind, port: number): DocChapter[] {
  return BUILDERS[kind](port);
}

export function serviceDocTagline(kind: ServiceKind): string {
  return TAGLINES[kind];
}
