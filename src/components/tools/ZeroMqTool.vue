<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  publishZeroMq,
  pullZeroMq,
  pushZeroMq,
  subscribeZeroMq,
} from "../../api/tools";
import type { ZeroMqResult } from "../../types";

type Pattern = "pubsub" | "pipeline";
type Role = "send" | "receive";

const { t } = useI18n();
const pattern = ref<Pattern>("pubsub");
const role = ref<Role>("send");
const endpoint = ref("tcp://127.0.0.1:5555");
const bind = ref(true);
const topic = ref("dev.events");
const payload = ref('{\n  "message": "Hello ZeroMQ"\n}');
const timeoutSeconds = ref(10);
const loading = ref(false);
const error = ref("");
const results = ref<ZeroMqResult[]>([]);

const actionLabel = computed(() => {
  if (loading.value) return role.value === "send" ? t("zeroMq.publishing") : t("zeroMq.receiving");
  if (pattern.value === "pubsub") return role.value === "send" ? t("zeroMq.publish") : t("zeroMq.subscribe");
  return role.value === "send" ? t("zeroMq.push") : t("zeroMq.pull");
});

function selectPattern(next: Pattern) {
  pattern.value = next;
  endpoint.value =
    next === "pubsub" ? "tcp://127.0.0.1:5555" : "tcp://127.0.0.1:5556";
}

async function execute() {
  if (loading.value || !endpoint.value.trim()) return;
  loading.value = true;
  error.value = "";
  try {
    let result: ZeroMqResult;
    if (pattern.value === "pubsub" && role.value === "send") {
      result = await publishZeroMq(endpoint.value, bind.value, topic.value, payload.value);
    } else if (pattern.value === "pubsub") {
      result = await subscribeZeroMq(
        endpoint.value,
        bind.value,
        topic.value,
        timeoutSeconds.value,
      );
    } else if (role.value === "send") {
      result = await pushZeroMq(endpoint.value, bind.value, payload.value);
    } else {
      result = await pullZeroMq(endpoint.value, bind.value, timeoutSeconds.value);
    }
    results.value.unshift(result);
    results.value = results.value.slice(0, 100);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

function formatTime(value: number) {
  return new Date(value).toLocaleTimeString([], {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo zeromq">Ø</span>
      <div>
        <div class="title-line"><h1>{{ t("zeroMq.title") }}</h1><span>BROKERLESS MESSAGING</span></div>
        <p>{{ t("zeroMq.subtitle") }}</p>
      </div>
    </div>
  </header>

  <main class="zmq-page">
    <nav class="zmq-patterns">
      <button :class="{ active: pattern === 'pubsub' }" @click="selectPattern('pubsub')">
        <strong>PUB / SUB</strong><small>{{ t("zeroMq.pubSubHint") }}</small>
      </button>
      <button :class="{ active: pattern === 'pipeline' }" @click="selectPattern('pipeline')">
        <strong>PUSH / PULL</strong><small>{{ t("zeroMq.pipelineHint") }}</small>
      </button>
    </nav>

    <section class="zmq-console">
      <header>
        <div><small>SOCKET</small><h2>{{ t("zeroMq.endpointTitle") }}</h2></div>
        <div class="role-switch">
          <button :class="{ active: role === 'send' }" @click="role = 'send'">
            {{ pattern === "pubsub" ? "PUB" : "PUSH" }}
          </button>
          <button :class="{ active: role === 'receive' }" @click="role = 'receive'">
            {{ pattern === "pubsub" ? "SUB" : "PULL" }}
          </button>
        </div>
      </header>

      <div class="endpoint-row">
        <label class="endpoint-field">Endpoint
          <input v-model="endpoint" spellcheck="false" @keyup.enter="execute" />
        </label>
        <label>{{ t("zeroMq.connectionMode") }}
          <select v-model="bind">
            <option :value="true">{{ t("zeroMq.bind") }}</option>
            <option :value="false">{{ t("zeroMq.connect") }}</option>
          </select>
        </label>
        <label v-if="role === 'receive'">{{ t("zeroMq.timeout") }}
          <select v-model.number="timeoutSeconds">
            <option :value="3">3 s</option>
            <option :value="10">10 s</option>
            <option :value="30">30 s</option>
            <option :value="60">60 s</option>
          </select>
        </label>
      </div>

      <div class="zmq-editor">
        <label v-if="pattern === 'pubsub'">{{ t("zeroMq.topic") }}
          <input v-model="topic" spellcheck="false" :placeholder="t('zeroMq.topicPlaceholder')" />
        </label>
        <label v-if="role === 'send'">{{ t("zeroMq.payload") }}
          <textarea v-model="payload" spellcheck="false"></textarea>
        </label>
        <div v-else class="receive-ready">
          <span>◎</span>
          <strong>{{ t("zeroMq.waitingTitle") }}</strong>
          <p>{{ t("zeroMq.waitingHint") }}</p>
        </div>
        <footer>
          <span>{{ t("zeroMq.limitHint") }}</span>
          <button class="primary" :disabled="loading" @click="execute">
            <span v-if="loading" class="spinner"></span>{{ actionLabel }}
          </button>
        </footer>
      </div>
    </section>

    <div v-if="error" class="notice danger"><span>{{ error }}</span><button @click="error = ''">×</button></div>

    <section class="zmq-history">
      <header><div><small>MESSAGE LOG</small><h2>{{ t("zeroMq.logTitle") }}</h2></div><button :disabled="!results.length" @click="results=[]">{{ t("zeroMq.clear") }}</button></header>
      <div v-if="!results.length" class="zmq-empty">{{ t("zeroMq.empty") }}</div>
      <article v-for="(item, index) in results" :key="`${item.timestampMillis}-${index}`">
        <div>
          <span :class="item.direction">{{ item.direction === "sent" ? t("zeroMq.sent") : t("zeroMq.received") }}</span>
          <strong>{{ item.pattern }}</strong>
          <time>{{ formatTime(item.timestampMillis) }}</time>
          <small>{{ item.bytes }} B · {{ item.endpoint }}</small>
        </div>
        <pre>{{ item.frames.join("\n— frame —\n") }}</pre>
      </article>
    </section>

    <p class="zmq-note">
      {{ t("zeroMq.note") }}
    </p>
  </main>
</template>

<style scoped>
.zmq-page{display:grid;gap:14px;padding:24px 32px 36px}.zmq-patterns{display:flex;gap:7px}.zmq-patterns button{display:grid;min-width:210px;gap:3px;padding:10px 14px;border:1px solid var(--color-border);background:var(--color-panel-translucent);text-align:left}.zmq-patterns button.active{border-color:var(--color-accent);box-shadow:inset 3px 0 var(--color-accent)}.zmq-patterns strong{font-size:10px}.zmq-patterns small{color:var(--color-text-muted);font-size:8px}.zmq-console,.zmq-history{border:1px solid var(--color-border);background:var(--color-panel-translucent)}.zmq-console>header,.zmq-history>header{display:flex;min-height:62px;align-items:center;justify-content:space-between;padding:10px 14px;border-bottom:1px solid var(--color-border)}header small{color:var(--color-text-muted);font:8px "SFMono-Regular",Consolas,monospace;letter-spacing:.13em}header h2{margin:4px 0 0;font-size:13px}.role-switch{display:flex}.role-switch button{min-width:78px;min-height:32px}.role-switch button.active{border-color:var(--color-accent);color:var(--color-accent)}.endpoint-row{display:grid;grid-template-columns:minmax(320px,1fr) 180px 120px;gap:12px;padding:14px;border-bottom:1px solid var(--color-border)}label{display:grid;gap:6px;color:var(--color-text-muted);font-size:8px}.endpoint-row input,.endpoint-row select,.zmq-editor input{height:36px;padding:0 10px;font:9px "SFMono-Regular",Consolas,monospace}.zmq-editor{display:grid;gap:12px;padding:14px}.zmq-editor textarea{min-height:180px;padding:12px;resize:vertical;background:var(--terminal-bg);color:#e7ece3;font:9px/1.6 "SFMono-Regular",Consolas,monospace}.zmq-editor footer{display:flex;align-items:center;justify-content:space-between}.zmq-editor footer>span{color:var(--color-text-muted);font-size:8px}.zmq-editor footer button{min-width:110px}.receive-ready{display:grid;min-height:180px;place-items:center;align-content:center;gap:8px;border:1px dashed var(--color-border);color:var(--color-text-muted)}.receive-ready span{font-size:24px;color:var(--color-accent)}.receive-ready strong{font-size:11px;color:var(--color-text)}.receive-ready p{margin:0;font-size:8px}.zmq-history>header button{min-height:28px;font-size:8px}.zmq-empty{display:grid;min-height:170px;place-items:center;color:var(--color-text-muted);font-size:9px}.zmq-history article{display:grid;grid-template-columns:250px minmax(0,1fr);border-top:1px solid var(--color-border)}.zmq-history article:first-of-type{border-top:0}.zmq-history article>div{display:grid;grid-template-columns:auto 1fr;align-content:start;gap:7px;padding:12px;border-right:1px solid var(--color-border)}.zmq-history article span{width:max-content;padding:3px 6px;border:1px solid var(--color-accent);color:var(--color-accent);font-size:7px}.zmq-history article span.received{border-color:var(--color-success);color:var(--color-success)}.zmq-history article strong{font-size:9px}.zmq-history article time,.zmq-history article small{grid-column:1/-1;color:var(--color-text-muted);font:8px "SFMono-Regular",Consolas,monospace}.zmq-history pre{margin:0;padding:12px;overflow:auto;background:var(--terminal-bg);color:#e7ece3;font:9px/1.6 "SFMono-Regular",Consolas,monospace;white-space:pre-wrap;word-break:break-word}.zmq-note{margin:0;color:var(--color-text-muted);font-size:8px;line-height:1.7}@media(max-width:980px){.endpoint-row{grid-template-columns:1fr 1fr}.endpoint-field{grid-column:1/-1}.zmq-history article{grid-template-columns:1fr}.zmq-history article>div{border-right:0;border-bottom:1px solid var(--color-border)}}
</style>
