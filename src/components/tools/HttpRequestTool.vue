<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { executeHttpRequest, getHttpWorkspace, saveHttpWorkspace } from "../../api/tools";
import type {
  AiAssistOption,
  HttpHeader,
  HttpResponseOutput,
  HttpWorkspaceAuth,
  HttpWorkspaceEnvironment,
  HttpWorkspaceRequest,
  HttpWorkspaceState,
} from "../../types";
import AiAssistDialog from "../AiAssistDialog.vue";

const LEGACY_KEY = "zhiyu.http-workspace.v1";
const blankAuth = (): HttpWorkspaceAuth => ({
  kind: "none", username: "", password: "", token: "", key: "", value: "", placement: "header",
});
const workspace = ref<HttpWorkspaceState>({
  version: 2,
  activeEnvironmentId: "default",
  environments: [{ id: "default", name: "本地开发", variables: [] }],
  requests: [],
});
const method = ref("GET");
const url = ref("http://127.0.0.1:9321/api/hello");
const queryParams = ref<HttpHeader[]>([{ name: "", value: "" }]);
const headers = ref<HttpHeader[]>([{ name: "", value: "" }]);
const auth = ref<HttpWorkspaceAuth>(blankAuth());
const body = ref("");
const folder = ref("默认集合");
const timeoutSeconds = ref(15);
const followRedirects = ref(true);
const requestTab = ref<"query" | "headers" | "auth" | "body">("query");
const responseTab = ref<"body" | "headers">("body");
const response = ref<HttpResponseOutput | null>(null);
const sending = ref(false);
const saving = ref(false);
const error = ref("");
const feedback = ref("");
const aiOpen = ref(false);
const environmentOpen = ref(false);
const importOpen = ref(false);
const importText = ref("");
const selectedRequestId = ref("");
const requestSearch = ref("");
let saveTimer = 0;
let hydrated = false;

const activeEnvironment = computed<HttpWorkspaceEnvironment>(() =>
  workspace.value.environments.find((item) => item.id === workspace.value.activeEnvironmentId)
  ?? workspace.value.environments[0],
);
const activeVariables = computed(() => Object.fromEntries(
  (activeEnvironment.value?.variables || [])
    .filter((item) => item.enabled && item.key.trim())
    .map((item) => [item.key.trim(), item.value]),
));
const filteredRequests = computed(() => {
  const term = requestSearch.value.trim().toLowerCase();
  return workspace.value.requests.filter((item) =>
    !term || `${item.folder} ${item.name} ${item.method} ${item.url}`.toLowerCase().includes(term),
  );
});
const groupedRequests = computed(() => {
  const groups = new Map<string, HttpWorkspaceRequest[]>();
  for (const item of filteredRequests.value) {
    const key = item.folder.trim() || "默认集合";
    groups.set(key, [...(groups.get(key) || []), item]);
  }
  return [...groups.entries()];
});
const formattedBody = computed(() => {
  if (!response.value) return "";
  if (response.value.contentType.includes("json")) {
    try { return JSON.stringify(JSON.parse(response.value.body), null, 2); } catch { /* raw text */ }
  }
  return response.value.body;
});
const aiOptions: AiAssistOption[] = [{
  id: "http_request", label: "生成请求", hint: "描述需要调用的接口、参数和请求体", canApply: true,
}];
const aiContext = computed(() => JSON.stringify({
  currentRequest: { method: method.value, url: url.value, queryParams: queryParams.value, headers: headers.value, auth: { kind: auth.value.kind }, body: body.value },
  environmentVariables: Object.keys(activeVariables.value),
  lastResponse: response.value,
}, null, 2));

function substitute(value: string) {
  return value.replace(/\{\{\s*([\w.-]+)\s*\}\}/g, (_, key: string) =>
    Object.prototype.hasOwnProperty.call(activeVariables.value, key) ? activeVariables.value[key] : `{{${key}}}`,
  );
}

function buildRequest() {
  const requestHeaders = headers.value
    .filter((item) => item.name.trim())
    .map((item) => ({ name: substitute(item.name.trim()), value: substitute(item.value) }));
  const target = new URL(substitute(url.value.trim()));
  for (const item of queryParams.value.filter((entry) => entry.name.trim())) {
    target.searchParams.append(substitute(item.name.trim()), substitute(item.value));
  }
  if (auth.value.kind === "basic") {
    requestHeaders.push({ name: "Authorization", value: `Basic ${utf8Base64(`${substitute(auth.value.username)}:${substitute(auth.value.password)}`)}` });
  } else if (auth.value.kind === "bearer" && auth.value.token) {
    requestHeaders.push({ name: "Authorization", value: `Bearer ${substitute(auth.value.token)}` });
  } else if (auth.value.kind === "apiKey" && auth.value.key) {
    if (auth.value.placement === "query") target.searchParams.append(substitute(auth.value.key), substitute(auth.value.value));
    else requestHeaders.push({ name: substitute(auth.value.key), value: substitute(auth.value.value) });
  }
  return { url: target.toString(), headers: requestHeaders, body: substitute(body.value) };
}

function utf8Base64(value: string) {
  const bytes = new TextEncoder().encode(value);
  let binary = "";
  bytes.forEach((byte) => (binary += String.fromCharCode(byte)));
  return btoa(binary);
}

async function send() {
  if (sending.value) return;
  sending.value = true;
  error.value = "";
  try {
    const resolved = buildRequest();
    response.value = await executeHttpRequest({
      method: method.value,
      url: resolved.url,
      headers: resolved.headers,
      body: resolved.body,
      timeoutSeconds: timeoutSeconds.value,
      followRedirects: followRedirects.value,
    });
  } catch (cause) {
    response.value = null;
    error.value = String(cause);
  } finally {
    sending.value = false;
  }
}

function newRequest() {
  selectedRequestId.value = "";
  method.value = "GET";
  url.value = "";
  queryParams.value = [{ name: "", value: "" }];
  headers.value = [{ name: "", value: "" }];
  auth.value = blankAuth();
  body.value = "";
  folder.value = "默认集合";
  response.value = null;
}

function loadRequest(item: HttpWorkspaceRequest) {
  selectedRequestId.value = item.id;
  method.value = item.method;
  url.value = item.url;
  queryParams.value = item.queryParams.length ? item.queryParams.map((entry) => ({ ...entry })) : [{ name: "", value: "" }];
  headers.value = item.headers.length ? item.headers.map((entry) => ({ ...entry })) : [{ name: "", value: "" }];
  auth.value = { ...blankAuth(), ...item.auth };
  body.value = item.body;
  folder.value = item.folder || "默认集合";
  response.value = null;
}

function saveCurrentRequest() {
  if (!url.value.trim()) {
    error.value = "请先填写请求地址";
    return;
  }
  const existing = workspace.value.requests.find((item) => item.id === selectedRequestId.value);
  let defaultName = `${method.value} request`;
  try { defaultName = `${method.value} ${new URL(substitute(url.value)).pathname || "/"}`; } catch { /* keep fallback */ }
  const name = existing?.name || prompt("请求名称", defaultName) || "";
  if (!name.trim()) return;
  const request: HttpWorkspaceRequest = {
    id: existing?.id || crypto.randomUUID(),
    name: name.trim(),
    folder: folder.value.trim() || "默认集合",
    method: method.value,
    url: url.value.trim(),
    queryParams: queryParams.value.filter((item) => item.name.trim()).map((item) => ({ ...item })),
    headers: headers.value.filter((item) => item.name.trim()).map((item) => ({ ...item })),
    body: body.value,
    auth: { ...auth.value },
    updatedAt: Date.now(),
  };
  const index = workspace.value.requests.findIndex((item) => item.id === request.id);
  if (index >= 0) workspace.value.requests[index] = request;
  else workspace.value.requests.unshift(request);
  selectedRequestId.value = request.id;
  flash("请求已保存");
}

function deleteRequest(item: HttpWorkspaceRequest) {
  if (!confirm(`删除请求“${item.name}”？`)) return;
  workspace.value.requests = workspace.value.requests.filter((request) => request.id !== item.id);
  if (selectedRequestId.value === item.id) newRequest();
}

function addRow(target: "header" | "query") {
  (target === "header" ? headers : queryParams).value.push({ name: "", value: "" });
}

function removeRow(target: "header" | "query", index: number) {
  const rows = (target === "header" ? headers : queryParams);
  rows.value.splice(index, 1);
  if (!rows.value.length) rows.value.push({ name: "", value: "" });
}

function addEnvironment() {
  const name = prompt("环境名称", "测试环境")?.trim();
  if (!name) return;
  const environment = { id: crypto.randomUUID(), name, variables: [] };
  workspace.value.environments.push(environment);
  workspace.value.activeEnvironmentId = environment.id;
}

function addVariable() {
  activeEnvironment.value.variables.push({ key: "", value: "", secret: false, enabled: true });
}

function parseCurl() {
  try {
    const tokens = importText.value.match(/(?:[^\s"'\\]+|\\.|"(?:\\.|[^"])*"|'[^']*')+/g)?.map(unquote) || [];
    if (!tokens.length || tokens[0] !== "curl") throw new Error("请输入以 curl 开头的命令");
    let importedMethod = "GET";
    let importedUrl = "";
    const importedHeaders: HttpHeader[] = [];
    let importedBody = "";
    for (let index = 1; index < tokens.length; index++) {
      const token = tokens[index];
      if (["-X", "--request"].includes(token)) importedMethod = (tokens[++index] || "GET").toUpperCase();
      else if (["-H", "--header"].includes(token)) {
        const value = tokens[++index] || "";
        const separator = value.indexOf(":");
        if (separator > 0) importedHeaders.push({ name: value.slice(0, separator).trim(), value: value.slice(separator + 1).trim() });
      } else if (["-d", "--data", "--data-raw", "--data-binary"].includes(token)) {
        importedBody = tokens[++index] || "";
        if (importedMethod === "GET") importedMethod = "POST";
      } else if (!token.startsWith("-") && /^https?:\/\//.test(token)) importedUrl = token;
    }
    if (!importedUrl) throw new Error("cURL 中没有找到 HTTP 地址");
    newRequest();
    method.value = importedMethod;
    url.value = importedUrl;
    headers.value = importedHeaders.length ? importedHeaders : [{ name: "", value: "" }];
    body.value = importedBody;
    importOpen.value = false;
    importText.value = "";
    flash("cURL 已导入");
  } catch (cause) {
    error.value = String(cause);
  }
}

function unquote(value: string) {
  if ((value.startsWith("'") && value.endsWith("'")) || (value.startsWith('"') && value.endsWith('"'))) {
    return value.slice(1, -1).replace(/\\"/g, '"');
  }
  return value.replace(/\\(.)/g, "$1");
}

function shellQuote(value: string) {
  return `'${value.replaceAll("'", "'\\''")}'`;
}

async function copyCurl() {
  try {
    const resolved = buildRequest();
    const parts = ["curl", "-X", method.value, shellQuote(resolved.url)];
    resolved.headers.forEach((header) => parts.push("-H", shellQuote(`${header.name}: ${header.value}`)));
    if (resolved.body && !["GET", "HEAD"].includes(method.value)) parts.push("--data-raw", shellQuote(resolved.body));
    await navigator.clipboard.writeText(parts.join(" "));
    flash("cURL 已复制");
  } catch (cause) { error.value = String(cause); }
}

function exportWorkspace() {
  const redacted = structuredClone(workspace.value);
  redacted.environments.forEach((environment) =>
    environment.variables.forEach((variable) => { if (variable.secret) variable.value = ""; }),
  );
  redacted.requests.forEach((request) => {
    request.auth.password = "";
    request.auth.token = "";
    request.auth.value = "";
  });
  const href = URL.createObjectURL(new Blob([JSON.stringify(redacted, null, 2)], { type: "application/json" }));
  const anchor = document.createElement("a");
  anchor.href = href;
  anchor.download = "zhiyu-http-workspace.json";
  anchor.click();
  URL.revokeObjectURL(href);
}

async function copyResponse() {
  if (!response.value) return;
  await navigator.clipboard.writeText(formattedBody.value);
  flash("响应已复制");
}

function flash(message: string) {
  feedback.value = message;
  window.setTimeout(() => { if (feedback.value === message) feedback.value = ""; }, 1300);
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`;
  return `${(value / 1024 / 1024).toFixed(1)} MiB`;
}

function applyAiRequest(content: string) {
  try {
    const value = JSON.parse(content);
    method.value = String(value.method || "GET").toUpperCase();
    url.value = String(value.url || "");
    headers.value = Array.isArray(value.headers) && value.headers.length ? value.headers : [{ name: "", value: "" }];
    queryParams.value = Array.isArray(value.queryParams) && value.queryParams.length ? value.queryParams : [{ name: "", value: "" }];
    body.value = typeof value.body === "string" ? value.body : JSON.stringify(value.body ?? "", null, 2);
    aiOpen.value = false;
  } catch { error.value = "AI 返回的请求不是有效 JSON，请重新生成或手动复制。"; }
}

function openAiSettings() {
  window.dispatchEvent(new CustomEvent("zhiyu:open-ai-settings"));
}

function scheduleSave() {
  if (!hydrated) return;
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(async () => {
    saving.value = true;
    try { await saveHttpWorkspace(workspace.value); }
    catch (cause) { error.value = String(cause); }
    finally { saving.value = false; }
  }, 500);
}

watch(workspace, scheduleSave, { deep: true });
onMounted(async () => {
  try {
    workspace.value = await getHttpWorkspace();
    if (!workspace.value.requests.length) {
      const legacy = JSON.parse(localStorage.getItem(LEGACY_KEY) || "[]");
      if (Array.isArray(legacy) && legacy.length) {
        workspace.value.requests = legacy.map((item) => ({
          ...item, folder: "默认集合", queryParams: [], auth: blankAuth(),
        }));
        workspace.value = await saveHttpWorkspace(workspace.value);
        localStorage.removeItem(LEGACY_KEY);
      }
    }
  } catch (cause) { error.value = String(cause); }
  hydrated = true;
});
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo http">H</span>
      <div>
        <div class="title-line"><h1>HTTP/API 工作区</h1><span>REST CLIENT 2.0</span></div>
        <p>用集合、环境变量和认证配置组织接口调试，数据保存在本机</p>
      </div>
    </div>
    <div class="header-actions">
      <select v-model="workspace.activeEnvironmentId" class="environment-select"><option v-for="item in workspace.environments" :key="item.id" :value="item.id">{{ item.name }}</option></select>
      <button type="button" @click="environmentOpen=true">变量</button>
      <button type="button" @click="importOpen=true">导入 cURL</button>
      <button type="button" @click="exportWorkspace">导出</button>
      <button type="button" @click="saveCurrentRequest">{{ saving ? "保存中…" : "保存请求" }}</button>
      <button type="button" @click="aiOpen=true">✦ AI 生成</button>
    </div>
  </header>
  <div v-if="error" class="notice danger"><span>{{ error }}</span><button type="button" @click="error=''">×</button></div>

  <section class="http-page">
    <aside class="http-collection">
      <div class="collection-head"><div><p>API WORKSPACE</p><h2>请求集合</h2></div><button type="button" @click="newRequest">＋</button></div>
      <input v-model="requestSearch" class="collection-search" placeholder="搜索名称、集合或地址…" />
      <div v-if="groupedRequests.length" class="collection-list">
        <section v-for="[group, items] in groupedRequests" :key="group">
          <h3>{{ group }} <span>{{ items.length }}</span></h3>
          <button v-for="item in items" :key="item.id" type="button" :class="{active:selectedRequestId===item.id}" @click="loadRequest(item)">
            <i :class="item.method.toLowerCase()">{{ item.method }}</i><span><strong>{{ item.name }}</strong><small>{{ item.url }}</small></span><b title="删除" @click.stop="deleteRequest(item)">×</b>
          </button>
        </section>
      </div>
      <div v-else class="collection-empty">还没有请求<small>填写请求后点击顶部“保存请求”</small></div>
      <footer><span>{{ workspace.requests.length }} 个请求</span><small>{{ feedback || "自动保存" }}</small></footer>
    </aside>

    <div class="http-workspace-main">
      <div class="request-line">
        <select v-model="method"><option v-for="item in ['GET','POST','PUT','PATCH','DELETE','HEAD','OPTIONS']" :key="item">{{ item }}</option></select>
        <input v-model="url" type="url" placeholder="{{baseUrl}}/api/users" @keyup.enter="send" />
        <button class="primary send-button" type="button" :disabled="sending" @click="send"><span v-if="sending" class="spinner"></span>{{ sending ? "发送中" : "发送请求" }}</button>
      </div>
      <div class="request-meta">
        <label>集合<input v-model="folder" placeholder="默认集合" /></label>
        <span>可在 URL、参数、请求头、认证和请求体中使用 <code v-pre>{{variable}}</code></span>
        <button type="button" @click="copyCurl">{{ feedback || "复制 cURL" }}</button>
      </div>

      <article class="http-panel request-panel">
        <div class="tabbar">
          <div class="tab-group">
            <button :class="{active:requestTab==='query'}" @click="requestTab='query'">参数 <small>{{queryParams.filter(i=>i.name.trim()).length}}</small></button>
            <button :class="{active:requestTab==='headers'}" @click="requestTab='headers'">请求头 <small>{{headers.filter(i=>i.name.trim()).length}}</small></button>
            <button :class="{active:requestTab==='auth'}" @click="requestTab='auth'">认证</button>
            <button :class="{active:requestTab==='body'}" @click="requestTab='body'">请求体</button>
          </div>
          <span></span>
          <div class="request-options"><label>超时 <input v-model.number="timeoutSeconds" type="number" min="1" max="120" /> 秒</label><label><input v-model="followRedirects" type="checkbox" /> 跟随重定向</label></div>
        </div>
        <div v-if="requestTab==='query'||requestTab==='headers'" class="pair-editor">
          <div class="pair-columns"><span>名称</span><span>值</span><span></span></div>
          <div v-for="(item,index) in requestTab==='query'?queryParams:headers" :key="index" class="pair-row">
            <input v-model="item.name" :placeholder="requestTab==='query'?'参数名称':'Header 名称'" />
            <input v-model="item.value" placeholder="支持 {{variable}}" />
            <button type="button" @click="removeRow(requestTab==='query'?'query':'header',index)">×</button>
          </div>
          <button class="add-row" type="button" @click="addRow(requestTab==='query'?'query':'header')">＋ 添加{{requestTab==='query'?'参数':'请求头'}}</button>
        </div>
        <div v-else-if="requestTab==='auth'" class="auth-editor">
          <label>认证方式<select v-model="auth.kind"><option value="none">无认证</option><option value="basic">Basic Auth</option><option value="bearer">Bearer Token</option><option value="apiKey">API Key</option></select></label>
          <template v-if="auth.kind==='basic'"><label>用户名<input v-model="auth.username" /></label><label>密码<input v-model="auth.password" type="password" /></label></template>
          <label v-else-if="auth.kind==='bearer'" class="auth-wide">Token<input v-model="auth.token" type="password" placeholder="{{token}}" /></label>
          <template v-else-if="auth.kind==='apiKey'"><label>Key<input v-model="auth.key" placeholder="X-API-Key" /></label><label>Value<input v-model="auth.value" type="password" /></label><label>位置<select v-model="auth.placement"><option value="header">Header</option><option value="query">Query</option></select></label></template>
          <p>认证信息只保存在本机；导出工作区时会自动清除密码与令牌。</p>
        </div>
        <div v-else class="body-editor"><textarea v-model="body" spellcheck="false" placeholder="{&#10;  &quot;name&quot;: &quot;{{userName}}&quot;&#10;}"></textarea><small>GET 和 HEAD 不发送请求体；单次请求体最大 2 MiB。</small></div>
      </article>

      <article class="http-panel response-panel">
        <div class="response-heading"><div><p>RESPONSE</p><h2>响应结果</h2></div><div v-if="response" class="response-metrics"><strong :class="{failed:response.statusCode>=400}">{{response.statusCode}} {{response.statusText}}</strong><span>{{response.elapsedMs}} ms</span><span>{{formatBytes(response.sizeBytes)}}</span></div></div>
        <div v-if="response" class="tabbar"><div class="tab-group"><button :class="{active:responseTab==='body'}" @click="responseTab='body'">响应体</button><button :class="{active:responseTab==='headers'}" @click="responseTab='headers'">响应头 <small>{{response.headers.length}}</small></button></div><span></span><button class="copy-response" @click="copyResponse">复制响应</button></div>
        <div v-if="sending" class="response-empty"><span class="spinner"></span> 正在等待服务器响应…</div>
        <div v-else-if="!response" class="response-empty">填写请求地址后发送<small>环境变量会在发送前解析，不会修改保存的请求模板。</small></div>
        <template v-else><pre v-if="responseTab==='body'" class="response-body">{{formattedBody}}</pre><div v-else class="response-headers"><div v-for="(item,index) in response.headers" :key="index"><code>{{item.name}}</code><span>{{item.value}}</span></div></div><div v-if="response.truncated" class="truncated-note">响应超过 2 MiB，仅展示前 2 MiB。</div></template>
      </article>
    </div>
  </section>

  <div v-if="environmentOpen" class="workspace-modal" @click.self="environmentOpen=false"><section>
    <header><div><small>ENVIRONMENTS</small><h2>环境变量</h2></div><button @click="environmentOpen=false">×</button></header>
    <div class="environment-toolbar"><select v-model="workspace.activeEnvironmentId"><option v-for="item in workspace.environments" :key="item.id" :value="item.id">{{item.name}}</option></select><button @click="addEnvironment">＋ 新建环境</button><button @click="addVariable">＋ 添加变量</button></div>
    <div class="variable-head"><span>启用</span><span>变量名</span><span>值</span><span>敏感</span><span></span></div>
    <div v-for="(item,index) in activeEnvironment.variables" :key="index" class="variable-row"><input v-model="item.enabled" type="checkbox" /><input v-model="item.key" placeholder="baseUrl" /><input v-model="item.value" :type="item.secret?'password':'text'" placeholder="http://127.0.0.1:3000" /><input v-model="item.secret" type="checkbox" /><button @click="activeEnvironment.variables.splice(index,1)">×</button></div>
    <div v-if="!activeEnvironment.variables.length" class="modal-empty">还没有变量，可以添加 <code v-pre>{{baseUrl}}</code>、<code v-pre>{{token}}</code> 等。</div>
  </section></div>
  <div v-if="importOpen" class="workspace-modal" @click.self="importOpen=false"><section class="import-dialog"><header><div><small>IMPORT</small><h2>导入 cURL</h2></div><button @click="importOpen=false">×</button></header><textarea v-model="importText" placeholder="curl -X POST 'https://api.example.com/users' -H 'Content-Type: application/json' --data-raw '{...}'"></textarea><footer><span>只解析命令文本，不会在系统 Shell 中执行。</span><button class="primary" @click="parseCurl">导入请求</button></footer></section></div>
  <AiAssistDialog :open="aiOpen" title="AI HTTP 请求助手" :context="aiContext" :options="aiOptions" @close="aiOpen=false" @settings="openAiSettings" @apply="applyAiRequest" />
</template>

<style scoped>
.http-page{display:grid;grid-template-columns:220px minmax(0,1fr);gap:12px;padding:22px 30px 38px}.http-workspace-main{display:grid;align-content:start;gap:11px;min-width:0}.environment-select{height:30px;max-width:120px;font-size:8px}.http-collection{display:grid;grid-template-rows:auto auto minmax(0,1fr) auto;min-height:620px;overflow:hidden;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.collection-head{display:flex;min-height:52px;align-items:center;justify-content:space-between;padding:7px 10px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.collection-head p,.response-heading p{margin:0;color:var(--color-text-muted);font:7px "SFMono-Regular",monospace;letter-spacing:.12em}.collection-head h2,.response-heading h2{margin:4px 0 0;font-size:12px}.collection-head button{width:26px;height:26px;padding:0}.collection-search{box-sizing:border-box;width:calc(100% - 18px);height:30px;margin:9px;padding:0 8px;font-size:8px}.collection-list{overflow:auto;border-top:1px solid var(--color-border)}.collection-list h3{display:flex;justify-content:space-between;margin:0;padding:7px 9px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted);color:var(--color-text-muted);font:7px "SFMono-Regular",monospace}.collection-list button{display:grid;width:100%;grid-template-columns:36px minmax(0,1fr) 18px;align-items:center;gap:6px;padding:9px;border:0;border-bottom:1px solid var(--color-border);background:transparent;color:var(--color-text-primary);text-align:left}.collection-list button.active{background:var(--color-panel-active);box-shadow:inset 3px 0 var(--color-accent)}.collection-list i{color:var(--color-accent);font:normal 7px "SFMono-Regular",monospace}.collection-list span{display:grid;min-width:0;gap:3px}.collection-list strong,.collection-list small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.collection-list strong{font-size:8px}.collection-list small{color:var(--color-text-muted);font-size:6px}.collection-list b{color:var(--color-text-muted);font-weight:400}.collection-empty,.modal-empty{display:grid;place-items:center;align-content:center;gap:6px;color:var(--color-text-muted);font-size:8px;text-align:center}.collection-empty small{font-size:7px}.http-collection footer{display:flex;justify-content:space-between;padding:7px 9px;border-top:1px solid var(--color-border);color:var(--color-text-muted);font-size:7px}.request-line{display:grid;grid-template-columns:100px minmax(0,1fr) 105px;overflow:hidden;border:1px solid var(--color-border-strong);background:var(--color-bg-elevated)}.request-line>*{height:39px;min-height:39px;border:0;border-radius:0}.request-line select{padding:0 11px;border-right:1px solid var(--color-border);background:var(--color-bg-muted);color:var(--color-accent);font:700 10px "SFMono-Regular",monospace}.request-line input{min-width:0;padding:0 12px;background:transparent;font:9px "SFMono-Regular",monospace}.request-line input:focus{box-shadow:inset 0 -2px var(--color-accent)}.request-meta{display:flex;align-items:center;gap:9px;min-height:34px;padding:0 9px;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.request-meta label{display:flex;align-items:center;gap:6px;color:var(--color-text-muted);font-size:7px}.request-meta input{width:115px;height:25px;padding:0 6px;font-size:7px}.request-meta>span{flex:1;color:var(--color-text-muted);font-size:7px}.request-meta button{height:25px;padding:0 8px;font-size:7px}.http-panel{min-width:0;overflow:hidden;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.tabbar{display:flex;min-height:46px;align-items:stretch;gap:5px;padding:0 12px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.tabbar>span{flex:1}.tab-group{display:flex;align-items:stretch}.tab-group button{min-width:66px;padding:0 10px;border:0;background:transparent;color:var(--color-text-secondary);font-size:8px}.tab-group button.active{background:var(--color-bg-panel);color:var(--color-text-primary);box-shadow:inset 0 -2px var(--color-accent)}.tab-group small{display:inline-grid;min-width:15px;height:15px;margin-left:3px;place-items:center;border-radius:8px;background:var(--color-selected);font-size:7px}.request-options{display:flex;align-items:center;gap:12px}.request-options label{display:flex;align-items:center;gap:5px;color:var(--color-text-muted);font-size:8px;white-space:nowrap}.request-options input[type=number]{width:44px;height:25px;padding:0 5px;font-size:8px}.request-options input[type=checkbox]{width:12px;height:12px;min-height:0}.pair-editor{min-height:135px;padding:11px 14px 14px}.pair-columns,.pair-row{display:grid;grid-template-columns:minmax(150px,.7fr) minmax(230px,1.3fr) 28px;gap:7px}.pair-columns{padding:0 7px 5px;color:var(--color-text-muted);font-size:7px}.pair-row{margin-bottom:6px}.pair-row input{height:30px;min-width:0;padding:0 8px;font:8px "SFMono-Regular",monospace}.pair-row button{min-height:30px;padding:0;background:transparent;color:var(--color-text-muted)}.add-row{height:27px;padding:0 8px;font-size:7px}.auth-editor{display:grid;grid-template-columns:repeat(3,minmax(140px,1fr));gap:10px;min-height:135px;padding:14px}.auth-editor label{display:grid;align-content:start;gap:6px;color:var(--color-text-muted);font-size:7px}.auth-editor input,.auth-editor select{height:31px;padding:0 8px;font-size:8px}.auth-editor .auth-wide{grid-column:span 2}.auth-editor p{grid-column:1/-1;margin:0;color:var(--color-text-muted);font-size:7px}.body-editor{padding:12px 14px}.body-editor textarea{box-sizing:border-box;width:100%;min-height:150px;padding:11px 12px;resize:vertical;background:var(--terminal-bg);color:#e9ede5;font:8px/1.6 "SFMono-Regular",monospace}.body-editor small{display:block;margin-top:6px;color:var(--color-text-muted);font-size:7px}.response-heading{display:flex;min-height:54px;align-items:center;justify-content:space-between;padding:7px 13px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.response-metrics{display:flex;align-items:center;gap:8px;font:7px "SFMono-Regular",monospace}.response-metrics strong{color:var(--color-success-text)}.response-metrics strong.failed{color:var(--color-danger-text)}.response-metrics span{color:var(--color-text-muted)}.response-empty{display:grid;min-height:200px;place-items:center;align-content:center;gap:7px;color:var(--color-text-muted);font-size:8px}.response-empty small{font-size:7px}.response-body{box-sizing:border-box;max-height:360px;min-height:200px;margin:0;overflow:auto;padding:14px;white-space:pre-wrap;color:var(--color-text-secondary);font:8px/1.65 "SFMono-Regular",monospace}.response-headers{max-height:360px;overflow:auto}.response-headers>div{display:grid;grid-template-columns:190px minmax(0,1fr);gap:10px;padding:8px 12px;border-bottom:1px solid var(--color-border);font-size:7px}.response-headers code{color:var(--color-accent)}.copy-response{align-self:center;height:26px;font-size:7px}.truncated-note{padding:7px 12px;border-top:1px solid var(--color-warning-text);color:var(--color-warning-text);font-size:7px}.workspace-modal{position:fixed;z-index:1300;inset:0;display:grid;place-items:center;padding:40px;background:rgba(8,12,15,.58);backdrop-filter:blur(5px)}.workspace-modal>section{width:min(760px,calc(100vw - 80px));max-height:calc(100vh - 100px);overflow:auto;border:1px solid var(--color-border-strong);background:var(--color-bg-panel);box-shadow:0 18px 60px rgba(0,0,0,.3)}.workspace-modal header{display:flex;align-items:center;justify-content:space-between;padding:12px 15px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.workspace-modal header small{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace;letter-spacing:.12em}.workspace-modal h2{margin:4px 0 0;font-size:13px}.workspace-modal header button{width:27px;height:27px;padding:0}.environment-toolbar{display:flex;gap:8px;padding:11px 14px;border-bottom:1px solid var(--color-border)}.environment-toolbar select{flex:1;height:31px}.environment-toolbar button{height:31px;font-size:8px}.variable-head,.variable-row{display:grid;grid-template-columns:40px minmax(120px,.7fr) minmax(220px,1.3fr) 40px 28px;align-items:center;gap:7px;padding:6px 14px}.variable-head{color:var(--color-text-muted);font-size:7px}.variable-row input:not([type=checkbox]){height:31px;padding:0 8px;font-size:8px}.variable-row input[type=checkbox]{width:13px;height:13px;min-height:0}.variable-row button{height:27px;padding:0}.modal-empty{min-height:150px}.import-dialog textarea{box-sizing:border-box;width:calc(100% - 28px);min-height:190px;margin:14px;padding:12px;resize:vertical;background:var(--terminal-bg);color:#e9ede5;font:8px/1.6 "SFMono-Regular",monospace}.import-dialog footer{display:flex;align-items:center;justify-content:space-between;padding:10px 14px;border-top:1px solid var(--color-border);color:var(--color-text-muted);font-size:7px}.import-dialog footer button{height:30px}@media(max-width:1050px){.http-page{grid-template-columns:190px minmax(0,1fr);padding-inline:18px}.header-actions>button:nth-of-type(3),.header-actions>button:nth-of-type(4){display:none}}
</style>
