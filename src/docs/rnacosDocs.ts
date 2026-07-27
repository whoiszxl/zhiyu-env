import type { DocChapter } from "./docTypes";

export function buildRnacosDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "连接 rnacos",
      navHint: "Nacos 兼容 · Console",
      title: "轻量 Nacos 兼容服务",
      intro:
        "rnacos 使用 Rust 实现 Nacos 兼容协议，适合本地调试配置中心、服务注册与发现，无需 Java Runtime。",
      blocks: [
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["Nacos HTTP", `http://127.0.0.1:${port}`, "1.x OpenAPI 与客户端"],
            ["Nacos gRPC", "127.0.0.1:9848", "2.x 客户端协议"],
            ["Web Console", "http://127.0.0.1:10848/rnacos/", "默认 admin / admin"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "仅限本地开发",
          value:
            "默认控制台账号密码为 admin / admin，OpenAPI 鉴权关闭。不要将此实例暴露到公网。",
        },
      ],
    },
  ];
}
