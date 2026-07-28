/** 每个服务的连接信息，用于「连接」标签页展示。 */
import type { ServiceKind } from "../types";

export interface UriEntry {
  label: string;
  value: string;
}

export interface EnvVar {
  key: string;
  value: string;
}

export interface ConfigSample {
  label: string;
  lang: string;
  caption: string;
  code: string;
}

export interface ServiceConnection {
  name: string;
  description: string;
  host: string;
  primaryPort: number;
  hasAuth: boolean;
  username: string;
  password: string;
  uris: UriEntry[];
  extras: { label: string; value: string }[];
  envVars: EnvVar[];
  configSamples: ConfigSample[];
}

function r(
  name: string,
  description: string,
  primaryPort: number,
  hasAuth: boolean,
  username: string,
  password: string,
  uris: UriEntry[],
  extras: { label: string; value: string }[],
  envVars: EnvVar[],
  configSamples: ConfigSample[],
): ServiceConnection {
  return {
    name,
    description,
    host: "127.0.0.1",
    primaryPort,
    hasAuth,
    username,
    password,
    uris,
    extras,
    envVars,
    configSamples,
  };
}

export function buildConnection(kind: ServiceKind): ServiceConnection {
  switch (kind) {
    case "redis":
      return r(
        "Redis",
        "内存键值数据库",
        6379,
        false, "", "",
        [
          { label: "连接串", value: "redis://127.0.0.1:6379/0" },
        ],
        [],
        [
          { key: "REDIS_URL", value: "redis://127.0.0.1:6379/0" },
        ],
        [
          {
            label: "Java Spring",
            lang: "yaml", caption: "application.yml",
            code: `spring:
  data:
    redis:
      host: 127.0.0.1
      port: 6379
      database: 0
      timeout: 2s
      lettuce:
        pool:
          max-active: 16`,
          },
          {
            label: "Go",
            lang: "go", caption: "go-redis",
            code: `import "github.com/redis/go-redis/v9"
rdb := redis.NewClient(&redis.Options{
    Addr: "127.0.0.1:6379",
    DB:   0,
})`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "ioredis",
            code: `import Redis from "ioredis";
const redis = new Redis({ host: "127.0.0.1", port: 6379, db: 0 });`,
          },
          {
            label: "Python",
            lang: "python", caption: "redis-py",
            code: `import redis
r = redis.Redis(host="127.0.0.1", port=6379, db=0, decode_responses=True)`,
          },
        ],
      );
    case "mysql":
      return r(
        "MySQL",
        "关系型数据库",
        3306,
        false, "root", "",
        [
          { label: "JDBC", value: "jdbc:mysql://127.0.0.1:3306/your_db?useUnicode=true&characterEncoding=utf8mb4" },
          { label: "连接串", value: "mysql://root@127.0.0.1:3306/your_db" },
        ],
        [],
        [
          { key: "DB_HOST", value: "127.0.0.1" },
          { key: "DB_PORT", value: "3306" },
          { key: "DB_USER", value: "root" },
          { key: "DB_PASSWORD", value: "" },
          { key: "DATABASE_URL", value: "mysql://root@127.0.0.1:3306/your_db" },
        ],
        [
          {
            label: "Java Spring",
            lang: "yaml", caption: "application.yml",
            code: `spring:
  datasource:
    url: jdbc:mysql://127.0.0.1:3306/demo?useUnicode=true&characterEncoding=utf8mb4
    username: root
    password:`,
          },
          {
            label: "Go",
            lang: "go", caption: "database/sql",
            code: `import "database/sql"
import _ "github.com/go-sql-driver/mysql"
db, _ := sql.Open("mysql", "root:@tcp(127.0.0.1:3306)/demo?charset=utf8mb4&parseTime=true")`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "mysql2",
            code: `import mysql from "mysql2/promise";
const pool = mysql.createPool({ host: "127.0.0.1", port: 3306, user: "root", database: "demo" });`,
          },
          {
            label: "Python",
            lang: "python", caption: "pymysql",
            code: `import pymysql
conn = pymysql.connect(host="127.0.0.1", port=3306, user="root", password="", database="demo")`,
          },
        ],
      );
    case "postgres":
      return r(
        "PostgreSQL",
        "关系型数据库",
        5432,
        false, "postgres", "",
        [
          { label: "JDBC", value: "jdbc:postgresql://127.0.0.1:5432/demo" },
          { label: "连接串", value: "postgresql://postgres@127.0.0.1:5432/demo" },
        ],
        [],
        [
          { key: "DB_HOST", value: "127.0.0.1" },
          { key: "DB_PORT", value: "5432" },
          { key: "DB_USER", value: "postgres" },
          { key: "DB_PASSWORD", value: "" },
          { key: "DATABASE_URL", value: "postgresql://postgres@127.0.0.1:5432/demo" },
        ],
        [
          {
            label: "Java Spring",
            lang: "yaml", caption: "application.yml",
            code: `spring:
  datasource:
    url: jdbc:postgresql://127.0.0.1:5432/demo
    username: postgres
    password:`,
          },
          {
            label: "Go",
            lang: "go", caption: "pgx",
            code: `import "github.com/jackc/pgx/v5/pgxpool"
pool, _ := pgxpool.New(ctx, "postgres://postgres@127.0.0.1:5432/demo?sslmode=disable")`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "node-postgres",
            code: `import { Pool } from "pg";
const pool = new Pool({ host: "127.0.0.1", port: 5432, user: "postgres", database: "demo" });`,
          },
          {
            label: "Python",
            lang: "python", caption: "psycopg3",
            code: `from psycopg_pool import ConnectionPool
pool = ConnectionPool("postgresql://postgres@127.0.0.1:5432/demo")`,
          },
        ],
      );
    case "mongodb":
      return r(
        "MongoDB",
        "文档数据库",
        27017,
        false, "", "",
        [
          { label: "连接串", value: "mongodb://127.0.0.1:27017/demo" },
        ],
        [],
        [
          { key: "MONGODB_URI", value: "mongodb://127.0.0.1:27017/demo" },
        ],
        [
          {
            label: "Java Spring",
            lang: "yaml", caption: "application.yml",
            code: `spring:
  data:
    mongodb:
      uri: mongodb://127.0.0.1:27017/demo`,
          },
          {
            label: "Go",
            lang: "go", caption: "mongo-driver",
            code: `import "go.mongodb.org/mongo-driver/mongo"
client, _ := mongo.Connect(ctx, options.Client().ApplyURI("mongodb://127.0.0.1:27017"))`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "mongodb",
            code: `import { MongoClient } from "mongodb";
const client = new MongoClient("mongodb://127.0.0.1:27017");
const db = client.db("demo");`,
          },
          {
            label: "Python",
            lang: "python", caption: "pymongo",
            code: `from pymongo import MongoClient
client = MongoClient("mongodb://127.0.0.1:27017")
db = client["demo"]`,
          },
        ],
      );
    case "mailpit":
      return r(
        "Mailpit",
        "本地邮件沙箱",
        1025,
        false, "", "",
        [
          { label: "SMTP", value: "127.0.0.1:1025" },
        ],
        [
          { label: "Web UI", value: "http://127.0.0.1:8025" },
        ],
        [
          { key: "SMTP_HOST", value: "127.0.0.1" },
          { key: "SMTP_PORT", value: "1025" },
          { key: "SMTP_USER", value: "" },
          { key: "SMTP_PASS", value: "" },
        ],
        [
          {
            label: "Java Spring",
            lang: "yaml", caption: "application.yml",
            code: `spring:
  mail:
    host: 127.0.0.1
    port: 1025
    properties:
      mail.smtp.auth: false
      mail.smtp.starttls.enable: false`,
          },
          {
            label: "Go",
            lang: "go", caption: "net/smtp",
            code: `import "net/smtp"
smtp.SendMail("127.0.0.1:1025", nil, "from@demo.local", []string{"to@demo.local"}, msg)`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "nodemailer",
            code: `import nodemailer from "nodemailer";
const t = nodemailer.createTransport({ host: "127.0.0.1", port: 1025, secure: false, ignoreTLS: true });`,
          },
          {
            label: "Python",
            lang: "python", caption: "smtplib",
            code: `import smtplib
with smtplib.SMTP("127.0.0.1", 1025) as smtp:
    smtp.send_message(msg)`,
          },
        ],
      );
    case "nats":
      return r(
        "NATS",
        "轻量消息服务器",
        4222,
        false, "", "",
        [
          { label: "NATS URL", value: "nats://127.0.0.1:4222" },
        ],
        [
          { label: "Monitoring", value: "http://127.0.0.1:8222" },
        ],
        [
          { key: "NATS_URL", value: "nats://127.0.0.1:4222" },
        ],
        [
          {
            label: "Java",
            lang: "java", caption: "jnats",
            code: `import io.nats.client.*;
Connection nc = Nats.connect("nats://127.0.0.1:4222");`,
          },
          {
            label: "Go",
            lang: "go", caption: "nats.go",
            code: `import "github.com/nats-io/nats.go"
nc, _ := nats.Connect("nats://127.0.0.1:4222")`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "nats.ws",
            code: `import { connect } from "nats.ws";
const nc = await connect({ servers: "127.0.0.1:4222" });`,
          },
          {
            label: "Python",
            lang: "python", caption: "nats-py",
            code: `from nats.aio.client import Client as NATS
nc = NATS()
await nc.connect("nats://127.0.0.1:4222")`,
          },
        ],
      );
    case "kafka":
      return r(
        "Kafka Sandbox",
        "Kafka API 兼容消息沙箱",
        9092,
        false, "", "",
        [
          { label: "Bootstrap Servers", value: "127.0.0.1:9092" },
        ],
        [
          { label: "运行时", value: "Tansu（无需 JVM / ZooKeeper）" },
          { label: "持久化", value: "SQLite" },
        ],
        [
          { key: "KAFKA_BOOTSTRAP_SERVERS", value: "127.0.0.1:9092" },
        ],
        [
          {
            label: "Java Spring",
            lang: "yaml", caption: "application.yml",
            code: `spring:
  kafka:
    bootstrap-servers: 127.0.0.1:9092`,
          },
          {
            label: "Go",
            lang: "go", caption: "franz-go",
            code: `client, _ := kgo.NewClient(kgo.SeedBrokers("127.0.0.1:9092"))`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "kafkajs",
            code: `const kafka = new Kafka({
  clientId: "local-app",
  brokers: ["127.0.0.1:9092"],
});`,
          },
          {
            label: "Python",
            lang: "python", caption: "kafka-python",
            code: `producer = KafkaProducer(
    bootstrap_servers="127.0.0.1:9092"
)`,
          },
        ],
      );
    case "meilisearch":
      return r(
        "Meilisearch",
        "全文搜索引擎",
        7700,
        false, "", "",
        [
          { label: "HTTP API", value: "http://127.0.0.1:7700" },
        ],
        [],
        [
          { key: "MEILISEARCH_URL", value: "http://127.0.0.1:7700" },
        ],
        [
          {
            label: "Java",
            lang: "java", caption: "meilisearch-java",
            code: `import com.meilisearch.sdk.*;
Client client = new Client(new Config("http://127.0.0.1:7700", null));`,
          },
          {
            label: "Go",
            lang: "go", caption: "meilisearch-go",
            code: `import "github.com/meilisearch/meilisearch-go"
client := meilisearch.New("http://127.0.0.1:7700")`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "meilisearch-js",
            code: `import { MeiliSearch } from "meilisearch";
const client = new MeiliSearch({ host: "http://127.0.0.1:7700" });`,
          },
          {
            label: "Python",
            lang: "python", caption: "meilisearch-python",
            code: `import meilisearch
client = meilisearch.Client("http://127.0.0.1:7700")`,
          },
        ],
      );
    case "minio":
      return r(
        "MinIO",
        "S3 兼容对象存储",
        9000,
        true, "zhiyuadmin", "zhiyu-local-minio-2026",
        [
          { label: "S3 Endpoint", value: "http://127.0.0.1:9000" },
        ],
        [
          { label: "Web Console", value: "http://127.0.0.1:9001" },
        ],
        [
          { key: "S3_ENDPOINT", value: "http://127.0.0.1:9000" },
          { key: "AWS_ACCESS_KEY_ID", value: "zhiyuadmin" },
          { key: "AWS_SECRET_ACCESS_KEY", value: "zhiyu-local-minio-2026" },
          { key: "AWS_REGION", value: "us-east-1" },
        ],
        [
          {
            label: "Java AWS SDK v2",
            lang: "java", caption: "S3Client",
            code: `S3Client s3 = S3Client.builder()
    .endpointOverride(URI.create("http://127.0.0.1:9000"))
    .credentialsProvider(StaticCredentialsProvider.create(
        AwsBasicCredentials.create("zhiyuadmin", "zhiyu-local-minio-2026")))
    .region(Region.US_EAST_1)
    .forcePathStyle(true)
    .build();`,
          },
          {
            label: "Go",
            lang: "go", caption: "minio-go",
            code: `import "github.com/minio/minio-go/v7"
client, _ := minio.New("127.0.0.1:9000", &minio.Options{
    Creds:  credentials.NewStaticV4("zhiyuadmin", "zhiyu-local-minio-2026", ""),
    Secure: false,
})`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "minio-js",
            code: `import * as Minio from "minio";
const client = new Minio.Client({
  endPoint: "127.0.0.1", port: 9000, useSSL: false,
  accessKey: "zhiyuadmin",
  secretKey: "zhiyu-local-minio-2026",
});`,
          },
          {
            label: "Python",
            lang: "python", caption: "minio-py",
            code: `from minio import Minio
client = Minio("127.0.0.1:9000",
    access_key="zhiyuadmin",
    secret_key="zhiyu-local-minio-2026",
    secure=False)`,
          },
        ],
      );
    case "rustfs":
      return r(
        "RustFS",
        "Rust 对象存储",
        9002,
        true, "zhiyuadmin", "zhiyu-local-rustfs-2026",
        [
          { label: "S3 Endpoint", value: "http://127.0.0.1:9002" },
        ],
        [
          { label: "Web Console", value: "http://127.0.0.1:7001" },
        ],
        [
          { key: "S3_ENDPOINT", value: "http://127.0.0.1:9002" },
          { key: "AWS_ACCESS_KEY_ID", value: "zhiyuadmin" },
          { key: "AWS_SECRET_ACCESS_KEY", value: "zhiyu-local-rustfs-2026" },
          { key: "AWS_REGION", value: "us-east-1" },
        ],
        [
          {
            label: "Go",
            lang: "go", caption: "minio-go",
            code: `client, _ := minio.New("127.0.0.1:9002", &minio.Options{
    Creds:  credentials.NewStaticV4("zhiyuadmin", "zhiyu-local-rustfs-2026", ""),
    Secure: false,
})`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "minio-js",
            code: `const client = new Minio.Client({
  endPoint: "127.0.0.1", port: 9002, useSSL: false,
  accessKey: "zhiyuadmin",
  secretKey: "zhiyu-local-rustfs-2026",
});`,
          },
        ],
      );
    case "etcd":
      return r(
        "etcd",
        "分布式键值存储",
        2379,
        false, "", "",
        [
          { label: "Client Endpoint", value: "http://127.0.0.1:2379" },
        ],
        [
          { label: "Peer Endpoint", value: "http://127.0.0.1:2380" },
        ],
        [
          { key: "ETCD_ENDPOINTS", value: "http://127.0.0.1:2379" },
        ],
        [
          {
            label: "Go",
            lang: "go", caption: "etcd client/v3",
            code: `import clientv3 "go.etcd.io/etcd/client/v3"
cli, _ := clientv3.New(clientv3.Config{
    Endpoints:   []string{"http://127.0.0.1:2379"},
    DialTimeout: 5 * time.Second,
})`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "etcd3",
            code: `import { Etcd3 } from "etcd3";
const client = new Etcd3({ hosts: "http://127.0.0.1:2379" });`,
          },
          {
            label: "Java",
            lang: "java", caption: "jetcd",
            code: `import io.etcd.jetcd.*;
Client client = Client.builder().endpoints("http://127.0.0.1:2379").build();`,
          },
          {
            label: "Python",
            lang: "python", caption: "etcd3-py",
            code: `import etcd3
client = etcd3.client(host="127.0.0.1", port=2379)`,
          },
        ],
      );
    case "consul":
      return r(
        "Consul",
        "服务发现与配置",
        8500,
        false, "", "",
        [
          { label: "HTTP API", value: "http://127.0.0.1:8500" },
        ],
        [
          { label: "DNS", value: "127.0.0.1:8600" },
        ],
        [
          { key: "CONSUL_HTTP_ADDR", value: "http://127.0.0.1:8500" },
        ],
        [
          {
            label: "Go",
            lang: "go", caption: "consul/api",
            code: `import "github.com/hashicorp/consul/api"
client, _ := api.NewClient(&api.Config{Address: "http://127.0.0.1:8500"})`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "consul",
            code: `import Consul from "consul";
const consul = new Consul({ host: "127.0.0.1", port: "8500" });`,
          },
          {
            label: "Java Spring",
            lang: "yaml", caption: "application.yml",
            code: `spring:
  cloud:
    consul:
      host: 127.0.0.1
      port: 8500`,
          },
          {
            label: "Python",
            lang: "python", caption: "python-consul",
            code: `import consul
c = consul.Consul(host="127.0.0.1", port=8500)`,
          },
        ],
      );
    case "rnacos":
      return r(
        "rnacos",
        "Nacos 兼容服务",
        8848,
        true, "admin", "admin",
        [
          { label: "Nacos HTTP", value: "http://127.0.0.1:8848" },
        ],
        [
          { label: "Nacos gRPC", value: "127.0.0.1:9848" },
          { label: "Web Console", value: "http://127.0.0.1:10848/rnacos/" },
        ],
        [
          { key: "NACOS_SERVER_ADDR", value: "127.0.0.1:8848" },
          { key: "NACOS_USERNAME", value: "admin" },
          { key: "NACOS_PASSWORD", value: "admin" },
        ],
        [
          {
            label: "Java Spring Cloud",
            lang: "yaml", caption: "application.yml",
            code: `spring:
  cloud:
    nacos:
      discovery:
        server-addr: 127.0.0.1:8848
      config:
        server-addr: 127.0.0.1:8848`,
          },
          {
            label: "Go",
            lang: "go", caption: "nacos-sdk-go",
            code: `sc := []constant.ServerConfig{
    *constant.NewServerConfig("127.0.0.1", 8848),
}`,
          },
        ],
      );
    case "rabbitmq":
      return r(
        "RabbitMQ",
        "AMQP 消息代理",
        5672,
        true, "zhiyu", "zhiyu-local-rabbitmq-2026",
        [
          { label: "AMQP", value: "amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:5672/" },
        ],
        [
          { label: "Management UI", value: "http://127.0.0.1:15672" },
        ],
        [
          { key: "RABBITMQ_URL", value: "amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:5672/" },
          { key: "RABBITMQ_HOST", value: "127.0.0.1" },
          { key: "RABBITMQ_PORT", value: "5672" },
          { key: "RABBITMQ_USER", value: "zhiyu" },
          { key: "RABBITMQ_PASS", value: "zhiyu-local-rabbitmq-2026" },
        ],
        [
          {
            label: "Java Spring AMQP",
            lang: "yaml", caption: "application.yml",
            code: `spring:
  rabbitmq:
    host: 127.0.0.1
    port: 5672
    username: zhiyu
    password: zhiyu-local-rabbitmq-2026`,
          },
          {
            label: "Go",
            lang: "go", caption: "amqp091-go",
            code: `import amqp "github.com/rabbitmq/amqp091-go"
conn, _ := amqp.Dial("amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:5672/")`,
          },
          {
            label: "TypeScript",
            lang: "typescript", caption: "amqplib",
            code: `import amqp from "amqplib";
const conn = await amqp.connect("amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:5672");`,
          },
          {
            label: "Python",
            lang: "python", caption: "pika",
            code: `import pika
conn = pika.BlockingConnection(pika.URLParameters(
    "amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:5672/"))`,
          },
        ],
      );
    case "nginx":
      return r(
        "Nginx",
        "轻量 Web 服务器 · 静态文件与反向代理",
        8081,
        false,
        "",
        "",
        [{ label: "HTTP", value: "http://127.0.0.1:8081" }],
        [],
        [],
        [],
      );
    default:
      return r(
        "Unknown",
        "",
        0,
        false,
        "",
        "",
        [],
        [],
        [],
        [],
      );
  }
}
