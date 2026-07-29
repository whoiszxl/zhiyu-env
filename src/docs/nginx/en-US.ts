import type { DocChapter } from "../docTypes";

export function buildNginxDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "About Nginx",
      navHint: "Web server · Reverse proxy",
      title: "What is Nginx",
      intro:
        "Nginx is the most widely deployed open-source web server and reverse proxy server. Zhiyu bundles a precompiled Nginx that listens on 127.0.0.1:{port} by default, suitable for local development of static sites, proxying API requests, and SPA frontend debugging.",
      blocks: [
        {
          kind: "text",
          value:
            "Nginx in Zhiyu runs in minimal mode (daemon off, master_process off, worker_processes 1). Zhiyu manages the process lifecycle directly, so no system-level Nginx or Homebrew installation is needed.",
        },
        {
          kind: "text",
          value: "Nginx's core capabilities within Zhiyu include:",
        },
        {
          kind: "list",
          items: [
            "Static file serving — files in the html directory are served directly to the browser via HTTP.",
            "try_files — essential for SPA frontend routing; rewrites all paths to index.html.",
            "Reverse proxy (proxy_pass) — forwards /api/ requests to local Node.js / Spring Boot / Go services.",
            "access_log and error_log — records all requests and errors for easy troubleshooting.",
          ],
        },
        {
          kind: "table",
          head: ["Item", "Value", "Description"],
          rows: [
            ["Access URL", `http://127.0.0.1:{port}`, "Local only"],
            ["Static directory", "~/.devbox/instances/nginx/default/html", "Site root"],
            ["Config file", "~/.devbox/instances/nginx/default/conf/nginx.conf", "Edit in the config tab"],
            ["Log directory", "~/.devbox/instances/nginx/default/logs", "access.log / error.log"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Zhiyu Nginx is for local development only",
          value:
            "The service listens only on 127.0.0.1 and is not exposed to the LAN or public internet. gzip, rewrite, and other features are disabled at compile time to keep it lightweight. Do not use the Zhiyu Nginx in production.",
        },
      ],
    },
    {
      id: "static",
      navLabel: "Static File Serving",
      navHint: "HTML · CSS · JS",
      title: "Deploy a Static Website",
      intro:
        "Place HTML, CSS, and JavaScript files in the site directory, and Nginx will automatically serve them via HTTP. The default index page is index.html.",
      blocks: [
        {
          kind: "text",
          value: "Example static site directory structure:",
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
            "The full static directory path is displayed in the Sites tab.",
            "Add HTML files directly to that directory and access them through the browser.",
            "Common MIME types are supported: .html, .css, .js, .json, .png, .jpg, .svg, and more.",
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Copy files to the static directory",
          code: `cp my-website/* ~/.devbox/instances/nginx/default/html/`,
        },
      ],
    },
    {
      id: "spa",
      navLabel: "SPA try_files",
      navHint: "Vue · React · Routing",
      title: "Configure SPA Frontend Routing",
      intro:
        "Frontend routing libraries like Vue Router and React Router need all paths rewritten to index.html. Nginx's try_files directive makes this straightforward.",
      blocks: [
        {
          kind: "code",
          lang: "nginx",
          caption: "Add try_files to the config file",
          code: `location / {
    try_files $uri $uri/ /index.html;
}`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Do not use rewrite rules",
          value:
            "The rewrite module is disabled at compile time in Zhiyu's Nginx to keep it lightweight. Use try_files for SPA routing instead — it is cleaner and the recommended approach.",
        },
        {
          kind: "list",
          items: [
            "After editing the config, save it in the Config tab — nginx -t will run automatically for validation.",
            "If Nginx is running, restart the service for the new config to take effect.",
            "When changing the port, restart from the running state; do not force-restart a port that is already in use.",
          ],
        },
      ],
    },
    {
      id: "proxy",
      navLabel: "Reverse Proxy",
      navHint: "API · Forwarding · proxy_pass",
      title: "Reverse Proxy to a Local API Service",
      intro:
        "Nginx can forward requests starting with /api/ to a local Node.js, Spring Boot, or Go backend, eliminating cross-origin issues during frontend development.",
      blocks: [
        {
          kind: "code",
          lang: "nginx",
          caption: "Vue / Vite frontend development proxy example",
          code: `# Forward /api/ requests to the Vite dev server
location /api/ {
    proxy_pass http://127.0.0.1:5173/;
}`,
        },
        {
          kind: "code",
          lang: "nginx",
          caption: "Node.js Express / Fastify API proxy example",
          code: `location /api/ {
    proxy_pass http://127.0.0.1:3000/;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
}`,
        },
        {
          kind: "code",
          lang: "nginx",
          caption: "Spring Boot API proxy example",
          code: `location /api/ {
    proxy_pass http://127.0.0.1:8080/;
    proxy_set_header Host $host;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
}`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "The proxy target must be a local address",
          value:
            "Zhiyu is designed for local development. The first version of the reverse proxy only supports 127.0.0.1 or localhost as targets. Do not use Zhiyu's Nginx as a public-facing reverse proxy in production.",
        },
      ],
    },
    {
      id: "logs",
      navLabel: "View Logs",
      navHint: "Access · Error",
      title: "View Access and Error Logs",
      intro:
        "Nginx records all HTTP requests to access.log and errors and warnings to error.log. You can view and switch between the two logs in the Runtime Logs tab.",
      blocks: [
        {
          kind: "list",
          items: [
            "access.log — method, path, status code, and response size for each HTTP request.",
            "error.log — Nginx startup, config reload, and runtime error information.",
            "Click the Refresh button at the top of the tab to get the latest logs.",
            "Use the Clear button to clean log files; a confirmation prompt appears before clearing.",
            "At most 64 KiB of the log tail is read at a time; the full file is never loaded at once.",
          ],
        },
        {
          kind: "table",
          head: ["Log", "Path", "Purpose"],
          rows: [
            ["access.log", "~/.devbox/instances/nginx/default/logs/access.log", "HTTP request records"],
            ["error.log", "~/.devbox/instances/nginx/default/logs/error.log", "Errors and warnings"],
            ["stdout.log", "~/.devbox/instances/nginx/default/logs/stdout.log", "Process stdout"],
            ["stderr.log", "~/.devbox/instances/nginx/default/logs/stderr.log", "Process stderr"],
          ],
        },
      ],
    },
    {
      id: "port",
      navLabel: "Change Port",
      navHint: "8081 · Port conflict",
      title: "Change the Listening Port",
      intro:
        "If the default port 8081 is occupied by another service, you can modify the listen directive in the config file.",
      blocks: [
        {
          kind: "code",
          lang: "nginx",
          caption: "Change the listening port to 3000",
          code: `server {
    listen 127.0.0.1:3000;
    server_name localhost;
    # ...
}`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Port conflict details",
          value:
            "If the specified port is already in use, Nginx will fail to start and show 'bind() to 127.0.0.1:XXXX failed (48: Address already in use)' in error.log. Use Zhiyu's built-in Port Checker tool to confirm port status.",
        },
        {
          kind: "list",
          items: [
            "Nginx listens only on 127.0.0.1, never on 0.0.0.0 or [::].",
            "After changing the port, save the config and restart Nginx for the change to take effect.",
            "When changing the port while Nginx is running, Zhiyu will prompt a restart but will not force it automatically.",
          ],
        },
      ],
    },
    {
      id: "validate",
      navLabel: "Config Validation",
      navHint: "nginx -t",
      title: "Validate Config Changes",
      intro:
        "Zhiyu automatically runs nginx -t for validation when you save the config file. If validation fails, the config is not written and the original configuration remains unchanged.",
      blocks: [
        {
          kind: "list",
          items: [
            "Edit nginx.conf in the Config tab.",
            "After clicking Save, Zhiyu writes the content to a temporary file and runs nginx -t.",
            "If validation passes, the temporary file replaces the live config.",
            "If validation fails, the error message is displayed in the frontend and the original config is preserved.",
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "You can also validate manually in the terminal",
          code: `~/.devbox/installations/nginx/1.30/bin/nginx \\
  -t \\
  -c ~/.devbox/instances/nginx/default/conf/nginx.conf \\
  -p ~/.devbox/instances/nginx/default/`,
        },
      ],
    },
  ];
}