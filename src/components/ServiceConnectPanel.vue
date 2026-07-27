<script setup lang="ts">
import { ref, computed } from "vue";
import { buildConnection, type ServiceConnection } from "../api/connectionData";
import { testServiceConnection } from "../api/services";
import type { ServiceKind } from "../types";

const props = defineProps<{ kind: ServiceKind }>();

const connection = computed<ServiceConnection>(() => buildConnection(props.kind));
const showPassword = ref(false);
const testing = ref(false);
const testResult = ref<"idle" | "ok" | "fail">("idle");
const testError = ref("");
const copiedLabel = ref("");

const togglePassword = () => { showPassword.value = !showPassword.value; };

async function copyToClipboard(text: string, label: string) {
  await navigator.clipboard.writeText(text);
  copiedLabel.value = label;
  setTimeout(() => { copiedLabel.value = ""; }, 1500);
}

async function testConnection() {
  testing.value = true;
  testResult.value = "idle";
  testError.value = "";
  try {
    await testServiceConnection(props.kind);
    testResult.value = "ok";
  } catch (e: any) {
    testResult.value = "fail";
    testError.value = String(e);
  } finally {
    testing.value = false;
  }
}

function exportEnv() {
  const conn = connection.value;
  const tpl = [
    `# ${conn.name} 智屿本地连接环境变量`,
    ...conn.envVars.map(v => `${v.key}=${v.value}`),
    `# 导出时间: ${new Date().toISOString()}`,
  ].join("\n");
  const blob = new Blob([tpl + "\n"], { type: "text/plain" });
  const a = document.createElement("a");
  a.href = URL.createObjectURL(blob);
  a.download = `.env.${props.kind}`;
  a.click();
  URL.revokeObjectURL(a.href);
}

const selectedSample = ref(0);
</script>

<template>
  <div class="connect-page">
    <div v-if="testResult === 'fail'" class="notice danger">
      <span>{{ testError }}</span>
      <button type="button" @click="testResult = 'idle'">&times;</button>
    </div>

    <div class="connect-metrics">
      <article class="connect-metric">
        <p>HOST</p>
        <strong>{{ connection.host }}</strong>
        <small>本地绑定地址</small>
        <button class="metric-copy" @click="copyToClipboard(connection.host, 'Host')">{{ copiedLabel === "Host" ? "已复制" : "复制" }}</button>
      </article>
      <article class="connect-metric">
        <p>PORT</p>
        <strong>{{ connection.primaryPort }}</strong>
        <small>主要通信端口</small>
        <button class="metric-copy" @click="copyToClipboard(String(connection.primaryPort), 'Port')">{{ copiedLabel === "Port" ? "已复制" : "复制" }}</button>
      </article>
      <article class="connect-metric" v-if="connection.hasAuth || connection.username">
        <p>USERNAME</p>
        <strong>{{ connection.username || "\u2014" }}</strong>
        <small>本地开发账号</small>
        <button v-if="connection.username" class="metric-copy" @click="copyToClipboard(connection.username, 'Username')">{{ copiedLabel === "Username" ? "已复制" : "复制" }}</button>
      </article>
      <article class="connect-metric" v-if="connection.hasAuth">
        <p>PASSWORD</p>
        <strong class="password-val">
          <span v-if="showPassword">{{ connection.password }}</span>
          <span v-else>{{ "\u2022".repeat(Math.min(connection.password.length, 15)) }}</span>
        </strong>
        <small>本地开发密码</small>
        <button class="metric-copy" @click="copyToClipboard(connection.password, 'Password')">{{ copiedLabel === "Password" ? "已复制" : "复制" }}</button>
        <button class="metric-copy" style="margin-left:4px" @click="togglePassword">{{ showPassword ? "隐藏" : "显示" }}</button>
      </article>
    </div>

    <div class="connect-layout">
      <section class="connect-main">
        <div class="connect-section-head">
          <p>CONNECTION STRINGS</p>
          <h2>连接字符串</h2>
          <div class="connect-section-actions">
            <button class="primary" type="button" :disabled="testing" @click="testConnection">
              {{ testing ? "测试中…" : testResult === "ok" ? "连接成功" : "测试连接" }}
            </button>
            <button type="button" @click="exportEnv" v-if="connection.envVars.length">导出 .env</button>
          </div>
        </div>

        <div v-if="connection.uris.length" class="uri-block">
          <div class="uri-row" v-for="uri in connection.uris" :key="uri.label">
            <span class="uri-label">{{ uri.label }}</span>
            <div class="uri-field">
              <code>{{ uri.value }}</code>
              <button class="field-copy" @click="copyToClipboard(uri.value, uri.label)">{{ copiedLabel === uri.label ? "已复制" : "复制" }}</button>
            </div>
          </div>
        </div>

        <div v-if="connection.extras.length" class="uri-block">
          <p style="color:#989a93;font-size:9px;margin:12px 0 6px;">ADDITIONAL ENDPOINTS</p>
          <div class="uri-row" v-for="extra in connection.extras" :key="extra.label">
            <span class="uri-label">{{ extra.label }}</span>
            <div class="uri-field">
              <code>{{ extra.value }}</code>
              <button class="field-copy" @click="copyToClipboard(extra.value, extra.label)">{{ copiedLabel === extra.label ? "已复制" : "复制" }}</button>
            </div>
          </div>
        </div>
      </section>

      <aside class="connect-samples" v-if="connection.configSamples.length">
        <div class="connect-section-head">
          <p>CLIENT EXAMPLES</p>
          <h2>客户端配置</h2>
        </div>
        <div class="sample-tabs">
          <button v-for="(s, i) in connection.configSamples" :key="i" :class="{ active: i === selectedSample }" @click="selectedSample = i">{{ s.label }}</button>
        </div>
        <div class="sample-body">
          <div class="sample-caption">{{ connection.configSamples[selectedSample].caption }}</div>
          <pre><code>{{ connection.configSamples[selectedSample].code }}</code></pre>
          <button class="sample-copy-btn" @click="copyToClipboard(connection.configSamples[selectedSample].code, 'config')">{{ copiedLabel === "config" ? "已复制" : "复制" }}</button>
        </div>
      </aside>
    </div>
  </div>
</template>

<style scoped>
.connect-page { padding: 26px 34px 34px; }

.connect-metrics {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  margin-bottom: 18px;
  border-top: 1px solid #d2d1c9;
  border-left: 1px solid #d2d1c9;
}

.connect-metric {
  position: relative;
  min-width: 0;
  padding: 18px 20px 36px;
  border-right: 1px solid #d2d1c9;
  border-bottom: 1px solid #d2d1c9;
  background: rgba(250, 249, 245, 0.84);
}

.connect-metric p {
  margin: 0 0 12px;
  color: #989a93;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.12em;
}

.connect-metric strong {
  display: block;
  overflow: hidden;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 18px;
  font-weight: 500;
  letter-spacing: -0.05em;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #353830;
}

.connect-metric strong.password-val { font-size: 14px; letter-spacing: 0.14em; }

.connect-metric small {
  display: block;
  margin-top: 8px;
  color: #989a93;
  font-size: 9px;
}

.metric-copy {
  position: absolute;
  right: 14px;
  bottom: 10px;
  padding: 3px 10px;
  border: 1px solid #cfcec6;
  background: #faf9f5;
  color: #6f7269;
  cursor: pointer;
  font-size: 8px;
}
.metric-copy:hover { border-color: #898b83; color: #252920; }

.connect-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 18px;
  align-items: start;
}

.connect-main, .connect-samples {
  border: 1px solid #d2d1c9;
  background: rgba(250, 249, 245, 0.9);
}

.connect-section-head {
  display: flex;
  min-height: 52px;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 13px 16px;
  border-bottom: 1px solid #d2d1c9;
}

.connect-section-head p {
  margin: 0 0 3px;
  color: #989a93;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.12em;
}

.connect-section-head h2 { margin: 0; font-size: 14px; color: #252920; }

.connect-section-actions {
  display: flex;
  gap: 8px;
}

.connect-section-actions button {
  height: 30px;
  padding: 0 12px;
  border: 1px solid #c8c7bf;
  background: #fffefa;
  color: #6f7269;
  font-size: 9px;
  cursor: pointer;
}
.connect-section-actions button:hover { border-color: #898b83; color: #252920; }
.connect-section-actions button.primary { background: #252920; color: white; border-color: #252920; }
.connect-section-actions button.primary:hover { background: #393d33; }
.connect-section-actions button:disabled { opacity: 0.5; cursor: default; }

.uri-block { padding: 4px 16px 16px; }

.uri-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 0;
  border-bottom: 1px solid #e8e6df;
}
.uri-row:last-child { border-bottom: 0; }

.uri-label {
  min-width: 80px;
  color: #73766d;
  font-size: 9px;
  font-family: "SFMono-Regular", Consolas, monospace;
  text-transform: uppercase;
}

.uri-field {
  display: flex;
  flex: 1;
  align-items: center;
  height: 30px;
  border: 1px solid #c8c7bf;
  background: #fffefa;
}

.uri-field code {
  flex: 1;
  padding: 0 10px;
  overflow: hidden;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
  color: #353830;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.field-copy {
  height: 100%;
  padding: 0 10px;
  border: none;
  border-left: 1px solid #e8e6df;
  background: #f5f3ee;
  color: #6f7269;
  cursor: pointer;
  font-size: 9px;
  white-space: nowrap;
}
.field-copy:hover { background: #edeae2; color: #252920; }

.connect-samples {
  width: 320px;
  overflow: hidden;
}

.sample-tabs {
  display: flex;
  gap: 0;
  padding: 10px 14px 0;
}

.sample-tabs button {
  padding: 5px 10px;
  border: 1px solid #d2d1c9;
  border-bottom: 0;
  background: #f0ede5;
  color: #8d8f87;
  font-size: 8px;
  cursor: pointer;
  margin-right: -1px;
}
.sample-tabs button.active {
  background: #faf9f5;
  color: #252920;
}

.sample-body {
  border-top: 1px solid #d2d1c9;
  padding: 14px;
  position: relative;
}

.sample-caption {
  color: #989a93;
  font-size: 8px;
  margin-bottom: 8px;
}

.sample-body pre {
  margin: 0;
  overflow-x: auto;
  font-size: 10px;
}

.sample-body code {
  font-family: "SFMono-Regular", Consolas, monospace;
  color: #353830;
  white-space: pre;
  line-height: 1.5;
}

.sample-copy-btn {
  margin-top: 10px;
  padding: 4px 12px;
  border: 1px solid #c8c7bf;
  background: #faf9f5;
  color: #6f7269;
  cursor: pointer;
  font-size: 8px;
}
.sample-copy-btn:hover { border-color: #898b83; color: #252920; }
</style>
