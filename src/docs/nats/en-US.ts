import type { DocChapter } from "../docTypes";

export function buildNatsDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "Meet NATS",
      navHint: "Subject · Pub/Sub",
      title: "What is NATS",
      intro:
        "NATS is a lightweight messaging server. Apps publish messages to a subject, and any client subscribed to the same subject receives them instantly.",
      blocks: [
        {
          kind: "text",
          value:
            "Unlike RabbitMQ or Kafka, NATS is designed to be radically simple. There are no complex routing rules, no exchanges or partitions to manage, and the core protocol has only a dozen or so commands. The server is a single executable that runs out of the box, and clients are available in dozens of languages with minimal integration cost. It fits perfectly for async communication between microservices, event-driven architectures, and command dispatch.",
        },
        {
          kind: "text",
          value: "NATS messaging has two layers:",
        },
        {
          kind: "list",
          items: [
            "Core NATS — at-most-once pub/sub. Messages are delivered if there are subscribers, otherwise dropped. Latency is extremely low, typically under 100 microseconds.",
            "JetStream — persistent message streams built on top of Core NATS. Supports at-least-once delivery, replay, consumer groups, and work queues, feature-wise close to a simplified Kafka.",
          ],
        },
        {
          kind: "table",
          head: ["Item", "Local config", "Notes"],
          rows: [
            ["Client address", `nats://127.0.0.1:${port}`, "Endpoint for apps to connect"],
            ["Monitoring address", "http://127.0.0.1:8222", "Zhiyu reads real-time metrics"],
            ["Auth", "None", "Dev instance listens on 127.0.0.1 only"],
            ["JetStream", "Enabled", "Create a stream when persistence is needed"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Understand subjects before you code",
          value:
            "NATS has no queues or exchanges — every message goes to a subject. A subject is a dot-separated string (like orders.created). When subscribing, * matches exactly one token and > matches all remaining tokens. This hierarchical token-based naming is both the address and the routing key, and it is the core design of NATS.",
        },
        {
          kind: "table",
          head: ["", "NATS", "RabbitMQ", "Kafka"],
          rows: [
            ["Messaging model", "Pub/Sub + Request/Reply", "AMQP queues & exchanges", "Partitioned log streams"],
            ["Consumption", "At-most-once / at-least-once with JetStream", "Manual/auto ACK", "Consumer group offsets"],
            ["Persistence", "Provided by JetStream", "Built-in durable queues", "Core design, cannot be disabled"],
            ["Deployment", "Single binary, one port", "Erlang + multiple processes", "Java + ZooKeeper/KRaft"],
            ["Typical latency", "Microseconds", "Milliseconds", "Milliseconds (batching boosts throughput)"],
            ["Best fit", "In-service async decoupling, event broadcast", "Task queues, complex routing", "High-throughput logs, stream processing"],
          ],
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quickstart",
      navHint: "Publish / Subscribe",
      title: "Publish your first message and receive it",
      intro:
        "Zhiyu has installed and started NATS for you — no downloads or setup required. The message debugging page handles publishing and receiving directly.",
      blocks: [
        {
          kind: "list",
          items: [
            "Confirm the status is Running on the Overview tab.",
            "Switch to the Message Debug tab: publish area on the left, subscribe area on the right.",
            "In the subscribe area, enter a subject (e.g. test.hello) and click Start listening.",
            "In the publish area, enter the same subject and a message body, then click Publish.",
            "The message arrives instantly on the right, showing the payload and latency.",
          ],
        },
        {
          kind: "text",
          value:
            "To use the command line, the nats client shipped with your system can connect like this (Zhiyu installs the binary under ~/.devbox/installations/nats):",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Publish and subscribe from the CLI",
          code: `NATS_BIN=~/.devbox/installations/nats/default/bin

# Terminal 1: subscribe to test.hello
$NATS_BIN/nats sub test.hello

# Terminal 2: publish a message
$NATS_BIN/nats pub test.hello "Hello Zhiyu"

# Terminal 1 prints the received message immediately

# Subscribe to any second-level subject under orders
$NATS_BIN/nats sub "orders.*"

# Subscribe to any depth of subject under orders
$NATS_BIN/nats sub "orders.>"`,
        },
        {
          kind: "text",
          value:
            "Request/Reply is another important pattern in Core NATS. The publisher attaches a reply subject, and the subscriber sends the result back to that address — essentially async RPC:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Request/Reply pattern",
          code: `# Send a request and wait for the reply (RPC-style)
$NATS_BIN/nats req "user.get" '{"id":1001}'

# Server side: listen and reply
$NATS_BIN/nats reply "user.get" '{"name":"Zhang San","age":28}'`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Core NATS messages are not persisted by default",
          value:
            "Without an active subscriber, a plain Publish is not stored anywhere. If you stop the subscriber and then publish, the message is dropped. When you need persistence, replay, or delivery acknowledgments, use JetStream (see the next chapter).",
        },
      ],
    },

    {
      id: "jetstream",
      navLabel: "JetStream persistence",
      navHint: "Stream · Consumer",
      title: "Use JetStream when you need persistence and replay",
      intro:
        "JetStream turns NATS from fire-and-forget into reliable delivery. It persists messages into a stream that consumers pull from, with ACKs, retries, and time-based replay.",
      blocks: [
        {
          kind: "text",
          value:
            "JetStream has only three core concepts: Stream (message storage), Consumer (consumption view), and Subject (entry point). Messages are published to a subject; a stream captures matching subjects and writes them to disk; consumers read from the stream at their own pace. A single stream can have multiple consumers, each maintaining its own progress independently.",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Create a Stream and a Consumer",
          code: `NATS_BIN=~/.devbox/installations/nats/default/bin

# Create a Stream that captures orders.* subjects
$NATS_BIN/nats str add ORDERS \\
  --subjects "orders.*" \\
  --storage file \\
  --max-age 24h \\
  --replicas 1

# Inspect stream info
$NATS_BIN/nats str info ORDERS

# Create a Pull Consumer
$NATS_BIN/nats con add ORDERS PROCESSOR \\
  --filter "orders.created" \\
  --ack explicit \\
  --max-deliver 3`,
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Publish and consume JetStream messages",
          code: `# Publish a JetStream message
$NATS_BIN/nats pub orders.created '{"id":88,"amount":299}' --js

# Pull consumption (batch of 10)
$NATS_BIN/nats con next ORDERS PROCESSOR --count 10

# Inspect stream status
$NATS_BIN/nats str report ORDERS`,
        },
        {
          kind: "table",
          head: ["Option", "Meaning", "Suggested value"],
          rows: [
            ["--max-age", "Maximum retention time", "Depends on the workload: 1d for logs, 7d for events"],
            ["--max-msgs", "Maximum message count in the stream", "Size based on capacity to avoid disk exhaustion"],
            ["--max-bytes", "Maximum disk usage for the stream", "1GB is plenty for local development"],
            ["--ack explicit", "Manual ACK — only confirmed once consumed", "Any workload that requires reliable consumption"],
            ["--ack none", "No ACK — delivery counts as done", "Workloads that tolerate message loss"],
            ["--max-deliver", "Maximum delivery attempts", "Move to dead-letter after 3-5 retries"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "When to use JetStream vs Core NATS",
          value:
            "If you just want to notify other services that something happened, Core NATS is enough — lower latency, less configuration. If messages cannot be lost, must be replayable, or must be consumed independently by multiple consumers, use JetStream. Most projects start with Core NATS and enable JetStream later when persistence becomes a requirement.",
        },
      ],
    },

    {
      id: "subjects",
      navLabel: "Subject design",
      navHint: "Naming · Wildcards",
      title: "Subject naming is the most important design decision in NATS",
      intro:
        "There are no exchanges or queue bindings in NATS — subjects act as both the address and the routing key. Good naming conventions keep the system maintainable; bad ones lead to message flow chaos.",
      blocks: [
        {
          kind: "text",
          value:
            "A subject is composed of dot-separated tokens, each containing letters, digits, and underscores. A common three-token convention is `domain.entity.event`:",
        },
        {
          kind: "code",
          lang: "text",
          caption: "Recommended subject naming",
          code: `order.created        # Order has been created
order.paid           # Order has been paid
user.registered      # User has registered
user.updated         # User profile updated
email.sent           # Email has been sent
system.heartbeat     # Service heartbeat`,
        },
        {
          kind: "text",
          value:
            "When subscribing, use wildcards to let NATS handle routing for you. The two wildcards mean different things:",
        },
        {
          kind: "table",
          head: ["Wildcard", "Meaning", "Example", "Matches"],
          rows: [
            ["*", "Matches exactly one token", `"orders.*"`, "orders.created / orders.paid (does not match orders.created.email)"],
            [">", "Matches all remaining tokens", `"orders.>"`, "orders.created / orders.created.email (matches everything)"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Wildcards are only for subscribing, not publishing",
          value:
            "When publishing, the subject must be an exact string — no * or > allowed. Tokens must not contain whitespace, and total subject length cannot exceed 256 bytes.",
        },
        {
          kind: "code",
          lang: "text",
          caption: "Typical hierarchical subject design",
          code: `# Split by service
svc.order.*         # Order service events
svc.user.*          # User service events
svc.payment.*       # Payment service events

# Split by environment (when sharing a single NATS across envs)
prod.order.created  # Production
dev.order.created   # Development

# Separate commands from events (CQRS-style)
cmd.order.create    # Command: create order
evt.order.created   # Event: order created`,
        },
      ],
    },

    {
      id: "clients",
      navLabel: "Client integration",
      navHint: "Java · Go · TS · Python",
      title: "Connect from your project",
      intro: `All snippets below point to nats://127.0.0.1:${port} on this machine. Key takeaways: NATS clients ship with connection pooling and automatic reconnection, so reuse a single global instance; the APIs across languages are highly consistent — once you know one, the rest are easy.`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java",
              lang: "xml",
              caption: "Spring Boot: pom.xml (or add jnats directly)",
              code: `<dependency>
  <groupId>io.nats</groupId>
  <artifactId>jnats</artifactId>
  <version>2.20.4</version>
</dependency>`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "Publish and subscribe",
              code: `import io.nats.client.*;

try (Connection nc = Nats.connect("nats://127.0.0.1:${port}")) {

    // Subscribe
    Subscription sub = nc.subscribe("orders.created");
    System.out.println("Received: " + new String(sub.nextMessage(Duration.ofSeconds(10)).getData()));

    // Publish
    nc.publish("orders.created", "{\\"id\\":88,\\"amount\\":299}".getBytes());

    // Request/Reply
    CompletableFuture<Message> reply = nc.request("user.get", "{\\"id\\":1001}".getBytes());
    System.out.println("Reply: " + new String(reply.get().getData()));
}`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "Install nats.go",
              code: `go get github.com/nats-io/nats.go`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "Publish and subscribe",
              code: `package messenger

import "github.com/nats-io/nats.go"

// Reuse a single global Conn — it handles reconnect and pooling
var nc, _ = nats.Connect("nats://127.0.0.1:${port}")

func Publish(subj string, data []byte) error {
    return nc.Publish(subj, data)
}

func Subscribe(subj string, handler func([]byte)) (*nats.Subscription, error) {
    return nc.Subscribe(subj, func(msg *nats.Msg) {
        handler(msg.Data)
    })
}

// Request/Reply pattern
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
              caption: "Install nats.ws (Node.js / Deno)",
              code: `npm install nats.ws`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "Publish and subscribe",
              code: `import { connect, StringCodec } from "nats.ws";

// Reuse a single global connection
const nc = await connect({ servers: "127.0.0.1:${port}" });
const sc = StringCodec();

// Subscribe
const sub = nc.subscribe("orders.created");
(async () => {
  for await (const m of sub) {
    console.log("Received:", sc.decode(m.data));
  }
})();

// Publish
nc.publish("orders.created", sc.encode(JSON.stringify({ id: 88, amount: 299 })));

// Request/Reply
const reply = await nc.request("user.get", sc.encode(JSON.stringify({ id: 1001 })));
console.log("Reply:", sc.decode(reply.data));`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "Install nats-py",
              code: `pip install nats-py`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "Publish and subscribe",
              code: `import asyncio
from nats.aio.client import Client as NATS

nc = NATS()

async def main():
    await nc.connect("nats://127.0.0.1:${port}")

    # Subscribe
    async def handler(msg):
        print(f"Received: {msg.data.decode()}")

    await nc.subscribe("orders.created", cb=handler)

    # Publish
    await nc.publish("orders.created", b'{"id":88,"amount":299}')

    # Request/Reply
    reply = await nc.request("user.get", b'{"id":1001}', timeout=5)
    print(f"Reply: {reply.data.decode()}")

    # Keep running to wait for messages
    await asyncio.sleep(60)

asyncio.run(main())`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Don't create a new connection for every message",
          value:
            "A NATS client connection is long-lived — it already manages pooling and automatic reconnection internally. Creating a new connection for every request causes the server-side connection count to explode, and every new connection incurs the INFO/CONNECT handshake — much slower than reusing an existing one. Keep the Connection instance as a module-level singleton or manage it through your DI container.",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "Pitfalls and tuning",
      navHint: "Troubleshooting · Common issues",
      title: "What to know before going to production",
      intro: "NATS runs smoothly on your laptop, but a few things are worth knowing before it hits production.",
      blocks: [
        {
          kind: "table",
          head: ["Issue", "Symptom", "Fix"],
          rows: [
            [
              "Messages silently disappear",
              "Messages published while a subscriber is offline are never received",
              "Move from Core NATS to JetStream — the stream persists messages until a consumer picks them up",
            ],
            [
              "Slow consumers drag down the server",
              "slow_consumers metric keeps rising",
              "NATS drops subsequent messages when a consumer is too slow; scale out consumers to process in parallel",
            ],
            [
              "Inconsistent subject naming",
              "Services use different subject formats, making integration painful",
              "Standardize on a three-token `domain.entity.event` scheme and document both publishers and subscribers",
            ],
            [
              "Too many connections",
              "nats server reports max_connections",
              "Check whether clients create a new connection per operation; reuse a global Connection",
            ],
            [
              "JetStream storage full",
              "Stream can no longer accept new messages",
              "Review max-bytes / max-msgs configuration; set a reasonable max-age to auto-clean",
            ],
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Troubleshooting commands",
          code: `NATS_BIN=~/.devbox/installations/nats/default/bin

# Inspect server status
$NATS_BIN/nats server report connections
$NATS_BIN/nats server info

# List all streams
$NATS_BIN/nats str ls

# Inspect a specific stream
$NATS_BIN/nats str info ORDERS

# List consumers
$NATS_BIN/nats con ls ORDERS

# Check consumer backlog (a large pending count means consumption cannot keep up)
$NATS_BIN/nats con info ORDERS PROCESSOR`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "How to do this in Zhiyu",
          value:
            "The Overview tab shows real-time metrics like connection count, message throughput, and slow consumers; the Message Debug tab lets you quickly test pub/sub; the Runtime Logs tab surfaces server startup errors and JetStream storage status; port and JetStream storage path can be edited on the Config File tab and applied by restarting the service.",
        },
      ],
    },
  ];
}
