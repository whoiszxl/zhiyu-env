<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  clipboardStart, clipboardStop, clipboardPause, clipboardResume,
  clipboardStatus, clipboardList, clipboardCopy, clipboardPin,
  clipboardDelete, clipboardClear,
  clipboardSettingsGet, clipboardSettingsSave,
} from "../../api/services";
import type { ClipboardItem, ClipboardStatus, ClipboardSettings } from "../../types";
import { formatBytes } from "../../utils/format";

const items = ref<ClipboardItem[]>([]);
const status = ref<ClipboardStatus>({ itemCount: 0, pinnedCount: 0, dbSizeBytes: 0, monitoring: false });
const settings = ref<ClipboardSettings>({ maxItems: 500, retentionDays: 30 });
const search = ref("");
const error = ref("");
const loading = ref(false);
const copiedId = ref(0);
const settingsSaved = ref(false);
let unlisten: UnlistenFn | null = null;

const paused = ref(false);

async function refreshList() {
  loading.value = true;
  try {
    const q = search.value.trim() || undefined;
    items.value = await clipboardList(q, 200, 0);
    error.value = "";
  } catch (e: any) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function refreshStatus() {
  try {
    status.value = await clipboardStatus();
  } catch { /* ignore */ }
}

async function toggleMonitoring() {
  try {
    if (status.value.monitoring) {
      await clipboardStop();
    } else {
      await clipboardStart();
    }
    await refreshStatus();
  } catch (e: any) {
    error.value = String(e);
  }
}

async function togglePause() {
  try {
    if (paused.value) {
      await clipboardResume();
    } else {
      await clipboardPause();
    }
    paused.value = !paused.value;
    await refreshStatus();
  } catch (e: any) {
    error.value = String(e);
  }
}

async function copyItem(item: ClipboardItem) {
  try {
    await clipboardCopy(item.id);
    copiedId.value = item.id;
    setTimeout(() => { copiedId.value = 0; }, 1200);
  } catch (e: any) {
    error.value = String(e);
  }
}

async function pinItem(id: number) {
  try {
    await clipboardPin(id);
    await refreshList();
  } catch (e: any) {
    error.value = String(e);
  }
}

async function deleteItem(id: number) {
  try {
    await clipboardDelete(id);
    await refreshList();
    await refreshStatus();
  } catch (e: any) {
    error.value = String(e);
  }
}

async function clearAll() {
  if (!confirm("清空所有未置顶的剪贴板记录？")) return;
  try {
    await clipboardClear();
    await refreshList();
    await refreshStatus();
  } catch (e: any) {
    error.value = String(e);
  }
}

async function loadSettings() {
  try { settings.value = await clipboardSettingsGet(); } catch { /* defaults */ }
}

async function saveSettings() {
  try {
    await clipboardSettingsSave(settings.value);
    settingsSaved.value = true;
    setTimeout(() => { settingsSaved.value = false; }, 1500);
  } catch (e: any) {
    error.value = String(e);
  }
}

function timeAgo(ms: number): string {
  const diff = Date.now() - ms;
  if (diff < 60_000) return "刚才";
  if (diff < 3600_000) return `${Math.floor(diff / 60_000)} 分钟前`;
  if (diff < 86400_000) return `${Math.floor(diff / 3600_000)} 小时前`;
  return `${Math.floor(diff / 86400_000)} 天前`;
}

const typeBadge = (t: string) => ({ text: "T", code: "{}", url: "URL" } as Record<string, string>)[t] || "T";

onMounted(async () => {
  await loadSettings();
  await refreshStatus();
  unlisten = await listen<ClipboardItem>("clipboard:changed", () => {
    refreshList();
    refreshStatus();
  });
  if (status.value.monitoring) {
    await refreshList();
  }
});

onUnmounted(() => { unlisten?.(); });
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo clipboard-logo">⌘</span>
      <div>
        <div class="title-line">
          <h1>剪贴板历史</h1>
          <span>LOCAL ONLY</span>
        </div>
        <p>
          <span
            class="status-dot"
            :class="{ running: status.monitoring && !paused }"
          ></span>
          本地记录、搜索并快速复用最近复制的文本
        </p>
      </div>
    </div>
    <div class="header-actions">
      <button
        v-if="status.monitoring"
        type="button"
        @click="togglePause"
      >
        {{ paused ? "继续记录" : "暂停记录" }}
      </button>
      <button
        class="primary"
        :class="{ danger: status.monitoring }"
        type="button"
        @click="toggleMonitoring"
      >
        <span class="record-dot"></span>
        {{ status.monitoring ? "关闭记录" : "开启记录" }}
      </button>
    </div>
  </header>

  <div v-if="error" class="notice danger">
    <span>{{ error }}</span>
    <button type="button" @click="error = ''">×</button>
  </div>

  <section class="clipboard-page">
    <template v-if="status.monitoring">
      <div class="clipboard-metrics">
        <article class="clipboard-metric">
          <p>HISTORY</p>
          <strong>{{ status.itemCount }}</strong>
          <small>本地历史记录</small>
        </article>
        <article class="clipboard-metric">
          <p>PINNED</p>
          <strong>{{ status.pinnedCount }}</strong>
          <small>长期保留的记录</small>
        </article>
        <article class="clipboard-metric">
          <p>STORAGE</p>
          <strong>{{ formatBytes(status.dbSizeBytes) }}</strong>
          <small>SQLite 数据占用</small>
        </article>
        <article class="clipboard-metric">
          <p>MONITOR</p>
          <strong class="monitor-state" :class="{ paused }">
            {{ paused ? "已暂停" : "记录中" }}
          </strong>
          <small>{{ paused ? "不会写入新的记录" : "仅监控文本内容" }}</small>
        </article>
      </div>

      <div class="clipboard-layout">
        <section class="history-panel">
          <div class="history-toolbar">
            <div>
              <p>CLIPBOARD HISTORY</p>
              <h2>最近复制</h2>
            </div>
            <div class="history-controls">
              <label class="search-field">
                <span>搜索</span>
                <input
                  v-model="search"
                  type="search"
                  placeholder="输入内容关键词"
                  spellcheck="false"
                  @input="refreshList"
                />
              </label>
              <button
                class="quiet-danger"
                type="button"
                :disabled="status.itemCount === status.pinnedCount"
                @click="clearAll"
              >
                清空未置顶
              </button>
            </div>
          </div>

          <div v-if="loading && items.length === 0" class="panel-state">
            <span class="spinner"></span>
            正在读取本地记录…
          </div>
          <div v-else-if="items.length === 0" class="panel-state empty">
            <span class="empty-symbol">⌘V</span>
            <strong>{{ search ? "没有匹配的记录" : "还没有剪贴板记录" }}</strong>
            <small>{{ search ? "换一个关键词试试" : "复制一段文本后，它会自动出现在这里" }}</small>
          </div>
          <div v-else class="item-list">
            <article
              v-for="item in items"
              :key="item.id"
              class="clip-item"
              :class="{ pinned: item.pinned }"
              @click="copyItem(item)"
            >
              <div class="clip-content">
                <div class="clip-item-meta">
                  <span class="clip-type">{{ typeBadge(item.contentType) }}</span>
                  <span>{{ timeAgo(item.copiedAtMillis) }}</span>
                  <span v-if="item.charCount">{{ item.charCount }} 字</span>
                  <span v-if="item.useCount > 1">使用 {{ item.useCount }} 次</span>
                  <span v-if="item.pinned" class="pinned-label">已置顶</span>
                </div>
                <div
                  class="clip-preview"
                  :class="{ code: item.contentType === 'code' || item.contentType === 'url' }"
                >
                  {{ item.preview || item.content.slice(0, 200) }}
                </div>
              </div>
              <div class="clip-actions" @click.stop>
                <button
                  class="clip-btn"
                  :class="{ copied: copiedId === item.id }"
                  type="button"
                  :title="copiedId === item.id ? '已复制' : '复制到剪贴板'"
                  @click="copyItem(item)"
                >
                  {{ copiedId === item.id ? "已复制" : "复制" }}
                </button>
                <button
                  class="clip-btn"
                  :class="{ active: item.pinned }"
                  type="button"
                  :title="item.pinned ? '取消置顶' : '置顶'"
                  @click="pinItem(item.id)"
                >
                  {{ item.pinned ? "取消置顶" : "置顶" }}
                </button>
                <button
                  class="clip-btn remove"
                  type="button"
                  title="删除"
                  @click="deleteItem(item.id)"
                >
                  删除
                </button>
              </div>
            </article>
          </div>
        </section>

        <aside class="settings-panel">
          <div class="settings-heading">
            <p>RETENTION</p>
            <h2>记录设置</h2>
            <small>限制本地数据规模，置顶内容不会被自动清理。</small>
          </div>
          <div class="settings-fields">
            <label>
              <span>最大记录数</span>
              <div class="number-field">
                <input
                  v-model.number="settings.maxItems"
                  type="number"
                  min="100"
                  max="2000"
                  step="50"
                />
                <em>条</em>
              </div>
            </label>
            <label>
              <span>保留天数</span>
              <div class="number-field">
                <input
                  v-model.number="settings.retentionDays"
                  type="number"
                  min="1"
                  max="365"
                />
                <em>天</em>
              </div>
            </label>
          </div>
          <button class="save-btn" type="button" @click="saveSettings">
            {{ settingsSaved ? "设置已保存" : "保存设置" }}
          </button>
          <p class="privacy-note">
            数据保存在当前用户目录，不会发送到网络。
          </p>
        </aside>
      </div>
    </template>

    <div v-else class="clipboard-onboarding">
      <div class="onboarding-copy">
        <span class="onboarding-kicker">CLIPBOARD HISTORY</span>
        <h2>把复制过的内容，留在手边。</h2>
        <p>
          智屿会在本机记录文本剪贴板，方便你搜索、置顶和再次复制。
          密码等敏感输入不会主动上传，所有数据仅保存在用户目录。
        </p>
        <button class="onboarding-action" type="button" @click="toggleMonitoring">
          开启剪贴板记录
        </button>
      </div>
      <div class="onboarding-points">
        <article>
          <span>01</span>
          <div>
            <strong>只保存在本机</strong>
            <p>使用轻量 SQLite 存储，不依赖云端服务。</p>
          </div>
        </article>
        <article>
          <span>02</span>
          <div>
            <strong>随时暂停或关闭</strong>
            <p>关闭记录不会删除已有历史，可随时继续。</p>
          </div>
        </article>
        <article>
          <span>03</span>
          <div>
            <strong>自动控制空间</strong>
            <p>按记录数量和保留天数清理过期内容。</p>
          </div>
        </article>
      </div>
    </div>
  </section>
</template>

<style scoped>
.clipboard-logo {
  background: #6b6659;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 19px;
  font-weight: 600;
}

.record-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
}

.clipboard-page {
  padding: 26px 34px 34px;
}

.clipboard-metrics {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  margin-bottom: 18px;
  border-top: 1px solid #d2d1c9;
  border-left: 1px solid #d2d1c9;
}

.clipboard-metric {
  min-width: 0;
  padding: 18px 20px;
  border-right: 1px solid #d2d1c9;
  border-bottom: 1px solid #d2d1c9;
  background: rgba(250, 249, 245, 0.84);
}

.clipboard-metric p,
.history-toolbar p,
.settings-heading > p {
  margin: 0 0 12px;
  color: #989a93;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.12em;
}

.clipboard-metric strong {
  display: block;
  overflow: hidden;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 21px;
  font-weight: 500;
  letter-spacing: -0.05em;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.clipboard-metric strong.monitor-state {
  color: #2f7950;
  font-family: inherit;
  font-size: 16px;
  line-height: 25px;
  letter-spacing: 0;
}

.clipboard-metric strong.monitor-state.paused {
  color: #a46925;
}

.clipboard-metric small {
  display: block;
  margin-top: 8px;
  color: #989a93;
  font-size: 9px;
}

.clipboard-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 238px;
  gap: 18px;
  align-items: start;
}

.history-panel,
.settings-panel {
  border: 1px solid #d2d1c9;
  background: rgba(250, 249, 245, 0.9);
}

.history-toolbar {
  display: flex;
  min-height: 68px;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 13px 16px;
  border-bottom: 1px solid #d2d1c9;
}

.history-toolbar p,
.settings-heading > p {
  margin-bottom: 4px;
}

.history-toolbar h2,
.settings-heading h2 {
  margin: 0;
  font-size: 14px;
}

.history-controls {
  display: flex;
  align-items: center;
  gap: 8px;
}

.search-field {
  display: flex;
  align-items: center;
  gap: 8px;
  color: #898b84;
  font-size: 9px;
}

.search-field input {
  width: min(25vw, 230px);
  height: 32px;
  padding: 0 10px;
  border: 1px solid #c8c7bf;
  outline: 0;
  background: #fffefa;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
}

.search-field input:focus,
.number-field:focus-within {
  border-color: #777a70;
  box-shadow: 0 0 0 2px rgba(77, 81, 69, 0.08);
}

.quiet-danger {
  height: 32px;
  padding: 0 10px;
  border: 1px solid #d4b4aa;
  background: #f8eee9;
  color: #a64c35;
  cursor: pointer;
  font-size: 8px;
}

.quiet-danger:hover:not(:disabled) {
  border-color: #b56a55;
  background: #f3e2db;
}

.quiet-danger:disabled {
  cursor: default;
  opacity: 0.45;
}

.item-list {
  max-height: calc(100vh - 360px);
  overflow-y: auto;
}

.clip-item {
  position: relative;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 16px;
  align-items: center;
  min-height: 76px;
  padding: 13px 15px;
  border-bottom: 1px solid #e2e1da;
  background: rgba(255, 254, 250, 0.58);
  cursor: pointer;
  transition: background 120ms ease;
}

.clip-item:last-child {
  border-bottom: 0;
}

.clip-item:hover {
  background: #f0eee7;
}

.clip-item.pinned {
  background: #f7f2e3;
}

.clip-item.pinned::before {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  width: 3px;
  background: #c08a32;
  content: "";
}

.clip-content {
  min-width: 0;
}

.clip-item-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 7px;
  align-items: center;
  margin-bottom: 7px;
  color: #92948c;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
}

.clip-item-meta > span + span::before {
  margin-right: 7px;
  color: #c3c2bb;
  content: "·";
}

.clip-item-meta .clip-type::before,
.clip-item-meta .pinned-label::before {
  display: none;
}

.clip-type,
.pinned-label {
  display: inline-flex;
  align-items: center;
  min-height: 18px;
  padding: 0 6px;
  border: 1px solid #cbc9c0;
  border-radius: 10px;
  background: #e9e7df;
  color: #66695f;
  font-size: 7px;
  font-weight: 600;
  letter-spacing: 0.04em;
}

.pinned-label {
  border-color: #ddc799;
  background: #efe3c6;
  color: #815f1d;
}

.clip-preview {
  display: -webkit-box;
  overflow: hidden;
  color: #383b34;
  font-size: 11px;
  line-height: 1.55;
  overflow-wrap: anywhere;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
}

.clip-preview.code {
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 9px;
}

.clip-actions {
  display: flex;
  gap: 5px;
  opacity: 0.56;
  transition: opacity 120ms ease;
}

.clip-item:hover .clip-actions,
.clip-actions:focus-within {
  opacity: 1;
}

.clip-btn {
  min-width: 42px;
  height: 27px;
  padding: 0 8px;
  border: 1px solid #cfcec6;
  background: #faf9f5;
  color: #6f7269;
  cursor: pointer;
  font-size: 8px;
}

.clip-btn:hover {
  border-color: #898b83;
  color: #252920;
}

.clip-btn.active {
  border-color: #d4b76f;
  background: #f3ead1;
  color: #7f5d18;
}

.clip-btn.copied {
  border-color: #91b39a;
  background: #e5efe4;
  color: #2f7047;
}

.clip-btn.remove:hover {
  border-color: #d2a396;
  background: #f7e9e4;
  color: #a64c35;
}

.panel-state {
  display: grid;
  min-height: 300px;
  place-items: center;
  color: #92948c;
  font-size: 11px;
}

.panel-state.empty {
  align-content: center;
  gap: 7px;
}

.panel-state.empty strong {
  color: #55584f;
  font-size: 12px;
}

.panel-state.empty small {
  color: #979990;
  font-size: 9px;
}

.empty-symbol {
  display: grid;
  width: 48px;
  height: 48px;
  margin-bottom: 4px;
  place-items: center;
  border: 1px solid #cbc9c0;
  border-radius: 50%;
  background: #efede6;
  color: #777970;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 11px;
}

.settings-panel {
  overflow: hidden;
}

.settings-heading {
  padding: 17px 16px 15px;
  border-bottom: 1px solid #d2d1c9;
}

.settings-heading small {
  display: block;
  margin-top: 10px;
  color: #8d8f87;
  font-size: 9px;
  line-height: 1.6;
}

.settings-fields {
  display: grid;
  gap: 14px;
  padding: 16px;
}

.settings-fields label {
  display: grid;
  gap: 6px;
}

.settings-fields label > span {
  color: #73766d;
  font-size: 9px;
}

.number-field {
  display: flex;
  height: 34px;
  align-items: center;
  border: 1px solid #c8c7bf;
  background: #fffefa;
}

.number-field input {
  min-width: 0;
  flex: 1;
  padding: 0 9px;
  border: 0;
  outline: 0;
  background: transparent;
  color: #353830;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
}

.number-field em {
  padding: 0 9px;
  color: #989a93;
  font-size: 9px;
  font-style: normal;
}

.save-btn {
  width: calc(100% - 32px);
  height: 34px;
  margin: 0 16px;
  border: 1px solid #252920;
  background: #252920;
  color: white;
  cursor: pointer;
  font-size: 9px;
}

.save-btn:hover {
  background: #393d33;
}

.privacy-note {
  margin: 14px 16px 16px;
  padding-top: 12px;
  border-top: 1px solid #e0dfd7;
  color: #9a9c94;
  font-size: 8px;
  line-height: 1.6;
}

.clipboard-onboarding {
  display: grid;
  grid-template-columns: minmax(0, 1.25fr) minmax(280px, 0.75fr);
  min-height: 420px;
  border: 1px solid #d2d1c9;
  background:
    radial-gradient(circle at 12% 18%, rgba(221, 86, 51, 0.07), transparent 32%),
    rgba(250, 249, 245, 0.9);
}

.onboarding-copy {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  padding: 54px;
  border-right: 1px solid #d2d1c9;
}

.onboarding-kicker {
  margin-bottom: 18px;
  color: #b4553b;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.14em;
}

.onboarding-copy h2 {
  max-width: 520px;
  margin: 0;
  color: #2f332b;
  font-family: Georgia, "Songti SC", serif;
  font-size: clamp(25px, 2.7vw, 34px);
  font-weight: 500;
  letter-spacing: -0.04em;
  line-height: 1.16;
}

.onboarding-copy p {
  max-width: 510px;
  margin: 20px 0 0;
  color: #777a71;
  font-size: 11px;
  line-height: 1.85;
}

.onboarding-action {
  margin-top: 28px;
  padding: 10px 18px;
  border: 1px solid #252920;
  background: #252920;
  color: white;
  cursor: pointer;
  font-size: 9px;
}

.onboarding-action:hover {
  background: #393d33;
}

.onboarding-points {
  display: grid;
  align-content: center;
  padding: 28px 34px;
}

.onboarding-points article {
  display: grid;
  grid-template-columns: 30px 1fr;
  gap: 12px;
  padding: 22px 0;
  border-bottom: 1px solid #dcdad2;
}

.onboarding-points article:last-child {
  border-bottom: 0;
}

.onboarding-points article > span {
  color: #b55b41;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
}

.onboarding-points strong {
  color: #45483f;
  font-size: 11px;
}

.onboarding-points p {
  margin: 6px 0 0;
  color: #8b8d85;
  font-size: 9px;
  line-height: 1.6;
}

@media (max-width: 980px) {
  .clipboard-layout {
    grid-template-columns: 1fr;
  }

  .settings-panel {
    display: grid;
    grid-template-columns: minmax(180px, 1fr) minmax(240px, 1.4fr);
    align-items: center;
  }

  .settings-heading {
    align-self: stretch;
    border-right: 1px solid #d2d1c9;
    border-bottom: 0;
  }

  .settings-fields {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .settings-panel .save-btn,
  .settings-panel .privacy-note {
    grid-column: 2;
  }
}

@media (max-width: 760px) {
  .clipboard-page {
    padding: 20px;
  }

  .clipboard-metrics {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .history-toolbar,
  .history-controls {
    align-items: stretch;
    flex-direction: column;
  }

  .search-field input {
    width: 100%;
  }

  .clip-item {
    grid-template-columns: 1fr;
  }

  .clip-actions {
    opacity: 1;
  }

  .settings-panel {
    display: block;
  }

  .settings-heading {
    border-right: 0;
    border-bottom: 1px solid #d2d1c9;
  }

  .clipboard-onboarding {
    grid-template-columns: 1fr;
  }

  .onboarding-copy {
    padding: 38px;
    border-right: 0;
    border-bottom: 1px solid #d2d1c9;
  }
}
</style>
