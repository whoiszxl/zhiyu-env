<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { showToast } from "../toast";
import type {
  InfluxdbDatabase,
  InfluxdbOverview,
  InfluxdbQueryResult,
} from "../types";

const props = defineProps<{ running: boolean }>();
const { locale } = useI18n();
const en = computed(() => locale.value === "en-US");
const overview = ref<InfluxdbOverview | null>(null);
const databases = ref<InfluxdbDatabase[]>([]);
const database = ref("");
const databaseName = ref("metrics");
const retention = ref("");
const query = ref('SELECT * FROM "cpu" ORDER BY time DESC LIMIT 100');
const lineProtocol = ref("cpu,host=local usage=12.5");
const precision = ref("auto");
const result = ref<InfluxdbQueryResult | null>(null);
const loading = ref(false);

async function load() {
  if (!props.running || loading.value) return;
  loading.value = true;
  try {
    const [nextOverview, nextDatabases] = await Promise.all([
      invoke<InfluxdbOverview>("influxdb_overview"),
      invoke<InfluxdbDatabase[]>("influxdb_databases"),
    ]);
    overview.value = nextOverview;
    databases.value = nextDatabases;
    if (!databases.value.some((item) => item.name === database.value)) {
      database.value = databases.value[0]?.name ?? "";
    }
  } catch (error) {
    showToast({ intent: "error", title: String(error) });
  } finally {
    loading.value = false;
  }
}

async function createDatabase() {
  if (!databaseName.value.trim()) return;
  try {
    databases.value = await invoke<InfluxdbDatabase[]>(
      "influxdb_database_create",
      {
        name: databaseName.value.trim(),
        retentionPeriod: retention.value.trim() || null,
      },
    );
    database.value = databaseName.value.trim();
    showToast({
      intent: "success",
      title: en.value ? "Database created" : "数据库已创建",
    });
  } catch (error) {
    showToast({ intent: "error", title: String(error) });
  }
}

async function deleteDatabase() {
  if (!database.value) return;
  const label = database.value;
  if (!window.confirm(en.value ? `Delete ${label}?` : `确认删除 ${label}？此操作不可恢复。`)) return;
  try {
    databases.value = await invoke<InfluxdbDatabase[]>(
      "influxdb_database_delete",
      { name: label },
    );
    database.value = databases.value[0]?.name ?? "";
    result.value = null;
  } catch (error) {
    showToast({ intent: "error", title: String(error) });
  }
}

async function runQuery() {
  if (!database.value || !query.value.trim() || loading.value) return;
  loading.value = true;
  try {
    result.value = await invoke<InfluxdbQueryResult>("influxdb_query", {
      database: database.value,
      query: query.value,
    });
  } catch (error) {
    showToast({ intent: "error", title: String(error) });
  } finally {
    loading.value = false;
  }
}

async function writeData() {
  if (!database.value || !lineProtocol.value.trim() || loading.value) return;
  loading.value = true;
  try {
    await invoke("influxdb_write", {
      database: database.value,
      lineProtocol: lineProtocol.value,
      precision: precision.value,
    });
    showToast({
      intent: "success",
      title: en.value ? "Data written" : "时序数据写入成功",
    });
  } catch (error) {
    showToast({ intent: "error", title: String(error) });
  } finally {
    loading.value = false;
  }
}

function display(value: unknown) {
  if (value === null || value === undefined) return "NULL";
  return typeof value === "object" ? JSON.stringify(value) : String(value);
}

onMounted(load);
</script>

<template>
  <section class="influx-panel">
    <div v-if="!running" class="empty">
      <strong>{{ en ? "InfluxDB is stopped" : "InfluxDB 尚未启动" }}</strong>
      <span>{{ en ? "Start the service to manage databases and time-series data." : "启动服务后即可管理数据库、执行 SQL 和写入时序数据。" }}</span>
    </div>
    <template v-else>
      <div class="summary">
        <div>
          <span>ENDPOINT</span>
          <strong>{{ overview?.endpoint ?? "http://127.0.0.1:8181" }}</strong>
        </div>
        <div>
          <span>{{ en ? "DATABASES" : "数据库" }}</span>
          <strong>{{ overview?.databaseCount ?? databases.length }}</strong>
        </div>
        <button type="button" :disabled="loading" @click="load">
          {{ loading ? (en ? "Loading…" : "读取中…") : (en ? "Refresh" : "刷新") }}
        </button>
      </div>

      <div class="database-bar">
        <label>
          <span>{{ en ? "Active database" : "当前数据库" }}</span>
          <select v-model="database">
            <option value="" disabled>{{ en ? "Create a database first" : "请先创建数据库" }}</option>
            <option v-for="item in databases" :key="item.name" :value="item.name">
              {{ item.name }}
            </option>
          </select>
        </label>
        <label>
          <span>{{ en ? "New database" : "新建数据库" }}</span>
          <input v-model="databaseName" maxlength="64" placeholder="metrics" />
        </label>
        <label class="retention">
          <span>{{ en ? "Retention (optional)" : "保留周期（可选）" }}</span>
          <input v-model="retention" placeholder="30d" />
        </label>
        <button class="primary" type="button" @click="createDatabase">
          {{ en ? "Create" : "创建" }}
        </button>
        <button class="danger" type="button" :disabled="!database" @click="deleteDatabase">
          {{ en ? "Delete" : "删除" }}
        </button>
      </div>

      <div class="workspace">
        <article class="card">
          <header>
            <div>
              <span>SQL</span>
              <h2>{{ en ? "Query console" : "查询控制台" }}</h2>
            </div>
            <button class="primary" type="button" :disabled="!database || loading" @click="runQuery">
              {{ en ? "Run query" : "执行查询" }}
            </button>
          </header>
          <textarea v-model="query" spellcheck="false"></textarea>
          <div v-if="result" class="result">
            <div class="result-meta">
              {{ result.rowCount }} {{ en ? "rows" : "行" }}
              <span v-if="result.truncated">· {{ en ? "first 500 shown" : "仅展示前 500 行" }}</span>
            </div>
            <div class="table-scroll">
              <table>
                <thead><tr><th v-for="column in result.columns" :key="column">{{ column }}</th></tr></thead>
                <tbody>
                  <tr v-for="(row, index) in result.rows" :key="index">
                    <td v-for="(value, cell) in row" :key="cell">{{ display(value) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
          <div v-else class="result-empty">
            {{ en ? "Query results appear here." : "查询结果会显示在这里。" }}
          </div>
        </article>

        <article class="card write-card">
          <header>
            <div>
              <span>LINE PROTOCOL</span>
              <h2>{{ en ? "Write data" : "写入数据" }}</h2>
            </div>
          </header>
          <textarea v-model="lineProtocol" spellcheck="false"></textarea>
          <label>
            <span>{{ en ? "Timestamp precision" : "时间精度" }}</span>
            <select v-model="precision">
              <option value="auto">Auto</option>
              <option value="second">Second</option>
              <option value="millisecond">Millisecond</option>
              <option value="microsecond">Microsecond</option>
              <option value="nanosecond">Nanosecond</option>
            </select>
          </label>
          <button class="primary" type="button" :disabled="!database || loading" @click="writeData">
            {{ en ? "Write" : "写入" }}
          </button>
          <p>{{ en ? "Tables are created automatically from the measurement name." : "表会根据 measurement 名称自动创建，适合快速验证采集与查询流程。" }}</p>
        </article>
      </div>
    </template>
  </section>
</template>

<style scoped>
.influx-panel { padding: 28px 32px 48px; display: grid; gap: 18px; }
.empty, .summary, .database-bar, .card { border: 1px solid var(--color-border); background: var(--color-bg-panel); }
.empty { min-height: 260px; display: grid; place-content: center; text-align: center; gap: 8px; color: var(--color-text-muted); }
.empty strong { color: var(--color-text-primary); font-size: 18px; }
.summary { min-height: 82px; display: grid; grid-template-columns: minmax(260px, 1fr) 180px auto; align-items: stretch; }
.summary > div { padding: 18px 22px; display: grid; gap: 6px; border-right: 1px solid var(--color-border); }
.summary span, header span, label > span { color: var(--color-text-muted); font: 700 11px/1.2 monospace; letter-spacing: .08em; }
.summary strong { font: 600 18px/1.2 monospace; }
.summary button { margin: 16px; min-width: 88px; }
.database-bar { padding: 16px; display: grid; grid-template-columns: minmax(190px, 1fr) minmax(180px, .8fr) 150px auto auto; gap: 10px; align-items: end; }
label { display: grid; gap: 7px; min-width: 0; }
input, select, textarea { width: 100%; min-width: 0; border: 1px solid var(--color-border); background: var(--color-bg-muted); color: var(--color-text-primary); }
input, select { height: 40px; padding: 0 12px; }
textarea { padding: 14px; min-height: 132px; resize: vertical; font: 13px/1.65 monospace; }
button { min-height: 40px; padding: 0 16px; }
.workspace { display: grid; grid-template-columns: minmax(0, 1.7fr) minmax(280px, .8fr); gap: 18px; }
.card { min-width: 0; padding: 18px; display: grid; align-content: start; gap: 14px; }
.card header { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
h2 { margin: 5px 0 0; font-size: 18px; }
.write-card p, .result-meta { color: var(--color-text-muted); font-size: 12px; line-height: 1.6; }
.result { border: 1px solid var(--color-border); min-height: 180px; }
.result-meta { padding: 9px 12px; border-bottom: 1px solid var(--color-border); }
.table-scroll { overflow: auto; max-height: 360px; }
table { width: 100%; border-collapse: collapse; font: 12px/1.45 monospace; }
th, td { padding: 9px 11px; text-align: left; white-space: nowrap; border-right: 1px solid var(--color-border); border-bottom: 1px solid var(--color-border); }
th { position: sticky; top: 0; background: var(--color-bg-panel); color: var(--color-text-muted); }
.result-empty { min-height: 180px; display: grid; place-content: center; color: var(--color-text-muted); border: 1px dashed var(--color-border); }
@media (max-width: 1080px) {
  .database-bar { grid-template-columns: 1fr 1fr; }
  .workspace { grid-template-columns: 1fr; }
}
</style>
