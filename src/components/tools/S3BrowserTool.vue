<script setup lang="ts">
import { computed, ref } from "vue";
import {
  s3ListBuckets, s3ListObjects, s3GetObject,
  s3PutObject, s3DeleteObject, s3PresignedUrl,
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
const objects = ref<S3Object[]>([]);
const currentPrefix = ref("");

const previewKey = ref("");
const previewContent = ref("");
const previewing = ref(false);

const uploadKey = ref("");
const uploadData = ref("");
const uploading = ref(false);

const presignedResult = ref("");
const isCos = computed(() =>
  config.value.endpoint.toLowerCase().includes(".myqcloud.com"),
);

async function connect() {
  loading.value = true;
  error.value = "";
  try {
    if (config.value.bucket) {
      objects.value = await s3ListObjects(config.value);
    } else {
      buckets.value = await s3ListBuckets(config.value);
    }
    connected.value = true;
  } catch (e: any) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function selectBucket(name: string) {
  config.value.bucket = name;
  buckets.value = [];
  connect();
}

async function refreshObjects() {
  if (!config.value.bucket) return;
  loading.value = true;
  error.value = "";
  try {
    objects.value = await s3ListObjects(config.value, currentPrefix.value || undefined);
  } catch (e: any) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function viewObject(key: string) {
  previewing.value = true;
  error.value = "";
  try {
    const result = await s3GetObject(config.value, key);
    previewKey.value = key;
    // Try to decode base64 as text
    try {
      previewContent.value = atob(result.data);
    } catch {
      previewContent.value = `[二进制内容，${formatBytes(result.size)}]`;
    }
  } catch (e: any) {
    error.value = String(e);
  } finally {
    previewing.value = false;
  }
}

async function uploadFile() {
  if (!uploadKey.value || !uploadData.value) return;
  uploading.value = true;
  error.value = "";
  try {
    await s3PutObject(config.value, uploadKey.value, uploadData.value);
    uploadKey.value = "";
    uploadData.value = "";
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
  objects.value = [];
  previewKey.value = "";
  previewContent.value = "";
}

const presets = [
  { label: "阿里云 OSS", endpoint: "https://oss-cn-hangzhou.aliyuncs.com", region: "oss-cn-hangzhou", pathStyle: true },
  { label: "腾讯云 COS", endpoint: "https://cos.ap-guangzhou.myqcloud.com", region: "ap-guangzhou", pathStyle: false },
  { label: "七牛云 Kodo", endpoint: "https://s3-cn-east-1.qiniucs.com", region: "cn-east-1", pathStyle: true },
  { label: "AWS S3", endpoint: "https://s3.amazonaws.com", region: "us-east-1", pathStyle: false },
  { label: "MinIO", endpoint: "http://127.0.0.1:9000", region: "us-east-1", pathStyle: true },
];

function applyPreset(p: typeof presets[0]) {
  config.value.endpoint = p.endpoint;
  config.value.region = p.region;
  config.value.pathStyle = p.pathStyle;
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
          <span>Bucket{{ isCos ? "（必填，包含 APPID）" : "（可选）" }}</span>
          <input
            v-model="config.bucket"
            type="text"
            :placeholder="isCos ? '例如 bucket-name-123456' : '留空则先列出 Bucket'"
            spellcheck="false"
          />
        </label>
        <label class="checkbox-label">
          <input v-model="config.pathStyle" type="checkbox" :disabled="isCos" />
          <span>{{ isCos ? "COS 使用虚拟主机域名" : "路径风格 (Path-Style)" }}</span>
        </label>
        <button
          v-if="!connected"
          class="primary connect-btn"
          type="button"
          :disabled="
            loading ||
            !config.endpoint ||
            !config.accessKey ||
            !config.secretKey ||
            (isCos && !config.bucket)
          "
          @click="connect"
        >
          {{ loading ? "连接中…" : "连接" }}
        </button>
        <button v-else class="quiet-danger connect-btn" type="button" @click="disconnect">
          断开
        </button>
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
              <p>BUCKET · {{ config.bucket }}</p>
              <h2>对象列表</h2>
            </div>
            <button type="button" @click="refreshObjects" :disabled="loading">刷新</button>
          </div>

          <div v-if="loading" class="panel-state">加载中…</div>
          <div v-else-if="objects.length === 0" class="panel-state empty">
            <span class="empty-symbol">&#x2610;</span>
            <strong>暂无对象</strong>
            <small>该 Bucket 下没有文件，或所有文件在当前前缀之外</small>
          </div>
          <div v-else class="s3-table">
            <div class="s3-row s3-row-head">
              <span>Key</span>
              <span>大小</span>
              <span>修改时间</span>
              <span>操作</span>
            </div>
            <div v-for="obj in objects" :key="obj.key" class="s3-row">
              <span class="s3-key">{{ obj.key }}</span>
              <span>{{ formatBytes(obj.size) }}</span>
              <span>{{ obj.lastModified.slice(0, 10) }}</span>
              <span class="s3-actions">
                <button type="button" class="clip-btn" :disabled="previewing" @click="viewObject(obj.key)">查看</button>
                <button type="button" class="clip-btn" @click="generatePresigned(obj.key)">预签名</button>
                <button type="button" class="clip-btn remove" @click="deleteObject(obj.key)">删除</button>
              </span>
            </div>
          </div>

          <!-- Upload section -->
          <div class="s3-upload">
            <p>UPLOAD</p>
            <div class="s3-upload-row">
              <input v-model="uploadKey" type="text" placeholder="对象 Key（路径）" spellcheck="false" />
              <textarea v-model="uploadData" placeholder="文件内容…" rows="3" spellcheck="false"></textarea>
              <button type="button" class="primary" :disabled="uploading || !uploadKey || !uploadData" @click="uploadFile">
                {{ uploading ? "上传中…" : "上传" }}
              </button>
            </div>
          </div>
        </div>

        <!-- Preview -->
        <div v-if="previewKey" class="s3-preview">
          <div class="s3-toolbar">
            <h2>{{ previewKey }}</h2>
            <button type="button" @click="previewKey = ''; previewContent = ''">×</button>
          </div>
          <pre><code>{{ previewContent }}</code></pre>
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

.s3-sidebar-body input,
.s3-upload input,
.s3-upload textarea {
  width: 100%;
  padding: 6px 9px;
  border: 1px solid #c8c7bf;
  outline: 0;
  background: #fffefa;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
  color: #353830;
}

.s3-sidebar-body input:focus,
.s3-upload input:focus,
.s3-upload textarea:focus {
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

.s3-toolbar button {
  padding: 4px 12px;
  border: 1px solid #d2d1c9;
  background: #faf9f5;
  color: #73766d;
  font-size: 9px;
  cursor: pointer;
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

.s3-key {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
  color: #353830;
}

.s3-actions {
  display: flex;
  gap: 4px;
}

.s3-upload {
  margin: 16px;
  padding: 14px 16px;
  border: 1px solid #d2d1c9;
  background: #faf9f5;
}

.s3-upload p {
  margin: 0 0 10px;
  color: #989a93;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.12em;
}

.s3-upload-row {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.s3-upload-row textarea {
  resize: vertical;
  min-height: 60px;
}

.s3-upload-row button {
  align-self: flex-start;
  padding: 6px 16px;
  border: 1px solid #252920;
  background: #252920;
  color: white;
  font-size: 9px;
  cursor: pointer;
}

.s3-preview {
  margin: 16px;
  border: 1px solid #d2d1c9;
}

.s3-preview pre {
  margin: 0;
  padding: 14px;
  overflow: auto;
  max-height: 400px;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
  line-height: 1.5;
  color: #353830;
  background: #fffefa;
  white-space: pre-wrap;
  word-break: break-all;
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
