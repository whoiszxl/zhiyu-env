use super::common::ManagedService;
use crate::config::{ServiceConfig, ServiceKind};
use crate::error::{DevBoxError, Result};
use crate::service::ServiceManager;
use crate::status::ServiceStatus;

pub struct CaddyService {
    inner: ManagedService,
}

impl CaddyService {
    pub fn new(config: ServiceConfig) -> Result<Self> {
        if config.kind != ServiceKind::Caddy {
            return Err(DevBoxError::InvalidConfig(
                "CaddyService requires kind=caddy".into(),
            ));
        }
        Ok(Self {
            inner: ManagedService::new(config)?,
        })
    }
}

impl ServiceManager for CaddyService {
    fn install(&self) -> Result<()> {
        let config = &self.inner.config;
        let port = config.port;
        let instance_dir = config.instance_dir.display();
        let contents = format!(
            "\
# 智屿 Caddy 本地开发配置
{{
    auto_https off
    admin off
}}

http://127.0.0.1:{port} {{
    root * {instance_dir}/html
    file_server

    log {{
        output file {instance_dir}/logs/access.log
    }}

    # handle_path /api/* {{
    #     reverse_proxy 127.0.0.1:3000
    # }}
}}
",
        );
        self.inner.install("Caddyfile", &contents)?;

        let html_dir = config.instance_dir.join("html");
        std::fs::create_dir_all(&html_dir).map_err(DevBoxError::Io)?;
        let index_path = html_dir.join("index.html");
        if !index_path.exists() {
            let index_html = format!(
                r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>智屿 Caddy</title>
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
  <h1>智屿 Caddy</h1>
  <p>Caddy 已成功运行。</p>
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
