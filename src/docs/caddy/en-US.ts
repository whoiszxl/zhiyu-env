import type { DocChapter } from "../docTypes";

export function buildCaddyDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro", navLabel: "Meet Caddy", navHint: "Web server · Caddyfile",
      title: "What is Caddy",
      intro:
        "Caddy is a modern web server written in Go, known for its extremely concise Caddyfile configuration and automatic HTTPS. Zhiyu ships with an official pre-built Caddy binary that listens on 127.0.0.1:{port} by default, ideal for serving static sites locally and debugging reverse proxies.",
      blocks: [
        { kind: "text", value: "Compared with Nginx, Caddy's configuration file is much more concise. A complete static file service takes only 3 lines of config, and a reverse proxy needs just a single reverse_proxy directive. The Caddy bundled with Zhiyu has auto_https and the admin API disabled, keeping only the core features needed for local development." },
        { kind: "text", value: "Core capabilities of Caddy:", },
        { kind: "list", items: [
          "Static file serving — the file_server directive exposes the html directory over HTTP.",
          "Reverse proxy — a single reverse_proxy line forwards requests to a local backend service.",
          "rewrite / try_files — supports rewriting SPA frontend routes to index.html.",
          "access_log — every request is logged to access.log for easy troubleshooting.",
        ]},
        { kind: "table", head: ["Item", "Value", "Notes"], rows: [
          ["URL", `http://127.0.0.1:${port}`, "Listens on localhost only"],
          ["Static directory", "~/.devbox/instances/caddy/default/html", "Site root"],
          ["Config file", "~/.devbox/instances/caddy/default/conf/Caddyfile", "Edit in the Config tab"],
          ["Logs", "~/.devbox/instances/caddy/default/logs/access.log", "Request log"],
        ]},
        { kind: "callout", tone: "warn", title: "Local development only",
          value: "The service only listens on 127.0.0.1, and auto_https is disabled. Do not use the Caddy bundled with Zhiyu in production." },
      ],
    },
    {
      id: "static", navLabel: "Static file serving", navHint: "file_server",
      title: "Deploying a static site",
      intro: "Drop your HTML, CSS, and JavaScript files into the site directory and Caddy will serve them over HTTP automatically. The default index page is index.html.",
      blocks: [
        { kind: "text", value: "Example directory structure for a static site:" },
        { kind: "code", lang: "text", caption: "~/.devbox/instances/caddy/default/html/", code: `html/
├── index.html
├── style.css
├── app.js
└── images/
    └── logo.png` },
        { kind: "list", items: [
          "The Site tab shows the full path to the static directory.",
          "Add HTML files directly to that directory to make them accessible from the browser.",
          "Caddy handles common MIME types out of the box — no need to configure mime.types." ],
        },
      ],
    },
    {
      id: "spa", navLabel: "SPA try_files", navHint: "URL rewrite",
      title: "Configuring SPA frontend routing",
      intro: "Frontend routers such as Vue Router and React Router need every path rewritten to index.html. Caddy's try_files directive makes this trivial.",
      blocks: [
        { kind: "code", lang: "caddyfile", caption: "Add try_files to the Caddyfile", code: `http://127.0.0.1:${port} {
    root * /path/to/html
    try_files {path} /index.html
    file_server
}` },
        { kind: "list", items: [
          "try_files first attempts the real file path and falls back to index.html when it does not exist.",
          "Restart Caddy after editing the config for changes to take effect.",
          "Editing Caddy's config is simpler than Nginx — no reload needed, just restart." ],
        },
      ],
    },
    {
      id: "proxy", navLabel: "Reverse proxy", navHint: "API · reverse_proxy",
      title: "Reverse proxy to a local API service",
      intro: "Caddy's reverse_proxy directive forwards requests on a given path to a local backend service, sidestepping CORS issues during frontend development.",
      blocks: [
        { kind: "code", lang: "caddyfile", caption: "Vue / Vite dev proxy example", code: `handle_path /api/* {
    reverse_proxy 127.0.0.1:5173
}` },
        { kind: "code", lang: "caddyfile", caption: "Node.js Express API proxy", code: `handle_path /api/* {
    reverse_proxy 127.0.0.1:3000 {
        header_up Host {host}
        header_up X-Real-IP {remote_host}
    }
}` },
        { kind: "code", lang: "caddyfile", caption: "Spring Boot API proxy", code: `handle_path /api/* {
    reverse_proxy 127.0.0.1:8080 {
        header_up X-Forwarded-For {remote_host}
    }
}` },
        { kind: "callout", tone: "tip", title: "handle_path strips the prefix",
          value: "With handle_path /api/*, /api/users is forwarded as /users. If your backend expects the /api prefix to be preserved, use handle instead of handle_path." },
      ],
    },
    {
      id: "logs", navLabel: "Viewing logs", navHint: "access.log",
      title: "Viewing access logs",
      intro: "Caddy records every HTTP request to access.log. The Runtime Logs tab lets you view and switch between log files.",
      blocks: [
        { kind: "list", items: [
          "access.log — the method, path, status code, and response size of each HTTP request.",
          "stdout.log — Caddy's console output, including startup information.",
          "stderr.log — Caddy's error output.",
          "Click the Refresh button at the top of the tab to fetch the latest log entries.",
        ]},
        { kind: "table", head: ["Log", "Path", "Purpose"], rows: [
          ["access.log", "~/.devbox/instances/caddy/default/logs/access.log", "HTTP request records"],
          ["stdout.log", "~/.devbox/instances/caddy/default/logs/stdout.log", "Process standard output"],
          ["stderr.log", "~/.devbox/instances/caddy/default/logs/stderr.log", "Process standard error"],
        ]},
      ],
    },
    {
      id: "port", navLabel: "Change port", navHint: "8082 · Port conflicts",
      title: "Changing the listen port",
      intro: "If the default port 8082 is already taken, you can change the listen address in the Caddyfile.",
      blocks: [
        { kind: "code", lang: "caddyfile", caption: "Switch to port 3000", code: `http://127.0.0.1:3000 {
    root * /path/to/html
    file_server
}` },
        { kind: "callout", tone: "warn", title: "About port conflicts",
          value: "If the port is already in use, Caddy will fail to start with 'address already in use'. Use Zhiyu's built-in Port Checker tool to verify port availability." },
        { kind: "list", items: [
          "Caddy only listens on 127.0.0.1 — it never binds to 0.0.0.0 or [::].",
          "Restart Caddy after changing the port for the change to take effect.",
        ]},
      ],
    },
    {
      id: "validate", navLabel: "Config validation", navHint: "caddy validate",
      title: "Validating after config changes",
      intro: "Caddy validates the Caddyfile syntax automatically at startup. If the config is invalid, Caddy refuses to start and reports the exact error location in stderr.log.",
      blocks: [
        { kind: "list", items: [
          "Edit the Caddyfile in the Config tab.",
          "After saving, restart Caddy — the syntax is validated automatically on startup.",
          "If startup fails, check stderr.log in the Runtime Logs tab for the specific error.",
          "Caddy's error messages typically include line and column numbers that pinpoint the syntax error.",
        ]},
        { kind: "code", lang: "bash", caption: "Validate the config manually", code: `~/.devbox/installations/caddy/2.11/bin/caddy \\
  validate \\
  --config ~/.devbox/instances/caddy/default/conf/Caddyfile` },
      ],
    },
    {
      id: "compare", navLabel: "Caddy vs Nginx", navHint: "Comparison",
      title: "Choosing between Caddy and Nginx",
      intro: "Zhiyu provides both Nginx and Caddy as web servers, and they serve slightly different purposes.",
      blocks: [
        { kind: "table", head: ["", "Caddy", "Nginx"], rows: [
          ["Config syntax", "Caddyfile (minimal)", "nginx.conf (traditional)"],
          ["Install method", "Pre-built Go binary", "Compiled from C source"],
          ["Reverse proxy", "One-line reverse_proxy", "proxy_pass block"],
          ["SPA routing", "try_files", "try_files"],
          ["Automatic HTTPS", "Supported (disabled in Zhiyu)", "Manual configuration required"],
          ["Module ecosystem", "Go plugins", "C modules (compile-time)"],
          ["Memory usage", "~10-30 MB", "~2-5 MB"],
        ]},
        { kind: "callout", tone: "tip", title: "Recommendation",
          value: "Pick Caddy if you want minimal configuration and a modern experience. Pick Nginx if you need the leanest footprint and traditional syntax. Both fit local development well and can run side by side on different ports." },
      ],
    },
  ];
}
