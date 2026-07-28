<script setup lang="ts">
import { ref, computed } from "vue";
import { buildConnection, type ServiceConnection } from "../api/connectionData";
import { testServiceConnection } from "../api/services";
import type { ServiceKind } from "../types";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";

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

async function exportEnv() {
  const conn = connection.value;
  const tpl = [
    `# ${conn.name} ${"\uD83C\uDF10"} ZhiYu local connection`,
    ...conn.envVars.map(v => `${v.key}=${v.value}`),
    `# exported: ${new Date().toISOString()}`,
  ].join("\n");

  const path = await save({
    defaultPath: `.env.${props.kind}`,
    filters: [{ name: "环境变量", extensions: ["env"] }, { name: "所有文件", extensions: ["*"] }],
  });
  if (path) {
    await writeTextFile(path, tpl + "\n");
  }
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
        <button
          class="metric-copy"
          :class="{ copied: copiedLabel === 'Host' }"
          @click="copyToClipboard(connection.host, 'Host')"
        >{{ copiedLabel === "Host" ? "\u2714 已复制" : "复制" }}</button>
      </article>
      <article class="connect-metric">
        <p>PORT</p>
        <strong>{{ connection.primaryPort }}</strong>
        <small>主要通信端口</small>
        <button
          class="metric-copy"
          :class="{ copied: copiedLabel === 'Port' }"
          @click="copyToClipboard(String(connection.primaryPort), 'Port')"
        >{{ copiedLabel === "Port" ? "\u2714 已复制" : "复制" }}</button>
      </article>
      <article class="connect-metric" v-if="connection.hasAuth || connection.username">
        <p>USERNAME</p>
        <strong>{{ connection.username || "\u2014" }}</strong>
        <small>本地开发账号</small>
        <button
          v-if="connection.username"
          class="metric-copy"
          :class="{ copied: copiedLabel === 'Username' }"
          @click="copyToClipboard(connection.username, 'Username')"
        >{{ copiedLabel === "Username" ? "\u2714 已复制" : "复制" }}</button>
      </article>
      <article class="connect-metric" v-if="connection.hasAuth">
        <p>PASSWORD</p>
        <strong class="password-val">
          <span v-if="showPassword">{{ connection.password }}</span>
          <span v-else>{{ "\u2022".repeat(Math.min(connection.password.length, 15)) }}</span>
        </strong>
        <small>本地开发密码</small>
        <div class="metric-btns">
          <button class="metric-copy" @click="copyToClipboard(connection.password, 'Password')" :class="{ copied: copiedLabel === 'Password' }">{{ copiedLabel === "Password" ? "\u2714 已复制" : "复制" }}</button>
          <button class="metric-copy" @click="togglePassword">{{ showPassword ? "隐藏" : "显示" }}</button>
        </div>
      </article>
    </div>

    <div class="connect-layout">
      <section class="connect-main">
        <div class="connect-section-head">
          <div>
            <p>CONNECTION STRINGS</p>
            <h2>连接字符串 &amp; 端点</h2>
          </div>
          <div class="connect-section-actions">
            <button
              type="button"
              :class="{ 'test-ok': testResult === 'ok', 'test-fail': testResult === 'fail' }"
              :disabled="testing"
              @click="testConnection"
            >
              {{ testing ? "测试中…" : testResult === "ok" ? "\u2714 连接成功" : testResult === "fail" ? "\u2718 连接失败" : "测试连接" }}
            </button>
            <button type="button" @click="exportEnv" v-if="connection.envVars.length">导出 .env</button>
          </div>
        </div>

        <div v-if="connection.uris.length" class="uri-block">
          <div class="uri-row" v-for="uri in connection.uris" :key="uri.label">
            <span class="uri-label">{{ uri.label }}</span>
            <div class="uri-field">
              <code>{{ uri.value }}</code>
              <button
                class="field-copy"
                :class="{ copied: copiedLabel === uri.label }"
                @click="copyToClipboard(uri.value, uri.label)"
              >{{ copiedLabel === uri.label ? "\u2714 已复制" : "复制" }}</button>
            </div>
          </div>
        </div>

        <div v-if="connection.extras.length" class="uri-block">
          <p class="extras-label">ADDITIONAL ENDPOINTS</p>
          <div class="uri-row" v-for="extra in connection.extras" :key="extra.label">
            <span class="uri-label">{{ extra.label }}</span>
            <div class="uri-field">
              <code>{{ extra.value }}</code>
              <button
                class="field-copy"
                :class="{ copied: copiedLabel === extra.label }"
                @click="copyToClipboard(extra.value, extra.label)"
              >{{ copiedLabel === extra.label ? "\u2714 已复制" : "复制" }}</button>
            </div>
          </div>
        </div>
      </section>
    </div>

    <section class="samples-section" v-if="connection.configSamples.length">
      <div class="connect-section-head">
        <div>
          <p>CLIENT EXAMPLES</p>
          <h2>客户端配置示例</h2>
        </div>
      </div>
      <div class="samples-body">
        <div class="sample-tabs">
          <button
            v-for="(s, i) in connection.configSamples"
            :key="i"
            :class="{ active: i === selectedSample }"
            @click="selectedSample = i"
          >{{ s.label }}</button>
        </div>
        <div class="sample-content">
          <div class="sample-caption">{{ connection.configSamples[selectedSample].caption }}</div>
          <pre><code>{{ connection.configSamples[selectedSample].code }}</code></pre>
          <button
            class="sample-copy-btn"
            :class="{ copied: copiedLabel === 'config' }"
            @click="copyToClipboard(connection.configSamples[selectedSample].code, 'config')"
          >{{ copiedLabel === "config" ? "\u2714 已复制" : "复制代码" }}</button>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.connect-page { padding: 26px 34px 34px; }

.connect-metrics {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  margin-bottom: 18px;
  border-top: 1px solid var(--color-border);
  border-left: 1px solid var(--color-border);
}

.connect-metric {
  position: relative;
  min-width: 0;
  padding: 18px 20px 42px;
  border-right: 1px solid var(--color-border);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

.connect-metric p {
  margin: 0 0 12px;
  color: var(--color-text-muted);
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
  color: var(--color-text-primary);
}

.connect-metric strong.password-val { font-size: 14px; letter-spacing: 0.14em; }

.connect-metric small {
  display: block;
  margin-top: 8px;
  color: var(--color-text-muted);
  font-size: 9px;
}

.metric-btns {
  position: absolute;
  right: 14px;
  bottom: 10px;
  display: flex;
  gap: 4px;
}

.metric-copy {
  min-width: 56px;
  padding: 3px 10px;
  border: 1px solid var(--color-border);
  background: var(--color-bg-panel);
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: 8px;
  white-space: nowrap;
  text-align: center;
}
.metric-copy:hover { border-color: var(--color-border-strong); color: var(--color-text-primary); }
.metric-copy.copied { background: var(--color-success-surface); border-color: #91b39a; color: var(--color-success-text); }

.connect-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 18px;
}

.connect-main {
  border: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

.connect-section-head {
  display: flex;
  min-height: 52px;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 13px 16px;
  border-bottom: 1px solid var(--color-border);
}

.connect-section-head p {
  margin: 0 0 3px;
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.12em;
}

.connect-section-head h2 { margin: 0; font-size: 14px; color: var(--color-text-primary); }

.connect-section-actions {
  display: flex;
  gap: 8px;
}

.connect-section-actions button {
  min-width: 64px;
  height: 30px;
  padding: 0 12px;
  border: 1px solid var(--color-border-strong);
  background: var(--color-bg-elevated);
  color: var(--color-text-secondary);
  font-size: 9px;
  cursor: pointer;
  white-space: nowrap;
}
.connect-section-actions button:hover:not(:disabled) { border-color: var(--color-border-strong); color: var(--color-text-primary); }
.connect-section-actions button:disabled { opacity: 0.5; cursor: default; }

.connect-section-actions button.test-ok {
  background: var(--color-success-surface);
  border-color: #639a6a;
  color: var(--color-success-text);
}

.connect-section-actions button.test-fail {
  background: var(--color-danger-surface);
  border-color: #d2a396;
  color: var(--color-danger-text);
}

.uri-block { padding: 4px 16px 16px; }

.uri-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 0;
  border-bottom: 1px solid var(--color-border);
}
.uri-row:last-child { border-bottom: 0; }

.uri-label {
  min-width: 80px;
  color: var(--color-text-secondary);
  font-size: 9px;
  font-family: "SFMono-Regular", Consolas, monospace;
  text-transform: uppercase;
}

.uri-field {
  display: flex;
  flex: 1;
  align-items: center;
  height: 30px;
  border: 1px solid var(--color-border-strong);
  background: var(--color-bg-elevated);
}

.uri-field code {
  flex: 1;
  padding: 0 10px;
  overflow: hidden;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
  color: var(--color-text-primary);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.field-copy {
  min-width: 64px;
  height: 100%;
  padding: 0 10px;
  border: none;
  border-left: 1px solid var(--color-border);
  background: var(--color-bg-content);
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: 9px;
  white-space: nowrap;
  text-align: center;
}
.field-copy:hover { background: var(--color-bg-muted); color: var(--color-text-primary); }
.field-copy.copied { background: var(--color-success-surface); color: var(--color-success-text); }

.extras-label {
  color: var(--color-text-muted);
  font-size: 9px;
  margin: 12px 0 6px;
}

.samples-section {
  margin-top: 18px;
  border: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

.samples-body {
  display: flex;
  flex-direction: column;
}

.sample-tabs {
  display: flex;
  gap: 0;
  padding: 10px 14px 0;
  border-bottom: 1px solid var(--color-border);
}

.sample-tabs button {
  padding: 5px 12px;
  border: 1px solid var(--color-border);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-muted);
  color: var(--color-text-muted);
  font-size: 8px;
  cursor: pointer;
  margin-right: -1px;
  margin-bottom: -1px;
}
.sample-tabs button.active {
  background: var(--color-bg-panel);
  color: var(--color-text-primary);
  border-bottom-color: var(--color-bg-panel);
}

.sample-content {
  padding: 14px;
  position: relative;
}

.sample-caption {
  color: var(--color-text-muted);
  font-size: 8px;
  margin-bottom: 8px;
}

.sample-content pre {
  margin: 0 0 36px;
  overflow-x: auto;
  font-size: 10px;
}

.sample-content code {
  font-family: "SFMono-Regular", Consolas, monospace;
  color: var(--color-text-primary);
  white-space: pre;
  line-height: 1.5;
}

.sample-copy-btn {
  min-width: 64px;
  position: absolute;
  right: 14px;
  bottom: 12px;
  padding: 4px 12px;
  border: 1px solid var(--color-border-strong);
  background: var(--color-bg-panel);
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: 8px;
  white-space: nowrap;
  text-align: center;
}
.sample-copy-btn:hover { border-color: var(--color-border-strong); color: var(--color-text-primary); }
.sample-copy-btn.copied { background: var(--color-success-surface); border-color: #91b39a; color: var(--color-success-text); }
</style>
