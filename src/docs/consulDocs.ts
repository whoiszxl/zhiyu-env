import type { DocChapter } from "./docTypes";

export function buildConsulDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "连接 Consul",
      navHint: "服务发现 · KV",
      title: "本地单节点 Consul",
      intro:
        "智屿运行一个只监听本机的 Consul Server Agent，适合调试服务注册、健康检查、KV 配置和 DNS 服务发现。",
      blocks: [
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["HTTP API", `http://127.0.0.1:${port}`, "SDK 与 REST API"],
            ["Web UI", `http://127.0.0.1:${port}/ui/`, "服务和 KV 管理界面"],
            ["DNS", "127.0.0.1:8600", "DNS 服务发现"],
          ],
        },
      ],
    },
  ];
}
