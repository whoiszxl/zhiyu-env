import type { DocChapter } from "./docTypes";

export function buildMeilisearchDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 Meilisearch",
      navHint: "索引 · 全文搜索",
      title: "Meilisearch 是什么",
      intro:
        "Meilisearch 是面向应用开发的全文搜索引擎。把 JSON 文档写入索引后，即可通过 HTTP API 获得支持前缀和拼写容错的搜索结果。",
      blocks: [
        {
          kind: "table",
          head: ["项目", "本地配置", "说明"],
          rows: [
            ["HTTP 地址", `http://127.0.0.1:${port}`, "应用和 SDK 连接地址"],
            ["环境", "development", "本地开发模式"],
            ["分析数据", "关闭", "不会发送匿名遥测"],
            ["数据目录", "~/.devbox/instances/meilisearch/default/data", "索引文件"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "索引类似一张用于搜索的表",
          value:
            "每个索引包含一组 JSON 文档，并用一个字段作为主键。文档字段默认都可以搜索，后续可通过 API 调整可搜索、可过滤和可排序字段。",
        },
      ],
    },
  ];
}
