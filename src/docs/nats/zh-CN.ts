import type { DocChapter } from "../docTypes";

export function buildNatsDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 NATS",
      navHint: "Subject · 发布订阅",
      title: "NATS 是什么",
      intro:
        "NATS 是一个轻量消息服务器。应用向 Subject 发布消息，正在订阅相同 Subject 的客户端会立即收到消息。",
      blocks: [
        {
          kind: "text",
          value:
            "与 RabbitMQ 和 Kafka 不同，NATS 的设计哲学是「简单到极致」。它没有复杂的路由规则、不需要管理交换机或分区，核心协议只有十几条命令。Server 端是单个可执行文件，启动即用；Client 端支持几十种语言，集成成本极低。它非常适合微服务之间的异步通信、事件驱动架构和命令分发。",
        },
        {
          kind: "text",
          value: "NATS 的通信模式分为两层：",
        },
        {
          kind: "list",
          items: [
            "Core NATS —— 最多投递一次（at-most-once）的发布/订阅。消息发出后如果有订阅者就送达，没有就丢弃。延迟极低，通常在 100 微秒以内。",
            "JetStream —— 构建在 Core NATS 之上的持久化消息流。支持至少投递一次（at-least-once）、消息重放、消费组和工作队列，功能上接近 Kafka 的简化版。",
          ],
        },
        {
          kind: "table",
          head: ["项目", "本地配置", "说明"],
          rows: [
            ["客户端地址", `nats://127.0.0.1:${port}`, "应用连接地址"],
            ["监控地址", "http://127.0.0.1:8222", "智屿读取实时指标"],
            ["认证", "无", "仅监听 127.0.0.1 的开发实例"],
            ["JetStream", "已启用", "需要持久化时可创建 Stream"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "先理解 Subject 再动手",
          value:
            "NATS 里没有队列和交换机的概念，所有消息都发到 Subject 上。Subject 是一个用点分隔的字符串（如 orders.created），订阅时可以用 * 匹配一个分段，用 > 匹配后续全部分段。这种「分段的层次命名」既是地址也是路由，是 NATS 最核心的设计。",
        },
        {
          kind: "table",
          head: ["", "NATS", "RabbitMQ", "Kafka"],
          rows: [
            ["消息模型", "发布/订阅 + 请求/回复", "AMQP 队列交换", "分区日志流"],
            ["消费模式", "最多一次 / JetStream 至少一次", "手动/自动 ACK", "消费者组偏移"],
            ["持久化", "JetStream 提供", "内置持久队列", "核心设计，不可关闭"],
            ["部署复杂度", "单个二进制，1 个端口", "Erlang + 多进程", "Java + ZooKeeper/KRaft"],
            ["典型延迟", "微秒级", "毫秒级", "毫秒级（批量提升吞吐）"],
            ["适合场景", "服务内异步解耦、事件广播", "任务队列、复杂路由", "高吞吐日志、流处理"],
          ],
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "发布 / 订阅",
      title: "发布第一条消息并收到它",
      intro:
        "智屿已经把 NATS 装好并启动，你不需要自己下载或配置。消息调试页面可以直接完成发布和接收。",
      blocks: [
        {
          kind: "list",
          items: [
            "在「概览」标签页确认状态是「运行中」。",
            "切到「消息调试」标签页，左侧是发布区，右侧是订阅区。",
            "在订阅区输入 Subject（比如 test.hello），点击「开始等待」。",
            "在发布区输入同样的 Subject 和一段消息内容，点击「发布」。",
            "右侧立即收到消息，展示 Payload 和耗时。",
          ],
        },
        {
          kind: "text",
          value:
            "如果想用命令行，系统里的 nats 客户端可以这样连（智屿安装的二进制在 ~/.devbox/installations/nats 下）：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "命令行发布与订阅",
          code: `NATS_BIN=~/.devbox/installations/nats/default/bin

# 终端一：订阅 test.hello
$NATS_BIN/nats sub test.hello

# 终端二：发布消息
$NATS_BIN/nats pub test.hello "Hello Zhiyu"

# 终端一会立即打印收到的消息

# 订阅所有 orders 下的二级 Subject
$NATS_BIN/nats sub "orders.*"

# 订阅 orders 下所有深度的 Subject
$NATS_BIN/nats sub "orders.>"`,
        },
        {
          kind: "text",
          value:
            "请求/回复是 Core NATS 另一个重要模式。发布方附带一个 reply Subject，订阅方收到后把结果发回那个地址，相当于异步 RPC：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "请求/回复模式",
          code: `# 发送请求并等待回复（类似 RPC）
$NATS_BIN/nats req "user.get" '{"id":1001}'

# 服务端：监听并回复
$NATS_BIN/nats reply "user.get" '{"name":"张三","age":28}'`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Core NATS 消息默认不持久化",
          value:
            "没有活跃订阅者时，普通 Publish 消息不会被保存。如果你停了订阅端再发消息，消息会直接丢弃。需要持久化、重放或消费确认时，请使用 JetStream（见下一章）。",
        },
      ],
    },

    {
      id: "jetstream",
      navLabel: "JetStream 持久化",
      navHint: "Stream · Consumer",
      title: "需要持久化和重放时用 JetStream",
      intro:
        "JetStream 让 NATS 从「即发即忘」变成「可靠投递」。它把消息持久写入 Stream，消费者从 Stream 里拉取，支持 ACK 确认、重试、按时间重放。",
      blocks: [
        {
          kind: "text",
          value:
            "JetStream 的核心概念只有三个：Stream（消息存储）、Consumer（消费视图）、Subject（入口）。消息先发布到 Subject，Stream 通过 Subject 过滤吸入消息并存盘，Consumer 从 Stream 里按自己的节奏读取。一个 Stream 可以有多个 Consumer，各自独立维护消费进度。",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "创建 Stream 和 Consumer",
          code: `NATS_BIN=~/.devbox/installations/nats/default/bin

# 创建一个 Stream，监控 orders.* Subject
$NATS_BIN/nats str add ORDERS \\
  --subjects "orders.*" \\
  --storage file \\
  --max-age 24h \\
  --replicas 1

# 查看 Stream 信息
$NATS_BIN/nats str info ORDERS

# 创建一个 Pull Consumer
$NATS_BIN/nats con add ORDERS PROCESSOR \\
  --filter "orders.created" \\
  --ack explicit \\
  --max-deliver 3`,
        },
        {
          kind: "code",
          lang: "bash",
          caption: "发布与消费 JetStream 消息",
          code: `# 发布一条 JetStream 消息
$NATS_BIN/nats pub orders.created '{"id":88,"amount":299}' --js

# 拉取消费（一次取 10 条）
$NATS_BIN/nats con next ORDERS PROCESSOR --count 10

# 查看 Stream 状态
$NATS_BIN/nats str report ORDERS`,
        },
        {
          kind: "table",
          head: ["参数", "含义", "建议值"],
          rows: [
            ["--max-age", "消息最长保留时间", "按业务需求：日志类 1d，事件类 7d"],
            ["--max-msgs", "Stream 最多存多少条", "按容量估算，避免耗尽磁盘"],
            ["--max-bytes", "Stream 最多占多少磁盘", "本地开发设 1GB 足够"],
            ["--ack explicit", "手动 ACK，消费完才确认", "需要可靠消费的场景"],
            ["--ack none", "不 ACK，投递就算完成", "允许丢失的场合"],
            ["--max-deliver", "最大投递次数", "重试 3-5 次后转入死信"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "什么时候用 JetStream，什么时候用 Core NATS",
          value:
            "如果你只需要「通知一下其他服务某件事发生了」，用 Core NATS 就够了，延迟更低、配置更少。如果消息不能丢、需要重放或需要多个消费者各自独立消费，则上 JetStream。大多数项目从 Core NATS 起步，遇到持久化需求再开 JetStream 完全来得及。",
        },
      ],
    },

    {
      id: "subjects",
      navLabel: "Subject 设计",
      navHint: "命名 · 通配符",
      title: "Subject 命名是 NATS 里最重要的设计",
      intro:
        "NATS 没有交换机、没有队列绑定，Subject 同时承担了地址和路由的角色。好的命名习惯让系统清晰可维护，坏的命名会让消息流失控。",
      blocks: [
        {
          kind: "text",
          value:
            "Subject 由若干个用点分隔的分段组成，每个分段包含字母、数字和下划线。通常用「领域.实体.事件」的三段式命名：",
        },
        {
          kind: "code",
          lang: "text",
          caption: "推荐的 Subject 命名",
          code: `order.created        # 订单已创建
order.paid           # 订单已支付
user.registered      # 用户已注册
user.updated         # 用户信息已更新
email.sent           # 邮件已发送
system.heartbeat     # 服务心跳`,
        },
        {
          kind: "text",
          value:
            "订阅时用通配符让 NATS 帮你做路由分发，两个通配符的含义不同：",
        },
        {
          kind: "table",
          head: ["通配符", "含义", "示例", "匹配"],
          rows: [
            ["*", "匹配恰好一个分段", `"orders.*"`, "orders.created / orders.paid（不匹配 orders.created.email）"],
            [">", "匹配剩余的全部分段", `"orders.>"`, "orders.created / orders.created.email（全部匹配）"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "通配符只用于订阅，不能用于发布",
          value:
            "发布消息时 Subject 必须是精确字符串，不能带 * 或 >。另外，点分段之间不能有空白字符，Subject 总长度不能超过 256 字节。",
        },
        {
          kind: "code",
          lang: "text",
          caption: "典型的 Subject 分层设计",
          code: `# 按服务拆分
svc.order.*         # 订单服务相关事件
svc.user.*          # 用户服务相关事件
svc.payment.*       # 支付服务相关事件

# 按环境拆分（多环境共用同一个 NATS 时）
prod.order.created  # 生产环境
dev.order.created   # 开发环境

# 命令与事件分开（CQRS 风格）
cmd.order.create    # 命令：创建订单
evt.order.created   # 事件：订单已创建`,
        },
      ],
    },

    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Java · Go · TS · Python",
      title: "在你的项目里连上它",
      intro: `下面的配置都指向本机 nats://127.0.0.1:${port}。关键要点：NATS 客户端自带连接池和自动重连，全局复用一个实例即可；大多数语言库 API 高度一致，学会一种就能触类旁通。`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java",
              lang: "xml",
              caption: "Spring Boot：pom.xml（或直接引入 jnats）",
              code: `<dependency>
  <groupId>io.nats</groupId>
  <artifactId>jnats</artifactId>
  <version>2.20.4</version>
</dependency>`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "发布与订阅",
              code: `import io.nats.client.*;

try (Connection nc = Nats.connect("nats://127.0.0.1:${port}")) {

    // 订阅
    Subscription sub = nc.subscribe("orders.created");
    System.out.println("收到: " + new String(sub.nextMessage(Duration.ofSeconds(10)).getData()));

    // 发布
    nc.publish("orders.created", "{\\"id\\":88,\\"amount\\":299}".getBytes());

    // 请求/回复
    CompletableFuture<Message> reply = nc.request("user.get", "{\\"id\\":1001}".getBytes());
    System.out.println("回复: " + new String(reply.get().getData()));
}`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "安装 nats.go",
              code: `go get github.com/nats-io/nats.go`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "发布与订阅",
              code: `package messenger

import "github.com/nats-io/nats.go"

// 全局复用一个 Conn，它自带重连和连接池
var nc, _ = nats.Connect("nats://127.0.0.1:${port}")

func Publish(subj string, data []byte) error {
    return nc.Publish(subj, data)
}

func Subscribe(subj string, handler func([]byte)) (*nats.Subscription, error) {
    return nc.Subscribe(subj, func(msg *nats.Msg) {
        handler(msg.Data)
    })
}

// 请求/回复模式
func Request(subj string, data []byte, timeout time.Duration) ([]byte, error) {
    msg, err := nc.Request(subj, data, timeout)
    if err != nil {
        return nil, err
    }
    return msg.Data, nil
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "安装 nats.ws（Node.js / Deno）",
              code: `npm install nats.ws`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "发布与订阅",
              code: `import { connect, StringCodec } from "nats.ws";

// 全局复用一个连接
const nc = await connect({ servers: "127.0.0.1:${port}" });
const sc = StringCodec();

// 订阅
const sub = nc.subscribe("orders.created");
(async () => {
  for await (const m of sub) {
    console.log("收到:", sc.decode(m.data));
  }
})();

// 发布
nc.publish("orders.created", sc.encode(JSON.stringify({ id: 88, amount: 299 })));

// 请求/回复
const reply = await nc.request("user.get", sc.encode(JSON.stringify({ id: 1001 })));
console.log("回复:", sc.decode(reply.data));`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "安装 nats-py",
              code: `pip install nats-py`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "发布与订阅",
              code: `import asyncio
from nats.aio.client import Client as NATS

nc = NATS()

async def main():
    await nc.connect("nats://127.0.0.1:${port}")

    # 订阅
    async def handler(msg):
        print(f"收到: {msg.data.decode()}")

    await nc.subscribe("orders.created", cb=handler)

    # 发布
    await nc.publish("orders.created", b'{"id":88,"amount":299}')

    # 请求/回复
    reply = await nc.request("user.get", b'{"id":1001}', timeout=5)
    print(f"回复: {reply.data.decode()}")

    # 保持运行等待消息
    await asyncio.sleep(60)

asyncio.run(main())`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "别每次发消息都创建新连接",
          value:
            "NATS 客户端连接是长连接，内部已经维护了连接池和自动重连逻辑。每次请求新建一个连接会让服务端连接数暴涨，而且新建连接需要 INFO/CONNECT 握手，延迟远高于复用已有连接。把 Connection 实例做成模块级单例或交给 IOC 容器管理。",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "避坑与调优",
      navHint: "排查 · 常见问题",
      title: "上线前该知道的事",
      intro: "NATS 在本地跑起来太平滑，到了生产环境有几个问题值得提前了解。",
      blocks: [
        {
          kind: "table",
          head: ["问题", "现象", "对策"],
          rows: [
            [
              "消息悄悄丢失",
              "订阅者离线时发布的消息收不到",
              "改 Core NATS 为 JetStream，Stream 会持久化消息直到消费者来取",
            ],
            [
              "慢消费者拖垮服务",
              "监控里 slow_consumers 持续增长",
              "消费者处理太慢时 NATS 会丢弃后续消息；增加消费者实例并行处理",
            ],
            [
              "Subject 命名混乱",
              "服务间用不同的 Subject 格式，集成困难",
              "统一用「领域.实体.事件」三段式，发布和订阅方对好文档",
            ],
            [
              "连接数过多",
              "nats server 报 max_connections",
              "检查是否有客户端每次操作都新建连接；全局复用 Connection",
            ],
            [
              "JetStream 存储满",
              "Stream 无法接收新消息",
              "检查 max-bytes / max-msgs 配置；设置合理的 max-age 自动清理",
            ],
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "排查命令",
          code: `NATS_BIN=~/.devbox/installations/nats/default/bin

# 查看服务端状态
$NATS_BIN/nats server report connections
$NATS_BIN/nats server info

# 列出所有 Stream
$NATS_BIN/nats str ls

# 看某个 Stream 的详情
$NATS_BIN/nats str info ORDERS

# 列出 Consumer
$NATS_BIN/nats con ls ORDERS

# 看 Consumer 积压情况（pending 数字很大说明消费跟不上）
$NATS_BIN/nats con info ORDERS PROCESSOR`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么做",
          value:
            "「概览」标签页展示了连接数、消息吞吐、慢消费者等实时指标；「消息调试」标签页可以快速做 Pub/Sub 调试；「运行日志」能看到服务端启动报错和 JetStream 存储状态；改端口或 JetStream 存储路径在「配置文件」标签页编辑后重启即可。",
        },
      ],
    },
  ];
}
