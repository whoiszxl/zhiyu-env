import type { DocChapter } from "../docTypes";

export function buildInfluxdbDocs(port: number): DocChapter[] {
  const base = `http://127.0.0.1:${port}`;
  return [
    {
      id: "intro",
      navLabel: "认识 InfluxDB",
      navHint: "时序数据 · 指标",
      title: "本地时序数据开发环境",
      intro: "InfluxDB 3 Core 适合保存带时间戳的指标、事件和传感器数据。智屿使用本地文件存储，并且只监听本机地址。",
      blocks: [
        { kind: "table", head: ["项目", "值", "说明"], rows: [
          ["HTTP API", base, "查询、写入与数据库管理"],
          ["认证", "本地免认证", "仅绑定 127.0.0.1"],
          ["数据目录", "~/.devbox/instances/influxdb/default/data/<版本>", "各版本数据彼此隔离"],
        ]},
        { kind: "callout", tone: "warn", title: "仅用于本地开发", value: "不要把这个免认证实例反向代理或暴露到局域网与公网。" },
      ],
    },
    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "建库 · 写入 · 查询",
      title: "三步验证时序数据",
      intro: "可以直接使用「时序数据」标签页，也可以调用 HTTP API。",
      blocks: [
        { kind: "code", lang: "bash", caption: "创建数据库", code: `curl -X POST "${base}/api/v3/configure/database" \\
  -H "Content-Type: application/json" \\
  -d '{"db":"metrics"}'` },
        { kind: "code", lang: "bash", caption: "写入 Line Protocol", code: `curl -X POST "${base}/api/v3/write_lp?db=metrics" \\
  -H "Content-Type: text/plain" \\
  --data-binary 'cpu,host=local usage=12.5'` },
        { kind: "code", lang: "bash", caption: "执行 SQL", code: `curl -X POST "${base}/api/v3/query_sql?db=metrics&format=json" \\
  -H "Content-Type: application/json" \\
  -d '{"q":"SELECT * FROM cpu ORDER BY time DESC LIMIT 10"}'` },
      ],
    },
  ];
}
