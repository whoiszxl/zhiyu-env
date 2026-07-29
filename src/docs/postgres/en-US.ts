import type { DocChapter } from "../docTypes";

/** PostgreSQL usage documentation. */
export function buildPostgresDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "Meet PostgreSQL",
      navHint: "Features · Differences from MySQL",
      title: "What is PostgreSQL",
      intro:
        "PostgreSQL is an open-source relational database known for standards compliance and a rich feature set. It uses tables and SQL like any other, but goes further than MySQL in data types, index variety, and extensibility.",
      blocks: [
        {
          kind: "text",
          value:
            "If MySQL positions itself as \"good enough, fast enough, mature ecosystem\", PostgreSQL positions itself as \"rigorous, capable, ready for complex work\". It follows the SQL standard more closely, ships with rich built-in types like JSONB, arrays, ranges, and geographic types, and can be turned into a time-series or vector database through extensions.",
        },
        {
          kind: "text",
          value: "A few things worth noting compared to MySQL:",
        },
        {
          kind: "list",
          items: [
            "JSONB is a native binary JSON type. It can be indexed and queried by path, so a single table can enjoy both structured and semi-structured data.",
            "Window functions, CTEs (WITH clauses), and materialized views make data analysis much easier.",
            "Rich index types: beyond the usual B-tree, there are GIN for full-text and JSONB, and GiST for range and geographic data.",
            "The schema layer lets you do logical isolation inside a single database, without creating multiple databases.",
            "Writes use multi-version concurrency control, so reads do not block writes and writes do not block reads.",
          ],
        },
        {
          kind: "table",
          head: ["", "PostgreSQL", "MySQL"],
          rows: [
            ["Auto-increment primary key", "GENERATED ALWAYS AS IDENTITY or SERIAL", "AUTO_INCREMENT"],
            ["String concatenation", "|| operator", "CONCAT() function"],
            ["Case sensitivity", "Identifiers folded to lowercase, string comparisons are case-sensitive", "Table name case depends on the OS, string comparisons are case-insensitive by default"],
            ["Pagination", "LIMIT 20 OFFSET 40", "LIMIT 20 OFFSET 40 (identical syntax)"],
            ["JSON", "JSONB, indexable and queryable", "JSON, weaker capabilities"],
            ["Hierarchy", "database → schema → table", "database → table"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Identifiers are folded to lowercase",
          value:
            "If you write CREATE TABLE Users in PostgreSQL, the actual table created is called users. If you use double quotes and write \"Users\", it really is uppercase, and every reference afterwards must include the double quotes. Stick to lowercase with underscores to avoid all this trouble.",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quick start",
      navHint: "Connect · Create tables",
      title: "Connect and create your first table",
      intro: "Zhiyu has already initialized the instance and superuser, so just connect.",
      blocks: [
        {
          kind: "table",
          head: ["Parameter", "Value", "Notes"],
          rows: [
            ["Host", "127.0.0.1", "Listens on localhost only"],
            ["Port", String(port), "Can be changed in the Config File tab"],
            ["Username", "postgres", "Superuser created during initialization"],
            ["Password", "(empty)", "Local development instance trusts local connections by default"],
            ["Default database", "postgres", "Prefer creating a separate business database instead of using this one"],
            ["Connection string", `postgresql://postgres@127.0.0.1:${port}/demo`, "Recognized by most client libraries"],
          ],
        },
        {
          kind: "code",
          lang: "sql",
          caption: "Create database and tables",
          code: `CREATE DATABASE demo ENCODING 'UTF8';

-- Run after switching to the demo database (Zhiyu's SQL console lets you switch directly)
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
          title: "Prefer TIMESTAMPTZ for time columns",
          value:
            "TIMESTAMPTZ stores timezone information, so cross-timezone deployments won't miscalculate; TIMESTAMP has no timezone, effectively pushing the problem up to the application layer. The same goes for money: use NUMERIC instead of REAL or DOUBLE PRECISION.",
        },
        {
          kind: "text",
          value: "If you prefer the terminal, the bundled psql connects like this:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "psql connection",
          code: `psql -h 127.0.0.1 -p ${port} -U postgres -d demo

-- A few common meta-commands once you're in
\\l          -- List all databases
\\dt         -- List tables in the current schema
\\d users    -- Show the structure of the users table
\\q          -- Quit`,
        },
      ],
    },

    {
      id: "sql",
      navLabel: "SQL & signature features",
      navHint: "CRUD · JSONB · Window functions",
      title: "Common SQL and PostgreSQL-specific patterns",
      intro:
        "Basic CRUD is the same as any relational database, so the focus here is on the signature features that noticeably cut down on code.",
      blocks: [
        {
          kind: "code",
          lang: "sql",
          caption: "CRUD",
          code: `INSERT INTO users (name, email, age) VALUES ('张三', 'zhang@demo.com', 28);

-- RETURNING lets you grab the generated id right after insert, saving a round trip
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
            "UPSERT (update if exists, insert otherwise) is expressed with ON CONFLICT in PostgreSQL, and it's used all the time:",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "UPSERT",
          code: `INSERT INTO users (name, email, age)
VALUES ('张三', 'zhang@demo.com', 30)
ON CONFLICT (email)                 -- when the email unique constraint is hit
DO UPDATE SET age = EXCLUDED.age,   -- EXCLUDED refers to the row we tried to insert
              name = EXCLUDED.name;

-- Do nothing on conflict, just skip
INSERT INTO users (name, email) VALUES ('王五', 'zhang@demo.com')
ON CONFLICT (email) DO NOTHING;`,
        },
        {
          kind: "text",
          value:
            "JSONB lets you put flexible fields inside relational tables while still being queryable and indexable. It's great for configs or extension attributes where every consumer has different fields:",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "JSONB",
          code: `UPDATE users
SET profile = '{"city":"杭州","vip":true,"score":88}'
WHERE id = 1;

-- ->  returns JSON, ->> returns text
SELECT profile -> 'city'    AS city_json,
       profile ->> 'city'   AS city_text
FROM users WHERE id = 1;

-- @> tests containment; this is the most common way to filter JSONB
SELECT * FROM users WHERE profile @> '{"vip": true}';

-- Fetch a value by nested path
SELECT profile #>> '{address,street}' FROM users;

-- Create a GIN index on JSONB so @> queries can use it
CREATE INDEX idx_users_profile ON users USING GIN (profile);`,
        },
        {
          kind: "text",
          value:
            "Window functions let you do grouped aggregations without collapsing rows, which is a huge time-saver for rankings, period-over-period comparisons, and picking top-N per group:",
        },
        {
          kind: "code",
          lang: "sql",
          caption: "Window functions",
          code: `-- Rank each user's orders by amount within the group
SELECT
  user_id,
  id AS order_id,
  amount,
  ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY amount DESC) AS rn,
  SUM(amount)  OVER (PARTITION BY user_id)                      AS user_total
FROM orders;

-- Combine with a CTE to pick the highest-amount order per user
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
          caption: "Arrays and joins",
          code: `-- Writing and querying array columns
UPDATE users SET tags = ARRAY['后端', 'Go'] WHERE id = 1;
SELECT * FROM users WHERE '后端' = ANY (tags);

-- JOIN syntax is the same as standard SQL
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
      navLabel: "Indexes & performance",
      navHint: "Index types · Query plans",
      title: "Make queries fast",
      intro:
        "PostgreSQL offers more index types than most databases, and picking the right one is more effective than blindly adding indexes.",
      blocks: [
        {
          kind: "table",
          head: ["Index type", "Best for", "Example scenario"],
          rows: [
            ["B-tree (default)", "Equality and range comparisons", "WHERE status = ? / created_at > ?"],
            ["GIN", "Containment, multi-value columns", "JSONB @> queries, arrays, full-text search"],
            ["GiST", "Geometric and range types", "Geographic locations, overlapping time intervals"],
            ["BRIN", "Naturally ordered columns on very large tables", "Log tables appended in time order"],
          ],
        },
        {
          kind: "code",
          lang: "sql",
          caption: "Create indexes",
          code: `CREATE INDEX idx_orders_status ON orders (status);
CREATE INDEX idx_orders_user_status ON orders (user_id, status);
CREATE INDEX idx_users_profile ON users USING GIN (profile);

-- Partial index: only index the rows you care about, keeping it smaller
CREATE INDEX idx_orders_pending ON orders (created_at) WHERE status = 'pending';

-- Expression index: lets functional predicates use an index
CREATE INDEX idx_users_lower_email ON users (LOWER(email));

-- In production add CONCURRENTLY so building the index won't lock the table for long
CREATE INDEX CONCURRENTLY idx_orders_created ON orders (created_at);`,
        },
        {
          kind: "code",
          lang: "sql",
          caption: "Query plans",
          code: `-- EXPLAIN gives estimates only; adding ANALYZE actually runs the query and reports real timings
EXPLAIN ANALYZE
SELECT * FROM orders WHERE user_id = 1 AND status = 'paid';

-- How to read the output:
--   Seq Scan   sequential scan, seeing it on a large table means the index wasn't used
--   Index Scan the index was used, usually what you want
--   actual time real elapsed time; comparing with rows exposes estimation errors
--   Rows Removed by Filter large values mean the filter wasn't pushed into the index`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Keep autovacuum healthy",
          value:
            "PostgreSQL doesn't reclaim space immediately on update or delete; it leaves dead tuples behind for autovacuum to clean up in the background. If it's disabled or can't keep up with the write rate, tables keep growing and queries slow down. Use pg_stat_user_tables to inspect n_dead_tup and the last vacuum time.",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "Language clients",
      navHint: "Java · Go · TS · Python",
      title: "Connecting from your project",
      intro: `The configurations below all point to 127.0.0.1:${port} on the local machine; substitute your own database name. Common principles: use a connection pool, use parameterized queries.`,
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
              caption: "JdbcTemplate query",
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

    // Pair RETURNING with queryForObject to get the new id right after insert
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
              caption: "Install the driver",
              code: `go get github.com/jackc/pgx/v5`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "pgx connection pool",
              code: `package store

import (
    "context"
    "github.com/jackc/pgx/v5/pgxpool"
)

var pool *pgxpool.Pool

func Init(ctx context.Context) error {
    dsn := "postgres://postgres@127.0.0.1:${port}/demo?sslmode=disable&pool_max_conns=10"

    var err error
    // Pool is concurrency-safe; share a single global instance
    pool, err = pgxpool.New(ctx, dsn)
    if err != nil {
        return err
    }
    return pool.Ping(ctx)
}

func FindByMinAge(ctx context.Context, minAge int) ([]User, error) {
    // pgx uses $1 $2 as placeholders, not ?
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
              caption: "Install the driver",
              code: `npm install pg`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "node-postgres connection pool",
              code: `import { Pool } from "pg";

// Module-level singleton, reused across the whole process
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
  // Placeholders are $1 $2; parameters go in the second array
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
              caption: "Install the driver",
              code: `pip install "psycopg[binary,pool]"`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "psycopg3 connection pool",
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
          title: "Watch out for placeholder syntax",
          value:
            "PostgreSQL's native placeholders are the numbered form $1, $2 (used by pgx and node-postgres), while JDBC uses ? and psycopg uses %s. Mixing them up produces a syntax error immediately. Either way, never build parameters by string concatenation.",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "Pitfalls & tuning",
      navHint: "Bloat · Connections · Troubleshooting",
      title: "What you should know before going live",
      intro: "PostgreSQL has a few gotchas that differ from MySQL and are worth knowing up front.",
      blocks: [
        {
          kind: "code",
          lang: "sql",
          caption: "Diagnostic queries",
          code: `-- Currently running queries; find ones taking too long
SELECT pid, state, now() - query_start AS duration, query
FROM pg_stat_activity
WHERE state != 'idle'
ORDER BY duration DESC;

-- Kill a stuck query (try cancel first; if that fails, terminate)
SELECT pg_cancel_backend(12345);
SELECT pg_terminate_backend(12345);

-- Table bloat and vacuum status
SELECT relname, n_live_tup, n_dead_tup, last_autovacuum
FROM pg_stat_user_tables
ORDER BY n_dead_tup DESC;

-- Disk usage per table
SELECT relname, pg_size_pretty(pg_total_relation_size(relid)) AS size
FROM pg_catalog.pg_statio_user_tables
ORDER BY pg_total_relation_size(relid) DESC;`,
        },
        {
          kind: "table",
          head: ["Problem", "Symptom", "What to do"],
          rows: [
            [
              "Table bloat",
              "Table keeps growing after many updates and deletes; queries slow down",
              "Confirm autovacuum is on; if necessary run VACUUM FULL (locks the table)",
            ],
            [
              "Idle transactions never committing",
              "Many idle in transaction rows in pg_stat_activity",
              "The application must commit or rollback when done; set idle_in_transaction_session_timeout",
            ],
            [
              "Out of connections",
              "Errors like too many clients already",
              "Each PostgreSQL connection is a separate process, more expensive than MySQL; always use a connection pool",
            ],
            [
              "Slow deep pagination",
              "The larger OFFSET grows, the slower it gets",
              "Switch to keyset pagination: WHERE id < ? ORDER BY id DESC LIMIT 20",
            ],
            [
              "Stale statistics",
              "The planner picks the wrong index",
              "Run ANALYZE manually to refresh the optimizer's statistics",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "How to do it in Zhiyu",
          value:
            "The Overview tab shows connection counts and live metrics; Runtime Logs surfaces startup errors; parameters like shared_buffers and max_connections can be edited in the Config File tab and take effect after a restart; before any risky operation, consider taking a snapshot from the Backup & Restore tab.",
        },
      ],
    },
  ];
}
