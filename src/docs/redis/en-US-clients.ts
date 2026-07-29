import type { DocChapter } from "../docTypes";

/** Chapters 5-7: language clients, real-world patterns, pitfalls and tuning. */
export function buildClientChapters(port: number): DocChapter[] {
  return [
    {
      id: "clients",
      navLabel: "Language clients",
      navHint: "Java · Go · TS · Python",
      title: "Connect it from your project",
      intro: `Each snippet below points directly at 127.0.0.1:${port} on this machine—copy it into your project and it runs. The common rule: reuse the client instance; do not open a fresh connection for every request.`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java",
              lang: "xml",
              caption: "Spring Boot: add dependency to pom.xml",
              code: `<dependency>
  <groupId>org.springframework.boot</groupId>
  <artifactId>spring-boot-starter-data-redis</artifactId>
</dependency>`,
            },
            {
              label: "Java",
              lang: "yaml",
              caption: "application.yml",
              code: `spring:
  data:
    redis:
      host: 127.0.0.1
      port: ${port}
      database: 0
      timeout: 2s
      lettuce:
        pool:
          max-active: 16   # Maximum pool size
          max-idle: 8
          min-idle: 2`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "Inject RedisTemplate and use it directly",
              code: `@Service
public class UserCache {

    private final StringRedisTemplate redis;

    public UserCache(StringRedisTemplate redis) {
        this.redis = redis;
    }

    public void put(long userId, String json) {
        // Write with a 10-minute TTL
        redis.opsForValue().set("cache:user:" + userId, json, Duration.ofMinutes(10));
    }

    public String get(long userId) {
        return redis.opsForValue().get("cache:user:" + userId);
    }

    public long incrViews(long articleId) {
        return redis.opsForValue().increment("article:" + articleId + ":views");
    }

    public void putProfile(long userId, String name, int age) {
        redis.opsForHash().putAll("user:" + userId,
                Map.of("name", name, "age", String.valueOf(age)));
    }
}`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "Install go-redis",
              code: `go get github.com/redis/go-redis/v9`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "Initialize the client and read/write",
              code: `package cache

import (
    "context"
    "time"

    "github.com/redis/go-redis/v9"
)

// Reuse one Client globally; it has a built-in pool and is concurrency-safe
var rdb = redis.NewClient(&redis.Options{
    Addr:         "127.0.0.1:${port}",
    Password:     "",
    DB:           0,
    PoolSize:     16,
    DialTimeout:  2 * time.Second,
    ReadTimeout:  2 * time.Second,
})

func Put(ctx context.Context, key, val string) error {
    return rdb.Set(ctx, key, val, 10*time.Minute).Err()
}

func Get(ctx context.Context, key string) (string, error) {
    val, err := rdb.Get(ctx, key).Result()
    if err == redis.Nil {
        return "", nil // Key not found—note that this is not an error
    }
    return val, err
}

func IncrViews(ctx context.Context, articleID int64) (int64, error) {
    return rdb.Incr(ctx, fmt.Sprintf("article:%d:views", articleID)).Result()
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "Install ioredis",
              code: `npm install ioredis`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "Node.js read/write",
              code: `import Redis from "ioredis";

// Module-level singleton, reused across the whole process
export const redis = new Redis({
  host: "127.0.0.1",
  port: ${port},
  db: 0,
  maxRetriesPerRequest: 2,
  lazyConnect: false,
});

export async function put(userId: number, json: string): Promise<void> {
  // "EX", 600 means expire after 600 seconds
  await redis.set(\`cache:user:\${userId}\`, json, "EX", 600);
}

export async function get(userId: number): Promise<string | null> {
  return redis.get(\`cache:user:\${userId}\`);
}

export async function incrViews(articleId: number): Promise<number> {
  return redis.incr(\`article:\${articleId}:views\`);
}

export async function putProfile(userId: number, name: string, age: number) {
  await redis.hset(\`user:\${userId}\`, { name, age: String(age) });
}`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "Install redis-py",
              code: `pip install redis`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "Synchronous client",
              code: `import redis

# decode_responses=True makes return values str instead of bytes
pool = redis.ConnectionPool(
    host="127.0.0.1",
    port=${port},
    db=0,
    max_connections=16,
    decode_responses=True,
)

r = redis.Redis(connection_pool=pool)


def put(user_id: int, payload: str) -> None:
    r.set(f"cache:user:{user_id}", payload, ex=600)


def get(user_id: int) -> str | None:
    return r.get(f"cache:user:{user_id}")


def incr_views(article_id: int) -> int:
    return r.incr(f"article:{article_id}:views")`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Two of the most common integration mistakes",
          value:
            "First, creating a new client on every request quickly exhausts connections—the client should be a singleton or managed by the framework. Second, forgetting to set timeouts: once Redis slows down, caller threads pile up and eventually take the whole service with it. Always configure connect and read/write timeouts.",
        },
      ],
    },

    {
      id: "patterns",
      navLabel: "Real-world patterns",
      navHint: "Cache · Lock · Rate limit",
      title: "The four most common playbooks",
      intro:
        "In real projects Redis usage boils down to these few patterns. Understand them and most requirements fall into place.",
      blocks: [
        {
          kind: "text",
          value:
            "The first is Cache-Aside, the mainstream caching pattern: on read, check the cache first; if it misses, load from the database and fill the cache; on write, update the database and delete the cache entry so the next read reloads it.",
        },
        {
          kind: "code",
          lang: "typescript",
          caption: "Cache-Aside",
          code: `async function getUser(id: number) {
  const key = \`cache:user:\${id}\`;

  const cached = await redis.get(key);
  if (cached) return JSON.parse(cached);          // Hit: return directly

  const user = await db.findUser(id);             // Miss: fall back to the database
  if (user) {
    await redis.set(key, JSON.stringify(user), "EX", 600);
  }
  return user;
}

async function updateUser(id: number, patch: Partial<User>) {
  await db.updateUser(id, patch);
  await redis.del(\`cache:user:\${id}\`);            // Update the DB first, then delete the cache
}`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Why delete the cache instead of updating it",
          value:
            "When two requests update concurrently, the \"update cache\" order can end up reversed relative to the \"update database\" order, leaving stale data cached indefinitely. Deleting instead makes the next read reload—simpler and less error-prone.",
        },
        {
          kind: "text",
          value:
            "The second is a distributed lock. When multiple service instances compete for the same resource, use SET NX to claim it—whoever writes successfully owns the lock. On release, verify ownership so you do not accidentally delete someone else's lock:",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "Distributed lock",
          code: `# Acquire: NX only sets if the key is absent; EX 30 is a safety net against deadlocks
SET lock:stock:sku-9527 "uuid-of-this-worker" NX EX 30
# OK means acquired; nil means someone else holds the lock

# Release: use Lua so "compare + delete" happens atomically
EVAL "if redis.call('GET', KEYS[1]) == ARGV[1] then
        return redis.call('DEL', KEYS[1])
      else
        return 0
      end" 1 lock:stock:sku-9527 "uuid-of-this-worker"`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Lock TTL must exceed the work duration",
          value:
            "If the work takes 40 seconds but the lock TTL is 30, the lock expires before the work is done and a second instance enters the critical section. Either set a generous TTL or use a mature client with auto-renewal (Redisson on the JVM, for example).",
        },
        {
          kind: "text",
          value:
            "The third is rate limiting. The simplest fixed-window approach: use \"resource + time window\" as the key, increment it, and set the expiration on the first increment:",
        },
        {
          kind: "code",
          lang: "go",
          caption: "Fixed-window rate limit",
          code: `// Limit each IP to 60 requests per minute
func Allow(ctx context.Context, ip string) (bool, error) {
    key := fmt.Sprintf("rate:%s:%d", ip, time.Now().Unix()/60)

    count, err := rdb.Incr(ctx, key).Result()
    if err != nil {
        return false, err
    }
    if count == 1 {
        // Only set TTL on the first increment; the window auto-clears at the end
        rdb.Expire(ctx, key, time.Minute)
    }
    return count <= 60, nil
}`,
        },
        {
          kind: "text",
          value: "The fourth is not a single pattern but three classic cache failure modes and how to handle them:",
        },
        {
          kind: "table",
          head: ["Problem", "Symptom", "Common mitigation"],
          rows: [
            [
              "Cache penetration",
              "Requests for keys that never exist in the database always hit the database",
              "Cache the empty result with a short TTL, or use a Bloom filter to block them upfront",
            ],
            [
              "Cache breakdown",
              "A hot key expires and a flood of requests hits the database simultaneously",
              "Acquire a distributed lock before falling back to the database so only one request loads and the rest wait",
            ],
            [
              "Cache avalanche",
              "A large batch of keys expires at the same moment and the database is crushed",
              "Add a random offset to TTLs so expirations are spread out",
            ],
          ],
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "Pitfalls and tuning",
      navHint: "Memory · Persistence · Debugging",
      title: "What to know before going to production",
      intro:
        "None of this shows up during local development, but almost all of it hits in production. Knowing it in advance saves a lot of debugging time.",
      blocks: [
        {
          kind: "text",
          value:
            "What happens when memory fills up depends on the eviction policy. For caching, allkeys-lru is a good pick—it automatically evicts the least recently used keys:",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "Memory and eviction",
          code: `CONFIG GET maxmemory-policy    # Show the current policy
CONFIG SET maxmemory 512mb     # Cap memory usage at 512MB
CONFIG SET maxmemory-policy allkeys-lru

INFO memory                    # Detailed memory breakdown
DBSIZE                         # Number of keys in the current database`,
        },
        {
          kind: "table",
          head: ["Policy", "Behavior", "When to use"],
          rows: [
            ["noeviction", "Return an error on writes when memory is full", "Data-store usage where nothing can be lost"],
            ["allkeys-lru", "Evict the least recently used key", "Pure caching—most common"],
            ["volatile-lru", "Evict LRU only among keys with a TTL", "Mixed cache and persistent data"],
            ["allkeys-random", "Evict a random key", "When access probabilities are roughly equal"],
          ],
        },
        {
          kind: "text",
          value:
            "There are two persistence modes and you can enable both. RDB writes periodic in-memory snapshots to a file—small and fast to restore, but data between snapshots is lost. AOF records every write command—smaller loss window but larger file and slower restore. For pure caching many teams simply disable persistence to gain performance.",
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Watch out for big keys and hot keys",
          value:
            "A big key is one whose value is unusually large, or a Hash/List packed with hundreds of thousands of elements—deleting or reading it blocks the single thread for a long time. A hot key is one that is accessed far more often than others, saturating a single node. Split the former; use a local cache or multiple replicas to spread the latter.",
        },
        {
          kind: "text",
          value: "When the service gets slow, these commands usually pinpoint the issue:",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "Debugging",
          code: `INFO stats             # Hit rate: keyspace_hits / keyspace_misses
INFO clients           # Current connection count; useful for spotting leaks
SLOWLOG GET 10         # The 10 most recent slow commands
CLIENT LIST            # Who is connected and what they're doing
MEMORY USAGE some:key  # Bytes used by a single key; handy for finding big keys`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Where to find this in Zhiyu",
          value:
            "The Overview tab already renders memory usage, connection counts, and hit rate as charts. The Logs tab shows startup errors and slow-query records. To change parameters like maxmemory, edit and save in the Config tab, then restart—no need to hunt down the config file path manually.",
        },
      ],
    },
  ];
}
