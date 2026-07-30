<script setup lang="ts">
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  rssAiCancel,
  rssAiGenerate,
  rssAiResultDelete,
  rssAiResultsList,
} from "../../api/services";
import { getAiSettings } from "../../api/services";
import type {
  AiSettings,
  RssAiAction,
  RssAiResult,
  RssAiStreamEvent,
  RssEntry,
} from "../../types";

const props = defineProps<{ entry: RssEntry }>();
const emit = defineEmits<{ close: [] }>();
const { t, locale } = useI18n();

const actions: RssAiAction[] = [
  "summary",
  "translate",
  "key_points",
  "question",
];
const activeAction = ref<RssAiAction>("summary");
const results = ref<RssAiResult[]>([]);
const settings = ref<AiSettings | null>(null);
const question = ref("");
const output = ref("");
const error = ref("");
const loading = ref(true);
const generating = ref(false);
const stopping = ref(false);
const activeRequestId = ref("");
let unlisten: UnlistenFn | undefined;

const configured = computed(
  () => settings.value?.enabled && settings.value.apiKeyConfigured,
);
const outputLanguage = computed(() =>
  locale.value.toLowerCase().startsWith("en") ? "en" : "zh-CN",
);
const activeResult = computed(
  () =>
    results.value.find(
      (result) =>
        result.action === activeAction.value &&
        result.outputLanguage === outputLanguage.value,
    ) ?? null,
);

function createRequestId() {
  return (
    globalThis.crypto?.randomUUID?.() ??
    `rss-ai-${Date.now()}-${Math.random().toString(16).slice(2)}`
  );
}

function openAiSettings() {
  window.dispatchEvent(new CustomEvent("zhiyu:open-ai-settings"));
}

async function loadResults() {
  loading.value = true;
  try {
    results.value = await rssAiResultsList(props.entry.id);
    output.value = activeResult.value?.content ?? "";
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

function chooseAction(action: RssAiAction) {
  if (generating.value) return;
  activeAction.value = action;
  output.value = activeResult.value?.content ?? "";
  error.value = "";
}

async function generate() {
  if (generating.value) return;
  if (!configured.value) {
    error.value = t("rss.aiReader.configureFirst");
    return;
  }
  if (activeAction.value === "question" && !question.value.trim()) {
    error.value = t("rss.aiReader.questionRequired");
    return;
  }
  const requestId = createRequestId();
  activeRequestId.value = requestId;
  output.value = "";
  error.value = "";
  generating.value = true;
  void rssAiGenerate({
    entryId: props.entry.id,
    requestId,
    action: activeAction.value,
    question: activeAction.value === "question" ? question.value.trim() : "",
    outputLanguage: outputLanguage.value,
  }).catch((cause) => {
    if (activeRequestId.value !== requestId || !generating.value) return;
    generating.value = false;
    activeRequestId.value = "";
    error.value = String(cause);
  });
}

async function stop() {
  if (!activeRequestId.value || stopping.value) return;
  stopping.value = true;
  try {
    await rssAiCancel(activeRequestId.value);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    stopping.value = false;
  }
}

async function removeResult() {
  if (!activeResult.value || generating.value) return;
  try {
    await rssAiResultDelete(activeResult.value.id);
    await loadResults();
  } catch (cause) {
    error.value = String(cause);
  }
}

async function copyOutput() {
  if (!output.value) return;
  try {
    await navigator.clipboard.writeText(output.value);
  } catch (cause) {
    error.value = String(cause);
  }
}

async function finish() {
  generating.value = false;
  stopping.value = false;
  activeRequestId.value = "";
  await loadResults();
}

watch(
  () => props.entry.id,
  async () => {
    if (generating.value && activeRequestId.value) {
      try {
        await rssAiCancel(activeRequestId.value);
      } catch {
        // The request may have completed while the article was changing.
      }
    }
    generating.value = false;
    stopping.value = false;
    activeRequestId.value = "";
    activeAction.value = "summary";
    question.value = "";
    output.value = "";
    await loadResults();
  },
);
watch(activeResult, (result) => {
  if (!generating.value) output.value = result?.content ?? "";
});

onMounted(async () => {
  unlisten = await listen<RssAiStreamEvent>("rss-ai-stream", ({ payload }) => {
    if (
      payload.entryId !== props.entry.id ||
      payload.requestId !== activeRequestId.value
    ) {
      return;
    }
    if (payload.event === "delta") {
      output.value += payload.content;
    } else if (payload.event === "error") {
      generating.value = false;
      stopping.value = false;
      activeRequestId.value = "";
      error.value = payload.content;
    } else {
      void finish();
    }
  });
  try {
    settings.value = await getAiSettings();
  } catch (cause) {
    error.value = String(cause);
  }
  await loadResults();
});

onUnmounted(() => {
  unlisten?.();
  if (activeRequestId.value) {
    void rssAiCancel(activeRequestId.value).catch(() => undefined);
  }
});
</script>

<template>
  <aside class="rss-ai-panel">
    <header>
      <div class="rss-ai-title">
        <span aria-hidden="true">
          <svg viewBox="0 0 24 24">
            <path d="M12 3.5a8.5 8.5 0 1 0 8.5 8.5" />
            <path d="M12 7.4a4.6 4.6 0 1 0 4.6 4.6" />
            <path d="M18.4 3.5v4.1h4.1M12 10.4V12l1.2 1.2" />
          </svg>
        </span>
        <div>
          <strong>{{ t("rss.aiReader.title") }}</strong>
          <small>{{ settings?.model || t("rss.aiReader.subtitle") }}</small>
        </div>
      </div>
      <button type="button" :title="t('common.close')" @click="emit('close')">×</button>
    </header>

    <nav class="rss-ai-actions">
      <button
        v-for="action in actions"
        :key="action"
        type="button"
        :class="{ active: activeAction === action }"
        :disabled="generating"
        @click="chooseAction(action)"
      >
        {{ t(`rss.aiReader.actions.${action}`) }}
      </button>
    </nav>

    <div v-if="activeAction === 'question'" class="rss-ai-question">
      <textarea
        v-model="question"
        rows="3"
        maxlength="2000"
        :disabled="generating"
        :placeholder="t('rss.aiReader.questionPlaceholder')"
        @keydown.meta.enter.prevent="generate"
        @keydown.ctrl.enter.prevent="generate"
      ></textarea>
    </div>

    <div class="rss-ai-output">
      <div v-if="loading" class="rss-ai-state">
        <span class="spinner"></span>{{ t("common.loading") }}…
      </div>
      <div v-else-if="!output && !generating" class="rss-ai-empty">
        <span>✦</span>
        <strong>{{ t(`rss.aiReader.empty.${activeAction}`) }}</strong>
        <p>{{ t("rss.aiReader.onDemandHint") }}</p>
      </div>
      <div v-else class="rss-ai-result">
        <div v-if="activeResult && !generating" class="rss-ai-result-meta">
          <span>{{ activeResult.model }}</span>
          <b v-if="activeResult.status === 'partial'">{{ t("rss.aiReader.partial") }}</b>
        </div>
        <pre>{{ output }}</pre>
        <span v-if="generating" class="rss-ai-cursor"></span>
      </div>
    </div>

    <div v-if="error" class="rss-ai-error">
      <span>!</span><p>{{ error }}</p>
    </div>

    <footer>
      <p>{{ t("rss.aiReader.privacyHint") }}</p>
      <div>
        <button v-if="activeResult && !generating" type="button" @click="removeResult">
          {{ t("common.delete") }}
        </button>
        <button v-if="output && !generating" type="button" @click="copyOutput">
          {{ t("common.copy") }}
        </button>
        <span></span>
        <button
          v-if="generating"
          type="button"
          class="rss-ai-stop"
          :disabled="stopping"
          @click="stop"
        >
          {{ stopping ? t("rss.aiReader.stopping") : t("rss.aiReader.stop") }}
        </button>
        <button
          v-else-if="!configured"
          type="button"
          class="primary"
          @click="openAiSettings"
        >
          {{ t("aiChat.openSettings") }}
        </button>
        <button
          v-else
          type="button"
          class="primary"
          :disabled="activeAction === 'question' && !question.trim()"
          @click="generate"
        >
          {{ activeResult ? t("rss.aiReader.regenerate") : t("rss.aiReader.generate") }}
        </button>
      </div>
    </footer>
  </aside>
</template>

<style scoped>
.rss-ai-panel{display:grid;min-width:0;overflow:hidden;grid-template-rows:58px auto auto minmax(0,1fr) auto auto;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.rss-ai-panel>header{display:flex;align-items:center;justify-content:space-between;padding:0 12px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.rss-ai-title{display:flex;min-width:0;align-items:center;gap:9px}.rss-ai-title>span{display:grid;width:30px;height:30px;flex:0 0 auto;place-items:center;border-radius:8px;background:color-mix(in srgb,var(--color-accent) 14%,var(--color-bg-panel));color:var(--color-accent)}.rss-ai-title svg{width:19px;fill:none;stroke:currentColor;stroke-linecap:round;stroke-linejoin:round;stroke-width:1.6}.rss-ai-title div{display:grid;min-width:0;gap:3px}.rss-ai-title strong{font-size:11px}.rss-ai-title small{overflow:hidden;color:var(--color-text-muted);font:7px/1.2 "SFMono-Regular",Consolas,monospace;text-overflow:ellipsis;white-space:nowrap}.rss-ai-panel>header>button{width:28px;min-height:28px;padding:0;border:0;background:transparent;font-size:15px}.rss-ai-actions{display:grid;grid-template-columns:repeat(4,1fr);border-bottom:1px solid var(--color-border)}.rss-ai-actions button{min-width:0;min-height:38px;padding:5px 3px;border:0;border-right:1px solid var(--color-border);background:transparent;color:var(--color-text-muted);font-size:8px}.rss-ai-actions button:last-child{border-right:0}.rss-ai-actions button.active{background:var(--color-panel-active);color:var(--color-accent);box-shadow:inset 0 -2px var(--color-accent)}.rss-ai-question{padding:10px 12px 0}.rss-ai-question textarea{box-sizing:border-box;width:100%;min-height:70px;resize:vertical;padding:9px 10px;border:1px solid var(--color-border);background:var(--color-input);color:var(--color-text-primary);font:9px/1.5 inherit}.rss-ai-output{min-height:0;overflow:auto}.rss-ai-state,.rss-ai-empty{display:grid;height:100%;min-height:220px;place-items:center;align-content:center;gap:8px;padding:24px;color:var(--color-text-muted);text-align:center}.rss-ai-state{grid-auto-flow:column}.rss-ai-empty>span{display:grid;width:42px;height:42px;place-items:center;border:1px solid var(--color-border);border-radius:12px;color:var(--color-accent);font-size:18px}.rss-ai-empty strong{color:var(--color-text-primary);font-size:11px}.rss-ai-empty p{max-width:250px;margin:0;font-size:8px;line-height:1.6}.rss-ai-result{padding:20px 18px 28px}.rss-ai-result-meta{display:flex;align-items:center;gap:7px;margin-bottom:14px;color:var(--color-text-muted);font:7px/1.2 "SFMono-Regular",Consolas,monospace}.rss-ai-result-meta b{padding:3px 5px;border:1px solid var(--color-warning);border-radius:8px;color:var(--color-warning);font-weight:500}.rss-ai-result pre{margin:0;color:var(--color-text-secondary);font:10px/1.8 inherit;white-space:pre-wrap;word-break:break-word}.rss-ai-cursor{display:inline-block;width:6px;height:12px;margin-left:2px;background:var(--color-accent);animation:rss-ai-blink 800ms steps(1) infinite}.rss-ai-error{display:flex;gap:8px;margin:0 12px 10px;padding:9px 10px;border:1px solid color-mix(in srgb,var(--color-danger) 45%,var(--color-border));background:var(--color-danger-surface);color:var(--color-danger-text)}.rss-ai-error span{font-weight:700}.rss-ai-error p{margin:0;font-size:8px;line-height:1.5}.rss-ai-panel>footer{display:grid;gap:8px;padding:10px 12px;border-top:1px solid var(--color-border);background:var(--color-bg-muted)}.rss-ai-panel>footer>p{margin:0;color:var(--color-text-muted);font-size:7px;line-height:1.4}.rss-ai-panel>footer>div{display:flex;gap:6px}.rss-ai-panel>footer>div>span{flex:1}.rss-ai-panel button{display:inline-flex;min-height:30px;align-items:center;justify-content:center;padding:6px 9px;border:1px solid var(--color-border-strong);background:var(--color-bg-panel);color:var(--color-text-primary);font-size:8px;cursor:pointer}.rss-ai-panel button.primary{border-color:var(--color-control-primary);background:var(--color-control-primary);color:#fff}.rss-ai-stop{border-color:var(--color-danger-text)!important;color:var(--color-danger-text)!important}@keyframes rss-ai-blink{50%{opacity:0}}
</style>
