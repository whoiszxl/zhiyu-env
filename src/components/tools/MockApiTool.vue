<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import {
  mockApiClearRequests,
  mockApiSaveRoutes,
  mockApiStart,
  mockApiState,
  mockApiStop,
} from "../../api/tools";
import type { MockApiState, MockRoute } from "../../types";

const state = ref<MockApiState>({
  running: false,
  port: 9321,
  baseUrl: "http://127.0.0.1:9321",
  routes: [],
  recentRequests: [],
});
const selectedId = ref("");
const busy = ref(false);
const saving = ref(false);
const error = ref("");
let refreshTimer: number | undefined;
let saveTimer: number | undefined;
let stopWatchingRoutes: (() => void) | undefined;
let saveAgain = false;

const selectedRoute = computed(
  () => state.value.routes.find((route) => route.id === selectedId.value) ?? null,
);

function applyState(next: MockApiState) {
  state.value = next;
  if (!next.routes.some((route) => route.id === selectedId.value)) {
    selectedId.value = next.routes[0]?.id ?? "";
  }
}

async function load(silent = false) {
  try {
    const next = await mockApiState();
    if (silent) {
      // 运行中的轮询只刷新服务状态和请求日志，不能覆盖用户正在编辑的路由草稿。
      // 保存接口时 mockApiSaveRoutes 会单独同步完整路由。
      state.value = {
        ...state.value,
        running: next.running,
        port: next.running ? state.value.port : next.port,
        baseUrl: next.baseUrl,
        recentRequests: next.recentRequests,
      };
    } else {
      applyState(next);
    }
    if (!silent) error.value = "";
  } catch (cause) {
    if (!silent) error.value = String(cause);
  }
}

async function toggleServer() {
  if (busy.value) return;
  busy.value = true;
  error.value = "";
  try {
    applyState(
      state.value.running
        ? await mockApiStop()
        : await mockApiStart(state.value.port, state.value.routes),
    );
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
}

async function saveRoutes() {
  if (saving.value) {
    saveAgain = true;
    return;
  }
  saving.value = true;
  error.value = "";
  try {
    const next = await mockApiSaveRoutes(state.value.routes);
    // 路由对象保留在前端，防止保存返回的新数组再次触发深度监听。
    state.value = {
      ...state.value,
      running: next.running,
      port: state.value.port,
      baseUrl: next.running
        ? next.baseUrl
        : `http://127.0.0.1:${state.value.port}`,
      recentRequests: next.recentRequests,
    };
  } catch (cause) {
    error.value = String(cause);
  } finally {
    saving.value = false;
    if (saveAgain) {
      saveAgain = false;
      scheduleSave(0);
    }
  }
}

function scheduleSave(delay = 600) {
  if (saveTimer) window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => void saveRoutes(), delay);
}

function addRoute() {
  const id = `route-${Date.now()}`;
  state.value.routes.push({
    id,
    method: "GET",
    path: "/api/example",
    statusCode: 200,
    contentType: "application/json; charset=utf-8",
    responseBody: "{\n  \"ok\": true\n}",
    delayMs: 0,
    enabled: true,
  });
  selectedId.value = id;
}

function removeRoute() {
  const index = state.value.routes.findIndex((route) => route.id === selectedId.value);
  if (index < 0) return;
  state.value.routes.splice(index, 1);
  selectedId.value = state.value.routes[index]?.id ?? state.value.routes[index - 1]?.id ?? "";
}

async function clearLogs() {
  applyState(await mockApiClearRequests());
}

async function copyUrl(route: MockRoute) {
  await navigator.clipboard.writeText(`${state.value.baseUrl}${route.path}`);
}

function formatTime(value: number) {
  return new Date(value).toLocaleTimeString("zh-CN", { hour12: false });
}

onMounted(async () => {
  await load();
  stopWatchingRoutes = watch(
    () => state.value.routes,
    () => scheduleSave(),
    { deep: true },
  );
  refreshTimer = window.setInterval(() => {
    if (state.value.running) void load(true);
  }, 1000);
});

onUnmounted(() => {
  if (refreshTimer) window.clearInterval(refreshTimer);
  if (saveTimer) window.clearTimeout(saveTimer);
  stopWatchingRoutes?.();
});
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo mockapi">M</span>
      <div>
        <div class="title-line"><h1>本地 Mock API</h1><span>LOCAL HTTP SERVER</span></div>
        <p>无需编写后端代码，在本机快速模拟 HTTP 接口</p>
      </div>
    </div>
    <div class="header-actions mock-server-actions">
      <label class="port-input">端口 <input v-model.number="state.port" type="number" min="1024" max="65535" :disabled="state.running" /></label>
      <button class="primary" type="button" :disabled="busy" @click="toggleServer">
        <span v-if="busy" class="spinner"></span>
        {{ busy ? "处理中" : state.running ? "停止服务" : "启动服务" }}
      </button>
    </div>
  </header>

  <div v-if="error" class="notice danger"><span>{{ error }}</span><button type="button" @click="error = ''">×</button></div>

  <section class="mock-page">
    <div class="mock-status">
      <div class="status-main">
        <span :class="['run-dot', { active: state.running }]"></span>
        <div>
          <strong>{{ state.running ? "服务运行中" : "服务未启动" }}</strong>
          <small>{{ state.running ? "正在监听本机请求" : "启动后即可访问接口" }}</small>
        </div>
      </div>
      <code>{{ state.baseUrl }}</code>
      <div class="status-count">
        <strong>{{ state.routes.filter((route) => route.enabled).length }}</strong>
        <span>启用接口</span>
      </div>
    </div>

    <div class="mock-workspace">
      <aside class="route-sidebar">
        <div class="panel-heading"><div><p>ROUTES</p><h2>接口规则</h2></div><button type="button" @click="addRoute">＋ 新建</button></div>
        <button
          v-for="route in state.routes"
          :key="route.id"
          type="button"
          :class="['route-item', { selected: route.id === selectedId }]"
          @click="selectedId = route.id"
        >
          <span :class="['method', route.method.toLowerCase()]">{{ route.method }}</span>
          <span><strong>{{ route.path }}</strong><small>{{ route.statusCode }} · {{ route.enabled ? "已启用" : "已停用" }}</small></span>
        </button>
        <div v-if="state.routes.length === 0" class="tool-empty">还没有接口规则</div>
      </aside>

      <main class="route-editor">
        <template v-if="selectedRoute">
          <div class="panel-heading">
            <div><p>RESPONSE</p><h2>编辑接口</h2></div>
            <div class="compact-actions">
              <button type="button" @click="copyUrl(selectedRoute)">复制地址</button>
              <button class="danger-text" type="button" @click="removeRoute">删除</button>
            </div>
          </div>
          <div class="route-form">
            <div class="form-section">
              <p>REQUEST DEFINITION</p>
              <div class="request-definition">
                <label>请求方法<select v-model="selectedRoute.method"><option v-for="method in ['GET','POST','PUT','PATCH','DELETE','HEAD']" :key="method">{{ method }}</option></select></label>
                <label>接口路径<input v-model="selectedRoute.path" placeholder="/api/users" /></label>
                <label class="switch-row"><input v-model="selectedRoute.enabled" type="checkbox" /><span>启用接口</span></label>
              </div>
            </div>
            <div class="form-section">
              <p>RESPONSE SETTINGS</p>
              <div class="response-settings">
                <label>状态码<input v-model.number="selectedRoute.statusCode" type="number" min="100" max="599" /></label>
                <label>延迟（ms）<input v-model.number="selectedRoute.delayMs" type="number" min="0" max="10000" /></label>
                <label>Content-Type<input v-model="selectedRoute.contentType" /></label>
              </div>
            </div>
            <div class="form-section body-section">
              <div class="body-label"><p>RESPONSE BODY</p><span>支持 JSON、文本和 HTML</span></div>
              <textarea v-model="selectedRoute.responseBody" spellcheck="false"></textarea>
            </div>
          </div>
        </template>
        <div v-else class="tool-empty large">选择一个接口，或新建接口规则</div>
      </main>
    </div>

    <div class="request-log-panel">
      <div class="panel-heading">
        <div><p>REQUEST LOG</p><h2>最近请求</h2></div>
        <button type="button" :disabled="state.recentRequests.length === 0" @click="clearLogs">清空</button>
      </div>
      <div v-if="state.recentRequests.length === 0" class="tool-empty">启动服务并访问接口后，请求会显示在这里</div>
      <div v-else class="request-log-list">
        <div v-for="log in state.recentRequests" :key="log.id" class="request-log-row">
          <time>{{ formatTime(log.timestampMillis) }}</time>
          <span :class="['method', log.method.toLowerCase()]">{{ log.method }}</span>
          <code>{{ log.path }}</code>
          <strong :class="{ failed: log.statusCode >= 400 }">{{ log.statusCode }}</strong>
          <span>{{ log.matchedRouteId ? "已匹配" : "未匹配" }}</span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.mock-page {
  display: grid;
  gap: 14px;
  padding: 24px 32px 36px;
}

.mock-server-actions,
.port-input {
  display: flex;
  align-items: center;
}

.port-input {
  gap: 8px;
  color: var(--color-text-muted);
  font-size: 10px;
}

.port-input input {
  width: 88px;
  height: 34px;
  padding: 0 9px;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
}

.mock-status {
  display: grid;
  grid-template-columns: minmax(160px, auto) minmax(220px, 1fr) auto;
  min-height: 64px;
  align-items: center;
  gap: 22px;
  padding: 10px 18px;
  border: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

.status-main {
  display: flex;
  align-items: center;
  gap: 11px;
}

.status-main strong,
.status-main small {
  display: block;
}

.status-main strong {
  font-size: 11px;
}

.status-main small {
  margin-top: 3px;
  color: var(--color-text-muted);
  font-size: 8px;
}

.mock-status code {
  overflow: hidden;
  color: var(--color-text-secondary);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-count {
  display: flex;
  align-items: baseline;
  gap: 7px;
  padding-left: 18px;
  border-left: 1px solid var(--color-border);
  color: var(--color-text-muted);
  font-size: 9px;
}

.status-count strong {
  color: var(--color-text-primary);
  font-size: 18px;
}

.run-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--color-text-muted);
}

.run-dot.active {
  background: var(--color-success);
  box-shadow: 0 0 0 4px var(--color-success-surface);
}

.mock-workspace {
  display: grid;
  grid-template-columns: 280px minmax(0, 1fr);
  min-height: 460px;
  overflow: hidden;
  border: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

.route-sidebar {
  min-width: 0;
  border-right: 1px solid var(--color-border);
  background: var(--color-bg-muted);
}

.panel-heading {
  display: flex;
  min-height: 62px;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 11px 14px;
  border-bottom: 1px solid var(--color-border);
}

.panel-heading p,
.form-section > p,
.body-label p {
  margin: 0 0 4px;
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.12em;
}

.panel-heading h2 {
  margin: 0;
  font-size: 14px;
}

.panel-heading button,
.compact-actions button {
  min-height: 30px;
  padding: 0 10px;
  font-size: 9px;
}

.route-item {
  display: grid;
  grid-template-columns: 52px minmax(0, 1fr);
  width: 100%;
  gap: 8px;
  padding: 13px 14px;
  border: 0;
  border-bottom: 1px solid var(--color-border);
  background: transparent;
  color: var(--color-text-primary);
  text-align: left;
}

.route-item:hover {
  background: var(--color-hover);
}

.route-item.selected {
  background: var(--color-selected);
  box-shadow: inset 3px 0 var(--color-accent);
}

.route-item span:last-child {
  min-width: 0;
}

.route-item strong {
  display: block;
  overflow: hidden;
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.route-item small {
  display: block;
  margin-top: 4px;
  color: var(--color-text-muted);
  font-size: 8px;
}

.method {
  font: 700 8px/20px "SFMono-Regular", Consolas, monospace;
  letter-spacing: 0.04em;
  color: #73a9d8;
}

.method.post {
  color: #61b886;
}

.method.put,
.method.patch {
  color: #d8a451;
}

.method.delete {
  color: #e47c69;
}

.route-editor {
  min-width: 0;
  background: var(--color-bg-panel);
}

.compact-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 7px;
}

.route-form {
  padding: 18px;
}

.form-section {
  padding-bottom: 16px;
}

.form-section + .form-section {
  padding-top: 16px;
  border-top: 1px solid var(--color-border);
}

.route-form label {
  display: grid;
  min-width: 0;
  gap: 6px;
  color: var(--color-text-secondary);
  font-size: 9px;
}

.request-definition {
  display: grid;
  grid-template-columns: 126px minmax(220px, 1fr) 108px;
  gap: 12px;
}

.response-settings {
  display: grid;
  grid-template-columns: 112px 126px minmax(220px, 1fr);
  gap: 12px;
}

.route-form input,
.route-form select {
  width: 100%;
  height: 34px;
  min-width: 0;
  padding: 0 10px;
  font-size: 10px;
}

.route-form .switch-row {
  display: flex;
  min-height: 34px;
  align-items: center;
  align-self: end;
  justify-content: center;
  gap: 7px;
  border: 1px solid var(--color-border);
  background: var(--color-bg-muted);
}

.route-form .switch-row input {
  width: 13px;
  height: 13px;
  min-height: 0;
  padding: 0;
}

.body-section {
  padding-bottom: 0;
}

.body-label {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
}

.body-label span {
  color: var(--color-text-muted);
  font-size: 8px;
}

.route-form textarea {
  display: block;
  width: 100%;
  min-height: 180px;
  margin-top: 8px;
  padding: 13px 14px;
  resize: vertical;
  border-color: #3d4139;
  background: #20231d;
  color: #e9ede5;
  font: 10px/1.65 "SFMono-Regular", Consolas, monospace;
}

.tool-empty {
  padding: 24px;
  color: var(--color-text-muted);
  font-size: 9px;
  text-align: center;
}

.tool-empty.large {
  padding-top: 190px;
}

.request-log-panel {
  overflow: hidden;
  border: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

.request-log-list {
  max-height: 210px;
  overflow: auto;
}

.request-log-row {
  display: grid;
  grid-template-columns: 72px 52px minmax(0, 1fr) 46px 58px;
  align-items: center;
  gap: 10px;
  padding: 9px 14px;
  border-top: 1px solid var(--color-border);
  font-size: 9px;
}

.request-log-row:first-child {
  border-top: 0;
}

.request-log-row time,
.request-log-row > span:last-child {
  color: var(--color-text-muted);
}

.request-log-row code {
  overflow: hidden;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 9px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.request-log-row strong {
  color: var(--color-success-text);
}

.request-log-row strong.failed,
.danger-text {
  color: var(--color-danger-text);
}

@media (max-width: 1050px) {
  .mock-workspace {
    grid-template-columns: 230px minmax(0, 1fr);
  }

  .request-definition,
  .response-settings {
    grid-template-columns: 1fr 1fr;
  }

  .request-definition label:nth-child(2),
  .response-settings label:last-child {
    grid-column: span 1;
  }
}
</style>
