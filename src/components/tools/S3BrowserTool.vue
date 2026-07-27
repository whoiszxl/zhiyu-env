<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import {
  s3DeleteObject,
  s3ConfigGet,
  s3ConfigSave,
  s3GetObject,
  s3ListBuckets,
  s3ListObjects,
  s3PresignedUrl,
  s3PutFile,
} from "../../api/tools";
import type { S3Config, S3Bucket, S3Object } from "../../types";
import { formatBytes } from "../../utils/format";

const config = ref<S3Config>({
  endpoint: "https://oss-cn-hangzhou.aliyuncs.com",
  accessKey: "",
  secretKey: "",
  region: "oss-cn-hangzhou",
  bucket: "",
  pathStyle: true,
});

const connected = ref(false);
const error = ref("");
const loading = ref(false);

const buckets = ref<S3Bucket[]>([]);
const folders = ref<string[]>([]);
const objects = ref<S3Object[]>([]);
const currentPrefix = ref("");
const nextContinuationToken = ref<string | null>(null);
const pageTokens = ref<string[]>([""]);
const pageIndex = ref(0);

const previewKey = ref("");
const previewContent = ref("");
const previewUrl = ref("");
const previewContentType = ref("");
const previewing = ref(false);

const uploading = ref(false);
const selectedPreset = ref("");
const connectionHistory = ref<Array<S3Config & { label: string; provider: string; lastUsedAt: number }>>([]);
const HISTORY_STORAGE_KEY = "zhiyu-env.s3.connection-history";

const presignedResult = ref("");
const isCos = computed(() =>
  config.value.endpoint.toLowerCase().includes(".myqcloud.com"),
);
const isVirtualHostOnly = computed(() => {
  const endpoint = config.value.endpoint.toLowerCase();
  return isCos.value || endpoint.includes(".aliyuncs.com");
});

function detectPathStyle(endpoint: string, fallback = true) {
  const value = endpoint.toLowerCase();
  if (value.includes(".myqcloud.com") || value.includes(".aliyuncs.com")) return false;
  if (value.includes("amazonaws.com")) return false;
  if (value.includes("qiniucs.com") || value.includes("127.0.0.1") || value.includes("localhost")) {
    return true;
  }
  return fallback;
}

const addressingMode = computed(() =>
  config.value.pathStyle ? "Path-Style" : "Virtual Host",
);
watch(
  () => config.value.endpoint,
  () => {
    config.value.pathStyle = detectPathStyle(config.value.endpoint, config.value.pathStyle);
  },
);
const previewKind = computed<
  "audio" | "video" | "image" | "pdf" | "text" | "download"
>(() => {
  const extension = previewKey.value.split(".").pop()?.toLowerCase() ?? "";
  if (["mp3", "wav", "ogg", "m4a", "aac", "flac"].includes(extension)) {
    return "audio";
  }
  if (["mp4", "webm", "mov", "m4v", "ogv"].includes(extension)) {
    return "video";
  }
  if (["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp"].includes(extension)) {
    return "image";
  }
  if (extension === "pdf") return "pdf";
  if (
    previewContentType.value.startsWith("text/") ||
    [
      "txt", "md", "json", "xml", "yaml", "yml", "toml", "ini", "csv",
      "log", "js", "ts", "vue", "css", "html", "java", "go", "rs", "py",
      "sh", "sql", "properties",
    ].includes(extension)
  ) {
    return "text";
  }
  return "download";
});

onMounted(async () => {
  try {
    const rawHistory = localStorage.getItem(HISTORY_STORAGE_KEY);
    if (rawHistory) {
      const parsed = JSON.parse(rawHistory);
      if (Array.isArray(parsed)) connectionHistory.value = parsed.slice(0, 8);
    }
  } catch {
    connectionHistory.value = [];
  }
  try {
    const saved = await s3ConfigGet();
    if (saved) {
      config.value = saved;
      config.value.pathStyle = detectPathStyle(saved.endpoint, saved.pathStyle);
      const matchingPreset = presets.find(
        (preset) =>
          preset.endpoint === saved.endpoint &&
          preset.region === saved.region &&
          preset.pathStyle === saved.pathStyle,
      );
      if (matchingPreset) selectedPreset.value = matchingPreset.label;
      if (saved && connectionHistory.value.length === 0) saveConnectionHistory();
    }
  } catch {
    // 本地配置不可用时保留默认配置，连接时再提示用户。
  }
});
const breadcrumbs = computed(() => {
  const crumbs = [{ label: "根目录", prefix: "" }];
  let prefix = "";
  for (const part of currentPrefix.value.split("/").filter(Boolean)) {
    prefix += `${part}/`;
    crumbs.push({ label: part, prefix });
  }
  return crumbs;
});

function resetPagination() {
  pageTokens.value = [""];
  pageIndex.value = 0;
  nextContinuationToken.value = null;
}

function folderName(prefix: string) {
  return prefix
    .slice(currentPrefix.value.length)
    .replace(/\/$/, "");
}

function objectName(key: string) {
  return key.slice(currentPrefix.value.length);
}

function fileTypeLabel(key: string) {
  const name = key.split("/").pop() ?? key;
  const extension = name.includes(".") ? name.split(".").pop() ?? "" : "";
  return extension ? extension.slice(0, 5).toUpperCase() : "FILE";
}

function historyLabel() {
  return selectedPreset.value || "S3 兼容存储";
}

function saveConnectionHistory() {
  const entry = {
    ...config.value,
    label: historyLabel(),
    provider: selectedPreset.value || "S3 兼容存储",
    lastUsedAt: Date.now(),
  };
  const identity = `${entry.endpoint}|${entry.region}|${entry.bucket}|${entry.accessKey}`;
  connectionHistory.value = [
    entry,
    ...connectionHistory.value.filter(
      (item) => `${item.endpoint}|${item.region}|${item.bucket}|${item.accessKey}` !== identity,
    ),
  ].slice(0, 8);
  try {
    localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify(connectionHistory.value));
  } catch {
    // 本地存储不可用时不影响当前连接。
  }
}

function useConnectionHistory(item: S3Config & { provider?: string }) {
  config.value = {
    endpoint: item.endpoint,
    accessKey: item.accessKey,
    secretKey: item.secretKey,
    region: item.region,
    bucket: item.bucket,
    pathStyle: item.pathStyle,
  };
  selectedPreset.value = item.provider ?? "";
  connected.value = false;
  buckets.value = [];
  folders.value = [];
  objects.value = [];
  error.value = "";
}

function parentPrefix() {
  const parts = currentPrefix.value.split("/").filter(Boolean);
  parts.pop();
  return parts.length ? `${parts.join("/")}/` : "";
}

async function connect() {
  loading.value = true;
  error.value = "";
  try {
    if (config.value.bucket) {
      currentPrefix.value = "";
      resetPagination();
      await loadObjectPage();
    } else {
      buckets.value = await s3ListBuckets(config.value);
    }
    await s3ConfigSave(config.value);
    saveConnectionHistory();
    connected.value = true;
  } catch (e: any) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function selectBucket(name: string) {
  config.value.bucket = name;
  buckets.value = [];
  currentPrefix.value = "";
  resetPagination();
  await loadObjectPage();
  await s3ConfigSave(config.value);
  saveConnectionHistory();
}

async function loadObjectPage(token = pageTokens.value[pageIndex.value] || "") {
  if (!config.value.bucket) return;
  loading.value = true;
  error.value = "";
  try {
    const result = await s3ListObjects(
      config.value,
      currentPrefix.value || undefined,
      token || undefined,
      200,
    );
    folders.value = result.folders;
    objects.value = result.objects.filter(
      (object) => object.key !== currentPrefix.value,
    );
    nextContinuationToken.value = result.nextContinuationToken;
  } catch (e: any) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function refreshObjects() {
  await loadObjectPage();
}

async function enterFolder(prefix: string) {
  currentPrefix.value = prefix;
  resetPagination();
  closePreview();
  await loadObjectPage();
}

async function navigatePrefix(prefix: string) {
  if (prefix === currentPrefix.value) return;
  currentPrefix.value = prefix;
  resetPagination();
  closePreview();
  await loadObjectPage();
}

async function nextPage() {
  if (!nextContinuationToken.value) return;
  const token = nextContinuationToken.value;
  pageTokens.value = pageTokens.value.slice(0, pageIndex.value + 1);
  pageTokens.value.push(token);
  pageIndex.value += 1;
  await loadObjectPage(token);
}

async function previousPage() {
  if (pageIndex.value === 0) return;
  pageIndex.value -= 1;
  await loadObjectPage(pageTokens.value[pageIndex.value]);
}

function decodeBase64Utf8(data: string) {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return new TextDecoder("utf-8", { fatal: false }).decode(bytes);
}

async function viewObject(key: string) {
  previewing.value = true;
  error.value = "";
  previewKey.value = key;
  previewContent.value = "";
  previewUrl.value = "";
  previewContentType.value = "";
  try {
    const extension = key.split(".").pop()?.toLowerCase() ?? "";
    const textLike = [
      "txt", "md", "json", "xml", "yaml", "yml", "toml", "ini", "csv",
      "log", "js", "ts", "vue", "css", "html", "java", "go", "rs", "py",
      "sh", "sql", "properties",
    ].includes(extension);
    if (textLike) {
      const result = await s3GetObject(config.value, key);
      previewContentType.value = result.contentType;
      previewContent.value = decodeBase64Utf8(result.data);
    } else {
      previewUrl.value = (await s3PresignedUrl(config.value, key, 900)).url;
    }
  } catch (e: any) {
    error.value = String(e);
  } finally {
    previewing.value = false;
  }
}

function closePreview() {
  previewKey.value = "";
  previewContent.value = "";
  previewUrl.value = "";
  previewContentType.value = "";
}

async function uploadSelectedFile() {
  const selected = await open({
    multiple: false,
    directory: false,
    title: `上传到 ${currentPrefix.value || "根目录"}`,
  });
  if (typeof selected !== "string") return;

  const fileName = selected.split(/[\\/]/).pop();
  if (!fileName) return;

  uploading.value = true;
  error.value = "";
  try {
    await s3PutFile(config.value, `${currentPrefix.value}${fileName}`, selected);
    await refreshObjects();
  } catch (e: any) {
    error.value = String(e);
  } finally {
    uploading.value = false;
  }
}

async function deleteObject(key: string) {
  if (!confirm(`确定删除 ${key}？`)) return;
  loading.value = true;
  error.value = "";
  try {
    await s3DeleteObject(config.value, key);
    await refreshObjects();
    if (
      folders.value.length === 0 &&
      objects.value.length === 0 &&
      pageIndex.value > 0
    ) {
      await previousPage();
    }
  } catch (e: any) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function generatePresigned(key: string) {
  error.value = "";
  try {
    const result = await s3PresignedUrl(config.value, key);
    presignedResult.value = result.url;
    await navigator.clipboard.writeText(result.url);
  } catch (e: any) {
    error.value = String(e);
  }
}

function disconnect() {
  connected.value = false;
  buckets.value = [];
  folders.value = [];
  objects.value = [];
  currentPrefix.value = "";
  resetPagination();
  closePreview();
}

const presets = [
  { label: "阿里云 OSS", endpoint: "https://oss-cn-hangzhou.aliyuncs.com", region: "oss-cn-hangzhou", pathStyle: false },
  { label: "腾讯云 COS", endpoint: "https://cos.ap-guangzhou.myqcloud.com", region: "ap-guangzhou", pathStyle: false },
  { label: "七牛云 Kodo", endpoint: "https://s3-cn-east-1.qiniucs.com", region: "cn-east-1", pathStyle: true },
  { label: "AWS S3", endpoint: "https://s3.amazonaws.com", region: "us-east-1", pathStyle: false },
  { label: "MinIO", endpoint: "http://127.0.0.1:9000", region: "us-east-1", pathStyle: true },
  { label: "RustFS", endpoint: "http://127.0.0.1:9000", region: "us-east-1", pathStyle: true },
];

function applyPreset(p: typeof presets[0]) {
  config.value.endpoint = p.endpoint;
  config.value.region = p.region;
  config.value.pathStyle = p.pathStyle;
  selectedPreset.value = p.label;
  error.value = "";
}
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo" style="background:#4a6741;font-size:16px;">&#x2610;</span>
      <div>
        <div class="title-line">
          <h1>S3 对象存储浏览器</h1>
          <span>LOCAL ONLY</span>
        </div>
        <p>连接兼容 S3 协议的对象存储，浏览 Bucket、上传下载、生成预签名链接</p>
      </div>
    </div>
  </header>

  <div v-if="error" class="notice danger">
    <span>{{ error }}</span>
    <button type="button" @click="error = ''">×</button>
  </div>

  <div class="s3-layout">
    <!-- Left: connection panel -->
    <aside class="s3-sidebar">
      <div class="s3-sidebar-head">
        <p>CONNECTION</p>
        <h2>连接配置</h2>
      </div>
      <div class="s3-sidebar-body">
        <div class="s3-presets">
          <button
            v-for="p in presets"
            :key="p.label"
            type="button"
            class="preset-btn"
            :class="{ selected: selectedPreset === p.label }"
            @click="applyPreset(p)"
          >{{ p.label }}</button>
        </div>
        <label>
          <span>Endpoint</span>
          <input v-model="config.endpoint" type="text" placeholder="https://..." spellcheck="false" />
        </label>
        <label>
          <span>Access Key</span>
          <input v-model="config.accessKey" type="text" placeholder="AccessKeyId" spellcheck="false" />
        </label>
        <label>
          <span>Secret Key</span>
          <input v-model="config.secretKey" type="password" placeholder="SecretAccessKey" spellcheck="false" />
        </label>
        <label>
          <span>Region</span>
          <input v-model="config.region" type="text" placeholder="oss-cn-hangzhou" spellcheck="false" />
        </label>
        <label>
          <span>Bucket{{ isVirtualHostOnly ? "（必填）" : "（可选）" }}</span>
          <input
            v-model="config.bucket"
            type="text"
            :placeholder="isVirtualHostOnly ? '请输入 Bucket' : '留空则先列出 Bucket'"
            spellcheck="false"
          />
        </label>
        <div class="s3-addressing-mode">
          <span>地址模式</span>
          <strong>{{ addressingMode }}</strong>
          <small>根据 Endpoint 自动判断</small>
        </div>
        <button
          v-if="!connected"
          class="primary connect-btn"
          type="button"
          :disabled="
            loading ||
            !config.endpoint ||
            !config.accessKey ||
            !config.secretKey ||
            (isVirtualHostOnly && !config.bucket)
          "
          @click="connect"
        >
          {{ loading ? "连接中…" : "连接" }}
        </button>
        <button v-else class="quiet-danger connect-btn" type="button" @click="disconnect">
          断开
        </button>
        <div v-if="connectionHistory.length" class="s3-history">
          <p>最近连接</p>
          <button
            v-for="item in connectionHistory"
            :key="`${item.endpoint}-${item.bucket}-${item.lastUsedAt}`"
            type="button"
            class="s3-history-item"
            @click="useConnectionHistory(item)"
          >
            <strong>{{ item.label }}</strong>
            <small>{{ item.endpoint }}</small>
          </button>
        </div>
      </div>
    </aside>

    <!-- Right: browser -->
    <section class="s3-main">
      <div v-if="!connected" class="panel-state">
        <span class="empty-symbol">&#x2610;</span>
        <strong>未连接</strong>
        <small>在左侧填入 Endpoint、Access Key 和 Secret Key 后点击连接</small>
      </div>

      <template v-else>
        <!-- Bucket list -->
        <div v-if="buckets.length > 0" class="s3-list">
          <div class="s3-toolbar">
            <div>
              <p>BUCKETS</p>
              <h2>选择 Bucket</h2>
            </div>
          </div>
          <div class="s3-table">
            <div class="s3-row s3-row-head">
              <span>Bucket 名称</span>
              <span>创建时间</span>
            </div>
            <div
              v-for="b in buckets"
              :key="b.name"
              class="s3-row clickable"
              @click="selectBucket(b.name)"
            >{{ b.name }}</div>
          </div>
        </div>

        <!-- Object list -->
        <div v-if="config.bucket" class="s3-list">
          <div class="s3-toolbar">
            <div>
              <p>OBJECTS</p>
              <div class="s3-breadcrumbs">
                <template
                  v-for="(crumb, index) in breadcrumbs"
                  :key="crumb.prefix"
                >
                  <span v-if="index > 0">/</span>
                  <button
                    type="button"
                    :disabled="crumb.prefix === currentPrefix"
                    @click="navigatePrefix(crumb.prefix)"
                  >
                    {{ crumb.label }}
                  </button>
                </template>
              </div>
            </div>
            <div class="s3-toolbar-actions">
              <button type="button" @click="refreshObjects" :disabled="loading">刷新</button>
              <button type="button" class="primary" @click="uploadSelectedFile" :disabled="uploading || loading">
                {{ uploading ? "上传中…" : "上传" }}
              </button>
            </div>
          </div>

          <div v-if="loading" class="panel-state">加载中…</div>
          <div
            v-else-if="folders.length === 0 && objects.length === 0"
            class="panel-state empty"
          >
            <span class="empty-symbol">&#x2610;</span>
            <strong>暂无对象</strong>
            <small>当前目录下没有子目录或文件</small>
          </div>
          <div v-else class="s3-table">
            <div class="s3-row s3-row-head">
              <span>名称</span>
              <span>大小</span>
              <span>修改时间</span>
              <span>操作</span>
            </div>
            <button
              v-if="currentPrefix"
              type="button"
              class="s3-row s3-folder-row"
              @click="navigatePrefix(parentPrefix())"
            >
              <span class="s3-key">📁 ..</span>
              <span>—</span>
              <span>—</span>
              <span></span>
            </button>
            <button
              v-for="folder in folders"
              :key="folder"
              type="button"
              class="s3-row s3-folder-row"
              @click="enterFolder(folder)"
            >
              <span class="s3-key">📁 {{ folderName(folder) }}</span>
              <span>—</span>
              <span>目录</span>
              <span></span>
            </button>
            <div v-for="obj in objects" :key="obj.key" class="s3-row">
              <span class="s3-key" :title="obj.key">
                <span class="s3-file-badge" aria-hidden="true">{{ fileTypeLabel(obj.key) }}</span>
                <span class="s3-key-name">{{ objectName(obj.key) }}</span>
              </span>
              <span>{{ formatBytes(obj.size) }}</span>
              <span>{{ obj.lastModified.slice(0, 10) }}</span>
              <span class="s3-actions">
                <button type="button" class="clip-btn" :disabled="previewing" @click="viewObject(obj.key)">查看</button>
                <button type="button" class="clip-btn" @click="generatePresigned(obj.key)">预签名</button>
                <button type="button" class="clip-btn remove" @click="deleteObject(obj.key)">删除</button>
              </span>
            </div>
          </div>

          <div class="s3-pagination">
            <span>第 {{ pageIndex + 1 }} 页 · 每页最多 200 项</span>
            <div>
              <button
                type="button"
                :disabled="loading || pageIndex === 0"
                @click="previousPage"
              >
                上一页
              </button>
              <button
                type="button"
                :disabled="loading || !nextContinuationToken"
                @click="nextPage"
              >
                下一页
              </button>
            </div>
          </div>

        </div>

        <!-- Preview -->
        <div
          v-if="previewKey"
          class="s3-preview-modal"
          role="dialog"
          aria-modal="true"
          :aria-label="`预览 ${previewKey}`"
          tabindex="-1"
          @click.self="closePreview"
          @keydown.esc="closePreview"
        >
          <div class="s3-preview-dialog">
            <div class="s3-toolbar">
              <div>
                <p>PREVIEW</p>
                <h2 :title="previewKey">{{ previewKey }}</h2>
              </div>
              <button type="button" aria-label="关闭预览" @click="closePreview">×</button>
            </div>
            <div v-if="previewing" class="panel-state">正在准备预览…</div>
            <pre v-else-if="previewKind === 'text'"><code>{{ previewContent }}</code></pre>
            <audio
              v-else-if="previewKind === 'audio' && previewUrl"
              :src="previewUrl"
              controls
              preload="metadata"
            ></audio>
            <video
              v-else-if="previewKind === 'video' && previewUrl"
              :src="previewUrl"
              controls
              preload="metadata"
            ></video>
            <img
              v-else-if="previewKind === 'image' && previewUrl"
              :src="previewUrl"
              alt="对象预览"
            />
            <iframe
              v-else-if="previewKind === 'pdf' && previewUrl"
              :src="previewUrl"
              title="PDF 预览"
            ></iframe>
            <div v-else class="s3-binary-preview">
              <p>该文件类型不支持直接预览，可通过临时链接打开或下载。</p>
              <a v-if="previewUrl" :href="previewUrl" target="_blank">打开临时链接</a>
            </div>
          </div>
        </div>

        <!-- Presigned notice -->
        <div v-if="presignedResult" class="notice info">
          <span>预签名 URL 已复制到剪贴板</span>
          <button type="button" @click="presignedResult = ''">×</button>
        </div>
      </template>
    </section>
  </div>
</template>

<style scoped>
.s3-layout {
  display: grid;
  grid-template-columns: 260px minmax(0, 1fr);
  height: calc(100vh - 88px);
  border-top: 1px solid #d2d1c9;
}

.s3-sidebar {
  display: flex;
  flex-direction: column;
  border-right: 1px solid #d2d1c9;
  background: rgba(250, 249, 245, 0.84);
  overflow-y: auto;
}

.s3-sidebar-head {
  padding: 17px 16px 15px;
  border-bottom: 1px solid #d2d1c9;
}

.s3-sidebar-head p,
.s3-toolbar p {
  margin: 0 0 4px;
  color: #989a93;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.12em;
}

.s3-sidebar-head h2,
.s3-toolbar h2 {
  margin: 0;
  font-size: 14px;
  color: #252920;
}

.s3-sidebar-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px 16px;
}

.s3-sidebar-body label {
  display: flex;
  flex-direction: column;
  gap: 5px;
}

.s3-sidebar-body label > span {
  color: #73766d;
  font-size: 9px;
}

.s3-addressing-mode {
  display: flex;
  align-items: baseline;
  gap: 7px;
  color: #73766d;
  font-size: 9px;
}

.s3-addressing-mode strong {
  color: #315c3a;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
}

.s3-addressing-mode small {
  color: #a0a299;
  font-size: 8px;
}

.s3-sidebar-body input {
  width: 100%;
  padding: 6px 9px;
  border: 1px solid #c8c7bf;
  outline: 0;
  background: #fffefa;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
  color: #353830;
}

.s3-sidebar-body input:focus {
  border-color: #777a70;
  box-shadow: 0 0 0 2px rgba(77, 81, 69, 0.08);
}

.s3-presets {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-bottom: 4px;
}

.preset-btn {
  padding: 3px 8px;
  border: 1px solid #d2d1c9;
  background: #faf9f5;
  color: #73766d;
  font-size: 8px;
  cursor: pointer;
}

.preset-btn:hover {
  border-color: #898b83;
  color: #252920;
}

.preset-btn.selected {
  border-color: #5d795f;
  background: #e6eee4;
  color: #315c3a;
  font-weight: 600;
}

.s3-history {
  display: flex;
  flex-direction: column;
  gap: 5px;
  padding-top: 3px;
  border-top: 1px solid #e4e2da;
}

.s3-history > p {
  margin: 0 0 2px;
  color: #989a93;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.12em;
}

.s3-history-item {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 2px;
  padding: 6px 8px;
  border: 1px solid #deddd5;
  background: #f7f6f1;
  color: #55584f;
  text-align: left;
  cursor: pointer;
}

.s3-history-item:hover {
  border-color: #9bb39b;
  background: #edf3ea;
}

.s3-history-item strong,
.s3-history-item small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.s3-history-item strong { font-size: 9px; }
.s3-history-item small { color: #96988f; font-size: 8px; }

.connect-btn {
  width: 100%;
  height: 34px;
  margin-top: 4px;
}

.s3-main {
  min-height: 0;
  overflow-y: auto;
  background: rgba(250, 249, 245, 0.6);
}

.s3-toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 13px 16px;
  border-bottom: 1px solid #d2d1c9;
  background: rgba(250, 249, 245, 0.9);
}

.s3-toolbar h2 {
  font-size: 13px;
}

.s3-breadcrumbs {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 5px;
}

.s3-breadcrumbs > span {
  color: #a1a39b;
  font-size: 10px;
}

.s3-breadcrumbs > button {
  padding: 0;
  border: 0;
  background: transparent;
  color: #4f7051;
  font: 600 11px/1.4 "SFMono-Regular", Consolas, monospace;
}

.s3-breadcrumbs > button:disabled {
  color: #252920;
}

.s3-toolbar button {
  padding: 4px 12px;
  border: 1px solid #d2d1c9;
  background: #faf9f5;
  color: #73766d;
  font-size: 9px;
  cursor: pointer;
}

.s3-toolbar-actions {
  display: flex;
  align-items: center;
  gap: 6px;
}

.s3-toolbar-actions button.primary {
  border-color: #526b54;
  background: #526b54;
  color: #fff;
}

.s3-table {
  font-size: 11px;
}

.s3-row {
  display: grid;
  grid-template-columns: minmax(0, 1.5fr) 100px 120px 160px;
  gap: 8px;
  align-items: center;
  padding: 8px 16px;
  border-bottom: 1px solid #e8e6df;
}

.s3-row-head {
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  color: #989a93;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  padding: 6px 16px;
  background: #f5f3ee;
}

.s3-row.clickable {
  cursor: pointer;
}

.s3-row.clickable:hover {
  background: #f0eee7;
}

.s3-folder-row {
  width: 100%;
  border: 0;
  border-bottom: 1px solid #e8e6df;
  background: rgba(244, 242, 235, 0.68);
  color: #73766d;
  text-align: left;
}

.s3-folder-row:hover {
  background: #eceae2;
}

.s3-key {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 6px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
  color: #353830;
}

.s3-file-badge {
  display: inline-grid;
  flex: 0 0 auto;
  width: 28px;
  height: 18px;
  place-items: center;
  border: 1px solid #c6c9bc;
  border-radius: 4px;
  background: #eef1e9;
  color: #5c765e;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 7px;
  font-weight: 700;
  letter-spacing: 0.04em;
}

.s3-key-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.s3-actions {
  display: flex;
  gap: 4px;
}

.s3-pagination {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 16px;
  border-bottom: 1px solid #d2d1c9;
  color: #8a8d84;
  font-size: 9px;
}

.s3-pagination > div {
  display: flex;
  gap: 6px;
}

.s3-pagination button {
  padding: 5px 10px;
  border: 1px solid #d2d1c9;
  background: #faf9f5;
  color: #55584f;
  font-size: 9px;
}

.s3-preview-modal {
  position: fixed;
  z-index: 1000;
  inset: 0;
  display: grid;
  padding: 32px;
  place-items: center;
  background: rgba(24, 27, 22, 0.52);
  backdrop-filter: blur(3px);
}

.s3-preview-dialog {
  display: flex;
  width: min(920px, 100%);
  height: min(680px, calc(100vh - 64px));
  max-height: min(760px, calc(100vh - 64px));
  flex-direction: column;
  overflow: hidden;
  border: 1px solid #b9b8ae;
  border-radius: 10px;
  background: #faf9f5;
  box-shadow: 0 24px 80px rgba(20, 23, 18, 0.28);
}

.s3-preview-dialog .s3-toolbar {
  flex: 0 0 auto;
}

.s3-preview-dialog .s3-toolbar > div {
  min-width: 0;
}

.s3-preview-dialog .s3-toolbar p {
  margin: 0 0 3px;
}

.s3-preview-dialog .s3-toolbar h2 {
  max-width: min(720px, 70vw);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.s3-preview-dialog pre {
  margin: 0;
  padding: 14px;
  overflow: auto;
  min-height: 0;
  flex: 1 1 auto;
  max-height: none;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
  line-height: 1.5;
  color: #353830;
  background: #fffefa;
  white-space: pre-wrap;
  word-break: break-all;
}

.s3-preview-dialog audio {
  display: block;
  width: calc(100% - 32px);
  margin: 22px 16px;
}

.s3-preview-dialog video {
  display: block;
  width: 100%;
  max-height: 560px;
  background: #171914;
}

.s3-preview-dialog img {
  display: block;
  max-width: calc(100% - 32px);
  max-height: 560px;
  margin: 16px auto;
  object-fit: contain;
}

.s3-preview-dialog iframe {
  width: 100%;
  flex: 1 1 auto;
  min-height: 0;
  height: auto;
  border: 0;
}

.s3-preview-dialog > .panel-state,
.s3-preview-dialog > .s3-binary-preview {
  flex: 1 1 auto;
  min-height: 0;
}

.s3-binary-preview {
  display: grid;
  min-height: 160px;
  place-items: center;
  align-content: center;
  gap: 12px;
  color: #73766d;
  font-size: 10px;
}

.s3-binary-preview a {
  color: #416b49;
  font-weight: 600;
}

@media (max-width: 700px) {
  .s3-preview-modal { padding: 12px; }
  .s3-preview-dialog {
    height: calc(100vh - 24px);
    max-height: calc(100vh - 24px);
  }
  .s3-preview-dialog iframe { height: min(620px, calc(100vh - 150px)); }
}

.panel-state {
  display: grid;
  min-height: 300px;
  place-items: center;
  align-content: center;
  gap: 7px;
  color: #92948c;
  font-size: 11px;
}
.panel-state strong { color: #55584f; font-size: 12px; }
.panel-state small { color: #979990; font-size: 9px; }
.empty-symbol {
  display: grid;
  width: 48px; height: 48px;
  margin-bottom: 4px;
  place-items: center;
  border: 1px solid #cbc9c0;
  border-radius: 50%;
  background: #efede6;
  color: #777970;
  font-size: 20px;
}

.info {
  background: #e5efe4;
  border: 1px solid #91b39a;
  color: #2f7047;
}
</style>
