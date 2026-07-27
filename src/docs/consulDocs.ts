import type { DocChapter } from "./docTypes";

export function buildConsulDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 Consul",
      navHint: "服务发现 · KV",
      title: "Consul 是什么",
      intro:
        "Consul 是 HashiCorp 开发的服务网络平台。它把服务注册、健康检查、KV 配置和 DNS 服务发现打包成一个二进制文件，是微服务架构里最成熟的开源方案之一。",
      blocks: [
        {
          kind: "text",
          value:
            "Consul 的设计非常务实：它的核心是一个 Server Agent 进程，所有功能都通过一个 HTTP API 暴露。服务实例通过 Agent 注册自己的地址和健康检查接口，Consul 自动维护一份「当前有哪些服务是健康的」列表。其他服务可以通过 HTTP API 或 DNS 查询这个列表，完全不需要自己维护注册中心。",
        },
        {
          kind: "text",
          value: "Consul 在一套系统里同时提供了：",
        },
        {
          kind: "list",
          items: [
            "服务注册：服务启动时调用 API 注册地址、端口和健康检查地址，Consul 持续探测。",
            "健康检查：支持 HTTP、TCP、gRPC 和脚本检查，自动剔除非健康实例。",
            "KV Store：和 etcd 一样的键值存储，可以用来存动态配置。",
            "DNS 服务发现：直接 dig 或 nslookup 服务名就能拿到健康实例的 IP 列表。",
            "访问控制（ACL）：控制谁能读、谁能写、谁能注册服务。",
          ],
        },
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["HTTP API", `http://127.0.0.1:${port}`, "SDK 与 REST API"],
            ["Web UI", `http://127.0.0.1:${port}/ui/`, "服务和 KV 管理界面"],
            ["DNS", "127.0.0.1:8600", "DNS 服务发现"],
            ["数据目录", "~/.devbox/instances/consul/default/data", "持久化数据"],
          ],
        },
        {
          kind: "table",
          head: ["", "Consul", "etcd", "Nacos"],
          rows: [
            ["服务发现", "内置 + DNS", "通过 Lease+Watch 实现", "内置 + gRPC"],
            ["健康检查", "内置多种检查器", "通过 Lease 超时判断", "Nacos Client 上报心跳"],
            ["KV 配置", "内置，支持 Watch", "内置，支持 Watch", "内置，支持 Watch"],
            ["Web UI", "内置，功能完整", "无（需独立工具）", "内置控制台"],
            ["多数据中心", "内置原生支持", "不支持", "不支持"],
            ["部署", "Go 单个二进制", "Go 单个二进制", "Java / Rust 各版本"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "智屿以 dev 模式启动单节点 Consul Server",
          value:
            "生产环境 Consul 通常以集群（3-5 个 Server + 若干 Client Agent）运行，但本地开发不需要。智屿以 -dev 模式启动单个 Server，数据不会持久化到磁盘（重启丢失），适合快速验证代码逻辑。",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "注册 · 查询 · KV",
      title: "注册一个服务并查回来",
      intro:
        "智屿已经把 Consul 装好并启动。「连接与调试」标签页展示了 HTTP API 地址和 DNS 端口，浏览器打开 Web UI 可以可视化操作所有服务和 KV。",
      blocks: [
        {
          kind: "list",
          items: [
            "在「概览」标签页确认服务是「运行中」。",
            "「连接与调试」标签页有 HTTP API 和 DNS 地址，以及 `consul members` 快速验证命令。",
            `浏览器打开 http://127.0.0.1:${port}/ui/，这是 Consul 自带的 Web 管理界面，可以直接注册服务和操作 KV。`,
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "服务注册",
          code: `CONSUL=http://127.0.0.1:${port}

# 注册一个服务实例
curl -X PUT "$CONSUL/v1/agent/service/register" \\
  -H "Content-Type: application/json" \\
  -d '{
    "ID": "user-service-1",
    "Name": "user-service",
    "Address": "127.0.0.1",
    "Port": 8080,
    "Tags": ["v1", "primary"],
    "Check": {
      "HTTP": "http://127.0.0.1:8080/health",
      "Interval": "10s",
      "Timeout": "2s",
      "DeregisterCriticalServiceAfter": "60s"
    }
  }'

# 再注册第二个实例（模拟多实例部署）
curl -X PUT "$CONSUL/v1/agent/service/register" \\
  -H "Content-Type: application/json" \\
  -d '{
    "ID": "user-service-2",
    "Name": "user-service",
    "Address": "127.0.0.1",
    "Port": 8081,
    "Tags": ["v1", "secondary"]
  }'`,
        },
        {
          kind: "code",
          lang: "bash",
          caption: "服务查询",
          code: `# HTTP API 查询服务实例列表
curl "$CONSUL/v1/agent/services"

# 只查某个服务的实例
curl "$CONSUL/v1/agent/service/user-service"

# 健康检查过滤（只返回健康的实例）
curl "$CONSUL/v1/health/service/user-service?passing"

# DNS 查询（8600 端口）
dig @127.0.0.1 -p 8600 user-service.service.consul
# 返回两条 A 记录 127.0.0.1
# 加上 SRV 查询能看到端口
dig @127.0.0.1 -p 8600 user-service.service.consul SRV

# 注销服务
curl -X PUT "$CONSUL/v1/agent/service/deregister/user-service-1"`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "DeregisterCriticalServiceAfter 很有用",
          value:
            "健康检查持续失败超过这个时间后，Consul 会自动注销该实例。这避免了你手动清理已宕机的服务注册记录。配合 DNS 查询，客户端永远不会拿到已经挂掉的服务地址。",
        },
      ],
    },

    {
      id: "kv",
      navLabel: "KV 配置中心",
      navHint: "动态配置 · Watch",
      title: "用 KV Store 存动态配置",
      intro:
        "Consul 的 KV Store 提供了和 etcd 类似的功能：写入键值、按前缀查询、Watch 变更。很多项目把 Consul 同时当服务发现和配置中心用，一套系统解决两个问题。",
      blocks: [
        {
          kind: "code",
          lang: "bash",
          caption: "KV 基本操作",
          code: `CONSUL=http://127.0.0.1:${port}

# 写入
curl -X PUT "$CONSUL/v1/kv/app/config/timeout" -d "30s"
curl -X PUT "$CONSUL/v1/kv/app/config/max_connections" -d "100"

# 读取（返回 JSON，value 是 base64 编码）
curl "$CONSUL/v1/kv/app/config/timeout?raw"
# 带 ?raw 参数直接返回原始值，不用 base64 解码

# 按前缀读取（递归）
curl "$CONSUL/v1/kv/app/config?recurse"

# 按前缀列出 key 名称
curl "$CONSUL/v1/kv/app/config?keys"

# 删除
curl -X DELETE "$CONSUL/v1/kv/app/config/timeout"

# 按前缀递归删除
curl -X DELETE "$CONSUL/v1/kv/app/config?recurse"`,
        },
        {
          kind: "text",
          value:
            "Watch 是配置中心的核心能力：应用启动时把配置拉下来，然后建立一个 Watch 连接。当运维在 Web UI 或 API 上修改配置后，应用会立刻收到通知并重新加载：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Watch 变更（阻塞查询）",
          code: `# Consul 的 Watch 通过「阻塞查询」实现：
# 每次请求带上 ?index=上次的 index 值，服务端挂起直到有变更才返回

# 获取当前 index
CONSUL_INDEX=$(curl -s "$CONSUL/v1/kv/app/config?recurse" | \\
  python3 -c "import sys,json; print(max(i['ModifyIndex'] for i in json.load(sys.stdin)))" 2>/dev/null || echo 0)

# 阻塞等待变更（最长等 60 秒）
curl "$CONSUL/v1/kv/app/config?recurse&index=$CONSUL_INDEX&wait=60s"

# 这个命令会一直挂着，直到有变更才会返回新数据。
# 这就是 HTTP 版的 Watch 机制，客户端库通常封装好了这个逻辑。`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Web UI 改配置很方便",
          value:
            `打开 http://127.0.0.1:${port}/ui/，左侧菜单进入 Key/Value，可以直接在界面上创建、编辑和删除 key。修改后 Watch 方会立刻感知到。这在开发阶段调试配置中心逻辑非常有用。`,
        },
      ],
    },

    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Java · Go · TS · Python",
      title: "在你的项目里连上它",
      intro: `下面的配置都指向本机 http://127.0.0.1:${port}。所有语言的库基本都是对 HTTP API 的封装，API 用法高度一致。`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java (Spring Cloud)",
              lang: "yaml",
              caption: "application.yml",
              code: `spring:
  cloud:
    consul:
      host: 127.0.0.1
      port: ${port}
      discovery:
        enabled: true
        register: true
        health-check-path: /actuator/health
        health-check-interval: 10s
        instance-id: \${spring.application.name}-\${server.port}
      config:
        enabled: true
        prefix: config
        default-context: application`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "Spring Cloud Consul 自动注册",
              code: `// Spring Cloud Consul 会自动：
// 1. 把服务注册到 Consul
// 2. 上报健康检查
// 3. 从 Consul KV 加载配置
// 4. 定时心跳续约
// 只需要上面的 yml 配置 + @EnableDiscoveryClient 即可
@SpringBootApplication
@EnableDiscoveryClient
public class UserServiceApplication {
    public static void main(String[] args) {
        SpringApplication.run(UserServiceApplication.class, args);
    }
}

// 代码里使用 DiscoveryClient 获取其他服务的地址
@Autowired
private DiscoveryClient discoveryClient;

List<ServiceInstance> instances = discoveryClient.getInstances("user-service");
String url = "http://" + instances.get(0).getHost() + ":" + instances.get(0).getPort();`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "安装 consul-api",
              code: `go get github.com/hashicorp/consul/api`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "服务注册与发现",
              code: `package consulutil

import (
    "github.com/hashicorp/consul/api"
)

var client, _ = api.NewClient(&api.Config{Address: "http://127.0.0.1:${port}"})

func RegisterService(id, name, address string, port int) error {
    return client.Agent().ServiceRegister(&api.AgentServiceRegistration{
        ID:      id,
        Name:    name,
        Address: address,
        Port:    port,
        Check: &api.AgentServiceCheck{
            HTTP:     "http://" + address + ":" + fmt.Sprint(port) + "/health",
            Interval: "10s",
            Timeout:  "2s",
            DeregisterCriticalServiceAfter: "60s",
        },
    })
}

func DeregisterService(id string) error {
    return client.Agent().ServiceDeregister(id)
}

func DiscoverService(name string) ([]*api.ServiceEntry, error) {
    entries, _, err := client.Health().Service(name, "", true, nil)
    return entries, err
}

// KV 操作
func PutKV(key, value string) error {
    _, err := client.KV().Put(&api.KVPair{Key: key, Value: []byte(value)}, nil)
    return err
}

func GetKV(key string) (string, error) {
    pair, _, err := client.KV().Get(key, nil)
    if err != nil || pair == nil {
        return "", err
    }
    return string(pair.Value), nil
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "安装 consul",
              code: `npm install consul`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "服务注册与发现",
              code: `import Consul from "consul";

const consul = new Consul({ host: "127.0.0.1", port: "${port}" });

// 注册服务
await consul.agent.service.register({
  id: "user-service-1",
  name: "user-service",
  address: "127.0.0.1",
  port: 8080,
  tags: ["v1"],
  check: {
    http: "http://127.0.0.1:8080/health",
    interval: "10s",
    timeout: "2s",
    deregistercriticalserviceafter: "60s",
  },
});

// 发现服务
const services = await consul.health.service({ service: "user-service", passing: true });
services.forEach(s => {
  console.log(s.Service.Address, s.Service.Port);
});

// KV 操作
await consul.kv.set("app/config/timeout", "30s");
const val = await consul.kv.get("app/config/timeout");
console.log(val?.Value); // "30s"`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "安装 python-consul",
              code: `pip install python-consul`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "服务注册与发现",
              code: `import consul

c = consul.Consul(host="127.0.0.1", port=${port})

# 注册服务
c.agent.service.register(
    name="user-service",
    service_id="user-service-1",
    address="127.0.0.1",
    port=8080,
    tags=["v1"],
    check=consul.Check.http(
        "http://127.0.0.1:8080/health",
        interval="10s",
        timeout="2s",
        deregister="60s",
    ),
)

# 发现服务（只返回健康的）
_, nodes = c.health.service("user-service", passing=True)
for node in nodes:
    print(node["Service"]["Address"], node["Service"]["Port"])

# 注销
c.agent.service.deregister("user-service-1")

# KV 操作
c.kv.put("app/config/timeout", "30s")
_, kv = c.kv.get("app/config/timeout")
print(kv["Value"].decode())  # "30s"`,
            },
          ],
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "避坑与调优",
      navHint: "排查 · 常见问题",
      title: "上线前该知道的事",
      intro: "Consul 本地开发很顺滑，但生产环境部署需要注意几个关键配置。",
      blocks: [
        {
          kind: "table",
          head: ["问题", "现象", "对策"],
          rows: [
            [
              "Dev 模式不持久化",
              "重启 Consul 后服务注册信息全丢了",
              "生产环境不要用 -dev 模式，配置 data_dir 开启持久化",
            ],
            [
              "健康检查不通",
              "所有服务都被标记为不健康",
              "确认健康检查地址可被 Consul 访问（绑定 0.0.0.0 而非 127.0.0.1 时需注意）",
            ],
            [
              "DNS 查询不到",
              "dig 服务名返回空",
              "Consul DNS 默认在 8600 端口，确认查的是 .service.consul 后缀",
            ],
            [
              "Agent 内存过高",
              "Consul Agent 内存持续增长",
              "限制 catalog 缓存数量；定期清理不活跃的 KV 条目",
            ],
            [
              "集群脑裂",
              "多节点间出现不一致",
              "生产部署至少 3 个 Server；确认 gossip 协议端口（8301）互通",
            ],
            [
              "ACL 未启用",
              "任何人都可以注册和注销服务",
              "生产环境启用 ACL Token，限制写入操作",
            ],
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "排查命令",
          code: `CONSUL=http://127.0.0.1:${port}

# 列出当前注册的所有服务
curl "$CONSUL/v1/agent/services"

# 查看集群成员
curl "$CONSUL/v1/agent/members"

# 查看节点健康检查状态
curl "$CONSUL/v1/agent/checks"

# 查看所有 KV
curl "$CONSUL/v1/kv/?keys"
curl "$CONSUL/v1/kv/?recurse"

# 强制触发健康检查
curl -X PUT "$CONSUL/v1/agent/check/pass/service:user-service-1"`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么做",
          value:
            `「概览」标签页展示了服务状态和端口；「连接与调试」标签页有 HTTP API 和 DNS 地址速查；打开 http://127.0.0.1:${port}/ui/ 可以看到所有注册的服务、健康检查状态和 KV 树；「运行日志」能看到启动报错和 API 调用日志；改端口或数据目录在「配置文件」标签页编辑后重启即可；做危险操作前记得去「备份恢复」打一个快照。`,
        },
      ],
    },
  ];
}
