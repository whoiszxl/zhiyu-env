import type { DocChapter } from "../docTypes";

export function buildKafkaDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 Kafka Sandbox",
      navHint: "兼容协议 · 适用边界",
      title: "不启动 JVM 的 Kafka 本地沙箱",
      intro:
        "智屿使用 Tansu 提供 Kafka API 兼容服务，用一个 Rust 进程和一个 SQLite 文件满足日常开发中的生产、消费与主题调试。",
      blocks: [
        {
          kind: "list",
          items: [
            `常用 Kafka 客户端连接 127.0.0.1:${port} 即可。`,
            "无需 Java、ZooKeeper、Docker 或虚拟机。",
            "数据保存在用户目录，停止服务后仍会保留。",
            "定位是功能兼容的本地沙箱，不用于集群、压测与生产环境。",
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "适合什么场景",
          value:
            "验证应用是否正确发布事件、调试消费者逻辑、复现主题命名和消息格式问题，是这个模块最合适的用法。",
        },
      ],
    },
    {
      id: "quickstart",
      navLabel: "快速连接",
      navHint: "地址 · 客户端",
      title: "应用只需要一个 Bootstrap Server",
      intro: "Kafka 客户端不需要账号和密码，直接连接本机端口。",
      blocks: [
        {
          kind: "table",
          head: ["配置", "值", "说明"],
          rows: [
            ["Bootstrap Servers", `127.0.0.1:${port}`, "应用连接地址"],
            ["安全协议", "PLAINTEXT", "仅监听本机"],
            ["默认分区", "3", "创建主题时可调整"],
            ["存储", "SQLite", "位于 ~/.devbox/instances/kafka/default/data"],
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
      navLabel: "使用边界",
      navHint: "轻量 · 非生产",
      title: "它刻意不解决的问题",
      intro:
        "为了保持轻量，Kafka Sandbox 不模拟完整生产集群。下面这些需求应使用真正的 Kafka 环境。",
      blocks: [
        {
          kind: "list",
          items: [
            "多 Broker、高可用副本与故障切换。",
            "生产容量评估、吞吐压测和延迟基准。",
            "复杂 ACL、SASL、TLS 与跨网络部署。",
            "依赖 Kafka 边缘协议特性的完整兼容性认证。",
          ],
        },
      ],
    },
  ];
}
