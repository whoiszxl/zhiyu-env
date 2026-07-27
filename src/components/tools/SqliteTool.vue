<script setup lang="ts">
import { open, save } from "@tauri-apps/plugin-dialog";
import { computed, ref } from "vue";
import {
  createSqliteDatabase,
  executeSqlite,
  getSqliteOverview,
  listSqliteTables,
} from "../../api/tools";
import type {
  SqliteOverview,
  SqliteQueryResult,
  SqliteTable,
} from "../../types";
import { formatBytes } from "../../utils/format";

const filePath = ref("");
const overview = ref<SqliteOverview | null>(null);
const tables = ref<SqliteTable[]>([]);
const sql = ref("SELECT name, type FROM sqlite_schema ORDER BY type, name;");
const result = ref<SqliteQueryResult | null>(null);
const loading = ref(false);
const querying = ref(false);
const notice = ref("");
const error = ref("");

const fileName = computed(
  () => filePath.value.split(/[\\/]/).at(-1) || "尚未选择数据库",
);

async function loadDatabase(path: string) {
  loading.value = true;
  notice.value = "";
  error.value = "";
  try {
    const [nextOverview, nextTables] = await Promise.all([
      getSqliteOverview(path),
      listSqliteTables(path),
    ]);
    filePath.value = path;
    overview.value = nextOverview;
    tables.value = nextTables;
    result.value = null;
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

async function chooseDatabase() {
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      title: "选择 SQLite 数据库",
      filters: [
        { name: "SQLite 数据库", extensions: ["sqlite", "sqlite3", "db"] },
        { name: "所有文件", extensions: ["*"] },
      ],
    });
    if (typeof selected === "string") await loadDatabase(selected);
  } catch (cause) {
    error.value = String(cause);
  }
}

async function createDatabase() {
  try {
    const selected = await save({
      title: "新建 SQLite 数据库",
      defaultPath: "database.sqlite",
      filters: [{ name: "SQLite 数据库", extensions: ["sqlite", "sqlite3", "db"] }],
    });
    if (typeof selected !== "string") return;
    loading.value = true;
    overview.value = await createSqliteDatabase(selected);
    filePath.value = selected;
    tables.value = [];
    result.value = null;
    notice.value = "SQLite 数据库创建成功";
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

function previewTable(table: SqliteTable) {
  const identifier = `"${table.name.replaceAll('"', '""')}"`;
  sql.value =
    table.tableType === "view"
      ? `SELECT * FROM ${identifier} LIMIT 100;`
      : `SELECT * FROM ${identifier} LIMIT 100;`;
  void runQuery();
}

async function runQuery(confirmed = false) {
  if (!filePath.value || querying.value || !sql.value.trim()) return;
  querying.value = true;
  notice.value = "";
  error.value = "";
  try {
    result.value = await executeSqlite(
      filePath.value,
      sql.value,
      confirmed,
    );
    const [nextOverview, nextTables] = await Promise.all([
      getSqliteOverview(filePath.value),
      listSqliteTables(filePath.value),
    ]);
    overview.value = nextOverview;
    tables.value = nextTables;
  } catch (cause) {
    const message = String(cause);
    if (
      message.includes("CONFIRM_REQUIRED:") &&
      window.confirm("该 SQL 会删除或清空数据，确定继续吗？")
    ) {
      querying.value = false;
      await runQuery(true);
      return;
    }
    result.value = null;
    error.value = message.replace("CONFIRM_REQUIRED:", "");
  } finally {
    querying.value = false;
  }
}

function handleShortcut(event: KeyboardEvent) {
  if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
    event.preventDefault();
    void runQuery();
  }
}
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo sqlite">S</span>
      <div>
        <div class="title-line">
          <h1>SQLite 本地数据库</h1>
          <span>v{{ overview?.version ?? "embedded" }}</span>
        </div>
        <p>打开、创建并查询本地 SQLite 文件，不启动后台服务</p>
      </div>
    </div>
    <div class="header-actions">
      <button type="button" :disabled="loading" @click="createDatabase">
        新建数据库
      </button>
      <button class="primary" type="button" :disabled="loading" @click="chooseDatabase">
        {{ filePath ? "更换文件" : "打开数据库" }}
      </button>
    </div>
  </header>

  <div v-if="notice || error" class="notice" :class="{ danger: error }">
    <span>{{ error || notice }}</span>
    <button type="button" @click="notice = error = ''">×</button>
  </div>

  <section class="duckdb-tool-page sqlite-tool-page">
    <div v-if="!filePath" class="duckdb-install-card sqlite-open-card">
      <span class="service-logo sqlite">S</span>
      <h2>打开本地 SQLite 数据库</h2>
      <p>
        SQLite 引擎已经内置在智屿中，不依赖系统安装，也没有常驻进程。数据库文件只在本机读取和修改。
      </p>
      <div class="sqlite-empty-actions">
        <button type="button" :disabled="loading" @click="chooseDatabase">
          打开现有数据库
        </button>
        <button type="button" :disabled="loading" @click="createDatabase">
          新建空数据库
        </button>
      </div>
    </div>

    <template v-else>
      <div class="metric-grid duckdb-metrics">
        <article class="metric-card">
          <p>ENGINE</p>
          <strong>v{{ overview?.version }}</strong>
          <small>内嵌 SQLite</small>
        </article>
        <article class="metric-card">
          <p>TABLES</p>
          <strong>{{ overview?.tableCount ?? 0 }}</strong>
          <small>用户数据表</small>
        </article>
        <article class="metric-card">
          <p>FILE SIZE</p>
          <strong>{{ formatBytes(overview?.fileSizeBytes ?? 0) }}</strong>
          <small>数据库磁盘占用</small>
        </article>
        <article class="metric-card">
          <p>JOURNAL</p>
          <strong class="small-metric">{{ overview?.journalMode ?? "—" }}</strong>
          <small>{{ overview?.indexCount ?? 0 }} 个索引</small>
        </article>
      </div>

      <div class="duckdb-file-card">
        <div class="duckdb-file-icon">S</div>
        <div>
          <p>DATABASE FILE</p>
          <strong>{{ fileName }}</strong>
          <small :title="filePath">{{ filePath }}</small>
        </div>
        <button type="button" @click="chooseDatabase">更换文件</button>
      </div>

      <div class="sqlite-layout">
        <aside class="sqlite-table-list">
          <div class="duckdb-result-head">
            <div>
              <p>SCHEMA</p>
              <h2>数据表与视图</h2>
            </div>
            <span>{{ tables.length }}</span>
          </div>
          <button
            v-for="table in tables"
            :key="`${table.tableType}:${table.name}`"
            type="button"
            @click="previewTable(table)"
          >
            <span>{{ table.tableType === "view" ? "V" : "T" }}</span>
            <strong>{{ table.name }}</strong>
            <small>{{ table.tableType }}</small>
          </button>
          <div v-if="tables.length === 0" class="duckdb-empty">
            暂无数据表，可在右侧执行 CREATE TABLE。
          </div>
        </aside>

        <div class="duckdb-workbench">
          <div class="duckdb-editor">
            <div class="duckdb-editor-head">
              <div>
                <p>SQL CONSOLE</p>
                <h2>查询与编辑</h2>
              </div>
              <div class="duckdb-templates">
                <button
                  type="button"
                  @click="sql = 'SELECT name, type FROM sqlite_schema ORDER BY type, name;'"
                >
                  查看结构
                </button>
                <button
                  type="button"
                  @click="sql = 'PRAGMA integrity_check;'"
                >
                  完整性检查
                </button>
              </div>
            </div>
            <textarea
              v-model="sql"
              spellcheck="false"
              @keydown="handleShortcut"
            ></textarea>
            <div class="duckdb-runbar">
              <span>写入直接保存到所选文件，危险操作需要确认</span>
              <span>⌘ Enter 执行</span>
              <button type="button" :disabled="querying" @click="runQuery()">
                <span v-if="querying" class="spinner"></span>
                {{ querying ? "执行中" : "运行 SQL" }}
              </button>
            </div>
          </div>

          <div class="duckdb-result-panel">
            <div class="duckdb-result-head">
              <div>
                <p>QUERY RESULT</p>
                <h2>结果</h2>
              </div>
              <span v-if="result">
                {{ result.summary }} · {{ result.elapsedMs }} ms
              </span>
            </div>
            <div v-if="querying && !result" class="duckdb-empty">
              正在执行 SQLite 查询…
            </div>
            <div v-else-if="!result" class="duckdb-empty">
              选择数据表或输入 SQL 后运行
            </div>
            <div v-else-if="result.columns.length === 0" class="duckdb-empty">
              {{ result.summary }}
            </div>
            <div v-else class="duckdb-table-wrap">
              <table>
                <thead>
                  <tr>
                    <th v-for="column in result.columns" :key="column">
                      {{ column }}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, rowIndex) in result.rows" :key="rowIndex">
                    <td
                      v-for="(value, columnIndex) in row"
                      :key="columnIndex"
                      :class="{ null: value === null }"
                    >
                      {{ value === null ? "NULL" : value }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p v-if="result?.truncated" class="duckdb-result-note">
              为保持界面流畅，单次最多展示 500 行。
            </p>
          </div>
        </div>
      </div>
    </template>
  </section>
</template>
