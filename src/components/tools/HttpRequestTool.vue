<script setup lang="ts">
import { computed, ref } from "vue";
import { executeHttpRequest } from "../../api/tools";
import type { HttpHeader, HttpResponseOutput } from "../../types";

const method = ref("GET");
const url = ref("http://127.0.0.1:9321/api/hello");
const headers = ref<HttpHeader[]>([{ name: "", value: "" }]);
const body = ref("");
const timeoutSeconds = ref(15);
const followRedirects = ref(true);
const requestTab = ref<"headers" | "body">("headers");
const responseTab = ref<"body" | "headers">("body");
const response = ref<HttpResponseOutput | null>(null);
const sending = ref(false);
const error = ref("");
const copied = ref("");

const formattedBody = computed(() => {
  if (!response.value) return "";
  if (response.value.contentType.includes("json")) {
    try {
      return JSON.stringify(JSON.parse(response.value.body), null, 2);
    } catch {
      return response.value.body;
    }
  }
  return response.value.body;
});

function addHeader() {
  headers.value.push({ name: "", value: "" });
}

function removeHeader(index: number) {
  headers.value.splice(index, 1);
  if (headers.value.length === 0) addHeader();
}

async function send() {
  if (sending.value) return;
  sending.value = true;
  error.value = "";
  try {
    response.value = await executeHttpRequest({
      method: method.value,
      url: url.value.trim(),
      headers: headers.value.filter((header) => header.name.trim()),
      body: body.value,
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

function shellQuote(value: string) {
  return `'${value.replaceAll("'", "'\\''")}'`;
}

async function copyCurl() {
  const parts = ["curl", "-X", method.value, shellQuote(url.value.trim())];
  for (const header of headers.value.filter((item) => item.name.trim())) {
    parts.push("-H", shellQuote(`${header.name}: ${header.value}`));
  }
  if (body.value && !["GET", "HEAD"].includes(method.value)) {
    parts.push("--data-raw", shellQuote(body.value));
  }
  await navigator.clipboard.writeText(parts.join(" "));
  copied.value = "cURL 已复制";
  window.setTimeout(() => (copied.value = ""), 1200);
}

async function copyResponse() {
  if (!response.value) return;
  await navigator.clipboard.writeText(formattedBody.value);
  copied.value = "响应已复制";
  window.setTimeout(() => (copied.value = ""), 1200);
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(1)} KiB`;
  return `${(value / 1024 / 1024).toFixed(1)} MiB`;
}
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo http">H</span>
      <div>
        <div class="title-line"><h1>HTTP 请求调试器</h1><span>REST CLIENT</span></div>
        <p>发送本地或远程 HTTP 请求，查看状态、响应头和响应内容</p>
      </div>
    </div>
    <div class="header-actions"><button type="button" @click="copyCurl">{{ copied || "复制 cURL" }}</button></div>
  </header>

  <div v-if="error" class="notice danger"><span>{{ error }}</span><button type="button" @click="error = ''">×</button></div>

  <section class="http-page">
    <div class="request-line">
      <select v-model="method" aria-label="请求方法"><option v-for="item in ['GET','POST','PUT','PATCH','DELETE','HEAD','OPTIONS']" :key="item">{{ item }}</option></select>
      <input v-model="url" type="url" placeholder="http://127.0.0.1:9321/api/hello" @keyup.enter="send" />
      <button class="primary send-button" type="button" :disabled="sending" @click="send"><span v-if="sending" class="spinner"></span>{{ sending ? "发送中" : "发送请求" }}</button>
    </div>

    <div class="http-layout">
      <article class="http-panel request-panel">
        <div class="tabbar">
          <div class="tab-group">
            <button :class="{ active: requestTab === 'headers' }" type="button" @click="requestTab = 'headers'">请求头 <small>{{ headers.filter((item) => item.name.trim()).length }}</small></button>
            <button :class="{ active: requestTab === 'body' }" type="button" @click="requestTab = 'body'">请求体</button>
          </div>
          <span></span>
          <div class="request-options">
            <label>超时 <input v-model.number="timeoutSeconds" type="number" min="1" max="120" /> 秒</label>
            <label><input v-model="followRedirects" type="checkbox" /> 跟随重定向</label>
          </div>
        </div>
        <div v-if="requestTab === 'headers'" class="header-editor">
          <div class="header-columns"><span>名称</span><span>值</span><span></span></div>
          <div v-for="(header, index) in headers" :key="index" class="header-row">
            <input v-model="header.name" placeholder="Header 名称" />
            <input v-model="header.value" placeholder="Header 值" />
            <button type="button" title="删除" @click="removeHeader(index)">×</button>
          </div>
          <button type="button" class="add-row" @click="addHeader">＋ 添加请求头</button>
        </div>
        <div v-else class="body-editor">
          <textarea v-model="body" spellcheck="false" placeholder="{&#10;  &quot;name&quot;: &quot;Zhiyu&quot;&#10;}"></textarea>
          <small>GET 和 HEAD 请求不会发送请求体；单次请求体最大 2 MiB。</small>
        </div>
      </article>

      <article class="http-panel response-panel">
        <div class="response-heading">
          <div><p>RESPONSE</p><h2>响应结果</h2></div>
          <div v-if="response" class="response-metrics">
            <strong :class="{ failed: response.statusCode >= 400 }">{{ response.statusCode }} {{ response.statusText }}</strong>
            <span>{{ response.elapsedMs }} ms</span><span>{{ formatBytes(response.sizeBytes) }}</span>
          </div>
        </div>
        <div v-if="response" class="tabbar">
          <div class="tab-group">
            <button :class="{ active: responseTab === 'body' }" type="button" @click="responseTab = 'body'">响应体</button>
            <button :class="{ active: responseTab === 'headers' }" type="button" @click="responseTab = 'headers'">响应头 <small>{{ response.headers.length }}</small></button>
          </div>
          <span></span>
          <button class="copy-response" type="button" @click="copyResponse">复制响应</button>
        </div>
        <div v-if="sending" class="response-empty"><span class="spinner"></span> 正在等待服务器响应…</div>
        <div v-else-if="!response" class="response-empty">填写请求地址后点击“发送请求”<small>可直接启动“本地 Mock API”，测试默认示例地址</small></div>
        <template v-else>
          <pre v-if="responseTab === 'body'" class="response-body">{{ formattedBody }}</pre>
          <div v-else class="response-headers">
            <div v-for="(header, index) in response.headers" :key="`${header.name}-${index}`"><code>{{ header.name }}</code><span>{{ header.value }}</span></div>
          </div>
          <div v-if="response.truncated" class="truncated-note">响应超过 2 MiB，仅展示前 2 MiB，避免占用过多内存。</div>
        </template>
      </article>
    </div>
  </section>
</template>

<style scoped>
.http-page {
  display: grid;
  gap: 14px;
  padding: 24px 32px 36px;
}

.request-line {
  display: grid;
  grid-template-columns: 108px minmax(0, 1fr) 118px;
  gap: 0;
  overflow: hidden;
  border: 1px solid var(--color-border-strong);
  background: var(--color-bg-elevated);
}

.request-line select,
.request-line input,
.request-line button {
  height: 42px;
  min-height: 42px;
  border: 0;
  border-radius: 0;
}

.request-line select {
  padding: 0 12px;
  border-right: 1px solid var(--color-border);
  background: var(--color-bg-muted);
  color: var(--color-accent);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 11px;
  font-weight: 700;
}

.request-line input {
  min-width: 0;
  padding: 0 14px;
  outline: 0;
  background: transparent;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 11px;
}

.request-line input:focus {
  box-shadow: inset 0 -2px var(--color-accent);
}

.send-button {
  justify-content: center;
}

.http-layout {
  display: grid;
  gap: 14px;
}

.http-panel {
  min-width: 0;
  overflow: hidden;
  border: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

.tabbar {
  display: flex;
  min-height: 52px;
  align-items: stretch;
  gap: 6px;
  padding: 0 14px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-muted);
}

.tabbar > span {
  flex: 1;
}

.tab-group {
  display: flex;
  align-items: stretch;
  gap: 2px;
}

.tab-group button {
  min-width: 76px;
  padding: 0 12px;
  border: 0;
  background: transparent;
  color: var(--color-text-secondary);
  font-size: 10px;
  white-space: nowrap;
}

.tab-group button.active {
  background: var(--color-bg-panel);
  color: var(--color-text-primary);
  box-shadow: inset 0 -2px var(--color-accent);
}

.tab-group small {
  display: inline-grid;
  min-width: 17px;
  height: 17px;
  margin-left: 4px;
  place-items: center;
  border-radius: 9px;
  background: var(--color-selected);
  color: var(--color-text-muted);
  font-size: 8px;
}

.request-options {
  display: flex;
  align-items: center;
  gap: 16px;
}

.request-options label {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--color-text-muted);
  font-size: 9px;
  white-space: nowrap;
}

.request-options input[type="number"] {
  width: 50px;
  height: 29px;
  min-height: 29px;
  padding: 0 6px;
  font-size: 9px;
}

.request-options input[type="checkbox"] {
  width: 13px;
  height: 13px;
  min-height: 0;
}

.header-editor {
  min-height: 154px;
  padding: 13px 16px 16px;
}

.header-columns,
.header-row {
  display: grid;
  grid-template-columns: minmax(160px, 0.7fr) minmax(240px, 1.3fr) 30px;
  gap: 8px;
}

.header-columns {
  padding: 0 8px 6px;
  color: var(--color-text-muted);
  font-size: 8px;
}

.header-row {
  margin-bottom: 7px;
}

.header-row input {
  width: 100%;
  height: 32px;
  min-width: 0;
  padding: 0 9px;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 9px;
}

.header-row button {
  min-height: 32px;
  padding: 0;
  border-color: transparent;
  background: transparent;
  color: var(--color-text-muted);
}

.header-row button:hover {
  border-color: var(--color-border);
  background: var(--color-bg-muted);
  color: var(--color-danger-text);
}

.add-row {
  min-height: 30px;
  margin-top: 2px;
  padding: 0 10px;
  font-size: 9px;
}

.body-editor {
  padding: 14px 16px 16px;
}

.body-editor textarea {
  display: block;
  width: 100%;
  min-height: 170px;
  padding: 13px 14px;
  resize: vertical;
  border-color: #3d4139;
  background: #20231d;
  color: #e9ede5;
  font: 10px/1.65 "SFMono-Regular", Consolas, monospace;
}

.body-editor small {
  display: block;
  margin-top: 8px;
  color: var(--color-text-muted);
  font-size: 8px;
}

.response-heading {
  display: flex;
  min-height: 64px;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 11px 16px;
  border-bottom: 1px solid var(--color-border);
}

.response-heading p {
  margin: 0 0 4px;
  color: var(--color-text-muted);
  font: 8px/1.2 "SFMono-Regular", Consolas, monospace;
  letter-spacing: 0.12em;
}

.response-heading h2 {
  margin: 0;
  font-size: 14px;
}

.response-metrics {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 9px;
}

.response-metrics strong {
  padding: 5px 8px;
  border: 1px solid var(--color-success-text);
  border-radius: 12px;
  color: var(--color-success-text);
  font-size: 9px;
}

.response-metrics strong.failed {
  border-color: var(--color-danger-text);
  color: var(--color-danger-text);
}

.response-metrics span {
  padding-left: 10px;
  border-left: 1px solid var(--color-border);
  color: var(--color-text-muted);
}

.copy-response {
  align-self: center;
  min-height: 30px;
  padding: 0 10px;
  font-size: 9px;
}

.response-empty {
  display: flex;
  min-height: 250px;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--color-text-secondary);
  font-size: 10px;
}

.response-empty small {
  color: var(--color-text-muted);
  font-size: 8px;
}

.response-body {
  min-height: 250px;
  max-height: 430px;
  margin: 0;
  overflow: auto;
  padding: 16px 18px;
  background: #20231d;
  color: #e9ede5;
  font: 10px/1.65 "SFMono-Regular", Consolas, monospace;
  white-space: pre-wrap;
  word-break: break-word;
}

.response-headers {
  min-height: 250px;
  max-height: 430px;
  overflow: auto;
  padding: 8px 18px;
}

.response-headers > div {
  display: grid;
  grid-template-columns: minmax(150px, 0.45fr) minmax(0, 1.55fr);
  gap: 16px;
  padding: 9px 0;
  border-bottom: 1px solid var(--color-border);
  font-size: 9px;
}

.response-headers code {
  color: var(--color-accent);
  font-family: "SFMono-Regular", Consolas, monospace;
}

.response-headers span {
  word-break: break-all;
}

.truncated-note {
  padding: 9px 18px;
  border-top: 1px solid var(--color-border);
  background: var(--color-warning-surface);
  color: var(--color-warning-text);
  font-size: 9px;
}

@media (max-width: 920px) {
  .request-options label:first-child {
    display: none;
  }
}
</style>
