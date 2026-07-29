import type { DocChapter } from "../docTypes";

/** Chapters 1-4: concepts, getting started, data structures, key management. */
export function buildBasicChapters(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "Meet Redis",
      navHint: "What it is · When to use it",
      title: "What is Redis",
      intro:
        "Redis is a key-value database that keeps data in memory. It stores more than just strings—it also has built-in hashes, lists, sets, sorted sets, and other structures, which is why the project calls itself a \"data structure server.\"",
      blocks: [
        {
          kind: "text",
          value:
            "Traditional databases write data to disk and go through the file system plus index lookups on read. Redis keeps data directly in memory, so a single read or write typically finishes within tens of microseconds. The trade-off is that memory is much more expensive than disk, and data may be lost when the process exits, so Redis is better suited to a \"fast and rebuildable\" layer of data.",
        },
        {
          kind: "text",
          value: "It is fast for three main reasons:",
        },
        {
          kind: "list",
          items: [
            "All data lives in memory, avoiding disk seeks and page loads.",
            "Command processing runs on a single-threaded event loop with no lock contention or thread-switching overhead, which also guarantees the atomicity of a single command.",
            "Each data structure is optimized for its use case—for example, small hashes use a compact encoding that saves both memory and CPU.",
          ],
        },
        {
          kind: "text",
          value: "The most common use cases:",
        },
        {
          kind: "list",
          items: [
            "Caching — buffer database query results to absorb most repeated reads.",
            "Sessions and login state — store sessions and tokens, with natural support for expiration.",
            "Counters and leaderboards — page views, likes, real-time rankings.",
            "Rate limiting — cap how many times a user or endpoint can be called per unit of time.",
            "Distributed locks — serialize operations on the same resource across multiple service instances.",
            "Lightweight queues — use List or Stream for async tasks and log pipelines.",
          ],
        },
        {
          kind: "table",
          head: ["", "Redis", "MySQL / PostgreSQL"],
          rows: [
            ["Data location", "Primarily in memory, can be persisted to disk", "Primarily on disk, memory used as cache"],
            ["Query style", "Direct lookup by key, no JOIN support", "SQL, supports JOIN, aggregation, transactions"],
            ["Typical latency", "Microseconds", "Milliseconds"],
            ["Good for", "Rebuildable hot data, transient state", "Long-lived business data"],
            ["Data size limit", "Bounded by memory size", "Bounded by disk size"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Do not treat Redis as your only source of truth",
          value:
            "Even with persistence enabled, Redis can still lose the most recent slice of writes on a crash. Data that cannot be lost—orders, accounting—should live in a relational database, with Redis holding only a copy or derived state. Also, keep each value under about 10KB; large objects slow down the whole instance.",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quick start",
      navHint: "Connect · Your first command",
      title: "Run your first command in Zhiyu",
      intro:
        "Zhiyu has Redis installed and managed locally—you do not have to configure ports or write startup scripts. Follow these three steps and you are connected.",
      blocks: [
        {
          kind: "list",
          items: [
            "Return to the Overview tab and confirm that the service status is Running. If it says Stopped, click Start.",
            "Switch to the Console tab, type a command, and press Enter—results are printed in real time.",
            "To browse existing data, use the Data Browser tab to page through keys by prefix; no need to type SCAN by hand.",
          ],
        },
        {
          kind: "text",
          value: "Here are the connection parameters for this instance—applications can connect using exactly these values:",
        },
        {
          kind: "table",
          head: ["Parameter", "Value", "Notes"],
          rows: [
            ["Host", "127.0.0.1", "Listens on localhost only; not reachable from external networks"],
            ["Port", String(port), "If the port is taken, you can change it in the Config tab"],
            ["Password", "(empty)", "Local development instances have no password by default"],
            ["Database number", "0", "Redis has 16 logical databases (0-15) by default"],
            ["Connection string", `redis://127.0.0.1:${port}/0`, "Most client libraries recognize this format"],
          ],
        },
        {
          kind: "text",
          value: "If you prefer a terminal, the bundled redis-cli also works:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Terminal connection",
          code: `redis-cli -h 127.0.0.1 -p ${port}

# Once connected, ping first—you should get PONG back
127.0.0.1:${port}> PING
PONG`,
        },
        {
          kind: "text",
          value:
            "The four most basic commands are write, read, existence check, and delete. Run them in the console first to get a feel for them:",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "CRUD basics",
          code: `SET user:1001:name "Alice"     # Write a string
GET user:1001:name             # Read it back -> "Alice"
EXISTS user:1001:name          # Returns 1 if it exists, 0 otherwise
DEL user:1001:name             # Delete; returns the number of keys removed

SET captcha:1001 "8823" EX 300 # Write and auto-expire after 300 seconds
TTL captcha:1001               # Seconds remaining -> 300
INCR page:home:views           # Increment counter; starts from 0 if key is absent`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Commands are case-insensitive",
          value:
            "SET and set do the same thing. The convention is uppercase commands and lowercase keys—it reads better that way. Keys themselves are case-sensitive: user:1 and User:1 are two different keys.",
        },
      ],
    },

    {
      id: "datatypes",
      navLabel: "Data structures",
      navHint: "Five core types",
      title: "Pick the right structure and cut your code in half",
      intro:
        "Many people treat Redis as \"just a cache for strings,\" but choosing the right structure lets Redis handle logic you would otherwise write in your application—atomically.",
      blocks: [
        {
          kind: "table",
          head: ["Type", "What it stores", "Typical uses"],
          rows: [
            ["String", "One key maps to a piece of text or a number", "Cached JSON, counters, verification codes"],
            ["Hash", "Multiple fields under one key", "Object attributes; edit individual fields"],
            ["List", "Ordered, allows duplicates, push/pop from both ends", "Message queues, recent-view history"],
            ["Set", "Unordered, no duplicates", "Tags, deduplication, mutual friends"],
            ["ZSet", "Sorted, unique members with scores", "Leaderboards, delayed queues, time-indexed data"],
          ],
        },
        {
          kind: "text",
          value:
            "String is the simplest. Beyond storing text it works as a counter—INCR is atomic, so you never lose updates under concurrency:",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "String",
          code: `SET article:42 '{"title":"Getting Started","author":"Bob"}'
GET article:42
SETEX session:abc 1800 "user-1001"   # Write with a 30-minute expiration

INCR   article:42:views      # +1
INCRBY article:42:views 10   # +10
DECR   stock:sku-9527        # -1, useful for stock decrement`,
        },
        {
          kind: "text",
          value:
            "Hash is good for objects. Compared with stuffing whole JSON into a String, it lets you read or update a single field, saving both bandwidth and serialization overhead:",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "Hash",
          code: `HSET user:1001 name "Alice" age 28 city "Hangzhou"
HGET user:1001 name          # Read one field
HMGET user:1001 name city    # Read multiple fields
HGETALL user:1001            # Read every field (use carefully with many fields)
HINCRBY user:1001 age 1      # Atomic increment on a single field
HDEL user:1001 city          # Remove a field`,
        },
        {
          kind: "text",
          value:
            "List is a doubly linked list; push and pop at either end are both fast. For a simple queue, write on one side and read from the other:",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "List",
          code: `LPUSH queue:mail "task-A"    # Enqueue from the left
RPUSH queue:mail "task-B"    # Enqueue from the right
RPOP  queue:mail             # Dequeue from the right
BRPOP queue:mail 10          # Blocking pop, waits up to 10s; good for consumers

LRANGE history:1001 0 9      # Get the latest 10 entries
LTRIM  history:1001 0 99     # Keep only the first 100; drop the rest`,
        },
        {
          kind: "text",
          value:
            "Set deduplicates automatically and supports intersection, union, and difference directly—many \"mutual follows\" or \"tag filter\" scenarios become a one-liner:",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "Set",
          code: `SADD  article:42:tags "backend" "cache" "Redis"
SMEMBERS  article:42:tags       # List all members
SISMEMBER article:42:tags "cache" # Membership check, returns 1 or 0
SCARD     article:42:tags       # Number of members

SINTER user:1:follow user:2:follow   # Intersection: mutual follows
SDIFF  user:1:follow user:2:follow   # Difference: I follow but they don't`,
        },
        {
          kind: "text",
          value:
            "ZSet builds on Set by attaching a score to each member and sorting by score. Leaderboards and time-sorted indexes rely on it:",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "ZSet",
          code: `ZADD rank:score 980 "Alice" 1250 "Bob" 760 "Carol"
ZINCRBY rank:score 50 "Alice"         # Add 50 to Alice's score

ZREVRANGE rank:score 0 9 WITHSCORES   # Top 10 by score, high to low
ZREVRANK  rank:score "Alice"          # Alice's rank (0-based)
ZRANGEBYSCORE rank:score 800 1000     # Members with a score between 800 and 1000`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "A few more advanced structures",
          value:
            "Bitmap stores boolean state as bits—great for check-ins. HyperLogLog estimates cardinality with tiny memory—great for unique-visitor counts. Stream is a full-featured message stream with consumer groups and ACKs, a much better fit than List for real queues. GEO enables nearby search by latitude and longitude. Explore them once you are comfortable with the basics.",
        },
      ],
    },

    {
      id: "keyspace",
      navLabel: "Keys and expiration",
      navHint: "Naming · TTL · Batch ops",
      title: "Key naming, expiration, and batch operations",
      intro:
        "Redis has no table schema—every piece of data sits flat in a single keyspace, so your naming convention and expiration strategy are effectively your \"schema design.\"",
      blocks: [
        {
          kind: "text",
          value:
            "The recommended style is colon-delimited hierarchy: domain:entity:id:field. That way a prefix search in the Data Browser scopes cleanly to a set of related keys:",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "Naming examples",
          code: `user:1001              # Information for user 1001 (Hash)
user:1001:follow       # User 1001's follow list (Set)
order:20240613:88      # A specific order on a specific day
cache:article:42       # Article cache
lock:stock:sku-9527    # Distributed lock
rate:login:192.168.1.5 # Rate-limit counter`,
        },
        {
          kind: "text",
          value:
            "Always set an expiration on cache keys. Otherwise memory only grows and eventually eviction removes them at random—or fills the instance:",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "Expiration control",
          code: `SET  cache:article:42 "..." EX 600   # Write with a 600-second TTL
EXPIRE cache:article:42 600          # Add TTL to an existing key
TTL    cache:article:42              # Seconds remaining; -1 means no TTL, -2 means the key doesn't exist
PERSIST cache:article:42             # Remove TTL, make it permanent`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Never run KEYS * in production",
          value:
            "KEYS scans the entire keyspace in one shot, and Redis is single-threaded—with many keys this command stalls the instance and every request times out at once. Use SCAN, which returns in batches and does not block. Same story for FLUSHALL and FLUSHDB: they wipe data and should be disabled in production.",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "Safe iteration",
          code: `# Start at cursor 0, take roughly 100 matching keys per batch
SCAN 0 MATCH "cache:article:*" COUNT 100
# The first line of the reply is the next cursor; cursor 0 again means the scan is done

HSCAN user:1001 0            # Iterate fields of a Hash
SSCAN article:42:tags 0      # Iterate members of a Set`,
        },
        {
          kind: "text",
          value:
            "When sending many commands at once, use a pipeline to batch them together—N round trips collapse into 1, and throughput jumps noticeably. If you need \"all or nothing\" semantics, wrap the commands in a transaction:",
        },
        {
          kind: "code",
          lang: "redis",
          caption: "Transaction",
          code: `MULTI                  # Start a transaction; subsequent commands are queued, not executed
INCR order:count
HSET order:88 status "paid"
EXEC                   # Run them in order in one shot; no other client's commands are interleaved`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Delegate complex logic to Lua",
          value:
            "Redis transactions do not support \"read a result, then decide the next step.\" When you need to check before writing (for example, verifying stock before decrementing), write the logic as a Lua script and run it with EVAL—the whole script executes atomically on the server without being interrupted by other commands.",
        },
      ],
    },
  ];
}
