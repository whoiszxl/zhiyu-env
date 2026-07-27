<script setup lang="ts">
import { computed, ref } from "vue";
import { diffJson, queryJsonPath, transformDataFormat } from "../../api/tools";
import type {
  DataFormat,
  JsonDiffResult,
  JsonPathResult,
  OutputStyle,
  TransformResult,
} from "../../types";
import { formatBytes } from "../../utils/format";

type Mode = "convert" | "diff" | "jsonpath" | "escape";

const MODES: Array<[Mode, string, string]> = [
  ["convert", "格式化与转换", "JSON · YAML · TOML"],
  ["diff", "JSON 差异", "逐字段比较"],
  ["jsonpath", "JSONPath", "按路径提取"],
  ["escape", "转义工具", "字符串转义"],
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

// ── 转义（纯前端字符串操作，无需往返后端）────────────────────
const escapeInput = ref('他说："你好"\t换行\n结束');
const escapeOutput = ref("");

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

function runEscape() {
  error.value = "";
  // JSON.stringify 会补上首尾引号，这里去掉只保留转义后的内容
  escapeOutput.value = JSON.stringify(escapeInput.value).slice(1, -1);
}

function runUnescape() {
  error.value = "";
  try {
    escapeOutput.value = JSON.parse(`"${escapeInput.value}"`);
  } catch {
    error.value = "无法反转义：内容不是合法的转义字符串，请检查反斜杠与引号";
  }
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
          <h1>JSON / YAML / TOML 工具箱</h1>
          <span>LOCAL ONLY</span>
        </div>
        <p>格式化、互转、差异比较与路径查询，全部在本机完成，内容不出网</p>
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

    <!-- 转义 -->
    <div v-else class="tool-panel">
      <div class="tool-controls">
        <button class="primary" type="button" @click="runEscape">转义</button>
        <button type="button" @click="runUnescape">反转义</button>
        <button v-if="escapeOutput" type="button" @click="moveOutputToInput">
          结果放回输入
        </button>
        <button v-if="escapeOutput" type="button" @click="copy(escapeOutput)">复制结果</button>
      </div>

      <div class="tool-split">
        <div class="tool-pane">
          <div class="tool-pane-head"><p>INPUT</p></div>
          <textarea v-model="escapeInput" spellcheck="false"></textarea>
        </div>
        <div class="tool-pane">
          <div class="tool-pane-head"><p>OUTPUT</p></div>
          <pre v-if="escapeOutput" class="tool-output">{{ escapeOutput }}</pre>
          <div v-else class="tool-empty">把内容按 JSON 字符串规则转义或还原</div>
        </div>
      </div>

      <p class="tool-note">
        转义会把换行、制表符和引号变成 <code>\n</code>、<code>\t</code>、<code>\"</code>，
        便于把一段文本安全地嵌进 JSON 字符串；反转义是相反的过程。
      </p>
    </div>
  </section>
</template>
