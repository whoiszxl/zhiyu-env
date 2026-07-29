import type { DocChapter } from "../docTypes";

export function buildRnacosDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "Meet rnacos",
      navHint: "Nacos compatible · Config center",
      title: "What is rnacos",
      intro:
        "rnacos is a Nacos-compatible service implemented in Rust. It is ideal for locally debugging the config center and service registration/discovery without needing a Java runtime.",
      blocks: [
        {
          kind: "text",
          value:
            "Nacos (Dynamic Naming and Configuration Service) is Alibaba's open-source service discovery and configuration management platform, and it is extremely widely used in the Chinese microservice ecosystem. However, the official Nacos Server is written in Java: running one locally means configuring JVM parameters and eating a lot of memory. rnacos reimplements the entire Nacos protocol in Rust — API-compatible, but with fast startup, small memory footprint, and delivered as a single binary. It's the best replacement for local development.",
        },
        {
          kind: "text",
          value: "rnacos also provides:",
        },
        {
          kind: "list",
          items: [
            "Config center — supports publishing, editing, versioning, and rollback of configurations; apps can hot-reload the latest config.",
            "Service registration and discovery — compatible with the Nacos 1.x OpenAPI and the 2.x gRPC client protocol.",
            "Web Console — a built-in management UI for managing configs and services from the browser.",
          ],
        },
        {
          kind: "table",
          head: ["Item", "Value", "Notes"],
          rows: [
            ["Nacos HTTP", `http://127.0.0.1:${port}`, "1.x OpenAPI and clients"],
            ["Nacos gRPC", "127.0.0.1:9848", "2.x client protocol"],
            ["Web Console", "http://127.0.0.1:10848/rnacos/", "Default admin / admin"],
          ],
        },
        {
          kind: "table",
          head: ["", "rnacos", "Nacos (Java)", "Consul"],
          rows: [
            ["Runtime", "Single Rust binary, ~10MB memory", "Java process, ~512MB baseline", "Single Go binary"],
            ["Startup speed", "Seconds", "Tens of seconds to minutes", "Seconds"],
            ["API compatibility", "Fully Nacos-compatible", "Official Nacos", "Not Nacos-compatible"],
            ["Config management", "Built-in, supports version rollback", "Built-in, most complete features", "KV Store, no automatic rollback"],
            ["Service discovery", "Both HTTP and gRPC", "Both HTTP and gRPC", "HTTP + DNS"],
            ["Best for", "Local Nacos development and testing", "Production", "General-purpose service discovery"],
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Local development only",
          value:
            "The default console credentials are admin / admin, and OpenAPI authentication is disabled. Do not expose this instance to the public internet. Use an official Nacos cluster deployment for production.",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quickstart",
      navHint: "Config · Service registration",
      title: "Publish a config and verify it",
      intro:
        "Zhiyu has already installed and started rnacos for you. In the Connect & Debug tab you can see the addresses for the HTTP, gRPC, and Web Console endpoints, or open the Console directly in a browser.",
      blocks: [
        {
          kind: "list",
          items: [
            "In the Overview tab, confirm the status is Running.",
            "The Connect & Debug tab lists three endpoints — Nacos HTTP, gRPC, and Web Console — all copyable directly.",
            "Open http://127.0.0.1:10848/rnacos/ in a browser and sign in to the Web Console with admin / admin to manage configs and services visually.",
          ],
        },
        {
          kind: "text",
          value:
            "Nacos config management core concepts: namespace (environment isolation) → group → dataId (config item identifier). Local development typically uses the default public namespace and DEFAULT_GROUP.",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Publish and read config",
          code: `NACOS=http://127.0.0.1:${port}

# Publish config
curl -X POST "$NACOS/nacos/v1/cs/configs" \\
  -d "dataId=application.properties" \\
  -d "group=DEFAULT_GROUP" \\
  -d "content=server.port=8080
app.name=demo
app.timeout=30s"

# Read config
curl "$NACOS/nacos/v1/cs/configs?dataId=application.properties&group=DEFAULT_GROUP"

# Delete config
curl -X DELETE "$NACOS/nacos/v1/cs/configs" \\
  -d "dataId=application.properties" \\
  -d "group=DEFAULT_GROUP"

# Publish JSON-formatted config
curl -X POST "$NACOS/nacos/v1/cs/configs" \\
  -d "dataId=database.json" \\
  -d "group=DEFAULT_GROUP" \\
  -d 'content={"host":"127.0.0.1","port":3306,"db":"demo"}'`,
        },
        {
          kind: "text",
          value: "View a config's version history and roll back:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Config versions and rollback",
          code: `# View the version history of a config
curl "$NACOS/nacos/v1/cs/history?dataId=application.properties&group=DEFAULT_GROUP"

# View the contents of a specific historical version
# Grab the id from the history list above, then:
curl "$NACOS/nacos/v1/cs/history?nid=<history_id>"

# Roll back to the previous version
curl -X POST "$NACOS/nacos/v1/cs/configs" \\
  -d "dataId=application.properties" \\
  -d "group=DEFAULT_GROUP" \\
  -d 'content=new content'`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Nacos dataId naming convention",
          value:
            "In Spring Cloud Alibaba projects the dataId format is ${prefix}-${spring.profile.active}.${file-extension}, e.g. application-dev.properties, userservice-dev.yml. This is the naming that Spring Cloud Nacos Config uses to auto-load configs — don't just pick anything.",
        },
      ],
    },

    {
      id: "service-discovery",
      navLabel: "Service registration & discovery",
      navHint: "Register · Discover · Heartbeat",
      title: "Register a service and discover it",
      intro:
        "Beyond the config center, rnacos is also fully compatible with the Nacos service registration/discovery protocol. Nacos 1.x clients register over HTTP API, 2.x clients register over gRPC — rnacos supports both.",
      blocks: [
        {
          kind: "code",
          lang: "bash",
          caption: "Register and discover services via the HTTP API",
          code: `NACOS=http://127.0.0.1:${port}

# Register a service instance
curl -X POST "$NACOS/nacos/v1/ns/instance" \\
  -d "serviceName=user-service" \\
  -d "ip=127.0.0.1" \\
  -d "port=8080" \\
  -d "weight=1.0" \\
  -d "healthy=true" \\
  -d "metadata={\\"version\\":\\"v1\\"}"

# List all instances under a service
curl "$NACOS/nacos/v1/ns/instance/list?serviceName=user-service"

# List only healthy instances
curl "$NACOS/nacos/v1/ns/instance/list?serviceName=user-service&healthyOnly=true"

# Send heartbeat (1.x clients send one every 5 seconds)
curl -X PUT "$NACOS/nacos/v1/ns/instance/beat" \\
  -d "serviceName=user-service" \\
  -d "ip=127.0.0.1" \\
  -d 'port=8080' \\
  -d 'beat={"metadata":{}}'

# Deregister a service
curl -X DELETE "$NACOS/nacos/v1/ns/instance" \\
  -d "serviceName=user-service" \\
  -d "ip=127.0.0.1" \\
  -d "port=8080"`,
        },
        {
          kind: "text",
          value:
            "rnacos is also compatible with the Nacos 2.x gRPC protocol (port 9848), supporting long-lived connections for change pushes. Spring Cloud Alibaba 2022.x+ and Nacos Client 2.x use gRPC by default and integrate seamlessly.",
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Heartbeats matter",
          value:
            "1.x clients need to actively send heartbeats to maintain their registration state — by default one every 5 seconds. If Nacos doesn't receive a heartbeat within 15 seconds, the instance is marked unhealthy; after 30 seconds it is automatically evicted. 2.x gRPC uses a long-lived connection, and a disconnect is treated as the service going offline.",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "Language integration",
      navHint: "Spring · Go · TS · Python",
      title: "Connect from your project",
      intro: `rnacos is fully Nacos-compatible, so any language's Nacos client can connect to it directly. Below are usage examples in various languages — just point server-addr at 127.0.0.1:${port}.`,
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
        # Auto-loads dataId: demo-service.yaml
        refresh-enabled: true   # Auto-refresh @RefreshScope on config change`,
            },
            {
              label: "Java",
              lang: "java",
              caption: "Hot config reload with @NacosValue",
              code: `@RestController
@RefreshScope  // When the underlying Nacos Config data changes,
              // beans annotated with this will be reloaded
public class DemoController {

    @Value("\${app.timeout:30s}")
    private String timeout;

    @GetMapping("/config/timeout")
    public String timeout() {
        return timeout;
    }
}

// Service discovery
@Autowired
private NacosDiscoveryClient discoveryClient;

List<ServiceInstance> instances = discoveryClient.getInstances("user-service");
String url = instances.get(0).getUri().toString();`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "Install nacos-sdk-go",
              code: `go get github.com/nacos-group/nacos-sdk-go/v2`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "Config and discovery",
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

// Read config
func GetConfig(dataId, group string) (string, error) {
    return configClient.GetConfig(vo.ConfigParam{DataId: dataId, Group: group})
}

// Listen for config changes
func ListenConfig(dataId, group string, callback func(string)) error {
    return configClient.ListenConfig(vo.ConfigParam{
        DataId: dataId,
        Group:  group,
        OnChange: func(namespace, group, dataId, content string) {
            callback(content)
        },
    })
}

// Register a service
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
              caption: "Install nacos-node",
              code: `npm install nacos`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "Config and discovery",
              code: `import Nacos from "nacos";

const nacos = new Nacos.NacosClient({
  serverAddr: "127.0.0.1:${port}",
  namespace: "",
  logger: console,
});

// Read config
const content = await nacos.getConfig("application.properties", "DEFAULT_GROUP");
console.log(content);

// Publish config
await nacos.publishConfig("application.properties", "DEFAULT_GROUP",
  "server.port=8080\\napp.name=demo");

// Register a service
await nacos.registerInstance("user-service", {
  ip: "127.0.0.1",
  port: 8080,
  weight: 1,
  healthy: true,
});

// Query service instances
const instances = await nacos.getAllInstances("user-service");
instances.forEach(i => console.log(i.ip, i.port));`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "Install nacos-sdk-python",
              code: `pip install nacos-sdk-python`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "Config and discovery",
              code: `import nacos

SERVER_ADDRESSES = "127.0.0.1:${port}"

client = nacos.NacosClient(SERVER_ADDRESSES)

# Read config
content = client.get_config("application.properties", "DEFAULT_GROUP")
print(content)

# Publish config
client.publish_config("application.properties", "DEFAULT_GROUP",
    "server.port=8080\\napp.version=v2")

# Remove config
client.remove_config("application.properties", "DEFAULT_GROUP")

# Register a service
client.add_naming_instance("user-service", "127.0.0.1", 8080,
    weight=1.0, healthy=True)

# Query service instances
instances = client.list_naming_instance("user-service")
for inst in instances.get("hosts", []):
    print(inst["ip"], inst["port"])`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Don't include the protocol prefix in Spring Cloud Nacos server-addr",
          value:
            "Set spring.cloud.nacos.discovery.server-addr to 127.0.0.1:8848 directly — not http://127.0.0.1:8848. Including http:// makes the client fail to parse the address. Almost every newcomer trips over this at least once.",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "Pitfalls & tuning",
      navHint: "Troubleshooting · FAQ",
      title: "Things to know before you ship",
      intro: "rnacos is a great Nacos substitute during development, but there are a few things to watch out for.",
      blocks: [
        {
          kind: "table",
          head: ["Issue", "Symptom", "Fix"],
          rows: [
            [
              "server-addr format is wrong",
              "Spring Cloud Nacos connection errors",
              "Drop the http:// prefix and write ip:port directly",
            ],
            [
              "2.x client can't connect",
              "gRPC connection fails",
              "Confirm port 9848 is reachable; rnacos exposes both 1.x HTTP and 2.x gRPC",
            ],
            [
              "Default console password is insecure",
              "Anyone can sign in with admin/admin and modify configs",
              "Fine to leave for local dev; when exposing beyond localhost, change the password or disable the console binding",
            ],
            [
              "Config doesn't refresh",
              "The app doesn't see the change after editing config",
              "Confirm @RefreshScope is added; or that ListenConfig is enabled in client code",
            ],
            [
              "Service unexpectedly evicted",
              "A healthy service disappears from the registry",
              "Check that heartbeats are being sent (1.x sends one every 5 seconds by default); network jitter can trigger the 15-second timeout",
            ],
            [
              "Namespace misunderstanding",
              "Config written into namespace A, client queries namespace B and can't find it",
              "namespace is the top isolation level; make sure server and client use the same namespaceId",
            ],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "How to do it in Zhiyu",
          value:
            "The Overview tab shows service status and ports; the Connect & Debug tab has quick-copy addresses for the HTTP, gRPC, and Console endpoints; opening http://127.0.0.1:10848/rnacos/ lets you manage configs and services from the Web Console; the Logs tab shows API call records and startup errors; take a snapshot in Backup & Restore before risky operations. To change ports and auth parameters, edit them in the Config File tab and restart.",
        },
      ],
    },
  ];
}
