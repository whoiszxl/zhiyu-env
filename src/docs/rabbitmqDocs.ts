import type { DocChapter } from "./docTypes";

export function buildRabbitmqDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "连接 RabbitMQ",
      navHint: "AMQP · Management",
      title: "本地 RabbitMQ 消息代理",
      intro:
        "RabbitMQ 适合验证 AMQP 队列、交换机、路由键、确认和重试。智屿会携带独立 Erlang/OTP，不读取系统全局安装。",
      blocks: [
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["AMQP", `amqp://zhiyu:zhiyu-local-rabbitmq-2026@127.0.0.1:${port}/`, "应用连接"],
            ["Management", "http://127.0.0.1:15672", "队列与交换机管理"],
            ["Runtime", "Erlang/OTP 27.3.4.6", "随服务安装在用户目录"],
          ],
        },
      ],
    },
  ];
}
