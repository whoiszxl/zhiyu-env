import type { DocChapter } from "./docTypes";

export function buildRustfsDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "连接 RustFS",
      navHint: "S3 API · Console",
      title: "Rust 实现的本地 S3 对象存储",
      intro:
        "RustFS 提供兼容 Amazon S3 的 API，可用于调试文件上传、Bucket、预签名 URL 和对象权限。",
      blocks: [
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["S3 Endpoint", `http://127.0.0.1:${port}`, "应用连接地址"],
            ["Web Console", "http://127.0.0.1:7001", "浏览器管理界面"],
            ["Access Key", "zhiyuadmin", "本地开发账号"],
            ["Secret Key", "zhiyu-local-rustfs-2026", "本地开发密码"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Beta 版本",
          value:
            "当前官方 macOS Apple Silicon 版本仍处于 Beta 阶段，仅建议用于本地开发验证；固定开发凭证不能用于生产环境。",
        },
      ],
    },
  ];
}
