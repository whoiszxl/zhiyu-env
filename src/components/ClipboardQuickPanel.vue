<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  clipboardList, clipboardCopy, clipboardPin, clipboardDelete,
} from "../api/services";
import type { ClipboardItem } from "../types";

const items = ref<ClipboardItem[]>([]);
const search = ref("");
const selectedIndex = ref(0);
const loading = ref(false);

const win = getCurrentWindow();

function filteredItems() {
  if (!search.value.trim()) return items.value;
  const q = search.value.toLowerCase();
  return items.value.filter(i =>
    (i.preview || i.content).toLowerCase().includes(q),
  );
}

async function loadItems() {
  loading.value = true;
  try {
    items.value = await clipboardList(undefined, 50, 0);
  } catch { /* ignore */ }
  loading.value = false;
}

async function copyAndClose(item: ClipboardItem) {
  await clipboardCopy(item.id);
  win.hide();
}

async function togglePin() {
  const filtered = filteredItems();
  const item = filtered[selectedIndex.value];
  if (!item) return;
  await clipboardPin(item.id);
  await loadItems();
}

async function deleteSelected() {
  const filtered = filteredItems();
  const item = filtered[selectedIndex.value];
  if (!item) return;
  await clipboardDelete(item.id);
  await loadItems();
}

function onKeyDown(e: KeyboardEvent) {
  const filtered = filteredItems();
  const isMod = e.metaKey || e.ctrlKey;

  if (e.key === "ArrowDown") {
    e.preventDefault();
    selectedIndex.value = Math.min(selectedIndex.value + 1, filtered.length - 1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    selectedIndex.value = Math.max(selectedIndex.value - 1, 0);
  } else if (e.key === "Enter") {
    e.preventDefault();
    const item = filtered[selectedIndex.value];
    if (!item) return;
    if (isMod) {
      copyAndClose(item);
    } else {
      clipboardCopy(item.id);
    }
  } else if (e.key === "p" && isMod) {
    e.preventDefault();
    togglePin();
  } else if (e.key === "Backspace" || e.key === "Delete") {
    e.preventDefault();
    deleteSelected();
  } else if (e.key === "Escape") {
    e.preventDefault();
    win.hide();
  }
}

function selectItem(item: ClipboardItem) {
  clipboardCopy(item.id).then(() => win.hide());
}

onMounted(async () => {
  await loadItems();
  await nextTick();
  const input = document.querySelector<HTMLInputElement>(".qp-search input");
  input?.focus();

  // Hide when window loses focus
  await win.listen("tauri://blur", () => {
    win.hide();
  });
});

onUnmounted(() => {});
</script>

<template>
  <div class="quick-panel" @keydown="onKeyDown">
    <div class="qp-header">
      <div class="qp-search">
        <span class="qp-search-icon">&#x2318;V</span>
        <input
          v-model="search"
          type="text"
          placeholder="搜索剪贴板历史…"
          spellcheck="false"
          autofocus
        />
        <span class="qp-hint">ESC 关闭</span>
      </div>
    </div>
    <div class="qp-list">
      <div v-if="loading" class="qp-state">加载中…</div>
      <div v-else-if="filteredItems().length === 0" class="qp-state empty">
        {{ search ? "无匹配结果" : "暂无剪贴板记录" }}
      </div>
      <div
        v-for="(item, idx) in filteredItems()"
        :key="item.id"
        class="qp-item"
        :class="{ selected: idx === selectedIndex, pinned: item.pinned }"
        @click="selectItem(item)"
        @mouseenter="selectedIndex = idx"
      >
        <div class="qp-item-pin" v-if="item.pinned">&#x2B50;</div>
        <div class="qp-item-body">
          <div class="qp-item-preview">{{ item.preview || item.content.slice(0, 200) }}</div>
          <div class="qp-item-meta">
            <span class="qp-type">{{ item.contentType }}</span>
            <span v-if="item.useCount > 1">{{ item.useCount }}x</span>
          </div>
        </div>
      </div>
    </div>
    <div class="qp-footer">
      <span>&#x21B5; 复制</span>
      <span>&#x2318;&#x21B5; 复制并关闭</span>
      <span>&#x2318;P 置顶</span>
      <span>&#x2326; 删除</span>
    </div>
  </div>
</template>

<style scoped>
.quick-panel {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #faf9f5;
  border: 1px solid #d2d1c9;
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.15), 0 2px 8px rgba(0, 0, 0, 0.08);
  outline: none;
}

.qp-header {
  padding: 12px 16px 10px;
  border-bottom: 1px solid #e8e6df;
}

.qp-search {
  display: flex;
  align-items: center;
  gap: 10px;
  background: #f0eee7;
  border: 1px solid #d8d6ce;
  border-radius: 8px;
  padding: 0 12px;
  height: 36px;
}

.qp-search-icon {
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
  color: #989a93;
  flex-shrink: 0;
}

.qp-search input {
  flex: 1;
  border: none;
  outline: none;
  background: transparent;
  font-size: 13px;
  color: #2f332b;
  font-family: inherit;
}
.qp-search input::placeholder {
  color: #b0b2ab;
}

.qp-hint {
  font-size: 9px;
  color: #b0b2ab;
  font-family: "SFMono-Regular", Consolas, monospace;
  flex-shrink: 0;
}

.qp-list {
  flex: 1;
  overflow-y: auto;
  padding: 4px 0;
}

.qp-state {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #989a93;
  font-size: 12px;
}

.qp-item {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 8px 16px;
  cursor: pointer;
  transition: background 80ms;
  border-left: 3px solid transparent;
}

.qp-item.selected {
  background: #e8ecf4;
  border-left-color: #5b7fc0;
}

.qp-item.pinned .qp-item-pin {
  opacity: 1;
}

.qp-item-pin {
  font-size: 9px;
  opacity: 0.3;
  margin-top: 2px;
  flex-shrink: 0;
}

.qp-item-body {
  min-width: 0;
  flex: 1;
}

.qp-item-preview {
  font-size: 12px;
  color: #2f332b;
  line-height: 1.4;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.qp-item-meta {
  display: flex;
  gap: 8px;
  margin-top: 3px;
  font-size: 9px;
  color: #989a93;
  font-family: "SFMono-Regular", Consolas, monospace;
  text-transform: uppercase;
}

.qp-footer {
  display: flex;
  gap: 16px;
  padding: 6px 16px;
  border-top: 1px solid #e8e6df;
  font-size: 9px;
  color: #b0b2ab;
  font-family: "SFMono-Regular", Consolas, monospace;
  background: #f5f3ee;
}
</style>
