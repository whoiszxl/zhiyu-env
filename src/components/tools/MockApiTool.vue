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
import type { AiAssistOption } from "../../types";
import AiAssistDialog from "../AiAssistDialog.vue";

type AiRouteCandidate = MockRoute & {
  selected: boolean;
  conflict: boolean;
};

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
const aiOpen = ref(false);
const aiPreviewOpen = ref(false);
const aiCandidates = ref<AiRouteCandidate[]>([]);
const aiRejected = ref<string[]>([]);
const aiImporting = ref(false);
const aiOptions: AiAssistOption[] = [{
  id: "mock_api",
  label: "批量生成",
  hint: "描述业务模块、接口、字段，以及希望覆盖的成功或异常场景",
  canApply: true,
}];
const aiContext = computed(() => JSON.stringify({
  server: state.value.baseUrl,
  existingRoutes: state.value.routes.map((route) => ({
    method: route.method,
    path: route.path,
    statusCode: route.statusCode,
  })),
}, null, 2));
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

function extractBalancedJson(content: string) {
  const start = content.search(/[\[{]/);
  if (start < 0) return "";
  const stack: string[] = [];
  let quoted = false;
  let escaped = false;
  for (let index = start; index < content.length; index += 1) {
    const character = content[index];
    if (quoted) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === '"') quoted = false;
      continue;
    }
    if (character === '"') {
      quoted = true;
      continue;
    }
    if (character === "{" || character === "[") stack.push(character);
    if (character === "}" || character === "]") {
      const expected = character === "}" ? "{" : "[";
      if (stack.pop() !== expected) return "";
      if (!stack.length) return content.slice(start, index + 1);
    }
  }
  return "";
}

function parseAiJson(content: string): unknown {
  const trimmed = content.trim().replace(/^\uFEFF/, "");
  const codeBlocks = Array.from(
    trimmed.matchAll(/```(?:json|javascript|js)?\s*([\s\S]*?)```/gi),
    (match) => match[1].trim(),
  );
  const candidates = [
    trimmed,
    ...codeBlocks,
    extractBalancedJson(trimmed),
    ...codeBlocks.map(extractBalancedJson),
  ].filter((value, index, values) => value && values.indexOf(value) === index);
  let lastError = "";
  for (const candidate of candidates) {
    try {
      return JSON.parse(candidate);
    } catch (cause) {
      lastError = cause instanceof Error ? cause.message : String(cause);
    }
  }
  throw new Error(
    `AI 返回的 JSON 语法不完整，无法导入${lastError ? `：${lastError}` : ""}`,
  );
}

function routeValues(parsed: unknown): unknown[] {
  if (Array.isArray(parsed)) return parsed;
  if (!parsed || typeof parsed !== "object") return [];
  const root = parsed as Record<string, unknown>;
  for (const key of ["routes", "endpoints", "apis", "data"]) {
    if (Array.isArray(root[key])) return root[key] as unknown[];
  }
  return [];
}

function routeField(
  value: Record<string, unknown>,
  ...keys: string[]
): unknown {
  for (const key of keys) {
    if (value[key] !== undefined && value[key] !== null) return value[key];
  }
  return undefined;
}

function normalizeRoutePath(value: unknown) {
  let path = String(value ?? "").trim();
  if (/^https?:\/\//i.test(path)) {
    try {
      path = new URL(path).pathname;
    } catch {
      return "";
    }
  }
  path = path.split(/[?#]/, 1)[0].trim();
  if (path && !path.startsWith("/")) path = `/${path}`;
  return path.replace(/\/{2,}/g, "/");
}

function parseInteger(value: unknown, fallback: number) {
  if (value === undefined || value === null || value === "") return fallback;
  const matched = String(value).trim().match(/^-?\d+/);
  return matched ? Number(matched[0]) : Number.NaN;
}

function parseEnabled(value: unknown) {
  if (typeof value === "boolean") return value;
  if (typeof value === "number") return value !== 0;
  if (typeof value === "string") {
    return !["false", "0", "off", "disabled", "no"].includes(
      value.trim().toLowerCase(),
    );
  }
  return true;
}

function previewAiRoutes(content: string) {
  try {
    const values = routeValues(parseAiJson(content));
    if (values.length === 0) {
      throw new Error("AI 没有返回可导入的接口");
    }
    if (values.length > 30) {
      throw new Error("单次最多导入 30 个接口");
    }
    const identities = new Set(
      state.value.routes.map((route) => `${route.method.toUpperCase()} ${route.path}`),
    );
    const generated = new Set<string>();
    const rejected: string[] = [];
    const candidates: AiRouteCandidate[] = [];
    values.forEach((raw, index) => {
      try {
        if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
          throw new Error("接口定义不是对象");
        }
        const value = raw as Record<string, unknown>;
        const method = String(
          routeField(value, "method", "httpMethod", "http_method") ?? "GET",
        ).trim().toUpperCase();
        const path = normalizeRoutePath(
          routeField(value, "path", "url", "route", "endpoint"),
        );
        const statusCode = parseInteger(
          routeField(value, "statusCode", "status_code", "status", "code"),
          200,
        );
        const delayMs = parseInteger(
          routeField(value, "delayMs", "delay_ms", "delay", "latency"),
          0,
        );
        if (!["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"].includes(method)) {
          throw new Error(`不支持请求方法 ${method || "（空）"}`);
        }
        if (!path.startsWith("/")) throw new Error("路径必须以 / 开头");
        if (!Number.isInteger(statusCode) || statusCode < 100 || statusCode > 599) {
          throw new Error(`状态码 ${statusCode} 无效`);
        }
        if (!Number.isInteger(delayMs) || delayMs < 0 || delayMs > 10_000) {
          throw new Error(`延迟 ${delayMs} ms 无效`);
        }
        const identity = `${method} ${path}`;
        const conflict = identities.has(identity) || generated.has(identity);
        generated.add(identity);
        const response = routeField(
          value,
          "responseBody",
          "response_body",
          "body",
          "response",
          "data",
        );
        const body = typeof response === "string"
          ? response
          : JSON.stringify(response ?? {}, null, 2);
        if (body.length > 1024 * 1024) throw new Error("响应内容超过 1 MiB");
        if (/<script\b|on\w+\s*=|javascript:/i.test(body)) {
          throw new Error("响应包含脚本或事件处理器");
        }
        const headers = value.headers && typeof value.headers === "object"
          ? value.headers as Record<string, unknown>
          : {};
        const contentType = routeField(
          value,
          "contentType",
          "content_type",
          "mimeType",
          "mime_type",
        ) ?? headers["Content-Type"] ?? headers["content-type"];
        candidates.push({
          id: `ai-route-${Date.now()}-${index}`,
          method,
          path,
          statusCode,
          contentType: String(contentType || "application/json; charset=utf-8"),
          responseBody: body,
          delayMs,
          enabled: parseEnabled(routeField(value, "enabled", "active")),
          selected: !conflict,
          conflict,
        });
      } catch (cause) {
        const message = cause instanceof Error ? cause.message : String(cause);
        rejected.push(`第 ${index + 1} 条：${message}`);
      }
    });
    if (!candidates.length) {
      throw new Error(`没有可导入的接口。${rejected.join("；")}`);
    }
    aiCandidates.value = candidates;
    aiRejected.value = rejected;
    aiOpen.value = false;
    aiPreviewOpen.value = true;
    error.value = "";
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
    aiOpen.value = false;
    aiPreviewOpen.value = false;
  }
}

async function importAiRoutes() {
  if (aiImporting.value) return;
  const routes = aiCandidates.value
    .filter((route) => route.selected && !route.conflict)
    .map(({ selected: _selected, conflict: _conflict, ...route }) => route);
  if (!routes.length) {
    error.value = "没有选择可导入的接口";
    return;
  }
  aiImporting.value = true;
  error.value = "";
  try {
    const next = await mockApiSaveRoutes([...state.value.routes, ...routes]);
    applyState(next);
    selectedId.value = routes[0].id;
    aiPreviewOpen.value = false;
    aiCandidates.value = [];
    aiRejected.value = [];
  } catch (cause) {
    error.value = `导入失败：${cause instanceof Error ? cause.message : String(cause)}`;
  } finally {
    aiImporting.value = false;
  }
}

function openAiSettings() {
  window.dispatchEvent(new CustomEvent("zhiyu:open-ai-settings"));
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
      <button type="button" @click="aiOpen = true">✦ AI 批量生成</button>
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
  <AiAssistDialog
    :open="aiOpen"
    title="AI Mock API 生成器"
    :context="aiContext"
    :options="aiOptions"
    @close="aiOpen = false"
    @settings="openAiSettings"
    @apply="previewAiRoutes"
  />
  <Teleport to="body">
    <div
      v-if="aiPreviewOpen"
      v-tool-i18n
      class="ai-route-backdrop"
      @mousedown.self="aiPreviewOpen = false"
    >
      <section class="ai-route-preview" role="dialog" aria-modal="true">
        <header>
          <div><p>IMPORT PREVIEW</p><h2>确认 AI 生成的接口</h2></div>
          <button type="button" @click="aiPreviewOpen = false">×</button>
        </header>
        <div class="ai-route-summary">
          <span>{{ aiCandidates.length }} 个候选接口</span>
          <span>{{ aiCandidates.filter((item) => item.conflict).length }} 个冲突</span>
          <span v-if="aiRejected.length" class="rejected">
            {{ aiRejected.length }} 条已跳过
          </span>
          <small>冲突接口不会覆盖现有配置</small>
        </div>
        <details v-if="aiRejected.length" class="ai-route-rejected">
          <summary>查看未导入条目</summary>
          <p v-for="item in aiRejected" :key="item">{{ item }}</p>
        </details>
        <p v-if="error" class="ai-route-error">{{ error }}</p>
        <div class="ai-route-list">
          <label
            v-for="route in aiCandidates"
            :key="route.id"
            :class="{ conflict: route.conflict }"
          >
            <input v-model="route.selected" type="checkbox" :disabled="route.conflict" />
            <b :class="route.method.toLowerCase()">{{ route.method }}</b>
            <code>{{ route.path }}</code>
            <span>{{ route.statusCode }} · {{ route.delayMs }} ms</span>
            <em>{{ route.conflict ? "与现有接口冲突" : "可导入" }}</em>
          </label>
        </div>
        <footer>
          <small>导入后会自动保存；服务运行中也会立即更新路由。</small>
          <button type="button" @click="aiPreviewOpen = false">取消</button>
          <button
            class="primary"
            type="button"
            :disabled="
              aiImporting ||
              !aiCandidates.some((item) => item.selected && !item.conflict)
            "
            @click="importAiRoutes"
          >{{ aiImporting ? "正在导入…" : "导入所选接口" }}</button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.mock-page {
  display: grid;
  gap: 14px;
  padding: 24px 32px 36px;
}

.ai-route-backdrop{position:fixed;z-index:10030;inset:0;display:grid;place-items:center;padding:24px;background:color-mix(in srgb,var(--color-bg-sidebar) 52%,transparent);backdrop-filter:blur(8px)}
.ai-route-preview{display:flex;width:min(720px,100%);max-height:min(650px,calc(100vh - 48px));flex-direction:column;overflow:hidden;border:1px solid var(--color-border-strong);border-radius:8px;background:var(--color-bg-content);box-shadow:0 24px 70px rgb(0 0 0/.3)}
.ai-route-preview>header{display:flex;min-height:62px;align-items:center;justify-content:space-between;padding:10px 16px;border-bottom:1px solid var(--color-border);background:var(--color-header)}
.ai-route-preview p{margin:0 0 3px;color:var(--color-text-muted);font:8px "SFMono-Regular",Consolas,monospace;letter-spacing:.12em}.ai-route-preview h2{margin:0;font-size:14px}.ai-route-preview>header button{width:30px;height:30px;border:0;background:transparent;color:var(--color-text-muted);font-size:18px}
.ai-route-summary{display:flex;align-items:center;gap:16px;padding:10px 14px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted);font-size:9px}.ai-route-summary small{margin-left:auto;color:var(--color-text-muted)}
.ai-route-summary .rejected{color:var(--color-warning-text)}.ai-route-rejected{padding:8px 14px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted);color:var(--color-warning-text);font-size:8px}.ai-route-rejected summary{cursor:pointer}.ai-route-rejected p{margin:5px 0 0;color:var(--color-text-muted)}
.ai-route-error{margin:0;padding:9px 14px;border-bottom:1px solid var(--color-danger-text);background:var(--color-danger-surface);color:var(--color-danger-text);font-size:8px}
.ai-route-list{min-height:180px;overflow:auto}.ai-route-list label{display:grid;grid-template-columns:18px 62px minmax(0,1fr) 90px 100px;min-height:48px;align-items:center;gap:8px;padding:8px 14px;border-bottom:1px solid var(--color-border);font-size:9px}.ai-route-list label.conflict{opacity:.58}.ai-route-list b{font:700 8px "SFMono-Regular",Consolas,monospace;color:var(--color-accent)}.ai-route-list code{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.ai-route-list span,.ai-route-list em{color:var(--color-text-muted);font-style:normal}.ai-route-list label.conflict em{color:var(--color-danger-text)}
.ai-route-preview>footer{display:flex;align-items:center;gap:8px;padding:11px 14px;border-top:1px solid var(--color-border);background:var(--color-header)}.ai-route-preview>footer small{margin-right:auto;color:var(--color-text-muted);font-size:8px}.ai-route-preview>footer button{min-height:32px;padding:0 13px;font-size:9px}

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
  background: var(--terminal-bg);
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
