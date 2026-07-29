import type { DocChapter } from "../docTypes";

export function buildNginxDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 Nginx",
      navHint: "Web 服务器 · 反向代理",
      title: "Nginx 是什么",
      intro:
        "Nginx 是部署最广泛的开源 Web 服务器和反向代理服务器。智屿内置了编译好的 Nginx，默认监听 127.0.0.1:{port}，适合本地开发静态站点、代理 API 请求和 SPA 前端调试。",
      blocks: [
        {
          kind: "text",
          value:
            "智屿中的 Nginx 以最简模式运行（daemon off、master_process off、worker_processes 1），由智屿直接管理进程生命周期，不需要系统级 Nginx 或 Homebrew 安装。",
        },
        {
          kind: "text",
          value: "Nginx 的核心能力在智屿中包括：",
        },
        {
          kind: "list",
          items: [
            "静态文件服务 —— 将 html 目录下的文件直接通过 HTTP 提供给浏览器。",
            "try_files —— SPA 前端路由必备，所有路径重写到 index.html。",
            "反向代理（proxy_pass）—— 将 /api/ 请求转发到本地 Node.js / Spring Boot / Go 服务。",
            "access_log 和 error_log —— 记录所有请求和错误，方便排查问题。",
          ],
        },
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["访问地址", `http://127.0.0.1:{port}`, "仅监听本地"],
            ["静态目录", "~/.devbox/instances/nginx/default/html", "站点根目录"],
            ["配置文件", "~/.devbox/instances/nginx/default/conf/nginx.conf", "在配置标签页编辑"],
            ["日志目录", "~/.devbox/instances/nginx/default/logs", "access.log / error.log"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "智屿 Nginx 仅用于本地开发",
          value:
            "服务只监听 127.0.0.1，不暴露到局域网或公网。编译时关闭了 gzip、rewrite 等功能以保持轻量，不要将智屿中的 Nginx 用于生产环境。",
        },
      ],
    },
    {
      id: "static",
      navLabel: "静态文件服务",
      navHint: "HTML · CSS · JS",
      title: "部署静态网站",
      intro:
        "将 HTML、CSS、JavaScript 文件放到站点目录后，Nginx 会自动通过 HTTP 提供服务。默认首页是 index.html。",
      blocks: [
        {
          kind: "text",
          value: "静态站点的目录结构示例：",
        },
        {
          kind: "code",
          lang: "text",
          caption: "~/.devbox/instances/nginx/default/html/",
          code: `html/
├── index.html
├── style.css
├── app.js
├── images/
│   └── logo.png
└── favicon.ico`,
        },
        {
          kind: "list",
          items: [
            "在「站点」标签页可以看到静态目录的完整路径。",
            "直接向该目录添加 HTML 文件即可通过浏览器访问。",
            "支持常见的 MIME 类型：.html、.css、.js、.json、.png、.jpg、.svg 等。",
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "将文件复制到静态目录",
          code: `cp my-website/* ~/.devbox/instances/nginx/default/html/`,
        },
      ],
    },
    {
      id: "spa",
      navLabel: "SPA try_files",
      navHint: "Vue · React · 路由",
      title: "配置 SPA 前端路由",
      intro:
        "Vue Router 和 React Router 等前端路由库需要把所有路径都重写到 index.html。Nginx 的 try_files 指令可以轻松实现这一点。",
      blocks: [
        {
          kind: "code",
          lang: "nginx",
          caption: "在配置文件中添加 try_files",
          code: `location / {
    try_files $uri $uri/ /index.html;
}`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "不要使用 rewrite 规则",
          value:
            "智屿 Nginx 编译时关闭了 rewrite 模块以保持轻量。请使用 try_files 处理 SPA 路由，这是更简洁且推荐的做法。",
        },
        {
          kind: "list",
          items: [
            "配置修改后在「配置文件」标签页保存，会自动运行 nginx -t 校验。",
            "如果 Nginx 正在运行，建议重启服务让新配置生效。",
            "修改端口后在运行状态中重启，不要自动强制重启正在监听中的端口。",
          ],
        },
      ],
    },
    {
      id: "proxy",
      navLabel: "反向代理",
      navHint: "API · 转发 · proxy_pass",
      title: "反向代理到本地 API 服务",
      intro:
        "Nginx 可以将 /api/ 开头的请求转发到本地的 Node.js、Spring Boot 或 Go 后端服务，避免前端开发时的跨域问题。",
      blocks: [
        {
          kind: "code",
          lang: "nginx",
          caption: "Vue / Vite 前端开发代理示例",
          code: `# 将 /api/ 请求转发到 Vite 开发服务器
location /api/ {
    proxy_pass http://127.0.0.1:5173/;
}`,
        },
        {
          kind: "code",
          lang: "nginx",
          caption: "Node.js Express / Fastify API 代理示例",
          code: `location /api/ {
    proxy_pass http://127.0.0.1:3000/;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
}`,
        },
        {
          kind: "code",
          lang: "nginx",
          caption: "Spring Boot API 代理示例",
          code: `location /api/ {
    proxy_pass http://127.0.0.1:8080/;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
}`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "代理目标必须是本地地址",
          value:
            "智屿定位本地开发，第一版反向代理目标仅支持 127.0.0.1 或 localhost。不要在生产环境中使用智屿 Nginx 作为公网反向代理。",
        },
      ],
    },
    {
      id: "logs",
      navLabel: "查看日志",
      navHint: "Access · Error",
      title: "查看访问日志和错误日志",
      intro:
        "Nginx 会记录所有 HTTP 请求到 access.log，错误和警告信息记录到 error.log。在「运行日志」标签页可以查看和切换两种日志。",
      blocks: [
        {
          kind: "list",
          items: [
            "access.log —— 每个 HTTP 请求的方法、路径、状态码、响应大小。",
            "error.log —— Nginx 启动、配置重载和运行时错误信息。",
            "点击标签页顶部的「刷新」按钮获取最新日志。",
            "使用「清空」按钮可以清理日志文件，清空前需要二次确认。",
            "单次最多读取 64 KiB 的日志尾部内容，不会一次性读取完整文件。",
          ],
        },
        {
          kind: "table",
          head: ["日志", "路径", "用途"],
          rows: [
            ["access.log", "~/.devbox/instances/nginx/default/logs/access.log", "HTTP 请求记录"],
            ["error.log", "~/.devbox/instances/nginx/default/logs/error.log", "错误与警告"],
            ["stdout.log", "~/.devbox/instances/nginx/default/logs/stdout.log", "进程标准输出"],
            ["stderr.log", "~/.devbox/instances/nginx/default/logs/stderr.log", "进程标准错误"],
          ],
        },
      ],
    },
    {
      id: "port",
      navLabel: "修改端口",
      navHint: "8081 · 端口冲突",
      title: "修改监听端口",
      intro:
        "默认端口 8081 如果被其他服务占用，可以在配置文件中修改 listen 指令。",
      blocks: [
        {
          kind: "code",
          lang: "nginx",
          caption: "修改监听端口为 3000",
          code: `server {
    listen 127.0.0.1:3000;
    server_name localhost;
    # ...
}`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "端口冲突说明",
          value:
            "如果指定的端口已被占用，Nginx 启动会失败并在 error.log 中显示 'bind() to 127.0.0.1:XXXX failed (48: Address already in use)'。使用智屿内置的「端口检查器」工具确认端口状态。",
        },
        {
          kind: "list",
          items: [
            "Nginx 只监听 127.0.0.1，不会绑定 0.0.0.0 或 [::]。",
            "修改端口后需要保存配置并重启 Nginx 才能生效。",
            "Nginx 运行中修改端口时，智屿会提示重启但不会自动强制操作。",
          ],
        },
      ],
    },
    {
      id: "validate",
      navLabel: "配置校验",
      navHint: "nginx -t",
      title: "配置修改后验证",
      intro:
        "智屿在保存配置文件时会自动运行 nginx -t 进行校验。如果校验失败，配置不会写入，原有配置保持不变。",
      blocks: [
        {
          kind: "list",
          items: [
            "在「配置文件」标签页中编辑 nginx.conf。",
            "点击保存后，智屿会将内容写入临时文件并运行 nginx -t。",
            "校验通过后，临时文件替换正式配置。",
            "校验失败时，错误信息会显示在前端，原配置不变。",
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "也可以手动在终端验证",
          code: `~/.devbox/installations/nginx/1.30/bin/nginx \\
  -t \\
  -c ~/.devbox/instances/nginx/default/conf/nginx.conf \\
  -p ~/.devbox/instances/nginx/default/`,
        },
      ],
    },
  ];
}
