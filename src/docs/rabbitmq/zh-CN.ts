import type { DocChapter } from "../docTypes";

export function buildRabbitmqDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 RabbitMQ",
      navHint: "AMQP · 交换机 · 队列",
      title: "RabbitMQ 是什么",
      intro:
        "RabbitMQ 是部署最广泛的开源消息代理。它完整实现了 AMQP 0-9-1 协议，通过交换机（Exchange）和队列（Queue）的灵活组合，能覆盖几乎所有异步通信场景。",
      blocks: [
        {
          kind: "text",
          value:
            "RabbitMQ 的消息模型和 NATS 截然不同。NATS 只有 Subject（主题），发布方直接往某个主题发消息。RabbitMQ 则在发布方和队列之间引入了一层交换机（Exchange）：发布方把消息发给交换机，交换机根据路由规则投递给绑定的队列。这种设计让路由逻辑从代码里抽离到消息代理层面，发布方不用知道消息最终会到哪些队列。",
        },
        {
          kind: "text",
          value: "RabbitMQ 的核心概念：",
        },
        {
          kind: "list",
          items: [
            "Producer（生产者）—— 发送消息的应用。消息先发到 Exchange，而不是直接入队列。",
            "Exchange（交换机）—— 接收消息并路由到队列。有 direct、topic、fanout、headers 四种类型。",
            "Queue（队列）—— 消息的存储和投递单位。消费者从队列中拉取或等待推送。",
            "Binding（绑定）—— 交换机和队列之间的路由规则。通常是 routing key 模式匹配。",
            "Consumer（消费者）—— 接收并处理消息的应用。支持 ACK 确认、拒绝和重入队。",
          ],
        },
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["AMQP", `amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:${port}/`, "应用连接"],
            ["Management", "http://127.0.0.1:15672", "队列与交换机管理"],
            ["Runtime", "Erlang/OTP 27.3.4.6", "随服务安装在用户目录"],
            ["用户名", "zhiyu", "本地开发实例"],
            ["密码", "zhiyu-local-rabbitmq-2026", "本地开发密码"],
          ],
        },
        {
          kind: "table",
          head: ["", "RabbitMQ", "NATS", "Kafka"],
          rows: [
            ["路由模型", "Exchange → Queue → Consumer", "Subject 直接发送", "Topic → Partition → Consumer"],
            ["消息保证", "ACK 确认，手动重试", "Core: 最多一次; JS: 至少一次", "偏移量提交"],
            ["消费模式", "Push 推送或 Pull 拉取", "订阅推送到客户端", "Pull 拉取"],
            ["持久化", "持久队列和消息到磁盘", "JetStream 提供", "核心设计，不可关闭"],
            ["路由能力", "极强：direct/topic/fanout/headers", "Subject 通配符", "Topic 分区"],
            ["管理界面", "内置 Web UI（15672）", "HTTP 监控（8222）", "需独立工具"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "RabbitMQ 的杀手锏是灵活的路由",
          value:
            "如果你只是需要「发一条消息通知其他服务」，NATS 更快更简单。如果你需要「根据消息内容分发到不同的处理链」、「某些消息需要延时投递」、「死信队列处理失败重试」这些复杂的路由逻辑，RabbitMQ 是更好的选择。",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "连接 · 发一条消息",
      title: "连上并收发第一条消息",
      intro:
        "智屿已经把 RabbitMQ 和 Erlang/OTP 装好并启动。「连接与控制台」标签页展示了 AMQP 连接串和 Management UI 地址，可以直接复制到代码中。",
      blocks: [
        {
          kind: "list",
          items: [
            "在「概览」标签页确认状态是「运行中」。",
            "「连接与控制台」标签页展示了 AMQP 地址、Management UI 地址和用户名密码，可以直接复制。",
            "浏览器打开 http://127.0.0.1:15672，用 zhiyu / zhiyu-local-rabbitmq-2026 登录 Management UI，可以在浏览器中看到实时消息流量、连接数和队列状态。",
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "用 rabbitmqctl 和 rabbitmqadmin 操作",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

# 查看集群状态
$RABBIT_HOME/rabbitmqctl status

# 列出所有队列
$RABBIT_HOME/rabbitmqctl list_queues name messages consumers

# 列出所有交换机
$RABBIT_HOME/rabbitmqctl list_exchanges name type durable

# 用 rabbitmqadmin（HTTP API 封装）发布消息
$RABBIT_HOME/rabbitmqadmin declare queue name=hello durable=false
$RABBIT_HOME/rabbitmqadmin publish \\
  routing_key=hello \\
  payload='{"event":"order.created","orderId":88}'

# 获取消息（消费后从队列中删除）
$RABBIT_HOME/rabbitmqadmin get queue=hello requeue=false

# 清空队列
$RABBIT_HOME/rabbitmqadmin purge queue name=hello`,
        },
        {
          kind: "text",
          value:
            "RabbitMQ 可以同时用 AMQP 协议（端口 5672）和 HTTP 协议（端口 15672）操作。AMQP 是应用代码使用的，HTTP 是管理和调试用的。上面的 rabbitmqadmin 就是通过 HTTP API 操作的，适合在「JSON 命令台」里用。",
        },
      ],
    },

    {
      id: "exchanges",
      navLabel: "交换机类型",
      navHint: "Direct · Topic · Fanout",
      title: "四种交换机，四种路由策略",
      intro:
        "交换机是 RabbitMQ 最核心的概念。理解了四种交换机类型，就理解了 RabbitMQ 80% 的路由能力。",
      blocks: [
        {
          kind: "table",
          head: ["类型", "路由规则", "典型场景", "示例"],
          rows: [
            [
              "Direct",
              "routing key 完全匹配",
              "点对点任务分发、单服务消费",
              "key=order:paid → Queue: order_paid",
            ],
            [
              "Topic",
              "routing key 模式匹配（* 一阶，# 多阶）",
              "多服务不同消费同一类消息",
              "order.# → Queue: order_all; order.paid.* → Queue: order_paid_payment",
            ],
            [
              "Fanout",
              "忽略 routing key，广播到所有绑定队列",
              "系统广播、缓存失效通知",
              "所有绑定队列都收到",
            ],
            [
              "Headers",
              "根据消息头匹配（忽略 routing key）",
              "多条件复杂路由（较少用）",
              "header: type=email AND priority=high",
            ],
          ],
        },
        {
          kind: "text",
          value:
            "Direct 最直观，交换机和队列之间的 routing key 必须完全一致。适合「一个消息只被一个消费者处理」的场景：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Direct 交换机示例",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

# 声明一个 Direct 交换机
$RABBIT_HOME/rabbitmqadmin declare exchange name=order-events type=direct

# 声明两个队列
$RABBIT_HOME/rabbitmqadmin declare queue name=order.created
$RABBIT_HOME/rabbitmqadmin declare queue name=order.paid

# 绑定：队列和 routing key 一一对应
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=order-events destination=order.created routing_key=order.created
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=order-events destination=order.paid routing_key=order.paid

# 发布到 order.paid 这个 routing key
$RABBIT_HOME/rabbitmqadmin publish exchange=order-events \\
  routing_key=order.paid \\
  payload='{"orderId":88,"amount":299}'

# 验证：只有 order.paid 队列收到消息
$RABBIT_HOME/rabbitmqadmin get queue=order.paid requeue=false
$RABBIT_HOME/rabbitmqadmin get queue=order.created requeue=false  # 为空`,
        },
        {
          kind: "text",
          value:
            "Topic 最灵活，routing key 用 . 分层，* 匹配一个分段，# 匹配任意多个分段：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Topic 交换机示例",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

$RABBIT_HOME/rabbitmqadmin declare exchange name=events type=topic

# 三个队列分别关心不同级别的消息
$RABBIT_HOME/rabbitmqadmin declare queue name=all.order
$RABBIT_HOME/rabbitmqadmin declare queue name=order.paid.processor
$RABBIT_HOME/rabbitmqadmin declare queue name=all.events

# order.#  匹配 order 开头的所有消息
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=events destination=all.order routing_key=order.#

# order.paid.*  精确匹配 paid 后缀
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=events destination=order.paid.processor routing_key=order.paid.*

# #  匹配所有消息
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=events destination=all.events routing_key=#

# 现在发一条 order.paid 的消息
$RABBIT_HOME/rabbitmqadmin publish exchange=events \\
  routing_key=order.paid \\
  payload='{"orderId":88,"amount":299}'

# order.paid 同时满足三个 routing key 模式，
# 三个队列都会收到这条消息`,
        },
        {
          kind: "text",
          value:
            "Fanout 最简单，不管 routing key 是什么，消息会被广播到所有绑定的队列。适合「通知所有服务刷新」这类广播场景：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Fanout 交换机示例",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

$RABBIT_HOME/rabbitmqadmin declare exchange name=cache-invalidation type=fanout

# 三个队列分别属于不同服务
$RABBIT_HOME/rabbitmqadmin declare queue name=svc-a-cache
$RABBIT_HOME/rabbitmqadmin declare queue name=svc-b-cache
$RABBIT_HOME/rabbitmqadmin declare queue name=svc-c-cache

# 都绑定到 fanout 交换机（不需要 routing key）
$RABBIT_HOME/rabbitmqadmin declare binding source=cache-invalidation destination=svc-a-cache
$RABBIT_HOME/rabbitmqadmin declare binding source=cache-invalidation destination=svc-b-cache
$RABBIT_HOME/rabbitmqadmin declare binding source=cache-invalidation destination=svc-c-cache

# 发一条消息，三个队列都会收到
$RABBIT_HOME/rabbitmqadmin publish exchange=cache-invalidation \\
  routing_key=ignored \\
  payload='{"type":"cache-clear","keys":["user:*"]}'`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Topic 的 # 通配作用类似 Fanout",
          value:
            "Topic 交换机用 routing_key=# 把队列绑定到交换机，效果和 Fanout 完全一样——所有消息都会进入这个队列。但 Topic 更灵活，因为你还可以同时给同一个交换机用更精确的 routing key 绑定其他队列，Fanout 做不到这一点。",
        },
      ],
    },

    {
      id: "patterns",
      navLabel: "常用模式",
      navHint: "工作队列 · 重试 · 死信",
      title: "四个最实用的消息模式",
      intro:
        "除了四种交换机类型，RabbitMQ 生态里还有几个用得非常普遍的消息模式，理解它们后大多数业务场景都能套用。",
      blocks: [
        {
          kind: "text",
          value:
            "工作队列（Work Queues）：多个消费者共享一个队列，消息分发给空闲的消费者处理。适合耗时任务（图片处理、邮件发送），用 prefetch 控制每个消费者同时处理的消息数量，避免慢消费者积压：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "工作队列",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

# 声明持久队列
$RABBIT_HOME/rabbitmqadmin declare queue name=tasks durable=true

# 多个消费者共享 tasks 队列，RabbitMQ 轮询分发消息
# 在代码里设置 prefetch=1（每次只取一条，处理完再取下一条）
# 这样慢消费者不会积压，快消费者多处理

# 模拟：先放 5 条任务
for i in 1 2 3 4 5; do
  $RABBIT_HOME/rabbitmqadmin publish routing_key=tasks \\
    payload='{"task":"image-resize","id":$i}'
done

# 现在启动两个消费者，消息会轮流分发
# Consumer-1: task 1, 3, 5
# Consumer-2: task 2, 4`,
        },
        {
          kind: "text",
          value:
            "消息确认与重试：消费者处理成功后发送 ACK，处理失败发送 NACK 消息可以重新入队或被丢弃。配合 durable 队列保证即使 RabbitMQ 重启消息也不丢失：",
        },
        {
          kind: "table",
          head: ["ACK 模式", "含义", "行为"],
          rows: [
            ["auto_ack=true", "自动确认，消息一到消费者就删", "简单但消息可能丢失（消费者崩溃）"],
            ["auto_ack=false", "手动确认", "消费者显式 basic_ack 后才删除，最安全"],
            ["basic_nack(requeue=true)", "拒绝并重新入队", "消息放回队首，其他消费者可处理"],
            ["basic_nack(requeue=false)", "拒绝并丢弃", "消息进入死信队列（如果有配置）"],
          ],
        },
        {
          kind: "text",
          value:
            "死信队列（DLX）是 RabbitMQ 最有价值的特性之一。它把无法处理的消息（被拒绝、TTL 过期、队列满）转发到另一个交换机，而不是直接丢弃，让你能事后排查或人工介入：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "死信队列配置",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

# 创建死信交换机
$RABBIT_HOME/rabbitmqadmin declare exchange name=dlx-exchange type=direct

# 创建死信队列
$RABBIT_HOME/rabbitmqadmin declare queue name=dlx-queue
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=dlx-exchange destination=dlx-queue routing_key=failed

# 创建业务队列，指定死信交换机
$RABBIT_HOME/rabbitmqadmin declare queue name=orders \\
  durable=true \\
  arguments='{"x-dead-letter-exchange":"dlx-exchange","x-dead-letter-routing-key":"failed"}'

# 之后：
# 1. 消费者 basic_nack(requeue=false) → 消息被转发到 dlx-exchange
# 2. 消息 TTL 过期 → 也会转发到 dlx-exchange
# 3. 在 dlx-queue 里查看失败消息，排查原因后手动重新发布`,
        },
        {
          kind: "text",
          value:
            "延时队列：利用消息 TTL + 死信队列的组合，实现「N 秒后执行某操作」。比如支付超时关单：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "延时队列（TTL + DLX）",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

# 创建最终处理的交换机
$RABBIT_HOME/rabbitmqadmin declare exchange name=payment-events type=direct

# 创建消费者队列（消息 TTL 过期后到达这里）
$RABBIT_HOME/rabbitmqadmin declare queue name=payment-timeout

# 绑定消费者队列
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=payment-events destination=payment-timeout routing_key=timeout

# 创建延时队列（消息在这里等待 TTL 过期，然后转发到 payment-events）
$RABBIT_HOME/rabbitmqadmin declare queue name=payment-delay-30m \\
  arguments='{
    "x-message-ttl":1800000,
    "x-dead-letter-exchange":"payment-events",
    "x-dead-letter-routing-key":"timeout"
  }'

# 创建订单时，发送消息到延时队列
$RABBIT_HOME/rabbitmqadmin publish routing_key=payment-delay-30m \\
  payload='{"orderId":88,"userId":1001}'

# 30 分钟后，消息自动到达 payment-timeout 队列
# 消费者检查订单状态，如果还是未支付就关闭`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "RPC 也可以用 RabbitMQ",
          value:
            "RabbitMQ 支持 Reply-To 和 CorrelationId 两个属性，可以做请求/回复式的 RPC：客户端发送消息时带上 reply_to 队列名和 correlation_id，服务端处理完后把结果发回 reply_to 队列，客户端通过 correlation_id 匹配请求和响应。这和 NATS 的 request/reply 模式原理一样，但 RabbitMQ 额外支持持久化和重试。",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Java · Go · TS · Python",
      title: "在你的项目里连上它",
      intro: `下面的配置都指向本机 amqp://127.0.0.1:${port}。所有语言的 AMQP 客户端用法相似：建立连接、声明 channel、声明交换机和队列、绑定、发布和消费。`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java (Spring AMQP)",
              lang: "xml",
              caption: "pom.xml",
              code: `<dependency>
  <groupId>org.springframework.boot</groupId>
  <artifactId>spring-boot-starter-amqp</artifactId>
</dependency>`,
            },
            {
              label: "Java",
              lang: "yaml",
              caption: "application.yml",
              code: `spring:
  rabbitmq:
    host: 127.0.0.1
    port: ${port}
    username: zhiyu
    password: zhiyu-local-rabbitmq-2026
    virtual-host: /
    listener:
      simple:
        acknowledge-mode: manual   # 手动 ACK
        prefetch: 1               # 每次取一条
        retry:
          enabled: true
          max-attempts: 3
          initial-interval: 1000`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "配置与收发",
              code: `@Configuration
public class RabbitConfig {

    @Bean
    public Queue orderQueue() {
        return QueueBuilder.durable("orders")
            .deadLetterExchange("dlx-exchange")
            .deadLetterRoutingKey("failed")
            .build();
    }

    @Bean
    public DirectExchange orderExchange() {
        return new DirectExchange("order-events");
    }

    @Bean
    public Binding binding(Queue orderQueue, DirectExchange exchange) {
        return BindingBuilder.bind(orderQueue).to(exchange).with("order.created");
    }
}

// 发布
@Service
public class OrderProducer {
    private final RabbitTemplate rabbit;

    public OrderProducer(RabbitTemplate rabbit) { this.rabbit = rabbit; }

    public void sendOrderCreated(Order order) {
        rabbit.convertAndSend("order-events", "order.created", order);
    }
}

// 消费
@Component
public class OrderConsumer {

    @RabbitListener(queues = "orders")
    public void handle(Order order, Channel channel,
                       @Header(AmqpHeaders.DELIVERY_TAG) long tag) throws IOException {
        try {
            processOrder(order);
            channel.basicAck(tag, false);     // 成功：确认
        } catch (Exception e) {
            channel.basicNack(tag, false, false); // 失败：丢弃到死信
        }
    }
}`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "安装 amqp091-go",
              code: `go get github.com/rabbitmq/amqp091-go`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "发布与消费",
              code: `package rabbit

import (
    "encoding/json"

    amqp "github.com/rabbitmq/amqp091-go"
)

var conn *amqp.Connection
var ch *amqp.Channel

func Init() error {
    cfg := amqp.Config{Properties: amqp.Table{}}
    var err error
    conn, err = amqp.DialConfig("amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:${port}/", cfg)
    if err != nil {
        return err
    }
    ch, err = conn.Channel()
    return err
}

func DeclareQueue(name string, dlx string) error {
    args := amqp.Table{}
    if dlx != "" {
        args["x-dead-letter-exchange"] = dlx
    }
    _, err := ch.QueueDeclare(name, true, false, false, false, args)
    return err
}

func Publish(exchange, routingKey string, body interface{}) error {
    data, _ := json.Marshal(body)
    return ch.Publish(exchange, routingKey, false, false,
        amqp.Publishing{ContentType: "application/json", Body: data})
}

func Consume(queue, consumer string, handler func([]byte) error) error {
    msgs, err := ch.Consume(queue, consumer, false, false, false, false, nil)
    if err != nil {
        return err
    }
    for msg := range msgs {
        if err := handler(msg.Body); err != nil {
            msg.Nack(false, false) // 丢弃到死信
        } else {
            msg.Ack(false)
        }
    }
    return nil
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "安装 amqplib",
              code: `npm install amqplib`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "发布与消费",
              code: `import amqp, { type Connection, type Channel } from "amqplib";

let conn: Connection;
let ch: Channel;

async function init() {
  conn = await amqp.connect("amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:${port}");
  ch = await conn.createChannel();
}

// 声明队列（支持死信）
async function declareQueue(name: string, dlx?: string) {
  const args: Record<string, any> = {};
  if (dlx) args["x-dead-letter-exchange"] = dlx;
  await ch.assertQueue(name, { durable: true, arguments: args });
}

// 发布消息
async function publish(exchange: string, routingKey: string, body: object) {
  ch.publish(exchange, routingKey,
    Buffer.from(JSON.stringify(body)), { contentType: "application/json" });
}

// 消费消息
async function consume(queue: string, handler: (msg: any) => Promise<void>) {
  await ch.prefetch(1);
  await ch.consume(queue, async (msg) => {
    if (!msg) return;
    try {
      await handler(JSON.parse(msg.content.toString()));
      ch.ack(msg);
    } catch {
      ch.nack(msg, false, false); // 丢弃到死信
    }
  });
}`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "安装 pika",
              code: `pip install pika`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "发布与消费",
              code: `import pika
import json

connection = pika.BlockingConnection(pika.URLParameters(
    "amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:${port}/"))
channel = connection.channel()

# 声明队列（带死信）
def declare_queue(name: str, dlx: str = None):
    args = {}
    if dlx:
        args["x-dead-letter-exchange"] = dlx
    channel.queue_declare(queue=name, durable=True, arguments=args)

# 发布消息
def publish(exchange: str, routing_key: str, body: dict):
    channel.basic_publish(
        exchange=exchange,
        routing_key=routing_key,
        body=json.dumps(body),
        properties=pika.BasicProperties(content_type="application/json"),
    )

# 消费消息
def consume(queue: str, handler):
    channel.basic_qos(prefetch_count=1)

    def callback(ch, method, properties, body):
        try:
            handler(json.loads(body))
            ch.basic_ack(delivery_tag=method.delivery_tag)
        except Exception:
            ch.basic_nack(delivery_tag=method.delivery_tag, requeue=False)

    channel.basic_consume(queue=queue, on_message_callback=callback)
    channel.start_consuming()`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Channel 不是线程安全的",
          value:
            "在 RabbitMQ 里 Connection 是线程安全的（开销较大），Channel 不是（开销较小）。每个线程/协程应该创建自己的 Channel，用完即关。框架（如 Spring AMQP、pika）通常已经帮你管理好了，但手写原生 AMQP 客户端时要特别注意这一点。",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "避坑与调优",
      navHint: "排查 · 常见问题",
      title: "上线前该知道的事",
      intro: "RabbitMQ 在功能上很灵活，但灵活也意味着用错了容易出问题。这些是生产环境常见的坑。",
      blocks: [
        {
          kind: "table",
          head: ["问题", "现象", "对策"],
          rows: [
            [
              "消息未持久化",
              "RabbitMQ 重启后队列和消息全部丢失",
              "声明队列时设 durable=true，发送消息时设 delivery_mode=2（持久）",
            ],
            [
              "自动 ACK 丢消息",
              "消费者崩溃时正在处理的消息丢失",
              "改用手动 ACK，处理完再 basic_ack",
            ],
            [
              "循环重新入队",
              "消息被 NACK 再 requeue，无限循环",
              "加上重试次数上限；超过上限后 nack(requeue=false) 进入死信",
            ],
            [
              "队列无限增长",
              "消费者跟不上生产速度，队列积压几十万条",
              "监控队列深度；增加消费者实例；考虑设置队列最大长度和 TTL",
            ],
            [
              "连接泄漏",
              "连接数超限，新连接被拒绝",
              "检查是否每次操作都新建 Connection；全局复用 Connection，每次操作新建 Channel",
            ],
            [
              "prefetch 过大",
              "一个消费者取了几百条消息，其他消费者空闲",
              "设置合理的 prefetch（如 1 或 10），让消息均匀分发",
            ],
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "排查命令",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

# 集群状态
$RABBIT_HOME/rabbitmqctl status

# 连接数
$RABBIT_HOME/rabbitmqctl list_connections

# 队列详情（消息数、消费者数、内存占用）
$RABBIT_HOME/rabbitmqctl list_queues name messages consumers memory state

# 查看未被确认的消息
$RABBIT_HOME/rabbitmqctl list_queues name messages_ready messages_unacknowledged

# 交换机列表
$RABBIT_HOME/rabbitmqctl list_exchanges name type durable

# 绑定关系
$RABBIT_HOME/rabbitmqctl list_bindings

# 清空队列
$RABBIT_HOME/rabbitmqctl purge_queue orders`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么做",
          value:
            `「概览」标签页展示了连接数、消息流入流出等实时指标；「连接与控制台」标签页有 AMQP 和管理地址速查；打开 http://127.0.0.1:15672 Management UI 可以看到所有交换机、队列、连接的可视化面板和消息流量图表，这也是日常调试最主要的入口；「运行日志」能看到 Erlang 层面的启动报错；改端口和内存限额在「配置文件」标签页编辑后重启即可；做危险操作前记得去「备份恢复」打一个快照。`,
        },
      ],
    },
  ];
}
