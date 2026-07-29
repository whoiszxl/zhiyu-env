import type { DocChapter } from "../docTypes";

/** MySQL documentation. */
export function buildMysqlDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "Meet MySQL",
      navHint: "Relational model · When to use it",
      title: "What is MySQL",
      intro:
        "MySQL is the most widely used open-source relational database. Data is organized into tables with fixed columns, tables are linked through foreign keys, and you query them with SQL.",
      blocks: [
        {
          kind: "text",
          value:
            "The core value of a relational database is \"structural constraints\" and \"transactional guarantees\". When you create a table, you declare exactly what type each column is, whether it can be null, and whether it must be unique — the database enforces these for you. When writing to multiple tables, wrap it in a transaction: either everything succeeds or everything rolls back, so you never end up with money deducted but no order created.",
        },
        {
          kind: "text",
          value: "When to choose it:",
        },
        {
          kind: "list",
          items: [
            "The data structure is relatively stable and the columns are mostly fixed.",
            "You need to join across tables, for example \"fetch all orders and order line items for a user\".",
            "High consistency requirements — money, inventory, accounting.",
            "The team knows SQL, and the ecosystem and operational experience are the most mature.",
          ],
        },
        {
          kind: "table",
          head: ["", "MySQL", "Redis", "MongoDB"],
          rows: [
            ["Data model", "Tables with fixed columns", "Key-value + data structures", "Free-form documents"],
            ["Query capability", "SQL with JOINs and aggregation", "Lookup by key", "Document queries + aggregation pipeline"],
            ["Transactions", "Full ACID transactions", "Single-command atomicity", "Multi-document transactions supported"],
            ["Typical latency", "Milliseconds", "Microseconds", "Milliseconds"],
            ["Best for", "Core business data", "Cache and ephemeral state", "Data with varying structure"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "InnoDB is the default storage engine",
          value:
            "When you create a table today without specifying otherwise, it uses InnoDB. It supports transactions, row-level locking, and foreign keys, and is the only recommended choice. In legacy projects you may see MyISAM, which does not support transactions — consider migrating when you encounter it.",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quick start",
      navHint: "Connect · Create databases and tables",
      title: "Connect and create your first table",
      intro:
        "Zhiyu has already installed and is managing MySQL locally, with credentials pre-configured — just connect.",
      blocks: [
        {
          kind: "list",
          items: [
            "On the Overview tab, confirm the status is \"Running\".",
            "Switch to the SQL Console tab to run SQL directly.",
            "The Data Browser tab lets you flip through data by database and table without writing SELECT statements by hand.",
          ],
        },
        {
          kind: "table",
          head: ["Parameter", "Value", "Notes"],
          rows: [
            ["Host", "127.0.0.1", "Listens on localhost only"],
            ["Port", String(port), "Can be changed on the Config File tab"],
            ["Username", "root", "Default account for the local dev instance"],
            ["Password", "(empty)", "No password by default on the local instance"],
            ["Connection string", `jdbc:mysql://127.0.0.1:${port}/your_db`, "JDBC format"],
          ],
        },
        {
          kind: "text",
          value: "First create a database and two related tables — the following examples all build on them:",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "Create database and tables",
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
          title: "Always use utf8mb4 as the character set",
          value:
            "The character set called utf8 in MySQL only supports 3 bytes and cannot store emoji or some rare characters — inserts will fail or be truncated. utf8mb4 is real, full UTF-8. Always specify it explicitly when creating databases and tables.",
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Don't use FLOAT or DOUBLE for money",
          value:
            "Floating-point numbers are binary approximations — 0.1 + 0.2 is not 0.3, and sooner or later your books will not balance. Use a fixed-point type like DECIMAL(10, 2) for money, which is exact.",
        },
      ],
    },

    {
      id: "sql",
      navLabel: "SQL basics",
      navHint: "CRUD · Joins",
      title: "The SQL you'll use every day",
      intro: "Master these categories of statements and they'll cover 90% of the database operations in day-to-day development.",
      blocks: [
        {
          kind: "code",
          lang: "sql",
          caption: "Insert and update",
          code: `INSERT INTO users (name, email, age) VALUES ('张三', 'zhang@demo.com', 28);

-- Insert multiple rows at once — far faster than looping single inserts
INSERT INTO users (name, email, age) VALUES
  ('李四', 'li@demo.com', 31),
  ('王五', 'wang@demo.com', 25);

UPDATE users SET age = 29 WHERE id = 1;

DELETE FROM users WHERE id = 3;`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Always include WHERE with UPDATE and DELETE",
          value:
            "Missing a WHERE clause hits the entire table. The safe habit is to first run a SELECT with the same condition to confirm the impact, then change it to UPDATE or DELETE once the row count looks right.",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "Queries",
          code: `-- Select only the columns you need; don't reflexively write SELECT *
SELECT id, name, email FROM users WHERE age >= 25 ORDER BY created_at DESC LIMIT 20;

-- Wildcard matching. Note: a leading % prevents index use
SELECT * FROM users WHERE name LIKE '张%';

-- Ranges and sets
SELECT * FROM orders WHERE amount BETWEEN 100 AND 500;
SELECT * FROM orders WHERE status IN ('pending', 'paid');

-- Pagination: skip the first 40 rows, take 20
SELECT * FROM orders ORDER BY id DESC LIMIT 20 OFFSET 40;`,
        },
        {
          kind: "text",
          value:
            "JOIN is the most valuable capability of a relational database — it pulls data spread across multiple tables in a single query:",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "Join queries",
          code: `-- INNER JOIN: return only rows that match on both sides
SELECT u.name, o.id AS order_id, o.amount
FROM orders o
INNER JOIN users u ON u.id = o.user_id
WHERE o.status = 'paid';

-- LEFT JOIN: keep all rows from the left table; unmatched rows on the right become NULL
-- The query below finds "users who don't have a single order"
SELECT u.name, COUNT(o.id) AS order_count
FROM users u
LEFT JOIN orders o ON o.user_id = u.id
GROUP BY u.id, u.name
HAVING COUNT(o.id) = 0;`,
        },
        {
          kind: "code",
          lang: "sql",
          caption: "Aggregation",
          code: `SELECT
  status,
  COUNT(*)    AS cnt,
  SUM(amount) AS total,
  AVG(amount) AS avg_amount
FROM orders
GROUP BY status
HAVING SUM(amount) > 1000;   -- HAVING filters grouped results, WHERE filters raw rows`,
        },
        {
          kind: "text",
          value:
            "Writes that span multiple tables should be wrapped in a transaction for consistency. You can run this directly in the Zhiyu SQL Console:",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "Transactions",
          code: `START TRANSACTION;

INSERT INTO orders (user_id, amount, status) VALUES (1, 299.00, 'paid');
UPDATE users SET age = age + 1 WHERE id = 1;

COMMIT;    -- Apply everything
-- ROLLBACK;  -- Undo everything on error`,
        },
      ],
    },

    {
      id: "index",
      navLabel: "Indexes and performance",
      navHint: "Add indexes · Read execution plans",
      title: "Make your queries fast",
      intro:
        "With small data everything is fast. Once you cross a few hundred thousand rows, the gap between having an index or not can be hundreds of times.",
      blocks: [
        {
          kind: "text",
          value:
            "An index is essentially a directory sorted by a specific column. Without an index, the database can only scan from start to end (a full table scan); with an index it can jump straight to the row like looking up a word in a dictionary.",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "Create indexes",
          code: `-- Regular index: speeds up queries on that column
CREATE INDEX idx_status ON orders (status);

-- Unique index: speeds up queries and enforces uniqueness
CREATE UNIQUE INDEX uk_email ON users (email);

-- Composite index: multiple columns combined; order matters
CREATE INDEX idx_user_status ON orders (user_id, status);

SHOW INDEX FROM orders;         -- View existing indexes
DROP INDEX idx_status ON orders;`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Leftmost-prefix rule for composite indexes",
          value:
            "The index (user_id, status) accelerates queries filtered by user_id alone and by user_id + status, but not queries filtered by status alone. When designing a composite index, put the most frequently filtered column first.",
        },
        {
          kind: "text",
          value:
            "If you suspect a query is slow, prefix it with EXPLAIN to see how the database plans to execute it:",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "Execution plan",
          code: `EXPLAIN SELECT * FROM orders WHERE user_id = 1 AND status = 'paid';

-- Focus on these columns:
--   type  : ALL means full table scan (worst); ref / range means an index is used; const is best
--   key   : the index actually used, NULL means no index used
--   rows  : estimated rows to scan, smaller is better
--   Extra : Using filesort or Using temporary means extra sorting overhead`,
        },
        {
          kind: "text",
          value: "A few common patterns that break index usage — fix these for a speedup:",
        },
        {
          kind: "table",
          head: ["Broken pattern", "Reason", "Fix"],
          rows: [
            [
              "WHERE YEAR(created_at) = 2024",
              "A function is applied to the column; the index is sorted by the raw value",
              "WHERE created_at >= '2024-01-01' AND created_at < '2025-01-01'",
            ],
            [
              "WHERE name LIKE '%三'",
              "Prefix is unknown, so the sorted index cannot locate rows",
              "Rewrite to '张%' when possible, or switch to full-text search",
            ],
            [
              "WHERE user_id = '1'",
              "Numeric column but a string is passed, triggering implicit type conversion",
              "Keep parameter types aligned with the column type",
            ],
            [
              "SELECT * fetches all columns",
              "More round-trips to the table and no chance for a covering index",
              "Only select the columns you actually need",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "More indexes isn't always better",
          value:
            "Every index takes extra space, and every INSERT, UPDATE, and DELETE has to keep all indexes in sync. A table with a dozen indexes will noticeably slow down writes — only index the query conditions that are truly high-frequency.",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "Language clients",
      navHint: "Java · Go · TS · Python",
      title: "Connect from your project",
      intro: `All the configs below point to 127.0.0.1:${port} on this machine — swap in your own database name. Two rules apply everywhere: always use a connection pool, and always use parameterized queries.`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java",
              lang: "xml",
              caption: "Spring Boot: pom.xml",
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
      ddl-auto: none     # Don't use update in production; manage schema with migration scripts
    show-sql: true`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "JdbcTemplate query",
              code: `@Repository
public class UserRepository {

    private final JdbcTemplate jdbc;

    public UserRepository(JdbcTemplate jdbc) {
        this.jdbc = jdbc;
    }

    public List<User> findByMinAge(int minAge) {
        // Use ? placeholders and pass args separately to eliminate SQL injection
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
              caption: "Install the driver",
              code: `go get github.com/go-sql-driver/mysql`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "database/sql connection and query",
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
    // sql.DB is itself a connection pool — reuse one globally
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
              caption: "Install the driver",
              code: `npm install mysql2`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "mysql2 connection pool",
              code: `import mysql from "mysql2/promise";

// Module-level singleton reused across the process
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
  // The second argument holds placeholder values; the driver escapes them safely
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
              caption: "Install the driver",
              code: `pip install pymysql`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "PyMySQL query",
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
            # %s is a placeholder, not string formatting — args go via the second parameter
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
          title: "Never concatenate SQL strings",
          value:
            "Splicing user input directly into SQL is the root of SQL injection — an attacker can use it to read or even wipe the entire database. Every language's driver provides placeholders (? or %s); pass parameters separately and the driver will handle escaping.",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "Pitfalls and tuning",
      navHint: "Slow queries · Locks · Troubleshooting",
      title: "Things to know before going live",
      intro: "You rarely see these issues in local development, but once data volume and concurrency grow, you'll almost certainly run into them.",
      blocks: [
        {
          kind: "code",
          lang: "sql",
          caption: "Troubleshooting commands",
          code: `SHOW PROCESSLIST;              -- Current connections and running queries; find stuck statements
SHOW ENGINE INNODB STATUS;     -- Deadlock info is in the LATEST DETECTED DEADLOCK section

SHOW VARIABLES LIKE 'slow_query_log';     -- Whether the slow query log is enabled
SHOW VARIABLES LIKE 'long_query_time';    -- Threshold (seconds) for what counts as a slow query
SHOW VARIABLES LIKE 'max_connections';    -- Max connection count

-- See how much space each table uses
SELECT table_name,
       ROUND((data_length + index_length) / 1024 / 1024, 2) AS mb
FROM information_schema.tables
WHERE table_schema = 'demo'
ORDER BY mb DESC;`,
        },
        {
          kind: "table",
          head: ["Problem", "Symptom", "Remedy"],
          rows: [
            [
              "N+1 queries",
              "Fetch a list, then loop through it querying details one by one, producing many small queries",
              "Switch to a single JOIN, or batch-fetch with IN",
            ],
            [
              "Slow deep pagination",
              "LIMIT 20 OFFSET 1000000 gets slower the further you page",
              "Use \"last max id\" cursor pagination: WHERE id < ? ORDER BY id DESC LIMIT 20",
            ],
            [
              "Connection exhaustion",
              "Reports Too many connections",
              "Use a connection pool with a cap; check for connections that aren't being closed",
            ],
            [
              "Deadlock",
              "Reports Deadlock found when trying to get lock",
              "Have all transactions access tables and rows in the same order; keep transactions short",
            ],
            [
              "Large transactions",
              "A transaction updates tens of thousands of rows, holding table locks for a long time",
              "Split into smaller batches — a few hundred to a few thousand rows per commit",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Schema changes on large tables are dangerous",
          value:
            "ALTER TABLE on a large table can lock it for minutes or hours, blocking all business traffic. In production, use online schema-change tools like pt-online-schema-change or gh-ost, or run the change during a low-traffic window.",
        },
        {
          kind: "callout",
          tone: "tip",
          title: "How to do it in Zhiyu",
          value:
            "The Overview tab shows real-time metrics like connection count and QPS. Runtime Logs shows startup errors and slow queries. To change parameters like max_connections, edit them in the Config File tab and restart. The Backup & Restore tab lets you snapshot before any risky operation.",
        },
      ],
    },
  ];
}
