<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { diagnoseNetwork, getNetworkProxySettings } from "../../api/tools";
import type {
  NetworkDiagnosticResult,
  NetworkFinding,
  NetworkProxySetting,
} from "../../types";

type Mode = "auto" | "tcp" | "http" | "https";
type RecentTarget = { target: string; mode: Mode; port: number | null; time: number; healthy: boolean };

const { t } = useI18n();
const HISTORY_KEY = "zhiyu.network-diagnostics.history.v1";
const target = ref("http://127.0.0.1:3000");
const mode = ref<Mode>("auto");
const portInput = ref("");
const timeoutSeconds = ref(5);
const loading = ref(false);
const error = ref("");
const result = ref<NetworkDiagnosticResult | null>(null);
const proxies = ref<NetworkProxySetting[]>([]);
const history = ref<RecentTarget[]>([]);
const copied = ref(false);

const connectedCount = computed(() =>
  result.value?.tcpAttempts.filter((attempt) => attempt.connected).length ?? 0,
);
const primaryState = computed(() => {
  if (!result.value) return "idle";
  if (result.value.findings.some((finding) => finding.level === "error")) return "error";
  if (result.value.findings.some((finding) => finding.level === "warning")) return "warning";
  return "success";
});

async function run() {
  if (loading.value) return;
  loading.value = true;
  error.value = "";
  try {
    result.value = await diagnoseNetwork({
      target: target.value.trim(),
      mode: mode.value,
      port: portInput.value ? Number(portInput.value) : null,
      timeoutSeconds: timeoutSeconds.value,
    });
    const item: RecentTarget = {
      target: target.value.trim(),
      mode: mode.value,
      port: portInput.value ? Number(portInput.value) : null,
      time: Date.now(),
      healthy: primaryState.value === "success",
    };
    history.value = [
      item,
      ...history.value.filter((entry) =>
        `${entry.target}|${entry.mode}|${entry.port}` !== `${item.target}|${item.mode}|${item.port}`,
      ),
    ].slice(0, 8);
    localStorage.setItem(HISTORY_KEY, JSON.stringify(history.value));
  } catch (cause) {
    result.value = null;
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

function useExample(value: string, valueMode: Mode, valuePort = "") {
  target.value = value;
  mode.value = valueMode;
  portInput.value = valuePort;
}

function useHistory(item: RecentTarget) {
  target.value = item.target;
  mode.value = item.mode;
  portInput.value = item.port ? String(item.port) : "";
  void run();
}

function findingTitle(finding: NetworkFinding) {
  return t(`network.findings.${finding.code}.title`, { value: finding.detail });
}

function findingDetail(finding: NetworkFinding) {
  if (finding.code === "tls_failed") {
    return `${t(`network.findings.${finding.code}.detail`)} ${finding.detail}`;
  }
  return t(`network.findings.${finding.code}.detail`, { value: finding.detail });
}

function formatBytes(value: number | null) {
  if (value == null) return "—";
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`;
  return `${(value / 1024 / 1024).toFixed(1)} MiB`;
}

async function copyReport() {
  if (!result.value) return;
  const report = [
    `Target: ${result.value.target}`,
    `DNS: ${result.value.dnsMillis} ms → ${result.value.addresses.map((item) => item.address).join(", ")}`,
    ...result.value.tcpAttempts.map((item) =>
      `TCP ${item.address}: ${item.connected ? "OK" : "FAILED"} (${item.elapsedMillis} ms)${item.error ? ` ${item.error}` : ""}`,
    ),
    result.value.http ? `HTTP: ${result.value.http.statusCode} ${result.value.http.statusText} (${result.value.http.elapsedMillis} ms)` : "",
    result.value.tls ? `TLS: ${result.value.tls.success ? "OK" : "FAILED"} ${result.value.tls.protocol} ${result.value.tls.error}` : "",
  ].filter(Boolean).join("\n");
  await navigator.clipboard.writeText(report);
  copied.value = true;
  window.setTimeout(() => (copied.value = false), 1200);
}

onMounted(async () => {
  try {
    const stored = JSON.parse(localStorage.getItem(HISTORY_KEY) || "[]");
    history.value = Array.isArray(stored) ? stored.slice(0, 8) : [];
  } catch {
    history.value = [];
  }
  proxies.value = await getNetworkProxySettings();
});
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo network">◎</span>
      <div>
        <div class="title-line"><h1>{{ t("network.title") }}</h1><span>NETWORK LAB</span></div>
        <p>{{ t("network.subtitle") }}</p>
      </div>
    </div>
    <div class="header-actions">
      <button type="button" :disabled="!result" @click="copyReport">{{ copied ? t("common.copied") : t("network.copyReport") }}</button>
      <button class="primary" type="button" :disabled="loading" @click="run">
        <span v-if="loading" class="spinner"></span>{{ loading ? t("network.diagnosing") : t("network.diagnose") }}
      </button>
    </div>
  </header>
  <div v-if="error" class="notice danger"><span>{{ error }}</span><button @click="error=''">×</button></div>

  <main class="network-page">
    <section class="diagnostic-input">
      <div><small>TARGET</small><h2>{{ t("network.target") }}</h2></div>
      <label class="target-field">{{ t("network.address") }}<input v-model="target" placeholder="https://example.com/health" @keyup.enter="run" /></label>
      <label>{{ t("network.mode") }}<select v-model="mode"><option value="auto">{{ t("network.modes.auto") }}</option><option value="tcp">TCP</option><option value="http">HTTP</option><option value="https">HTTPS</option></select></label>
      <label>{{ t("network.port") }}<input v-model="portInput" type="number" min="1" max="65535" :placeholder="t('network.auto')" /></label>
      <label>{{ t("network.timeout") }}<select v-model.number="timeoutSeconds"><option :value="3">3s</option><option :value="5">5s</option><option :value="10">10s</option><option :value="15">15s</option></select></label>
    </section>

    <section class="quick-targets">
      <span>{{ t("network.quick") }}</span>
      <button @click="useExample('127.0.0.1','tcp','6379')">Redis · 6379</button>
      <button @click="useExample('localhost','tcp','3306')">MySQL · 3306</button>
      <button @click="useExample('http://localhost:8082','auto')">Caddy · HTTP</button>
      <button @click="useExample('https://example.com','auto')">HTTPS</button>
    </section>

    <template v-if="result">
      <section class="network-metrics">
        <article><small>DNS</small><strong>{{ result.dnsMillis }} ms</strong><span>{{ result.addresses.length }} {{ t("network.addressCount") }}</span></article>
        <article :class="{failed:!connectedCount}"><small>TCP</small><strong>{{ connectedCount }}/{{ result.tcpAttempts.length }}</strong><span>{{ connectedCount ? t("network.connected") : t("network.unreachable") }}</span></article>
        <article :class="{failed:result.http && result.http.statusCode>=400}"><small>HTTP</small><strong>{{ result.http ? result.http.statusCode : "—" }}</strong><span>{{ result.http ? `${result.http.elapsedMillis} ms` : t("network.notChecked") }}</span></article>
        <article :class="{failed:result.tls && !result.tls.success}"><small>TLS</small><strong>{{ result.tls ? (result.tls.success ? result.tls.protocol.replace("TLSv","TLS ") : t("network.failed")) : "—" }}</strong><span>{{ result.tls ? `${result.tls.elapsedMillis} ms` : t("network.notChecked") }}</span></article>
        <article :class="primaryState"><small>{{ t("network.result") }}</small><strong>{{ t(`network.states.${primaryState}`) }}</strong><span>{{ result.host }}:{{ result.port }}</span></article>
      </section>

      <section class="result-grid">
        <article class="findings-panel">
          <header><div><small>DIAGNOSIS</small><h2>{{ t("network.findingsTitle") }}</h2></div><span>{{ result.findings.length }}</span></header>
          <div class="finding-list">
            <section v-for="finding in result.findings" :key="finding.code" :class="finding.level">
              <i></i><div><strong>{{ findingTitle(finding) }}</strong><p>{{ findingDetail(finding) }}</p></div>
            </section>
          </div>
        </article>

        <article class="steps-panel">
          <header><div><small>NETWORK PATH</small><h2>{{ t("network.path") }}</h2></div></header>
          <div class="step dns-step"><b>1</b><div><strong>{{ t("network.dnsResolution") }}</strong><p>{{ result.addresses.map(item => `${item.address} · ${item.family}`).join("  /  ") }}</p></div><span>{{ result.dnsMillis }} ms</span></div>
          <div v-for="attempt in result.tcpAttempts" :key="attempt.address" class="step" :class="{failed:!attempt.connected}"><b>2</b><div><strong>TCP · {{ attempt.address }}</strong><p>{{ attempt.connected ? t("network.handshakeOk") : attempt.error }}</p></div><span>{{ attempt.elapsedMillis }} ms</span></div>
          <div v-if="result.tls" class="step" :class="{failed:!result.tls.success}"><b>3</b><div><strong>TLS · {{ result.tls.protocol || t("network.failed") }}</strong><p>{{ result.tls.success ? `${result.tls.cipherSuite} · ${result.tls.alpn || "HTTP/1.1"}` : result.tls.error }}</p></div><span>{{ result.tls.elapsedMillis }} ms</span></div>
          <div v-if="result.http" class="step" :class="{failed:result.http.statusCode>=400}"><b>{{ result.tls ? 4 : 3 }}</b><div><strong>HTTP · {{ result.http.statusCode }} {{ result.http.statusText }}</strong><p>{{ result.http.effectiveUrl }}</p></div><span>{{ result.http.elapsedMillis }} ms</span></div>
        </article>
      </section>

      <section class="detail-grid">
        <article>
          <header><small>HTTP</small><h3>{{ t("network.httpDetails") }}</h3></header>
          <dl v-if="result.http"><div><dt>Server</dt><dd>{{ result.http.server || "—" }}</dd></div><div><dt>Content-Type</dt><dd>{{ result.http.contentType || "—" }}</dd></div><div><dt>Content-Length</dt><dd>{{ formatBytes(result.http.contentLength) }}</dd></div><div><dt>URL</dt><dd>{{ result.http.effectiveUrl }}</dd></div></dl>
          <p v-else>{{ t("network.httpEmpty") }}</p>
        </article>
        <article>
          <header><small>TLS</small><h3>{{ t("network.tlsDetails") }}</h3></header>
          <dl v-if="result.tls"><div><dt>Protocol</dt><dd>{{ result.tls.protocol || "—" }}</dd></div><div><dt>Cipher</dt><dd>{{ result.tls.cipherSuite || "—" }}</dd></div><div><dt>ALPN</dt><dd>{{ result.tls.alpn || "—" }}</dd></div><div><dt>SHA-256</dt><dd class="fingerprint">{{ result.tls.sha256Fingerprint || "—" }}</dd></div></dl>
          <p v-else>{{ t("network.tlsEmpty") }}</p>
        </article>
        <article>
          <header><small>LOCAL</small><h3>{{ t("network.localProcess") }}</h3></header>
          <dl v-if="result.portOwner"><div><dt>Process</dt><dd>{{ result.portOwner.process }}</dd></div><div><dt>PID</dt><dd>{{ result.portOwner.pid }}</dd></div><div><dt>Listen</dt><dd>{{ result.portOwner.address }}:{{ result.portOwner.port }}</dd></div><div><dt>Owner</dt><dd>{{ result.portOwner.managedService || t("network.externalProcess") }}</dd></div></dl>
          <p v-else>{{ t("network.noLocalProcess") }}</p>
        </article>
        <article>
          <header><small>PROXY</small><h3>{{ t("network.proxy") }}</h3></header>
          <dl v-if="result.proxies.length"><div v-for="item in result.proxies" :key="`${item.name}-${item.value}`"><dt>{{ item.name }}</dt><dd>{{ item.value }}</dd></div></dl>
          <p v-else>{{ t("network.noProxy") }}</p>
        </article>
      </section>
    </template>

    <section v-else class="network-empty">
      <div><span>◎</span><strong>{{ t("network.emptyTitle") }}</strong><p>{{ t("network.emptyDescription") }}</p></div>
      <aside v-if="history.length"><small>RECENT</small><button v-for="item in history" :key="`${item.target}-${item.mode}-${item.port}`" @click="useHistory(item)"><i :class="{ok:item.healthy}"></i><span>{{ item.target }}</span><code>{{ item.port || item.mode.toUpperCase() }}</code></button></aside>
    </section>
  </main>
</template>

<style scoped>
.service-logo.network{background:#345f68}.network-page{display:grid;gap:11px;padding:22px 30px 38px}.diagnostic-input{display:grid;grid-template-columns:120px minmax(260px,1fr) 120px 100px 90px;align-items:end;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.diagnostic-input>div{align-self:stretch;padding:12px 14px;border-right:1px solid var(--color-border);background:var(--color-bg-muted)}.diagnostic-input small,.findings-panel header small,.steps-panel header small,.detail-grid header small{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace;letter-spacing:.12em}.diagnostic-input h2,.result-grid h2{margin:5px 0 0;font-size:12px}.diagnostic-input label{display:grid;gap:5px;padding:9px 6px;color:var(--color-text-muted);font-size:7px}.diagnostic-input input,.diagnostic-input select{box-sizing:border-box;width:100%;height:31px;padding:0 8px;font-size:8px}.quick-targets{display:flex;align-items:center;gap:6px;min-height:34px;padding:0 9px;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.quick-targets>span{margin-right:3px;color:var(--color-text-muted);font-size:7px}.quick-targets button{height:24px;padding:0 8px;border-color:var(--color-border);background:var(--color-bg-muted);font-size:7px}.network-metrics{display:grid;grid-template-columns:repeat(5,1fr);border:1px solid var(--color-border);background:var(--color-panel-translucent)}.network-metrics article{display:grid;min-height:72px;align-content:center;gap:4px;padding:8px 13px;border-right:1px solid var(--color-border);box-shadow:inset 0 2px var(--color-success-text)}.network-metrics article:last-child{border-right:0}.network-metrics article.failed{box-shadow:inset 0 2px var(--color-danger-text)}.network-metrics article.warning{box-shadow:inset 0 2px var(--color-warning-text)}.network-metrics small{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace}.network-metrics strong{font-size:15px}.network-metrics span{overflow:hidden;color:var(--color-text-muted);font-size:7px;text-overflow:ellipsis;white-space:nowrap}.result-grid{display:grid;grid-template-columns:minmax(300px,.72fr) minmax(460px,1.28fr);gap:11px}.result-grid>article,.detail-grid>article{min-width:0;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.result-grid>article>header{display:flex;min-height:50px;align-items:center;justify-content:space-between;padding:7px 12px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.result-grid>article>header>span{color:var(--color-text-muted);font:8px "SFMono-Regular",monospace}.finding-list section{display:grid;grid-template-columns:12px minmax(0,1fr);gap:8px;padding:11px 12px;border-bottom:1px solid var(--color-border)}.finding-list i{width:7px;height:7px;margin-top:3px;border-radius:50%;background:var(--color-success-text)}.finding-list .warning i{background:var(--color-warning-text)}.finding-list .error i{background:var(--color-danger-text)}.finding-list strong{font-size:8px}.finding-list p{margin:4px 0 0;color:var(--color-text-muted);font-size:7px;line-height:1.5}.step{display:grid;grid-template-columns:25px minmax(0,1fr) auto;align-items:center;gap:9px;min-height:48px;padding:5px 12px;border-bottom:1px solid var(--color-border)}.step>b{display:grid;width:22px;height:22px;place-items:center;border:1px solid var(--color-success-text);border-radius:50%;color:var(--color-success-text);font:7px "SFMono-Regular",monospace}.step.failed>b{border-color:var(--color-danger-text);color:var(--color-danger-text)}.step>div{min-width:0}.step strong{font-size:8px}.step p{overflow:hidden;margin:3px 0 0;color:var(--color-text-muted);font-size:7px;text-overflow:ellipsis;white-space:nowrap}.step>span{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace}.detail-grid{display:grid;grid-template-columns:repeat(4,1fr);gap:11px}.detail-grid header{padding:10px 11px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.detail-grid h3{margin:4px 0 0;font-size:9px}.detail-grid dl{margin:0}.detail-grid dl>div{display:grid;grid-template-columns:72px minmax(0,1fr);gap:7px;padding:7px 10px;border-bottom:1px solid var(--color-border);font-size:7px}.detail-grid dt{color:var(--color-text-muted)}.detail-grid dd{overflow:hidden;margin:0;text-overflow:ellipsis;white-space:nowrap}.detail-grid .fingerprint{font:6px "SFMono-Regular",monospace}.detail-grid>article>p{min-height:70px;margin:0;padding:12px;color:var(--color-text-muted);font-size:7px;line-height:1.55}.network-empty{display:grid;grid-template-columns:minmax(0,1fr) 300px;min-height:420px;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.network-empty>div{display:grid;place-items:center;align-content:center;gap:8px;color:var(--color-text-muted);text-align:center}.network-empty>div>span{display:grid;width:50px;height:50px;place-items:center;border:1px solid var(--color-border);border-radius:50%;color:var(--color-accent);font-size:18px}.network-empty strong{color:var(--color-text-primary);font-size:10px}.network-empty p{max-width:420px;margin:0;font-size:8px}.network-empty aside{padding:14px;border-left:1px solid var(--color-border);background:var(--color-bg-muted)}.network-empty aside>small{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace}.network-empty aside button{display:grid;width:100%;grid-template-columns:8px minmax(0,1fr) auto;align-items:center;gap:7px;margin-top:7px;padding:8px;border:1px solid var(--color-border);background:var(--color-bg-panel);color:var(--color-text-primary);text-align:left}.network-empty aside i{width:6px;height:6px;border-radius:50%;background:var(--color-warning-text)}.network-empty aside i.ok{background:var(--color-success-text)}.network-empty aside span{overflow:hidden;font-size:7px;text-overflow:ellipsis;white-space:nowrap}.network-empty aside code{color:var(--color-text-muted);font-size:6px}@media(max-width:1100px){.diagnostic-input{grid-template-columns:110px minmax(220px,1fr) 100px 90px 80px}.detail-grid{grid-template-columns:repeat(2,1fr)}.result-grid{grid-template-columns:1fr}.network-metrics{grid-template-columns:repeat(5,minmax(120px,1fr));overflow:auto}}
</style>
