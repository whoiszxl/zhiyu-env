<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { cancelAiToolResult, generateAiToolResult } from "../api/ai";
import { getAiSettings } from "../api/services";
import type {
  AiAssistOption,
  AiToolCapability,
  AiToolStreamEvent,
} from "../types";
import { renderMarkdown } from "../utils/markdown";

const props = defineProps<{
  open: boolean;
  title: string;
  context: string;
  options: AiAssistOption[];
}>();
const emit = defineEmits<{
  close: [];
  apply: [content: string, capability: AiToolCapability];
  settings: [];
}>();

const selected = ref<AiToolCapability>("service_logs");
const instruction = ref("");
const output = ref("");
const error = ref("");
const running = ref(false);
const configured = ref(true);
const requestId = ref("");
let unlisten: UnlistenFn | null = null;

const activeOption = computed(
  () => props.options.find((item) => item.id === selected.value) ?? props.options[0],
);

async function ensureListener() {
  if (unlisten) return;
  unlisten = await listen<AiToolStreamEvent>("ai-tool-stream", ({ payload }) => {
    if (payload.requestId !== requestId.value) return;
    if (payload.event === "delta") output.value += payload.content;
    if (payload.event === "error") error.value = payload.content;
    if (["done", "cancelled", "error"].includes(payload.event)) {
      running.value = false;
    }
  });
}

async function generate() {
  if (!instruction.value.trim() || running.value) return;
  await ensureListener();
  output.value = "";
  error.value = "";
  running.value = true;
  requestId.value = crypto.randomUUID();
  try {
    await generateAiToolResult({
      requestId: requestId.value,
      capability: selected.value,
      instruction: instruction.value,
      context: props.context,
      outputLanguage: document.documentElement.lang || "zh-CN",
    });
  } catch (cause) {
    error.value ||= String(cause);
    running.value = false;
  }
}

async function stop() {
  if (!requestId.value) return;
  await cancelAiToolResult(requestId.value);
}

async function copy() {
  if (output.value) await navigator.clipboard.writeText(output.value);
}

watch(
  () => props.open,
  async (open) => {
    if (!open) {
      if (running.value) await stop();
      return;
    }
    selected.value = props.options[0]?.id ?? "service_logs";
    instruction.value = "";
    output.value = "";
    error.value = "";
    const settings = await getAiSettings().catch(() => null);
    configured.value = Boolean(settings?.enabled && settings.apiKeyConfigured);
  },
);

onBeforeUnmount(() => {
  if (running.value) void stop();
  unlisten?.();
});
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      v-tool-i18n
      class="ai-assist-backdrop"
      @mousedown.self="emit('close')"
    >
      <section class="ai-assist-dialog" role="dialog" aria-modal="true">
        <header>
          <div class="ai-assist-title">
            <span class="ai-assist-mark" aria-hidden="true">
              <svg viewBox="0 0 24 24">
                <path d="M12 3.5a8.5 8.5 0 1 0 8.5 8.5" />
                <path d="M12 7.4a4.6 4.6 0 1 0 4.6 4.6" />
                <path d="M18.4 3.5v4.1h4.1M12 10.4V12l1.2 1.2" />
              </svg>
            </span>
            <div><span>AI ASSISTANT</span><h2>{{ title }}</h2></div>
          </div>
          <button class="ai-assist-close" type="button" aria-label="关闭" @click="emit('close')">
            <svg viewBox="0 0 20 20" aria-hidden="true">
              <path d="m5 5 10 10M15 5 5 15" />
            </svg>
          </button>
        </header>
        <nav v-if="options.length > 1">
          <button
            v-for="option in options"
            :key="option.id"
            type="button"
            :class="{ active: selected === option.id }"
            @click="selected = option.id; output = ''; error = ''"
          >{{ option.label }}</button>
        </nav>
        <div v-if="!configured" class="ai-assist-config">
          请先配置并启用模型 API。
          <button type="button" @click="emit('settings')">打开 AI 设置</button>
        </div>
        <template v-else>
          <label class="ai-assist-prompt">
            <span>
              <b>REQUEST</b>
              <em>{{ activeOption?.hint }}</em>
            </span>
            <textarea
              v-model="instruction"
              :placeholder="activeOption?.hint"
              @keydown.meta.enter.prevent="generate"
              @keydown.ctrl.enter.prevent="generate"
            ></textarea>
          </label>
          <div class="ai-assist-actions">
            <small>AI 仅生成建议，不会自动执行 · ⌘/Ctrl + Enter</small>
            <button v-if="running" type="button" @click="stop">停止</button>
            <button v-else class="primary" type="button" :disabled="!instruction.trim()" @click="generate">
              {{ output ? "重新生成" : "生成" }}
            </button>
          </div>
          <p v-if="error" class="ai-assist-error">{{ error }}</p>
          <section class="ai-assist-result">
            <header>
              <div><span>RESULT</span><strong>生成结果</strong></div>
              <i v-if="running"><span></span> STREAMING</i>
            </header>
            <div class="ai-assist-output">
              <div v-if="output" class="ai-assist-markdown" v-html="renderMarkdown(output)"></div>
              <span v-else>{{ running ? "正在连接模型…" : "生成结果会显示在这里" }}</span>
            </div>
          </section>
          <footer v-if="output">
            <button type="button" @click="copy">复制</button>
            <button
              v-if="activeOption?.canApply"
              class="primary"
              type="button"
              @click="emit('apply', output.trim(), selected)"
            >应用到编辑器</button>
          </footer>
        </template>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.ai-assist-backdrop {
  position: fixed;
  z-index: 10020;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 24px;
  background: color-mix(in srgb, var(--color-bg-sidebar) 48%, transparent);
  backdrop-filter: blur(8px) saturate(0.9);
}

.ai-assist-dialog {
  display: flex;
  width: min(680px, 100%);
  max-height: min(650px, calc(100vh - 48px));
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--color-border-strong);
  border-radius: 8px;
  background-color: var(--color-bg-content);
  background-image:
    linear-gradient(var(--color-grid-line) 1px, transparent 1px),
    linear-gradient(90deg, var(--color-grid-line) 1px, transparent 1px);
  background-size: 36px 36px;
  box-shadow: 0 24px 70px rgb(0 0 0 / 0.28);
}

.ai-assist-dialog > header {
  display: flex;
  min-height: 64px;
  align-items: center;
  justify-content: space-between;
  padding: 10px 14px 10px 16px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-header);
}

.ai-assist-title {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 10px;
}

.ai-assist-mark {
  display: grid;
  width: 34px;
  height: 34px;
  flex: 0 0 auto;
  place-items: center;
  border-radius: 9px;
  background: color-mix(in srgb, var(--color-accent) 14%, var(--color-bg-muted));
  color: var(--color-accent);
}

.ai-assist-mark svg {
  width: 21px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.55;
}

.ai-assist-title > div > span,
.ai-assist-result > header span {
  display: block;
  color: var(--color-text-muted);
  font: 8px/1.35 "SFMono-Regular", Consolas, monospace;
  letter-spacing: 0.13em;
}

.ai-assist-title h2 {
  overflow: hidden;
  margin: 3px 0 0;
  color: var(--color-text-primary);
  font-size: 14px;
  line-height: 1.25;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ai-assist-close {
  display: grid;
  width: 30px;
  height: 30px;
  place-items: center;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
}

.ai-assist-close:hover {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

.ai-assist-close svg {
  width: 16px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.5;
}

nav {
  display: flex;
  min-height: 42px;
  gap: 0;
  overflow-x: auto;
  padding: 0 16px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

nav button {
  min-height: 42px;
  padding: 0 13px;
  border: 0;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  font-size: 9px;
  font-weight: 650;
  white-space: nowrap;
}

nav button:hover {
  color: var(--color-text-primary);
}

nav button.active {
  border-color: var(--color-accent);
  color: var(--color-text-primary);
}

.ai-assist-config {
  margin: 18px;
  padding: 13px 14px;
  border: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
  color: var(--color-text-secondary);
  font-size: 9px;
}

.ai-assist-config button {
  min-height: 30px;
  margin-left: 10px;
  padding: 0 11px;
}

.ai-assist-prompt {
  display: grid;
  gap: 7px;
  padding: 16px 18px 8px;
}

.ai-assist-prompt > span {
  display: flex;
  align-items: baseline;
  gap: 9px;
}

.ai-assist-prompt b {
  color: var(--color-text-muted);
  font: 8px "SFMono-Regular", Consolas, monospace;
  letter-spacing: 0.12em;
}

.ai-assist-prompt em {
  color: var(--color-text-secondary);
  font-size: 9px;
  font-style: normal;
}

.ai-assist-prompt textarea {
  min-height: 78px;
  max-height: 150px;
  resize: vertical;
  padding: 10px 11px;
  border: 1px solid var(--color-border-strong);
  outline: 0;
  background: var(--color-bg-elevated);
  color: var(--color-text-primary);
  font: 10px/1.6 "SFMono-Regular", Consolas, monospace;
}

.ai-assist-prompt textarea:focus {
  border-color: var(--color-accent);
  box-shadow: inset 0 -2px var(--color-accent);
}

.ai-assist-prompt textarea::placeholder {
  color: var(--color-text-muted);
  opacity: 0.72;
}

.ai-assist-actions,
footer {
  display: flex;
  min-height: 40px;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding: 0 18px 10px;
}

.ai-assist-actions small {
  margin-right: auto;
  color: var(--color-text-muted);
  font-size: 8px;
}

.ai-assist-actions button,
footer button {
  min-height: 32px;
  padding: 0 13px;
  border: 1px solid var(--color-border-strong);
  border-radius: 0;
  background: var(--color-bg-elevated);
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: 9px;
}

.ai-assist-actions button:hover,
footer button:hover {
  border-color: var(--color-accent);
  color: var(--color-text-primary);
}

.ai-assist-actions button:disabled {
  cursor: default;
  opacity: 0.42;
}

.primary {
  border-color: var(--color-accent) !important;
  background: var(--color-accent) !important;
  color: var(--color-accent-contrast, #fff) !important;
}

.ai-assist-error {
  margin: 0 18px 9px;
  padding: 9px 10px;
  border: 1px solid var(--color-danger);
  background: var(--color-danger-surface);
  color: var(--color-danger-text);
  font-size: 8px;
}

.ai-assist-result {
  min-height: 0;
  margin: 0 18px 12px;
  overflow: hidden;
  border: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

.ai-assist-result > header {
  display: flex;
  min-height: 45px;
  align-items: center;
  justify-content: space-between;
  padding: 7px 11px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-muted);
}

.ai-assist-result > header strong {
  display: block;
  margin-top: 2px;
  font-size: 11px;
}

.ai-assist-result > header i {
  display: flex;
  align-items: center;
  gap: 5px;
  color: var(--color-success-text);
  font: 7px "SFMono-Regular", Consolas, monospace;
  font-style: normal;
  letter-spacing: 0.08em;
}

.ai-assist-result > header i span {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: var(--color-success);
  animation: ai-assist-pulse 1.1s ease-in-out infinite;
}

@keyframes ai-assist-pulse {
  50% { opacity: 0.35; }
}

.ai-assist-output {
  min-height: 150px;
  max-height: 260px;
  overflow: auto;
  background: var(--terminal-bg, #151a17);
  color: var(--terminal-text, var(--color-text-primary));
}

.ai-assist-output > span {
  display: grid;
  min-height: 150px;
  place-items: center;
  color: var(--color-text-muted);
  font-size: 9px;
}

.ai-assist-markdown {
  padding: 13px 14px;
  color: inherit;
  font: 10px/1.7 "SFMono-Regular", Consolas, monospace;
  overflow-wrap: anywhere;
}

.ai-assist-markdown :deep(p) {
  margin: 0 0 9px;
}

.ai-assist-markdown :deep(p:last-child),
.ai-assist-markdown :deep(ul:last-child),
.ai-assist-markdown :deep(ol:last-child),
.ai-assist-markdown :deep(pre:last-child) {
  margin-bottom: 0;
}

.ai-assist-markdown :deep(h2),
.ai-assist-markdown :deep(h3),
.ai-assist-markdown :deep(h4),
.ai-assist-markdown :deep(h5) {
  margin: 13px 0 7px;
  color: var(--color-text-primary);
  font-family: inherit;
  font-size: 11px;
}

.ai-assist-markdown :deep(ul),
.ai-assist-markdown :deep(ol) {
  margin: 7px 0 10px;
  padding-left: 20px;
}

.ai-assist-markdown :deep(code) {
  color: var(--color-warning-text);
}

.ai-assist-markdown :deep(pre) {
  margin: 9px 0;
  overflow: auto;
  padding: 10px;
  border: 1px solid var(--color-border);
  background: color-mix(in srgb, var(--terminal-bg, #151a17) 82%, black);
}

.ai-assist-markdown :deep(pre code) {
  color: inherit;
  white-space: pre;
}

.ai-assist-markdown :deep(a) {
  color: var(--color-accent);
}

footer {
  padding-top: 0;
}

@media (max-width: 700px) {
  .ai-assist-backdrop { padding: 12px; }
  .ai-assist-dialog {
    width: 100%;
    max-height: calc(100vh - 24px);
  }
  .ai-assist-actions small { display: none; }
}
</style>
