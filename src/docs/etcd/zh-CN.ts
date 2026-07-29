import type { DocChapter } from "../docTypes";

export function buildEtcdDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 etcd",
      navHint: "分布式 KV · Raft",
      title: "etcd 是什么",
      intro:
        "etcd 是一个分布式的、强一致的键值存储系统。它通过 Raft 一致性算法保证多个节点之间的数据一致，是 Kubernetes 的核心依赖。",
      blocks: [
        {
          kind: "text",
          value:
            "etcd 的前身是 CoreOS 团队为服务协调设计的存储引擎。它用简单的 KV 接口提供了三个关键保证：强一致性（任何节点读到的一定是最新数据）、Watch 机制（客户端可以监听 key 的变化并实时收到通知）和 Lease 租约（key 可以绑定一个带 TTL 的租约，到期自动删除）。",
        },
        {
          kind: "text",
          value: "etcd 最擅长这些事情：",
        },
        {
          kind: "list",
          items: [
            "服务注册与发现 —— 服务启动时把自己的地址写入 etcd 并绑定租约，定期续约，宕机后租约过期 key 自动消失。",
            "配置中心 —— 应用启动时拉取配置，并通过 Watch 监听变更，配置更新后应用秒级感知。",
            "分布式锁 —— 基于租约和事务实现的互斥锁，多个实例抢同一个资源时保证只有一个能操作。",
            "Leader 选举 —— 利用租约的自动过期机制，实现简单的选主逻辑。",
            "元数据存储 —— 存储集群拓扑、节点信息、运维元数据等小量但关键的数据。",
          ],
        },
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["Client Endpoint", `http://127.0.0.1:${port}`, "应用与 etcdctl 连接地址"],
            ["Peer Endpoint", "http://127.0.0.1:2380", "单节点内部通信端口"],
            ["TLS / Auth", "关闭", "仅绑定本机开发环境"],
            ["数据目录", "~/.devbox/instances/etcd/default/data", "持久化数据文件"],
          ],
        },
        {
          kind: "table",
          head: ["", "etcd", "Consul", "ZooKeeper"],
          rows: [
            ["一致性协议", "Raft", "Raft", "ZAB"],
            ["数据模型", "扁平 KV，支持前缀", "树形 KV，支持前缀", "树状节点（ZNode）"],
            ["Watch 机制", "支持，单 key / 前缀", "支持，通过阻塞查询", "支持，触发后需重新注册"],
            ["Gateway", "etcd gateway 代理", "Consul Client Agent", "无内建 gateway"],
            ["部署复杂度", "中等，需要证书", "低，单节点二元即可", "高，需要 Java"],
            ["典型场景", "K8s 核心、分布式协调", "服务发现 + KV 配置", "早年代协调服务"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "智屿以单节点模式运行",
          value:
            "集群模式下 etcd 需要至少 3 个节点才能保证高可用，但本地开发不需要。智屿以单节点启动，Raft 选举步骤直接跳过，写操作不会被阻塞，足够验证所有 etcd 客户端调用逻辑。",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "Put · Get · Delete",
      title: "读写第一条键值",
      intro:
        "智屿已经把 etcd 装好并启动。「连接与调试」标签页展示了 Client Endpoint 和 Peer Endpoint，以及常用命令速查。",
      blocks: [
        {
          kind: "list",
          items: [
            "在「概览」标签页确认服务是「运行中」。",
            "「连接与调试」标签页展示了 etcdctl 快速验证命令，可以直接复制到终端。",
            "下面的 etcdctl 命令在终端中直接运行即可，所有操作即时生效。",
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "设置别名方便使用",
          code: `export ETCD_ENDPOINTS=http://127.0.0.1:${port}
alias etcdctl='ETCDCTL_API=3 ~/.devbox/installations/etcd/3.6/bin/etcdctl --endpoints=$ETCD_ENDPOINTS'

# 验证连接
etcdctl version`,
        },
        {
          kind: "code",
          lang: "bash",
          caption: "基本读写操作",
          code: `# 写入
etcdctl put /app/config/timeout "30s"
etcdctl put /app/config/max_connections "100"

# 读取
etcdctl get /app/config/timeout
# 输出：/app/config/timeout
#       30s

# 按前缀读取
etcdctl get /app/config --prefix
# 返回 /app/config 下所有 key

# 只列出 key 名称
etcdctl get /app --prefix --keys-only

# 删除
etcdctl del /app/config/timeout

# 按前缀批量删除
etcdctl del /app/config --prefix`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "key 可以是任意字符串，但推荐用 / 分层",
          value:
            "etcd 的 key 没有强制命名规则，但社区习惯用类似文件路径的 / 分隔命名（如 /services/user-service/address）。这样做的好处是按前缀批量操作一批相关的 key（etcdctl get /services --prefix），逻辑清晰。",
        },
      ],
    },

    {
      id: "watch-lease",
      navLabel: "Watch 与 Lease",
      navHint: "监听变更 · 自动过期",
      title: "活用 Watch 和 Lease",
      intro:
        "etcd 的两个核心功能远超普通 KV 存储：Watch 让客户端实时感知数据变更，Lease 让 key 带自动过期能力。",
      blocks: [
        {
          kind: "text",
          value:
            "Watch 是 etcd 最具特色的功能之一。客户端对某个 key 或某个前缀建立 Watch 后，etcd 会在该范围内的数据发生变化时主动推送事件。这对配置中心、服务发现等场景至关重要：应用不需要轮询，变更发生后秒级感知。",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Watch 监听",
          code: `# 终端一：启动 Watch，会一直等待直到有变更
etcdctl watch /app/config/timeout

# 终端二：对同一个 key 做写入
etcdctl put /app/config/timeout "60s"

# 终端一会立即打印：
#   PUT
#   /app/config/timeout
#   60s

# 前缀 Watch
etcdctl watch /app --prefix

# 终端二：
etcdctl put /app/config/host "localhost"
# 终端一：
#   PUT
#   /app/config/host
#   localhost`,
        },
        {
          kind: "text",
          value:
            "Lease（租约）让 key 拥有 TTL。租约过期后，绑定在该租约上的所有 key 都会自动删除。这是服务注册发现的基础：服务实例把自己的信息写入 etcd 并绑定一个短租约，然后定期续约。实例崩溃后租约自然过期，注册信息自动消失。",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Lease 租约",
          code: `# 创建一个 30 秒的租约，返回租约 ID
etcdctl lease grant 30
# 输出：lease 694d8c6a6e7e0001 granted with TTL(30s)

# 写入 key 并绑定该租约
etcdctl put /services/gateway/instance-1 "http://127.0.0.1:8080" \\
  --lease=694d8c6a6e7e0001

# 查看 key 的剩余 TTL
etcdctl get /services/gateway/instance-1 -w json | grep lease

# 续约（让租约不掉）
etcdctl lease keep-alive 694d8c6a6e7e0001

# 30 秒不续约后，key 自动消失
sleep 35
etcdctl get /services/gateway/instance-1
# 输出：空（key 已自动删除）

# 查看所有租约
etcdctl lease list

# 撤销租约（立即删除绑定该租约的所有 key）
etcdctl lease revoke 694d8c6a6e7e0001`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "服务注册 = Lease + 续约 + Watch",
          value:
            "一个完整的服务注册流程就是：服务启动时 put 地址并绑定 10 秒 lease → 后台 goroutine 每 3 秒 keep-alive 一次 → 服务退出时 revoke lease → 其他服务 Watch 前缀感知到变更。这个模式几乎所有 etcd 客户端库都封装好了。",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Java · Go · TS · Python",
      title: "在你的项目里连上它",
      intro: `下面的配置都指向本机 http://127.0.0.1:${port}。所有语言的 etcd 客户端都默认使用 gRPC 与 etcd 通信（v3 API），无需额外配置。`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java",
              lang: "xml",
              caption: "pom.xml",
              code: `<dependency>
  <groupId>io.etcd</groupId>
  <artifactId>jetcd-core</artifactId>
  <version>0.8.2</version>
</dependency>`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "读写与 Watch",
              code: `import io.etcd.jetcd.*;
import io.etcd.jetcd.kv.GetResponse;
import io.etcd.jetcd.watch.WatchEvent;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.ExecutionException;

Client client = Client.builder()
    .endpoints("http://127.0.0.1:${port}")
    .build();
KV kv = client.getKVClient();

// 写入
kv.put(ByteSequence.from("hello", StandardCharsets.UTF_8),
       ByteSequence.from("zhiyu", StandardCharsets.UTF_8)).get();

// 读取
GetResponse resp = kv.get(ByteSequence.from("hello", StandardCharsets.UTF_8)).get();
String value = resp.getKvs().get(0).getValue().toString(StandardCharsets.UTF_8);

// 按前缀读取
kv.get(ByteSequence.from("/app/", StandardCharsets.UTF_8),
       io.etcd.jetcd.options.GetOption.newBuilder().isPrefix(true).build());

// Watch
Watch watch = client.getWatchClient();
watch.watch(ByteSequence.from("/app/", StandardCharsets.UTF_8),
    io.etcd.jetcd.options.WatchOption.newBuilder().isPrefix(true).build(),
    response -> {
        for (WatchEvent event : response.getEvents()) {
            System.out.println(event.getEventType() + " " +
                event.getKeyValue().getKey().toString(StandardCharsets.UTF_8));
        }
    });

// Lease
Lease lease = client.getLeaseClient();
long leaseId = lease.grant(30).get().getID();
kv.put(ByteSequence.from("/svc/instance-1", StandardCharsets.UTF_8),
       ByteSequence.from("http://localhost:8080", StandardCharsets.UTF_8),
       io.etcd.jetcd.options.PutOption.newBuilder().withLeaseId(leaseId).build()).get();
lease.keepAlive(leaseId, observer -> {});`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "安装 etcd Go SDK",
              code: `go get go.etcd.io/etcd/client/v3`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "读写与 Watch",
              code: `package etcdutil

import (
    "context"
    "time"

    clientv3 "go.etcd.io/etcd/client/v3"
)

var cli, _ = clientv3.New(clientv3.Config{
    Endpoints:   []string{"http://127.0.0.1:${port}"},
    DialTimeout: 5 * time.Second,
})

func Put(ctx context.Context, key, val string) error {
    _, err := cli.Put(ctx, key, val)
    return err
}

func Get(ctx context.Context, key string) (string, error) {
    resp, err := cli.Get(ctx, key)
    if err != nil {
        return "", err
    }
    if len(resp.Kvs) == 0 {
        return "", nil
    }
    return string(resp.Kvs[0].Value), nil
}

func GetByPrefix(ctx context.Context, prefix string) ([]string, error) {
    resp, err := cli.Get(ctx, prefix, clientv3.WithPrefix())
    if err != nil {
        return nil, err
    }
    var keys []string
    for _, kv := range resp.Kvs {
        keys = append(keys, string(kv.Key))
    }
    return keys, nil
}

func WatchPrefix(ctx context.Context, prefix string, handler func(eventType, key, value string)) {
    ch := cli.Watch(ctx, prefix, clientv3.WithPrefix())
    for resp := range ch {
        for _, ev := range resp.Events {
            handler(ev.Type.String(), string(ev.Kv.Key), string(ev.Kv.Value))
        }
    }
}

func RegisterService(ctx context.Context, key, addr string, ttl int64) (<-chan *clientv3.LeaseKeepAliveResponse, error) {
    lease, err := cli.Grant(ctx, ttl)
    if err != nil {
        return nil, err
    }
    _, err = cli.Put(ctx, key, addr, clientv3.WithLease(lease.ID))
    if err != nil {
        return nil, err
    }
    keepAlive, err := cli.KeepAlive(ctx, lease.ID)
    return keepAlive, err
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "安装 etcd3",
              code: `npm install etcd3`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "读写与 Watch",
              code: `import { Etcd3 } from "etcd3";

const client = new Etcd3({ hosts: "http://127.0.0.1:${port}" });

// 写入
await client.put("/app/config/timeout").value("30s");

// 读取
const val = await client.get("/app/config/timeout").string();
console.log(val); // "30s"

// 按前缀读取
const all = await client.getAll().prefix("/app/").strings();
console.log(all); // Map: { "/app/config/timeout" => "30s", ... }

// Watch
const watcher = await client.watch().prefix("/app/").create();
watcher.on("put", (res) => {
  console.log("PUT", res.key.toString(), res.value.toString());
});

// Lease
const lease = client.lease(30);
await lease.put("/svc/instance-1").value("http://localhost:8080");
lease.on("lost", () => console.log("Lease lost!"));
setInterval(() => lease.keepaliveOnce(), 10_000); // 每 10 秒续约`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "安装 etcd3-py",
              code: `pip install etcd3`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "读写与 Watch",
              code: `import etcd3

client = etcd3.client(host="127.0.0.1", port=${port})

# 写入
client.put("/app/config/timeout", "30s")

# 读取
value, _ = client.get("/app/config/timeout")
print(value.decode() if value else None)  # b"30s"

# 按前缀读取
for value, metadata in client.get_prefix("/app/"):
    print(metadata.key.decode(), value.decode())

# Watch（会阻塞当前线程）
events_iter, cancel = client.watch_prefix("/app/")
for event in events_iter:
    print(f"{event.event_type} {event.key}")

# Lease
lease = client.lease(30)
client.put("/svc/instance-1", "http://localhost:8080", lease=lease)
lease.refresh()  # 续约`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "注意 etcd v2 和 v3 API 的区别",
          value:
            "etcd v2 API 已经弃用，v3 使用 gRPC 协议。所有现代 etcd 客户端库默认都是 v3 API。如果你项目中同时引用了旧版依赖，确认 etcdctl 的环境变量设置了 ETCDCTL_API=3，否则 get 和 put 走的是不同版本的 API，数据互不可见。",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "避坑与调优",
      navHint: "排查 · 常见问题",
      title: "上线前该知道的事",
      intro: "etcd 在单节点开发模式下特别顺利，生产环境有几个需要提前注意的地方。",
      blocks: [
        {
          kind: "table",
          head: ["问题", "现象", "对策"],
          rows: [
            [
              "单节点无高可用",
              "etcd 挂了所有依赖它的服务都受影响",
              "生产环境至少部署 3 节点集群",
            ],
            [
              "数据量过大",
              "etcd 不适合存大量数据，默认有 1.5 GiB 存储限制",
              "只存关键元数据和配置；大文件用对象存储",
            ],
            [
              "频繁 Watch 重连",
              "网络抖动导致 Watch 断开，需要重连",
              "客户端库自带自动重连，确认超时和重试参数设置合理",
            ],
            [
              "Lease 未续约导致服务误下线",
              "服务还在运行但注册信息消失了",
              "续约间隔设为 TTL 的 1/3，留足余量",
            ],
            [
              "v2 API 已弃用",
              "旧版工具默认走 v2，和 v3 数据隔离",
              "export ETCDCTL_API=3；生产升级前检查所有调用方",
            ],
            [
              "权限未配置",
              "本地开发无认证，生产环境要配",
              "生产环境启用 RBAC 权限控制，设置 root 用户密码",
            ],
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "排查命令",
          code: `alias etcdctl='ETCDCTL_API=3 ~/.devbox/installations/etcd/3.6/bin/etcdctl --endpoints=http://127.0.0.1:2379'

# 集群状态
etcdctl endpoint status
etcdctl endpoint health

# 查看所有 key（小心大数据量）
etcdctl get "" --prefix --keys-only

# 查看存储用量
etcdctl endpoint status -w table

# 压缩历史版本（释放空间）
etcdctl compact $(etcdctl endpoint status -w json | grep -o '"revision":[0-9]*' | head -1 | cut -d: -f2)`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "生产环境必须启用 TLS 和认证",
          value:
            "本地开发智屿关闭了 TLS 和认证以便快速上手，但生产环境请务必启用。不设防的 etcd 等于把服务注册信息、配置数据和分布式锁密钥全部暴露。至少要做：① 启用 TLS 加密通信；② 设置 root 密码并开启认证；③ 用 etcd auth 命令创建有限权限的子账号。",
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么做",
          value:
            "「概览」标签页展示了版本、存储大小等指标；「连接与调试」标签页有常用 etcdctl 命令快速参考，可以直接复制到终端使用；「运行日志」能看到启动报错和 Raft 状态信息；改端口或数据目录在「配置文件」标签页编辑后重启即可；做危险操作前去「备份恢复」打一个快照。",
        },
      ],
    },
  ];
}
