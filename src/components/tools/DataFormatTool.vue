<script setup lang="ts">
import { computed, ref } from "vue";
import {
  diffJson,
  queryJsonPath,
  transformCsv,
  transformDataFormat,
} from "../../api/tools";
import type {
  CsvDelimiter,
  CsvDirection,
  CsvTransformResult,
  DataFormat,
  JsonDiffResult,
  JsonPathResult,
  OutputStyle,
  TransformResult,
} from "../../types";
import { formatBytes } from "../../utils/format";

type Mode = "convert" | "csv" | "diff" | "jsonpath" | "escape";
type EncodingKind = "json" | "base64" | "url" | "html";

const MODES: Array<[Mode, string, string]> = [
  ["convert", "格式化与转换", "JSON · YAML · TOML"],
  ["csv", "CSV / JSON", "表格数据互转"],
  ["diff", "JSON 差异", "逐字段比较"],
  ["jsonpath", "JSONPath", "按路径提取"],
  ["escape", "编码与转义", "Base64 · URL · HTML"],
];

const SOURCE_FORMATS: Array<[DataFormat, string]> = [
  ["auto", "自动识别"],
  ["json", "JSON"],
  ["yaml", "YAML"],
  ["toml", "TOML"],
];

const TARGET_FORMATS: Array<[DataFormat, string]> = [
  ["json", "JSON"],
  ["yaml", "YAML"],
  ["toml", "TOML"],
];

const SAMPLE = `{
  "service": "redis",
  "port": 6379,
  "tags": ["cache", "local"],
  "options": { "persistent": true }
}`;

const mode = ref<Mode>("convert");
const error = ref("");

// ── 格式化与转换 ────────────────────────────────────────────
const input = ref(SAMPLE);
const source = ref<DataFormat>("auto");
const target = ref<DataFormat>("json");
const style = ref<OutputStyle>("pretty");
const transformed = ref<TransformResult | null>(null);
const converting = ref(false);

// ── JSON 差异 ──────────────────────────────────────────────
const leftInput = ref('{\n  "name": "张三",\n  "age": 28\n}');
const rightInput = ref('{\n  "name": "张三",\n  "age": 30,\n  "city": "杭州"\n}');
const diffResult = ref<JsonDiffResult | null>(null);
const diffing = ref(false);

// ── JSONPath ───────────────────────────────────────────────
const pathInput = ref(SAMPLE);
const pathExpression = ref("$.tags[*]");
const pathResult = ref<JsonPathResult | null>(null);
const querying = ref(false);

// ── CSV / JSON ────────────────────────────────────────────
const csvInput = ref("name,age,city\n张三,28,杭州\n李四,31,上海\n");
const csvDirection = ref<CsvDirection>("csvToJson");
const csvDelimiter = ref<CsvDelimiter>("comma");
const csvResult = ref<CsvTransformResult | null>(null);
const csvConverting = ref(false);

// ── 编码与转义（纯前端字符串操作，无需往返后端）───────────────
const escapeInput = ref('他说："你好"\t换行\n结束');
const escapeOutput = ref("");
const encodingKind = ref<EncodingKind>("json");

const compressionRatio = computed(() => {
  const result = transformed.value;
  if (!result || result.inputBytes === 0) return null;
  return Math.round((result.outputBytes / result.inputBytes) * 100);
});

const diffKindLabel: Record<string, string> = {
  added: "新增",
  removed: "缺失",
  changed: "变更",
};

async function runTransform() {
  if (converting.value) return;
  converting.value = true;
  error.value = "";
  try {
    transformed.value = await transformDataFormat(
      input.value,
      source.value,
      target.value,
      style.value,
    );
  } catch (cause) {
    transformed.value = null;
    error.value = String(cause);
  } finally {
    converting.value = false;
  }
}

async function runDiff() {
  if (diffing.value) return;
  diffing.value = true;
  error.value = "";
  try {
    diffResult.value = await diffJson(leftInput.value, rightInput.value);
  } catch (cause) {
    diffResult.value = null;
    error.value = String(cause);
  } finally {
    diffing.value = false;
  }
}

async function runJsonPath() {
  if (querying.value) return;
  querying.value = true;
  error.value = "";
  try {
    pathResult.value = await queryJsonPath(pathInput.value, pathExpression.value);
  } catch (cause) {
    pathResult.value = null;
    error.value = String(cause);
  } finally {
    querying.value = false;
  }
}

async function runCsvTransform() {
  if (csvConverting.value) return;
  csvConverting.value = true;
  error.value = "";
  try {
    csvResult.value = await transformCsv(
      csvInput.value,
      csvDirection.value,
      csvDelimiter.value,
    );
  } catch (cause) {
    csvResult.value = null;
    error.value = String(cause);
  } finally {
    csvConverting.value = false;
  }
}

function loadCsvSample() {
  const separator = {
    comma: ",",
    tab: "\t",
    semicolon: ";",
    pipe: "|",
  }[csvDelimiter.value];
  csvInput.value =
    csvDirection.value === "csvToJson"
      ? [
          ["name", "age", "city"].join(separator),
          ["张三", "28", "杭州"].join(separator),
          ["李四", "31", "上海"].join(separator),
          "",
        ].join("\n")
      : '[\n  { "name": "张三", "age": 28, "city": "杭州" },\n  { "name": "李四", "age": 31, "city": "上海" }\n]';
  csvResult.value = null;
}

function encodeText() {
  error.value = "";
  try {
    switch (encodingKind.value) {
      case "json":
        escapeOutput.value = JSON.stringify(escapeInput.value).slice(1, -1);
        break;
      case "base64":
        escapeOutput.value = encodeBase64Utf8(escapeInput.value);
        break;
      case "url":
        escapeOutput.value = encodeURIComponent(escapeInput.value);
        break;
      case "html":
        escapeOutput.value = escapeInput.value
          .replaceAll("&", "&amp;")
          .replaceAll("<", "&lt;")
          .replaceAll(">", "&gt;")
          .replaceAll('"', "&quot;")
          .replaceAll("'", "&#39;");
        break;
    }
  } catch (cause) {
    error.value = `编码失败：${String(cause)}`;
  }
}

function decodeText() {
  error.value = "";
  try {
    switch (encodingKind.value) {
      case "json":
        escapeOutput.value = JSON.parse(`"${escapeInput.value}"`);
        break;
      case "base64":
        escapeOutput.value = decodeBase64Utf8(escapeInput.value);
        break;
      case "url":
        escapeOutput.value = decodeURIComponent(escapeInput.value);
        break;
      case "html": {
        const textarea = document.createElement("textarea");
        textarea.innerHTML = escapeInput.value;
        escapeOutput.value = textarea.value;
        break;
      }
    }
  } catch {
    error.value = `无法解码：请确认输入是合法的${encodingKindLabel(encodingKind.value)}内容`;
  }
}

function encodeBase64Utf8(value: string): string {
  const bytes = new TextEncoder().encode(value);
  let binary = "";
  for (let offset = 0; offset < bytes.length; offset += 8192) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 8192));
  }
  return btoa(binary);
}

function decodeBase64Utf8(value: string): string {
  const binary = atob(value.replace(/\s+/g, ""));
  const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
  return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
}

function encodingKindLabel(kind: EncodingKind): string {
  return {
    json: "JSON 字符串",
    base64: "Base64",
    url: "URL Component",
    html: "HTML 实体",
  }[kind];
}

/** 转义结果回填到输入框，方便连续做「转义 → 反转义」的往返验证。 */
function moveOutputToInput() {
  escapeInput.value = escapeOutput.value;
  escapeOutput.value = "";
  error.value = "";
}

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    error.value = "复制失败，请手动选中内容复制";
  }
}

function switchMode(next: Mode) {
  mode.value = next;
  error.value = "";
}
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo dataformat">{ }</span>
      <div>
        <div class="title-line">
          <h1>数据格式工具箱</h1>
          <span>LOCAL ONLY</span>
        </div>
        <p>格式转换、CSV、差异比较、路径查询与常用编码，全部在本机完成</p>
      </div>
    </div>
  </header>

  <div v-if="error" class="notice danger">
    <span>{{ error }}</span>
    <button type="button" @click="error = ''">×</button>
  </div>

  <section class="tool-page">
    <nav class="tool-modes">
      <button
        v-for="item in MODES"
        :key="item[0]"
        type="button"
        :class="{ active: mode === item[0] }"
        @click="switchMode(item[0])"
      >
        <strong>{{ item[1] }}</strong>
        <small>{{ item[2] }}</small>
      </button>
    </nav>

    <!-- 格式化与转换 -->
    <div v-if="mode === 'convert'" class="tool-panel">
      <div class="tool-controls">
        <label>
          来源
          <select v-model="source">
            <option v-for="item in SOURCE_FORMATS" :key="item[0]" :value="item[0]">
              {{ item[1] }}
            </option>
          </select>
        </label>
        <label>
          目标
          <select v-model="target">
            <option v-for="item in TARGET_FORMATS" :key="item[0]" :value="item[0]">
              {{ item[1] }}
            </option>
          </select>
        </label>
        <label>
          样式
          <select v-model="style">
            <option value="pretty">缩进美化</option>
            <option value="compact">压缩</option>
          </select>
        </label>
        <button class="primary" type="button" :disabled="converting" @click="runTransform">
          <span v-if="converting" class="spinner"></span>
          {{ converting ? "处理中" : "执行" }}
        </button>
      </div>

      <div class="tool-split">
        <div class="tool-pane">
          <div class="tool-pane-head">
            <p>INPUT</p>
            <span>{{ formatBytes(input.length) }}</span>
          </div>
          <textarea v-model="input" spellcheck="false" placeholder="粘贴 JSON、YAML 或 TOML"></textarea>
        </div>

        <div class="tool-pane">
          <div class="tool-pane-head">
            <p>OUTPUT</p>
            <span v-if="transformed">
              识别为 {{ transformed.detectedFormat.toUpperCase() }} ·
              {{ formatBytes(transformed.outputBytes) }}
              <template v-if="compressionRatio !== null"> · {{ compressionRatio }}%</template>
            </span>
            <button
              v-if="transformed"
              type="button"
              class="tool-copy"
              @click="copy(transformed.output)"
            >
              复制
            </button>
          </div>
          <pre v-if="transformed" class="tool-output">{{ transformed.output }}</pre>
          <div v-else class="tool-empty">点击「执行」查看结果</div>
        </div>
      </div>

      <div
        v-for="(warning, index) in transformed?.warnings ?? []"
        :key="index"
        class="tool-warning"
      >
        {{ warning }}
      </div>
    </div>

    <!-- CSV / JSON -->
    <div v-else-if="mode === 'csv'" class="tool-panel">
      <div class="tool-controls">
        <label>
          转换方向
          <select v-model="csvDirection" @change="csvResult = null">
            <option value="csvToJson">CSV → JSON</option>
            <option value="jsonToCsv">JSON → CSV</option>
          </select>
        </label>
        <label>
          分隔符
          <select v-model="csvDelimiter">
            <option value="comma">逗号 ,</option>
            <option value="tab">制表符 Tab</option>
            <option value="semicolon">分号 ;</option>
            <option value="pipe">竖线 |</option>
          </select>
        </label>
        <button
          class="primary"
          type="button"
          :disabled="csvConverting"
          @click="runCsvTransform"
        >
          <span v-if="csvConverting" class="spinner"></span>
          {{ csvConverting ? "转换中" : "开始转换" }}
        </button>
        <button type="button" @click="loadCsvSample">载入示例</button>
        <span v-if="csvResult" class="tool-summary">
          {{ csvResult.rowCount }} 行 · {{ csvResult.columnCount }} 列 ·
          {{ formatBytes(csvResult.outputBytes) }}
        </span>
      </div>

      <div class="tool-split">
        <div class="tool-pane">
          <div class="tool-pane-head">
            <p>{{ csvDirection === "csvToJson" ? "CSV INPUT" : "JSON INPUT" }}</p>
            <span>{{ formatBytes(csvInput.length) }}</span>
          </div>
          <textarea
            v-model="csvInput"
            spellcheck="false"
            :placeholder="csvDirection === 'csvToJson' ? '第一行需要包含表头' : '输入 JSON 对象数组'"
          ></textarea>
        </div>
        <div class="tool-pane">
          <div class="tool-pane-head">
            <p>{{ csvDirection === "csvToJson" ? "JSON OUTPUT" : "CSV OUTPUT" }}</p>
            <button
              v-if="csvResult"
              type="button"
              class="tool-copy"
              @click="copy(csvResult.output)"
            >
              复制
            </button>
          </div>
          <pre v-if="csvResult" class="tool-output">{{ csvResult.output }}</pre>
          <div v-else class="tool-empty">设置转换方向后点击「开始转换」</div>
        </div>
      </div>

      <p class="tool-note">
        CSV 第一行作为字段名；JSON 必须是对象数组。嵌套对象和数组会保留为紧凑 JSON
        字符串。单次最多处理 5 MiB、10000 行。
      </p>
    </div>

    <!-- JSON 差异 -->
    <div v-else-if="mode === 'diff'" class="tool-panel">
      <div class="tool-controls">
        <button class="primary" type="button" :disabled="diffing" @click="runDiff">
          <span v-if="diffing" class="spinner"></span>
          {{ diffing ? "比较中" : "开始比较" }}
        </button>
        <span v-if="diffResult" class="tool-summary">
          <template v-if="diffResult.identical">两份 JSON 完全一致</template>
          <template v-else>
            新增 {{ diffResult.added }} · 缺失 {{ diffResult.removed }} · 变更
            {{ diffResult.changed }}
          </template>
        </span>
      </div>

      <div class="tool-split">
        <div class="tool-pane">
          <div class="tool-pane-head"><p>LEFT</p></div>
          <textarea v-model="leftInput" spellcheck="false"></textarea>
        </div>
        <div class="tool-pane">
          <div class="tool-pane-head"><p>RIGHT</p></div>
          <textarea v-model="rightInput" spellcheck="false"></textarea>
        </div>
      </div>

      <div v-if="diffResult && !diffResult.identical" class="tool-table-wrap">
        <table class="tool-table">
          <thead>
            <tr>
              <th>路径</th>
              <th>类型</th>
              <th>左侧</th>
              <th>右侧</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="entry in diffResult.entries" :key="entry.path + entry.kind">
              <td><code>{{ entry.path }}</code></td>
              <td>
                <span class="diff-badge" :class="entry.kind">
                  {{ diffKindLabel[entry.kind] }}
                </span>
              </td>
              <td>{{ entry.left ?? "—" }}</td>
              <td>{{ entry.right ?? "—" }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- JSONPath -->
    <div v-else-if="mode === 'jsonpath'" class="tool-panel">
      <div class="tool-controls">
        <label class="grow">
          表达式
          <input
            v-model="pathExpression"
            type="text"
            spellcheck="false"
            placeholder="$.store.book[?(@.price > 50)].title"
          />
        </label>
        <button class="primary" type="button" :disabled="querying" @click="runJsonPath">
          <span v-if="querying" class="spinner"></span>
          {{ querying ? "查询中" : "查询" }}
        </button>
      </div>

      <div class="tool-split">
        <div class="tool-pane">
          <div class="tool-pane-head"><p>JSON</p></div>
          <textarea v-model="pathInput" spellcheck="false"></textarea>
        </div>
        <div class="tool-pane">
          <div class="tool-pane-head">
            <p>MATCHES</p>
            <span v-if="pathResult">命中 {{ pathResult.count }} 条</span>
          </div>
          <div v-if="!pathResult" class="tool-empty">输入表达式后点击「查询」</div>
          <div v-else-if="pathResult.count === 0" class="tool-empty">没有匹配的节点</div>
          <pre v-else class="tool-output">{{ pathResult.matches.join("\n") }}</pre>
        </div>
      </div>

      <p class="tool-note">
        遵循 RFC 9535。常用写法：<code>$.a.b</code> 取字段、<code>$.list[*]</code>
        取全部元素、<code>$..name</code> 递归查找、<code>$.list[?(@.age > 18)]</code> 条件过滤。
      </p>
    </div>

    <!-- 编码与转义 -->
    <div v-else class="tool-panel">
      <div class="tool-controls">
        <label>
          处理类型
          <select v-model="encodingKind" @change="escapeOutput = ''">
            <option value="json">JSON 字符串</option>
            <option value="base64">Base64 UTF-8</option>
            <option value="url">URL Component</option>
            <option value="html">HTML 实体</option>
          </select>
        </label>
        <button class="primary" type="button" @click="encodeText">编码</button>
        <button type="button" @click="decodeText">解码</button>
        <button v-if="escapeOutput" type="button" @click="moveOutputToInput">
          结果放回输入
        </button>
        <button v-if="escapeOutput" type="button" @click="copy(escapeOutput)">复制结果</button>
      </div>

      <div class="tool-split">
        <div class="tool-pane">
          <div class="tool-pane-head">
            <p>{{ encodingKindLabel(encodingKind).toUpperCase() }} INPUT</p>
          </div>
          <textarea v-model="escapeInput" spellcheck="false"></textarea>
        </div>
        <div class="tool-pane">
          <div class="tool-pane-head"><p>OUTPUT</p></div>
          <pre v-if="escapeOutput" class="tool-output">{{ escapeOutput }}</pre>
          <div v-else class="tool-empty">
            选择处理类型，然后对内容进行编码或解码
          </div>
        </div>
      </div>

      <p class="tool-note">
        Base64 使用 UTF-8，可正确处理中文；URL 模式使用
        <code>encodeURIComponent</code>；HTML 模式处理
        <code>&amp;</code>、<code>&lt;</code>、引号等常用实体。
      </p>
    </div>
  </section>
</template>
