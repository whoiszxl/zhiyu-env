<script setup lang="ts">
import { open } from "@tauri-apps/plugin-dialog";
import { computed, inject, onMounted, ref } from "vue";
import {
  getDuckdbStatus,
  installDuckdb,
  queryDuckdbFile,
} from "../../api/services";
import { INSTALL_TASK_KEY } from "../../tools/types";
import type { DuckdbQueryResult, DuckdbStatus } from "../../types";
import { formatBytes } from "../../utils/format";

const FALLBACK_VERSION = "1.5.5";
const DATABASE_EXTENSIONS = ["duckdb", "db"];
const QUERYABLE_EXTENSIONS = [
  "csv",
  "tsv",
  "json",
  "jsonl",
  "ndjson",
  "parquet",
  "duckdb",
  "db",
];

const SQL_TEMPLATES = {
  preview: "SELECT * FROM selected_file LIMIT 100;",
  count: "SELECT count(*) AS total_rows FROM selected_file;",
  schema: "DESCRIBE selected_file;",
  tables: "SHOW ALL TABLES;",
} as const;

type TemplateKey = keyof typeof SQL_TEMPLATES;

const installTask = inject(INSTALL_TASK_KEY);

const status = ref<DuckdbStatus | null>(null);
const filePath = ref("");
const sql = ref<string>(SQL_TEMPLATES.preview);
const result = ref<DuckdbQueryResult | null>(null);
const statusLoading = ref(false);
const installing = ref(false);
const querying = ref(false);
const notice = ref("");
const error = ref("");

const fileName = computed(() => {
  const parts = filePath.value.split(/[\\/]/);
  return parts.at(-1) || "尚未选择文件";
});

const fileExtension = computed(
  () => fileName.value.split(".").at(-1)?.toLowerCase() ?? "",
);

const fileType = computed(() => {
  const extension = fileExtension.value;
  if (!filePath.value) return "—";
  if (DATABASE_EXTENSIONS.includes(extension)) return "DUCKDB";
  return extension.toUpperCase();
});

const isDatabase = computed(() =>
  DATABASE_EXTENSIONS.includes(fileExtension.value),
);

async function loadStatus() {
  if (statusLoading.value) return;
  statusLoading.value = true;
  try {
    status.value = await getDuckdbStatus();
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    statusLoading.value = false;
  }
}

async function install() {
  if (installing.value) return;
  installing.value = true;
  notice.value = "";
  error.value = "";
  const operationId = installTask?.start("duckdb", `DuckDB ${FALLBACK_VERSION}`);
  try {
    status.value = await installDuckdb(operationId ?? "");
    if (operationId) installTask?.succeed(operationId);
    notice.value = `DuckDB ${status.value.version} 安装成功`;
  } catch (cause) {
    if (operationId) installTask?.fail(operationId, cause);
    error.value = String(cause);
  } finally {
    installing.value = false;
  }
}

async function chooseFile() {
  try {
    const selected = await open({
      multiple: false,
      directory: false,
      title: "选择本地数据文件",
      filters: [{ name: "DuckDB 可查询文件", extensions: QUERYABLE_EXTENSIONS }],
    });
    if (typeof selected !== "string") return;

    filePath.value = selected;
    result.value = null;
    const extension = selected.split(".").at(-1)?.toLowerCase() ?? "";
    sql.value = DATABASE_EXTENSIONS.includes(extension)
      ? SQL_TEMPLATES.tables
      : SQL_TEMPLATES.preview;
    notice.value = "";
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  }
}

function useTemplate(template: TemplateKey) {
  sql.value = SQL_TEMPLATES[template];
}

async function runQuery() {
  if (querying.value || !status.value?.installed || !filePath.value) return;

  querying.value = true;
  notice.value = "";
  error.value = "";
  try {
    result.value = await queryDuckdbFile(filePath.value, sql.value);
  } catch (cause) {
    result.value = null;
    error.value = String(cause);
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

onMounted(loadStatus);
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo duckdb">D</span>
      <div>
        <div class="title-line">
          <h1>DuckDB 本地文件查询器</h1>
          <span>v{{ status?.version ?? FALLBACK_VERSION }}</span>
        </div>
        <p>直接查询 CSV、JSON、Parquet 和 DuckDB 文件，不启动后台服务</p>
      </div>
    </div>
    <div class="header-actions">
      <button
        v-if="!status?.installed"
        class="primary"
        type="button"
        :disabled="installing || statusLoading"
        @click="install"
      >
        <template v-if="installing">
          <span class="spinner"></span>
          <span>安装中</span>
        </template>
        <span v-else>下载并安装</span>
      </button>
      <button v-else class="primary" type="button" @click="chooseFile">
        选择本地文件
      </button>
    </div>
  </header>

  <div v-if="notice || error" class="notice" :class="{ danger: error }">
    <span>{{ error || notice }}</span>
    <button type="button" @click="notice = error = ''">×</button>
  </div>

  <section class="duckdb-tool-page">
    <div v-if="statusLoading && !status" class="duckdb-empty">
      正在检查 DuckDB CLI…
    </div>

    <div v-else-if="!status?.installed" class="duckdb-install-card">
      <span class="service-logo duckdb">D</span>
      <h2>安装 DuckDB CLI</h2>
      <p>
        智屿会下载官方 macOS universal 单文件程序，校验 SHA-256 后安装到
        <code>~/.devbox/</code>，不会修改系统 PATH。
      </p>
      <button type="button" :disabled="installing" @click="install">
        <template v-if="installing">
          <span class="spinner"></span>
          <span>正在下载并校验…</span>
        </template>
        <span v-else>安装 DuckDB {{ FALLBACK_VERSION }}</span>
      </button>
    </div>

    <template v-else>
      <div class="metric-grid duckdb-metrics">
        <article class="metric-card">
          <p>ENGINE</p>
          <strong>v{{ status.version }}</strong>
          <small>官方 DuckDB CLI</small>
        </article>
        <article class="metric-card">
          <p>FILE TYPE</p>
          <strong class="small-metric">{{ fileType }}</strong>
          <small>当前数据源</small>
        </article>
        <article class="metric-card">
          <p>DISK</p>
          <strong>{{ formatBytes(status.installationBytes) }}</strong>
          <small>查询引擎占用</small>
        </article>
        <article class="metric-card">
          <p>EXECUTION</p>
          <strong class="small-metric">LOCAL</strong>
          <small>只读 · 15 秒超时</small>
        </article>
      </div>

      <div class="duckdb-file-card">
        <div class="duckdb-file-icon">{{ fileType.slice(0, 1) }}</div>
        <div>
          <p>SELECTED FILE</p>
          <strong>{{ fileName }}</strong>
          <small :title="filePath">
            {{ filePath || "选择一个 CSV、JSON、Parquet 或 DuckDB 文件" }}
          </small>
        </div>
        <button type="button" @click="chooseFile">
          {{ filePath ? "更换文件" : "选择文件" }}
        </button>
      </div>

      <div class="duckdb-workbench">
        <div class="duckdb-editor">
          <div class="duckdb-editor-head">
            <div>
              <p>READ-ONLY SQL</p>
              <h2>查询语句</h2>
            </div>
            <div class="duckdb-templates">
              <template v-if="!isDatabase">
                <button type="button" @click="useTemplate('preview')">
                  预览 100 行
                </button>
                <button type="button" @click="useTemplate('count')">
                  统计行数
                </button>
                <button type="button" @click="useTemplate('schema')">
                  查看字段
                </button>
              </template>
              <button v-else type="button" @click="useTemplate('tables')">
                查看所有表
              </button>
            </div>
          </div>
          <textarea
            v-model="sql"
            spellcheck="false"
            :disabled="!filePath"
            @keydown="handleShortcut"
          ></textarea>
          <div class="duckdb-runbar">
            <span>
              {{
                isDatabase
                  ? ".duckdb 文件以 safe + readonly 模式打开"
                  : "使用 selected_file 作为所选文件的表名"
              }}
            </span>
            <span>⌘ Enter 执行</span>
            <button
              type="button"
              :disabled="!filePath || querying"
              @click="runQuery"
            >
              <span v-if="querying" class="spinner"></span>
              {{ querying ? "查询中" : "运行查询" }}
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
            正在本机执行查询…
          </div>
          <div v-else-if="!result" class="duckdb-empty">
            {{ filePath ? "输入只读 SQL 后运行查询" : "请先选择一个本地文件" }}
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
            为保持界面流畅，单次最多展示 500 行；请用 WHERE 或 LIMIT 缩小结果。
          </p>
        </div>
      </div>
    </template>
  </section>
</template>
