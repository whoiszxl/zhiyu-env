<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  deleteOlapProfile,
  executeOlapSql,
  listOlapDatabases,
  listOlapProfiles,
  listOlapTables,
  saveOlapProfile,
  testOlapConnection,
} from "../../api/tools";
import type {
  OlapDatabaseInfo,
  OlapEngine,
  OlapProfile,
  OlapProfileInput,
  OlapQueryResult,
  OlapTableInfo,
} from "../../types";

const props = defineProps<{ engine: OlapEngine }>();
const { t } = useI18n();
const profiles = ref<OlapProfile[]>([]);
const selectedId = ref("");
const databases = ref<OlapDatabaseInfo[]>([]);
const selectedDatabase = ref("");
const tables = ref<OlapTableInfo[]>([]);
const selectedTable = ref("");
const query = ref("");
const result = ref<OlapQueryResult | null>(null);
const loading = ref(false);
const testing = ref(false);
const error = ref("");
const connectionVersion = ref("");
const connectionLatency = ref<number | null>(null);
const showProfileDialog = ref(false);
const editingId = ref<string | null>(null);
const draft = ref<OlapProfileInput>(defaultDraft());

const selectedProfile = computed(
  () => profiles.value.find((profile) => profile.id === selectedId.value) ?? null,
);
const title = computed(() => (props.engine === "clickhouse" ? "ClickHouse" : "Apache Doris"));
const defaultSql = computed(() =>
  props.engine === "clickhouse"
    ? "SELECT version(), now(), currentDatabase()"
    : "SELECT version(), now(), database()",
);

function defaultDraft(): OlapProfileInput {
  return {
    id: null,
    name: props.engine === "clickhouse" ? "本地 ClickHouse" : "Doris 开发集群",
    engine: props.engine,
    endpoint:
      props.engine === "clickhouse"
        ? "http://127.0.0.1:8123"
        : "http://127.0.0.1:8030",
    username: props.engine === "clickhouse" ? "default" : "root",
    password: "",
    database: props.engine === "clickhouse" ? "default" : "information_schema",
  };
}

async function loadProfiles(selectId?: string) {
  profiles.value = await listOlapProfiles(props.engine);
  if (selectId && profiles.value.some((profile) => profile.id === selectId)) {
    selectedId.value = selectId;
  } else if (!profiles.value.some((profile) => profile.id === selectedId.value)) {
    selectedId.value = profiles.value[0]?.id ?? "";
  }
}

function openCreate() {
  editingId.value = null;
  draft.value = defaultDraft();
  showProfileDialog.value = true;
}

function openEdit() {
  if (!selectedProfile.value) return;
  const profile = selectedProfile.value;
  editingId.value = profile.id;
  draft.value = {
    id: profile.id,
    name: profile.name,
    engine: props.engine,
    endpoint: profile.endpoint,
    username: profile.username,
    password: "",
    database: profile.database,
  };
  showProfileDialog.value = true;
}

async function saveProfile() {
  loading.value = true;
  error.value = "";
  try {
    const profile = await saveOlapProfile(draft.value);
    showProfileDialog.value = false;
    await loadProfiles(profile.id);
    await connect();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

async function removeProfile() {
  const profile = selectedProfile.value;
  if (!profile || !window.confirm(t("olap.deleteConfirm", { name: profile.name }))) return;
  try {
    await deleteOlapProfile(profile.id);
    resetConnection();
    await loadProfiles();
  } catch (cause) {
    error.value = String(cause);
  }
}

async function connect() {
  if (!selectedId.value || testing.value) return;
  testing.value = true;
  error.value = "";
  result.value = null;
  try {
    const [test, items] = await Promise.all([
      testOlapConnection(selectedId.value),
      listOlapDatabases(selectedId.value),
    ]);
    connectionVersion.value = test.version;
    connectionLatency.value = test.elapsedMs;
    databases.value = items;
    const preferred = selectedProfile.value?.database;
    selectedDatabase.value =
      items.find((item) => item.name === preferred)?.name ??
      items.find((item) => !item.system)?.name ??
      items[0]?.name ??
      "";
    await loadTables();
  } catch (cause) {
    resetConnection();
    error.value = String(cause);
  } finally {
    testing.value = false;
  }
}

async function loadTables() {
  if (!selectedId.value || !selectedDatabase.value) {
    tables.value = [];
    return;
  }
  try {
    tables.value = await listOlapTables(selectedId.value, selectedDatabase.value);
    selectedTable.value = tables.value.some((table) => table.name === selectedTable.value)
      ? selectedTable.value
      : "";
  } catch (cause) {
    tables.value = [];
    error.value = String(cause);
  }
}

function chooseTable(table: OlapTableInfo) {
  selectedTable.value = table.name;
  const escaped = props.engine === "clickhouse"
    ? `\`${selectedDatabase.value.replaceAll("`", "``")}\`.\`${table.name.replaceAll("`", "``")}\``
    : `\`${selectedDatabase.value.replaceAll("`", "``")}\`.\`${table.name.replaceAll("`", "``")}\``;
  query.value = `SELECT * FROM ${escaped} LIMIT 100`;
}

async function run(confirmed = false) {
  if (!selectedId.value || !query.value.trim() || loading.value) return;
  loading.value = true;
  error.value = "";
  try {
    result.value = await executeOlapSql(
      selectedId.value,
      selectedDatabase.value,
      query.value,
      confirmed,
    );
  } catch (cause) {
    const message = String(cause);
    if (message.includes("CONFIRM_REQUIRED:")) {
      const keyword = message.split("CONFIRM_REQUIRED:")[1]?.split(/[^\w]/)[0] ?? "SQL";
      if (window.confirm(t("olap.destructiveConfirm", { keyword }))) {
        loading.value = false;
        await run(true);
        return;
      }
    } else {
      error.value = message;
    }
  } finally {
    loading.value = false;
  }
}

function resetConnection() {
  connectionVersion.value = "";
  connectionLatency.value = null;
  databases.value = [];
  selectedDatabase.value = "";
  tables.value = [];
  selectedTable.value = "";
  result.value = null;
}

function formatRows(value: number | null) {
  if (value == null) return "—";
  return new Intl.NumberFormat().format(value);
}

watch(selectedDatabase, () => void loadTables());
watch(
  () => props.engine,
  async () => {
    query.value = defaultSql.value;
    resetConnection();
    await loadProfiles();
  },
);

onMounted(async () => {
  query.value = defaultSql.value;
  try {
    await loadProfiles();
  } catch (cause) {
    error.value = String(cause);
  }
});
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo olap" :class="engine">{{ engine === "clickhouse" ? "C" : "D" }}</span>
      <div>
        <div class="title-line">
          <h1>{{ title }}</h1>
          <span>OLAP DATABASE</span>
        </div>
        <p>{{ t(`olap.${engine}.subtitle`) }}</p>
      </div>
    </div>
    <div class="header-actions">
      <button type="button" @click="openCreate">＋ {{ t("olap.newConnection") }}</button>
      <button class="primary" type="button" :disabled="!selectedId || testing" @click="connect">
        <span v-if="testing" class="spinner"></span>
        {{ testing ? t("olap.connecting") : t("olap.connect") }}
      </button>
    </div>
  </header>

  <div v-if="error" class="notice danger">
    <span>{{ error }}</span><button type="button" @click="error = ''">×</button>
  </div>

  <main class="olap-page">
    <aside class="connection-pane">
      <header>
        <div><small>CONNECTIONS</small><h2>{{ t("olap.connections") }}</h2></div>
        <button type="button" title="New" @click="openCreate">＋</button>
      </header>
      <div v-if="profiles.length" class="profile-list">
        <button
          v-for="profile in profiles"
          :key="profile.id"
          type="button"
          :class="{ active: selectedId === profile.id }"
          @click="selectedId = profile.id; resetConnection()"
        >
          <i></i>
          <span><strong>{{ profile.name }}</strong><small>{{ profile.endpoint }}</small></span>
        </button>
      </div>
      <div v-else class="profile-empty">
        <b>{{ engine === "clickhouse" ? "C" : "D" }}</b>
        <strong>{{ t("olap.noConnections") }}</strong>
        <p>{{ t(`olap.${engine}.empty`) }}</p>
        <button type="button" @click="openCreate">{{ t("olap.addFirst") }}</button>
      </div>
      <footer v-if="selectedProfile">
        <button type="button" @click="openEdit">{{ t("common.edit") }}</button>
        <button class="danger-text" type="button" @click="removeProfile">{{ t("common.delete") }}</button>
      </footer>
    </aside>

    <section class="database-pane">
      <header>
        <div><small>DATABASES</small><h2>{{ t("olap.databases") }}</h2></div>
        <span v-if="connectionVersion"><i></i>{{ connectionLatency }} ms</span>
      </header>
      <div v-if="connectionVersion" class="server-version">
        <small>SERVER</small><strong>{{ connectionVersion }}</strong>
      </div>
      <div v-if="databases.length" class="database-list">
        <button
          v-for="database in databases"
          :key="database.name"
          type="button"
          :class="{ active: selectedDatabase === database.name }"
          @click="selectedDatabase = database.name"
        >
          <span>▱</span><strong>{{ database.name }}</strong><small v-if="database.system">SYS</small>
        </button>
      </div>
      <div v-else class="side-empty">{{ t("olap.connectHint") }}</div>
    </section>

    <section class="workspace-pane">
      <div class="table-strip">
        <header><small>TABLES · {{ selectedDatabase || "—" }}</small><span>{{ tables.length }}</span></header>
        <div v-if="tables.length">
          <button
            v-for="table in tables"
            :key="table.name"
            type="button"
            :class="{ active: selectedTable === table.name }"
            @click="chooseTable(table)"
          >
            <span><strong>{{ table.name }}</strong><small>{{ table.engine || t("olap.table") }}</small></span>
            <code>{{ formatRows(table.rows) }}</code>
          </button>
        </div>
        <p v-else>{{ selectedDatabase ? t("olap.noTables") : t("olap.selectDatabase") }}</p>
      </div>

      <div class="query-workspace">
        <header>
          <div><small>SQL CONSOLE</small><h2>{{ t("olap.query") }}</h2></div>
          <div>
            <button type="button" @click="query = defaultSql">{{ t("olap.example") }}</button>
            <button class="primary" type="button" :disabled="!selectedId || loading" @click="run()">
              <span v-if="loading" class="spinner"></span>
              {{ loading ? t("olap.running") : t("olap.run") }}
            </button>
          </div>
        </header>
        <textarea v-model="query" spellcheck="false" @keydown.meta.enter.prevent="run()" @keydown.ctrl.enter.prevent="run()"></textarea>
        <div class="query-hint"><span>{{ t("olap.shortcut") }}</span><span>{{ selectedDatabase || t("olap.noDatabase") }}</span></div>

        <section class="result-panel">
          <header>
            <div><small>RESULT</small><h3>{{ t("olap.result") }}</h3></div>
            <span v-if="result">{{ result.summary }} · {{ result.elapsedMs }} ms</span>
          </header>
          <div v-if="result?.columns.length" class="result-scroll">
            <table>
              <thead><tr><th v-for="column in result.columns" :key="column">{{ column }}</th></tr></thead>
              <tbody>
                <tr v-for="(row, rowIndex) in result.rows" :key="rowIndex">
                  <td v-for="(cell, cellIndex) in row" :key="cellIndex">{{ cell ?? "NULL" }}</td>
                </tr>
              </tbody>
            </table>
            <p v-if="result.truncated" class="truncated">{{ t("olap.truncated") }}</p>
          </div>
          <div v-else-if="result" class="result-empty success">{{ result.summary }}</div>
          <div v-else class="result-empty">{{ t("olap.resultEmpty") }}</div>
        </section>
      </div>
    </section>
  </main>

  <div v-if="showProfileDialog" class="modal-backdrop" @click.self="showProfileDialog = false">
    <form class="profile-dialog" @submit.prevent="saveProfile">
      <header>
        <div><small>DATABASE CONNECTION</small><h2>{{ editingId ? t("olap.editConnection") : t("olap.newConnection") }}</h2></div>
        <button type="button" @click="showProfileDialog = false">×</button>
      </header>
      <div class="profile-fields">
        <label>{{ t("olap.name") }}<input v-model="draft.name" required maxlength="80" /></label>
        <label>{{ t("olap.endpoint") }}<input v-model="draft.endpoint" required /></label>
        <label>{{ t("olap.username") }}<input v-model="draft.username" required autocomplete="username" /></label>
        <label>
          {{ t("olap.password") }}
          <input v-model="draft.password" type="password" autocomplete="new-password" :placeholder="editingId ? t('olap.keepPassword') : t('olap.optional')" />
        </label>
        <label>{{ t("olap.defaultDatabase") }}<input v-model="draft.database" /></label>
      </div>
      <p class="platform-note">{{ t(`olap.${engine}.platformNote`) }}</p>
      <footer>
        <button type="button" @click="showProfileDialog = false">{{ t("common.cancel") }}</button>
        <button class="primary" type="submit" :disabled="loading">{{ t("common.save") }}</button>
      </footer>
    </form>
  </div>
</template>

<style scoped>
.service-logo.olap.clickhouse{background:#f0c933;color:#171717}.service-logo.olap.doris{background:#3568b8}.olap-page{display:grid;grid-template-columns:220px 210px minmax(0,1fr);min-height:calc(100vh - 118px);padding:20px 28px 34px;gap:0}.connection-pane,.database-pane,.workspace-pane{min-width:0;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.database-pane,.workspace-pane{border-left:0}.connection-pane>header,.database-pane>header,.table-strip>header,.query-workspace>header,.result-panel>header{display:flex;min-height:48px;align-items:center;justify-content:space-between;padding:7px 11px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}header small{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace;letter-spacing:.11em}header h2{margin:4px 0 0;font-size:10px}.connection-pane>header button{display:grid;width:26px;height:26px;padding:0;place-items:center;font-size:12px}.profile-list button,.database-list button,.table-strip>div>button{display:grid;width:100%;border:0;border-bottom:1px solid var(--color-border);background:transparent;color:var(--color-text-primary);text-align:left}.profile-list button{grid-template-columns:7px minmax(0,1fr);align-items:center;gap:9px;padding:10px 11px}.profile-list i{width:6px;height:6px;border-radius:50%;background:var(--color-text-muted)}.profile-list button.active{box-shadow:inset 2px 0 var(--color-accent);background:var(--color-selected-bg)}.profile-list button.active i{background:var(--color-success-text)}.profile-list span{min-width:0}.profile-list strong,.database-list strong{display:block;overflow:hidden;font-size:8px;text-overflow:ellipsis;white-space:nowrap}.profile-list small{display:block;overflow:hidden;margin-top:3px;color:var(--color-text-muted);font:6px "SFMono-Regular",monospace;text-overflow:ellipsis;white-space:nowrap}.profile-empty,.side-empty{display:grid;min-height:300px;align-content:center;justify-items:center;gap:8px;padding:18px;color:var(--color-text-muted);text-align:center}.profile-empty b{display:grid;width:40px;height:40px;place-items:center;border:1px solid var(--color-border);border-radius:50%;font-size:14px}.profile-empty strong{color:var(--color-text-primary);font-size:9px}.profile-empty p{margin:0;font-size:7px;line-height:1.55}.profile-empty button{font-size:7px}.connection-pane{display:flex;min-height:620px;flex-direction:column}.connection-pane footer{display:flex;margin-top:auto;border-top:1px solid var(--color-border)}.connection-pane footer button{width:50%;border:0;border-right:1px solid var(--color-border);background:var(--color-bg-muted);font-size:7px}.danger-text{color:var(--color-danger-text)!important}.database-pane>header>span{display:flex;align-items:center;gap:5px;color:var(--color-text-muted);font:7px "SFMono-Regular",monospace}.database-pane>header i{width:6px;height:6px;border-radius:50%;background:var(--color-success-text)}.server-version{padding:8px 10px;border-bottom:1px solid var(--color-border)}.server-version small{display:block;color:var(--color-text-muted);font-size:6px}.server-version strong{display:block;overflow:hidden;margin-top:3px;font:7px "SFMono-Regular",monospace;text-overflow:ellipsis;white-space:nowrap}.database-list button{grid-template-columns:18px minmax(0,1fr) auto;align-items:center;gap:6px;padding:9px 10px}.database-list button>span{color:var(--color-text-muted)}.database-list button>small{color:var(--color-text-muted);font-size:6px}.database-list button.active{box-shadow:inset 2px 0 var(--color-accent);background:var(--color-selected-bg)}.workspace-pane{display:grid;grid-template-columns:190px minmax(0,1fr)}.table-strip{min-width:0;border-right:1px solid var(--color-border)}.table-strip>header>span{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace}.table-strip>div>button{grid-template-columns:minmax(0,1fr) auto;align-items:center;gap:6px;padding:9px 10px}.table-strip strong{display:block;overflow:hidden;font-size:8px;text-overflow:ellipsis;white-space:nowrap}.table-strip small{display:block;overflow:hidden;margin-top:3px;color:var(--color-text-muted);font-size:6px;text-overflow:ellipsis;white-space:nowrap}.table-strip code{color:var(--color-text-muted);font-size:6px}.table-strip button.active{box-shadow:inset 2px 0 var(--color-accent);background:var(--color-selected-bg)}.table-strip>p{padding:16px;color:var(--color-text-muted);font-size:7px;text-align:center}.query-workspace{min-width:0}.query-workspace>header>div:last-child{display:flex;gap:6px}.query-workspace>header button{height:28px;font-size:7px}.query-workspace>textarea{box-sizing:border-box;width:100%;height:145px;padding:12px;border:0;border-bottom:1px solid var(--color-border);border-radius:0;background:var(--color-code-bg);color:var(--color-text-primary);font:8px/1.65 "SFMono-Regular",monospace;resize:vertical}.query-workspace>textarea:focus{outline:1px solid var(--color-focus);outline-offset:-1px}.query-hint{display:flex;justify-content:space-between;padding:6px 10px;border-bottom:1px solid var(--color-border);color:var(--color-text-muted);font:6px "SFMono-Regular",monospace}.result-panel>header h3{margin:4px 0 0;font-size:9px}.result-panel>header>span{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace}.result-scroll{max-height:390px;overflow:auto}.result-scroll table{width:100%;border-collapse:collapse;font-size:7px;white-space:nowrap}.result-scroll th,.result-scroll td{max-width:280px;overflow:hidden;padding:7px 9px;border-right:1px solid var(--color-border);border-bottom:1px solid var(--color-border);text-align:left;text-overflow:ellipsis}.result-scroll th{position:sticky;top:0;background:var(--color-bg-muted);color:var(--color-text-muted);font-weight:600}.result-scroll td{font-family:"SFMono-Regular",monospace}.truncated{margin:0;padding:7px;color:var(--color-warning-text);font-size:7px}.result-empty{display:grid;min-height:190px;place-items:center;color:var(--color-text-muted);font-size:8px}.result-empty.success{color:var(--color-success-text)}.modal-backdrop{position:fixed;z-index:3000;inset:0;display:grid;place-items:center;padding:40px;background:var(--color-overlay)}.profile-dialog{width:min(620px,calc(100vw - 80px));border:1px solid var(--color-border-strong);background:var(--color-bg-panel);box-shadow:0 18px 60px rgba(0,0,0,.3)}.profile-dialog>header{display:flex;align-items:center;justify-content:space-between;padding:13px 16px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.profile-dialog>header h2{margin:5px 0 0;font-size:13px}.profile-dialog>header button{border:0;background:transparent;font-size:15px}.profile-fields{display:grid;grid-template-columns:1fr 1fr;gap:12px;padding:16px}.profile-fields label{display:grid;gap:6px;color:var(--color-text-muted);font-size:7px}.profile-fields label:nth-child(2){grid-column:span 2}.profile-fields input{box-sizing:border-box;width:100%;height:34px;padding:0 9px;font-size:8px}.platform-note{margin:0 16px 14px;padding:9px 10px;border:1px solid var(--color-border);background:var(--color-bg-muted);color:var(--color-text-muted);font-size:7px;line-height:1.55}.profile-dialog>footer{display:flex;justify-content:flex-end;gap:7px;padding:11px 16px;border-top:1px solid var(--color-border)}.profile-dialog>footer button{height:30px;font-size:7px}@media(max-width:1200px){.olap-page{grid-template-columns:190px 180px minmax(0,1fr);padding:16px}.workspace-pane{grid-template-columns:155px minmax(0,1fr)}}@media(max-width:900px){.olap-page{grid-template-columns:180px minmax(0,1fr)}.database-pane{display:none}.workspace-pane{grid-template-columns:150px minmax(0,1fr)}}
</style>
