import type { DocChapter } from "../docTypes";

export function buildFtpDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 FTP",
      navHint: "本地文件传输",
      title: "智屿 FTP Server",
      intro:
        "智屿使用 SFTPGo 的便携模式提供 FTP 服务。它是独立二进制，不依赖系统安装，默认只监听本机。",
      blocks: [
        {
          kind: "table",
          head: ["项目", "默认值", "说明"],
          rows: [
            ["地址", `127.0.0.1:${port}`, "仅本机可访问"],
            ["用户名", "zhiyu", "本地开发账号"],
            ["共享目录", "~/.devbox/instances/ftp/default/data", "上传文件保存在这里"],
            ["被动端口", "50000–50009", "用于目录列表和文件传输"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "明文协议",
          value:
            "普通 FTP 不会加密账号和文件内容。智屿默认限制为 127.0.0.1，请勿把监听地址改为 0.0.0.0 或用于公网传输。",
        },
      ],
    },
    {
      id: "use",
      navLabel: "连接与传输",
      navHint: "curl · 客户端",
      title: "上传和下载文件",
      intro: "可使用 FileZilla、Cyberduck、curl 或语言标准库连接。",
      blocks: [
        {
          kind: "code",
          lang: "bash",
          caption: "使用 curl 上传和下载",
          code: `curl --ftp-pasv -T ./demo.txt \\
  "ftp://zhiyu:zhiyu-local-ftp-2026@127.0.0.1:${port}/"

curl --ftp-pasv \\
  "ftp://zhiyu:zhiyu-local-ftp-2026@127.0.0.1:${port}/demo.txt" \\
  -o demo.txt`,
        },
        {
          kind: "list",
          items: [
            "从「连接」页复制地址、用户名和密码。",
            "客户端连接模式建议选择被动模式（PASV）。",
            "服务停止后，共享目录中的文件仍会保留。",
          ],
        },
      ],
    },
    {
      id: "troubleshooting",
      navLabel: "排查问题",
      navHint: "端口 · 日志",
      title: "连接失败时怎么检查",
      intro: "FTP 同时使用控制端口和数据端口，目录列表失败通常与被动端口有关。",
      blocks: [
        {
          kind: "list",
          items: [
            "先在「端口检查器」确认 2121 没有被其他进程占用。",
            "能登录但无法列目录时，确认 50000–50009 没有端口冲突。",
            "查看「运行日志」中的 stderr.log 获取启动错误。",
            "配置修改后需要重启服务才会生效。",
          ],
        },
      ],
    },
  ];
}
