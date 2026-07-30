<script setup lang="ts">
import { ref, computed } from "vue";
import { useI18n } from "vue-i18n";
import { buildConnection, type ServiceConnection } from "../api/connectionData";
import { testServiceConnection } from "../api/services";
import type { ServiceKind } from "../types";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";

const props = defineProps<{ kind: ServiceKind }>();
const { t, locale } = useI18n();

const connection = computed<ServiceConnection>(() => buildConnection(props.kind));
const showPassword = ref(false);
const testing = ref(false);
const testResult = ref<"idle" | "ok" | "fail">("idle");
const testError = ref("");
const copiedLabel = ref("");

function localizedLabel(label: string) {
  const labels: Record<string, string> = {
    连接串: "connection.connectionString",
    运行时: "connection.runtime",
    持久化: "connection.persistence",
  };
  return labels[label] ? t(labels[label]) : label;
}

function localizedValue(value: string) {
  return locale.value === "en-US" && value === "Tansu（无需 JVM / ZooKeeper）"
    ? t("connection.tansuRuntime")
    : value;
}

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
    filters: [
      { name: t("connection.envFilter"), extensions: ["env"] },
      { name: t("connection.allFilesFilter"), extensions: ["*"] },
    ],
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

    <section class="connect-summary">
      <div class="connect-summary-head">
        <div>
          <p>{{ t("connection.summaryEyebrow") }}</p>
          <h2>{{ t("connection.summaryTitle", { service: connection.name }) }}</h2>
          <span>{{ t("connection.summaryHint") }}</span>
        </div>
        <div class="connect-section-actions">
          <button
            type="button"
            :class="{ 'test-ok': testResult === 'ok', 'test-fail': testResult === 'fail' }"
            :disabled="testing"
            @click="testConnection"
          >
            {{ testing ? t("connection.testing") : testResult === "ok" ? t("connection.success") : testResult === "fail" ? t("connection.failed") : t("connection.test") }}
          </button>
          <button type="button" @click="exportEnv" v-if="connection.envVars.length">{{ t("connection.exportEnv") }}</button>
        </div>
      </div>

      <div class="connect-metrics">
        <article class="connect-metric host">
          <p>HOST</p>
          <strong>{{ connection.host }}</strong>
          <small>{{ t("connection.hostHint") }}</small>
          <button
            type="button"
            class="metric-copy"
            :class="{ copied: copiedLabel === 'Host' }"
            @click="copyToClipboard(connection.host, 'Host')"
          >{{ copiedLabel === "Host" ? t("connection.copied") : t("connection.copy") }}</button>
        </article>
        <article class="connect-metric port">
          <p>PORT</p>
          <strong>{{ connection.primaryPort }}</strong>
          <small>{{ t("connection.portHint") }}</small>
          <button
            type="button"
            class="metric-copy"
            :class="{ copied: copiedLabel === 'Port' }"
            @click="copyToClipboard(String(connection.primaryPort), 'Port')"
          >{{ copiedLabel === "Port" ? t("connection.copied") : t("connection.copy") }}</button>
        </article>
        <article class="connect-metric username" v-if="connection.hasAuth || connection.username">
          <p>USERNAME</p>
          <strong>{{ connection.username || "\u2014" }}</strong>
          <small>{{ t("connection.usernameHint") }}</small>
          <button
            v-if="connection.username"
            type="button"
            class="metric-copy"
            :class="{ copied: copiedLabel === 'Username' }"
            @click="copyToClipboard(connection.username, 'Username')"
          >{{ copiedLabel === "Username" ? t("connection.copied") : t("connection.copy") }}</button>
        </article>
        <article class="connect-metric password" v-if="connection.hasAuth">
          <p>PASSWORD</p>
          <strong class="password-val">
            <span v-if="showPassword">{{ connection.password }}</span>
            <span v-else>{{ "\u2022".repeat(Math.min(connection.password.length, 15)) }}</span>
          </strong>
          <small>{{ t("connection.passwordHint") }}</small>
          <div class="metric-btns">
            <button type="button" class="metric-copy" @click="copyToClipboard(connection.password, 'Password')" :class="{ copied: copiedLabel === 'Password' }">{{ copiedLabel === "Password" ? t("connection.copied") : t("connection.copy") }}</button>
            <button type="button" class="metric-copy" @click="togglePassword">{{ showPassword ? t("connection.hide") : t("connection.show") }}</button>
          </div>
        </article>
      </div>
    </section>

    <div class="connect-layout">
      <section class="connect-main">
        <div class="connect-section-head">
          <div>
            <p>CONNECTION STRINGS</p>
            <h2>{{ t("connection.stringsTitle") }}</h2>
          </div>
        </div>

        <div v-if="connection.uris.length" class="uri-block">
          <div class="uri-row" v-for="uri in connection.uris" :key="uri.label">
            <span class="uri-label">{{ localizedLabel(uri.label) }}</span>
            <div class="uri-field">
              <code>{{ uri.value }}</code>
              <button
                class="field-copy"
                :class="{ copied: copiedLabel === uri.label }"
                @click="copyToClipboard(uri.value, uri.label)"
              >{{ copiedLabel === uri.label ? t("connection.copied") : t("connection.copy") }}</button>
            </div>
          </div>
        </div>

        <div v-if="connection.extras.length" class="uri-block">
          <p class="extras-label">ADDITIONAL ENDPOINTS</p>
          <div class="uri-row" v-for="extra in connection.extras" :key="extra.label">
            <span class="uri-label">{{ localizedLabel(extra.label) }}</span>
            <div class="uri-field">
              <code>{{ localizedValue(extra.value) }}</code>
              <button
                class="field-copy"
                :class="{ copied: copiedLabel === extra.label }"
                @click="copyToClipboard(extra.value, extra.label)"
              >{{ copiedLabel === extra.label ? t("connection.copied") : t("connection.copy") }}</button>
            </div>
          </div>
        </div>
      </section>
    </div>

    <section class="samples-section" v-if="connection.configSamples.length">
      <div class="connect-section-head">
        <div>
          <p>CLIENT EXAMPLES</p>
          <h2>{{ t("connection.examplesTitle") }}</h2>
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
          >{{ copiedLabel === "config" ? t("connection.copied") : t("connection.copyCode") }}</button>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.connect-page {
  display: grid;
  gap: 12px;
  padding: 22px 28px 32px;
}

.connect-summary,
.connect-main,
.samples-section {
  border: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

.connect-summary {
  overflow: hidden;
  box-shadow: inset 3px 0 0 var(--color-accent);
}

.connect-summary-head {
  display: flex;
  min-height: 48px;
  align-items: center;
  justify-content: space-between;
  gap: 18px;
  padding: 11px 14px 10px 17px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-muted);
}

.connect-summary-head > div:first-child {
  display: grid;
  min-width: 0;
  grid-template-columns: auto auto minmax(0, 1fr);
  align-items: baseline;
  gap: 6px 10px;
}

.connect-summary-head p {
  grid-column: 1 / -1;
  margin: 0;
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 7px;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.connect-summary-head h2 {
  margin: 0;
  color: var(--color-text-primary);
  font-size: 13px;
  white-space: nowrap;
}

.connect-summary-head span {
  overflow: hidden;
  color: var(--color-text-muted);
  font-size: 8px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.connect-metrics {
  display: flex;
  flex-wrap: wrap;
  align-items: stretch;
  gap: 8px;
  padding: 10px 14px 12px 17px;
}

.connect-metric {
  position: relative;
  box-sizing: border-box;
  flex: 0 1 210px;
  min-width: 0;
  min-height: 66px;
  padding: 9px 60px 8px 12px;
  border: 1px solid var(--color-border);
  background: var(--color-bg-panel);
}

.connect-metric.host { flex-basis: 210px; }
.connect-metric.port { flex-basis: 128px; }
.connect-metric.username { flex-basis: 180px; }
.connect-metric.password {
  flex-basis: 250px;
  padding-right: 112px;
}

.connect-metric p {
  margin: 0 0 5px;
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 7px;
  letter-spacing: 0.12em;
}

.connect-metric strong {
  display: block;
  overflow: hidden;
  color: var(--color-text-primary);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 13px;
  font-weight: 600;
  letter-spacing: -0.03em;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.connect-metric strong.password-val {
  font-size: 11px;
  letter-spacing: 0.08em;
}

.connect-metric small {
  display: block;
  overflow: hidden;
  margin-top: 3px;
  color: var(--color-text-muted);
  font-size: 7px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.metric-btns {
  position: absolute;
  top: 8px;
  right: 7px;
  display: flex;
  gap: 4px;
}

.metric-copy {
  position: absolute;
  top: 8px;
  right: 7px;
  min-width: 34px;
  height: 22px;
  padding: 0 7px;
  border: 0;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  font-size: 7px;
  white-space: nowrap;
  text-align: center;
}

.metric-btns .metric-copy { position: static; }
.metric-copy:hover {
  background: var(--color-bg-muted);
  color: var(--color-text-primary);
}
.metric-copy.copied {
  background: var(--color-success-surface);
  color: var(--color-success-text);
}

.connect-layout {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 12px;
}

.connect-main {
  overflow: hidden;
}

.connect-section-head {
  display: flex;
  min-height: 46px;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 10px 14px;
  border-bottom: 1px solid var(--color-border);
  background: var(--color-bg-muted);
}

.connect-section-head p {
  margin: 0 0 3px;
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.12em;
}

.connect-section-head h2 { margin: 0; font-size: 13px; color: var(--color-text-primary); }

.connect-section-actions {
  display: flex;
  gap: 8px;
}

.connect-section-actions button {
  min-width: 64px;
  height: 28px;
  padding: 0 10px;
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

.uri-block { padding: 4px 14px 11px; }

.uri-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 7px 0;
  border-bottom: 1px solid var(--color-border);
}
.uri-row:last-child { border-bottom: 0; }

.uri-label {
  width: 108px;
  min-width: 108px;
  color: var(--color-text-secondary);
  font-size: 9px;
  font-family: "SFMono-Regular", Consolas, monospace;
  text-transform: uppercase;
}

.uri-field {
  display: flex;
  flex: 1;
  align-items: center;
  height: 32px;
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
  margin-top: 0;
  overflow: hidden;
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

@media (max-width: 820px) {
  .connect-page { padding: 18px 20px 28px; }

  .connect-summary-head {
    align-items: flex-start;
    flex-direction: column;
    gap: 10px;
  }

  .connect-summary-head > div:first-child { width: 100%; }
  .connect-section-actions { align-self: flex-end; }

  .connect-metrics {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .connect-metric,
  .connect-metric.host,
  .connect-metric.port,
  .connect-metric.username,
  .connect-metric.password {
    width: auto;
  }
}

@media (max-width: 560px) {
  .connect-page { padding: 14px; }
  .connect-summary-head > div:first-child { grid-template-columns: 1fr; }
  .connect-summary-head span { display: none; }
  .connect-section-actions { width: 100%; }
  .connect-section-actions button { flex: 1; }
  .connect-metrics { grid-template-columns: 1fr; }

  .uri-row {
    display: grid;
    gap: 5px;
  }

  .uri-label {
    width: auto;
    min-width: 0;
  }

  .sample-tabs { overflow-x: auto; }
}
</style>
