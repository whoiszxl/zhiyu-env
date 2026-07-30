<script setup lang="ts">
import { computed, ref } from "vue";
import { save } from "@tauri-apps/plugin-dialog";
import { exportTestData } from "../../api/tools";
import type { TestDataExportInput } from "../../types";

type FieldType = "id" | "name" | "email" | "phone" | "integer" | "decimal" | "boolean" | "date" | "uuid" | "text" | "enum";
type Field = {
  id: string;
  name: string;
  type: FieldType;
  options: string;
  nullablePercent: number;
  unique: boolean;
  prefix: string;
  suffix: string;
  expanded: boolean;
};
type OutputFormat = "json" | "csv" | "sql";

const makeField = (name: string, type: FieldType, options = ""): Field => ({
  id: crypto.randomUUID(), name, type, options, nullablePercent: 0, unique: type === "id" || type === "email", prefix: "", suffix: "", expanded: false,
});
const fields = ref<Field[]>([
  makeField("id", "id"),
  makeField("name", "name"),
  makeField("email", "email"),
  makeField("status", "enum", "active,pending,disabled"),
  makeField("created_at", "date"),
]);
const count = ref(100);
const seed = ref("zhiyu-demo");
const format = ref<OutputFormat>("json");
const tableName = ref("users");
const generatedRows = ref<Record<string, unknown>[]>([]);
const generatedCount = ref(0);
const copied = ref(false);
const exporting = ref(false);
const message = ref("");
const error = ref("");
const output = computed(() => serialize(generatedRows.value, format.value));
const fieldTypes: Array<{ value: FieldType; label: string; hint: string }> = [
  { value: "id", label: "递增 ID", hint: "按行递增" },
  { value: "name", label: "中文姓名", hint: "内置姓名样本" },
  { value: "email", label: "邮箱", hint: "example.com" },
  { value: "phone", label: "手机号", hint: "中国大陆格式" },
  { value: "integer", label: "整数", hint: "选项：1-100" },
  { value: "decimal", label: "小数", hint: "选项：0-1000" },
  { value: "boolean", label: "布尔值", hint: "true / false" },
  { value: "date", label: "日期时间", hint: "ISO 8601" },
  { value: "uuid", label: "UUID", hint: "确定性 UUID v4" },
  { value: "text", label: "文本", hint: "选项为固定文本" },
  { value: "enum", label: "枚举", hint: "逗号分隔候选值" },
];
const surnames = ["林","陈","周","吴","徐","孙","胡","朱","高","何","郭","马"];
const given = ["一航","子墨","雨桐","浩然","思远","清扬","若溪","嘉宁","云舟","星野"];
const words = ["轻量开发环境","本地测试数据","接口联调样本","智屿生成内容","示例业务记录"];

function generate() {
  const total = Math.max(1, Math.min(1_000_000, Math.floor(Number(count.value) || 1)));
  count.value = total;
  const random = createRandom(seed.value);
  const previewCount = Math.min(total, 50);
  generatedRows.value = Array.from({ length: previewCount }, (_, index) =>
    Object.fromEntries(fields.value.filter((field) => field.name.trim()).map((field) => [
      safeIdentifier(field.name), valueFor(field, index, random),
    ])),
  );
  generatedCount.value = total;
  message.value = total > previewCount ? `已生成一致性预览；导出时将流式写入 ${total.toLocaleString()} 条` : `已生成 ${total} 条`;
  error.value = "";
}

function valueFor(field: Field, index: number, random: () => number): unknown {
  if (field.nullablePercent > 0 && range(random, 1, 100) <= field.nullablePercent) return null;
  const uniqueTail = field.unique ? `-${index + 1}` : "";
  let value: unknown;
  switch (field.type) {
    case "id": value = index + 1; break;
    case "name": value = `${pick(surnames, random)}${pick(given, random)}${uniqueTail}`; break;
    case "email": value = `dev${index + 1}_${range(random,100,999)}${uniqueTail}@example.com`; break;
    case "phone": value = `1${pick([3,5,6,7,8,9],random)}${String(range(random,0,999999999)).padStart(9,"0")}`; break;
    case "integer": { const [min,max] = parseRange(field.options,[1,100]); value = range(random,min,max); break; }
    case "decimal": { const [min,max] = parseRange(field.options,[0,1000]); value = Number((min + random()*(max-min)).toFixed(2)); break; }
    case "boolean": value = random() >= .5; break;
    case "date": value = new Date(1767225600000 - range(random,0,365*86400000)).toISOString(); break;
    case "uuid": value = deterministicUuid(random); break;
    case "enum": { const values = field.options.split(",").map((item) => item.trim()).filter(Boolean); value = `${pick(values.length ? values : ["A"],random)}${uniqueTail}`; break; }
    case "text": value = `${field.options.trim() || pick(words,random)}${uniqueTail}`; break;
  }
  if ((field.prefix || field.suffix) && value != null) return `${field.prefix}${String(value)}${field.suffix}`;
  return value;
}

function createRandom(value: string) {
  let state = 0xcbf29ce484222325n;
  for (const byte of new TextEncoder().encode(value)) {
    state ^= BigInt(byte);
    state = BigInt.asUintN(64, state * 0x100000001b3n);
  }
  if (state === 0n) state = 1n;
  return () => {
    state ^= BigInt.asUintN(64, state << 13n);
    state ^= state >> 7n;
    state ^= BigInt.asUintN(64, state << 17n);
    state = BigInt.asUintN(64, state);
    return Number(state >> 11n) / 9007199254740992;
  };
}

function range(random: () => number, min: number, max: number) {
  return Math.floor(random() * (max - min + 1)) + min;
}

function pick<T>(items: T[], random: () => number): T {
  return items[Math.min(items.length - 1, Math.floor(random() * items.length))];
}

function deterministicUuid(random: () => number) {
  const bytes = Array.from({ length: 16 }, () => range(random, 0, 255));
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = bytes.map((byte) => byte.toString(16).padStart(2,"0")).join("");
  return `${hex.slice(0,8)}-${hex.slice(8,12)}-${hex.slice(12,16)}-${hex.slice(16,20)}-${hex.slice(20)}`;
}

function parseRange(options: string, fallback: [number,number]): [number,number] {
  const values = options.split(/[,~-]/).map(Number).filter(Number.isFinite);
  return values.length >= 2 ? [Math.min(values[0],values[1]),Math.max(values[0],values[1])] : fallback;
}

function safeIdentifier(value: string) {
  return value.trim().replace(/[^a-zA-Z0-9_]/g,"_") || "sample_data";
}

function csvCell(value: unknown) {
  return `"${(value == null ? "" : String(value)).replaceAll('"','""')}"`;
}

function sqlValue(value: unknown) {
  if (value == null) return "NULL";
  if (typeof value === "number") return String(value);
  if (typeof value === "boolean") return value ? "TRUE" : "FALSE";
  return `'${String(value).replaceAll("'","''")}'`;
}

function serialize(rows: Record<string,unknown>[], target: OutputFormat) {
  if (!rows.length) return "";
  const keys = Object.keys(rows[0]);
  if (target === "json") return JSON.stringify(rows,null,2);
  if (target === "csv") return [keys.map(csvCell).join(","),...rows.map((row)=>keys.map((key)=>csvCell(row[key])).join(","))].join("\n");
  return rows.map((row)=>`INSERT INTO ${safeIdentifier(tableName.value)} (${keys.join(", ")}) VALUES (${keys.map((key)=>sqlValue(row[key])).join(", ")});`).join("\n");
}

function addField() {
  fields.value.push(makeField(`field_${fields.value.length + 1}`, "text"));
}

async function copyOutput() {
  if (!output.value) return;
  await navigator.clipboard.writeText(output.value);
  copied.value = true;
  window.setTimeout(() => (copied.value = false), 1200);
}

async function exportFile() {
  if (!fields.value.some((field) => field.name.trim())) {
    error.value = "请至少保留一个有效字段";
    return;
  }
  const extension = format.value;
  const path = await save({
    title: "导出测试数据",
    defaultPath: `zhiyu-test-data.${extension}`,
    filters: [{ name: extension.toUpperCase(), extensions: [extension] }],
  });
  if (!path) return;
  exporting.value = true;
  error.value = "";
  try {
    const input: TestDataExportInput = {
      seed: seed.value,
      count: count.value,
      format: format.value,
      tableName: tableName.value,
      fields: fields.value.filter((field) => field.name.trim()).map((field) => ({
        name: field.name,
        kind: field.type,
        options: field.options,
        nullablePercent: field.nullablePercent,
        unique: field.unique,
        prefix: field.prefix,
        suffix: field.suffix,
      })),
      path,
    };
    const result = await exportTestData(input);
    message.value = `已导出 ${result.rows.toLocaleString()} 条 · ${formatBytes(result.bytes)} · ${result.path}`;
  } catch (cause) {
    error.value = String(cause);
  } finally {
    exporting.value = false;
  }
}

function formatBytes(value: number) {
  if (value < 1024) return `${value} B`;
  if (value < 1024*1024) return `${(value/1024).toFixed(1)} KiB`;
  return `${(value/1024/1024).toFixed(1)} MiB`;
}
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo data-logo">D</span>
      <div><div class="title-line"><h1>测试数据生成器</h1><span>DATA FACTORY 2.0</span></div><p>固定种子、字段约束与百万级流式导出，数据全程在本机生成</p></div>
    </div>
    <div class="header-actions"><button type="button" :disabled="!output" @click="copyOutput">{{copied?"已复制":"复制预览"}}</button><button type="button" :disabled="exporting" @click="exportFile"><span v-if="exporting" class="spinner"></span>{{exporting?"导出中…":"导出文件"}}</button><button class="primary" type="button" @click="generate">生成预览</button></div>
  </header>
  <div v-if="error" class="notice danger"><span>{{error}}</span><button @click="error=''">×</button></div>
  <div v-if="message" class="notice"><span>{{message}}</span><button @click="message=''">×</button></div>

  <main class="data-page">
    <section class="data-config">
      <div class="data-panel-head"><div><small>SCHEMA</small><h2>字段结构</h2></div><button type="button" @click="addField">＋ 添加字段</button></div>
      <div class="field-columns"><span>字段名</span><span>数据类型</span><span>选项 / 范围</span><span>唯一</span><span>空值 %</span><span></span></div>
      <article v-for="field in fields" :key="field.id" class="field-item">
        <div class="field-row">
          <input v-model="field.name" spellcheck="false" />
          <select v-model="field.type"><option v-for="type in fieldTypes" :key="type.value" :value="type.value">{{type.label}}</option></select>
          <input v-model="field.options" :placeholder="fieldTypes.find(item=>item.value===field.type)?.hint" :disabled="!['integer','decimal','text','enum'].includes(field.type)" />
          <input v-model="field.unique" type="checkbox" />
          <input v-model.number="field.nullablePercent" type="number" min="0" max="100" />
          <div><button title="前后缀" type="button" @click="field.expanded=!field.expanded">•••</button><button title="删除" type="button" :disabled="fields.length===1" @click="fields=fields.filter(item=>item.id!==field.id)">×</button></div>
        </div>
        <div v-if="field.expanded" class="field-advanced"><label>前缀<input v-model="field.prefix" placeholder="user_" /></label><label>后缀<input v-model="field.suffix" placeholder="_test" /></label><p>前后缀会把数值转换为字符串；空值比例在生成每一行时独立计算。</p></div>
      </article>
      <div class="generation-options">
        <label>固定种子<input v-model="seed" spellcheck="false" /></label>
        <label>生成数量<input v-model.number="count" type="number" min="1" max="1000000" /></label>
        <label>输出格式<select v-model="format"><option value="json">JSON</option><option value="csv">CSV</option><option value="sql">SQL INSERT</option></select></label>
        <label v-if="format==='sql'">表名<input v-model="tableName" /></label>
        <span>相同种子和字段配置始终生成相同数据。页面最多预览 50 条，完整结果由 Rust 流式写入文件，不会一次性占满内存。</span>
      </div>
    </section>

    <section class="data-output">
      <div class="data-panel-head"><div><small>PREVIEW</small><h2>结果预览</h2></div><div><span v-if="generatedRows.length">显示 {{generatedRows.length}} / {{generatedCount.toLocaleString()}} 条</span><span v-else>尚未生成</span></div></div>
      <pre v-if="output">{{output}}</pre>
      <div v-else class="data-empty"><span>{ }</span><strong>配置字段后生成预览</strong><small>大批量数据请使用“导出文件”。</small></div>
    </section>
  </main>
</template>

<style scoped>
.data-logo{background:#84672d}.data-page{display:grid;grid-template-columns:minmax(560px,1.08fr) minmax(430px,.92fr);gap:12px;padding:22px 30px 38px}.data-config,.data-output{min-width:0;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.data-panel-head{display:flex;min-height:52px;align-items:center;justify-content:space-between;padding:7px 13px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.data-panel-head small{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace;letter-spacing:.12em}.data-panel-head h2{margin:4px 0 0;font-size:12px}.data-panel-head button{height:27px;padding:0 8px;font-size:7px}.data-panel-head>div:last-child{color:var(--color-text-muted);font-size:7px}.field-columns,.field-row{display:grid;grid-template-columns:minmax(90px,.75fr) minmax(105px,.8fr) minmax(130px,1.2fr) 42px 58px 58px;gap:6px;padding:0 11px}.field-columns{height:30px;align-items:center;border-bottom:1px solid var(--color-border);color:var(--color-text-muted);font-size:7px}.field-columns span:nth-child(4),.field-columns span:nth-child(5){text-align:center}.field-item{border-bottom:1px solid var(--color-border)}.field-row{align-items:center;padding-top:6px;padding-bottom:6px}.field-row input:not([type=checkbox]),.field-row select,.generation-options input,.generation-options select{box-sizing:border-box;width:100%;height:29px;padding:0 7px;font-size:7px}.field-row input[type=checkbox]{width:13px;height:13px;min-height:0;justify-self:center}.field-row>div{display:flex;gap:3px}.field-row button{width:27px;height:27px;padding:0;background:transparent;color:var(--color-text-muted)}.field-row button:last-child{color:var(--color-danger-text)}.field-advanced{display:grid;grid-template-columns:150px 150px minmax(0,1fr);align-items:end;gap:8px;padding:0 11px 9px}.field-advanced label{display:grid;gap:4px;color:var(--color-text-muted);font-size:7px}.field-advanced input{height:28px;padding:0 7px;font-size:7px}.field-advanced p{margin:0 0 6px;color:var(--color-text-muted);font-size:6px}.generation-options{display:grid;grid-template-columns:minmax(120px,1fr) 100px 110px minmax(100px,.8fr);gap:8px;padding:12px}.generation-options label{display:grid;gap:5px;color:var(--color-text-muted);font-size:7px}.generation-options>span{grid-column:1/-1;color:var(--color-text-muted);font-size:7px;line-height:1.55}.data-output{display:grid;grid-template-rows:auto minmax(0,1fr);min-height:590px}.data-output pre{max-height:calc(100vh - 230px);margin:0;overflow:auto;padding:14px;color:var(--color-text-secondary);font:8px/1.6 "SFMono-Regular",Consolas,monospace;white-space:pre}.data-empty{display:grid;place-items:center;align-content:center;gap:7px;color:var(--color-text-muted)}.data-empty>span{display:grid;width:45px;height:45px;place-items:center;border:1px solid var(--color-border);border-radius:50%;color:var(--color-accent)}.data-empty strong{color:var(--color-text-primary);font-size:9px}.data-empty small{font-size:7px}@media(max-width:1100px){.data-page{grid-template-columns:1fr}.data-output{min-height:430px}}
</style>
