import type { DocChapter } from "./docTypes";

export function buildEtcdDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "连接 etcd",
      navHint: "KV · 服务协调",
      title: "本地单节点 etcd",
      intro:
        "智屿以单节点模式运行 etcd，适合验证配置读取、服务协调、分布式锁和 Kubernetes 相关客户端。",
      blocks: [
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["Client Endpoint", `http://127.0.0.1:${port}`, "应用与 etcdctl 连接地址"],
            ["Peer Endpoint", "http://127.0.0.1:2380", "单节点内部通信端口"],
            ["TLS / Auth", "关闭", "仅绑定本机开发环境"],
          ],
        },
        {
          kind: "code",
          caption: "使用内置 etcdctl",
          lang: "bash",
          code:
            "ETCDCTL_API=3 ~/.devbox/installations/etcd/3.6/bin/etcdctl --endpoints=http://127.0.0.1:2379 put hello zhiyu\nETCDCTL_API=3 ~/.devbox/installations/etcd/3.6/bin/etcdctl --endpoints=http://127.0.0.1:2379 get hello",
        },
      ],
    },
  ];
}
