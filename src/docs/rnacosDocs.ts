import type { DocChapter } from "./docTypes";

export function buildRnacosDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "认识 rnacos",
      navHint: "Nacos 兼容 · 配置中心",
      title: "rnacos 是什么",
      intro:
        "rnacos 是使用 Rust 实现的 Nacos 兼容服务，适合本地调试配置中心、服务注册与发现，无需安装 Java Runtime。",
      blocks: [
        {
          kind: "text",
          value:
            "Nacos（Dynamic Naming and Configuration Service）是阿里巴巴开源的服务发现和配置管理平台，在国内微服务生态中使用极广。但官方 Nacos Server 是 Java 编写，本地跑一个 Nacos 需要先配 JVM 参数，占内存也大。rnacos 把整个 Nacos 协议用 Rust 重新实现了一遍，API 完全兼容，但启动快、内存小、一个二进制搞定，是本地开发的最佳替代。",
        },
        {
          kind: "text",
          value: "rnacos 同时提供：",
        },
        {
          kind: "list",
          items: [
            "配置中心 —— 支持配置的发布、修改、版本管理和回滚，应用可以热加载最新配置。",
            "服务注册与发现 —— 兼容 Nacos 1.x OpenAPI 和 2.x gRPC 客户端协议。",
            "Web Console —— 自带管理界面，可以在浏览器里管理配置和服务。",
          ],
        },
        {
          kind: "table",
          head: ["项目", "值", "说明"],
          rows: [
            ["Nacos HTTP", `http://127.0.0.1:${port}`, "1.x OpenAPI 与客户端"],
            ["Nacos gRPC", "127.0.0.1:9848", "2.x 客户端协议"],
            ["Web Console", "http://127.0.0.1:10848/rnacos/", "默认 admin / admin"],
          ],
        },
        {
          kind: "table",
          head: ["", "rnacos", "Nacos (Java)", "Consul"],
          rows: [
            ["运行时", "Rust 单个二进制，~10MB 内存", "Java 进程，~512MB 起步", "Go 单个二进制"],
            ["启动速度", "秒级", "几十秒到分钟级", "秒级"],
            ["API 兼容性", "完全兼容 Nacos", "Nacos 官方", "不兼容 Nacos"],
            ["配置管理", "内置，支持版本回滚", "内置，功能最完整", "KV Store，无自动回滚"],
            ["服务发现", "HTTP + gRPC 双协议", "HTTP + gRPC 双协议", "HTTP + DNS"],
            ["适合场景", "本地 Nacos 开发测试", "生产环境", "通用服务发现"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "仅限本地开发",
          value:
            "默认控制台账号密码为 admin / admin，OpenAPI 鉴权关闭。不要将此实例暴露到公网。生产环境请使用官方 Nacos 集群部署。",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "快速上手",
      navHint: "配置 · 服务注册",
      title: "发布一条配置并验证",
      intro:
        "智屿已经把 rnacos 装好并启动。你可以在「连接与调试」标签页看到 HTTP、gRPC 和 Web Console 三个端点的地址，也可以用浏览器打开 Console 进行操作。",
      blocks: [
        {
          kind: "list",
          items: [
            "在「概览」标签页确认状态是「运行中」。",
            "「连接与调试」标签页有 Nacos HTTP、gRPC 和 Web Console 三个地址，可以直接复制使用。",
            "浏览器打开 http://127.0.0.1:10848/rnacos/，用 admin / admin 登录 Web Console，可以可视化管理配置和服务。",
          ],
        },
        {
          kind: "text",
          value:
            "Nacos 配置管理的核心概念：namespace（命名空间/环境隔离）→ group（分组）→ dataId（配置项标识）。本地开发通常用默认的 public namespace 和 DEFAULT_GROUP。",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "发布和读取配置",
          code: `NACOS=http://127.0.0.1:${port}

# 发布配置
curl -X POST "$NACOS/nacos/v1/cs/configs" \\
  -d "dataId=application.properties" \\
  -d "group=DEFAULT_GROUP" \\
  -d "content=server.port=8080
app.name=demo
app.timeout=30s"

# 读取配置
curl "$NACOS/nacos/v1/cs/configs?dataId=application.properties&group=DEFAULT_GROUP"

# 删除配置
curl -X DELETE "$NACOS/nacos/v1/cs/configs" \\
  -d "dataId=application.properties" \\
  -d "group=DEFAULT_GROUP"

# 发布 JSON 格式配置
curl -X POST "$NACOS/nacos/v1/cs/configs" \\
  -d "dataId=database.json" \\
  -d "group=DEFAULT_GROUP" \\
  -d 'content={"host":"127.0.0.1","port":3306,"db":"demo"}'`,
        },
        {
          kind: "text",
          value: "查看配置的版本历史和回滚：",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "配置版本与回滚",
          code: `# 查看配置的版本历史
curl "$NACOS/nacos/v1/cs/history?dataId=application.properties&group=DEFAULT_GROUP"

# 查看某个历史版本的内容
# 从上面的历史列表里拿到 id，然后：
curl "$NACOS/nacos/v1/cs/history?nid=<history_id>"

# 回滚到上一个版本
curl -X POST "$NACOS/nacos/v1/cs/configs" \\
  -d "dataId=application.properties" \\
  -d "group=DEFAULT_GROUP" \\
  -d 'content=new content'`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Nacos 的 dataId 命名惯例",
          value:
            "Spring Cloud Alibaba 项目中，dataId 格式是 ${prefix}-${spring.profile.active}.${file-extension}。例如 application-dev.properties、userservice-dev.yml。这个命名是 Spring Cloud Nacos Config 自动加载的依据，不要随便取。",
        },
      ],
    },

    {
      id: "service-discovery",
      navLabel: "服务注册发现",
      navHint: "注册 · 发现 · 心跳",
      title: "注册一个服务并发现它",
      intro:
        "除了配置中心，rnacos 也完全兼容 Nacos 的服务注册发现协议。Nacos 1.x 客户端用 HTTP API 注册，2.x 客户端用 gRPC 注册，rnacos 两者都支持。",
      blocks: [
        {
          kind: "code",
          lang: "bash",
          caption: "通过 HTTP API 注册和发现服务",
          code: `NACOS=http://127.0.0.1:${port}

# 注册一个服务实例
curl -X POST "$NACOS/nacos/v1/ns/instance" \\
  -d "serviceName=user-service" \\
  -d "ip=127.0.0.1" \\
  -d "port=8080" \\
  -d "weight=1.0" \\
  -d "healthy=true" \\
  -d "metadata={\\"version\\":\\"v1\\"}"

# 查询服务下的所有实例列表
curl "$NACOS/nacos/v1/ns/instance/list?serviceName=user-service"

# 查询健康的实例
curl "$NACOS/nacos/v1/ns/instance/list?serviceName=user-service&healthyOnly=true"

# 发送心跳（1.x 客户端每 5 秒发一次）
curl -X PUT "$NACOS/nacos/v1/ns/instance/beat" \\
  -d "serviceName=user-service" \\
  -d "ip=127.0.0.1" \\
  -d 'port=8080' \\
  -d 'beat={"metadata":{}}'

# 注销服务
curl -X DELETE "$NACOS/nacos/v1/ns/instance" \\
  -d "serviceName=user-service" \\
  -d "ip=127.0.0.1" \\
  -d "port=8080"`,
        },
        {
          kind: "text",
          value:
            "rnacos 还兼容 Nacos 2.x 的 gRPC 协议（端口 9848），支持长连接推送变更。Spring Cloud Alibaba 2022.x+ 和 Nacos Client 2.x 默认使用 gRPC，可以无缝集成。",
        },
        {
          kind: "callout",
          tone: "tip",
          title: "心跳很重要",
          value:
            "1.x 客户端需要主动发送心跳来维持注册状态，默认每隔 5 秒发一次。如果 Nacos 在 15 秒内没收到心跳，会标记实例为不健康；30 秒后自动剔除。2.x gRPC 是长连接，断连即认为服务下线。",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "语言接入",
      navHint: "Spring · Go · TS · Python",
      title: "在你的项目里连上它",
      intro: `rnacos 完全兼容 Nacos 协议，所以所有语言的 Nacos 客户端都可以直接连上它。下面是各语言的使用示例，把 server-addr 指向 127.0.0.1:${port} 即可。`,
      blocks: [
        {
          kind: "samples",
          samples: [
            {
              label: "Java (Spring Cloud Alibaba)",
              lang: "yaml",
              caption: "application.yml",
              code: `spring:
  application:
    name: demo-service
  cloud:
    nacos:
      discovery:
        server-addr: 127.0.0.1:${port}
        enabled: true
        namespace: public
      config:
        server-addr: 127.0.0.1:${port}
        namespace: public
        group: DEFAULT_GROUP
        file-extension: yaml
        # 自动加载 dataId: demo-service.yaml
        refresh-enabled: true   # 配置变更后自动刷新 @RefreshScope`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "配置热刷新与 @NacosValue",
              code: `@RestController
@RefreshScope  // 默认 Nacos Config 的数据变更后
              // 带有此注解的 Bean 会重新加载
public class DemoController {

    @Value("\${app.timeout:30s}")
    private String timeout;

    @GetMapping("/config/timeout")
    public String timeout() {
        return timeout;
    }
}

// 服务发现
@Autowired
private NacosDiscoveryClient discoveryClient;

List<ServiceInstance> instances = discoveryClient.getInstances("user-service");
String url = instances.get(0).getUri().toString();`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "安装 nacos-sdk-go",
              code: `go get github.com/nacos-group/nacos-sdk-go/v2`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "配置与发现",
              code: `package nacosutil

import (
    "github.com/nacos-group/nacos-sdk-go/v2/clients"
    "github.com/nacos-group/nacos-sdk-go/v2/common/constant"
    "github.com/nacos-group/nacos-sdk-go/v2/vo"
)

var configClient, namingClient = initClients()

func initClients() (config_client.IConfigClient, naming_client.INamingClient) {
    sc := []constant.ServerConfig{
        *constant.NewServerConfig("127.0.0.1", ${port}),
    }
    cc := *constant.NewClientConfig(
        constant.WithNamespaceId(""),
        constant.WithTimeoutMs(5000),
        constant.WithLogLevel("info"),
    )

    configClient, _ := clients.NewConfigClient(
        vo.NacosClientParam{ClientConfig: &cc, ServerConfigs: sc},
    )

    namingClient, _ := clients.NewNamingClient(
        vo.NacosClientParam{ClientConfig: &cc, ServerConfigs: sc},
    )
    return configClient, namingClient
}

// 读取配置
func GetConfig(dataId, group string) (string, error) {
    return configClient.GetConfig(vo.ConfigParam{DataId: dataId, Group: group})
}

// 监听配置变更
func ListenConfig(dataId, group string, callback func(string)) error {
    return configClient.ListenConfig(vo.ConfigParam{
        DataId: dataId,
        Group:  group,
        OnChange: func(namespace, group, dataId, content string) {
            callback(content)
        },
    })
}

// 注册服务
func RegisterService(name, ip string, port uint64) (bool, error) {
    return namingClient.RegisterInstance(vo.RegisterInstanceParam{
        ServiceName: name,
        Ip:          ip,
        Port:        port,
        Weight:      1,
        Healthy:     true,
        Enable:      true,
        Ephemeral:   true,
    })
}`,
            },
            {
              label: "TypeScript",
              lang: "bash",
              caption: "安装 nacos-node",
              code: `npm install nacos`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "配置与发现",
              code: `import Nacos from "nacos";

const nacos = new Nacos.NacosClient({
  serverAddr: "127.0.0.1:${port}",
  namespace: "",
  logger: console,
});

// 读取配置
const content = await nacos.getConfig("application.properties", "DEFAULT_GROUP");
console.log(content);

// 发布配置
await nacos.publishConfig("application.properties", "DEFAULT_GROUP",
  "server.port=8080\\napp.name=demo");

// 注册服务
await nacos.registerInstance("user-service", {
  ip: "127.0.0.1",
  port: 8080,
  weight: 1,
  healthy: true,
});

// 查询服务实例
const instances = await nacos.getAllInstances("user-service");
instances.forEach(i => console.log(i.ip, i.port));`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "安装 nacos-sdk-python",
              code: `pip install nacos-sdk-python`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "配置与发现",
              code: `import nacos

SERVER_ADDRESSES = "127.0.0.1:${port}"

client = nacos.NacosClient(SERVER_ADDRESSES)

# 读取配置
content = client.get_config("application.properties", "DEFAULT_GROUP")
print(content)

# 发布配置
client.publish_config("application.properties", "DEFAULT_GROUP",
    "server.port=8080\\napp.version=v2")

# 移除配置
client.remove_config("application.properties", "DEFAULT_GROUP")

# 注册服务
client.add_naming_instance("user-service", "127.0.0.1", 8080,
    weight=1.0, healthy=True)

# 查询服务实例
instances = client.list_naming_instance("user-service")
for inst in instances.get("hosts", []):
    print(inst["ip"], inst["port"])`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Spring Cloud Nacos 的 server-addr 不要带协议前缀",
          value:
            "spring.cloud.nacos.discovery.server-addr 的值直接写成 127.0.0.1:8848，不要写成 http://127.0.0.1:8848。带上 http:// 会导致客户端解析地址失败。这个坑几乎所有新手都会踩一次。",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "避坑与调优",
      navHint: "排查 · 常见问题",
      title: "上线前该知道的事",
      intro: "rnacos 作为 Nacos 的替代品在开发阶段很好用，但有几个注意点。",
      blocks: [
        {
          kind: "table",
          head: ["问题", "现象", "对策"],
          rows: [
            [
              "server-addr 格式错误",
              "Spring Cloud Nacos 连接报错",
              "去掉 http:// 前缀，直接写 ip:port",
            ],
            [
              "2.x 客户端连不上",
              "gRPC 连接失败",
              "确认 9848 端口可访问；rnacos 同时开启了 1.x HTTP 和 2.x gRPC",
            ],
            [
              "控制台默认密码不安全",
              "任何人都能用 admin/admin 登录修改配置",
              "本地开发可不改；暴露到非本机网络时务必改密码或关闭控制台绑定",
            ],
            [
              "配置不刷新",
              "修改配置后应用没收到变更",
              "确认加上了 @RefreshScope；或客户端代码里开启了 ListenConfig",
            ],
            [
              "服务被误剔",
              "运行正常的服务在注册列表里消失了",
              "检查心跳是否正常发送（1.x 默认 5 秒一次）；网络抖动可能导致 15 秒超时",
            ],
            [
              "namespace 理解错误",
              "配置写进了 A 命名空间，客户端从 B 查不到",
              "namespace 隔离级别最高；确认服务端和客户端用了同一个 namespaceId",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "在智屿里怎么做",
          value:
            "「概览」标签页能看到服务状态和端口；「连接与调试」标签页有 HTTP、gRPC 和 Console 三个端点地址速查；打开 http://127.0.0.1:10848/rnacos/ 可以用 Web Console 管理配置和服务；「运行日志」能看到 API 调用记录和启动报错；做危险操作前记得去「备份恢复」打一个快照。改端口和鉴权参数在「配置文件」标签页编辑后重启即可。",
        },
      ],
    },
  ];
}
