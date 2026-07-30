<script setup lang="ts">
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { convertFileSrc } from "@tauri-apps/api/core";
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  cancelAiChatMessage,
  createAiChatSession,
  deleteAiChatSession,
  listAiChatMessages,
  listAiChatSessions,
  sendAiChatMessage,
} from "../api/ai";
import { getAiSettings } from "../api/services";
import type {
  AiChatMessage,
  AiChatSession,
  AiChatStreamEvent,
  AiSettings,
} from "../types";
import { renderMarkdown } from "../utils/markdown";

const emit = defineEmits<{
  close: [];
  configure: [];
}>();
const { t } = useI18n();

const settings = ref<AiSettings | null>(null);
const sessions = ref<AiChatSession[]>([]);
const activeSessionId = ref("");
const messages = ref<AiChatMessage[]>([]);
const input = ref("");
const loading = ref(true);
const sending = ref(false);
const stopping = ref(false);
const error = ref("");
const activeRequestId = ref("");
const messageViewport = ref<HTMLElement | null>(null);
const composer = ref<HTMLTextAreaElement | null>(null);
const copiedMessageId = ref<number | null>(null);
let unlistenStream: UnlistenFn | undefined;

const userAvatarUrl = computed(() =>
  settings.value?.userAvatarPath
    ? convertFileSrc(settings.value.userAvatarPath)
    : "",
);
const assistantAvatarUrl = computed(() =>
  settings.value?.assistantAvatarPath
    ? convertFileSrc(settings.value.assistantAvatarPath)
    : "",
);

function requestId() {
  return globalThis.crypto?.randomUUID?.() ??
    `ai-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

async function scrollToBottom(behavior: ScrollBehavior = "smooth") {
  await nextTick();
  messageViewport.value?.scrollTo({
    top: messageViewport.value.scrollHeight,
    behavior,
  });
}

async function loadMessages(sessionId: string) {
  activeSessionId.value = sessionId;
  messages.value = await listAiChatMessages(sessionId);
  error.value = "";
  await scrollToBottom("auto");
}

async function refreshSessions() {
  sessions.value = await listAiChatSessions();
}

async function createSession(select = true) {
  const session = await createAiChatSession();
  await refreshSessions();
  if (select) await loadMessages(session.id);
  return session;
}

async function initialize() {
  loading.value = true;
  try {
    settings.value = await getAiSettings();
    await refreshSessions();
    if (sessions.value.length === 0) {
      await createSession();
    } else {
      await loadMessages(sessions.value[0].id);
    }
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

async function removeSession(session: AiChatSession) {
  if (sending.value) return;
  if (!window.confirm(t("aiChat.deleteConfirm", { title: session.title }))) {
    return;
  }
  try {
    await deleteAiChatSession(session.id);
    await refreshSessions();
    if (activeSessionId.value === session.id) {
      if (sessions.value.length) {
        await loadMessages(sessions.value[0].id);
      } else {
        await createSession();
      }
    }
  } catch (cause) {
    error.value = String(cause);
  }
}

async function chooseSession(sessionId: string) {
  if (sending.value || activeSessionId.value === sessionId) return;
  try {
    await loadMessages(sessionId);
  } catch (cause) {
    error.value = String(cause);
  }
}

async function sendMessage() {
  const content = input.value.trim();
  if (!content || sending.value) return;
  if (!settings.value?.enabled || !settings.value.apiKeyConfigured) {
    error.value = t("aiChat.configureFirst");
    return;
  }
  try {
    if (!activeSessionId.value) await createSession();
    const sessionId = activeSessionId.value;
    const id = requestId();
    activeRequestId.value = id;
    input.value = "";
    await nextTick();
    resizeComposer();
    error.value = "";
    sending.value = true;
    messages.value.push(
      {
        id: -Date.now(),
        sessionId,
        role: "user",
        content,
        createdAtMillis: Date.now(),
      },
      {
        id: -Date.now() - 1,
        sessionId,
        role: "assistant",
        content: "",
        createdAtMillis: Date.now(),
      },
    );
    await scrollToBottom();
    void sendAiChatMessage(sessionId, id, content).catch((cause) => {
      if (activeRequestId.value !== id || !sending.value) return;
      const assistant = messages.value.at(-1);
      if (assistant?.role === "assistant" && !assistant.content) {
        messages.value.pop();
      }
      sending.value = false;
      error.value = String(cause);
    });
  } catch (cause) {
    sending.value = false;
    error.value = String(cause);
  }
}

async function stopMessage() {
  if (!activeRequestId.value || stopping.value) return;
  stopping.value = true;
  try {
    await cancelAiChatMessage(activeRequestId.value);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    stopping.value = false;
  }
}

function handleComposerKeydown(event: KeyboardEvent) {
  if (event.key === "Enter" && !event.shiftKey) {
    event.preventDefault();
    void sendMessage();
  }
}

function resizeComposer() {
  const element = composer.value;
  if (!element) return;
  element.style.height = "auto";
  element.style.height = `${Math.min(element.scrollHeight, 132)}px`;
}

async function copyMessage(message: AiChatMessage) {
  await navigator.clipboard.writeText(message.content);
  copiedMessageId.value = message.id;
  window.setTimeout(() => {
    if (copiedMessageId.value === message.id) copiedMessageId.value = null;
  }, 1_400);
}

function useSuggestion(text: string) {
  input.value = text;
  void nextTick().then(resizeComposer);
}

async function finishStream() {
  sending.value = false;
  stopping.value = false;
  activeRequestId.value = "";
  await Promise.all([
    refreshSessions(),
    activeSessionId.value
      ? loadMessages(activeSessionId.value)
      : Promise.resolve(),
  ]);
}

onMounted(async () => {
  unlistenStream = await listen<AiChatStreamEvent>(
    "ai-chat-stream",
    async ({ payload }) => {
      if (
        payload.requestId !== activeRequestId.value ||
        payload.sessionId !== activeSessionId.value
      ) {
        return;
      }
      if (payload.event === "delta") {
        const assistant = messages.value.at(-1);
        if (assistant?.role === "assistant") {
          assistant.content += payload.content;
          await scrollToBottom();
        }
      } else if (payload.event === "error") {
        const assistant = messages.value.at(-1);
        if (assistant?.role === "assistant" && !assistant.content) {
          messages.value.pop();
        }
        error.value = payload.content;
        sending.value = false;
        stopping.value = false;
        activeRequestId.value = "";
      } else {
        await finishStream();
      }
    },
  );
  await initialize();
});

onUnmounted(() => {
  unlistenStream?.();
});
</script>

<template>
  <div class="ai-chat-backdrop" role="presentation">
    <section
      class="ai-chat-dialog"
      role="dialog"
      aria-modal="true"
      :aria-label="t('aiChat.title')"
    >
      <header class="ai-chat-header">
        <div class="ai-chat-title">
          <span class="ai-orbit-icon" aria-hidden="true">
            <img
              v-if="assistantAvatarUrl"
              :src="assistantAvatarUrl"
              alt=""
            />
            <svg v-else viewBox="0 0 24 24">
              <path d="M12 3.5a8.5 8.5 0 1 0 8.5 8.5" />
              <path d="M12 7.4a4.6 4.6 0 1 0 4.6 4.6" />
              <path d="M18.4 3.5v4.1h4.1M12 10.4V12l1.2 1.2" />
            </svg>
          </span>
          <div>
            <strong>{{ t("aiChat.title") }}</strong>
            <small>
              {{
                settings?.model
                  ? `${settings.model} · ${t("aiChat.localHistory")}`
                  : t("aiChat.subtitle")
              }}
            </small>
          </div>
        </div>
        <button
          type="button"
          class="ai-chat-close"
          :aria-label="t('common.close')"
          @click="emit('close')"
        >
          <svg viewBox="0 0 20 20" aria-hidden="true">
            <path d="m5 5 10 10M15 5 5 15" />
          </svg>
        </button>
      </header>

      <div class="ai-chat-layout">
        <aside class="ai-chat-sessions">
          <button
            type="button"
            class="ai-new-chat"
            :disabled="sending"
            @click="createSession()"
          >
            <svg viewBox="0 0 18 18" aria-hidden="true">
              <path d="M9 3v12M3 9h12" />
            </svg>
            {{ t("aiChat.newChat") }}
          </button>
          <div class="ai-session-list">
            <div
              v-for="session in sessions"
              :key="session.id"
              class="ai-session-item"
              :class="{ active: activeSessionId === session.id }"
            >
              <button
                type="button"
                class="ai-session-select"
                @click="chooseSession(session.id)"
              >
                <strong>{{ session.title }}</strong>
                <small>
                  {{
                    session.preview ||
                    t("aiChat.emptySession")
                  }}
                </small>
              </button>
              <button
                type="button"
                class="ai-session-delete"
                :title="t('common.delete')"
                @click="removeSession(session)"
              >
                ×
              </button>
            </div>
          </div>
          <p>{{ t("aiChat.storageHint") }}</p>
        </aside>

        <main class="ai-chat-main">
          <div v-if="loading" class="ai-chat-loading">
            <span class="spinner"></span>
            {{ t("common.loading") }}…
          </div>

          <template v-else>
            <div ref="messageViewport" class="ai-message-viewport">
              <div v-if="messages.length === 0" class="ai-chat-empty">
                <span class="ai-empty-mark">
                  <img
                    v-if="assistantAvatarUrl"
                    :src="assistantAvatarUrl"
                    alt=""
                  />
                  <template v-else>AI</template>
                </span>
                <h2>{{ t("aiChat.welcome") }}</h2>
                <p>{{ t("aiChat.welcomeHint") }}</p>
                <div>
                  <button
                    type="button"
                    @click="useSuggestion(t('aiChat.suggestionOne'))"
                  >
                    {{ t("aiChat.suggestionOne") }}
                  </button>
                  <button
                    type="button"
                    @click="useSuggestion(t('aiChat.suggestionTwo'))"
                  >
                    {{ t("aiChat.suggestionTwo") }}
                  </button>
                  <button
                    type="button"
                    @click="useSuggestion(t('aiChat.suggestionThree'))"
                  >
                    {{ t("aiChat.suggestionThree") }}
                  </button>
                </div>
              </div>

              <article
                v-for="message in messages"
                :key="message.id"
                class="ai-message"
                :class="message.role"
              >
                <span class="ai-message-avatar" aria-hidden="true">
                  <img
                    v-if="
                      message.role === 'assistant'
                        ? assistantAvatarUrl
                        : userAvatarUrl
                    "
                    :src="
                      message.role === 'assistant'
                        ? assistantAvatarUrl
                        : userAvatarUrl
                    "
                    alt=""
                  />
                  <svg
                    v-else-if="message.role === 'assistant'"
                    viewBox="0 0 28 28"
                  >
                    <path d="M14 3.5a10.5 10.5 0 1 0 10.5 10.5" />
                    <path d="M14 8a6 6 0 1 0 6 6" />
                    <path d="M20.7 3.7v5.1h5.1M14 11.2V14l2 2" />
                  </svg>
                  <svg v-else viewBox="0 0 28 28">
                    <circle cx="14" cy="10" r="4.2" />
                    <path d="M6.5 23c.8-4.3 3.3-6.5 7.5-6.5s6.7 2.2 7.5 6.5" />
                  </svg>
                </span>
                <div class="ai-message-content">
                  <div
                    v-if="message.content && message.role === 'assistant'"
                    class="ai-markdown"
                    v-html="renderMarkdown(message.content)"
                  ></div>
                  <p v-else-if="message.content">{{ message.content }}</p>
                  <div v-else class="ai-thinking">
                    <i></i><i></i><i></i>
                    {{ t("aiChat.thinking") }}
                  </div>
                  <button
                    v-if="message.content"
                    type="button"
                    class="ai-message-copy"
                    @click="copyMessage(message)"
                  >
                    <svg viewBox="0 0 18 18" aria-hidden="true">
                      <rect x="6" y="5" width="8" height="9" rx="1.5" />
                      <path d="M4 11H3.5A1.5 1.5 0 0 1 2 9.5v-6A1.5 1.5 0 0 1 3.5 2h6A1.5 1.5 0 0 1 11 3.5V4" />
                    </svg>
                    {{
                      copiedMessageId === message.id
                        ? t("aiChat.copied")
                        : t("aiChat.copy")
                    }}
                  </button>
                </div>
              </article>
            </div>

            <div v-if="error" class="ai-chat-error">
              <span>!</span>
              <p>{{ error }}</p>
              <button
                v-if="!settings?.enabled || !settings?.apiKeyConfigured"
                type="button"
                @click="emit('configure')"
              >
                {{ t("aiChat.openSettings") }}
              </button>
            </div>

            <footer class="ai-chat-composer">
              <textarea
                ref="composer"
                v-model="input"
                rows="1"
                :disabled="sending"
                :placeholder="t('aiChat.placeholder')"
                @keydown="handleComposerKeydown"
                @input="resizeComposer"
              ></textarea>
              <button
                v-if="sending"
                type="button"
                class="ai-stop-button"
                :disabled="stopping"
                @click="stopMessage"
              >
                <span></span>
                {{ stopping ? t("aiChat.stopping") : t("aiChat.stop") }}
              </button>
              <button
                v-else
                type="button"
                class="ai-send-button"
                :disabled="!input.trim()"
                @click="sendMessage"
              >
                <svg viewBox="0 0 20 20" aria-hidden="true">
                  <path d="m4 10 12-6-3.8 12-2.4-4.2L4 10Zm5.8 1.8L16 4" />
                </svg>
              </button>
              <small>{{ t("aiChat.shortcut") }}</small>
            </footer>
          </template>
        </main>
      </div>
    </section>
  </div>
</template>

<style scoped>
.ai-chat-backdrop {
  position: fixed;
  z-index: 720;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 28px;
  background: color-mix(in srgb, var(--color-bg-sidebar) 42%, transparent);
  backdrop-filter: blur(7px) saturate(90%);
}

.ai-chat-dialog {
  display: grid;
  width: min(940px, calc(100vw - 56px));
  height: min(720px, calc(100vh - 56px));
  overflow: hidden;
  grid-template-rows: 62px minmax(0, 1fr);
  border: 1px solid var(--color-border-strong);
  border-radius: 10px;
  background: var(--color-bg-content);
  box-shadow: 0 24px 70px rgb(0 0 0 / 0.28);
}

.ai-chat-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 16px 0 18px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-header);
}

.ai-chat-title {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 11px;
}

.ai-orbit-icon {
  display: grid;
  width: 34px;
  height: 34px;
  flex: 0 0 auto;
  overflow: hidden;
  place-items: center;
  border-radius: 9px;
  background: color-mix(in srgb, var(--color-accent) 14%, var(--color-bg-muted));
  color: var(--color-accent);
}

.ai-orbit-icon svg {
  width: 22px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.6;
}

.ai-orbit-icon img,
.ai-empty-mark img,
.ai-message-avatar img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.ai-chat-title strong,
.ai-chat-title small {
  display: block;
}

.ai-chat-title strong {
  font-size: 13px;
}

.ai-chat-title small {
  overflow: hidden;
  margin-top: 3px;
  color: var(--color-text-muted);
  font: 8px "SFMono-Regular", Consolas, monospace;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ai-chat-close {
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

.ai-chat-close:hover {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

.ai-chat-close svg {
  width: 17px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.5;
}

.ai-chat-layout {
  display: grid;
  min-height: 0;
  grid-template-columns: 210px minmax(0, 1fr);
}

.ai-chat-sessions {
  display: grid;
  min-height: 0;
  grid-template-rows: auto minmax(0, 1fr) auto;
  padding: 12px;
  border-right: 1px solid var(--color-border);
  background: var(--color-bg-sidebar);
}

.ai-new-chat {
  display: flex;
  height: 34px;
  align-items: center;
  justify-content: center;
  gap: 7px;
  border: 1px solid var(--color-border-strong);
  border-radius: 5px;
  background: var(--color-bg-elevated);
  color: var(--color-text-primary);
  cursor: pointer;
  font-size: 9px;
  font-weight: 650;
}

.ai-new-chat:hover {
  border-color: var(--color-accent);
}

.ai-new-chat svg {
  width: 14px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.5;
}

.ai-session-list {
  overflow: auto;
  margin: 10px -5px 0;
  padding: 0 5px;
}

.ai-session-item {
  display: grid;
  width: 100%;
  grid-template-columns: minmax(0, 1fr) 18px;
  align-items: center;
  gap: 4px;
  padding: 9px 7px 9px 9px;
  border: 0;
  border-radius: 5px;
  background: transparent;
  color: var(--color-text-primary);
}

.ai-session-item:hover {
  background: var(--color-hover);
}

.ai-session-item.active {
  background: color-mix(in srgb, var(--color-accent) 10%, var(--color-bg-elevated));
  box-shadow: inset 2px 0 var(--color-accent);
}

.ai-session-select {
  min-width: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
  text-align: left;
}

.ai-session-select strong,
.ai-session-select small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ai-session-select strong {
  font-size: 9px;
}

.ai-session-select small {
  margin-top: 4px;
  color: var(--color-text-muted);
  font-size: 7px;
}

.ai-session-delete {
  display: grid;
  width: 18px;
  height: 18px;
  place-items: center;
  border-radius: 3px;
  padding: 0;
  border: 0;
  background: transparent;
  color: transparent;
  cursor: pointer;
}

.ai-session-item:hover .ai-session-delete {
  color: var(--color-text-muted);
}

.ai-session-delete:hover {
  background: var(--color-danger-surface);
  color: var(--color-danger-text);
}

.ai-chat-sessions > p {
  margin: 10px 2px 0;
  color: var(--color-text-muted);
  font-size: 7px;
  line-height: 1.5;
}

.ai-chat-main {
  display: grid;
  min-width: 0;
  min-height: 0;
  grid-template-rows: minmax(0, 1fr) auto auto;
  background-image:
    linear-gradient(var(--color-grid-line) 1px, transparent 1px),
    linear-gradient(90deg, var(--color-grid-line) 1px, transparent 1px);
  background-size: 36px 36px;
}

.ai-chat-loading {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  color: var(--color-text-muted);
  font-size: 9px;
}

.ai-message-viewport {
  overflow: auto;
  padding: 28px max(26px, calc((100% - 680px) / 2));
  scroll-behavior: smooth;
}

.ai-chat-empty {
  display: grid;
  min-height: 100%;
  place-content: center;
  justify-items: center;
  text-align: center;
}

.ai-empty-mark {
  display: grid;
  width: 46px;
  height: 46px;
  overflow: hidden;
  place-items: center;
  border: 1px solid color-mix(in srgb, var(--color-accent) 48%, var(--color-border));
  border-radius: 14px;
  background: var(--color-panel-translucent);
  color: var(--color-accent);
  font: 600 12px "SFMono-Regular", Consolas, monospace;
}

.ai-chat-empty h2 {
  margin: 13px 0 5px;
  font-size: 17px;
}

.ai-chat-empty p {
  max-width: 420px;
  margin: 0;
  color: var(--color-text-muted);
  font-size: 9px;
  line-height: 1.6;
}

.ai-chat-empty > div {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 6px;
  margin-top: 18px;
}

.ai-chat-empty button {
  padding: 7px 10px;
  border: 1px solid var(--color-border);
  border-radius: 4px;
  background: var(--color-panel-translucent);
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: 8px;
}

.ai-chat-empty button:hover {
  border-color: var(--color-accent);
  color: var(--color-text-primary);
}

.ai-message {
  display: flex;
  max-width: 100%;
  align-items: start;
  gap: 11px;
}

.ai-message + .ai-message {
  margin-top: 26px;
}

.ai-message.user {
  flex-direction: row-reverse;
}

.ai-message-avatar {
  display: grid;
  width: 30px;
  height: 30px;
  flex: 0 0 auto;
  overflow: hidden;
  place-items: center;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background:
    linear-gradient(145deg, var(--color-bg-elevated), var(--color-bg-muted));
  color: var(--color-text-secondary);
  box-shadow: 0 5px 14px rgb(0 0 0 / 0.08);
}

.ai-message-avatar svg {
  width: 19px;
  height: 19px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.55;
}

.ai-message.assistant .ai-message-avatar {
  border-color: color-mix(in srgb, var(--color-accent) 42%, var(--color-border));
  background:
    linear-gradient(
      145deg,
      color-mix(in srgb, var(--color-accent) 18%, var(--color-bg-elevated)),
      color-mix(in srgb, var(--color-accent) 7%, var(--color-bg-muted))
    );
  color: var(--color-accent);
}

.ai-message.user .ai-message-avatar {
  border-color: color-mix(in srgb, var(--color-info, #5b8fd1) 42%, var(--color-border));
  color: var(--color-info-text, #79a9df);
}

.ai-message-content {
  min-width: 0;
  max-width: min(82%, 620px);
  padding-top: 2px;
}

.ai-message.user .ai-message-content {
  display: grid;
  justify-items: end;
}

.ai-message-content > p {
  margin: 0;
  padding: 9px 12px;
  border: 1px solid color-mix(in srgb, var(--color-border-strong) 76%, transparent);
  border-radius: 12px 3px 12px 12px;
  background: color-mix(in srgb, var(--color-accent) 10%, var(--color-bg-elevated));
  color: var(--color-text-primary);
  font-size: 10px;
  line-height: 1.65;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}

.ai-markdown {
  color: var(--color-text-primary);
  font-size: 10px;
  line-height: 1.78;
  overflow-wrap: anywhere;
}

.ai-markdown :deep(p) {
  margin: 0 0 11px;
}

.ai-markdown :deep(p:last-child),
.ai-markdown :deep(ul:last-child),
.ai-markdown :deep(ol:last-child),
.ai-markdown :deep(pre:last-child),
.ai-markdown :deep(blockquote:last-child) {
  margin-bottom: 0;
}

.ai-markdown :deep(h2),
.ai-markdown :deep(h3),
.ai-markdown :deep(h4),
.ai-markdown :deep(h5) {
  margin: 18px 0 8px;
  color: var(--color-text-primary);
  line-height: 1.35;
}

.ai-markdown :deep(h2) { font-size: 15px; }
.ai-markdown :deep(h3) { font-size: 13px; }
.ai-markdown :deep(h4),
.ai-markdown :deep(h5) { font-size: 11px; }

.ai-markdown :deep(ul),
.ai-markdown :deep(ol) {
  margin: 7px 0 12px;
  padding-left: 21px;
}

.ai-markdown :deep(li) {
  margin: 4px 0;
  padding-left: 2px;
}

.ai-markdown :deep(strong) {
  color: var(--color-text-primary);
  font-weight: 720;
}

.ai-markdown :deep(a) {
  color: var(--color-accent);
  text-decoration: none;
}

.ai-markdown :deep(a:hover) {
  text-decoration: underline;
}

.ai-markdown :deep(code) {
  padding: 2px 5px;
  border: 1px solid var(--color-border);
  border-radius: 4px;
  background: var(--color-bg-muted);
  color: var(--color-warning-text);
  font: 9px/1.5 "SFMono-Regular", Consolas, monospace;
}

.ai-markdown :deep(pre) {
  position: relative;
  margin: 11px 0;
  overflow: auto;
  padding: 13px 14px;
  border: 1px solid var(--color-border);
  border-radius: 7px;
  background: var(--terminal-bg, #111814);
}

.ai-markdown :deep(pre[data-language]:not([data-language=""])::before) {
  display: block;
  margin-bottom: 8px;
  color: var(--color-text-muted);
  content: attr(data-language);
  font: 7px "SFMono-Regular", Consolas, monospace;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.ai-markdown :deep(pre code) {
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--terminal-text, var(--color-text-primary));
  white-space: pre;
}

.ai-markdown :deep(blockquote) {
  margin: 10px 0;
  padding: 7px 11px;
  border-left: 3px solid var(--color-accent);
  background: var(--color-bg-muted);
  color: var(--color-text-secondary);
}

.ai-markdown :deep(hr) {
  margin: 17px 0;
  border: 0;
  border-top: 1px solid var(--color-border);
}

.ai-message-copy {
  display: flex;
  min-height: 25px;
  align-items: center;
  gap: 5px;
  margin-top: 7px;
  padding: 0 7px;
  border: 0;
  border-radius: 4px;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  font-size: 7px;
  opacity: 0;
  transition: opacity 120ms ease, background 120ms ease;
}

.ai-message:hover .ai-message-copy,
.ai-message-copy:focus-visible {
  opacity: 1;
}

.ai-message-copy:hover {
  background: var(--color-hover);
  color: var(--color-text-primary);
}

.ai-message-copy svg {
  width: 12px;
  height: 12px;
  fill: none;
  stroke: currentColor;
  stroke-width: 1.4;
}

.ai-thinking {
  display: flex;
  align-items: center;
  gap: 4px;
  color: var(--color-text-muted);
  font-size: 8px;
}

.ai-thinking i {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  animation: ai-pulse 1.2s ease-in-out infinite;
  background: var(--color-accent);
}

.ai-thinking i:nth-child(2) { animation-delay: 150ms; }
.ai-thinking i:nth-child(3) { animation-delay: 300ms; }

@keyframes ai-pulse {
  0%, 70%, 100% { opacity: 0.25; transform: translateY(0); }
  35% { opacity: 1; transform: translateY(-2px); }
}

.ai-chat-error {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 18px 8px;
  padding: 8px 10px;
  border: 1px solid color-mix(in srgb, var(--color-danger) 45%, var(--color-border));
  border-radius: 5px;
  background: var(--color-danger-surface);
  color: var(--color-danger-text);
}

.ai-chat-error > span {
  display: grid;
  width: 18px;
  height: 18px;
  flex: 0 0 auto;
  place-items: center;
  border-radius: 50%;
  background: var(--color-danger);
  color: white;
  font-size: 8px;
}

.ai-chat-error p {
  flex: 1;
  margin: 0;
  font-size: 8px;
  line-height: 1.45;
}

.ai-chat-error button {
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
  font-size: 8px;
  font-weight: 650;
  text-decoration: underline;
}

.ai-chat-composer {
  position: relative;
  margin: 0 18px 16px;
  padding: 10px 48px 24px 12px;
  border: 1px solid var(--color-border-strong);
  border-radius: 7px;
  background: var(--color-bg-elevated);
  box-shadow: 0 8px 24px rgb(0 0 0 / 0.06);
}

.ai-chat-composer:focus-within {
  border-color: var(--color-accent);
  box-shadow:
    inset 0 0 0 1px var(--color-focus),
    0 8px 24px rgb(0 0 0 / 0.06);
}

.ai-chat-composer textarea {
  display: block;
  width: 100%;
  height: 25px;
  min-height: 25px;
  max-height: 132px;
  resize: none;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--color-text-primary);
  font-family: inherit;
  font-size: 10px;
  font-weight: 450;
  line-height: 1.55;
}

.ai-chat-composer textarea::placeholder {
  color: var(--color-text-muted);
  font-size: 10px;
  opacity: 0.82;
}

.ai-chat-composer > button {
  position: absolute;
  right: 7px;
  bottom: 9px;
  display: grid;
  height: 28px;
  place-items: center;
  border: 0;
  border-radius: 5px;
  cursor: pointer;
}

.ai-send-button {
  width: 30px;
  background: var(--color-accent);
  color: white;
}

.ai-send-button:disabled {
  cursor: default;
  opacity: 0.38;
}

.ai-send-button svg {
  width: 16px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.5;
}

.ai-chat-composer > .ai-stop-button {
  display: flex;
  width: auto;
  gap: 6px;
  padding: 0 9px;
  background: var(--color-bg-muted);
  color: var(--color-text-secondary);
  font-size: 8px;
}

.ai-stop-button span {
  width: 7px;
  height: 7px;
  border-radius: 1px;
  background: currentColor;
}

.ai-chat-composer > small {
  position: absolute;
  bottom: 7px;
  left: 12px;
  color: var(--color-text-muted);
  font-size: 8px;
  line-height: 1;
}

@media (max-width: 760px) {
  .ai-chat-backdrop { padding: 14px; }
  .ai-chat-dialog {
    width: calc(100vw - 28px);
    height: calc(100vh - 28px);
  }
  .ai-chat-layout { grid-template-columns: 150px minmax(0, 1fr); }
  .ai-chat-sessions { padding: 9px; }
  .ai-message-viewport { padding-inline: 18px; }
}
</style>
