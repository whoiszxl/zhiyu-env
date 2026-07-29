import type { Directive } from "vue";

const english = new Map<string, string>([
  // Shared tool actions and states
  ["处理中", "Processing"],
  ["执行", "Run"],
  ["执行中", "Running"],
  ["查询", "Query"],
  ["查询中", "Querying"],
  ["连接", "Connect"],
  ["连接中…", "Connecting…"],
  ["断开", "Disconnect"],
  ["删除", "Delete"],
  ["复制", "Copy"],
  ["清空", "Clear"],
  ["关闭", "Close"],
  ["取消", "Cancel"],
  ["选择", "Choose"],
  ["刷新", "Refresh"],
  ["上传", "Upload"],
  ["上传中…", "Uploading…"],
  ["加载中…", "Loading…"],
  ["保存中", "Saving"],
  ["隐藏", "Hide"],
  ["显示", "Show"],
  ["名称", "Name"],
  ["大小", "Size"],
  ["类型", "Type"],
  ["状态", "Status"],
  ["操作", "Actions"],
  ["内容", "Content"],
  ["结果", "Result"],
  ["路径", "Path"],
  ["端口", "Port"],
  ["用户名", "Username"],
  ["密码", "Password"],
  ["秒", "sec"],
  ["分钟", "min"],
  ["小时", "hr"],
  ["天", "days"],
  ["条", "items"],
  ["个", ""],
  ["未设置", "Not set"],
  ["尚未选择文件", "No file selected"],
  ["刚才", "Just now"],
  ["清空所有未置顶的剪贴板记录？", "Clear all unpinned clipboard items?"],
  ["请输入标准 5 段 Cron：分 时 日 月 星期", "Enter a standard 5-field Cron expression: minute hour day month weekday"],
  ["未来两年内没有匹配的运行时间", "No matching run time in the next two years"],
  ["等待输入合法的 5 段 Cron 表达式", "Enter a valid 5-field Cron expression"],
  ["每 5 分钟执行一次", "Runs every 5 minutes"],
  ["每小时整点执行", "Runs at the start of every hour"],
  ["每天 00:00 执行", "Runs daily at 00:00"],
  ["每周一至周五 09:00 执行", "Runs weekdays at 09:00"],
  ["复制失败，请手动选中内容复制", "Copy failed. Select and copy the content manually."],
  ["cURL 已复制", "cURL copied"],
  ["响应已复制", "Response copied"],
  ["无法创建图片画布", "Could not create an image canvas"],
  ["PNG 导出失败", "PNG export failed"],
  ["当前系统 WebView 不支持二维码图片识别", "The system WebView does not support QR image scanning"],
  ["图片中没有识别到二维码", "No QR code was found in the image"],
  ["L · 约 7%", "L · about 7%"],
  ["M · 约 15%", "M · about 15%"],
  ["Q · 约 25%", "Q · about 25%"],
  ["H · 约 30%", "H · about 30%"],
  ["连接中", "Connecting"],
  ["WebSocket 连接成功", "WebSocket connected"],
  ["[二进制消息]", "[Binary message]"],
  ["WebSocket 连接或通信发生错误", "WebSocket connection or communication error"],
  ["SSE 连接成功", "SSE connected"],
  ["SSE 连接已关闭", "SSE connection closed"],
  ["SSE 连接中断，浏览器将尝试重连", "SSE connection interrupted; the client will try to reconnect"],
  ["WebSocket 地址必须以 ws:// 或 wss:// 开头", "WebSocket URL must start with ws:// or wss://"],
  ["SSE 地址必须以 http:// 或 https:// 开头", "SSE URL must start with http:// or https://"],
  ["尚未选择数据库", "No database selected"],
  ["选择 SQLite 数据库", "Choose SQLite Database"],
  ["SQLite 数据库", "SQLite Database"],
  ["所有文件", "All Files"],
  ["新建 SQLite 数据库", "Create SQLite Database"],
  ["SQLite 数据库创建成功", "SQLite database created"],
  ["该 SQL 会删除或清空数据，确定继续吗？", "This SQL will delete or clear data. Continue?"],
  ["选择 SSH 私钥", "Choose SSH Private Key"],
  ["连接配置已安全保存在本机", "Connection saved locally"],
  ["主机指纹已写入智屿独立的 known_hosts", "Host key added to Zhiyu’s private known_hosts"],
  ["请输入本次会话使用的 SSH 密码", "Enter the SSH password for this session"],
  ["SSH 连接成功", "SSH connection successful"],
  ["SSH 连接失败", "SSH connection failed"],
  ["请先选择或创建一个 SSH 连接。", "Select or create an SSH connection first."],
  ["请输入有效的 Unix 时间戳", "Enter a valid Unix timestamp"],
  ["时间戳超出可表示范围", "Timestamp is outside the supported range"],
  ["请输入有效的日期时间", "Enter a valid date and time"],
  ["UNIX 秒", "UNIX Seconds"],
  ["UNIX 毫秒", "UNIX Milliseconds"],
  ["Unix 秒", "Unix Seconds"],
  ["Unix 毫秒", "Unix Milliseconds"],
  ["有效", "Valid"],
  ["已过期", "Expired"],
  ["尚未生效", "Not active yet"],
  ["无过期时间", "No expiration"],
  ["JSON 字符串", "JSON String"],
  ["HTML 实体", "HTML Entities"],
  ["选择本地数据文件", "Choose Local Data File"],
  ["DuckDB 可查询文件", "DuckDB-compatible files"],
  ["常用前端开发端口", "Common frontend development port"],
  ["常用应用开发端口", "Common application development port"],
  ["常用 HTTP 开发端口", "Common HTTP development port"],
  ["个启用接口", "enabled endpoints"],
  ["左侧", "Left"],
  ["右侧", "Right"],
  ["用途", "Use"],
  ["摘要", "Summary"],
  ["行", "rows"],
  ["列", "columns"],
  ["↵ 复制", "↵ Copy"],
  ["⌘↵ 复制并关闭", "⌘↵ Copy & Close"],
  ["⌘P 置顶", "⌘P Pin"],
  ["⌦ 删除", "⌦ Delete"],
  ["扩展更多服务", "More Services"],
  ["小", "Small"],
  ["标准", "Default"],
  ["大", "Large"],
  ["特大", "Extra Large"],
  ["智屿经典", "Zhiyu Classic"],
  ["品牌墨绿与暖橙", "Signature ink green and warm orange"],
  ["深海终端", "Deep Sea Terminal"],
  ["冷静、专注、高对比", "Calm, focused, and high contrast"],
  ["松林", "Pine Forest"],
  ["自然、柔和、耐久看", "Natural, soft, and easy on the eyes"],
  ["暖沙", "Warm Sand"],
  ["明亮、温和、低刺激", "Bright, gentle, and low strain"],
  ["暮光", "Twilight"],
  ["低饱和紫灰氛围", "Muted violet-gray atmosphere"],
  ["极光青", "Aurora Cyan"],
  ["深蓝与清透薄荷", "Deep blue and clear mint"],
  ["石墨红", "Graphite Red"],
  ["冷灰与克制红色", "Cool gray and restrained red"],
  ["薄荷珊瑚", "Mint Coral"],
  ["柔和青绿与暖珊瑚", "Soft teal and warm coral"],
  ["落日琥珀", "Sunset Amber"],
  ["海军蓝与金橙", "Navy blue and golden orange"],
  ["霓虹波普", "Neon Pop"],
  ["高能撞色与深色底", "Vivid accents on a dark base"],
  ["北境冰川", "Nord Glacier"],
  ["克制蓝灰与冰川青", "Restrained blue-gray and glacier cyan"],
  ["樱雾", "Sakura Mist"],
  ["柔和梅紫与樱花粉", "Soft plum and sakura pink"],
  ["深焙咖啡", "Dark Roast"],
  ["温暖棕褐与奶油色", "Warm brown and cream"],
  ["日光终端", "Solarized Terminal"],
  ["经典低对比开发配色", "Classic low-contrast developer palette"],
  ["薰衣草", "Lavender"],
  ["清透紫蓝与柔雾白", "Clear violet-blue and mist white"],
  ["跟随主题", "Follow Theme"],
  ["自动匹配当前配色", "Match the active palette automatically"],
  ["无纹理", "No Pattern"],
  ["纯净、简洁", "Clean and minimal"],
  ["方格", "Grid"],
  ["开发者网格", "Developer grid"],
  ["点阵", "Dots"],
  ["轻盈、现代", "Light and modern"],
  ["斜线", "Diagonal"],
  ["细腻、利落", "Subtle and crisp"],
  ["交叉线", "Crosshatch"],
  ["细密工程草图", "Fine engineering sketch"],
  ["电路", "Circuit"],
  ["节点与线路", "Nodes and traces"],
  ["涟漪", "Rings"],
  ["柔和同心圆", "Soft concentric circles"],
  ["横线纸", "Ruled Paper"],
  ["轻量书写节奏", "A light writing rhythm"],
  ["棋盘", "Checker"],
  ["低对比方块", "Low-contrast squares"],
  ["原图", "Original"],
  ["保留图片细节与色彩", "Preserve image detail and color"],
  ["磨砂", "Frosted"],
  ["轻柔玻璃质感，推荐", "Soft glass effect, recommended"],
  ["高斯模糊", "Gaussian Blur"],
  ["弱化细节，专注内容", "Reduce detail and focus on content"],
  ["雾气", "Mist"],
  ["低饱和柔雾氛围", "Muted mist atmosphere"],
  ["顶部", "Top"],
  ["居中", "Center"],
  ["底部", "Bottom"],
  ["暂无日志", "No logs"],
  ["等待开始", "Waiting to start"],
  ["安装失败", "Installation failed"],
  ["正在取消", "Cancelling"],
  ["安装完成", "Installation complete"],
  ["安装、配置和初始化均已完成", "Installation, configuration, and initialization are complete"],
  ["概览", "Overview"],
  ["连接与控制台", "Connection & Console"],
  ["备份恢复", "Backup & Restore"],
  ["配置文件", "Configuration"],
  ["运行日志", "Logs"],
  ["版本管理", "Versions"],
  ["使用文档", "Documentation"],
  ["数据浏览", "Data Browser"],
  ["命令台", "Console"],
  ["SQL 命令台", "SQL Console"],
  ["JSON 命令台", "JSON Console"],
  ["邮件收件箱", "Inbox"],
  ["消息调试", "Message Debugging"],
  ["主题与消息", "Topics & Messages"],
  ["索引与搜索", "Indexes & Search"],
  ["连接与调试", "Connection & Debugging"],
  ["站点", "Sites"],
  ["文件管理", "File Manager"],
  ["无需 Java 的 Nacos 兼容配置中心与服务注册中心", "A Nacos-compatible configuration and service registry with no Java dependency"],
  ["单节点 · 无需 Java", "Single node · No Java"],
  ["1.x OpenAPI 与客户端", "1.x OpenAPI and clients"],
  ["默认 admin / admin", "Default admin / admin"],
  ["OpenAPI 鉴权默认关闭，控制台使用开发账号 admin / admin；不要暴露到公网。", "OpenAPI authentication is off by default. The console uses admin / admin for local development; do not expose it publicly."],
  ["适合服务注册、健康检查、KV 配置和 DNS 服务发现", "For service registration, health checks, KV configuration, and DNS discovery"],
  ["单节点 Server · 仅本机", "Single-node server · Local only"],
  ["API 与 /ui/ 管理界面", "API and /ui/ console"],
  ["DNS 服务发现", "DNS service discovery"],
  ["智屿只运行本机单节点 Server Agent，不启用 ACL，也不模拟生产集群。", "Zhiyu runs a local single-node server agent without ACLs and does not simulate a production cluster."],
  ["适合配置读取、服务协调、分布式锁和客户端兼容调试", "For configuration, coordination, distributed locks, and client compatibility testing"],
  ["单节点 · 仅本机", "Single node · Local only"],
  ["应用和 etcdctl 使用", "For applications and etcdctl"],
  ["单节点内部通信", "Single-node peer communication"],
  ["智屿只启用本机单节点模式，不开放远程监听，也不模拟生产集群。", "Zhiyu uses local single-node mode, exposes no remote listener, and does not simulate a production cluster."],
  ["推荐尝鲜 · Beta", "Recommended preview · Beta"],
  ["RustFS 当前仍处于 Beta 阶段，适合本地开发验证，不建议保存唯一副本或生产数据。", "RustFS is currently beta and suitable for local development. Do not use it as the only copy of production data."],
  ["存量兼容 · 官方仓库已归档", "Legacy compatibility · Official repository archived"],
  ["MinIO 社区仓库已归档，本模块用于兼容已有开发项目；新项目建议选择 RustFS。", "The MinIO community repository is archived. This module supports existing projects; use RustFS for new projects."],
  ["诊断完成，没有需要自动修复的项目", "Diagnostics complete; no automatic repairs were needed"],
  ["诊断报告已复制，用户目录已脱敏", "Diagnostic report copied with the user directory redacted"],
  ["选择智屿背景图", "Choose Zhiyu Background Image"],
  ["图片", "Images"],
  ["选择智屿安装目录", "Choose Zhiyu Installation Directory"],
  ["选择智屿环境目录", "Choose Zhiyu Environment Directory"],
  ["更换安装目录后，智屿会切换到一个新的环境。旧目录中的服务和数据不会自动迁移，确定保存吗？", "Changing the installation directory creates a new environment. Services and data in the old directory will not be migrated automatically. Save this change?"],
  ["已自动保存，并切换到新的安装目录", "Saved automatically and switched to the new installation directory"],
  ["将删除所有服务的下载包和安装临时文件。已安装程序、配置、数据和备份不会被删除，确定继续吗？", "Delete all downloaded packages and temporary installation files? Installed programs, configuration, data, and backups will be kept."],
  ["停止", "Stop"],
  ["已通过全局概览停止", "Stopped from Global Overview"],
  ["修复状态", "Repair State"],
  ["已清理异常运行记录", "Invalid runtime record removed"],
  ["安装", "Install"],
  ["启动", "Start"],
  ["重启", "Restart"],
  ["强制停止", "Force Stop"],
  ["最后 50 行日志", "last 50 log lines"],
  ["正在等待一条匹配的 NATS 消息，最多等待 8 秒", "Waiting up to 8 seconds for a matching NATS message"],

  // Port inspector
  ["端口检查器", "Port Inspector"],
  ["查看本机正在监听的 TCP 端口，不修改任何进程", "Inspect local TCP listeners without changing any processes"],
  ["检查中", "Checking"],
  ["重新检查", "Check again"],
  ["正在监听的地址", "Listening addresses"],
  ["占用端口的进程", "Processes using ports"],
  ["智屿管理的监听地址", "Listeners managed by Zhiyu"],
  ["监听全部网络接口", "Listening on all interfaces"],
  ["监听端口", "Listening Ports"],
  ["筛选", "Filter"],
  ["端口、进程或服务", "Port, process, or service"],
  ["正在读取本机端口…", "Reading local ports…"],
  ["没有匹配的监听端口", "No matching listening ports"],
  ["当前没有 TCP 监听端口", "No TCP ports are currently listening"],
  ["监听地址", "Listen Address"],
  ["进程", "Process"],
  ["归属", "Owner"],
  ["常见用途", "Common Use"],
  ["全部网卡", "All interfaces"],
  ["未知进程", "Unknown process"],
  ["外部进程", "External process"],
  ["这里只显示 TCP 监听端口。监听", "Only TCP listeners are shown. Services listening on"],
  ["或", "or"],
  ["的服务仅供本机访问；“全部网卡”表示局域网设备也可能连接。", "are local-only; “All interfaces” means devices on the LAN may also connect."],

  // Mock API
  ["本地 Mock API", "Local Mock API"],
  ["无需编写后端代码，在本机快速模拟 HTTP 接口", "Mock HTTP endpoints locally without writing backend code"],
  ["停止服务", "Stop server"],
  ["启动服务", "Start server"],
  ["服务运行中", "Server running"],
  ["服务未启动", "Server stopped"],
  ["正在监听本机请求", "Listening for local requests"],
  ["启动后即可访问接口", "Start the server to access endpoints"],
  ["启用接口", "Enable endpoint"],
  ["接口规则", "Routes"],
  ["＋ 新建", "+ New"],
  ["已启用", "Enabled"],
  ["已停用", "Disabled"],
  ["还没有接口规则", "No routes yet"],
  ["编辑接口", "Edit Route"],
  ["复制地址", "Copy URL"],
  ["请求方法", "Method"],
  ["接口路径", "Route Path"],
  ["状态码", "Status Code"],
  ["延迟（ms）", "Delay (ms)"],
  ["支持 JSON、文本和 HTML", "Supports JSON, text, and HTML"],
  ["选择一个接口，或新建接口规则", "Select a route or create a new one"],
  ["最近请求", "Recent Requests"],
  ["启动服务并访问接口后，请求会显示在这里", "Requests appear here after the server is started and accessed"],
  ["已匹配", "Matched"],
  ["未匹配", "Not matched"],

  // HTTP client
  ["HTTP 请求调试器", "HTTP Client"],
  ["发送本地或远程 HTTP 请求，查看状态、响应头和响应内容", "Send local or remote HTTP requests and inspect the response"],
  ["复制 cURL", "Copy cURL"],
  ["发送中", "Sending"],
  ["发送请求", "Send Request"],
  ["请求头", "Headers"],
  ["请求体", "Body"],
  ["超时", "Timeout"],
  ["跟随重定向", "Follow redirects"],
  ["值", "Value"],
  ["Header 名称", "Header name"],
  ["Header 值", "Header value"],
  ["＋ 添加请求头", "+ Add header"],
  ["GET 和 HEAD 请求不会发送请求体；单次请求体最大 2 MiB。", "GET and HEAD requests do not send a body. Maximum request body: 2 MiB."],
  ["响应结果", "Response"],
  ["响应体", "Response Body"],
  ["响应头", "Response Headers"],
  ["复制响应", "Copy Response"],
  ["正在等待服务器响应…", "Waiting for the server response…"],
  ["填写请求地址后点击“发送请求”", "Enter a URL, then select “Send Request”"],
  ["可直接启动“本地 Mock API”，测试默认示例地址", "Start “Local Mock API” to test the example URL"],
  ["响应超过 2 MiB，仅展示前 2 MiB，避免占用过多内存。", "Responses over 2 MiB are truncated to keep memory usage low."],

  // Realtime client
  ["已连接", "Connected"],
  ["未连接", "Disconnected"],
  ["用户断开", "Disconnected by user"],
  ["已主动断开连接", "Connection closed"],
  ["WebSocket / SSE 调试器", "WebSocket / SSE Client"],
  ["测试双向 WebSocket 消息和服务器推送事件流", "Test bidirectional WebSocket messages and server-sent events"],
  ["双向消息", "WebSocket"],
  ["服务器推送", "Server-Sent Events"],
  ["事件名（可选）", "Event name (optional)"],
  ["发送消息", "Message"],
  ["文本或 JSON", "Text or JSON"],
  ["发送", "Send"],
  ["通信日志", "Communication Log"],
  ["连接服务后，消息与连接状态会显示在这里", "Messages and connection events appear here after connecting"],
  ["接收", "Received"],
  ["错误", "Error"],
  ["WebSocket 与 SSE 使用系统 WebView 原生网络能力，不经过远程代理。浏览器接口不支持为连接添加任意请求头；需要鉴权时可使用 URL 查询参数或服务端 Cookie。", "WebSocket and SSE use the system WebView directly, without a remote proxy. Custom connection headers are unavailable; use URL parameters or server cookies for authentication."],

  // Time
  ["本地时区", "Local Time Zone"],
  ["亚洲 / 上海", "Asia / Shanghai"],
  ["亚洲 / 东京", "Asia / Tokyo"],
  ["美国 / 纽约", "America / New York"],
  ["欧洲 / 伦敦", "Europe / London"],
  ["时间与时间戳", "Time & Timestamp"],
  ["Unix 时间戳、日期时间和常用时区快速互转", "Convert Unix timestamps, date/time values, and common time zones"],
  ["使用当前时间", "Use Current Time"],
  ["时间戳转日期", "Timestamp to Date"],
  ["Unix 时间戳", "Unix Timestamp"],
  ["输入单位", "Input Unit"],
  ["自动判断", "Auto detect"],
  ["毫秒", "ms"],
  ["转换", "Convert"],
  ["自动判断：少于 12 位按秒处理，其余按毫秒处理。", "Auto detect: fewer than 12 digits uses seconds; otherwise milliseconds."],
  ["日期转时间戳", "Date to Timestamp"],
  ["本地日期时间", "Local Date & Time"],
  ["日期输入按当前系统时区解释。", "Date input is interpreted in the current system time zone."],
  ["转换结果", "Conversion Result"],
  ["显示时区", "Display Time Zone"],
  ["输入时间戳或日期后进行转换", "Enter a timestamp or date to convert"],
  ["同一时刻的时区对照", "The Same Instant Across Time Zones"],

  // Regex
  ["选择常用表达式", "Choose a common pattern"],
  ["邮箱地址", "Email address"],
  ["IPv4 地址", "IPv4 address"],
  ["日期 YYYY-MM-DD", "Date YYYY-MM-DD"],
  ["中国大陆手机号", "Mainland China mobile number"],
  ["正则表达式调试器", "Regex Tester"],
  ["实时匹配、捕获组查看与替换结果预览", "Live matching, capture groups, and replacement preview"],
  ["输入正则表达式", "Enter a regular expression"],
  ["全局", "Global"],
  ["忽略大小写", "Ignore case"],
  ["多行", "Multiline"],
  ["点匹配换行", "Dot matches newline"],
  ["个匹配", " matches"],
  ["测试文本", "Test Text"],
  ["字符", "characters"],
  ["匹配预览", "Match Preview"],
  ["没有匹配内容", "No matches"],
  ["匹配结果与捕获组", "Matches & Capture Groups"],
  ["输入表达式和测试文本后，匹配结果会显示在这里", "Enter a pattern and test text to see matches"],
  ["空匹配", "Empty match"],
  ["位置", "Position"],
  ["空", "Empty"],
  ["替换预览", "Replacement Preview"],
  ["复制结果", "Copy Result"],
  ["替换内容", "Replacement"],

  // Cron
  ["Cron 表达式工具", "Cron Expression Tool"],
  ["校验 Cron 表达式并计算未来运行时间", "Validate Cron expressions and calculate upcoming runs"],
  ["复制表达式", "Copy Expression"],
  ["解析并计算", "Parse & Calculate"],
  ["日期", "Day"],
  ["月份", "Month"],
  ["星期", "Weekday"],
  ["常用表达式", "Common Expressions"],
  ["表达式说明", "Expression Guide"],
  ["当前按照本机时区计算。日期和星期同时受限时，遵循常见 Linux Cron 语义：任一条件满足即可。", "Calculated in the local time zone. When both day and weekday are restricted, common Linux Cron OR semantics apply."],
  ["任意值", "Any value"],
  ["每 5 个单位", "Every 5 units"],
  ["范围", "Range"],
  ["多个值", "Multiple values"],
  ["未来 10 次运行时间", "Next 10 Runs"],
  ["输入表达式后点击“解析并计算”", "Enter an expression, then select “Parse & Calculate”"],
  ["当前支持 Linux 常用 5 段语法以及列表、范围、步长和英文月份/星期缩写；不支持 Quartz 的秒、年份、", "Supports standard 5-field Linux syntax, lists, ranges, steps, and English month/weekday names. Quartz seconds, years, and "],
  ["。", "."],
  ["每 5 分钟", "Every 5 minutes"],
  ["每小时整点", "Hourly"],
  ["工作日 09:00", "Weekdays at 09:00"],
  ["每天 00:00", "Daily at 00:00"],
  ["每周日 03:00", "Sundays at 03:00"],
  ["每月 1 日", "First day of each month"],

  // QR Code
  ["QR Code 工具", "QR Code Tool"],
  ["本地生成二维码，并在系统支持时识别二维码图片", "Generate QR codes locally and scan images when supported"],
  ["选择二维码图片", "Choose QR Code Image"],
  ["当前系统不支持图片识别", "Image scanning is unavailable on this system"],
  ["识别中", "Scanning"],
  ["识别图片", "Scan Image"],
  ["二维码内容", "QR Code Content"],
  ["文本、URL、邮箱或其他内容", "Text, URL, email, or other content"],
  ["纠错等级", "Error Correction"],
  ["导出尺寸", "Export Size"],
  ["生成中", "Generating"],
  ["生成二维码", "Generate QR Code"],
  ["内容只在本机编码，不会上传。纠错等级越高，二维码越密集，但污损后的可恢复能力更强。", "Content is encoded locally and never uploaded. Higher correction levels create denser codes that better tolerate damage."],
  ["二维码预览", "QR Code Preview"],
  ["模块", "modules"],
  ["输入内容后生成二维码", "Enter content to generate a QR code"],
  ["复制 SVG", "Copy SVG"],
  ["导出 SVG", "Export SVG"],
  ["导出 PNG", "Export PNG"],

  // DuckDB and SQLite
  ["DuckDB 本地文件查询器", "DuckDB Local File Query"],
  ["直接查询 CSV、JSON、Parquet 和 DuckDB 文件，不启动后台服务", "Query CSV, JSON, Parquet, and DuckDB files without a background service"],
  ["安装中", "Installing"],
  ["下载并安装", "Download & Install"],
  ["选择本地文件", "Choose Local File"],
  ["正在检查 DuckDB CLI…", "Checking DuckDB CLI…"],
  ["安装 DuckDB CLI", "Install DuckDB CLI"],
  ["正在下载并校验…", "Downloading and verifying…"],
  ["安装 DuckDB", "Install DuckDB"],
  ["官方 DuckDB CLI", "Official DuckDB CLI"],
  ["当前数据源", "Current Data Source"],
  ["查询引擎占用", "Query Engine Size"],
  ["只读 · 15 秒超时", "Read-only · 15 sec timeout"],
  ["选择一个 CSV、JSON、Parquet 或 DuckDB 文件", "Choose a CSV, JSON, Parquet, or DuckDB file"],
  ["更换文件", "Change File"],
  ["选择文件", "Choose File"],
  ["查询语句", "Query"],
  ["预览 100 行", "Preview 100 Rows"],
  ["统计行数", "Count Rows"],
  ["查看字段", "Show Columns"],
  ["查看所有表", "Show All Tables"],
  [".duckdb 文件以 safe + readonly 模式打开", ".duckdb files open in safe, read-only mode"],
  ["使用 selected_file 作为所选文件的表名", "Use selected_file as the selected file’s table name"],
  ["⌘ Enter 执行", "⌘ Enter to run"],
  ["运行查询", "Run Query"],
  ["正在本机执行查询…", "Running query locally…"],
  ["输入只读 SQL 后运行查询", "Enter a read-only SQL query to run"],
  ["请先选择一个本地文件", "Choose a local file first"],
  ["为保持界面流畅，单次最多展示 500 行；请用 WHERE 或 LIMIT 缩小结果。", "Up to 500 rows are displayed. Use WHERE or LIMIT to narrow the result."],
  ["SQLite 本地数据库", "SQLite Local Database"],
  ["打开、创建并查询本地 SQLite 文件，不启动后台服务", "Open, create, and query local SQLite files without a background service"],
  ["新建数据库", "New Database"],
  ["打开数据库", "Open Database"],
  ["打开本地 SQLite 数据库", "Open Local SQLite Database"],
  ["SQLite 引擎已经内置在智屿中，不依赖系统安装，也没有常驻进程。数据库文件只在本机读取和修改。", "SQLite is built into Zhiyu, requires no system installation, and has no resident process. Database files stay local."],
  ["打开现有数据库", "Open Existing Database"],
  ["新建空数据库", "Create Empty Database"],
  ["内嵌 SQLite", "Embedded SQLite"],
  ["用户数据表", "User Tables"],
  ["数据库磁盘占用", "Database Size"],
  ["个索引", " indexes"],
  ["数据表与视图", "Tables & Views"],
  ["暂无数据表，可在右侧执行 CREATE TABLE。", "No tables yet. Run CREATE TABLE on the right."],
  ["查询与编辑", "Query & Edit"],
  ["查看结构", "Show Schema"],
  ["完整性检查", "Integrity Check"],
  ["写入直接保存到所选文件，危险操作需要确认", "Writes are saved directly; destructive actions require confirmation"],
  ["运行 SQL", "Run SQL"],
  ["正在执行 SQLite 查询…", "Running SQLite query…"],
  ["选择数据表或输入 SQL 后运行", "Select a table or enter SQL to run"],
  ["为保持界面流畅，单次最多展示 500 行。", "Up to 500 rows are displayed to keep the interface responsive."],

  // Clipboard
  ["剪贴板历史", "Clipboard History"],
  ["本地记录、搜索并快速复用最近复制的文本", "Record, search, and reuse recently copied text locally"],
  ["继续记录", "Resume"],
  ["暂停记录", "Pause"],
  ["关闭记录", "Turn Off"],
  ["开启记录", "Turn On"],
  ["记录中", "Recording"],
  ["已暂停", "Paused"],
  ["已关闭", "Off"],
  ["本地历史记录", "Local History"],
  ["长期保留的记录", "Pinned Records"],
  ["SQLite 数据占用", "SQLite Storage"],
  ["不会写入新的记录", "No new items will be recorded"],
  ["仅监控文本内容", "Text content only"],
  ["最近复制", "Recently Copied"],
  ["搜索", "Search"],
  ["输入内容关键词", "Search clipboard content"],
  ["清空未置顶", "Clear Unpinned"],
  ["正在读取本地记录…", "Loading local history…"],
  ["没有匹配的记录", "No matching items"],
  ["还没有剪贴板记录", "No clipboard history yet"],
  ["换一个关键词试试", "Try another search term"],
  ["复制一段文本后，它会自动出现在这里", "Copy some text and it will appear here automatically"],
  ["字", "chars"],
  ["使用", "Used"],
  ["次", "times"],
  ["已置顶", "Pinned"],
  ["已复制", "Copied"],
  ["复制到剪贴板", "Copy to Clipboard"],
  ["取消置顶", "Unpin"],
  ["置顶", "Pin"],
  ["记录设置", "History Settings"],
  ["限制本地数据规模，置顶内容不会被自动清理。", "Limit local storage. Pinned items are never removed automatically."],
  ["最大记录数", "Maximum Items"],
  ["保留天数", "Retention"],
  ["启动时自动开启记录", "Start recording on launch"],
  ["设置已保存", "Settings saved"],
  ["保存设置", "Save Settings"],
  ["数据保存在当前用户目录，不会发送到网络。", "Data stays in the current user directory and is never sent over the network."],
  ["把复制过的内容，留在手边。", "Keep copied content close at hand."],
  ["智屿会在本机记录文本剪贴板，方便你搜索、置顶和再次复制。", "Zhiyu records text clipboard history locally so you can search, pin, and copy it again."],
  ["密码等敏感输入不会主动上传，所有数据仅保存在用户目录。", "Nothing is uploaded; all data remains in your user directory."],
  ["开启剪贴板记录", "Enable Clipboard History"],
  ["只保存在本机", "Local Only"],
  ["使用轻量 SQLite 存储，不依赖云端服务。", "Uses lightweight SQLite storage with no cloud dependency."],
  ["随时暂停或关闭", "Pause Anytime"],
  ["关闭记录不会删除已有历史，可随时继续。", "Turning off recording does not delete history; resume anytime."],
  ["自动控制空间", "Automatic Storage Control"],
  ["按记录数量和保留天数清理过期内容。", "Old content is removed by item count and retention period."],
  ["搜索剪贴板历史…", "Search clipboard history…"],
  ["ESC 关闭", "ESC Close"],
  ["无匹配结果", "No matches"],
  ["暂无剪贴板记录", "No clipboard history"],

  // S3
  ["阿里云 OSS", "Alibaba Cloud OSS"],
  ["腾讯云 COS", "Tencent Cloud COS"],
  ["七牛云 Kodo", "Qiniu Kodo"],
  ["根目录", "Root"],
  ["S3 兼容存储", "S3-Compatible Storage"],
  ["S3 对象存储浏览器", "S3 Object Storage Browser"],
  ["连接兼容 S3 协议的对象存储，浏览 Bucket、上传下载、生成预签名链接", "Connect to S3-compatible storage to browse buckets, transfer objects, and create presigned URLs"],
  ["连接配置", "Connection"],
  ["填写 R2 S3 API Endpoint，需包含 Account ID；EU、FedRAMP 可直接填写对应辖区 Endpoint", "Enter the R2 S3 API endpoint including the Account ID. EU and FedRAMP jurisdiction endpoints are supported."],
  ["Cloudflare R2 固定使用 auto", "Cloudflare R2 always uses auto"],
  ["（必填）", "(required)"],
  ["（可选）", "(optional)"],
  ["请输入 Bucket", "Enter a bucket"],
  ["留空则先列出 Bucket", "Leave empty to list buckets first"],
  ["地址模式", "Addressing Style"],
  ["根据 Endpoint 自动判断", "Detected automatically from endpoint"],
  ["最近连接", "Recent Connections"],
  ["未连接", "Not connected"],
  ["在左侧填入 Endpoint、Access Key 和 Secret Key 后点击连接", "Enter the endpoint, access key, and secret key on the left, then connect"],
  ["选择 Bucket", "Select a Bucket"],
  ["Bucket 名称", "Bucket Name"],
  ["创建时间", "Created"],
  ["暂无对象", "No objects"],
  ["当前目录下没有子目录或文件", "This folder has no subfolders or files"],
  ["修改时间", "Modified"],
  ["目录", "Folder"],
  ["查看", "Preview"],
  ["预签名", "Presign"],
  ["页 · 每页最多 200 项", " · up to 200 items per page"],
  ["上一页", "Previous"],
  ["下一页", "Next"],
  ["关闭预览", "Close Preview"],
  ["正在准备预览…", "Preparing preview…"],
  ["对象预览", "Object Preview"],
  ["PDF 预览", "PDF Preview"],
  ["该文件类型不支持直接预览，可通过临时链接打开或下载。", "This file type cannot be previewed directly. Open or download it using a temporary link."],
  ["打开临时链接", "Open Temporary Link"],
  ["预签名 URL 已复制到剪贴板", "Presigned URL copied to the clipboard"],

  // SSH
  ["SSH 连接管理", "SSH Manager"],
  ["安全直连远程服务器，支持密钥和密码认证", "Connect directly to remote servers using key or password authentication"],
  ["服务器", "Servers"],
  ["新建连接", "New Connection"],
  ["隐藏服务器列表", "Hide Server List"],
  ["正在读取本地连接…", "Loading local connections…"],
  ["还没有连接", "No connections yet"],
  ["点击右上角加号创建", "Use the plus button above to create one"],
  ["本地安全存储", "Local Secure Storage"],
  ["私钥内容不会进入智屿；密码仅保留在当前应用会话内。", "Private-key content is never stored in Zhiyu; passwords remain only for the current app session."],
  ["展开服务器列表", "Show Server List"],
  ["密钥", "Key"],
  ["本次会话已输入", "Entered for this session"],
  ["需要输入密码", "Password required"],
  ["指定私钥", "Custom private key"],
  ["ssh-agent / 默认密钥", "ssh-agent / default key"],
  ["编辑连接", "Edit Connection"],
  ["核对主机指纹", "Verify Host Key"],
  ["测试连接", "Test Connection"],
  ["创建一个 SSH 连接开始使用", "Create an SSH connection to get started"],
  ["支持密钥认证和仅在当前会话保存的密码认证", "Supports key authentication and session-only passwords"],
  ["＋ 新建连接", "+ New Connection"],
  ["请通过服务器控制台或管理员核对指纹", "Verify this fingerprint with the server console or administrator"],
  ["已核对，信任此主机", "Verified — Trust This Host"],
  ["交互终端", "Interactive Terminal"],
  ["未选择连接", "No connection selected"],
  ["超过所选时间未操作终端时自动断开", "Disconnect after the terminal is idle for the selected time"],
  ["闲置断开", "Idle Timeout"],
  ["SSH 闲置自动断开时间", "SSH idle disconnect time"],
  ["10 分钟", "10 minutes"],
  ["30 分钟", "30 minutes"],
  ["1 小时", "1 hour"],
  ["2 小时", "2 hours"],
  ["永不断开", "Never"],
  ["连接终端", "Connect Terminal"],
  ["支持 Tab 补全、方向键、Ctrl+C 和交互式程序", "Supports Tab completion, arrow keys, Ctrl+C, and interactive programs"],
  ["密码请直接在 SSH 提示符中输入，输入内容不会显示或落盘", "Enter passwords directly at the SSH prompt; input is neither displayed nor stored"],
  ["编辑 SSH 连接", "Edit SSH Connection"],
  ["新建 SSH 连接", "New SSH Connection"],
  ["认证方式", "Authentication"],
  ["密钥认证", "Key Authentication"],
  ["私钥或 ssh-agent", "Private key or ssh-agent"],
  ["密码认证", "Password Authentication"],
  ["仅保留当前会话", "Current session only"],
  ["连接名称", "Connection Name"],
  ["例如：开发服务器", "e.g. Development Server"],
  ["主机地址", "Host"],
  ["192.168.1.10 或 server.example.com", "192.168.1.10 or server.example.com"],
  ["私钥文件", "Private Key File"],
  ["可留空，使用 ssh-agent 和默认密钥", "Optional; uses ssh-agent and default keys"],
  ["清除私钥路径", "Clear private key path"],
  ["SSH 密码", "SSH Password"],
  ["不会写入 profiles.json", "Never written to profiles.json"],
  ["输入本次会话使用的密码", "Enter the password for this session"],
  ["关闭智屿后密码自动清除，下次连接时需要重新输入。", "The password is cleared when Zhiyu closes and must be entered again next time."],
  ["删除连接", "Delete Connection"],
  ["保存并使用", "Save & Use"],

  // Data and JWT
  ["数据格式工具箱", "Data Format Toolbox"],
  ["格式转换、CSV、差异比较、路径查询与常用编码，全部在本机完成", "Format conversion, CSV, diff, path queries, and encoding — all processed locally"],
  ["格式化与转换", "Format & Convert"],
  ["表格数据互转", "Tabular Conversion"],
  ["JSON 差异", "JSON Diff"],
  ["逐字段比较", "Field-by-field comparison"],
  ["按路径提取", "Extract by path"],
  ["编码与转义", "Encode & Escape"],
  ["自动识别", "Auto detect"],
  ["来源", "Source"],
  ["目标", "Target"],
  ["样式", "Style"],
  ["缩进美化", "Pretty"],
  ["压缩", "Minify"],
  ["粘贴 JSON、YAML 或 TOML", "Paste JSON, YAML, or TOML"],
  ["识别为", "Detected as"],
  ["点击「执行」查看结果", "Select “Run” to view the result"],
  ["转换方向", "Direction"],
  ["分隔符", "Delimiter"],
  ["逗号 ,", "Comma ,"],
  ["制表符 Tab", "Tab"],
  ["分号 ;", "Semicolon ;"],
  ["竖线 |", "Pipe |"],
  ["转换中", "Converting"],
  ["开始转换", "Convert"],
  ["载入示例", "Load Example"],
  ["第一行需要包含表头", "The first row must contain headers"],
  ["输入 JSON 对象数组", "Enter an array of JSON objects"],
  ["设置转换方向后点击「开始转换」", "Choose a direction, then select “Convert”"],
  ["CSV 第一行作为字段名；JSON 必须是对象数组。嵌套对象和数组会保留为紧凑 JSON", "The first CSV row is used as field names; JSON must be an array of objects. Nested objects and arrays remain compact JSON"],
  ["字符串。单次最多处理 5 MiB、10000 行。", " strings. A single operation is limited to 5 MiB and 10,000 rows."],
  ["开始比较", "Compare"],
  ["比较中", "Comparing"],
  ["两份 JSON 完全一致", "The two JSON values are identical"],
  ["新增", "Added"],
  ["缺失", "Missing"],
  ["变更", "Changed"],
  ["表达式", "Expression"],
  ["命中", "Matched"],
  ["输入表达式后点击「查询」", "Enter an expression, then select “Query”"],
  ["没有匹配的节点", "No matching nodes"],
  ["遵循 RFC 9535。常用写法：", "Follows RFC 9535. Common forms: "],
  ["取字段、", " selects a field, "],
  ["取全部元素、", " selects all elements, "],
  ["递归查找、", " searches recursively, and "],
  ["条件过滤。", " filters by condition."],
  ["处理类型", "Operation"],
  ["编码", "Encode"],
  ["解码", "Decode"],
  ["结果放回输入", "Use Result as Input"],
  ["复制结果", "Copy Result"],
  ["选择处理类型，然后对内容进行编码或解码", "Choose an operation, then encode or decode the content"],
  ["Base64 使用 UTF-8，可正确处理中文；URL 模式使用", "Base64 uses UTF-8 and supports Unicode; URL mode uses "],
  ["；HTML 模式处理", "; HTML mode handles "],
  ["、引号等常用实体。", ", quotes, and other common entities."],
  ["JWT 调试器", "JWT Debugger"],
  ["解码、验签与生成测试 Token，全部在本机完成，密钥和 Token 不会发往任何服务", "Decode, verify, and generate test tokens locally. Keys and tokens are never sent anywhere."],
  ["解码与验签", "Decode & Verify"],
  ["查看内容 · 校验签名", "Inspect claims · verify signature"],
  ["生成测试 Token", "Generate Test Token"],
  ["密钥集合查看", "Inspect Key Set"],
  ["Authorization 请求头", "Authorization Header"],
  ["复制 Authorization", "Copy Authorization"],
  ["粘贴 JWT，可直接带 Bearer 前缀或整行 Authorization 请求头", "Paste a JWT, with an optional Bearer prefix or full Authorization header"],
  ["解析中", "Decoding"],
  ["解码", "Decode"],
  ["验签密钥", "Verification Key"],
  ["HMAC 密钥，仅用于本机校验", "HMAC key, used only for local verification"],
  ["密钥编码", "Key Encoding"],
  ["文本", "Text"],
  ["校验中", "Verifying"],
  ["验证签名", "Verify Signature"],
  ["✓ 签名有效", "✓ Valid signature"],
  ["✕ 签名无效", "✕ Invalid signature"],
  ["头部", "Header"],
  ["载荷", "Payload"],
  ["声明", "Claim"],
  ["含义", "Meaning"],
  ["时间", "Time"],
  ["距现在", "Relative"],
  ["说明", "Description"],
  ["算法", "Algorithm"],
  ["kid（可选）", "kid (optional)"],
  ["留空则不写入", "Leave empty to omit"],
  ["签发中", "Signing"],
  ["生成 Token", "Generate Token"],
  ["填好载荷与密钥后点击「生成 Token」", "Enter a payload and key, then select “Generate Token”"],
  ["送去解码页检查", "Open in Decoder"],
  ["生成的 Token 仅供本地联调使用。载荷里的", "Generated tokens are for local testing only."],
  ["是 Unix 秒，", "is a Unix timestamp in seconds."],
  ["不写", "Omitting"],
  ["就是永不过期的 Token，请不要用于任何真实环境。", "creates a token that never expires; do not use it in a real environment."],
  ["解析密钥", "Parse Keys"],
  ["密钥集合", "Key Set"],
  ["单个密钥", "Single Key"],
  ["含私钥", "Contains private key"],
  ["粘贴单个 JWK 或形如 {\"keys\":[...]} 的密钥集合", "Paste a single JWK or a key set shaped like {\"keys\":[...]}"],
  ["JWKS 是服务端公开的公钥集合，验签方按 Token 头部的", "JWKS is a server-published public-key set. Verifiers use the token header’s"],
  ["找到对应公钥。这里只做本地解析，不会去请求任何 JWKS 地址。", " to locate the matching key. Parsing is local; no JWKS URL is requested."],
  ["未声明", "Not declared"],
  ["签发时间", "Issued At"],
  ["生效时间", "Not Before"],
  ["过期时间", "Expiration"],
  ["Token 的生成时刻，用于判断它签发了多久", "When the token was issued"],
  ["在这个时刻之前，Token 应当被拒绝", "The token should be rejected before this time"],
  ["到达这个时刻后，Token 应当被拒绝", "The token should be rejected after this time"],
  ["签发方", "Issuer"],
  ["主体", "Subject"],
  ["接收方", "Audience"],
  ["Token 编号", "Token ID"],
  ["授权范围", "Scope"],
]);

const originalText = new WeakMap<Text, string>();
const originalAttributes = new WeakMap<Element, Map<string, string>>();
const ignoredSelector =
  "script, style, textarea, pre, code, .xterm, .terminal-output, [data-no-tool-i18n]";

function translateDynamic(value: string): string {
  return value
    .replace(/^目录 "(.+)" 及其所有内容$/, 'folder "$1" and all its contents')
    .replace(/^文件 "(.+)"$/, 'file "$1"')
    .replace(/^确定删除 (.+)？此操作不可恢复。$/, "Delete $1? This cannot be undone.")
    .replace(/^(.+) 进程已意外退出（原 PID (.+)），可一键修复状态后重新启动$/, "$1 exited unexpectedly (previous PID $2). Repair its state, then start it again.")
    .replace(/^(.+) 的 PID 身份校验失败，已清理过期 PID 文件$/, "$1 failed PID identity validation. The stale PID file was removed.")
    .replace(/^(.+) 默认端口 (\\d+) 已被 (.+)（PID (\\d+)）占用$/, "$1’s default port $2 is used by $3 (PID $4).")
    .replace(/^最近一次失败：(.+)$/, "Latest failure: $1")
    .replace(/^诊断修复完成，共处理 (\\d+) 项$/, "Diagnostic repair complete; $1 items processed")
    .replace(/^智屿诊断报告 (.+)$/, "Zhiyu Diagnostic Report $1")
    .replace(/^通过 (\\d+) · 警告 (\\d+) · 错误 (\\d+)$/, "Passed $1 · Warnings $2 · Errors $3")
    .replace(/^复制诊断报告失败：(.+)$/, "Failed to copy diagnostic report: $1")
    .replace(/^请先停止当前运行的 (\\d+) 个服务，再更换安装目录$/, "Stop the $1 running services before changing the installation directory")
    .replace(/^已清理 (\\d+) 项缓存，释放 (.+)$/, "Removed $1 cache items and reclaimed $2")
    .replace(/^确定停止当前运行的 (\\d+) 个服务吗？服务数据不会被删除。$/, "Stop the $1 running services? Service data will not be deleted.")
    .replace(/^(\\d+) 个服务已停止，(\\d+) 个服务停止失败$/, "$1 services stopped; $2 failed to stop")
    .replace(/^已停止 (\\d+) 个服务$/, "Stopped $1 services")
    .replace(/^部分服务修复失败：(.+)$/, "Some services could not be repaired: $1")
    .replace(/^已修复 (\\d+) 个异常服务状态$/, "Repaired $1 invalid service states")
    .replace(/^(.+) (安装|启动|停止|重启)成功$/, "$1 $2 succeeded")
    .replace(/^(.+) 未能在正常停止时限内退出，是否强制停止？([\\s\\S]*)$/, "$1 did not exit within the normal timeout. Force stop it?$2")
    .replace(/^(.+) 已强制停止$/, "$1 was force-stopped")
    .replace(/^文档导入任务 #(\\d+) 已进入队列$/, "Document import task #$1 was queued")
    .replace(/^消息已发布到 (.+)$/, "Message published to $1")
    .replace(/^已收到 (.+)$/, "Received $1")
    .replace(/^主题 (.+) 已创建$/, "Topic $1 created")
    .replace(/^确定删除 Kafka 主题 (.+) 吗？主题内消息会一并删除。$/, "Delete Kafka topic $1 and all of its messages?")
    .replace(/^主题 (.+) 已删除$/, "Topic $1 deleted")
    .replace(/^测试消息已发送到 (.+)$/, "Test message sent to $1")
    .replace(/^在侧栏显示 (.+)$/, "Show $1 in the sidebar")
    .replace(/^(\\d+) 个运行中，(\\d+) 个未运行$/, "$1 running, $2 stopped")
    .replace(/^(.+)，(.+)，端口 (\\d+)$/, "$1, $2, port $3")
    .replace(/^一键修复 \\((\\d+)\\)$/, "Repair All ($1)")
    .replace(/ 安装 succeeded$/, " installation succeeded")
    .replace(/ 启动 succeeded$/, " started successfully")
    .replace(/ 停止 succeeded$/, " stopped successfully")
    .replace(/ 重启 succeeded$/, " restarted successfully")
    .replace(/^(\\d+)\\s+秒$/, "$1 sec")
    .replace(/^(\\d+)\\s+分钟$/, "$1 min")
    .replace(/^(\\d+)\\s+小时$/, "$1 hr")
    .replace(/^(\\d+)\\s+天$/, "$1 days")
    .replace(/^(\\d+)\\s+条$/, "$1 items")
    .replace(/^(\\d+)\\s+个$/, "$1")
    .replace(/^(\\d+)\\s+行\\s*·\\s*(\\d+)\\s+列(.*)$/, "$1 rows · $2 columns$3")
    .replace(/^使用\\s+(\\d+)\\s+次$/, "Used $1 times")
    .replace(/^(\\d+)\\s+分钟前$/, "$1 minutes ago")
    .replace(/^(\\d+)\\s+小时前$/, "$1 hours ago")
    .replace(/^(\\d+)\\s+天前$/, "$1 days ago")
    .replace(/^DuckDB\\s+(.+)\\s+安装成功$/, "DuckDB $1 installed")
    .replace(/^编码失败：(.+)$/, "Encoding failed: $1")
    .replace(/^无法解码：请确认输入是合法的(.+)内容$/, "Could not decode. Make sure the input is valid $1 content.")
    .replace(/^(.+)已复制到剪贴板$/, "$1 copied to the clipboard")
    .replace(/^无法识别“(.+)”$/, "Could not parse “$1”")
    .replace(/^步长无效：“(.+)”$/, "Invalid step: “$1”")
    .replace(/^取值超出\\s+(.+)：“(.+)”$/, "Value outside $1: “$2”")
    .replace(/^分钟\\s+(.+)、小时\\s+(.+)；日期\\s+(.+)；月份\\s+(.+)；星期\\s+(.+)$/, "Minute $1; hour $2; day $3; month $4; weekday $5")
    .replace(/^分钟\\s+(.+)、小时\\s+(.+)$/, "Minute $1; hour $2")
    .replace(/^已过期\\s+(.+)$/, "Expired $1 ago")
    .replace(/^尚未生效，还需等待\\s+(.+)$/, "Not active yet; starts in $1")
    .replace(/^有效，(.+)后过期$/, "Valid; expires in $1")
    .replace(/^该 Token 没有设置过期时间$/, "This token has no expiration time")
    .replace(/^第\\s*(\\d+)\\s*$/, "Page $1")
    .replace(/^预览\\s+(.+)$/, "Preview $1")
    .replace(/^上传到\\s+(.+)$/, "Upload to $1")
    .replace(/^确定删除\\s+(.+?)[？?]$/, "Delete $1?")
    .replace(/^正在连接\\s+(.+)$/, "Connecting to $1")
    .replace(/^连接已关闭 · code\\s+(.+)$/, "Connection closed · code $1")
    .replace(/^已闲置\\s+(\\d+)\\s+分钟，智屿自动断开 SSH 连接$/, "Idle for $1 minutes. Zhiyu disconnected the SSH session.")
    .replace(/^确定删除连接“(.+)”吗？私钥文件不会被删除。$/, "Delete connection “$1”? The private-key file will not be deleted.")
    .replace(/^点击“连接终端”进入\\s+(.+)$/, "Select “Connect Terminal” to open $1")
    .replace(/^终端错误：(.+)$/, "Terminal error: $1")
    .replace(/^连接失败：(.+)$/, "Connection failed: $1")
    .replace(/^正在连接\\s+(.+?)…$/, "Connecting to $1…");
}

function translated(value: string): string {
  const leading = value.match(/^\\s*/)?.[0] ?? "";
  const trailing = value.match(/\\s*$/)?.[0] ?? "";
  const core = value.slice(leading.length, value.length - trailing.length);
  const exact = english.get(core);
  if (exact) return `${leading}${exact}${trailing}`;

  const dynamic = translateDynamic(core);
  if (dynamic !== core) return `${leading}${dynamic}${trailing}`;

  // Vue frequently renders an interpolated value and its label into one text
  // node (for example "200 · 已启用"). Apply the longest known phrases first
  // so these composed labels are localized without touching form values.
  let composed = core;
  const phrases = [...english.entries()]
    .filter(([source]) => source.length >= 2 && composed.includes(source))
    .sort(([left], [right]) => right.length - left.length);
  for (const [source, target] of phrases) {
    composed = composed.replaceAll(source, target);
  }
  return `${leading}${composed}${trailing}`;
}

export function toolUiText(chinese: string, englishText?: string): string {
  if (document.documentElement.lang !== "en-US") return chinese;
  return englishText ?? translated(chinese);
}

function localize(root: Element, useEnglish: boolean) {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let node: Text | null;
  while ((node = walker.nextNode() as Text | null)) {
    if (node.parentElement?.closest(ignoredSelector)) continue;
    const current = node.data;
    const knownOriginal = originalText.get(node);
    if (!knownOriginal || (current !== translated(knownOriginal) && current !== knownOriginal)) {
      originalText.set(node, current);
    }
    const source = originalText.get(node) ?? current;
    const next = useEnglish ? translated(source) : source;
    if (current !== next) node.data = next;
  }

  for (const element of [root, ...root.querySelectorAll("*")]) {
    if (element.closest(ignoredSelector)) continue;
    let originals = originalAttributes.get(element);
    if (!originals) {
      originals = new Map();
      originalAttributes.set(element, originals);
    }
    for (const name of ["placeholder", "title", "aria-label"]) {
      const current = element.getAttribute(name);
      if (current === null) continue;
      const stored = originals.get(name);
      if (!stored || (current !== translated(stored) && current !== stored)) {
        originals.set(name, current);
      }
      const source = originals.get(name) ?? current;
      const next = useEnglish ? translated(source) : source;
      if (current !== next) element.setAttribute(name, next);
    }
  }
}

type ToolI18nElement = HTMLElement & {
  __toolI18nObserver?: MutationObserver;
  __toolI18nListener?: EventListener;
  __toolI18nFrame?: number;
};

function schedule(element: ToolI18nElement) {
  if (element.__toolI18nFrame) return;
  element.__toolI18nFrame = requestAnimationFrame(() => {
    element.__toolI18nFrame = undefined;
    localize(element, document.documentElement.lang === "en-US");
  });
}

export const toolI18nDirective: Directive<ToolI18nElement> = {
  mounted(element) {
    localize(element, document.documentElement.lang === "en-US");
    element.__toolI18nObserver = new MutationObserver(() => schedule(element));
    element.__toolI18nObserver.observe(element, {
      subtree: true,
      childList: true,
      characterData: true,
      attributes: true,
      attributeFilter: ["placeholder", "title", "aria-label"],
    });
    element.__toolI18nListener = () => schedule(element);
    window.addEventListener("zhiyu:locale-changed", element.__toolI18nListener);
  },
  updated(element) {
    schedule(element);
  },
  unmounted(element) {
    element.__toolI18nObserver?.disconnect();
    if (element.__toolI18nListener) {
      window.removeEventListener("zhiyu:locale-changed", element.__toolI18nListener);
    }
    if (element.__toolI18nFrame) cancelAnimationFrame(element.__toolI18nFrame);
  },
};
