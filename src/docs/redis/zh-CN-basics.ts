import type { DocChapter } from "../docTypes";

/** 第 1-4 章：概念、上手、数据结构、键管理。 */
export function buildBasicChapters(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 Redis",
      navHint: "它是什么 · 何时该用",
      title: "Redis 是什么",
      intro:
        "Redis 是一个把数据放在内存里的键值数据库。它不只能存字符串，还内置了哈希、列表、集合、有序集合等结构，所以官方把它称作「数据结构服务器」。",
      blocks: [
        {
          kind: "text",
          value:
            "传统数据库把数据写在磁盘上，读取时要经过文件系统和索引查找；Redis 把数据直接放在内存里，一次读写通常在几十微秒内完成。代价是内存比磁盘贵得多，而且进程退出后数据可能丢失，因此 Redis 更适合做「快而可重建」的那一层数据。",
        },
        {
          kind: "text",
          value: "它快的原因主要有三点：",
        },
        {
          kind: "list",
          items: [
            "数据全在内存，省掉了磁盘寻道和页加载。",
            "命令处理是单线程事件循环，没有锁竞争和线程切换开销，同时也保证了单条命令的原子性。",
            "每种数据结构都针对场景做过优化，例如小哈希会用紧凑编码存储，省内存也省 CPU。",
          ],
        },
        {
          kind: "text",
          value: "最常见的用途：",
        },
        {
          kind: "list",
          items: [
            "缓存 —— 把数据库查询结果暂存起来，挡住大部分重复读请求。",
            "会话与登录态 —— 存 Session、Token，天然支持过期时间。",
            "计数与排行榜 —— 浏览量、点赞数、实时榜单。",
            "限流 —— 限制某个用户或接口在单位时间内的调用次数。",
            "分布式锁 —— 让多个服务实例对同一资源串行操作。",
            "轻量队列 —— 用 List 或 Stream 做异步任务、日志管道。",
          ],
        },
        {
          kind: "table",
          head: ["", "Redis", "MySQL / PostgreSQL"],
          rows: [
            ["数据位置", "内存为主，可持久化到磁盘", "磁盘为主，内存做缓存"],
            ["查询方式", "按 key 直接定位，不支持 JOIN", "SQL，支持 JOIN、聚合、事务"],
            ["典型延迟", "微秒级", "毫秒级"],
            ["适合存", "可重建的热数据、临时状态", "需要长期保存的业务数据"],
            ["数据量上限", "受内存大小限制", "受磁盘大小限制"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "不要把 Redis 当作唯一的数据来源",
          value:
            "即使开启了持久化，Redis 在宕机时仍可能丢失最后一小段时间的写入。订单、账务这类不能丢的数据请放在关系型数据库里，Redis 只保存它的副本或衍生状态。另外，单个 value 建议控制在 10KB 以内，大对象会拖慢整个实例。",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "连接 · 第一条命令",
      title: "在智屿里跑通第一条命令",
      intro:
        "智屿已经把 Redis 装好并托管在本地，你不需要自己配置端口或写启动脚本。下面这三步走完，就算是接上了。",
      blocks: [
        {
          kind: "list",
          items: [
            "回到「概览」标签页，确认服务状态是「运行中」；如果是「已停止」，点一下启动。",
            "切到「命令台」标签页，直接输入命令回车，结果会实时打印出来。",
            "想看已有的数据，用「数据浏览」标签页按前缀翻 key，不需要手敲 SCAN。",
          ],
        },
        {
          kind: "text",
          value: "当前这个实例的连接参数如下，应用程序按这个填就能连上：",
        },
        {
          kind: "table",
          head: ["参数", "值", "说明"],
          rows: [
            ["主机", "127.0.0.1", "只监听本机，外部网络访问不到"],
            ["端口", String(port), "如果被占用，可以在「配置文件」标签页改"],
            ["密码", "（空）", "本地开发实例默认不设密码"],
            ["数据库编号", "0", "Redis 默认有 0-15 共 16 个逻辑库"],
            ["连接串", `redis://127.0.0.1:${port}/0`, "大多数客户端库都认这个格式"],
          ],
        },
        {
          kind: "text",
          value: "如果你更习惯用终端，系统自带的 redis-cli 也可以连：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "终端连接",
          code: `redis-cli -h 127.0.0.1 -p ${port}

# 连上之后先探活，正常会回 PONG
127.0.0.1:${port}> PING
PONG`,
        },
        {
          kind: "text",
          value:
            "最基础的四个命令是写入、读取、判断存在和删除。先在命令台里把它们跑一遍，建立手感：",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "增删改查",
          code: `SET user:1001:name "张三"      # 写入一个字符串
GET user:1001:name             # 读出来 -> "张三"
EXISTS user:1001:name          # 存在返回 1，不存在返回 0
DEL user:1001:name             # 删除，返回被删掉的数量

SET captcha:1001 "8823" EX 300 # 写入并设置 300 秒后自动过期
TTL captcha:1001               # 还剩多少秒 -> 300
INCR page:home:views           # 计数器加一，key 不存在时按 0 起算`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "命令不区分大小写",
          value:
            "SET 和 set 效果一样。习惯上命令写大写、key 写小写，读起来更清楚。key 本身是区分大小写的，user:1 和 User:1 是两个不同的键。",
        },
      ],
    },

    {
      id: "datatypes",
      navLabel: "数据结构",
      navHint: "五种核心类型",
      title: "选对数据结构，代码能少一半",
      intro:
        "很多人只把 Redis 当成「能存字符串的缓存」，其实用对结构可以把原本要写在应用里的逻辑直接交给 Redis 完成，而且是原子的。",
      blocks: [
        {
          kind: "table",
          head: ["类型", "存什么", "典型用途"],
          rows: [
            ["String", "一个键对应一段文本或数字", "缓存 JSON、计数器、验证码"],
            ["Hash", "一个键里放多个字段", "对象属性，可单独改某个字段"],
            ["List", "有序可重复，两头进出", "消息队列、最近浏览记录"],
            ["Set", "无序不重复", "标签、去重、共同好友"],
            ["ZSet", "带分数的有序不重复集合", "排行榜、延时队列、按时间索引"],
          ],
        },
        {
          kind: "text",
          value:
            "String 最简单，除了存文本还能当计数器用，INCR 是原子操作，不用担心并发下丢更新：",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "String",
          code: `SET article:42 '{"title":"入门指南","author":"李四"}'
GET article:42
SETEX session:abc 1800 "user-1001"   # 存入并设置 30 分钟过期

INCR   article:42:views      # +1
INCRBY article:42:views 10   # +10
DECR   stock:sku-9527        # -1，可用于扣库存`,
        },
        {
          kind: "text",
          value:
            "Hash 适合存对象。相比把整个 JSON 塞进 String，它可以只读或只改其中一个字段，省带宽也省序列化开销：",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "Hash",
          code: `HSET user:1001 name "张三" age 28 city "杭州"
HGET user:1001 name          # 只取一个字段
HMGET user:1001 name city    # 取多个字段
HGETALL user:1001            # 取全部字段（字段多时慎用）
HINCRBY user:1001 age 1      # 对某个字段做原子加法
HDEL user:1001 city          # 删掉一个字段`,
        },
        {
          kind: "text",
          value:
            "List 是一个双向链表，两端插入删除都很快。做简单队列时，一端写入另一端读出即可：",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "List",
          code: `LPUSH queue:mail "任务A"     # 从左侧入队
RPUSH queue:mail "任务B"     # 从右侧入队
RPOP  queue:mail             # 从右侧出队
BRPOP queue:mail 10          # 阻塞出队，最多等 10 秒，适合做消费者

LRANGE history:1001 0 9      # 取最近 10 条
LTRIM  history:1001 0 99     # 只保留前 100 条，超出的丢弃`,
        },
        {
          kind: "text",
          value:
            "Set 自动去重，还能直接做交集、并集、差集，很多「共同关注」「标签筛选」的逻辑一条命令就够：",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "Set",
          code: `SADD  article:42:tags "后端" "缓存" "Redis"
SMEMBERS  article:42:tags       # 列出全部成员
SISMEMBER article:42:tags "缓存" # 是否包含，返回 1 或 0
SCARD     article:42:tags       # 成员个数

SINTER user:1:follow user:2:follow   # 交集：共同关注
SDIFF  user:1:follow user:2:follow   # 差集：我关注但对方没关注`,
        },
        {
          kind: "text",
          value:
            "ZSet 在 Set 的基础上给每个成员加了一个分数，并按分数排序。排行榜、按时间排序的索引都靠它：",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "ZSet",
          code: `ZADD rank:score 980 "张三" 1250 "李四" 760 "王五"
ZINCRBY rank:score 50 "张三"          # 给张三加 50 分

ZREVRANGE rank:score 0 9 WITHSCORES   # 分数从高到低取前 10 名
ZREVRANK  rank:score "张三"            # 张三排第几（从 0 开始）
ZRANGEBYSCORE rank:score 800 1000     # 取分数在 800-1000 之间的成员`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "还有几种进阶结构",
          value:
            "Bitmap 用位存布尔状态，适合签到打卡；HyperLogLog 用极小内存估算去重数量，适合统计 UV；Stream 是功能完整的消息流，支持消费组和 ACK，比 List 更适合正经的消息队列；GEO 可以按经纬度做附近搜索。等基础结构用熟了再看它们。",
        },
      ],
    },

    {
      id: "keyspace",
      navLabel: "键与过期",
      navHint: "命名 · TTL · 批量操作",
      title: "键的命名、过期和批量操作",
      intro:
        "Redis 没有表结构，所有数据平铺在一个键空间里，所以命名规范和过期策略基本就是你的「schema 设计」。",
      blocks: [
        {
          kind: "text",
          value:
            "推荐用冒号分层命名，形如 业务:实体:标识:字段。这样在「数据浏览」里按前缀一搜就能圈出一批相关的键：",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "命名示例",
          code: `user:1001              # 用户 1001 的信息（Hash）
user:1001:follow       # 用户 1001 的关注列表（Set）
order:20240613:88      # 某天的某个订单
cache:article:42       # 文章缓存
lock:stock:sku-9527    # 分布式锁
rate:login:192.168.1.5 # 限流计数`,
        },
        {
          kind: "text",
          value:
            "缓存类的键一定要设过期时间，否则内存只增不减，迟早被淘汰策略随机清掉或者直接把实例撑满：",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "过期控制",
          code: `SET  cache:article:42 "..." EX 600   # 写入时直接带 600 秒过期
EXPIRE cache:article:42 600          # 给已存在的键补设过期
TTL    cache:article:42              # 剩余秒数；-1 表示永不过期，-2 表示键不存在
PERSIST cache:article:42             # 去掉过期时间，变成永久`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "线上永远不要执行 KEYS *",
          value:
            "KEYS 会一次性遍历整个键空间，而 Redis 是单线程的，键多的时候这条命令会把整个实例卡住，所有请求一起超时。要遍历请用 SCAN，它分批返回，不会阻塞。同理，FLUSHALL 和 FLUSHDB 会清空数据，在生产环境应该禁用掉。",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "安全遍历",
          code: `# 从游标 0 开始，每次约取 100 个匹配的键
SCAN 0 MATCH "cache:article:*" COUNT 100
# 返回值第一行是下一次的游标，游标回到 0 表示遍历结束

HSCAN user:1001 0            # 遍历 Hash 的字段
SSCAN article:42:tags 0      # 遍历 Set 的成员`,
        },
        {
          kind: "text",
          value:
            "一次要发很多条命令时，用管道把它们打包发送，可以把 N 次网络往返压成 1 次，吞吐提升非常明显。如果需要「要么全做要么全不做」，用事务把命令包起来：",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "事务",
          code: `MULTI                  # 开启事务，后续命令先入队不执行
INCR order:count
HSET order:88 status "paid"
EXEC                   # 一次性按顺序执行，中途不会插入别的客户端命令`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "复杂逻辑交给 Lua",
          value:
            "Redis 的事务不支持「读到结果再决定下一步」。如果需要判断后再写（比如扣库存前先检查够不够），把逻辑写成 Lua 脚本用 EVAL 执行——整个脚本在服务端原子运行，中途不会被其他命令打断。",
        },
      ],
    },
  ];
}
