import type { DocChapter } from "../docTypes";

export function buildEtcdDocs(port: number): DocChapter[] {
  return [
    {
      id: "intro",
      navLabel: "Meet etcd",
      navHint: "Distributed KV · Raft",
      title: "What is etcd",
      intro:
        "etcd is a distributed, strongly consistent key-value store. It uses the Raft consensus algorithm to keep data consistent across nodes, and is a core dependency of Kubernetes.",
      blocks: [
        {
          kind: "text",
          value:
            "etcd started life as the storage engine the CoreOS team built for service coordination. Its simple KV interface delivers three key guarantees: strong consistency (any node always reads the latest data), the Watch mechanism (clients can subscribe to key changes and receive real-time notifications), and Lease (keys can be bound to a TTL-backed lease and are deleted automatically on expiry).",
        },
        {
          kind: "text",
          value: "etcd is best at things like:",
        },
        {
          kind: "list",
          items: [
            "Service registry and discovery — a service writes its address into etcd on startup, binds it to a lease, and renews periodically; when it crashes the lease expires and the key disappears automatically.",
            "Configuration center — applications pull config on startup and Watch for changes, so config updates propagate within seconds.",
            "Distributed locks — mutex locks built on leases and transactions, so multiple instances competing for the same resource are guaranteed serial access.",
            "Leader election — leveraging automatic lease expiry to implement simple leader-election logic.",
            "Metadata storage — storing cluster topology, node information, ops metadata, and other small but critical data.",
          ],
        },
        {
          kind: "table",
          head: ["Item", "Value", "Notes"],
          rows: [
            ["Client Endpoint", `http://127.0.0.1:${port}`, "Address used by apps and etcdctl"],
            ["Peer Endpoint", "http://127.0.0.1:2380", "Internal comm port for single node"],
            ["TLS / Auth", "Disabled", "Bound to localhost dev environment only"],
            ["Data directory", "~/.devbox/instances/etcd/default/data", "Persistent data files"],
          ],
        },
        {
          kind: "table",
          head: ["", "etcd", "Consul", "ZooKeeper"],
          rows: [
            ["Consensus protocol", "Raft", "Raft", "ZAB"],
            ["Data model", "Flat KV with prefix support", "Tree-like KV with prefix support", "Tree-like nodes (ZNode)"],
            ["Watch mechanism", "Supported, single key / prefix", "Supported, via blocking queries", "Supported, must re-register after each trigger"],
            ["Gateway", "etcd gateway proxy", "Consul Client Agent", "No built-in gateway"],
            ["Deployment complexity", "Medium, needs certificates", "Low, single binary is enough", "High, requires Java"],
            ["Typical scenarios", "K8s core, distributed coordination", "Service discovery + KV config", "Legacy coordination service"],
          ],
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Zhiyu runs in single-node mode",
          value:
            "In cluster mode etcd requires at least 3 nodes to guarantee high availability, but local development doesn't need that. Zhiyu launches a single node, so Raft election is skipped entirely and writes are never blocked — plenty to validate every etcd client call.",
        },
      ],
    },

    {
      id: "quickstart",
      navLabel: "Quick start",
      navHint: "Put · Get · Delete",
      title: "Read and write your first key-value pair",
      intro:
        "Zhiyu has already installed and launched etcd. The \"Connect & debug\" tab shows the Client Endpoint and Peer Endpoint together with a quick reference of common commands.",
      blocks: [
        {
          kind: "list",
          items: [
            "Check that the service is \"Running\" on the \"Overview\" tab.",
            "The \"Connect & debug\" tab lists etcdctl verification commands you can copy straight into a terminal.",
            "The etcdctl commands below run directly in the terminal and take effect immediately.",
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Set an alias for convenience",
          code: `export ETCD_ENDPOINTS=http://127.0.0.1:${port}
alias etcdctl='ETCDCTL_API=3 ~/.devbox/installations/etcd/3.6/bin/etcdctl --endpoints=$ETCD_ENDPOINTS'

# Verify connectivity
etcdctl version`,
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Basic read and write operations",
          code: `# Write
etcdctl put /app/config/timeout "30s"
etcdctl put /app/config/max_connections "100"

# Read
etcdctl get /app/config/timeout
# Output: /app/config/timeout
#         30s

# Read by prefix
etcdctl get /app/config --prefix
# Returns every key under /app/config

# List key names only
etcdctl get /app --prefix --keys-only

# Delete
etcdctl del /app/config/timeout

# Bulk delete by prefix
etcdctl del /app/config --prefix`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Keys can be any string, but slash-based hierarchy is recommended",
          value:
            "etcd doesn't enforce naming rules on keys, but the community convention is filesystem-style paths separated by / (for example /services/user-service/address). The upside is being able to operate on a batch of related keys by prefix (etcdctl get /services --prefix) with clear semantics.",
        },
      ],
    },

    {
      id: "watch-lease",
      navLabel: "Watch and Lease",
      navHint: "Change subscription · Auto expiry",
      title: "Making the most of Watch and Lease",
      intro:
        "etcd's two headline features go well beyond a plain KV store: Watch lets clients react to data changes in real time, and Lease gives keys automatic expiry.",
      blocks: [
        {
          kind: "text",
          value:
            "Watch is one of etcd's most distinctive features. Once a client sets up a Watch on a key or prefix, etcd proactively pushes events whenever data in that range changes. This is critical for configuration centers, service discovery, and similar scenarios: applications don't need to poll, and changes are picked up within seconds.",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Watch a key",
          code: `# Terminal 1: start Watch — it blocks until a change happens
etcdctl watch /app/config/timeout

# Terminal 2: write to the same key
etcdctl put /app/config/timeout "60s"

# Terminal 1 prints immediately:
#   PUT
#   /app/config/timeout
#   60s

# Prefix Watch
etcdctl watch /app --prefix

# Terminal 2:
etcdctl put /app/config/host "localhost"
# Terminal 1:
#   PUT
#   /app/config/host
#   localhost`,
        },
        {
          kind: "text",
          value:
            "A Lease attaches a TTL to keys. Once the lease expires, every key bound to it is deleted automatically. This is the foundation of service registry and discovery: a service instance writes its info into etcd bound to a short lease, then renews periodically. When the instance crashes the lease expires and the registration disappears on its own.",
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Lease usage",
          code: `# Grant a 30-second lease and get back a lease ID
etcdctl lease grant 30
# Output: lease 694d8c6a6e7e0001 granted with TTL(30s)

# Write a key bound to that lease
etcdctl put /services/gateway/instance-1 "http://127.0.0.1:8080" \\
  --lease=694d8c6a6e7e0001

# Inspect the remaining TTL for the key
etcdctl get /services/gateway/instance-1 -w json | grep lease

# Keep the lease alive
etcdctl lease keep-alive 694d8c6a6e7e0001

# Without renewal for 30 seconds, the key disappears
sleep 35
etcdctl get /services/gateway/instance-1
# Output: empty (key auto-deleted)

# List every lease
etcdctl lease list

# Revoke the lease (deletes every key bound to it immediately)
etcdctl lease revoke 694d8c6a6e7e0001`,
        },
        {
          kind: "callout",
          tone: "tip",
          title: "Service registry = Lease + keep-alive + Watch",
          value:
            "A complete service registration flow looks like this: on startup put the address bound to a 10-second lease → a background goroutine keeps-alive every 3 seconds → on shutdown revoke the lease → other services Watch the prefix and pick up the change. Almost every etcd client library ships this pattern out of the box.",
        },
      ],
    },

    {
      id: "clients",
      navLabel: "Language clients",
      navHint: "Java · Go · TS · Python",
      title: "Connect from your project",
      intro: `The snippets below all target http://127.0.0.1:${port} on your machine. Every etcd client speaks gRPC to etcd by default (v3 API), no extra configuration required.`,
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
              caption: "Read/write and Watch",
              code: `import io.etcd.jetcd.*;
import io.etcd.jetcd.kv.GetResponse;
import io.etcd.jetcd.watch.WatchEvent;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.ExecutionException;

Client client = Client.builder()
    .endpoints("http://127.0.0.1:${port}")
    .build();
KV kv = client.getKVClient();

// Write
kv.put(ByteSequence.from("hello", StandardCharsets.UTF_8),
       ByteSequence.from("zhiyu", StandardCharsets.UTF_8)).get();

// Read
GetResponse resp = kv.get(ByteSequence.from("hello", StandardCharsets.UTF_8)).get();
String value = resp.getKvs().get(0).getValue().toString(StandardCharsets.UTF_8);

// Read by prefix
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
              caption: "Install the etcd Go SDK",
              code: `go get go.etcd.io/etcd/client/v3`,
            },
            {
              label: "Go",
              lang: "go",
              caption: "Read/write and Watch",
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
              caption: "Install etcd3",
              code: `npm install etcd3`,
            },
            {
              label: "TypeScript",
              lang: "typescript",
              caption: "Read/write and Watch",
              code: `import { Etcd3 } from "etcd3";

const client = new Etcd3({ hosts: "http://127.0.0.1:${port}" });

// Write
await client.put("/app/config/timeout").value("30s");

// Read
const val = await client.get("/app/config/timeout").string();
console.log(val); // "30s"

// Read by prefix
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
setInterval(() => lease.keepaliveOnce(), 10_000); // renew every 10 seconds`,
            },
            {
              label: "Python",
              lang: "bash",
              caption: "Install etcd3-py",
              code: `pip install etcd3`,
            },
            {
              label: "Python",
              lang: "python",
              caption: "Read/write and Watch",
              code: `import etcd3

client = etcd3.client(host="127.0.0.1", port=${port})

# Write
client.put("/app/config/timeout", "30s")

# Read
value, _ = client.get("/app/config/timeout")
print(value.decode() if value else None)  # b"30s"

# Read by prefix
for value, metadata in client.get_prefix("/app/"):
    print(metadata.key.decode(), value.decode())

# Watch (blocks the current thread)
events_iter, cancel = client.watch_prefix("/app/")
for event in events_iter:
    print(f"{event.event_type} {event.key}")

# Lease
lease = client.lease(30)
client.put("/svc/instance-1", "http://localhost:8080", lease=lease)
lease.refresh()  # renew`,
            },
          ],
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Watch out for the differences between etcd v2 and v3 APIs",
          value:
            "The etcd v2 API is deprecated; v3 uses the gRPC protocol. Every modern etcd client library defaults to v3. If your project still pulls in legacy dependencies, make sure etcdctl has ETCDCTL_API=3 set — otherwise get and put may target different API versions and the data won't be visible across them.",
        },
      ],
    },

    {
      id: "pitfalls",
      navLabel: "Pitfalls and tuning",
      navHint: "Troubleshooting · Common issues",
      title: "What to know before going live",
      intro: "etcd runs smoothly in single-node dev mode, but production has a few gotchas to know up front.",
      blocks: [
        {
          kind: "table",
          head: ["Issue", "Symptom", "Fix"],
          rows: [
            [
              "Single node has no HA",
              "Every service that depends on etcd is affected when etcd goes down",
              "In production deploy at least a 3-node cluster",
            ],
            [
              "Data volume too large",
              "etcd isn't meant to hold large data — the default storage cap is 1.5 GiB",
              "Store only critical metadata and configuration; put large files in object storage",
            ],
            [
              "Frequent Watch reconnects",
              "Network jitter drops the Watch and forces reconnection",
              "Client libraries reconnect automatically; verify timeout and retry settings are sane",
            ],
            [
              "Lease not renewed, service falsely deregistered",
              "The service is still running but its registration entry has vanished",
              "Set the renewal interval to 1/3 of the TTL to leave plenty of headroom",
            ],
            [
              "v2 API deprecated",
              "Legacy tools default to v2, isolated from v3 data",
              "export ETCDCTL_API=3; audit every caller before upgrading production",
            ],
            [
              "Permissions not configured",
              "Local dev has no auth, but production requires it",
              "Enable RBAC in production and set the root user's password",
            ],
          ],
        },
        {
          kind: "code",
          lang: "bash",
          caption: "Troubleshooting commands",
          code: `alias etcdctl='ETCDCTL_API=3 ~/.devbox/installations/etcd/3.6/bin/etcdctl --endpoints=http://127.0.0.1:2379'

# Cluster status
etcdctl endpoint status
etcdctl endpoint health

# List every key (careful with large datasets)
etcdctl get "" --prefix --keys-only

# Inspect storage usage
etcdctl endpoint status -w table

# Compact historical revisions (reclaim space)
etcdctl compact $(etcdctl endpoint status -w json | grep -o '"revision":[0-9]*' | head -1 | cut -d: -f2)`,
        },
        {
          kind: "callout",
          tone: "warn",
          title: "Production must enable TLS and authentication",
          value:
            "Zhiyu disables TLS and auth locally for quick onboarding, but production must turn them on. An unguarded etcd exposes service registrations, configuration data, and distributed-lock keys. At a minimum: (1) enable TLS-encrypted transport; (2) set a root password and enable authentication; (3) use etcd auth commands to create limited-privilege sub-accounts.",
        },
        {
          kind: "callout",
          tone: "tip",
          title: "How to do it in Zhiyu",
          value:
            "The \"Overview\" tab shows metrics like version and storage size; the \"Connect & debug\" tab has a quick reference of etcdctl commands you can copy straight into a terminal; \"Runtime logs\" surfaces startup errors and Raft state; to change the port or data directory, edit the \"Config file\" tab and restart; before any risky operation, take a snapshot from \"Backup & restore\".",
        },
      ],
    },
  ];
}
