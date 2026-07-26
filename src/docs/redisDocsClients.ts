import type { DocChapter } from "./docTypes";

/** 第 5-7 章：各语言接入、实战模式、避坑建议。 */
export function buildClientChapters(port: number): DocChapter[] {
  return [
    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Java · Go · TS · Python",
      title: "在你的项目里连上它",
      intro: `下面每段代码都直接指向本机的 127.0.0.1:${port}，复制到项目里就能跑。共同的要点是：客户端一定要复用，不要每次请求都新建连接。`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java",
              lang: "xml",
              caption: "Spring Boot：pom.xml 加依赖",
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
          max-active: 16   # 连接池最大连接数
          max-idle: 8
          min-idle: 2`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "注入 RedisTemplate 直接用",
              code: `@Service
public class UserCache {

    private final StringRedisTemplate redis;

    public UserCache(StringRedisTemplate redis) {
        this.redis = redis;
    }

    public void put(long userId, String json) {
        // 写入并设置 10 分钟过期
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
              caption: "安装 go-redis",
              code: `go get github.com/redis/go-redis/v9`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "初始化客户端并读写",
              code: `package cache

import (
    "context"
    "time"

    "github.com/redis/go-redis/v9"
)

// 全局复用一个 Client，它内部自带连接池，并发安全
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
        return "", nil // 键不存在，注意这不是错误
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
              caption: "安装 ioredis",
              code: `npm install ioredis`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "Node.js 端读写",
              code: `import Redis from "ioredis";

// 模块级单例，整个进程复用
export const redis = new Redis({
  host: "127.0.0.1",
  port: ${port},
  db: 0,
  maxRetriesPerRequest: 2,
  lazyConnect: false,
});

export async function put(userId: number, json: string): Promise<void> {
  // "EX", 600 表示 600 秒后过期
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
              caption: "安装 redis-py",
              code: `pip install redis`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "同步客户端",
              code: `import redis

# decode_responses=True 让返回值直接是 str 而不是 bytes
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
          title: "两个最常见的接入错误",
          value:
            "一是每次请求都 new 一个客户端，连接数很快耗尽——客户端应该做成单例或交给框架管理。二是不设超时，Redis 一旦响应变慢，调用方线程会全部堆积，最终拖垮整个服务，务必配上连接超时和读写超时。",
        },
      ],
    },

    {
      id: "patterns",
      navLabel: "实战模式",
      navHint: "缓存 · 锁 · 限流",
      title: "四个最常用的套路",
      intro:
        "真实项目里 Redis 的用法基本就集中在这几个模式上，理解了它们，大部分需求都能套。",
      blocks: [
        {
          kind: "text",
          value:
            "第一个是缓存旁路（Cache-Aside），也是最主流的缓存写法：读的时候先查缓存，没有再查数据库并回填；写的时候更新完数据库直接删掉缓存，让下次读重新加载。",
        },
        {
          kind: "code",
          lang: "typescript",
          caption: "缓存旁路",
          code: `async function getUser(id: number) {
  const key = \`cache:user:\${id}\`;

  const cached = await redis.get(key);
  if (cached) return JSON.parse(cached);          // 命中，直接返回

  const user = await db.findUser(id);             // 未命中，回源数据库
  if (user) {
    await redis.set(key, JSON.stringify(user), "EX", 600);
  }
  return user;
}

async function updateUser(id: number, patch: Partial<User>) {
  await db.updateUser(id, patch);
  await redis.del(\`cache:user:\${id}\`);            // 先改库，再删缓存
}`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "为什么是删缓存而不是改缓存",
          value:
            "两个请求并发更新时，「改缓存」的顺序可能和「改数据库」的顺序相反，导致缓存里留下旧值且长期不失效。直接删掉，让下一次读请求重新加载，逻辑简单且不容易出错。",
        },
        {
          kind: "text",
          value:
            "第二个是分布式锁。多个服务实例要抢同一个资源时，用 SET NX 抢占，谁设置成功谁拿到锁。释放时必须校验持有者，避免误删别人的锁：",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "分布式锁",
          code: `# 抢锁：NX 表示只在键不存在时设置，EX 30 是防止死锁的兜底过期
SET lock:stock:sku-9527 "uuid-of-this-worker" NX EX 30
# 返回 OK 表示抢到，返回 nil 表示锁已被别人持有

# 释放锁：用 Lua 保证「比对 + 删除」是原子的
EVAL "if redis.call('GET', KEYS[1]) == ARGV[1] then
        return redis.call('DEL', KEYS[1])
      else
        return 0
      end" 1 lock:stock:sku-9527 "uuid-of-this-worker"`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "锁的过期时间要大于业务耗时",
          value:
            "如果业务执行 40 秒而锁只有 30 秒，锁会在业务没做完时自动释放，第二个实例就会同时进入临界区。要么把过期时间设得足够宽裕，要么用带自动续期的成熟客户端（Java 可以用 Redisson）。",
        },
        {
          kind: "text",
          value:
            "第三个是限流。最简单的固定窗口做法：以「资源 + 时间窗」为 key 计数，第一次计数时设置过期时间：",
        },
        {
          kind: "code",
          lang: "go",
          caption: "固定窗口限流",
          code: `// 限制单个 IP 每分钟最多 60 次请求
func Allow(ctx context.Context, ip string) (bool, error) {
    key := fmt.Sprintf("rate:%s:%d", ip, time.Now().Unix()/60)

    count, err := rdb.Incr(ctx, key).Result()
    if err != nil {
        return false, err
    }
    if count == 1 {
        // 第一次计数时才设过期，窗口结束自动清理
        rdb.Expire(ctx, key, time.Minute)
    }
    return count <= 60, nil
}`,
        },
        {
          kind: "text",
          value: "第四个不是单一写法，而是缓存的三个经典故障和对应做法：",
        },
        {
          kind: "table",
          head: ["问题", "现象", "常用对策"],
          rows: [
            [
              "缓存穿透",
              "查一个数据库里根本不存在的 key，每次都打到数据库",
              "把「空结果」也缓存起来并设短过期；或用布隆过滤器提前挡掉",
            ],
            [
              "缓存击穿",
              "某个热点 key 恰好过期，大量请求同时回源",
              "回源前先抢一把分布式锁，只放一个请求去查库，其余等待",
            ],
            [
              "缓存雪崩",
              "大批 key 在同一时刻集中过期，数据库瞬间被压垮",
              "给过期时间加一个随机偏移量，把失效时间打散",
            ],
          ],
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "避坑与调优",
      navHint: "内存 · 持久化 · 排查",
      title: "上线前该知道的事",
      intro:
        "本地开发时这些问题都不明显，但换到生产环境几乎都会遇到，提前知道能省很多排查时间。",
      blocks: [
        {
          kind: "text",
          value:
            "内存满了之后 Redis 怎么办，取决于淘汰策略。做缓存用途时建议设置成 allkeys-lru，让它自动淘汰最久没用过的键：",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "内存与淘汰",
          code: `CONFIG GET maxmemory-policy    # 查看当前策略
CONFIG SET maxmemory 512mb     # 限制最多用 512MB
CONFIG SET maxmemory-policy allkeys-lru

INFO memory                    # 查看内存占用明细
DBSIZE                         # 当前库有多少个键`,
        },
        {
          kind: "table",
          head: ["策略", "行为", "适用场景"],
          rows: [
            ["noeviction", "内存满了直接对写命令报错", "当数据存储用，不能丢数据"],
            ["allkeys-lru", "淘汰最久未使用的键", "纯缓存场景，最常用"],
            ["volatile-lru", "只在设了过期时间的键里淘汰", "缓存和持久数据混用"],
            ["allkeys-random", "随机淘汰", "各键访问概率差不多时"],
          ],
        },
        {
          kind: "text",
          value:
            "持久化有两种方式，可以同时开启。RDB 是定时把内存快照写成一个文件，体积小、恢复快，但两次快照之间的数据会丢；AOF 记录每一条写命令，丢失窗口小，但文件更大、恢复更慢。对缓存用途来说，很多团队会直接关掉持久化换取性能。",
        },
        {
          kind: "callout",
          tone: "warn",
          title: "留意大 key 和热 key",
          value:
            "大 key 指单个 value 特别大，或者一个 Hash / List 里塞了几十万个元素——删除或读取它会长时间阻塞单线程。热 key 指某一个键的访问量远超其他键，会把单个节点打满。前者用拆分解决，后者用本地缓存或多副本分摊。",
        },
        {
          kind: "text",
          value: "服务变慢时，这几条命令基本能定位问题：",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "排查",
          code: `INFO stats             # 命中率：keyspace_hits / keyspace_misses
INFO clients           # 当前连接数，排查连接泄漏
SLOWLOG GET 10         # 最近 10 条慢命令
CLIENT LIST            # 谁连着，各自在干什么
MEMORY USAGE some:key  # 单个键占多少字节，用来找大 key`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么看",
          value:
            "「概览」标签页已经把内存占用、连接数、命中率这些指标做成了图表；「运行日志」可以看启动报错和慢查询记录；想改 maxmemory 这类参数，直接在「配置文件」标签页编辑保存后重启即可，不用手动找配置文件路径。",
        },
      ],
    },
  ];
}
