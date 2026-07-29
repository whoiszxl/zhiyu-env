import type { DocChapter } from "../docTypes";

export function buildConsulDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "What is Consul",
      navHint: "Service Discovery · KV",
      title: "What is Consul",
      intro:
        "Consul is a service networking platform developed by HashiCorp. It bundles service registration, health checks, KV configuration, and DNS service discovery into a single binary, making it one of the most mature open-source solutions for microservice architectures.",
      blocks: [
        {
          kind: "text",
          value:
            "Consul's design is pragmatic at its core: a Server Agent process that exposes all functionality through an HTTP API. Service instances register their address and health check endpoint through the Agent, and Consul automatically maintains a live list of which services are healthy. Other services can query this list via the HTTP API or DNS — no need to maintain a registry yourself.",
        },
        {
          kind: "text",
          value: "Consul provides all of the following in one system:",
        },
        {
          kind: "list",
          items: [
            "Service registration: services register their address, port, and health check endpoint via the API on startup; Consul continuously probes them.",
            "Health checks: supports HTTP, TCP, gRPC, and script checks; automatically removes unhealthy instances.",
            "KV Store: a key-value store similar to etcd, usable for dynamic configuration.",
            "DNS service discovery: use dig or nslookup with the service name to get a list of healthy instance IPs.",
            "Access Control (ACL): controls who can read, write, and register services.",
          ],
        },
        {
          kind: "table",
          head: ["Item", "Value", "Description"],
          rows: [
            ["HTTP API", `http://127.0.0.1:${port}`, "SDK & REST API"],
            ["Web UI", `http://127.0.0.1:${port}/ui/`, "Service and KV management UI"],
            ["DNS", "127.0.0.1:8600", "DNS service discovery"],
            ["Data Directory", "~/.devbox/instances/consul/default/data", "Persistent data"],
          ],
        },
        {
          kind: "table",
          head: ["", "Consul", "etcd", "Nacos"],
          rows: [
            ["Service Discovery", "Built-in + DNS", "Via Lease + Watch", "Built-in + gRPC"],
            ["Health Checks", "Built-in with multiple checkers", "Via Lease timeout", "Nacos Client heartbeat"],
            ["KV Config", "Built-in, supports Watch", "Built-in, supports Watch", "Built-in, supports Watch"],
            ["Web UI", "Built-in, feature-complete", "None (needs separate tools)", "Built-in console"],
            ["Multi-Datacenter", "Built-in native support", "Not supported", "Not supported"],
            ["Deployment", "Go single binary", "Go single binary", "Java / Rust multiple versions"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Zhiyu starts a single-node Consul Server in dev mode",
          value:
            "In production, Consul typically runs as a cluster (3-5 Servers plus Client Agents), but local development doesn't need that. Zhiyu starts a single Server in -dev mode; data is not persisted to disk (lost on restart), making it suitable for quickly validating your code logic.",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quick Start",
      navHint: "Register · Query · KV",
      title: "Register a Service and Query It Back",
      intro:
        "Zhiyu has already installed Consul and started it. The \"Connect & Debug\" tab shows the HTTP API address and DNS port. Open the Web UI in your browser to visually manage all services and KV entries.",
      blocks: [
        {
          kind: "list",
          items: [
            "On the \"Overview\" tab, confirm the service is \"Running\".",
            "The \"Connect & Debug\" tab has the HTTP API and DNS addresses, plus a `consul members` quick verification command.",
            `Open http://127.0.0.1:${port}/ui/ in your browser — this is Consul's built-in Web UI for registering services and managing KV directly.`,
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Service Registration",
          code: `CONSUL=http://127.0.0.1:${port}

# Register a service instance
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

# Register a second instance (simulating multi-instance deployment)
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
          caption: "Service Query",
          code: `# HTTP API: list all service instances
curl "$CONSUL/v1/agent/services"

# Query only instances of a specific service
curl "$CONSUL/v1/agent/service/user-service"

# Health check filter (return only healthy instances)
curl "$CONSUL/v1/health/service/user-service?passing"

# DNS query (port 8600)
dig @127.0.0.1 -p 8600 user-service.service.consul
# Returns two A records 127.0.0.1
# Add SRV query to see ports
dig @127.0.0.1 -p 8600 user-service.service.consul SRV

# Deregister a service
curl -X PUT "$CONSUL/v1/agent/service/deregister/user-service-1"`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "DeregisterCriticalServiceAfter is very useful",
          value:
            "After a health check has been failing for longer than this duration, Consul automatically deregisters the instance. This saves you from manually cleaning up registration records for dead services. Combined with DNS queries, clients will never receive addresses of downed services.",
        },
      ],
    },

    {
      id: "kv",
      navLabel: "KV Config Center",
      navHint: "Dynamic Config · Watch",
      title: "Dynamic Configuration with KV Store",
      intro:
        "Consul's KV Store offers functionality similar to etcd: writing key-value pairs, prefix queries, and Watch for changes. Many projects use Consul for both service discovery and configuration management, solving two problems with one system.",
      blocks: [
        {
          kind: "code",
          lang: "bash",
          caption: "KV Basic Operations",
          code: `CONSUL=http://127.0.0.1:${port}

# Write
curl -X PUT "$CONSUL/v1/kv/app/config/timeout" -d "30s"
curl -X PUT "$CONSUL/v1/kv/app/config/max_connections" -d "100"

# Read (returns JSON, value is base64-encoded)
curl "$CONSUL/v1/kv/app/config/timeout?raw"
# With the ?raw parameter, directly returns the raw value — no base64 decode needed

# Read by prefix (recursive)
curl "$CONSUL/v1/kv/app/config?recurse"

# List key names by prefix
curl "$CONSUL/v1/kv/app/config?keys"

# Delete
curl -X DELETE "$CONSUL/v1/kv/app/config/timeout"

# Recursive delete by prefix
curl -X DELETE "$CONSUL/v1/kv/app/config?recurse"`,
        },
        {
          kind: "text",
          value:
            "Watch is the core capability of a config center: the application pulls configuration on startup and establishes a Watch connection. When an operator modifies configuration via the Web UI or API, the application is notified immediately and reloads:",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Watch for Changes (Blocking Query)",
          code: `# Consul Watch is implemented via "blocking queries":
# Each request includes ?index=<last index value>, and the server
# hangs the connection until a change occurs.

# Get the current index
CONSUL_INDEX=$(curl -s "$CONSUL/v1/kv/app/config?recurse" | \\
  python3 -c "import sys,json; print(max(i['ModifyIndex'] for i in json.load(sys.stdin)))" 2>/dev/null || echo 0)

# Block and wait for changes (up to 60 seconds)
curl "$CONSUL/v1/kv/app/config?recurse&index=$CONSUL_INDEX&wait=60s"

# This command will hang until a change occurs and new data is returned.
# This is the HTTP-based Watch mechanism; client libraries typically wrap this logic.`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Web UI makes config changes easy",
          value:
            `Open http://127.0.0.1:${port}/ui/, go to Key/Value in the left menu, and you can create, edit, and delete keys directly in the UI. After modification, the Watch side will pick up the change immediately. This is very useful for debugging config center logic during development.`,
        },
      ],
    },

    {
      id: "clients",
      navLabel: "Language SDKs",
      navHint: "Java · Go · TS · Python",
      title: "Connect from Your Project",
      intro: `The following configurations all point to your local http://127.0.0.1:${port}. Libraries in every language are essentially wrappers around the HTTP API, so the API usage is highly consistent.`,
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
              caption: "Spring Cloud Consul auto-registration",
              code: `// Spring Cloud Consul automatically:
// 1. Registers the service with Consul
// 2. Reports health checks
// 3. Loads configuration from Consul KV
// 4. Sends periodic heartbeat renewals
// Only the above yml config + @EnableDiscoveryClient are needed
@SpringBootApplication
@EnableDiscoveryClient
public class UserServiceApplication {
    public static void main(String[] args) {
        SpringApplication.run(UserServiceApplication.class, args);
    }
}

// Use DiscoveryClient in code to discover other services
@Autowired
private DiscoveryClient discoveryClient;

List<ServiceInstance> instances = discoveryClient.getInstances("user-service");
String url = "http://" + instances.get(0).getHost() + ":" + instances.get(0).getPort();`,
            },
            {
              label: "Go",
              lang: "bash",
              caption: "Install consul-api",
              code: `go get github.com/hashicorp/consul/api`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "Service Registration & Discovery",
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

// KV operations
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
              caption: "Install consul",
              code: `npm install consul`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "Service Registration & Discovery",
              code: `import Consul from "consul";

const consul = new Consul({ host: "127.0.0.1", port: "${port}" });

// Register service
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

// Discover service
const services = await consul.health.service({ service: "user-service", passing: true });
services.forEach(s => {
  console.log(s.Service.Address, s.Service.Port);
});

// KV operations
await consul.kv.set("app/config/timeout", "30s");
const val = await consul.kv.get("app/config/timeout");
console.log(val?.Value); // "30s"`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "Install python-consul",
              code: `pip install python-consul`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "Service Registration & Discovery",
              code: `import consul

c = consul.Consul(host="127.0.0.1", port=${port})

# Register service
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

# Discover service (healthy only)
_, nodes = c.health.service("user-service", passing=True)
for node in nodes:
    print(node["Service"]["Address"], node["Service"]["Port"])

# Deregister
c.agent.service.deregister("user-service-1")

# KV operations
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
      navLabel: "Pitfalls & Tuning",
      navHint: "Troubleshooting · FAQ",
      title: "What to Know Before Going Live",
      intro: "Consul is smooth in local development, but production deployment requires attention to a few key configurations.",
      blocks: [
        {
          kind: "table",
          head: ["Issue", "Symptom", "Remedy"],
          rows: [
            [
              "Dev mode does not persist data",
              "All service registrations are lost after Consul restart",
              "Do not use -dev mode in production; configure data_dir to enable persistence",
            ],
            [
              "Health check unreachable",
              "All services are marked unhealthy",
              "Ensure the health check address is accessible by Consul (bind to 0.0.0.0 instead of 127.0.0.1 when needed)",
            ],
            [
              "DNS query returns nothing",
              "dig returns empty results for the service name",
              "Consul DNS defaults to port 8600; confirm you are querying the .service.consul suffix",
            ],
            [
              "Agent memory too high",
              "Consul Agent memory keeps growing",
              "Limit catalog cache size; periodically clean up stale KV entries",
            ],
            [
              "Cluster split-brain",
              "Inconsistency between nodes",
              "Deploy at least 3 Servers in production; ensure gossip protocol port (8301) is reachable",
            ],
            [
              "ACL not enabled",
              "Anyone can register and deregister services",
              "Enable ACL Tokens in production to restrict write operations",
            ],
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Troubleshooting Commands",
          code: `CONSUL=http://127.0.0.1:${port}

# List all currently registered services
curl "$CONSUL/v1/agent/services"

# View cluster members
curl "$CONSUL/v1/agent/members"

# View node health check status
curl "$CONSUL/v1/agent/checks"

# View all KV entries
curl "$CONSUL/v1/kv/?keys"
curl "$CONSUL/v1/kv/?recurse"

# Force-trigger a health check
curl -X PUT "$CONSUL/v1/agent/check/pass/service:user-service-1"`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "How to do it in Zhiyu",
          value:
            `The "Overview" tab shows service status and ports; the "Connect & Debug" tab has HTTP API and DNS address quick references; open http://127.0.0.1:${port}/ui/ to see all registered services, health check status, and the KV tree; "Logs" shows startup errors and API call logs; to change the port or data directory, edit the config in the "Config" tab and restart; before making dangerous operations, remember to take a snapshot in "Backup & Restore".`,
        },
      ],
    },
  ];
}