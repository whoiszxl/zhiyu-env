import type { DocChapter } from "../docTypes";

export function buildActivemqDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "About ActiveMQ",
      navHint: "JMS · OpenWire",
      title: "Local ActiveMQ Classic broker",
      intro:
        "ActiveMQ Classic is useful for testing Java/JMS, OpenWire, AMQP, and STOMP applications. Zhiyu keeps its program, config, data, and logs inside the user directory.",
      blocks: [
        {
          kind: "table",
          head: ["Item", "Default", "Notes"],
          rows: [
            ["OpenWire", `tcp://127.0.0.1:${port}`, "Common Java client endpoint"],
            ["Web console", "http://127.0.0.1:8161/admin/", "Inspect queues, topics, and consumers"],
            ["Credentials", "admin / admin", "Local development only"],
            ["Java", "6.2: 17/21; 6.3: 25", "Uses the Java runtime managed by Zhiyu"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Local development only",
          value:
            "Zhiyu binds transport ports to 127.0.0.1. Never reuse these default credentials or settings in production.",
        },
      ],
    },
    {
      id: "quickstart",
      navLabel: "Quick start",
      navHint: "Queue · Topic",
      title: "Connect and send a message",
      intro:
        "Install a compatible Java runtime under Languages before starting ActiveMQ, then copy endpoints from the Connection tab.",
      blocks: [
        {
          kind: "code",
          lang: "yaml",
          caption: "Spring Boot application.yml",
          code: `spring:
  activemq:
    broker-url: tcp://127.0.0.1:${port}
    user: admin
    password: admin`,
        },
        {
          kind: "list",
          items: [
            "Use a Queue when workers compete for tasks.",
            "Use a Topic when every active subscriber should receive the event.",
            "If startup fails, check the selected Java version, port 61616, and service logs.",
          ],
        },
      ],
    },
    {
      id: "versions",
      navLabel: "Versions",
      navHint: "Java compatibility",
      title: "Choose the broker for your Java runtime",
      intro:
        "ActiveMQ 6.2.8 is the recommended default. Select 6.3.0 only when the project already uses Java 25.",
      blocks: [
        {
          kind: "table",
          head: ["ActiveMQ", "Java runtime", "Recommendation"],
          rows: [
            ["6.2.8", "Java 17 or 21", "Recommended for common modern projects"],
            ["6.3.0", "Java 25", "Use when the newest broker line is required"],
          ],
        },
      ],
    },
  ];
}
