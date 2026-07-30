<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import {
  applyLocalDomains,
  checkLocalDomainTarget,
  getLocalDomains,
  restoreLocalDomains,
  saveLocalDomains,
} from "../../api/tools";
import { listServices, runServiceAction } from "../../api/services";
import type { LocalDomainRoute, LocalDomainsState, ServiceInfo } from "../../types";

const LEGACY_KEY = "zhiyu.local-domains.v1";
const state = ref<LocalDomainsState>({
  version: 2,
  httpPort: 8082,
  httpsPort: 8443,
  routes: [],
  lastBackupPath: "",
  lastAppliedAtMillis: 0,
});
const caddy = ref<ServiceInfo | null>(null);
const applying = ref(false);
const saving = ref(false);
const notice = ref("");
const error = ref("");
const checks = ref<Record<string, { checking: boolean; reachable?: boolean; message?: string }>>({});
const draft = ref({
  name: "",
  hostname: "",
  target: "127.0.0.1:3000",
  path: "/",
  https: false,
});
const enabledCount = computed(() => state.value.routes.filter((route) => route.enabled).length);
const hasBackup = computed(() => Boolean(state.value.lastBackupPath));
let saveTimer = 0;
let hydrated = false;

function normalizeHostname(value: string) {
  let hostname = value.trim().toLowerCase().replace(/^https?:\/\//, "").split("/")[0];
  hostname = hostname.replace(/:\d+$/, "").replace(/[^a-z0-9.-]/g, "-");
  if (hostname && !hostname.endsWith(".localhost")) hostname += ".localhost";
  return hostname;
}

function normalizePath(value: string) {
  const path = `/${value.trim()}`.replace(/\/+/g, "/").replace(/\/$/, "");
  return path || "/";
}

function addRoute() {
  const hostname = normalizeHostname(draft.value.hostname || draft.value.name);
  const path = normalizePath(draft.value.path);
  const target = draft.value.target.trim();
  if (!hostname) {
    error.value = "请输入本地域名";
    return;
  }
  if (!/^(127\.0\.0\.1|localhost|\[::1\]):\d{1,5}$/.test(target)) {
    error.value = "目标必须是本机地址，例如 127.0.0.1:3000";
    return;
  }
  if (state.value.routes.some((route) =>
    route.hostname === hostname && route.https === draft.value.https && normalizePath(route.path) === path
  )) {
    error.value = "相同协议、域名和路径的路由已经存在";
    return;
  }
  state.value.routes.push({
    id: crypto.randomUUID(),
    name: draft.value.name.trim() || hostname,
    hostname,
    target,
    path,
    https: draft.value.https,
    enabled: true,
  });
  draft.value = { name: "", hostname: "", target: "127.0.0.1:3000", path: "/", https: false };
  error.value = "";
}

function routeUrl(route: LocalDomainRoute) {
  const port = route.https ? state.value.httpsPort : state.value.httpPort;
  return `${route.https ? "https" : "http"}://${route.hostname}:${port}${normalizePath(route.path)}`;
}

async function refreshCaddy() {
  caddy.value = (await listServices()).find((service) => service.kind === "caddy") ?? null;
}

async function persist() {
  if (!hydrated || saving.value) return;
  saving.value = true;
  try {
    await saveLocalDomains(state.value);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    saving.value = false;
  }
}

async function applyRoutes() {
  if (applying.value) return;
  if (!caddy.value || caddy.value.status === "not_installed") {
    error.value = "请先安装 Caddy，再应用本地域名配置";
    return;
  }
  applying.value = true;
  try {
    state.value = await applyLocalDomains(state.value);
    try {
      await runServiceAction(caddy.value.status === "running" ? "restart" : "start", "caddy");
    } catch (startError) {
      if (state.value.lastBackupPath) {
        state.value = await restoreLocalDomains();
        if (caddy.value.status === "running") await runServiceAction("restart", "caddy");
      }
      throw new Error(`Caddy 启动失败，已自动恢复原配置：${String(startError)}`);
    }
    await refreshCaddy();
    notice.value = "配置已校验并应用，Caddy 网关已就绪";
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    applying.value = false;
  }
}

async function restoreConfig() {
  if (!confirm("恢复应用本地域名前的 Caddy 配置？")) return;
  applying.value = true;
  try {
    state.value = await restoreLocalDomains();
    if (caddy.value?.status === "running") await runServiceAction("restart", "caddy");
    notice.value = "原始 Caddy 配置已恢复";
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    applying.value = false;
  }
}

async function checkTarget(route: LocalDomainRoute) {
  checks.value[route.id] = { checking: true };
  try {
    const result = await checkLocalDomainTarget(route.target);
    checks.value[route.id] = {
      checking: false,
      reachable: result.reachable,
      message: `${result.message}${result.reachable ? ` · ${result.latencyMillis} ms` : ""}`,
    };
  } catch (cause) {
    checks.value[route.id] = { checking: false, reachable: false, message: String(cause) };
  }
}

function openRoute(route: LocalDomainRoute) {
  window.open(routeUrl(route), "_blank", "noopener,noreferrer");
}

watch(state, () => {
  if (!hydrated) return;
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => void persist(), 450);
}, { deep: true });

onMounted(async () => {
  try {
    state.value = await getLocalDomains();
    if (!state.value.routes.length) {
      const legacy = JSON.parse(localStorage.getItem(LEGACY_KEY) || "[]");
      if (Array.isArray(legacy) && legacy.length) {
        state.value.routes = legacy.map((route) => ({ ...route, path: "/" }));
        state.value = await saveLocalDomains(state.value);
        localStorage.removeItem(LEGACY_KEY);
      }
    }
  } catch (cause) {
    error.value = String(cause);
  } finally {
    hydrated = true;
  }
  await refreshCaddy();
});
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo domain-logo">⌁</span>
      <div>
        <div class="title-line"><h1>本地域名与 HTTPS</h1><span>LOCAL GATEWAY 2.0</span></div>
        <p>用 .localhost 域名和路径访问本地服务，配置由 Caddy 校验、备份并安全应用</p>
      </div>
    </div>
    <div class="header-actions">
      <span class="gateway-state" :class="caddy?.status"><i></i>{{ caddy?.status === "running" ? "网关运行中" : "网关未运行" }}</span>
      <button v-if="hasBackup" type="button" :disabled="applying" @click="restoreConfig">恢复配置</button>
      <button class="primary" type="button" :disabled="applying || !state.routes.length" @click="applyRoutes">
        <span v-if="applying" class="spinner"></span>{{ applying ? "校验并应用…" : "应用路由" }}
      </button>
    </div>
  </header>
  <div v-if="notice" class="notice"><span>{{ notice }}</span><button @click="notice=''">×</button></div>
  <div v-if="error" class="notice danger"><span>{{ error }}</span><button @click="error=''">×</button></div>

  <main class="domain-page">
    <section class="gateway-summary">
      <div><small>HTTP ENTRY</small><label><span>:</span><input v-model.number="state.httpPort" type="number" min="1" max="65535" /></label></div>
      <div><small>HTTPS ENTRY</small><label><span>:</span><input v-model.number="state.httpsPort" type="number" min="1" max="65535" /></label></div>
      <div><small>ACTIVE ROUTES</small><strong>{{ enabledCount }}</strong></div>
      <div><small>LAST APPLIED</small><strong>{{ state.lastAppliedAtMillis ? new Date(state.lastAppliedAtMillis).toLocaleString() : "尚未应用" }}</strong></div>
      <p>路由编辑会自动保存到智屿目录；点击“应用路由”后才会更新 Caddy。</p>
    </section>

    <section class="domain-create">
      <div><small>NEW ROUTE</small><h2>添加路由</h2></div>
      <label>名称<input v-model="draft.name" placeholder="商城 API" /></label>
      <label>域名<input v-model="draft.hostname" placeholder="shop.localhost" /></label>
      <label>路径<input v-model="draft.path" placeholder="/api" /></label>
      <label>目标地址<input v-model="draft.target" placeholder="127.0.0.1:3000" /></label>
      <label class="https-switch"><input v-model="draft.https" type="checkbox" /><i></i><span>HTTPS</span></label>
      <button type="button" @click="addRoute">＋ 添加</button>
    </section>

    <section class="domain-panel">
      <div class="domain-panel-head">
        <div><small>ROUTES</small><h2>路由列表</h2></div>
        <span>{{ saving ? "正在保存…" : `${enabledCount} 个已启用` }}</span>
      </div>
      <div v-if="state.routes.length" class="route-list">
        <article v-for="route in state.routes" :key="route.id" :class="{ disabled: !route.enabled }">
          <label class="route-toggle"><input v-model="route.enabled" type="checkbox" /><i></i></label>
          <span class="route-protocol" :class="{ secure: route.https }">{{ route.https ? "TLS" : "HTTP" }}</span>
          <div class="route-name"><strong>{{ route.name }}</strong><code>{{ routeUrl(route) }}</code></div>
          <span class="route-arrow">→</span>
          <div class="route-target"><code>{{ route.target }}</code><small :class="{ ok: checks[route.id]?.reachable, failed: checks[route.id]?.reachable === false }">{{ checks[route.id]?.message || "本机目标" }}</small></div>
          <button type="button" :disabled="checks[route.id]?.checking" @click="checkTarget(route)">{{ checks[route.id]?.checking ? "检查中" : "检查" }}</button>
          <button type="button" @click="openRoute(route)">打开 ↗</button>
          <button class="danger" type="button" @click="state.routes = state.routes.filter(item => item.id !== route.id)">×</button>
        </article>
      </div>
      <div v-else class="domain-empty">还没有路由<small>可以让 demo.localhost/api 指向 127.0.0.1:3000。</small></div>
    </section>

    <section class="domain-help">
      <article><strong>域名自动解析</strong><p><code>*.localhost</code> 自动指向本机，不需要修改 hosts，也不会污染系统配置。</p></article>
      <article><strong>按路径转发</strong><p>一个域名可用 <code>/api</code>、<code>/admin</code> 分别代理多个本地服务。</p></article>
      <article><strong>安全应用与回滚</strong><p>应用前先备份并调用 Caddy 校验；启动失败会恢复上一次配置。</p></article>
      <article><strong>WebSocket 可用</strong><p>Caddy 反向代理自动支持 WebSocket 升级，无需额外开关。</p></article>
    </section>
  </main>
</template>

<style scoped>
.domain-logo{background:#2b796a}.gateway-state{display:flex;align-items:center;gap:6px;padding:5px 8px;border:1px solid var(--color-border);color:var(--color-text-muted);font-size:8px}.gateway-state i{width:6px;height:6px;border-radius:50%;background:var(--color-text-muted)}.gateway-state.running{border-color:var(--color-success-text);color:var(--color-success-text)}.gateway-state.running i{background:var(--color-success-text)}.domain-page{display:grid;gap:12px;padding:22px 30px 40px}.gateway-summary{display:grid;grid-template-columns:150px 150px 120px minmax(220px,1fr);border:1px solid var(--color-border);background:var(--color-panel-translucent)}.gateway-summary>div{display:grid;align-content:center;gap:5px;min-height:64px;padding:8px 13px;border-right:1px solid var(--color-border)}.gateway-summary small,.domain-create small,.domain-panel-head small{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace;letter-spacing:.12em}.gateway-summary label{display:flex;align-items:center}.gateway-summary label span{color:var(--color-accent);font:13px "SFMono-Regular",monospace}.gateway-summary input{width:60px;height:25px;padding:0 5px;border:0;background:transparent;font:10px "SFMono-Regular",monospace}.gateway-summary strong{font-size:10px}.gateway-summary>p{display:flex;align-items:center;margin:0;padding:0 14px;color:var(--color-text-muted);font-size:7px}.domain-create{display:grid;grid-template-columns:125px minmax(100px,.65fr) minmax(160px,.9fr) 100px minmax(150px,.8fr) auto auto;align-items:end;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.domain-create>div{align-self:stretch;padding:12px 14px;border-right:1px solid var(--color-border);background:var(--color-bg-muted)}.domain-create h2,.domain-panel-head h2{margin:5px 0 0;font-size:12px}.domain-create>label{display:grid;gap:5px;padding:9px 5px;color:var(--color-text-muted);font-size:7px}.domain-create input:not([type=checkbox]){box-sizing:border-box;width:100%;height:31px;padding:0 8px;font-size:8px}.domain-create>button{height:31px;margin:0 10px 9px 3px;padding:0 10px;border:1px solid var(--color-control-primary);background:var(--color-control-primary);color:#fff;font-size:8px}.https-switch{display:flex!important;height:31px;align-items:center;gap:5px}.https-switch input,.route-toggle input{display:none}.https-switch i,.route-toggle i{position:relative;width:25px;height:14px;border:1px solid var(--color-border-strong);border-radius:10px;background:var(--color-bg-muted)}.https-switch i:after,.route-toggle i:after{position:absolute;content:"";top:2px;left:2px;width:8px;height:8px;border-radius:50%;background:var(--color-text-muted);transition:transform .15s ease}.https-switch input:checked+i:after,.route-toggle input:checked+i:after{transform:translateX(11px);background:var(--color-success-text)}.domain-panel{border:1px solid var(--color-border);background:var(--color-panel-translucent)}.domain-panel-head{display:flex;min-height:52px;align-items:center;justify-content:space-between;padding:7px 13px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.domain-panel-head>span{color:var(--color-text-muted);font-size:7px}.route-list article{display:grid;grid-template-columns:30px 42px minmax(210px,1fr) 20px minmax(130px,.65fr) 52px 58px 28px;align-items:center;gap:8px;min-height:58px;padding:0 11px;border-bottom:1px solid var(--color-border)}.route-list article:last-child{border-bottom:0}.route-list article.disabled{opacity:.48}.route-toggle{display:flex}.route-protocol{padding:3px 4px;border:1px solid var(--color-border);color:var(--color-text-muted);font:7px "SFMono-Regular",monospace;text-align:center}.route-protocol.secure{border-color:var(--color-success-text);color:var(--color-success-text)}.route-name,.route-target{display:grid;min-width:0;gap:3px}.route-list strong{font-size:9px}.route-list code{overflow:hidden;color:var(--color-text-muted);font:7px "SFMono-Regular",monospace;text-overflow:ellipsis;white-space:nowrap}.route-target small{overflow:hidden;color:var(--color-text-muted);font-size:6px;text-overflow:ellipsis;white-space:nowrap}.route-target small.ok{color:var(--color-success-text)}.route-target small.failed{color:var(--color-danger-text)}.route-arrow{color:var(--color-accent)}.route-list button{height:26px;padding:0 7px;border:1px solid var(--color-border-strong);background:var(--color-bg-panel);color:var(--color-text-primary);font-size:7px}.route-list button.danger{width:26px;padding:0;border-color:var(--color-danger-text);color:var(--color-danger-text)}.domain-empty{display:grid;min-height:170px;place-items:center;align-content:center;gap:7px;color:var(--color-text-muted);font-size:9px}.domain-empty small{font-size:7px}.domain-help{display:grid;grid-template-columns:repeat(4,1fr);border:1px solid var(--color-border);background:var(--color-panel-translucent)}.domain-help article{padding:12px;border-right:1px solid var(--color-border)}.domain-help article:last-child{border-right:0}.domain-help strong{font-size:8px}.domain-help p{margin:5px 0 0;color:var(--color-text-muted);font-size:7px;line-height:1.55}@media(max-width:1150px){.domain-create{grid-template-columns:repeat(3,1fr)}.domain-create>div{grid-column:1/-1}.gateway-summary{grid-template-columns:repeat(2,1fr)}.route-list article{grid-template-columns:30px 42px minmax(180px,1fr) 18px minmax(120px,.6fr) 50px 55px 28px}.domain-help{grid-template-columns:repeat(2,1fr)}}
</style>
