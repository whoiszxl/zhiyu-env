import type { DocChapter } from "./docTypes";

/** PostgreSQL 使用文档。 */
export function buildPostgresDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 PostgreSQL",
      navHint: "特点 · 与 MySQL 的差异",
      title: "PostgreSQL 是什么",
      intro:
        "PostgreSQL 是一个以标准兼容和功能完整著称的开源关系型数据库。它同样用表和 SQL，但在数据类型、索引种类和扩展能力上比 MySQL 走得更远。",
      blocks: [
        {
          kind: "text",
          value:
            "如果说 MySQL 的定位是「够用、够快、生态成熟」，PostgreSQL 的定位则是「严谨、能力强、能干复杂的活」。它对 SQL 标准的遵循度更高，内置了 JSONB、数组、范围、地理位置等丰富类型，还能通过扩展直接变成时序库或向量库。",
        },
        {
          kind: "text",
          value: "相比 MySQL 值得关注的几点：",
        },
        {
          kind: "list",
          items: [
            "JSONB 是原生的二进制 JSON 类型，能建索引、能按路径查询，可以在一张表里同时享受结构化和半结构化的好处。",
            "支持窗口函数、CTE（WITH 子句）、物化视图这些做数据分析很顺手的能力。",
            "索引类型丰富：除了常规的 B-tree，还有适合全文和 JSONB 的 GIN、适合范围和地理的 GiST。",
            "有 schema（模式）这一层，可以在同一个数据库里做逻辑隔离，不必建多个库。",
            "写入用多版本并发控制，读不阻塞写、写不阻塞读。",
          ],
        },
        {
          kind: "table",
          head: ["", "PostgreSQL", "MySQL"],
          rows: [
            ["自增主键", "GENERATED ALWAYS AS IDENTITY 或 SERIAL", "AUTO_INCREMENT"],
            ["字符串拼接", "|| 运算符", "CONCAT() 函数"],
            ["大小写", "标识符默认转小写，字符串比较区分大小写", "表名大小写随系统，字符串比较默认不区分"],
            ["分页", "LIMIT 20 OFFSET 40", "LIMIT 20 OFFSET 40（写法相同）"],
            ["JSON", "JSONB，可索引可查询", "JSON，能力较弱"],
            ["层级", "库 → schema → 表", "库 → 表"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "标识符会被转成小写",
          value:
            "在 PostgreSQL 里写 CREATE TABLE Users，实际建出来的表叫 users。如果你用双引号写成 \"Users\"，那它就真的是大写，之后每次引用都必须带双引号。建议统一用小写加下划线命名，省掉所有麻烦。",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "连接 · 建表",
      title: "连上并建出第一张表",
      intro: "智屿已经初始化好实例和超级用户，直接连即可。",
      blocks: [
        {
          kind: "table",
          head: ["参数", "值", "说明"],
          rows: [
            ["主机", "127.0.0.1", "只监听本机"],
            ["端口", String(port), "可在「配置文件」标签页修改"],
            ["用户名", "postgres", "初始化时创建的超级用户"],
            ["密码", "（空）", "本地开发实例默认信任本机连接"],
            ["默认库", "postgres", "建议另建业务库，不要直接用它"],
            ["连接串", `postgresql://postgres@127.0.0.1:${port}/demo`, "多数客户端库都认这个格式"],
          ],
        },
        {
          kind: "code",
          lang: "sql",
          caption: "建库建表",
          code: `CREATE DATABASE demo ENCODING 'UTF8';

-- 切到 demo 库后执行（智屿的 SQL 命令台可以直接切换库）
CREATE TABLE users (
  id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  name       VARCHAR(64)  NOT NULL,
  email      VARCHAR(128) NOT NULL UNIQUE,
  age        INT,
  profile    JSONB        NOT NULL DEFAULT '{}',
  tags       TEXT[]       NOT NULL DEFAULT '{}',
  created_at TIMESTAMPTZ  NOT NULL DEFAULT now()
);

CREATE TABLE orders (
  id         BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
  user_id    BIGINT         NOT NULL REFERENCES users (id) ON DELETE CASCADE,
  amount     NUMERIC(10, 2) NOT NULL,
  status     VARCHAR(16)    NOT NULL DEFAULT 'pending',
  created_at TIMESTAMPTZ    NOT NULL DEFAULT now()
);

CREATE INDEX idx_orders_user_id ON orders (user_id);`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "时间列优先用 TIMESTAMPTZ",
          value:
            "TIMESTAMPTZ 会带时区信息存储，跨时区部署时不会算错；而 TIMESTAMP 不带时区，等于把时区问题留给应用层。金额同理，用 NUMERIC 而不是 REAL 或 DOUBLE PRECISION。",
        },
        {
          kind: "text",
          value: "如果习惯终端，系统自带的 psql 可以这样连：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "psql 连接",
          code: `psql -h 127.0.0.1 -p ${port} -U postgres -d demo

-- 进去之后几个常用的元命令
\\l          -- 列出所有数据库
\\dt         -- 列出当前 schema 下的表
\\d users    -- 查看 users 表结构
\\q          -- 退出`,
        },
      ],
    },

    {
      id: "sql",
      navLabel: "SQL 与特色能力",
      navHint: "CRUD · JSONB · 窗口函数",
      title: "常用 SQL 和 PostgreSQL 特色写法",
      intro:
        "基础的增删改查和其他关系库没有区别，这里重点放在那些能显著省代码的特色能力上。",
      blocks: [
        {
          kind: "code",
          lang: "sql",
          caption: "增删改查",
          code: `INSERT INTO users (name, email, age) VALUES ('张三', 'zhang@demo.com', 28);

-- RETURNING 让插入的同时直接拿回自增 id，省掉一次查询
INSERT INTO users (name, email, age) VALUES ('李四', 'li@demo.com', 31)
RETURNING id, created_at;

UPDATE users SET age = 29 WHERE id = 1;
DELETE FROM orders WHERE status = 'cancelled';

SELECT id, name, email FROM users
WHERE age >= 25
ORDER BY created_at DESC
LIMIT 20 OFFSET 0;`,
        },
        {
          kind: "text",
          value:
            "UPSERT（存在就更新、不存在就插入）在 PostgreSQL 里用 ON CONFLICT 表达，非常常用：",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "UPSERT",
          code: `INSERT INTO users (name, email, age)
VALUES ('张三', 'zhang@demo.com', 30)
ON CONFLICT (email)                 -- 命中 email 唯一约束时
DO UPDATE SET age = EXCLUDED.age,   -- EXCLUDED 指代本次想插入的那行
              name = EXCLUDED.name;

-- 冲突时什么都不做，直接跳过
INSERT INTO users (name, email) VALUES ('王五', 'zhang@demo.com')
ON CONFLICT (email) DO NOTHING;`,
        },
        {
          kind: "text",
          value:
            "JSONB 让你在关系表里放灵活字段，而且能查能索引，适合存那些「每个业务方字段都不一样」的配置或扩展属性：",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "JSONB",
          code: `UPDATE users
SET profile = '{"city":"杭州","vip":true,"score":88}'
WHERE id = 1;

-- ->  取出的还是 JSON，->> 取出的是文本
SELECT profile -> 'city'    AS city_json,
       profile ->> 'city'   AS city_text
FROM users WHERE id = 1;

-- @> 判断包含关系，这是最常用的 JSONB 过滤方式
SELECT * FROM users WHERE profile @> '{"vip": true}';

-- 按嵌套路径取值
SELECT profile #>> '{address,street}' FROM users;

-- 给 JSONB 建 GIN 索引，@> 查询就能走索引
CREATE INDEX idx_users_profile ON users USING GIN (profile);`,
        },
        {
          kind: "text",
          value:
            "窗口函数可以在不折叠行的前提下做分组统计，写排名、环比、组内取 TopN 时特别省事：",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "窗口函数",
          code: `-- 每个用户的订单，按金额在组内排名
SELECT
  user_id,
  id AS order_id,
  amount,
  ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY amount DESC) AS rn,
  SUM(amount)  OVER (PARTITION BY user_id)                      AS user_total
FROM orders;

-- 配合 CTE 取每个用户金额最高的那一单
WITH ranked AS (
  SELECT id, user_id, amount,
         ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY amount DESC) AS rn
  FROM orders
)
SELECT * FROM ranked WHERE rn = 1;`,
        },
        {
          kind: "code",
          lang: "sql",
          caption: "数组与关联",
          code: `-- 数组类型的写入与查询
UPDATE users SET tags = ARRAY['后端', 'Go'] WHERE id = 1;
SELECT * FROM users WHERE '后端' = ANY (tags);

-- JOIN 写法和标准 SQL 一致
SELECT u.name, COUNT(o.id) AS order_count, COALESCE(SUM(o.amount), 0) AS total
FROM users u
LEFT JOIN orders o ON o.user_id = u.id
GROUP BY u.id, u.name
ORDER BY total DESC;`,
        },
      ],
    },

    {
      id: "index",
      navLabel: "索引与性能",
      navHint: "索引类型 · 执行计划",
      title: "让查询快起来",
      intro:
        "PostgreSQL 的索引种类比一般数据库多，选对类型比盲目加索引更有效。",
      blocks: [
        {
          kind: "table",
          head: ["索引类型", "适合", "示例场景"],
          rows: [
            ["B-tree（默认）", "等值和范围比较", "WHERE status = ? / created_at > ?"],
            ["GIN", "包含关系、多值列", "JSONB 的 @> 查询、数组、全文检索"],
            ["GiST", "几何、范围类型", "地理位置、时间区间重叠"],
            ["BRIN", "超大表上天然有序的列", "按时间顺序追加的日志表"],
          ],
        },
        {
          kind: "code",
          lang: "sql",
          caption: "创建索引",
          code: `CREATE INDEX idx_orders_status ON orders (status);
CREATE INDEX idx_orders_user_status ON orders (user_id, status);
CREATE INDEX idx_users_profile ON users USING GIN (profile);

-- 部分索引：只给关心的那部分行建，体积更小
CREATE INDEX idx_orders_pending ON orders (created_at) WHERE status = 'pending';

-- 表达式索引：让函数条件也能走索引
CREATE INDEX idx_users_lower_email ON users (LOWER(email));

-- 生产环境建索引加 CONCURRENTLY，不会长时间锁表
CREATE INDEX CONCURRENTLY idx_orders_created ON orders (created_at);`,
        },
        {
          kind: "code",
          lang: "sql",
          caption: "执行计划",
          code: `-- EXPLAIN 只给预估，加上 ANALYZE 会真正执行并给出实际耗时
EXPLAIN ANALYZE
SELECT * FROM orders WHERE user_id = 1 AND status = 'paid';

-- 读结果的要点：
--   Seq Scan   顺序扫描，大表上出现说明没走索引
--   Index Scan 走了索引，通常是想要的结果
--   actual time 实际耗时，和 rows 一起看能发现预估偏差
--   Rows Removed by Filter 很大，说明过滤条件没被索引利用上`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "记得让 autovacuum 正常工作",
          value:
            "PostgreSQL 更新和删除时不会立刻回收空间，而是留下死元组，靠 autovacuum 后台清理。如果它被关掉或跟不上写入速度，表会不断膨胀、查询逐渐变慢。可以用 pg_stat_user_tables 查看 n_dead_tup 和上次 vacuum 时间。",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Java · Go · TS · Python",
      title: "在你的项目里连上它",
      intro: `下面的配置都指向本机 127.0.0.1:${port}，库名按你自己建的替换。共同要点：用连接池，用参数化查询。`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java",
              lang: "xml",
              caption: "Spring Boot：pom.xml",
              code: `<dependency>
  <groupId>org.springframework.boot</groupId>
  <artifactId>spring-boot-starter-data-jpa</artifactId>
</dependency>
<dependency>
  <groupId>org.postgresql</groupId>
  <artifactId>postgresql</artifactId>
  <scope>runtime</scope>
</dependency>`,
            },
            {
              label: "Java",
              lang: "yaml",
              caption: "application.yml",
              code: `spring:
  datasource:
    url: jdbc:postgresql://127.0.0.1:${port}/demo
    username: postgres
    password:
    hikari:
      maximum-pool-size: 10
      connection-timeout: 3000
  jpa:
    hibernate:
      ddl-auto: none
    properties:
      hibernate:
        dialect: org.hibernate.dialect.PostgreSQLDialect`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "JdbcTemplate 查询",
              code: `@Repository
public class UserRepository {

    private final JdbcTemplate jdbc;

    public UserRepository(JdbcTemplate jdbc) {
        this.jdbc = jdbc;
    }

    public List<User> findByMinAge(int minAge) {
        return jdbc.query(
            "SELECT id, name, email FROM users WHERE age >= ?",
            (rs, rowNum) -> new User(
                rs.getLong("id"),
                rs.getString("name"),
                rs.getString("email")),
            minAge);
    }

    // RETURNING 配合 queryForObject，插入后直接拿到新 id
    public long insert(String name, String email, int age) {
        return jdbc.queryForObject(
            "INSERT INTO users (name, email, age) VALUES (?, ?, ?) RETURNING id",
            Long.class, name, email, age);
    }
}`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "安装驱动",
              code: `go get github.com/jackc/pgx/v5`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "pgx 连接池",
              code: `package store

import (
    "context"
    "github.com/jackc/pgx/v5/pgxpool"
)

var pool *pgxpool.Pool

func Init(ctx context.Context) error {
    dsn := "postgres://postgres@127.0.0.1:${port}/demo?sslmode=disable&pool_max_conns=10"

    var err error
    // Pool 是并发安全的，全局复用一个
    pool, err = pgxpool.New(ctx, dsn)
    if err != nil {
        return err
    }
    return pool.Ping(ctx)
}

func FindByMinAge(ctx context.Context, minAge int) ([]User, error) {
    // pgx 用 $1 $2 作为占位符，不是 ?
    rows, err := pool.Query(ctx,
        "SELECT id, name, email FROM users WHERE age >= $1", minAge)
    if err != nil {
        return nil, err
    }
    defer rows.Close()

    var users []User
    for rows.Next() {
        var u User
        if err := rows.Scan(&u.ID, &u.Name, &u.Email); err != nil {
            return nil, err
        }
        users = append(users, u)
    }
    return users, rows.Err()
}

func Insert(ctx context.Context, name, email string, age int) (int64, error) {
    var id int64
    err := pool.QueryRow(ctx,
        "INSERT INTO users (name, email, age) VALUES ($1, $2, $3) RETURNING id",
        name, email, age).Scan(&id)
    return id, err
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "安装驱动",
              code: `npm install pg`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "node-postgres 连接池",
              code: `import { Pool } from "pg";

// 模块级单例，整个进程复用
export const pool = new Pool({
  host: "127.0.0.1",
  port: ${port},
  user: "postgres",
  password: "",
  database: "demo",
  max: 10,
  idleTimeoutMillis: 30_000,
});

export async function findByMinAge(minAge: number) {
  // 占位符是 $1 $2，参数走第二个数组
  const { rows } = await pool.query(
    "SELECT id, name, email FROM users WHERE age >= $1",
    [minAge],
  );
  return rows;
}

export async function insertUser(name: string, email: string, age: number) {
  const { rows } = await pool.query(
    "INSERT INTO users (name, email, age) VALUES ($1, $2, $3) RETURNING id",
    [name, email, age],
  );
  return rows[0].id as number;
}`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "安装驱动",
              code: `pip install "psycopg[binary,pool]"`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "psycopg3 连接池",
              code: `from psycopg_pool import ConnectionPool
from psycopg.rows import dict_row

pool = ConnectionPool(
    "postgresql://postgres@127.0.0.1:${port}/demo",
    min_size=2,
    max_size=10,
)


def find_by_min_age(min_age: int):
    with pool.connection() as conn:
        with conn.cursor(row_factory=dict_row) as cur:
            cur.execute(
                "SELECT id, name, email FROM users WHERE age >= %s",
                (min_age,),
            )
            return cur.fetchall()


def insert_user(name: str, email: str, age: int) -> int:
    with pool.connection() as conn:
        with conn.cursor() as cur:
            cur.execute(
                "INSERT INTO users (name, email, age) "
                "VALUES (%s, %s, %s) RETURNING id",
                (name, email, age),
            )
            return cur.fetchone()[0]`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "注意占位符写法的差异",
          value:
            "PostgreSQL 的原生占位符是 $1、$2 这种带编号的形式（pgx、node-postgres 用它），而 JDBC 和 psycopg 分别用 ? 和 %s。用错会直接报语法错误。无论哪种，都不要用字符串拼接来传参。",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "避坑与调优",
      navHint: "膨胀 · 连接 · 排查",
      title: "上线前该知道的事",
      intro: "PostgreSQL 有几个和 MySQL 不太一样的坑，值得提前了解。",
      blocks: [
        {
          kind: "code",
          lang: "sql",
          caption: "排查命令",
          code: `-- 当前正在执行的查询，找出跑太久的
SELECT pid, state, now() - query_start AS duration, query
FROM pg_stat_activity
WHERE state != 'idle'
ORDER BY duration DESC;

-- 干掉一个卡死的查询（先 cancel，不行再 terminate）
SELECT pg_cancel_backend(12345);
SELECT pg_terminate_backend(12345);

-- 表膨胀与清理情况
SELECT relname, n_live_tup, n_dead_tup, last_autovacuum
FROM pg_stat_user_tables
ORDER BY n_dead_tup DESC;

-- 各表占用空间
SELECT relname, pg_size_pretty(pg_total_relation_size(relid)) AS size
FROM pg_catalog.pg_statio_user_tables
ORDER BY pg_total_relation_size(relid) DESC;`,
        },
        {
          kind: "table",
          head: ["问题", "现象", "对策"],
          rows: [
            [
              "表膨胀",
              "大量更新删除后表越来越大，查询变慢",
              "确认 autovacuum 开启；必要时 VACUUM FULL（会锁表）",
            ],
            [
              "空闲事务不提交",
              "pg_stat_activity 里大量 idle in transaction",
              "应用里事务用完必须 commit 或 rollback；设 idle_in_transaction_session_timeout",
            ],
            [
              "连接数耗尽",
              "报 too many clients already",
              "PostgreSQL 每个连接是一个进程，开销比 MySQL 大，务必用连接池",
            ],
            [
              "深分页慢",
              "OFFSET 很大时越翻越慢",
              "改用游标翻页：WHERE id < ? ORDER BY id DESC LIMIT 20",
            ],
            [
              "统计信息过期",
              "执行计划选错索引",
              "手动 ANALYZE 表，更新优化器统计信息",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么做",
          value:
            "「概览」标签页有连接数和实时指标；「运行日志」能看到启动报错；调 shared_buffers、max_connections 这些参数在「配置文件」标签页改完重启即可；做危险操作前建议先去「备份恢复」标签页打一个快照。",
        },
      ],
    },
  ];
}
