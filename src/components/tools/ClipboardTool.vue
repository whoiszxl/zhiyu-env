<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  clipboardStart, clipboardStop, clipboardPause, clipboardResume,
  clipboardStatus, clipboardList, clipboardCopy, clipboardPin,
  clipboardDelete, clipboardClear,
} from "../../api/services";
import type { ClipboardItem, ClipboardStatus } from "../../types";
import { formatBytes } from "../../utils/format";

const items = ref<ClipboardItem[]>([]);
const status = ref<ClipboardStatus>({ itemCount: 0, pinnedCount: 0, dbSizeBytes: 0, monitoring: false });
const search = ref("");
const error = ref("");
const loading = ref(false);
const copiedId = ref(0);
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

function timeAgo(ms: number): string {
  const diff = Date.now() - ms;
  if (diff < 60_000) return "刚才";
  if (diff < 3600_000) return `${Math.floor(diff / 60_000)} 分钟前`;
  if (diff < 86400_000) return `${Math.floor(diff / 3600_000)} 小时前`;
  return `${Math.floor(diff / 86400_000)} 天前`;
}

const typeBadge = (t: string) => ({ text: "T", code: "{}", url: "URL" } as Record<string, string>)[t] || "T";

onMounted(async () => {
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
  <div class="clipboard-tool">
    <!-- header -->
    <header class="detail-header">
      <div class="detail-header-left">
        <div class="nav-icon clipboard-icon">&#x2398;</div>
        <div class="detail-header-text">
          <h2>剪贴板历史</h2>
          <p>本地记录 · 最新复制</p>
        </div>
      </div>
      <div class="header-actions">
        <button
          class="toggle-btn"
          :class="{ active: status.monitoring }"
          @click="toggleMonitoring"
        >
          <span class="dot"></span>
          {{ status.monitoring ? (paused ? "已暂停" : "记录中") : "已关闭" }}
        </button>
        <button
          v-if="status.monitoring"
          class="link-btn"
          @click="togglePause"
        >
          {{ paused ? "恢复" : "暂停" }}
        </button>
      </div>
    </header>

    <!-- error -->
    <p class="error-msg" v-if="error">{{ error }}</p>

    <!-- search bar -->
    <div class="search-bar" v-if="status.monitoring">
      <input
        v-model="search"
        type="text"
        placeholder="搜索剪贴板历史…"
        spellcheck="false"
        @input="refreshList"
      />
    </div>

    <!-- empty -->
    <div class="empty-state" v-if="status.monitoring && items.length === 0 && !loading">
      <p v-if="status.monitoring">暂无记录，开始复制文本吧</p>
      <p v-else>开启记录后，复制的文本会自动保存</p>
    </div>

    <!-- stats -->
    <div class="stats" v-if="status.monitoring && items.length > 0">
      <span>{{ status.itemCount }} 条 · {{ status.pinnedCount }} 置顶 · {{ formatBytes(status.dbSizeBytes) }}</span>
      <button class="link-btn danger" @click="clearAll">清空未置顶</button>
    </div>

    <!-- item list -->
    <div class="item-list" v-if="items.length > 0">
      <div
        v-for="item in items"
        :key="item.id"
        class="clip-item"
        :class="{ pinned: item.pinned }"
        @click="copyItem(item)"
      >
        <div class="clip-item-meta">
          <span class="clip-type">{{ typeBadge(item.contentType) }}</span>
          <span class="clip-time">{{ timeAgo(item.copiedAtMillis) }}</span>
          <span v-if="item.charCount" class="clip-chars">{{ item.charCount }} 字</span>
          <span v-if="item.useCount > 1" class="clip-uses">{{ item.useCount }}x</span>
        </div>
        <div class="clip-preview">{{ item.preview || item.content.slice(0, 200) }}</div>
        <div class="clip-actions" @click.stop>
          <button
            class="clip-btn"
            :class="{ copied: copiedId === item.id }"
            @click="copyItem(item)"
            :title="copiedId === item.id ? '已复制' : '复制到剪贴板'"
          >
            {{ copiedId === item.id ? "&#x2714;" : "&#x2398;" }}
          </button>
          <button
            class="clip-btn"
            :title="item.pinned ? '取消置顶' : '置顶'"
            @click="pinItem(item.id)"
          >
            {{ item.pinned ? "&#x2B50;" : "&#x2606;" }}
          </button>
          <button
            class="clip-btn"
            title="删除"
            @click="deleteItem(item.id)"
          >
            &#x2715;
          </button>
        </div>
      </div>
    </div>

    <!-- loading -->
    <p v-if="loading" class="loading">加载中…</p>

    <!-- disabled state -->
    <div class="disabled-state" v-if="!status.monitoring">
      <p>剪贴板历史默认关闭，点击上方按钮开启。</p>
      <p class="hint">开启后，复制到剪贴板的文本会自动保存到本地数据库，不会上传。</p>
    </div>
  </div>
</template>

<style scoped>
.clipboard-tool {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 0 6px;
}

.detail-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.detail-header-left {
  display: flex;
  align-items: center;
  gap: 12px;
}

.clipboard-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  background: #1f2b4d;
  color: #5b8dee;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
}

.detail-header-text h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary, #e1e4eb);
}

.detail-header-text p {
  margin: 2px 0 0;
  font-size: 12px;
  color: var(--text-secondary, #787f8e);
}

.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.toggle-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border-radius: 6px;
  border: 1px solid var(--border, #2d3240);
  background: var(--input-bg, #10141b);
  color: var(--text-secondary, #787f8e);
  font-size: 13px;
  cursor: pointer;
  transition: all .15s;
}
.toggle-btn.active {
  background: #1a2e1a;
  color: #6abf6a;
  border-color: #2a4a2a;
}
.toggle-btn .dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: currentColor;
  opacity: .5;
}
.toggle-btn.active .dot {
  opacity: 1;
}

.link-btn {
  background: none;
  border: 1px solid var(--border, #2d3240);
  border-radius: 6px;
  padding: 5px 12px;
  font-size: 12px;
  color: var(--text-secondary, #787f8e);
  cursor: pointer;
  transition: all .15s;
}
.link-btn:hover { color: var(--text-primary, #e1e4eb); border-color: #4a5060; }
.link-btn.danger:hover { color: #f5a6a6; border-color: #7a2a2a; }

.error-msg {
  margin: 0;
  padding: 8px 12px;
  background: #3d1a1a;
  border-radius: 6px;
  color: #f5a6a6;
  font-size: 13px;
}

.search-bar input {
  width: 100%;
  box-sizing: border-box;
  padding: 8px 12px;
  background: var(--input-bg, #10141b);
  border: 1px solid var(--border, #2d3240);
  border-radius: 8px;
  color: var(--text-primary, #e1e4eb);
  font-size: 13px;
  outline: none;
}
.search-bar input:focus { border-color: var(--accent-border, #2e4275); }

.empty-state, .disabled-state {
  text-align: center;
  padding: 32px 16px;
  color: var(--text-secondary, #787f8e);
}
.disabled-state .hint { font-size: 12px; opacity: .7; margin-top: 4px; }

.stats {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
  color: var(--text-secondary, #787f8e);
  padding: 0 2px;
}

.item-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  max-height: calc(100vh - 320px);
  overflow-y: auto;
}

.clip-item {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  background: var(--input-bg, #10141b);
  border: 1px solid var(--border, #2d3240);
  border-radius: 8px;
  cursor: pointer;
  transition: all .12s;
}
.clip-item:hover { border-color: #4a5060; }
.clip-item.pinned { border-color: #5b4a1a; background: #1a180e; }

.clip-item-meta {
  display: flex;
  gap: 8px;
  align-items: center;
  font-size: 11px;
  color: var(--text-secondary, #787f8e);
}

.clip-type {
  font-weight: 600;
  color: var(--accent, #5b8dee);
  text-transform: uppercase;
}

.clip-preview {
  font-size: 13px;
  color: var(--text-primary, #e1e4eb);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  line-height: 1.4;
}

.clip-actions {
  display: flex;
  gap: 4px;
  justify-content: flex-end;
}

.clip-btn {
  background: none;
  border: 1px solid transparent;
  border-radius: 4px;
  padding: 2px 6px;
  font-size: 12px;
  color: var(--text-secondary, #787f8e);
  cursor: pointer;
  transition: all .12s;
}
.clip-btn:hover { color: var(--accent, #5b8dee); border-color: #2e4275; }
.clip-btn.copied { color: #6abf6a; }

.loading {
  text-align: center;
  color: var(--text-secondary, #787f8e);
  font-size: 13px;
}
</style>
