import type { DocChapter } from "./docTypes";

export function buildCaddyDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro", navLabel: "认识 Caddy", navHint: "Web 服务器 · Caddyfile",
      title: "Caddy 是什么",
      intro:
        "Caddy 是 Go 语言编写的现代化 Web 服务器，以极其简洁的 Caddyfile 配置和自动 HTTPS 闻名。智屿内置了官方预编译的 Caddy 二进制，默认监听 127.0.0.1:{port}，适合本地开发静态站点和反向代理调试。",
      blocks: [
        { kind: "text", value: "和 Nginx 相比，Caddy 的配置文件更加简洁。一个完整的静态文件服务只需要 3 行配置。反向代理也只需要一行 reverse_proxy 指令。智屿中的 Caddy 关闭了 auto_https 和 admin API，只保留本地开发所需的核心功能。" },
        { kind: "text", value: "Caddy 的核心能力：", },
        { kind: "list", items: [
          "静态文件服务 —— file_server 指令让 html 目录直接对外提供 HTTP 服务。",
          "反向代理 —— reverse_proxy 一行即可将请求转发到本地后端服务。",
          "rewrite / try_files —— 支持 SPA 前端路由重写到 index.html。",
          "access_log —— 所有请求日志记录到 access.log，方便排查。",
        ]},
        { kind: "table", head: ["项目", "值", "说明"], rows: [
          ["访问地址", `http://127.0.0.1:${port}`, "仅监听本地"],
          ["静态目录", "~/.devbox/instances/caddy/default/html", "站点根目录"],
          ["配置文件", "~/.devbox/instances/caddy/default/conf/Caddyfile", "在配置标签页编辑"],
          ["日志", "~/.devbox/instances/caddy/default/logs/access.log", "请求日志"],
        ]},
        { kind: "callout", tone: "warn", title: "仅用于本地开发",
          value: "服务只监听 127.0.0.1，auto_https 已关闭。不要将智屿中的 Caddy 用于生产环境。" },
      ],
    },
    {
      id: "static", navLabel: "静态文件服务", navHint: "file_server",
      title: "部署静态网站",
      intro: "将 HTML、CSS、JavaScript 文件放到站点目录后，Caddy 会自动通过 HTTP 提供服务。默认首页是 index.html。",
      blocks: [
        { kind: "text", value: "静态站点的目录结构示例：" },
        { kind: "code", lang: "text", caption: "~/.devbox/instances/caddy/default/html/", code: `html/
├── index.html
├── style.css
├── app.js
└── images/
    └── logo.png` },
        { kind: "list", items: [
          "在「站点」标签页可以看到静态目录的完整路径。",
          "直接向该目录添加 HTML 文件即可通过浏览器访问。",
          "Caddy 支持常见的 MIME 类型，无需额外配置 mime.types。" ],
        },
      ],
    },
    {
      id: "spa", navLabel: "SPA try_files", navHint: "路由重写",
      title: "配置 SPA 前端路由",
      intro: "Vue Router 和 React Router 等前端路由库需要把所有路径都重写到 index.html。Caddy 的 try_files 指令可以轻松实现。",
      blocks: [
        { kind: "code", lang: "caddyfile", caption: "在 Caddyfile 中添加 try_files", code: `http://127.0.0.1:${port} {
    root * /path/to/html
    try_files {path} /index.html
    file_server
}` },
        { kind: "list", items: [
          "try_files 指令首先尝试访问真实的文件路径，不存在时回退到 index.html。",
          "配置修改后重启 Caddy 即可生效。",
          "Caddy 的配置修改比 Nginx 更简单，不需要 reload —— 直接重启即可。" ],
        },
      ],
    },
    {
      id: "proxy", navLabel: "反向代理", navHint: "API · reverse_proxy",
      title: "反向代理到本地 API 服务",
      intro: "Caddy 的 reverse_proxy 指令可以将指定路径的请求转发到本地后端服务，避免前端开发时的跨域问题。",
      blocks: [
        { kind: "code", lang: "caddyfile", caption: "Vue / Vite 开发代理示例", code: `handle_path /api/* {
    reverse_proxy 127.0.0.1:5173
}` },
        { kind: "code", lang: "caddyfile", caption: "Node.js Express API 代理", code: `handle_path /api/* {
    reverse_proxy 127.0.0.1:3000 {
        header_up Host {host}
        header_up X-Real-IP {remote_host}
    }
}` },
        { kind: "code", lang: "caddyfile", caption: "Spring Boot API 代理", code: `handle_path /api/* {
    reverse_proxy 127.0.0.1:8080 {
        header_up X-Forwarded-For {remote_host}
    }
}` },
        { kind: "callout", tone: "tip", title: "handle_path 会去除前缀",
          value: "使用 handle_path /api/* 时，/api/users 会被转发为 /users。如果后端期望保留 /api 前缀，使用 handle 而不是 handle_path。" },
      ],
    },
    {
      id: "logs", navLabel: "查看日志", navHint: "access.log",
      title: "查看访问日志",
      intro: "Caddy 会记录所有 HTTP 请求到 access.log。在「运行日志」标签页可以查看和切换不同日志文件。",
      blocks: [
        { kind: "list", items: [
          "access.log —— 每个 HTTP 请求的方法、路径、状态码、响应大小。",
          "stdout.log —— Caddy 的控制台输出，包含启动信息。",
          "stderr.log —— Caddy 的错误输出。",
          "点击标签页顶部的「刷新」按钮获取最新日志。",
        ]},
        { kind: "table", head: ["日志", "路径", "用途"], rows: [
          ["access.log", "~/.devbox/instances/caddy/default/logs/access.log", "HTTP 请求记录"],
          ["stdout.log", "~/.devbox/instances/caddy/default/logs/stdout.log", "进程标准输出"],
          ["stderr.log", "~/.devbox/instances/caddy/default/logs/stderr.log", "进程标准错误"],
        ]},
      ],
    },
    {
      id: "port", navLabel: "修改端口", navHint: "8082 · 端口冲突",
      title: "修改监听端口",
      intro: "默认端口 8082 如果被其他服务占用，可以在 Caddyfile 中修改监听地址。",
      blocks: [
        { kind: "code", lang: "caddyfile", caption: "修改为 3000 端口", code: `http://127.0.0.1:3000 {
    root * /path/to/html
    file_server
}` },
        { kind: "callout", tone: "warn", title: "端口冲突说明",
          value: "如果端口被占用，Caddy 启动会失败并显示 'address already in use'。使用智屿内置的「端口检查器」工具确认端口状态。" },
        { kind: "list", items: [
          "Caddy 只监听 127.0.0.1，不会绑定 0.0.0.0 或 [::]。",
          "修改端口后需要重启 Caddy 才能生效。",
        ]},
      ],
    },
    {
      id: "validate", navLabel: "配置校验", navHint: "caddy validate",
      title: "配置修改后验证",
      intro: "Caddy 会在启动时自动校验 Caddyfile 语法。如果配置有误，Caddy 会拒绝启动并在 stderr.log 中显示具体错误位置。",
      blocks: [
        { kind: "list", items: [
          "在「配置文件」标签页中编辑 Caddyfile。",
          "保存后重启 Caddy，启动时自动校验语法。",
          "如果启动失败，查看「运行日志」标签页的 stderr.log 了解具体错误。",
          "Caddy 的错误信息通常包含行号和列号，直接定位到语法错误位置。",
        ]},
        { kind: "code", lang: "bash", caption: "手动验证配置", code: `~/.devbox/installations/caddy/2.11/bin/caddy \\
  validate \\
  --config ~/.devbox/instances/caddy/default/conf/Caddyfile` },
      ],
    },
    {
      id: "compare", navLabel: "Caddy vs Nginx", navHint: "对比",
      title: "Caddy 和 Nginx 怎么选",
      intro: "智屿同时提供了 Nginx 和 Caddy 两种 Web 服务器，它们的定位有所不同。",
      blocks: [
        { kind: "table", head: ["", "Caddy", "Nginx"], rows: [
          ["配置语法", "Caddyfile（极简）", "nginx.conf（传统）"],
          ["安装方式", "预编译 Go 二进制", "源码编译 C"],
          ["反向代理", "reverse_proxy 一行", "proxy_pass 配置"],
          ["SPA 路由", "try_files", "try_files"],
          ["自动 HTTPS", "支持（智屿已关闭）", "需手动配置"],
          ["模块生态", "Go 插件", "C 模块（编译时）"],
          ["内存占用", "约 10-30 MB", "约 2-5 MB"],
        ]},
        { kind: "callout", tone: "tip", title: "建议",
          value: "如果追求极简配置和现代化体验，选 Caddy。如果需要极致轻量和传统配置语法，选 Nginx。两者都适合本地开发场景，可以同时运行在不同端口。" },
      ],
    },
  ];
}
