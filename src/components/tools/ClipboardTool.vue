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
const status = ref<ClipboardStatus>({ itemCount: 0, pinnedCount: 0, dbSizeBytes: 0, runState: "stopped" });
const settings = ref<ClipboardSettings>({ maxItems: 500, retentionDays: 30, autoStartMonitoring: false });
const search = ref("");
const notice = ref("");
const loading = ref(false);
const copiedId = ref(0);
const settingsSaved = ref(false);
const actionBusy = ref(false);

let unlisten: UnlistenFn | null = null;
let searchTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleRefreshList() {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => refreshList(), 250);
}

async function refreshList() {
  loading.value = true;
  try {
    const q = search.value.trim() || undefined;
    items.value = await clipboardList(q, 200, 0);
    notice.value = "";
  } catch (e: any) {
    notice.value = String(e);
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
  if (actionBusy.value) return;
  actionBusy.value = true;
  try {
    if (status.value.runState !== "stopped") {
      await clipboardStop();
    } else {
      await clipboardStart();
    }
    await refreshStatus();
    if (status.value.runState !== "stopped") {
      await refreshList();
    }
    notice.value = "";
  } catch (e: any) {
    notice.value = String(e);
  } finally {
    actionBusy.value = false;
  }
}

async function togglePause() {
  if (actionBusy.value) return;
  actionBusy.value = true;
  try {
    if (status.value.runState === "paused") {
      await clipboardResume();
    } else {
      await clipboardPause();
    }
    await refreshStatus();
    notice.value = "";
  } catch (e: any) {
    notice.value = String(e);
  } finally {
    actionBusy.value = false;
  }
}

async function copyItem(item: ClipboardItem) {
  if (copiedId.value === item.id) return;
  try {
    await clipboardCopy(item.id);
    copiedId.value = item.id;
    setTimeout(() => { copiedId.value = 0; }, 1200);
    notice.value = "";
  } catch (e: any) {
    notice.value = String(e);
  }
}

async function pinItem(id: number) {
  try {
    await clipboardPin(id);
    await refreshList();
    notice.value = "";
  } catch (e: any) {
    notice.value = String(e);
  }
}

async function deleteItem(id: number) {
  try {
    await clipboardDelete(id);
    await refreshList();
    await refreshStatus();
    notice.value = "";
  } catch (e: any) {
    notice.value = String(e);
  }
}

async function clearAll() {
  if (!confirm("清空所有未置顶的剪贴板记录？")) return;
  try {
    await clipboardClear();
    await refreshList();
    await refreshStatus();
    notice.value = "";
  } catch (e: any) {
    notice.value = String(e);
  }
}

async function loadSettings() {
  try { settings.value = await clipboardSettingsGet(); } catch { /* defaults */ }
}

async function saveSettings() {
  try {
    const result = await clipboardSettingsSave(settings.value);
    status.value = result;
    settingsSaved.value = true;
    setTimeout(() => { settingsSaved.value = false; }, 1500);
    await refreshList();
    notice.value = "";
  } catch (e: any) {
    notice.value = String(e);
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

const runStateLabel = (s: string) => ({ running: "记录中", paused: "已暂停", stopped: "已关闭" } as Record<string, string>)[s] || "已关闭";

onMounted(async () => {
  await loadSettings();
  await refreshStatus();
  unlisten = await listen<ClipboardItem>("clipboard:changed", () => {
    refreshList();
    refreshStatus();
  });
  if (status.value.runState !== "stopped") {
    await refreshList();
  }
});

onUnmounted(() => {
  unlisten?.();
  if (searchTimer) clearTimeout(searchTimer);
});
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
            :class="{ running: status.runState === 'running', paused: status.runState === 'paused' }"
          ></span>
          本地记录、搜索并快速复用最近复制的文本
        </p>
      </div>
    </div>
    <div class="header-actions">
      <button
        v-if="status.runState !== 'stopped'"
        type="button"
        :disabled="actionBusy"
        @click="togglePause"
      >
        {{ status.runState === "paused" ? "继续记录" : "暂停记录" }}
      </button>
      <button
        class="primary"
        :class="{ danger: status.runState !== 'stopped' }"
        type="button"
        :disabled="actionBusy"
        @click="toggleMonitoring"
      >
        <span class="record-dot"></span>
        {{ status.runState !== "stopped" ? "关闭记录" : "开启记录" }}
      </button>
    </div>
  </header>

  <div v-if="notice" class="notice danger">
    <span>{{ notice }}</span>
    <button type="button" @click="notice = ''">×</button>
  </div>

  <section class="clipboard-page">
    <template v-if="status.runState !== 'stopped'">
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
          <strong
            class="monitor-state"
            :class="{ paused: status.runState === 'paused' }"
          >
            {{ runStateLabel(status.runState) }}
          </strong>
          <small>{{ status.runState === "paused" ? "不会写入新的记录" : "仅监控文本内容" }}</small>
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
                  @input="scheduleRefreshList"
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
            <label class="checkbox-label">
              <input v-model="settings.autoStartMonitoring" type="checkbox" />
              <span>启动时自动开启记录</span>
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
        <button class="onboarding-action" type="button" :disabled="actionBusy" @click="toggleMonitoring">
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
/* --- identical to existing styles, unchanged --- */

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

.status-dot {
  display: inline-block;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  margin-right: 6px;
  background: #c4c2ba;
}
.status-dot.running { background: #2f7950; }
.status-dot.paused { background: #a46925; }

.clipboard-page {
  padding: 26px 34px 34px;
}

.clipboard-metrics {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  margin-bottom: 18px;
  border-top: 1px solid var(--color-border);
  border-left: 1px solid var(--color-border);
}

.clipboard-metric {
  min-width: 0;
  padding: 18px 20px;
  border-right: 1px solid var(--color-border);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

.clipboard-metric p,
.history-toolbar p,
.settings-heading > p {
  margin: 0 0 12px;
  color: var(--color-text-muted);
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
  color: var(--color-success-text);
  font-family: inherit;
  font-size: 16px;
  line-height: 25px;
  letter-spacing: 0;
}

.clipboard-metric strong.monitor-state.paused {
  color: var(--color-warning-text);
}

.clipboard-metric small {
  display: block;
  margin-top: 8px;
  color: var(--color-text-muted);
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
  border: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

.history-toolbar {
  display: flex;
  min-height: 68px;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 13px 16px;
  border-bottom: 1px solid var(--color-border);
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
  color: var(--color-text-muted);
  font-size: 9px;
}

.search-field input {
  width: min(25vw, 230px);
  height: 32px;
  padding: 0 10px;
  border: 1px solid var(--color-border-strong);
  outline: 0;
  background: var(--color-bg-elevated);
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
  background: var(--color-danger-surface);
  color: var(--color-danger-text);
  cursor: pointer;
  font-size: 8px;
}

.quiet-danger:hover:not(:disabled) {
  border-color: #b56a55;
  background: var(--color-danger-surface);
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
  border-bottom: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
  cursor: pointer;
  transition: background 120ms ease;
}

.clip-item:last-child {
  border-bottom: 0;
}

.clip-item:hover {
  background: var(--color-bg-muted);
}

.clip-item.pinned {
  background: var(--color-warning-surface);
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
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
}

.clip-item-meta > span + span::before {
  margin-right: 7px;
  color: var(--color-text-muted);
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
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-bg-muted);
  color: var(--color-text-secondary);
  font-size: 7px;
  font-weight: 600;
  letter-spacing: 0.04em;
}

.pinned-label {
  border-color: #ddc799;
  background: var(--color-warning-surface);
  color: var(--color-warning-text);
}

.clip-preview {
  display: -webkit-box;
  overflow: hidden;
  color: var(--color-text-primary);
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
  border: 1px solid var(--color-border);
  background: var(--color-bg-panel);
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: 8px;
}

.clip-btn:hover {
  border-color: var(--color-border-strong);
  color: var(--color-text-primary);
}

.clip-btn.active {
  border-color: #d4b76f;
  background: var(--color-warning-surface);
  color: var(--color-warning-text);
}

.clip-btn.copied {
  border-color: #91b39a;
  background: var(--color-success-surface);
  color: var(--color-success-text);
}

.clip-btn.remove:hover {
  border-color: #d2a396;
  background: var(--color-danger-surface);
  color: var(--color-danger-text);
}

.panel-state {
  display: grid;
  min-height: 300px;
  place-items: center;
  color: var(--color-text-muted);
  font-size: 11px;
}

.panel-state.empty {
  align-content: center;
  gap: 7px;
}

.panel-state.empty strong {
  color: var(--color-text-secondary);
  font-size: 12px;
}

.panel-state.empty small {
  color: var(--color-text-muted);
  font-size: 9px;
}

.empty-symbol {
  display: grid;
  width: 48px;
  height: 48px;
  margin-bottom: 4px;
  place-items: center;
  border: 1px solid var(--color-border);
  border-radius: 50%;
  background: var(--color-bg-muted);
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 11px;
}

.settings-panel {
  overflow: hidden;
}

.settings-heading {
  padding: 17px 16px 15px;
  border-bottom: 1px solid var(--color-border);
}

.settings-heading small {
  display: block;
  margin-top: 10px;
  color: var(--color-text-muted);
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
  color: var(--color-text-secondary);
  font-size: 9px;
}

.checkbox-label {
  display: flex !important;
  flex-direction: row !important;
  align-items: center;
  gap: 8px;
  color: var(--color-text-secondary);
  font-size: 9px;
  cursor: pointer;
}

.checkbox-label input[type="checkbox"] {
  width: 14px;
  height: 14px;
  accent-color: var(--color-accent);
}

.number-field {
  display: flex;
  height: 34px;
  align-items: center;
  border: 1px solid var(--color-border-strong);
  background: var(--color-bg-elevated);
}

.number-field input {
  min-width: 0;
  flex: 1;
  padding: 0 9px;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--color-text-primary);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
}

.number-field em {
  padding: 0 9px;
  color: var(--color-text-muted);
  font-size: 9px;
  font-style: normal;
}

.save-btn {
  width: calc(100% - 32px);
  height: 34px;
  margin: 0 16px;
  border: 1px solid var(--color-control-primary);
  background: var(--color-control-primary);
  color: white;
  cursor: pointer;
  font-size: 9px;
}

.save-btn:hover {
  background: var(--color-control-primary-hover);
}

.privacy-note {
  margin: 14px 16px 16px;
  padding-top: 12px;
  border-top: 1px solid var(--color-border);
  color: var(--color-text-muted);
  font-size: 8px;
  line-height: 1.6;
}

.clipboard-onboarding {
  display: grid;
  grid-template-columns: minmax(0, 1.25fr) minmax(280px, 0.75fr);
  min-height: 420px;
  border: 1px solid var(--color-border);
  background:
    radial-gradient(circle at 12% 18%, rgba(221, 86, 51, 0.07), transparent 32%),
    var(--color-panel-translucent);
}

.onboarding-copy {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  padding: 54px;
  border-right: 1px solid var(--color-border);
}

.onboarding-kicker {
  margin-bottom: 18px;
  color: var(--color-danger-text);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.14em;
}

.onboarding-copy h2 {
  max-width: 520px;
  margin: 0;
  color: var(--color-text-primary);
  font-family: Georgia, "Songti SC", serif;
  font-size: clamp(25px, 2.7vw, 34px);
  font-weight: 500;
  letter-spacing: -0.04em;
  line-height: 1.16;
}

.onboarding-copy p {
  max-width: 510px;
  margin: 20px 0 0;
  color: var(--color-text-secondary);
  font-size: 11px;
  line-height: 1.85;
}

.onboarding-action {
  margin-top: 28px;
  padding: 10px 18px;
  border: 1px solid var(--color-control-primary);
  background: var(--color-control-primary);
  color: white;
  cursor: pointer;
  font-size: 9px;
}

.onboarding-action:hover:not(:disabled) {
  background: var(--color-control-primary-hover);
}

.onboarding-action:disabled {
  opacity: 0.5;
  cursor: default;
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
  border-bottom: 1px solid var(--color-border);
}

.onboarding-points article:last-child {
  border-bottom: 0;
}

.onboarding-points article > span {
  color: var(--color-danger-text);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
}

.onboarding-points strong {
  color: var(--color-text-primary);
  font-size: 11px;
}

.onboarding-points p {
  margin: 6px 0 0;
  color: var(--color-text-muted);
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
    border-right: 1px solid var(--color-border);
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
    border-bottom: 1px solid var(--color-border);
  }

  .clipboard-onboarding {
    grid-template-columns: 1fr;
  }

  .onboarding-copy {
    padding: 38px;
    border-right: 0;
    border-bottom: 1px solid var(--color-border);
  }
}
</style>
