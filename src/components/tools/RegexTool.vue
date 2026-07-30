<script setup lang="ts">
import { computed, ref } from "vue";
import { toolUiText } from "../../i18n/toolUi";
import AiAssistDialog from "../AiAssistDialog.vue";
import type { AiAssistOption } from "../../types";

interface RegexMatch {
  index: number;
  value: string;
  groups: string[];
  namedGroups: Record<string, string>;
}

const PRESETS = [
  ["", "选择常用表达式"],
  ["^[\\w.+-]+@[\\w.-]+\\.[A-Za-z]{2,}$", "邮箱地址"],
  ["https?://[^\\s]+", "HTTP / HTTPS URL"],
  ["(?:\\d{1,3}\\.){3}\\d{1,3}", "IPv4 地址"],
  ["\\b\\d{4}-\\d{2}-\\d{2}\\b", "日期 YYYY-MM-DD"],
  ["^1[3-9]\\d{9}$", "中国大陆手机号"],
] as const;

const pattern = ref("(\\w+)@(\\w+\\.\\w+)");
const testText = ref(
  toolUiText(
    "联系 alice@example.com 或 bob@test.dev 获取帮助。",
    "Contact alice@example.com or bob@test.dev for help.",
  ),
);
const replacement = ref("$1 [at] $2");
const flagGlobal = ref(true);
const flagIgnoreCase = ref(false);
const flagMultiline = ref(false);
const flagDotAll = ref(false);
const selectedPreset = ref("");
const aiOpen = ref(false);
const aiOptions: AiAssistOption[] = [{
  id: "regex", label: "生成正则", hint: "描述需要匹配的文本规则，并提供正例和反例", canApply: true,
}];

const flags = computed(
  () =>
    `${flagGlobal.value ? "g" : ""}${flagIgnoreCase.value ? "i" : ""}${
      flagMultiline.value ? "m" : ""
    }${flagDotAll.value ? "s" : ""}`,
);

const evaluation = computed(() => {
  if (!pattern.value) return { error: "", matches: [] as RegexMatch[] };
  try {
    const matchFlags = flags.value.includes("g") ? flags.value : `${flags.value}g`;
    const expression = new RegExp(pattern.value, matchFlags);
    const matches: RegexMatch[] = [];
    for (const match of testText.value.matchAll(expression)) {
      matches.push({
        index: match.index,
        value: match[0],
        groups: match.slice(1).map((value) => value ?? ""),
        namedGroups: Object.fromEntries(
          Object.entries(match.groups ?? {}).map(([key, value]) => [key, value ?? ""]),
        ),
      });
      if (matches.length >= (flagGlobal.value ? 500 : 1)) break;
    }
    return { error: "", matches };
  } catch (cause) {
    return { error: cause instanceof Error ? cause.message : String(cause), matches: [] };
  }
});

const replacementOutput = computed(() => {
  if (!pattern.value || evaluation.value.error) return "";
  try {
    return testText.value.replace(new RegExp(pattern.value, flags.value), replacement.value);
  } catch {
    return "";
  }
});

const previewSegments = computed(() => {
  const matches = evaluation.value.matches.slice(0, 200);
  const segments: Array<{ text: string; matched: boolean }> = [];
  let cursor = 0;
  for (const match of matches) {
    if (match.index > cursor) {
      segments.push({ text: testText.value.slice(cursor, match.index), matched: false });
    }
    segments.push({ text: match.value || "​", matched: true });
    cursor = match.index + match.value.length;
  }
  if (cursor < testText.value.length) {
    segments.push({ text: testText.value.slice(cursor), matched: false });
  }
  return segments;
});

function applyPreset() {
  if (selectedPreset.value) pattern.value = selectedPreset.value;
}

async function copy(value: string) {
  await navigator.clipboard.writeText(value);
}

function applyAiRegex(content: string) {
  pattern.value = content.trim().replace(/^\/|\/[gimsuy]*$/g, "");
  aiOpen.value = false;
}

function openAiSettings() {
  window.dispatchEvent(new CustomEvent("zhiyu:open-ai-settings"));
}
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo regex">.*</span>
      <div>
        <div class="title-line"><h1>正则表达式调试器</h1><span>JAVASCRIPT REGEX</span></div>
        <p>实时匹配、捕获组查看与替换结果预览</p>
      </div>
    </div>
    <div class="header-actions"><button type="button" @click="aiOpen = true">✦ AI 生成</button></div>
  </header>

  <section class="regex-page">
    <div class="regex-builder">
      <div class="pattern-input"><span>/</span><input v-model="pattern" spellcheck="false" placeholder="输入正则表达式" /><span>/{{ flags }}</span></div>
      <select v-model="selectedPreset" @change="applyPreset">
        <option v-for="[value, label] in PRESETS" :key="label" :value="value">{{ label }}</option>
      </select>
    </div>

    <div class="flag-bar">
      <label><input v-model="flagGlobal" type="checkbox" /><code>g</code> 全局</label>
      <label><input v-model="flagIgnoreCase" type="checkbox" /><code>i</code> 忽略大小写</label>
      <label><input v-model="flagMultiline" type="checkbox" /><code>m</code> 多行</label>
      <label><input v-model="flagDotAll" type="checkbox" /><code>s</code> 点匹配换行</label>
      <span v-if="evaluation.error" class="regex-error">{{ evaluation.error }}</span>
      <span v-else class="match-count">{{ evaluation.matches.length }} 个匹配</span>
    </div>

    <div class="regex-grid">
      <article class="regex-panel">
        <div class="panel-head"><div><p>TEST TEXT</p><h2>测试文本</h2></div><span>{{ testText.length }} 字符</span></div>
        <textarea v-model="testText" spellcheck="false"></textarea>
      </article>
      <article class="regex-panel">
        <div class="panel-head"><div><p>MATCH PREVIEW</p><h2>匹配预览</h2></div></div>
        <div class="highlight-preview">
          <template v-if="previewSegments.length">
            <mark v-for="(segment, index) in previewSegments" :key="index" :class="{ plain: !segment.matched }">{{ segment.text }}</mark>
          </template>
          <span v-else class="empty-copy">没有匹配内容</span>
        </div>
      </article>
    </div>

    <article class="regex-panel">
      <div class="panel-head">
        <div><p>MATCHES</p><h2>匹配结果与捕获组</h2></div>
      </div>
      <div v-if="evaluation.matches.length === 0" class="regex-empty">输入表达式和测试文本后，匹配结果会显示在这里</div>
      <div v-else class="match-list">
        <div v-for="(match, index) in evaluation.matches" :key="`${match.index}-${index}`" class="match-row">
          <span>#{{ index + 1 }}</span><code>{{ match.value || "空匹配" }}</code><small>位置 {{ match.index }}</small>
          <div v-if="match.groups.length || Object.keys(match.namedGroups).length" class="group-list">
            <span v-for="(group, groupIndex) in match.groups" :key="groupIndex"><b>${{ groupIndex + 1 }}</b>{{ group || "空" }}</span>
            <span v-for="(group, name) in match.namedGroups" :key="name"><b>{{ name }}</b>{{ group || "空" }}</span>
          </div>
        </div>
      </div>
    </article>

    <article class="regex-panel replace-panel">
      <div class="panel-head"><div><p>REPLACE</p><h2>替换预览</h2></div><button type="button" @click="copy(replacementOutput)">复制结果</button></div>
      <div class="replace-input"><label>替换内容<input v-model="replacement" spellcheck="false" placeholder="$1、$2 或 $&" /></label></div>
      <pre>{{ replacementOutput }}</pre>
    </article>
  </section>
  <AiAssistDialog
    :open="aiOpen"
    title="AI 正则助手"
    :context="`当前表达式：${pattern}\nFlags：${flags}\n测试文本：\n${testText.slice(0, 8000)}`"
    :options="aiOptions"
    @close="aiOpen = false"
    @settings="openAiSettings"
    @apply="applyAiRegex"
  />
</template>

<style scoped>
.regex-page{display:grid;gap:14px;padding:24px 32px 36px}.regex-builder{display:grid;grid-template-columns:minmax(0,1fr) 210px;gap:10px}.pattern-input{display:flex;height:42px;align-items:center;border:1px solid var(--color-border-strong);background:var(--color-bg-elevated);font:11px "SFMono-Regular",Consolas,monospace}.pattern-input span{padding:0 12px;color:var(--color-accent)}.pattern-input input{min-width:0;flex:1;height:40px;padding:0;border:0;outline:0;background:transparent;font:inherit}.regex-builder select{padding:0 10px}.flag-bar{display:flex;min-height:42px;align-items:center;gap:18px;padding:8px 14px;border:1px solid var(--color-border);background:var(--color-bg-muted)}.flag-bar label{display:flex;align-items:center;gap:6px;color:var(--color-text-secondary);font-size:9px}.flag-bar code{color:var(--color-accent);font-weight:700}.flag-bar input{width:13px;height:13px}.match-count,.regex-error{margin-left:auto;font-size:9px}.match-count{color:var(--color-success-text)}.regex-error{color:var(--color-danger-text)}.regex-grid{display:grid;grid-template-columns:1fr 1fr;gap:14px}.regex-panel{overflow:hidden;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.panel-head{display:flex;min-height:58px;align-items:center;justify-content:space-between;gap:16px;padding:10px 14px;border-bottom:1px solid var(--color-border)}.panel-head p{margin:0 0 4px;color:var(--color-text-muted);font:8px "SFMono-Regular",Consolas,monospace;letter-spacing:.12em}.panel-head h2{margin:0;font-size:13px}.panel-head>span{color:var(--color-text-muted);font-size:8px}.panel-head button{min-height:30px;padding:0 10px;font-size:9px}.regex-panel textarea{display:block;width:100%;min-height:220px;padding:13px 14px;resize:vertical;border:0;outline:0;background:transparent;font:10px/1.7 "SFMono-Regular",Consolas,monospace}.highlight-preview{min-height:220px;max-height:360px;overflow:auto;padding:13px 14px;font:10px/1.7 "SFMono-Regular",Consolas,monospace;white-space:pre-wrap;word-break:break-word}.highlight-preview mark{padding:1px 2px;border-radius:2px;background:var(--color-warning-surface);color:var(--color-warning-text)}.highlight-preview mark.plain{padding:0;background:transparent;color:var(--color-text-primary)}.empty-copy,.regex-empty{color:var(--color-text-muted)}.regex-empty{padding:35px;text-align:center;font-size:9px}.match-list{max-height:280px;overflow:auto}.match-row{display:grid;grid-template-columns:42px minmax(160px,1fr) 70px minmax(220px,1.2fr);align-items:center;gap:10px;padding:10px 14px;border-bottom:1px solid var(--color-border);font-size:9px}.match-row:last-child{border-bottom:0}.match-row>span,.match-row>small{color:var(--color-text-muted)}.match-row>code{overflow:hidden;font-family:"SFMono-Regular",Consolas,monospace;text-overflow:ellipsis;white-space:nowrap}.group-list{display:flex;flex-wrap:wrap;gap:5px}.group-list span{display:flex;gap:5px;padding:3px 6px;border:1px solid var(--color-border);background:var(--color-bg-muted)}.group-list b{color:var(--color-accent)}.replace-input{padding:12px 14px;border-bottom:1px solid var(--color-border)}.replace-input label{display:grid;grid-template-columns:70px minmax(0,1fr);align-items:center;color:var(--color-text-secondary);font-size:9px}.replace-input input{height:32px;padding:0 9px;font:9px "SFMono-Regular",Consolas,monospace}.replace-panel pre{min-height:90px;max-height:240px;margin:0;overflow:auto;padding:13px 14px;font:10px/1.65 "SFMono-Regular",Consolas,monospace;white-space:pre-wrap}@media(max-width:1000px){.regex-grid{grid-template-columns:1fr}.match-row{grid-template-columns:38px minmax(0,1fr) 65px}.group-list{grid-column:2/-1}}
</style>
