import type { DocChapter } from "./docTypes";

export function buildMinioDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "连接 MinIO",
      navHint: "S3 API · Console",
      title: "本地 S3 兼容对象存储",
      intro:
        "MinIO 为本地应用提供兼容 Amazon S3 的 API，适合调试文件上传、Bucket、预签名 URL 和对象权限。",
      blocks: [
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["S3 Endpoint", `http://127.0.0.1:${port}`, "应用连接地址"],
            ["Web Console", "http://127.0.0.1:9001", "浏览器管理界面"],
            ["Access Key", "zhiyuadmin", "本地开发账号"],
            ["Secret Key", "zhiyu-local-minio-2026", "本地开发密码"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "仅用于本地开发",
          value:
            "固定开发凭证不能用于生产环境。MinIO 社区仓库已归档，智屿保留它用于验证存量项目兼容性。",
        },
      ],
    },
  ];
}
