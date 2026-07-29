import type { DocChapter } from "../docTypes";

/** MySQL 使用文档。 */
export function buildMysqlDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 MySQL",
      navHint: "关系模型 · 何时该用",
      title: "MySQL 是什么",
      intro:
        "MySQL 是使用最广泛的开源关系型数据库。数据被组织成一张张有固定列的表，表与表之间通过外键关联，用 SQL 语言查询。",
      blocks: [
        {
          kind: "text",
          value:
            "关系型数据库的核心价值是「结构约束」和「事务保证」。你在建表时就声明清楚每列是什么类型、能不能为空、是否唯一，数据库会替你把住这道关；写入多张表时用事务包起来，要么全部成功要么全部回滚，不会出现钱扣了但订单没生成的中间状态。",
        },
        {
          kind: "text",
          value: "什么时候该选它：",
        },
        {
          kind: "list",
          items: [
            "数据结构相对稳定，字段基本固定下来了。",
            "需要跨表关联查询，比如「查某个用户的所有订单及订单明细」。",
            "对一致性要求高，涉及金额、库存、账务。",
            "团队熟悉 SQL，生态和运维经验最成熟。",
          ],
        },
        {
          kind: "table",
          head: ["", "MySQL", "Redis", "MongoDB"],
          rows: [
            ["数据模型", "固定列的表", "键值 + 数据结构", "自由结构的文档"],
            ["查询能力", "SQL，支持 JOIN、聚合", "按 key 定位", "文档查询 + 聚合管道"],
            ["事务", "完整的 ACID 事务", "单命令原子", "支持多文档事务"],
            ["典型延迟", "毫秒级", "微秒级", "毫秒级"],
            ["适合", "核心业务数据", "缓存与临时状态", "结构多变的数据"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "InnoDB 是默认存储引擎",
          value:
            "现在建表如果不特别指定，用的都是 InnoDB。它支持事务、行级锁和外键，是唯一推荐的选择。老项目里可能见到 MyISAM，它不支持事务，遇到了应该考虑迁移。",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "连接 · 建库建表",
      title: "连上并建出第一张表",
      intro:
        "智屿已经把 MySQL 装好并托管在本地，账号密码都配置完毕，直接连就行。",
      blocks: [
        {
          kind: "list",
          items: [
            "在「概览」标签页确认状态是「运行中」。",
            "切到「SQL 命令台」标签页，可以直接写 SQL 执行。",
            "「数据浏览」标签页能按库、按表翻数据，不用手写 SELECT。",
          ],
        },
        {
          kind: "table",
          head: ["参数", "值", "说明"],
          rows: [
            ["主机", "127.0.0.1", "只监听本机"],
            ["端口", String(port), "可在「配置文件」标签页修改"],
            ["用户名", "root", "本地开发实例的默认账号"],
            ["密码", "（空）", "本地实例默认不设密码"],
            ["连接串", `jdbc:mysql://127.0.0.1:${port}/your_db`, "JDBC 格式"],
          ],
        },
        {
          kind: "text",
          value: "先建一个库和两张有关联关系的表，后面的例子都基于它们：",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "建库建表",
          code: `CREATE DATABASE demo
  DEFAULT CHARACTER SET utf8mb4
  COLLATE utf8mb4_general_ci;

USE demo;

CREATE TABLE users (
  id          BIGINT       NOT NULL AUTO_INCREMENT,
  name        VARCHAR(64)  NOT NULL,
  email       VARCHAR(128) NOT NULL,
  age         INT          NULL,
  created_at  DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (id),
  UNIQUE KEY uk_email (email)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;

CREATE TABLE orders (
  id          BIGINT         NOT NULL AUTO_INCREMENT,
  user_id     BIGINT         NOT NULL,
  amount      DECIMAL(10, 2) NOT NULL,
  status      VARCHAR(16)    NOT NULL DEFAULT 'pending',
  created_at  DATETIME       NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (id),
  KEY idx_user_id (user_id),
  CONSTRAINT fk_orders_user FOREIGN KEY (user_id) REFERENCES users (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "字符集一定要用 utf8mb4",
          value:
            "MySQL 里叫 utf8 的那个字符集其实只支持 3 字节，存不了 emoji 和部分生僻字，插入时会直接报错或截断。utf8mb4 才是真正完整的 UTF-8，新建库表时务必显式指定。",
        },
        {
          kind: "callout",
          tone: "warn",
          title: "金额不要用 FLOAT 或 DOUBLE",
          value:
            "浮点数是二进制近似表示，0.1 + 0.2 不等于 0.3，用来存钱迟早对不上账。金额请用 DECIMAL(10, 2) 这样的定点类型，它是精确的。",
        },
      ],
    },

    {
      id: "sql",
      navLabel: "SQL 基础",
      navHint: "增删改查 · 关联",
      title: "日常最常用的 SQL",
      intro: "掌握这几类语句，日常开发里九成的数据库操作都能覆盖。",
      blocks: [
        {
          kind: "code",
          lang: "sql",
          caption: "写入与修改",
          code: `INSERT INTO users (name, email, age) VALUES ('张三', 'zhang@demo.com', 28);

-- 一次插入多行，比循环单条插入快得多
INSERT INTO users (name, email, age) VALUES
  ('李四', 'li@demo.com', 31),
  ('王五', 'wang@demo.com', 25);

UPDATE users SET age = 29 WHERE id = 1;

DELETE FROM users WHERE id = 3;`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "UPDATE 和 DELETE 永远要带 WHERE",
          value:
            "漏掉 WHERE 会作用于全表。稳妥的做法是先用同样的条件写一条 SELECT 确认影响范围，确认行数对了再改成 UPDATE 或 DELETE。",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "查询",
          code: `-- 只取需要的列，别习惯性写 SELECT *
SELECT id, name, email FROM users WHERE age >= 25 ORDER BY created_at DESC LIMIT 20;

-- 模糊匹配。注意前缀带 % 时用不上索引
SELECT * FROM users WHERE name LIKE '张%';

-- 范围与集合
SELECT * FROM orders WHERE amount BETWEEN 100 AND 500;
SELECT * FROM orders WHERE status IN ('pending', 'paid');

-- 分页：跳过前 40 条取 20 条
SELECT * FROM orders ORDER BY id DESC LIMIT 20 OFFSET 40;`,
        },
        {
          kind: "text",
          value:
            "JOIN 是关系型数据库最有价值的能力，把分散在多张表里的数据一次查出来：",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "关联查询",
          code: `-- INNER JOIN：只返回两边都能匹配上的行
SELECT u.name, o.id AS order_id, o.amount
FROM orders o
INNER JOIN users u ON u.id = o.user_id
WHERE o.status = 'paid';

-- LEFT JOIN：保留左表全部行，右表没匹配到的填 NULL
-- 下面这句能查出「一个订单都没有的用户」
SELECT u.name, COUNT(o.id) AS order_count
FROM users u
LEFT JOIN orders o ON o.user_id = u.id
GROUP BY u.id, u.name
HAVING COUNT(o.id) = 0;`,
        },
        {
          kind: "code",
          lang: "sql",
          caption: "聚合统计",
          code: `SELECT
  status,
  COUNT(*)    AS cnt,
  SUM(amount) AS total,
  AVG(amount) AS avg_amount
FROM orders
GROUP BY status
HAVING SUM(amount) > 1000;   -- HAVING 过滤分组结果，WHERE 过滤原始行`,
        },
        {
          kind: "text",
          value:
            "涉及多张表的写入要放进事务，保证一致性。智屿的 SQL 命令台里也可以直接执行：",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "事务",
          code: `START TRANSACTION;

INSERT INTO orders (user_id, amount, status) VALUES (1, 299.00, 'paid');
UPDATE users SET age = age + 1 WHERE id = 1;

COMMIT;    -- 全部生效
-- ROLLBACK;  -- 出错时全部撤销`,
        },
      ],
    },

    {
      id: "index",
      navLabel: "索引与性能",
      navHint: "加索引 · 看执行计划",
      title: "让查询快起来",
      intro:
        "数据量小的时候怎么写都快，上了几十万行之后，有没有索引的差距可能是几百倍。",
      blocks: [
        {
          kind: "text",
          value:
            "索引本质是一份按某列排好序的目录。没有索引时数据库只能从头扫到尾（全表扫描），有索引就能像查字典一样直接定位。",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "创建索引",
          code: `-- 普通索引：加速按该列的查询
CREATE INDEX idx_status ON orders (status);

-- 唯一索引：加速的同时保证不重复
CREATE UNIQUE INDEX uk_email ON users (email);

-- 联合索引：多列组合，顺序很重要
CREATE INDEX idx_user_status ON orders (user_id, status);

SHOW INDEX FROM orders;         -- 查看已有索引
DROP INDEX idx_status ON orders;`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "联合索引的最左前缀原则",
          value:
            "索引 (user_id, status) 能加速「只按 user_id 查」和「按 user_id + status 查」，但加速不了「只按 status 查」。建联合索引时，把最常用于过滤的列放在最前面。",
        },
        {
          kind: "text",
          value:
            "怀疑某条 SQL 慢，在前面加 EXPLAIN 就能看到数据库打算怎么执行它：",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "执行计划",
          code: `EXPLAIN SELECT * FROM orders WHERE user_id = 1 AND status = 'paid';

-- 重点看这几列：
--   type  : ALL 表示全表扫描（最差），ref / range 说明用上了索引，const 最好
--   key   : 实际使用的索引名，NULL 表示没用上索引
--   rows  : 预计要扫描的行数，越小越好
--   Extra : 出现 Using filesort 或 Using temporary 说明有额外排序开销`,
        },
        {
          kind: "text",
          value: "几个会让索引失效的常见写法，改掉就能提速：",
        },
        {
          kind: "table",
          head: ["失效写法", "原因", "改法"],
          rows: [
            [
              "WHERE YEAR(created_at) = 2024",
              "对列用了函数，索引是按原值排序的",
              "WHERE created_at >= '2024-01-01' AND created_at < '2025-01-01'",
            ],
            [
              "WHERE name LIKE '%三'",
              "前缀不确定，无法用有序目录定位",
              "尽量改成 '张%'，或改用全文检索",
            ],
            [
              "WHERE user_id = '1'",
              "字段是数字却传字符串，触发隐式类型转换",
              "参数类型和列类型保持一致",
            ],
            [
              "SELECT * 取全部列",
              "回表次数多，也无法走覆盖索引",
              "只写真正需要的列",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "索引不是越多越好",
          value:
            "每个索引都要占额外空间，而且每次 INSERT、UPDATE、DELETE 都要同步维护所有索引。一张表上十几个索引会让写入明显变慢，只给真正高频的查询条件建索引。",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Java · Go · TS · Python",
      title: "在你的项目里连上它",
      intro: `下面的配置都指向本机 127.0.0.1:${port}，数据库名按你自己建的替换。共同要点：一定要用连接池，一定要用参数化查询。`,
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
  <groupId>com.mysql</groupId>
  <artifactId>mysql-connector-j</artifactId>
  <scope>runtime</scope>
</dependency>`,
            },
            {
              label: "Java",
              lang: "yaml",
              caption: "application.yml",
              code: `spring:
  datasource:
    url: jdbc:mysql://127.0.0.1:${port}/demo?useUnicode=true&characterEncoding=utf8mb4&serverTimezone=Asia/Shanghai
    username: root
    password:
    hikari:
      maximum-pool-size: 10
      connection-timeout: 3000
  jpa:
    hibernate:
      ddl-auto: none     # 生产环境别用 update，靠迁移脚本管理表结构
    show-sql: true`,
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
        // 用 ? 占位，参数单独传，杜绝 SQL 注入
        return jdbc.query(
            "SELECT id, name, email FROM users WHERE age >= ?",
            (rs, rowNum) -> new User(
                rs.getLong("id"),
                rs.getString("name"),
                rs.getString("email")),
            minAge);
    }

    public int insert(String name, String email, int age) {
        return jdbc.update(
            "INSERT INTO users (name, email, age) VALUES (?, ?, ?)",
            name, email, age);
    }
}`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "安装驱动",
              code: `go get github.com/go-sql-driver/mysql`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "database/sql 连接与查询",
              code: `package store

import (
    "context"
    "database/sql"
    "time"

    _ "github.com/go-sql-driver/mysql"
)

var db *sql.DB

func Init() error {
    dsn := "root:@tcp(127.0.0.1:${port})/demo?charset=utf8mb4&parseTime=true&loc=Asia%2FShanghai"

    var err error
    db, err = sql.Open("mysql", dsn)
    if err != nil {
        return err
    }
    // sql.DB 本身就是连接池，全局复用一个即可
    db.SetMaxOpenConns(10)
    db.SetMaxIdleConns(5)
    db.SetConnMaxLifetime(time.Hour)

    return db.Ping()
}

func FindByMinAge(ctx context.Context, minAge int) ([]User, error) {
    rows, err := db.QueryContext(ctx,
        "SELECT id, name, email FROM users WHERE age >= ?", minAge)
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
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "安装驱动",
              code: `npm install mysql2`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "mysql2 连接池",
              code: `import mysql from "mysql2/promise";

// 模块级单例，整个进程复用
export const pool = mysql.createPool({
  host: "127.0.0.1",
  port: ${port},
  user: "root",
  password: "",
  database: "demo",
  charset: "utf8mb4",
  waitForConnections: true,
  connectionLimit: 10,
});

export async function findByMinAge(minAge: number) {
  // 第二个参数是占位符的值，驱动会安全转义
  const [rows] = await pool.query(
    "SELECT id, name, email FROM users WHERE age >= ?",
    [minAge],
  );
  return rows;
}

export async function insertUser(name: string, email: string, age: number) {
  const [result] = await pool.execute(
    "INSERT INTO users (name, email, age) VALUES (?, ?, ?)",
    [name, email, age],
  );
  return result;
}`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "安装驱动",
              code: `pip install pymysql`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "PyMySQL 查询",
              code: `import pymysql
from pymysql.cursors import DictCursor


def connect():
    return pymysql.connect(
        host="127.0.0.1",
        port=${port},
        user="root",
        password="",
        database="demo",
        charset="utf8mb4",
        cursorclass=DictCursor,
        autocommit=False,
    )


def find_by_min_age(min_age: int):
    with connect() as conn:
        with conn.cursor() as cur:
            # %s 是占位符，不是字符串格式化，参数走第二个入参
            cur.execute(
                "SELECT id, name, email FROM users WHERE age >= %s",
                (min_age,),
            )
            return cur.fetchall()


def insert_user(name: str, email: str, age: int) -> int:
    with connect() as conn:
        with conn.cursor() as cur:
            cur.execute(
                "INSERT INTO users (name, email, age) VALUES (%s, %s, %s)",
                (name, email, age),
            )
        conn.commit()
        return cur.lastrowid`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "永远不要拼接 SQL 字符串",
          value:
            "把用户输入直接拼进 SQL 是 SQL 注入的根源，攻击者可以借此读取甚至删除整库数据。所有语言的驱动都提供了占位符（? 或 %s），把参数单独传进去，驱动会负责转义。",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "避坑与调优",
      navHint: "慢查询 · 锁 · 排查",
      title: "上线前该知道的事",
      intro: "这些问题本地开发时基本遇不到，数据量和并发上来之后几乎都会碰到。",
      blocks: [
        {
          kind: "code",
          lang: "sql",
          caption: "排查命令",
          code: `SHOW PROCESSLIST;              -- 当前在跑的连接和查询，找卡住的语句
SHOW ENGINE INNODB STATUS;     -- 死锁信息在 LATEST DETECTED DEADLOCK 段落

SHOW VARIABLES LIKE 'slow_query_log';     -- 慢查询日志是否开启
SHOW VARIABLES LIKE 'long_query_time';    -- 超过多少秒算慢查询
SHOW VARIABLES LIKE 'max_connections';    -- 最大连接数

-- 查看每张表占了多少空间
SELECT table_name,
       ROUND((data_length + index_length) / 1024 / 1024, 2) AS mb
FROM information_schema.tables
WHERE table_schema = 'demo'
ORDER BY mb DESC;`,
        },
        {
          kind: "table",
          head: ["问题", "现象", "对策"],
          rows: [
            [
              "N+1 查询",
              "先查列表再循环逐条查详情，产生大量小查询",
              "改成一次 JOIN，或用 IN 批量查",
            ],
            [
              "深分页慢",
              "LIMIT 20 OFFSET 1000000 越翻越慢",
              "改用「上次最大 id」游标翻页：WHERE id < ? ORDER BY id DESC LIMIT 20",
            ],
            [
              "连接数耗尽",
              "报 Too many connections",
              "用连接池并设上限，排查是否有连接未关闭",
            ],
            [
              "死锁",
              "报 Deadlock found when trying to get lock",
              "让所有事务按相同顺序访问表和行；缩短事务持有时间",
            ],
            [
              "大事务",
              "事务里做了几万行更新，锁表时间长",
              "拆成小批次提交，每批几百到几千行",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "改表结构在大表上会很危险",
          value:
            "ALTER TABLE 在数据量大的表上可能锁表几分钟到几小时，期间业务全部阻塞。生产环境改大表应该用 pt-online-schema-change 或 gh-ost 这类在线改表工具，或者选在低峰期操作。",
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么做",
          value:
            "「概览」标签页有连接数、QPS 等实时指标；「运行日志」能看到启动报错和慢查询；改 max_connections 这类参数在「配置文件」标签页编辑保存后重启即可；「备份恢复」标签页可以在做危险操作前先打一个快照。",
        },
      ],
    },
  ];
}
