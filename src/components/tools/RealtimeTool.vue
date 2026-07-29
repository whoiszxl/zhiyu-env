<script setup lang="ts">
import { computed, onUnmounted, ref } from "vue";

type Protocol = "websocket" | "sse";
type LogKind = "system" | "sent" | "received" | "error";

interface RealtimeLog {
  id: number;
  time: number;
  kind: LogKind;
  content: string;
}

const protocol = ref<Protocol>("websocket");
const url = ref("ws://127.0.0.1:8080/ws");
const sseEventName = ref("");
const message = ref('{\n  "type": "ping"\n}');
const connected = ref(false);
const connecting = ref(false);
const logs = ref<RealtimeLog[]>([]);
let socket: WebSocket | null = null;
let eventSource: EventSource | null = null;
let nextLogId = 1;

const statusLabel = computed(() =>
  connecting.value ? "连接中" : connected.value ? "已连接" : "未连接",
);

function appendLog(kind: LogKind, content: string) {
  logs.value.push({ id: nextLogId++, time: Date.now(), kind, content });
  if (logs.value.length > 500) logs.value.splice(0, logs.value.length - 500);
}

function switchProtocol(next: Protocol) {
  disconnect();
  protocol.value = next;
  url.value =
    next === "websocket"
      ? "ws://127.0.0.1:8080/ws"
      : "http://127.0.0.1:8080/events";
}

function connect() {
  if (connected.value || connecting.value) return;
  const target = url.value.trim();
  if (!target) return;
  connecting.value = true;
  appendLog("system", `正在连接 ${target}`);

  try {
    if (protocol.value === "websocket") {
      if (!/^wss?:\/\//i.test(target)) throw new Error("WebSocket 地址必须以 ws:// 或 wss:// 开头");
      socket = new WebSocket(target);
      socket.onopen = () => {
        connecting.value = false;
        connected.value = true;
        appendLog("system", "WebSocket 连接成功");
      };
      socket.onmessage = (event) => {
        if (typeof event.data === "string") appendLog("received", event.data);
        else if (event.data instanceof Blob) {
          appendLog("received", `[二进制消息] ${event.data.size} bytes`);
        } else {
          appendLog("received", "[二进制消息]");
        }
      };
      socket.onerror = () => appendLog("error", "WebSocket 连接或通信发生错误");
      socket.onclose = (event) => {
        connecting.value = false;
        connected.value = false;
        socket = null;
        appendLog(
          "system",
          `连接已关闭 · code ${event.code}${event.reason ? ` · ${event.reason}` : ""}`,
        );
      };
    } else {
      if (!/^https?:\/\//i.test(target)) throw new Error("SSE 地址必须以 http:// 或 https:// 开头");
      eventSource = new EventSource(target);
      eventSource.onopen = () => {
        connecting.value = false;
        connected.value = true;
        appendLog("system", "SSE 连接成功");
      };
      eventSource.onmessage = (event) => appendLog("received", event.data);
      if (sseEventName.value.trim()) {
        eventSource.addEventListener(sseEventName.value.trim(), (event) => {
          appendLog("received", `[${sseEventName.value.trim()}] ${(event as MessageEvent).data}`);
        });
      }
      eventSource.onerror = () => {
        connecting.value = false;
        connected.value = eventSource?.readyState === EventSource.OPEN;
        appendLog(
          "error",
          eventSource?.readyState === EventSource.CLOSED
            ? "SSE 连接已关闭"
            : "SSE 连接中断，浏览器将尝试重连",
        );
      };
    }
  } catch (cause) {
    connecting.value = false;
    connected.value = false;
    appendLog("error", cause instanceof Error ? cause.message : String(cause));
  }
}

function disconnect() {
  socket?.close(1000, "用户断开");
  socket = null;
  eventSource?.close();
  eventSource = null;
  connecting.value = false;
  if (connected.value) appendLog("system", "已主动断开连接");
  connected.value = false;
}

function sendMessage() {
  if (!socket || socket.readyState !== WebSocket.OPEN) return;
  socket.send(message.value);
  appendLog("sent", message.value);
}

function formatTime(value: number) {
  return new Date(value).toLocaleTimeString("zh-CN", {
    hour12: false,
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

async function copyLogs() {
  const content = logs.value
    .map((log) => `${formatTime(log.time)} [${log.kind}] ${log.content}`)
    .join("\n");
  await navigator.clipboard.writeText(content);
}

onUnmounted(disconnect);
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo realtime">↯</span>
      <div>
        <div class="title-line"><h1>WebSocket / SSE 调试器</h1><span>REALTIME CLIENT</span></div>
        <p>测试双向 WebSocket 消息和服务器推送事件流</p>
      </div>
    </div>
  </header>

  <section class="realtime-page">
    <nav class="protocol-tabs">
      <button :class="{ active: protocol === 'websocket' }" type="button" @click="switchProtocol('websocket')"><strong>WebSocket</strong><small>双向消息</small></button>
      <button :class="{ active: protocol === 'sse' }" type="button" @click="switchProtocol('sse')"><strong>Server-Sent Events</strong><small>服务器推送</small></button>
    </nav>

    <div class="connection-bar">
      <span :class="['connection-dot', { active: connected, pending: connecting }]"></span>
      <strong>{{ statusLabel }}</strong>
      <input v-model="url" :placeholder="protocol === 'websocket' ? 'ws://127.0.0.1:8080/ws' : 'http://127.0.0.1:8080/events'" :disabled="connected || connecting" @keyup.enter="connect" />
      <input v-if="protocol === 'sse'" v-model="sseEventName" class="event-input" placeholder="事件名（可选）" :disabled="connected || connecting" />
      <button v-if="!connected" class="primary" type="button" :disabled="connecting" @click="connect">{{ connecting ? "连接中" : "连接" }}</button>
      <button v-else type="button" @click="disconnect">断开</button>
    </div>

    <div class="realtime-layout">
      <article v-if="protocol === 'websocket'" class="message-panel">
        <div class="panel-head"><div><p>MESSAGE</p><h2>发送消息</h2></div></div>
        <textarea v-model="message" spellcheck="false"></textarea>
        <div class="send-bar"><span>文本或 JSON</span><button class="primary" type="button" :disabled="!connected" @click="sendMessage">发送</button></div>
      </article>

      <article class="log-panel">
        <div class="panel-head">
          <div><p>EVENT LOG</p><h2>通信日志</h2></div>
          <div><button type="button" :disabled="logs.length === 0" @click="copyLogs">复制</button><button type="button" :disabled="logs.length === 0" @click="logs = []">清空</button></div>
        </div>
        <div v-if="logs.length === 0" class="log-empty">连接服务后，消息与连接状态会显示在这里</div>
        <div v-else class="event-logs">
          <div v-for="log in logs" :key="log.id" :class="['event-row', log.kind]">
            <time>{{ formatTime(log.time) }}</time>
            <span>{{ { system: "状态", sent: "发送", received: "接收", error: "错误" }[log.kind] }}</span>
            <pre>{{ log.content }}</pre>
          </div>
        </div>
      </article>
    </div>

    <p class="realtime-note">
      WebSocket 与 SSE 使用系统 WebView 原生网络能力，不经过远程代理。浏览器接口不支持为连接添加任意请求头；需要鉴权时可使用 URL 查询参数或服务端 Cookie。
    </p>
  </section>
</template>

<style scoped>
.realtime-page{display:grid;gap:14px;padding:24px 32px 36px}.protocol-tabs{display:flex;gap:6px}.protocol-tabs button{display:grid;min-width:170px;gap:3px;padding:9px 14px;border:1px solid var(--color-border);background:var(--color-panel-translucent);text-align:left}.protocol-tabs button.active{border-color:var(--color-accent);background:var(--color-danger-surface)}.protocol-tabs strong{font-size:10px}.protocol-tabs small{color:var(--color-text-muted);font-size:8px}.connection-bar{display:flex;align-items:center;gap:9px;padding:10px 12px;border:1px solid var(--color-border);background:var(--color-bg-muted)}.connection-bar strong{min-width:48px;font-size:9px}.connection-bar input{min-width:0;height:34px;flex:1;padding:0 10px;font:10px "SFMono-Regular",Consolas,monospace}.connection-bar .event-input{max-width:180px}.connection-bar button{min-width:72px;min-height:34px}.connection-dot{width:8px;height:8px;border-radius:50%;background:var(--color-text-muted)}.connection-dot.active{background:var(--color-success);box-shadow:0 0 0 4px var(--color-success-surface)}.connection-dot.pending{background:var(--color-warning)}.realtime-layout{display:grid;grid-template-columns:340px minmax(0,1fr);gap:14px}.realtime-layout>.log-panel:only-child{grid-column:1/-1}.message-panel,.log-panel{min-width:0;overflow:hidden;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.panel-head{display:flex;min-height:58px;align-items:center;justify-content:space-between;gap:12px;padding:10px 14px;border-bottom:1px solid var(--color-border)}.panel-head p{margin:0 0 4px;color:var(--color-text-muted);font:8px "SFMono-Regular",Consolas,monospace;letter-spacing:.12em}.panel-head h2{margin:0;font-size:13px}.panel-head>div:last-child{display:flex;gap:6px}.panel-head button{min-height:28px;padding:0 9px;font-size:8px}.message-panel textarea{display:block;width:100%;min-height:300px;padding:13px 14px;resize:vertical;border:0;outline:0;background:#20231d;color:#e9ede5;font:10px/1.65 "SFMono-Regular",Consolas,monospace}.send-bar{display:flex;align-items:center;justify-content:space-between;padding:9px 12px;border-top:1px solid var(--color-border)}.send-bar span{color:var(--color-text-muted);font-size:8px}.send-bar button{min-width:72px}.log-empty{display:grid;min-height:358px;place-items:center;color:var(--color-text-muted);font-size:9px}.event-logs{min-height:358px;max-height:520px;overflow:auto;background:#20231d;color:#dfe5dc}.event-row{display:grid;grid-template-columns:70px 42px minmax(0,1fr);gap:10px;padding:10px 12px;border-bottom:1px solid #34382f;font-size:9px}.event-row time{color:#80877b}.event-row>span{color:#c4a276}.event-row.sent>span{color:#79a9d4}.event-row.received>span{color:#79b68a}.event-row.error>span{color:#e1816c}.event-row pre{margin:0;font:9px/1.55 "SFMono-Regular",Consolas,monospace;white-space:pre-wrap;word-break:break-word}.realtime-note{margin:0;color:var(--color-text-muted);font-size:8px;line-height:1.7}@media(max-width:1000px){.realtime-layout{grid-template-columns:1fr}.connection-bar .event-input{max-width:140px}}
</style>
