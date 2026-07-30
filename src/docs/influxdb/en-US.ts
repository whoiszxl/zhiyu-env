import type { DocChapter } from "../docTypes";

export function buildInfluxdbDocs(port: number): DocChapter[] {
  const base = `http://127.0.0.1:${port}`;
  return [
    {
      id: "intro",
      navLabel: "About InfluxDB",
      navHint: "Time series · Metrics",
      title: "A local time-series development environment",
      intro: "InfluxDB 3 Core stores timestamped metrics, events, and sensor data. Zhiyu uses local file storage and binds the server to localhost only.",
      blocks: [
        { kind: "table", head: ["Item", "Value", "Description"], rows: [
          ["HTTP API", base, "Query, write, and database management"],
          ["Authentication", "Disabled locally", "Bound to 127.0.0.1 only"],
          ["Data directory", "~/.devbox/instances/influxdb/default/data/<version>", "Data is isolated by version"],
        ]},
        { kind: "callout", tone: "warn", title: "Local development only", value: "Do not expose this unauthenticated instance through a proxy, LAN, or public network." },
      ],
    },
    {
      id: "quickstart",
      navLabel: "Quick start",
      navHint: "Create · Write · Query",
      title: "Verify a time-series workflow",
      intro: "Use the Time-series Data tab or call the HTTP API directly.",
      blocks: [
        { kind: "code", lang: "bash", caption: "Create a database", code: `curl -X POST "${base}/api/v3/configure/database" \\
  -H "Content-Type: application/json" \\
  -d '{"db":"metrics"}'` },
        { kind: "code", lang: "bash", caption: "Write line protocol", code: `curl -X POST "${base}/api/v3/write_lp?db=metrics" \\
  -H "Content-Type: text/plain" \\
  --data-binary 'cpu,host=local usage=12.5'` },
        { kind: "code", lang: "bash", caption: "Run SQL", code: `curl -X POST "${base}/api/v3/query_sql?db=metrics&format=json" \\
  -H "Content-Type: application/json" \\
  -d '{"q":"SELECT * FROM cpu ORDER BY time DESC LIMIT 10"}'` },
      ],
    },
  ];
}
