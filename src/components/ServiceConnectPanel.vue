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
  <div class="connect-panel">
    <!-- 连接信息卡片 -->
    <div class="card grid-card">
      <div class="card-header">
        <h3>连接参数</h3>
        <div class="header-actions">
          <button class="link-btn" @click="testConnection" :disabled="testing">
            {{ testing ? "测试中..." : testResult === "ok" ? "\u2714 连接成功" : testResult === "fail" ? "\u2718 连接失败" : "测试连接" }}
          </button>
          <button class="link-btn export-btn" @click="exportEnv" v-if="connection.envVars.length">
            导出 .env
          </button>
        </div>
      </div>
      <p class="card-error" v-if="testResult === 'fail'">{{ testError }}</p>

      <div class="props-grid">
        <div class="prop">
          <span class="prop-label">Host</span>
          <span class="prop-value">{{ connection.host }}</span>
          <button class="copy-btn" @click="copyToClipboard(connection.host, 'Host')" :title="copiedLabel === 'Host' ? '已复制' : '复制'">
            {{ copiedLabel === "Host" ? "\u2714" : "\u2398" }}
          </button>
        </div>
        <div class="prop">
          <span class="prop-label">Port</span>
          <span class="prop-value">{{ connection.primaryPort }}</span>
          <button class="copy-btn" @click="copyToClipboard(String(connection.primaryPort), 'Port')" :title="copiedLabel === 'Port' ? '已复制' : '复制'">
            {{ copiedLabel === "Port" ? "\u2714" : "\u2398" }}
          </button>
        </div>
        <div class="prop" v-if="connection.hasAuth || connection.username">
          <span class="prop-label">Username</span>
          <span class="prop-value">{{ connection.username || "\u2014" }}</span>
          <button class="copy-btn" @click="copyToClipboard(connection.username, 'Username')" :title="copiedLabel === 'Username' ? '已复制' : '复制'" v-if="connection.username">
            {{ copiedLabel === "Username" ? "\u2714" : "\u2398" }}
          </button>
        </div>
        <div class="prop" v-if="connection.hasAuth">
          <span class="prop-label">Password</span>
          <span class="prop-value">
            <span v-if="showPassword">{{ connection.password }}</span>
            <span v-else>{{ "\u2022".repeat(Math.min(connection.password.length, 16)) }}</span>
          </span>
          <button class="copy-btn" @click="copyToClipboard(connection.password, 'Password')" :title="copiedLabel === 'Password' ? '已复制' : '复制'">
            {{ copiedLabel === "Password" ? "\u2714" : "\u2398" }}
          </button>
          <button class="copy-btn" @click="togglePassword" :title="showPassword ? '隐藏' : '显示'">
            {{ showPassword ? "\u25C9" : "\u25CE" }}
          </button>
        </div>
      </div>
    </div>

    <!-- 连接字符串 -->
    <div class="card" v-if="connection.uris.length">
      <h4>连接字符串</h4>
      <div class="uri-list">
        <div class="uri-row" v-for="uri in connection.uris" :key="uri.label">
          <span class="uri-label">{{ uri.label }}</span>
          <code class="uri-value">{{ uri.value }}</code>
          <button class="copy-btn uri-copy" @click="copyToClipboard(uri.value, uri.label)" :title="copiedLabel === uri.label ? '已复制' : '复制'">
            {{ copiedLabel === uri.label ? "\u2714" : "\u2398" }}
          </button>
        </div>
      </div>
    </div>

    <!-- 额外端点 -->
    <div class="card" v-if="connection.extras.length">
      <h4>其他端点</h4>
      <div class="uri-list">
        <div class="uri-row" v-for="extra in connection.extras" :key="extra.label">
          <span class="uri-label">{{ extra.label }}</span>
          <code class="uri-value">{{ extra.value }}</code>
          <button class="copy-btn uri-copy" @click="copyToClipboard(extra.value, extra.label)" :title="copiedLabel === extra.label ? '已复制' : '复制'">
            {{ copiedLabel === extra.label ? "\u2714" : "\u2398" }}
          </button>
        </div>
      </div>
    </div>

    <!-- 客户端配置示例 -->
    <div class="card" v-if="connection.configSamples.length">
      <h4>客户端配置示例</h4>
      <div class="sample-tabs">
        <button
          v-for="(sample, idx) in connection.configSamples"
          :key="idx"
          :class="['sample-tab', { active: idx === selectedSample }]"
          @click="selectedSample = idx"
        >{{ sample.label }}</button>
      </div>
      <div class="sample-body">
        <div class="sample-caption">{{ connection.configSamples[selectedSample].caption }}</div>
        <pre><code>{{ connection.configSamples[selectedSample].code }}</code></pre>
        <button class="copy-btn sample-copy" @click="copyToClipboard(connection.configSamples[selectedSample].code, 'config')">
          {{ copiedLabel === "config" ? "已复制" : "复制代码" }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.connect-panel {
  max-width: 780px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.card {
  background: var(--card-bg, #1a1e26);
  border: 1px solid var(--border, #2d3240);
  border-radius: 10px;
  padding: 20px;
}

.card h3, .card h4 {
  margin: 0 0 12px;
  font-size: 15px;
  font-weight: 600;
  color: var(--text-primary, #e1e4eb);
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.card-header h3 { margin: 0; }

.header-actions {
  display: flex;
  gap: 8px;
}

.card-error {
  margin: 0 0 12px;
  padding: 8px 12px;
  background: #3d1a1a;
  border: 1px solid #7a2a2a;
  border-radius: 6px;
  font-size: 13px;
  color: #f5a6a6;
  word-break: break-all;
}

.link-btn {
  background: var(--accent-bg, #1f2b4d);
  color: var(--accent, #5b8dee);
  border: 1px solid var(--accent-border, #2e4275);
  border-radius: 6px;
  padding: 6px 14px;
  font-size: 13px;
  cursor: pointer;
  transition: all .15s;
}
.link-btn:hover:not(:disabled) { background: #25345c; }
.link-btn:disabled { opacity: .5; cursor: default; }
.export-btn { background: #1a2e1a; color: #6abf6a; border-color: #2a4a2a; }

.props-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
}

.prop {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--input-bg, #10141b);
  border-radius: 6px;
}

.prop-label {
  font-size: 12px;
  color: var(--text-secondary, #787f8e);
  min-width: 60px;
  text-transform: uppercase;
  letter-spacing: .5px;
}

.prop-value {
  font-size: 14px;
  font-family: "SF Mono", "Fira Code", monospace;
  color: var(--text-primary, #e1e4eb);
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.copy-btn {
  background: none;
  border: none;
  color: var(--text-secondary, #787f8e);
  cursor: pointer;
  padding: 2px 4px;
  border-radius: 4px;
  font-size: 13px;
  line-height: 1;
  transition: color .15s, background .15s;
  flex-shrink: 0;
}
.copy-btn:hover { color: var(--accent, #5b8dee); background: #252e3d; }

.uri-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.uri-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.uri-label {
  font-size: 12px;
  color: var(--text-secondary, #787f8e);
  min-width: 100px;
  text-transform: uppercase;
  letter-spacing: .5px;
}

.uri-value {
  font-size: 13px;
  font-family: "SF Mono", "Fira Code", monospace;
  color: var(--accent, #8badf5);
  background: var(--input-bg, #10141b);
  padding: 4px 10px;
  border-radius: 4px;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.uri-copy { margin-left: 4px; }

.sample-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 12px;
}

.sample-tab {
  background: var(--input-bg, #10141b);
  color: var(--text-secondary, #787f8e);
  border: 1px solid var(--border, #2d3240);
  border-radius: 6px 6px 0 0;
  padding: 6px 14px;
  font-size: 13px;
  cursor: pointer;
  transition: all .15s;
}
.sample-tab.active {
  background: var(--accent-bg, #1f2b4d);
  color: var(--accent, #5b8dee);
  border-color: var(--accent-border, #2e4275);
}
.sample-tab:hover:not(.active) { color: var(--text-primary, #e1e4eb); }

.sample-body {
  background: var(--input-bg, #10141b);
  border: 1px solid var(--border, #2d3240);
  border-radius: 0 6px 6px 6px;
  padding: 14px;
  position: relative;
}

.sample-caption {
  font-size: 12px;
  color: var(--text-secondary, #787f8e);
  margin-bottom: 8px;
}

.sample-body pre {
  margin: 0;
  overflow-x: auto;
  font-size: 13px;
}

.sample-body code {
  font-family: "SF Mono", "Fira Code", monospace;
  color: var(--text-primary, #e1e4eb);
  white-space: pre;
}

.sample-copy {
  position: absolute;
  top: 10px;
  right: 10px;
  padding: 4px 10px;
  font-size: 12px;
  border: 1px solid var(--border, #2d3240);
  border-radius: 4px;
  background: var(--card-bg, #1a1e26);
}
</style>
