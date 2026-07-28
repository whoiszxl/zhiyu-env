use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct NginxService {
    inner: ManagedService,
}

impl NginxService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Nginx {
            return Err(DevBoxError::InvalidConfig(
                "NginxService requires kind=nginx".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for NginxService {
    fn install(&self) -> Result<()> {
        let config = &self.inner.config;
        let port = config.port;
        let contents = format!(
            "\
worker_processes  1;
daemon off;
master_process off;
pid run/nginx.pid;
error_log logs/error.log;
events {{
    worker_connections 256;
}}
http {{
    include mime.types;
    default_type application/octet-stream;
    sendfile on;
    keepalive_timeout 30;

    access_log logs/access.log;

    server {{
        listen 127.0.0.1:{port};
        server_name localhost;
        charset utf-8;
        root html;
        index index.html;

        location / {{
            try_files $uri $uri/ =404;
        }}

        # location /api/ {{
        #     proxy_pass http://127.0.0.1:3000/;
        # }}
    }}
}}
",
        );
        self.inner.install("nginx.conf", &contents)?;

        let installation_dir = self.inner.config.executable
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf());
        if let Some(install_dir) = installation_dir {
            let source = install_dir.join("conf/mime.types");
            if source.is_file() {
                let dest = self.inner.config.config_dir().join("mime.types");
                std::fs::copy(&source, &dest).map_err(DevBoxError::Io)?;
            }
        }

        let html_dir = config.data_dir().join("html");
        std::fs::create_dir_all(&html_dir).map_err(DevBoxError::Io)?;
        let index_path = html_dir.join("index.html");
        if !index_path.exists() {
            let index_html = format!(
                r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>智屿 Nginx</title>
<style>
  body {{ margin:0; display:grid; min-height:100vh; place-items:center; font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif; color:#2c2e27; background:#f5f3ee; }}
  main {{ display:grid; gap:20px; padding:32px; text-align:center; }}
  h1 {{ margin:0; font-size:26px; font-weight:600; }}
  p {{ margin:0; color:#6e7069; font-size:15px; }}
  code {{ padding:8px 16px; border:1px solid #d2d1c9; border-radius:8px; background:#fcfcf8; font-family:"SFMono-Regular",Consolas,monospace; font-size:13px; }}
  a {{ color:#416b49; }}
</style>
</head>
<body>
<main>
  <h1>智屿 Nginx</h1>
  <p>Nginx 已成功运行。</p>
  <code>http://127.0.0.1:{port}</code>
  <p><small>将此目录作为静态站根目录，或配置反向代理来调试本地 API 服务。</small></p>
</main>
</body>
</html>"#
            );
            std::fs::write(index_path, index_html).map_err(DevBoxError::Io)?;
        }
        Ok(())
    }

    fn start(&self) -> Result<u32> {
        self.inner.start()
    }

    fn stop(&self) -> Result<()> {
        self.inner.stop()
    }

    fn force_stop(&self) -> Result<()> {
        self.inner.force_stop()
    }

    fn restart(&self) -> Result<u32> {
        self.inner.restart()
    }

    fn status(&self) -> Result<ServiceStatus> {
        self.inner.status()
    }

    fn repair(&self) -> Result<()> {
        self.inner.repair()
    }
}
