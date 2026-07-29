import type { DocChapter } from "../docTypes";

export function buildKafkaDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "Meet Kafka Sandbox",
      navHint: "Protocol-compatible · Scope-aware",
      title: "A local Kafka sandbox with no JVM required",
      intro:
        "Zhiyu uses Tansu to provide a Kafka API-compatible service, delivering produce, consume, and topic debugging for daily development from a single Rust process and one SQLite file.",
      blocks: [
        {
          kind: "list",
          items: [
            `Point any standard Kafka client to 127.0.0.1:${port}.`,
            "No Java, ZooKeeper, Docker, or virtual machine required.",
            "Data lives in your user directory and persists after the service stops.",
            "Positioned as a feature-compatible local sandbox — not for clusters, load testing, or production.",
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "What it's good for",
          value:
            "Verifying that your app publishes events correctly, debugging consumer logic, and reproducing topic naming or message format issues — that's the sweet spot for this module.",
        },
      ],
    },
    {
      id: "quickstart",
      navLabel: "Quick connect",
      navHint: "Address · Clients",
      title: "Your app only needs a bootstrap server",
      intro: "Kafka clients need no username or password — just connect to the local port.",
      blocks: [
        {
          kind: "table",
          head: ["Setting", "Value", "Notes"],
          rows: [
            ["Bootstrap Servers", `127.0.0.1:${port}`, "Application connection address"],
            ["Security protocol", "PLAINTEXT", "Listens on localhost only"],
            ["Default partitions", "3", "Adjustable when creating a topic"],
            ["Storage", "SQLite", "Located at ~/.devbox/instances/kafka/default/data"],
          ],
        },
        {
          kind: "code",
          lang: "properties",
          caption: "Spring Boot",
          code: `spring.kafka.bootstrap-servers=127.0.0.1:${port}`,
        },
      ],
    },
    {
      id: "limits",
      navLabel: "Scope & limits",
      navHint: "Lightweight · Non-production",
      title: "Things it deliberately does not solve",
      intro:
        "To stay lightweight, Kafka Sandbox does not simulate a full production cluster. The needs below belong on a real Kafka deployment.",
      blocks: [
        {
          kind: "list",
          items: [
            "Multi-broker setups, high-availability replicas, and failover.",
            "Capacity planning, throughput load testing, and latency benchmarks.",
            "Complex ACLs, SASL, TLS, and cross-network deployments.",
            "Full compatibility certification for edge-case Kafka protocol features.",
          ],
        },
      ],
    },
  ];
}
