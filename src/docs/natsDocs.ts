import type { DocChapter } from "./docTypes";

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
          title: "先用 Subject 组织消息",
          value:
            "推荐使用 orders.created、users.updated 这类点分 Subject。订阅时可用 * 匹配一个分段，用 > 匹配后续全部分段。",
        },
      ],
    },
    {
      id: "workflow",
      navLabel: "消息调试",
      navHint: "Publish · Subscribe",
      title: "在智屿里验证消息",
      intro:
        "“消息调试”页面提供发布和单次订阅，适合快速确认应用有没有正确连接和发送消息。",
      blocks: [
        {
          kind: "list",
          items: [
            "先点击“开始等待”，让智屿订阅目标 Subject。",
            "从应用或左侧发布区域发送一条消息。",
            "智屿收到第一条匹配消息后自动结束订阅并展示 Payload。",
            "单次等待最多 8 秒，不会在后台保留隐藏订阅。",
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Core NATS 消息默认不持久化",
          value:
            "没有活跃订阅者时，普通 Publish 消息不会被保存。需要持久化、重放或消费确认时，请使用 JetStream。",
        },
      ],
    },
  ];
}
