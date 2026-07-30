<script setup lang="ts">
import { computed, ref } from "vue";
import type { ServiceKind } from "../../types";

type TargetLanguage = "typescript" | "java" | "go" | "rust";
type Column = { name: string; sqlType: string; nullable: boolean };
const activeTab = ref<"formatter" | "model" | "snippets">("formatter");
const sql = ref("SELECT u.id, u.name, COUNT(o.id) AS order_count FROM users u LEFT JOIN orders o ON o.user_id = u.id WHERE u.enabled = 1 GROUP BY u.id, u.name ORDER BY order_count DESC;");
const ddl = ref("CREATE TABLE users (\n  id BIGINT NOT NULL,\n  name VARCHAR(100) NOT NULL,\n  email VARCHAR(255),\n  created_at TIMESTAMP NOT NULL\n);");
const language = ref<TargetLanguage>("typescript");
const copied = ref(false);
const snippets = [
  { name: "分页查询", engine: "通用", sql: "SELECT * FROM table_name ORDER BY id DESC LIMIT 20 OFFSET 0;" },
  { name: "查找重复值", engine: "通用", sql: "SELECT field_name, COUNT(*) AS count FROM table_name GROUP BY field_name HAVING COUNT(*) > 1;" },
  { name: "MySQL 表占用", engine: "MySQL", sql: "SELECT table_schema, table_name, data_length + index_length AS total_bytes FROM information_schema.tables ORDER BY total_bytes DESC;" },
  { name: "PostgreSQL 表占用", engine: "PostgreSQL", sql: "SELECT schemaname, relname, pg_total_relation_size(relid) AS total_bytes FROM pg_catalog.pg_statio_user_tables ORDER BY total_bytes DESC;" },
  { name: "最近执行中的查询", engine: "PostgreSQL", sql: "SELECT pid, now() - query_start AS duration, query FROM pg_stat_activity WHERE state <> 'idle' ORDER BY duration DESC;" },
  { name: "MongoDB 集合统计", engine: "MongoDB", sql: "db.getCollectionNames().map(name => ({ name, stats: db.getCollection(name).stats() }))" },
];

const formattedSql = computed(() => formatSql(sql.value));
const columns = computed(() => parseColumns(ddl.value));
const modelCode = computed(() => generateModel(columns.value, language.value));

function formatSql(value: string) {
  const keywords = ["SELECT","FROM","LEFT JOIN","RIGHT JOIN","INNER JOIN","JOIN","WHERE","GROUP BY","ORDER BY","HAVING","LIMIT","OFFSET","VALUES","SET","RETURNING","UNION ALL","UNION"];
  let output = value.trim().replace(/\s+/g," ");
  for (const keyword of keywords) {
    output = output.replace(new RegExp(`\\s+${keyword.replace(" ","\\\\s+")}\\s+`,"gi"), `\n${keyword} `);
  }
  output = output.replace(/\s*,\s*/g,",\n  ").replace(/^SELECT\s+/i,"SELECT\n  ");
  return output;
}
function parseColumns(value: string): Column[] {
  const body = value.match(/\(([\s\S]*)\)/)?.[1] ?? "";
  return body.split(",").map((line) => line.trim()).filter((line) => line && !/^(PRIMARY|UNIQUE|KEY|CONSTRAINT|FOREIGN|CHECK)\b/i.test(line)).map((line) => {
    const match = line.match(/^[`"]?([a-zA-Z_][\w]*)[`"]?\s+([a-zA-Z]+(?:\s+[a-zA-Z]+)?)(?:\([^)]*\))?/);
    return match ? { name: match[1], sqlType: match[2].toUpperCase(), nullable: !/\bNOT\s+NULL\b/i.test(line) } : null;
  }).filter((item): item is Column => Boolean(item));
}
function words(value: string) { return value.split("_").filter(Boolean); }
function pascal(value: string) { return words(value).map((part) => part[0].toUpperCase()+part.slice(1)).join(""); }
function camel(value: string) { const p=pascal(value); return p[0]?.toLowerCase()+p.slice(1); }
function typeFor(column: Column, target: TargetLanguage) {
  const type = column.sqlType;
  const family = /INT|SERIAL|NUMBER/.test(type) ? "integer" : /DECIMAL|NUMERIC|REAL|DOUBLE|FLOAT/.test(type) ? "decimal" : /BOOL/.test(type) ? "boolean" : /DATE|TIME/.test(type) ? "date" : /JSON/.test(type) ? "json" : "string";
  const table: Record<TargetLanguage,Record<string,string>> = {
    typescript:{integer:"number",decimal:"number",boolean:"boolean",date:"string",json:"unknown",string:"string"},
    java:{integer:"Long",decimal:"BigDecimal",boolean:"Boolean",date:"Instant",json:"String",string:"String"},
    go:{integer:"int64",decimal:"float64",boolean:"bool",date:"time.Time",json:"json.RawMessage",string:"string"},
    rust:{integer:"i64",decimal:"f64",boolean:"bool",date:"chrono::DateTime<chrono::Utc>",json:"serde_json::Value",string:"String"},
  };
  return table[target][family];
}
function generateModel(items: Column[], target: TargetLanguage) {
  if (!items.length) return "";
  if (target === "typescript") return `export interface User {\n${items.map((c)=>`  ${camel(c.name)}${c.nullable?"?":""}: ${typeFor(c,target)};`).join("\n")}\n}`;
  if (target === "java") return `public class User {\n${items.map((c)=>`    private ${typeFor(c,target)} ${camel(c.name)};`).join("\n")}\n`;
  if (target === "go") return `type User struct {\n${items.map((c)=>`    ${pascal(c.name)} ${typeFor(c,target)} \`json:"${c.name}"\``).join("\n")}\n`;
  return `#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct User {\n${items.map((c)=>`    pub ${c.name}: ${c.nullable?`Option<${typeFor(c,target)}>`:typeFor(c,target)},`).join("\n")}\n}`;
}
async function copy(value: string) { if(!value)return; await navigator.clipboard.writeText(value); copied.value=true; setTimeout(()=>copied.value=false,1000); }
function useSnippet(value: string) { sql.value=value; activeTab.value="formatter"; }
function navigate(kind: ServiceKind) { window.dispatchEvent(new CustomEvent("zhiyu:navigate",{detail:{type:"service",id:kind}})); }
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity"><span class="service-logo dbdev-logo">DB</span><div><div class="title-line"><h1>数据库开发辅助</h1><span>DATABASE LAB</span></div><p>格式化 SQL、生成数据模型并收藏常用开发查询</p></div></div>
    <div class="header-actions"><button @click="navigate('mysql')">MySQL ↗</button><button @click="navigate('postgres')">PostgreSQL ↗</button><button @click="navigate('mongodb')">MongoDB ↗</button></div>
  </header>
  <nav class="detail-tabs db-tabs"><button :class="{active:activeTab==='formatter'}" @click="activeTab='formatter'">SQL 格式化</button><button :class="{active:activeTab==='model'}" @click="activeTab='model'">模型生成</button><button :class="{active:activeTab==='snippets'}" @click="activeTab='snippets'">查询模板</button></nav>
  <main class="dbdev-page">
    <template v-if="activeTab==='formatter'">
      <section class="db-editor"><div class="db-head"><div><small>INPUT SQL</small><h2>原始 SQL</h2></div><button @click="sql=''">清空</button></div><textarea v-model="sql" spellcheck="false"></textarea></section>
      <section class="db-editor"><div class="db-head"><div><small>FORMATTED</small><h2>格式化结果</h2></div><button :disabled="!formattedSql" @click="copy(formattedSql)">{{copied?'已复制':'复制'}}</button></div><pre>{{ formattedSql }}</pre></section>
    </template>
    <template v-else-if="activeTab==='model'">
      <section class="db-editor"><div class="db-head"><div><small>CREATE TABLE</small><h2>表结构</h2></div><span>{{columns.length}} 个字段</span></div><textarea v-model="ddl" spellcheck="false"></textarea><p>支持常见 CREATE TABLE 字段定义；索引和约束会被忽略。</p></section>
      <section class="db-editor"><div class="db-head"><div><small>MODEL</small><h2>代码模型</h2></div><select v-model="language"><option value="typescript">TypeScript</option><option value="java">Java</option><option value="go">Go</option><option value="rust">Rust</option></select><button :disabled="!modelCode" @click="copy(modelCode)">复制</button></div><pre>{{modelCode}}</pre></section>
    </template>
    <section v-else class="snippet-panel">
      <div class="db-head"><div><small>QUERY RECIPES</small><h2>开发查询模板</h2></div><span>只生成查询，不会自动执行</span></div>
      <div class="snippet-grid"><article v-for="item in snippets" :key="item.name"><span>{{item.engine}}</span><h3>{{item.name}}</h3><pre>{{item.sql}}</pre><button @click="useSnippet(item.sql)">放入格式化器</button></article></div>
    </section>
  </main>
</template>

<style scoped>
.dbdev-logo{background:#385f79;font-size:10px}.db-tabs{height:44px}.dbdev-page{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:14px;padding:24px 32px 40px}.db-editor,.snippet-panel{min-width:0;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.db-head{display:flex;min-height:58px;align-items:center;justify-content:space-between;gap:8px;padding:8px 14px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.db-head>div:first-child{margin-right:auto}.db-head small{color:var(--color-text-muted);font:8px "SFMono-Regular",monospace;letter-spacing:.12em}.db-head h2{margin:4px 0 0;font-size:14px}.db-head button,.db-head select{height:29px;padding:0 9px;border:1px solid var(--color-border-strong);background:var(--color-bg-panel);color:var(--color-text-primary);font-size:8px}.db-head>span{color:var(--color-text-muted);font-size:8px}.db-editor textarea,.db-editor pre{box-sizing:border-box;width:100%;height:430px;margin:0;overflow:auto;padding:15px;border:0;background:transparent;color:var(--color-text-secondary);font:9px/1.7 "SFMono-Regular",Consolas,monospace;resize:none;white-space:pre-wrap}.db-editor textarea:focus{outline:0;box-shadow:inset 0 -2px var(--color-accent)}.db-editor>p{margin:0;padding:9px 14px;border-top:1px solid var(--color-border);color:var(--color-text-muted);font-size:8px}.snippet-panel{grid-column:1/-1}.snippet-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr))}.snippet-grid article{display:grid;min-height:170px;align-content:start;gap:8px;padding:14px;border-right:1px solid var(--color-border);border-bottom:1px solid var(--color-border)}.snippet-grid span{color:var(--color-accent);font:8px "SFMono-Regular",monospace}.snippet-grid h3{margin:0;font-size:11px}.snippet-grid pre{min-height:62px;margin:0;color:var(--color-text-muted);font:8px/1.5 "SFMono-Regular",monospace;white-space:pre-wrap}.snippet-grid button{justify-self:start;height:28px;padding:0 9px;border:1px solid var(--color-border-strong);background:var(--color-bg-panel);color:var(--color-text-primary);font-size:8px}@media(max-width:1000px){.dbdev-page{grid-template-columns:1fr}.snippet-grid{grid-template-columns:repeat(2,1fr)}}
</style>
