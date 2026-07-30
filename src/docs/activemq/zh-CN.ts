import type { DocChapter } from "../docTypes";

export function buildActivemqDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 ActiveMQ",
      navHint: "JMS · OpenWire",
      title: "ActiveMQ Classic 本地消息代理",
      intro:
        "ActiveMQ Classic 适合验证传统 Java/JMS、OpenWire、AMQP 和 STOMP 项目。智屿将程序、配置、数据与日志完整隔离在用户目录。",
      blocks: [
        {
          kind: "table",
          head: ["项目", "默认值", "说明"],
          rows: [
            ["OpenWire", `tcp://127.0.0.1:${port}`, "Java 客户端常用连接"],
            ["管理控制台", "http://127.0.0.1:8161/admin/", "队列、主题与消费者状态"],
            ["账号", "admin / admin", "仅用于本机开发"],
            ["Java", "6.2：17/21；6.3：25", "使用智屿安装并选择的 Java Runtime"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "只用于本地开发",
          value:
            "智屿会把传输端口绑定到 127.0.0.1。不要把默认账号和本地配置直接用于生产环境。",
        },
      ],
    },
    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "Queue · Topic",
      title: "连接并发送第一条消息",
      intro:
        "先在「语言环境」安装兼容的 Java，再启动 ActiveMQ。启动完成后可从「连接」页复制地址或打开管理控制台。",
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
            "Queue 适合多个消费者竞争处理任务，一条消息只交给其中一个消费者。",
            "Topic 适合广播事件，在线订阅者各自收到一份消息。",
            "连接失败时先检查 Java 版本、61616 端口与运行日志。",
          ],
        },
      ],
    },
    {
      id: "versions",
      navLabel: "版本选择",
      navHint: "Java 兼容性",
      title: "先按项目 Java 版本选择 ActiveMQ",
      intro:
        "ActiveMQ 6.2.8 是默认推荐版本；只有项目已经使用 Java 25 时才建议选择 6.3.0。",
      blocks: [
        {
          kind: "table",
          head: ["ActiveMQ", "Java Runtime", "建议"],
          rows: [
            ["6.2.8", "Java 17 或 21", "默认推荐，兼容常见现代项目"],
            ["6.3.0", "Java 25", "需要最新特性时选择"],
          ],
        },
      ],
    },
  ];
}
