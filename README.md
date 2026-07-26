<p align="center">
  <img src="assets/devbox-icon.svg" width="96" height="96" alt="智屿 Logo">
</p>

<h1 align="center">智屿 · Zhiyu Env</h1>

<p align="center">
  <strong>轻量本地开发环境管理工具</strong><br>
  不用 Docker，不用虚拟机，不污染系统环境。
</p>

<p align="center">
  <a href="README.md">简体中文</a> ·
  <a href="README_EN.md">English</a>
</p>

<p align="center">
  <img alt="Status" src="https://img.shields.io/badge/status-alpha-df552f">
  <img alt="Platform" src="https://img.shields.io/badge/platform-macOS%20Apple%20Silicon-20241c">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-20241c?logo=rust">
  <img alt="Tauri" src="https://img.shields.io/badge/Tauri-2-24c8db?logo=tauri">
  <img alt="Vue" src="https://img.shields.io/badge/Vue-3-42b883?logo=vuedotjs">
  <img alt="License" src="https://img.shields.io/badge/license-Apache--2.0-blue">
</p>

## 智屿是什么？

智屿是一个面向个人开发者的桌面端本地开发环境管理工具。

它不创建容器或虚拟机，而是将 Redis、MySQL、PostgreSQL 等服务安装到当前用户目录，通过 Rust 直接管理服务进程。你可以在一个轻量桌面应用中完成安装、启动、停止、配置、日志查看、资源监控和本地数据调试。

智屿专注于“快速拥有一个能用的本地服务”，不处理生产环境部署、集群编排和高可用。

## 为什么选择智屿？

- **没有 Docker Runtime**：不需要常驻 Docker Desktop。
- **没有虚拟机**：服务直接作为本机用户进程运行。
- **不污染系统**：不写入 `/usr/local`，不要求全局安装数据库。
- **按版本隔离**：程序安装在独立的版本目录中。
- **开箱即用**：自动下载官方发行包或源码包，并校验 SHA-256。
- **轻量可见**：实时查看 CPU、内存、运行时间和磁盘占用。
- **开发工具内置**：直接浏览数据、执行受限命令和检查端口。

## 已支持的服务

| 服务 | 当前版本 | 默认端口 | 内置开发能力 |
| --- | ---: | ---: | --- |
| Redis | 7.2.15 | 6379 | Key 浏览、类型与 TTL 查看、命令台 |
| MySQL | 8.4.10 | 3306 | 数据库与表浏览、字段说明、SQL 命令台 |
| PostgreSQL | 17.10 | 5432 | Schema 与表浏览、字段说明、SQL 命令台 |
| MongoDB | 8.0.26 | 27017 | 数据库与集合浏览、字段识别、JSON 命令台 |
| Mailpit | 1.30.5 | 1025 / 8025 | 本地邮件捕获、邮件列表与正文查看 |

所有服务均支持：

- 自动安装
- 启动、停止和重启
- PID 与运行状态检测
- 配置文件编辑
- 运行日志查看
- CPU、内存和运行时间监控
- 程序、数据、日志、配置及下载缓存的磁盘占用统计

此外，智屿提供一个只读的 **TCP 端口检查器**，用于识别本机监听端口及其所属进程。

## 界面能力

### 服务概览

每个服务都有独立的详情页面，可查看运行状态、PID、本地端点、实时资源曲线、磁盘占用和文件路径。

### 数据调试

- Redis：扫描 Key、查看值、类型、TTL 和内存大小。
- MySQL / PostgreSQL：浏览数据库、数据表、字段结构和前 100 行数据。
- MongoDB：浏览数据库、集合、推断字段类型并预览文档。
- 数据库类型旁提供中文解释角标。

### 安全命令台

智屿内置 Redis、SQL 和 MongoDB 命令台。阻塞服务或明显危险的管理命令会被禁止，清空或删除数据的命令需要二次确认。

### 本地邮件沙箱

Mailpit 只监听 `127.0.0.1`，不会将邮件投递到外部服务器：

```text
SMTP_HOST=127.0.0.1
SMTP_PORT=1025
SMTP_AUTH=false
SMTP_TLS=false
```

Web/API 地址为 `http://127.0.0.1:8025`。智屿中的邮件 HTML 以源码文本显示，不会直接执行邮件内的脚本或加载远程样式。

## 工作原理

```mermaid
flowchart LR
    UI["Vue 3 桌面界面"] -->|"Tauri Commands"| APP["Rust / Tauri 后端"]
    APP --> CORE["DevBox Core"]
    CORE --> INSTALLER["下载、校验与安装"]
    CORE --> PROCESS["进程与 PID 管理"]
    CORE --> CONFIG["配置、数据与日志"]
    PROCESS --> SERVICES["Redis · MySQL · PostgreSQL · MongoDB · Mailpit"]
    CONFIG --> HOME["~/.devbox/"]
```

智屿不会修改系统 PATH。所有运行参数、PID、配置、数据和日志都保存在用户目录。

## 数据目录

```text
~/.devbox/
├── downloads/                 # 已校验的安装包缓存
├── installations/             # 按服务和版本隔离的程序
│   ├── redis/7.2/
│   ├── mysql/8.4/
│   ├── postgres/17/
│   ├── mongodb/8.0/
│   └── mailpit/1.30/
├── instances/                 # 当前服务实例
│   └── <service>/default/
│       ├── conf/
│       ├── data/
│       ├── logs/
│       ├── run/
│       └── service.json
└── tmp/                       # 安装过程临时文件
```

## 当前平台支持

当前版本优先支持：

- macOS
- Apple Silicon（ARM64）

项目仍处于 Alpha 阶段。安装器中的发行包、校验值和构建流程目前都针对 Apple Silicon 验证，尚不建议在日常重要数据或生产环境中使用。

## 本地开发

### 环境要求

- macOS Apple Silicon
- Rust stable
- Node.js 20.19+ 或兼容 Vite 7 的更新版本
- npm
- Xcode Command Line Tools

### 启动桌面端

```bash
npm install
npm run tauri dev
```

### 构建

```bash
npm run tauri build
```

### 测试

```bash
cargo test --workspace
npm run build
```

部分真实服务集成测试默认标记为 `ignored`，避免自动修改或启动本机服务。

## 项目结构

```text
zhiyu-env/
├── crates/devbox-core/        # 服务抽象、安装器、进程与配置管理
├── src-tauri/                 # Tauri Commands 与数据库工具接口
├── src/                       # Vue 3 + TypeScript 桌面界面
├── assets/                    # 项目资源
├── Cargo.toml                 # Rust workspace
└── package.json               # 前端与 Tauri 脚本
```

核心服务统一实现 `ServiceManager`：

```text
install · start · stop · restart · status
```

## 安全边界

- 下载内容必须通过预置 SHA-256 校验。
- 服务默认仅用于本地开发，不提供生产级安全加固。
- Mailpit 强制使用本地监听，不启用 SMTP 中继或转发。
- 配置文件保存前会生成 `.bak` 备份。
- 邮件 HTML 不直接渲染。
- 数据命令台对阻塞命令和破坏性命令进行限制。

> 智屿不是容器隔离方案。服务进程仍然以当前用户权限直接运行在 macOS 上。

## 路线图

- 多版本选择与多实例管理
- 安装缓存清理
- 数据备份与恢复
- Linux 与 Intel Mac 支持
- 更多轻量开发工具
- DuckDB 本地文件查询器

## 参与贡献

欢迎提交 Issue 和 Pull Request。建议一次只处理一个清晰模块，并同时提供：

- 修改内容
- 设计原因
- 测试方式

提交代码前请运行：

```bash
cargo fmt --all
cargo test --workspace
npm run build
```

## 许可证

本项目根据 [Apache License 2.0](LICENSE) 开源。
