import type { DocChapter } from "../docTypes";

export function buildRabbitmqDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "About RabbitMQ",
      navHint: "AMQP · Exchange · Queue",
      title: "What is RabbitMQ",
      intro:
        "RabbitMQ is the most widely deployed open-source message broker. It fully implements the AMQP 0-9-1 protocol, and through the flexible combination of exchanges and queues, it can cover nearly every asynchronous communication scenario.",
      blocks: [
        {
          kind: "text",
          value:
            "RabbitMQ's messaging model is fundamentally different from NATS. NATS has only subjects—the publisher sends a message directly to a subject. RabbitMQ introduces an exchange layer between the publisher and the queue: the publisher sends messages to the exchange, which then routes them to bound queues according to routing rules. This design decouples routing logic from the application code, so the publisher never needs to know which queues ultimately receive the message.",
        },
        {
          kind: "text",
          value: "RabbitMQ core concepts:",
        },
        {
          kind: "list",
          items: [
            "Producer — the application that sends messages. Messages go to the Exchange first, not directly into a queue.",
            "Exchange — receives messages and routes them to queues. Four types: direct, topic, fanout, headers.",
            "Queue — the unit of message storage and delivery. Consumers pull messages from queues or wait for push delivery.",
            "Binding — the routing rule connecting an exchange to a queue, typically via routing key pattern matching.",
            "Consumer — the application that receives and processes messages. Supports ACK, rejection, and requeue.",
          ],
        },
        {
          kind: "table",
          head: ["Item", "Value", "Description"],
          rows: [
            ["AMQP", `amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:${port}/`, "Application connection"],
            ["Management", "http://127.0.0.1:15672", "Queue and exchange management"],
            ["Runtime", "Erlang/OTP 27.3.4.6", "Installed with the service under user directory"],
            ["Username", "zhiyu", "Local development instance"],
            ["Password", "zhiyu-local-rabbitmq-2026", "Local development password"],
          ],
        },
        {
          kind: "table",
          head: ["", "RabbitMQ", "NATS", "Kafka"],
          rows: [
            ["Routing Model", "Exchange → Queue → Consumer", "Subject direct send", "Topic → Partition → Consumer"],
            ["Delivery Guarantee", "ACK, manual retry", "Core: at-most-once; JS: at-least-once", "Offset commit"],
            ["Consumption Mode", "Push or Pull", "Push subscription to client", "Pull"],
            ["Persistence", "Durable queues and messages to disk", "JetStream provides it", "Core design, cannot be disabled"],
            ["Routing Capability", "Very strong: direct/topic/fanout/headers", "Subject wildcards", "Topic partition"],
            ["Management UI", "Built-in Web UI (15672)", "HTTP monitoring (8222)", "Requires separate tools"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "RabbitMQ's killer feature is flexible routing",
          value:
            "If you just need to 'send a message to notify another service', NATS is faster and simpler. If you need to 'distribute messages to different processing chains based on content', 'delay delivery of certain messages', or 'handle failed retries with a dead-letter queue', RabbitMQ is the better choice.",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quick Start",
      navHint: "Connect · Send a Message",
      title: "Connect and Send Your First Message",
      intro:
        "Zhiyu has already installed RabbitMQ and Erlang/OTP and started the service. The 'Connection & Console' tab shows the AMQP connection string and Management UI address — you can copy them directly into your code.",
      blocks: [
        {
          kind: "list",
          items: [
            "On the 'Overview' tab, confirm the status is 'Running'.",
            "The 'Connection & Console' tab shows the AMQP address, Management UI address, username, and password — ready to copy.",
            "Open http://127.0.0.1:15672 in your browser and log in with zhiyu / zhiyu-local-rabbitmq-2026 to the Management UI, where you can see real-time message throughput, connection counts, and queue status.",
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Operate with rabbitmqctl and rabbitmqadmin",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

# Check cluster status
$RABBIT_HOME/rabbitmqctl status

# List all queues
$RABBIT_HOME/rabbitmqctl list_queues name messages consumers

# List all exchanges
$RABBIT_HOME/rabbitmqctl list_exchanges name type durable

# Publish a message with rabbitmqadmin (HTTP API wrapper)
$RABBIT_HOME/rabbitmqadmin declare queue name=hello durable=false
$RABBIT_HOME/rabbitmqadmin publish \\
  routing_key=hello \\
  payload='{"event":"order.created","orderId":88}'

# Get a message (consumed and removed from queue)
$RABBIT_HOME/rabbitmqadmin get queue=hello requeue=false

# Purge a queue
$RABBIT_HOME/rabbitmqadmin purge queue name=hello`,
        },
        {
          kind: "text",
          value:
            "RabbitMQ can be operated via both the AMQP protocol (port 5672) and the HTTP protocol (port 15672). AMQP is used by application code; HTTP is for management and debugging. The rabbitmqadmin tool above operates through the HTTP API, making it convenient to use in the 'JSON Console'.",
        },
      ],
    },

    {
      id: "exchanges",
      navLabel: "Exchange Types",
      navHint: "Direct · Topic · Fanout",
      title: "Four Exchanges, Four Routing Strategies",
      intro:
        "The exchange is RabbitMQ's most fundamental concept. Understanding the four exchange types unlocks 80% of RabbitMQ's routing capabilities.",
      blocks: [
        {
          kind: "table",
          head: ["Type", "Routing Rule", "Typical Use Case", "Example"],
          rows: [
            [
              "Direct",
              "Exact routing key match",
              "Point-to-point task distribution, single-service consumption",
              "key=order:paid → Queue: order_paid",
            ],
            [
              "Topic",
              "Routing key pattern matching (* single word, # multi-word)",
              "Multiple services consuming the same type of message differently",
              "order.# → Queue: order_all; order.paid.* → Queue: order_paid_payment",
            ],
            [
              "Fanout",
              "Ignores routing key, broadcasts to all bound queues",
              "System broadcast, cache invalidation notifications",
              "All bound queues receive the message",
            ],
            [
              "Headers",
              "Matches on message headers (ignores routing key)",
              "Complex multi-condition routing (less common)",
              "header: type=email AND priority=high",
            ],
          ],
        },
        {
          kind: "text",
          value:
            "Direct is the most straightforward — the routing key between the exchange and queue must match exactly. Suitable for scenarios where 'one message is processed by exactly one consumer':",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Direct exchange example",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

# Declare a Direct exchange
$RABBIT_HOME/rabbitmqadmin declare exchange name=order-events type=direct

# Declare two queues
$RABBIT_HOME/rabbitmqadmin declare queue name=order.created
$RABBIT_HOME/rabbitmqadmin declare queue name=order.paid

# Bind: queues and routing keys in one-to-one mapping
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=order-events destination=order.created routing_key=order.created
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=order-events destination=order.paid routing_key=order.paid

# Publish to the order.paid routing key
$RABBIT_HOME/rabbitmqadmin publish exchange=order-events \\
  routing_key=order.paid \\
  payload='{"orderId":88,"amount":299}'

# Verify: only the order.paid queue receives the message
$RABBIT_HOME/rabbitmqadmin get queue=order.paid requeue=false
$RABBIT_HOME/rabbitmqadmin get queue=order.created requeue=false  # empty`,
        },
        {
          kind: "text",
          value:
            "Topic is the most flexible. The routing key uses . as a delimiter, * matches one segment, and # matches any number of segments:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Topic exchange example",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

$RABBIT_HOME/rabbitmqadmin declare exchange name=events type=topic

# Three queues, each interested in different levels of messages
$RABBIT_HOME/rabbitmqadmin declare queue name=all.order
$RABBIT_HOME/rabbitmqadmin declare queue name=order.paid.processor
$RABBIT_HOME/rabbitmqadmin declare queue name=all.events

# order.# matches all messages starting with order
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=events destination=all.order routing_key=order.#

# order.paid.* matches the exact paid suffix
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=events destination=order.paid.processor routing_key=order.paid.*

# # matches all messages
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=events destination=all.events routing_key=#

# Now send a message on order.paid
$RABBIT_HOME/rabbitmqadmin publish exchange=events \\
  routing_key=order.paid \\
  payload='{"orderId":88,"amount":299}'

# order.paid matches all three routing key patterns,
# so all three queues will receive this message`,
        },
        {
          kind: "text",
          value:
            "Fanout is the simplest — regardless of the routing key, the message is broadcast to all bound queues. Ideal for broadcast scenarios like 'notify all services to refresh':",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Fanout exchange example",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

$RABBIT_HOME/rabbitmqadmin declare exchange name=cache-invalidation type=fanout

# Three queues, each belonging to a different service
$RABBIT_HOME/rabbitmqadmin declare queue name=svc-a-cache
$RABBIT_HOME/rabbitmqadmin declare queue name=svc-b-cache
$RABBIT_HOME/rabbitmqadmin declare queue name=svc-c-cache

# All bind to the fanout exchange (no routing key needed)
$RABBIT_HOME/rabbitmqadmin declare binding source=cache-invalidation destination=svc-a-cache
$RABBIT_HOME/rabbitmqadmin declare binding source=cache-invalidation destination=svc-b-cache
$RABBIT_HOME/rabbitmqadmin declare binding source=cache-invalidation destination=svc-c-cache

# Send one message, all three queues will receive it
$RABBIT_HOME/rabbitmqadmin publish exchange=cache-invalidation \\
  routing_key=ignored \\
  payload='{"type":"cache-clear","keys":["user:*"]}'`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Topic's # wildcard behaves like Fanout",
          value:
            "Binding a queue to a Topic exchange with routing_key=# achieves the exact same effect as Fanout—all messages enter the queue. But Topic is more flexible because you can also bind other queues to the same exchange with more precise routing keys, which Fanout cannot do.",
        },
      ],
    },

    {
      id: "patterns",
      navLabel: "Common Patterns",
      navHint: "Work Queues · Retry · DLX",
      title: "Four Most Useful Message Patterns",
      intro:
        "Beyond the four exchange types, the RabbitMQ ecosystem includes several widely used message patterns. Understanding them lets you tackle most business scenarios.",
      blocks: [
        {
          kind: "text",
          value:
            "Work Queues: Multiple consumers share a single queue, and messages are distributed to idle consumers. Ideal for time-consuming tasks (image processing, email sending). Use prefetch to control how many messages each consumer processes simultaneously, preventing slow consumers from becoming a bottleneck:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Work Queues",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

# Declare a durable queue
$RABBIT_HOME/rabbitmqadmin declare queue name=tasks durable=true

# Multiple consumers share the tasks queue; RabbitMQ round-robins messages
# In code, set prefetch=1 (fetch one at a time, process it, then fetch the next)
# This prevents slow consumers from accumulating backlog and lets fast consumers handle more

# Simulate: put 5 tasks first
for i in 1 2 3 4 5; do
  $RABBIT_HOME/rabbitmqadmin publish routing_key=tasks \\
    payload='{"task":"image-resize","id":$i}'
done

# Now start two consumers; messages will be distributed round-robin
# Consumer-1: task 1, 3, 5
# Consumer-2: task 2, 4`,
        },
        {
          kind: "text",
          value:
            "Message Acknowledgment and Retry: The consumer sends an ACK on successful processing and a NACK on failure — the message can then be requeued or discarded. Combined with durable queues, messages are preserved even if RabbitMQ restarts:",
        },
        {
          kind: "table",
          head: ["ACK Mode", "Meaning", "Behavior"],
          rows: [
            ["auto_ack=true", "Auto-ack, message deleted as soon as it reaches the consumer", "Simple but messages may be lost (consumer crash)"],
            ["auto_ack=false", "Manual ack", "Message deleted only after explicit basic_ack; safest option"],
            ["basic_nack(requeue=true)", "Reject and requeue", "Message returned to the front of the queue for other consumers"],
            ["basic_nack(requeue=false)", "Reject and discard", "Message routed to the dead-letter queue (if configured)"],
          ],
        },
        {
          kind: "text",
          value:
            "Dead-Letter Exchange (DLX) is one of RabbitMQ's most valuable features. It forwards messages that cannot be processed (rejected, TTL expired, queue full) to another exchange instead of discarding them, allowing you to investigate and intervene later:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Dead-letter queue configuration",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

# Create a dead-letter exchange
$RABBIT_HOME/rabbitmqadmin declare exchange name=dlx-exchange type=direct

# Create a dead-letter queue
$RABBIT_HOME/rabbitmqadmin declare queue name=dlx-queue
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=dlx-exchange destination=dlx-queue routing_key=failed

# Create a business queue with a dead-letter exchange configured
$RABBIT_HOME/rabbitmqadmin declare queue name=orders \\
  durable=true \\
  arguments='{"x-dead-letter-exchange":"dlx-exchange","x-dead-letter-routing-key":"failed"}'

# After that:
# 1. Consumer basic_nack(requeue=false) → message is forwarded to dlx-exchange
# 2. Message TTL expires → also forwarded to dlx-exchange
# 3. Inspect failed messages in dlx-queue, diagnose, then manually republish`,
        },
        {
          kind: "text",
          value:
            "Delayed Queues: Combining message TTL with a dead-letter queue implements 'execute an operation after N seconds'. For example, closing an unpaid order after a payment timeout:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Delayed queue (TTL + DLX)",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

# Create the final processing exchange
$RABBIT_HOME/rabbitmqadmin declare exchange name=payment-events type=direct

# Create the consumer queue (messages arrive here after TTL expires)
$RABBIT_HOME/rabbitmqadmin declare queue name=payment-timeout

# Bind the consumer queue
$RABBIT_HOME/rabbitmqadmin declare binding \\
  source=payment-events destination=payment-timeout routing_key=timeout

# Create a delay queue (messages wait here for TTL to expire, then forward to payment-events)
$RABBIT_HOME/rabbitmqadmin declare queue name=payment-delay-30m \\
  arguments='{
    "x-message-ttl":1800000,
    "x-dead-letter-exchange":"payment-events",
    "x-dead-letter-routing-key":"timeout"
  }'

# When creating an order, send a message to the delay queue
$RABBIT_HOME/rabbitmqadmin publish routing_key=payment-delay-30m \\
  payload='{"orderId":88,"userId":1001}'

# After 30 minutes, the message automatically arrives in the payment-timeout queue
# The consumer checks the order status and closes it if still unpaid`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "RPC works with RabbitMQ too",
          value:
            "RabbitMQ supports Reply-To and CorrelationId properties for request/response-style RPC: the client includes a reply_to queue name and correlation_id when sending a message; the server processes the request and sends the result back to the reply_to queue; the client matches requests to responses using the correlation_id. The principle is the same as NATS's request/reply pattern, but RabbitMQ additionally supports persistence and retries.",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "Client SDKs",
      navHint: "Java · Go · TS · Python",
      title: "Connect from Your Project",
      intro: `The following configurations all point to the local instance at amqp://127.0.0.1:${port}. AMQP clients in all languages follow a similar pattern: establish a connection, create a channel, declare exchanges and queues, bind, publish, and consume.`,
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
        acknowledge-mode: manual   # Manual ACK
        prefetch: 1               # Fetch one at a time
        retry:
          enabled: true
          max-attempts: 3
          initial-interval: 1000`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "Configuration and messaging",
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

// Publish
@Service
public class OrderProducer {
    private final RabbitTemplate rabbit;

    public OrderProducer(RabbitTemplate rabbit) { this.rabbit = rabbit; }

    public void sendOrderCreated(Order order) {
        rabbit.convertAndSend("order-events", "order.created", order);
    }
}

// Consume
@Component
public class OrderConsumer {

    @RabbitListener(queues = "orders")
    public void handle(Order order, Channel channel,
                       @Header(AmqpHeaders.DELIVERY_TAG) long tag) throws IOException {
        try {
            processOrder(order);
            channel.basicAck(tag, false);     // Success: ack
        } catch (Exception e) {
            channel.basicNack(tag, false, false); // Failure: discard to dead letter
        }
    }
}`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "Install amqp091-go",
              code: `go get github.com/rabbitmq/amqp091-go`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "Publish and consume",
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
            msg.Nack(false, false) // Discard to dead letter
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
              caption: "Install amqplib",
              code: `npm install amqplib`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "Publish and consume",
              code: `import amqp, { type Connection, type Channel } from "amqplib";

let conn: Connection;
let ch: Channel;

async function init() {
  conn = await amqp.connect("amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:${port}");
  ch = await conn.createChannel();
}

// Declare a queue (supports dead-letter configuration)
async function declareQueue(name: string, dlx?: string) {
  const args: Record<string, any> = {};
  if (dlx) args["x-dead-letter-exchange"] = dlx;
  await ch.assertQueue(name, { durable: true, arguments: args });
}

// Publish a message
async function publish(exchange: string, routingKey: string, body: object) {
  ch.publish(exchange, routingKey,
    Buffer.from(JSON.stringify(body)), { contentType: "application/json" });
}

// Consume messages
async function consume(queue: string, handler: (msg: any) => Promise<void>) {
  await ch.prefetch(1);
  await ch.consume(queue, async (msg) => {
    if (!msg) return;
    try {
      await handler(JSON.parse(msg.content.toString()));
      ch.ack(msg);
    } catch {
      ch.nack(msg, false, false); // Discard to dead letter
    }
  });
}`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "Install pika",
              code: `pip install pika`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "Publish and consume",
              code: `import pika
import json

connection = pika.BlockingConnection(pika.URLParameters(
    "amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:${port}/"))
channel = connection.channel()

# Declare a queue (with dead-letter support)
def declare_queue(name: str, dlx: str = None):
    args = {}
    if dlx:
        args["x-dead-letter-exchange"] = dlx
    channel.queue_declare(queue=name, durable=True, arguments=args)

# Publish a message
def publish(exchange: str, routing_key: str, body: dict):
    channel.basic_publish(
        exchange=exchange,
        routing_key=routing_key,
        body=json.dumps(body),
        properties=pika.BasicProperties(content_type="application/json"),
    )

# Consume messages
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
          title: "Channel is not thread-safe",
          value:
            "In RabbitMQ, a Connection is thread-safe (expensive to create), but a Channel is not (cheap to create). Each thread or coroutine should create its own Channel and close it when done. Frameworks like Spring AMQP and pika typically manage this for you, but pay close attention when writing a raw AMQP client by hand.",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "Pitfalls & Tuning",
      navHint: "Troubleshooting · Common Issues",
      title: "What to Know Before Going Live",
      intro: "RabbitMQ is flexible, but that flexibility also means things can go wrong if used incorrectly. These are the common production pitfalls.",
      blocks: [
        {
          kind: "table",
          head: ["Issue", "Symptom", "Solution"],
          rows: [
            [
              "Messages not persisted",
              "Queues and messages are lost after RabbitMQ restarts",
              "Set durable=true when declaring queues, and delivery_mode=2 (persistent) when sending messages",
            ],
            [
              "Auto-ACK loses messages",
              "Messages being processed are lost when the consumer crashes",
              "Switch to manual ACK; call basic_ack only after processing completes",
            ],
            [
              "Infinite requeue loop",
              "Message is NACK'd and requeued repeatedly, causing an infinite loop",
              "Add a maximum retry limit; after exceeding it, nack(requeue=false) to route to the dead-letter queue",
            ],
            [
              "Unbounded queue growth",
              "Consumers can't keep up with producers, queue backlog grows to hundreds of thousands",
              "Monitor queue depth; add more consumer instances; consider setting max queue length and TTL",
            ],
            [
              "Connection leak",
              "Connection count exceeds the limit, new connections are rejected",
              "Check whether a new Connection is created for every operation; reuse a single Connection globally and create a new Channel per operation",
            ],
            [
              "prefetch too large",
              "One consumer grabs hundreds of messages while others sit idle",
              "Set a reasonable prefetch value (e.g., 1 or 10) to distribute messages evenly",
            ],
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Diagnostic commands",
          code: `RABBIT_HOME=~/.devbox/installations/rabbitmq/default/sbin

# Cluster status
$RABBIT_HOME/rabbitmqctl status

# Connection count
$RABBIT_HOME/rabbitmqctl list_connections

# Queue details (message count, consumer count, memory usage)
$RABBIT_HOME/rabbitmqctl list_queues name messages consumers memory state

# View unacknowledged messages
$RABBIT_HOME/rabbitmqctl list_queues name messages_ready messages_unacknowledged

# Exchange list
$RABBIT_HOME/rabbitmqctl list_exchanges name type durable

# Bindings
$RABBIT_HOME/rabbitmqctl list_bindings

# Purge a queue
$RABBIT_HOME/rabbitmqctl purge_queue orders`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "How to do this in Zhiyu",
          value:
            `The 'Overview' tab shows real-time metrics like connection count and message throughput; the 'Connection & Console' tab provides quick access to AMQP and management addresses; open http://127.0.0.1:15672 Management UI to see visual panels for all exchanges, queues, connections, and message traffic charts — this is the primary entry point for daily debugging; the 'Run Log' shows Erlang-level startup errors; to change ports or memory limits, edit the 'Config File' tab and restart; before performing dangerous operations, remember to take a snapshot on the 'Backup & Restore' tab.`,
        },
      ],
    },
  ];
}