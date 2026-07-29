<script setup lang="ts">
import { computed, ref } from "vue";

const PRESETS = [
  ["*/5 * * * *", "每 5 分钟"],
  ["0 * * * *", "每小时整点"],
  ["0 9 * * 1-5", "工作日 09:00"],
  ["0 0 * * *", "每天 00:00"],
  ["0 3 * * 0", "每周日 03:00"],
  ["0 0 1 * *", "每月 1 日"],
] as const;

const MONTH_NAMES: Record<string, number> = {
  JAN: 1, FEB: 2, MAR: 3, APR: 4, MAY: 5, JUN: 6,
  JUL: 7, AUG: 8, SEP: 9, OCT: 10, NOV: 11, DEC: 12,
};
const WEEKDAY_NAMES: Record<string, number> = {
  SUN: 0, MON: 1, TUE: 2, WED: 3, THU: 4, FRI: 5, SAT: 6,
};

const expression = ref("*/5 * * * *");
const selectedPreset = ref("*/5 * * * *");
const error = ref("");
const nextRuns = ref<Date[]>([]);

interface ParsedCron {
  minutes: Set<number>;
  hours: Set<number>;
  days: Set<number>;
  months: Set<number>;
  weekdays: Set<number>;
  dayWildcard: boolean;
  weekdayWildcard: boolean;
}

const explanation = computed(() => explainCron(expression.value));

function valueOf(raw: string, names?: Record<string, number>): number {
  const upper = raw.toUpperCase();
  if (names && upper in names) return names[upper];
  const value = Number(raw);
  if (!Number.isInteger(value)) throw new Error(`无法识别“${raw}”`);
  return value;
}

function parseField(
  source: string,
  minimum: number,
  maximum: number,
  names?: Record<string, number>,
  normalize?: (value: number) => number,
): Set<number> {
  const values = new Set<number>();
  for (const part of source.split(",")) {
    const [rangeSource, stepSource] = part.split("/");
    const step = stepSource === undefined ? 1 : Number(stepSource);
    if (!Number.isInteger(step) || step <= 0) throw new Error(`步长无效：“${part}”`);
    let start = minimum;
    let end = maximum;
    if (rangeSource !== "*") {
      if (rangeSource.includes("-")) {
        const [left, right] = rangeSource.split("-");
        start = valueOf(left, names);
        end = valueOf(right, names);
      } else {
        start = valueOf(rangeSource, names);
        end = stepSource === undefined ? start : maximum;
      }
    }
    if (start < minimum || start > maximum || end < minimum || end > maximum || start > end) {
      throw new Error(`取值超出 ${minimum}-${maximum}：“${part}”`);
    }
    for (let value = start; value <= end; value += step) {
      values.add(normalize ? normalize(value) : value);
    }
  }
  return values;
}

function parseCron(source: string): ParsedCron {
  const fields = source.trim().split(/\s+/);
  if (fields.length !== 5) throw new Error("请输入标准 5 段 Cron：分 时 日 月 星期");
  return {
    minutes: parseField(fields[0], 0, 59),
    hours: parseField(fields[1], 0, 23),
    days: parseField(fields[2], 1, 31),
    months: parseField(fields[3], 1, 12, MONTH_NAMES),
    weekdays: parseField(fields[4], 0, 7, WEEKDAY_NAMES, (value) => value === 7 ? 0 : value),
    dayWildcard: fields[2] === "*",
    weekdayWildcard: fields[4] === "*",
  };
}

function matches(cron: ParsedCron, date: Date): boolean {
  if (!cron.minutes.has(date.getMinutes())) return false;
  if (!cron.hours.has(date.getHours())) return false;
  if (!cron.months.has(date.getMonth() + 1)) return false;
  const dayMatches = cron.days.has(date.getDate());
  const weekdayMatches = cron.weekdays.has(date.getDay());
  if (cron.dayWildcard && cron.weekdayWildcard) return true;
  if (cron.dayWildcard) return weekdayMatches;
  if (cron.weekdayWildcard) return dayMatches;
  return dayMatches || weekdayMatches;
}

function calculate() {
  error.value = "";
  nextRuns.value = [];
  try {
    const cron = parseCron(expression.value);
    const cursor = new Date();
    cursor.setSeconds(0, 0);
    cursor.setMinutes(cursor.getMinutes() + 1);
    const maximumChecks = 366 * 24 * 60 * 2;
    for (let checked = 0; checked < maximumChecks && nextRuns.value.length < 10; checked++) {
      if (matches(cron, cursor)) nextRuns.value.push(new Date(cursor));
      cursor.setMinutes(cursor.getMinutes() + 1);
    }
    if (nextRuns.value.length === 0) error.value = "未来两年内没有匹配的运行时间";
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  }
}

function usePreset() {
  expression.value = selectedPreset.value;
  calculate();
}

function explainCron(source: string): string {
  const fields = source.trim().split(/\s+/);
  if (fields.length !== 5) return "等待输入合法的 5 段 Cron 表达式";
  const [minute, hour, day, month, weekday] = fields;
  if (source === "*/5 * * * *") return "每 5 分钟执行一次";
  if (source === "0 * * * *") return "每小时整点执行";
  if (source === "0 0 * * *") return "每天 00:00 执行";
  if (source === "0 9 * * 1-5") return "每周一至周五 09:00 执行";
  const time =
    /^\d+$/.test(minute) && /^\d+$/.test(hour)
      ? `${hour.padStart(2, "0")}:${minute.padStart(2, "0")}`
      : `分钟 ${minute}、小时 ${hour}`;
  return `${time}；日期 ${day}；月份 ${month}；星期 ${weekday}`;
}

function format(date: Date) {
  return new Intl.DateTimeFormat("zh-CN", {
    dateStyle: "full",
    timeStyle: "medium",
    hour12: false,
  }).format(date);
}

async function copy() {
  await navigator.clipboard.writeText(expression.value);
}

calculate();
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo cron">C</span>
      <div>
        <div class="title-line"><h1>Cron 表达式工具</h1><span>5-FIELD CRON</span></div>
        <p>校验 Cron 表达式并计算未来运行时间</p>
      </div>
    </div>
    <div class="header-actions"><button type="button" @click="copy">复制表达式</button></div>
  </header>

  <div v-if="error" class="notice danger"><span>{{ error }}</span><button type="button" @click="error = ''">×</button></div>

  <section class="cron-page">
    <div class="cron-input-row">
      <input v-model="expression" spellcheck="false" placeholder="*/5 * * * *" @keyup.enter="calculate" />
      <button class="primary" type="button" @click="calculate">解析并计算</button>
    </div>
    <div class="field-labels">
      <span>分钟<br /><code>0-59</code></span><span>小时<br /><code>0-23</code></span><span>日期<br /><code>1-31</code></span><span>月份<br /><code>1-12</code></span><span>星期<br /><code>0-7</code></span>
    </div>

    <div class="cron-grid">
      <article class="cron-panel">
        <div class="panel-head"><div><p>PRESETS</p><h2>常用表达式</h2></div></div>
        <div class="preset-list">
          <button v-for="[value, label] in PRESETS" :key="value" type="button" :class="{ active: expression === value }" @click="selectedPreset = value; usePreset()"><span>{{ label }}</span><code>{{ value }}</code></button>
        </div>
      </article>

      <article class="cron-panel">
        <div class="panel-head"><div><p>EXPLANATION</p><h2>表达式说明</h2></div></div>
        <div class="cron-explanation"><strong>{{ explanation }}</strong><p>当前按照本机时区计算。日期和星期同时受限时，遵循常见 Linux Cron 语义：任一条件满足即可。</p></div>
        <div class="syntax-list">
          <span><code>*</code>任意值</span><span><code>*/5</code>每 5 个单位</span><span><code>1-5</code>范围</span><span><code>1,3,5</code>多个值</span>
        </div>
      </article>
    </div>

    <article class="cron-panel">
      <div class="panel-head"><div><p>NEXT RUNS</p><h2>未来 10 次运行时间</h2></div><span>{{ nextRuns.length }} 条</span></div>
      <div v-if="nextRuns.length" class="run-list">
        <div v-for="(date, index) in nextRuns" :key="date.getTime()"><span>#{{ index + 1 }}</span><strong>{{ format(date) }}</strong><code>{{ date.toISOString() }}</code></div>
      </div>
      <div v-else class="cron-empty">输入表达式后点击“解析并计算”</div>
    </article>

    <p class="cron-note">当前支持 Linux 常用 5 段语法以及列表、范围、步长和英文月份/星期缩写；不支持 Quartz 的秒、年份、<code>?</code>、<code>L</code>、<code>W</code>。</p>
  </section>
</template>

<style scoped>
.cron-page{display:grid;gap:14px;padding:24px 32px 36px}.cron-input-row{display:grid;grid-template-columns:minmax(0,1fr) 130px;overflow:hidden;border:1px solid var(--color-border-strong);background:var(--color-bg-elevated)}.cron-input-row input{height:48px;padding:0 18px;border:0;outline:0;background:transparent;font:15px "SFMono-Regular",Consolas,monospace;letter-spacing:.08em}.cron-input-row button{border:0;border-radius:0}.field-labels{display:grid;grid-template-columns:repeat(5,1fr);padding:10px 16px;border:1px solid var(--color-border);background:var(--color-bg-muted);text-align:center}.field-labels span{border-right:1px solid var(--color-border);color:var(--color-text-secondary);font-size:9px}.field-labels span:last-child{border-right:0}.field-labels code{color:var(--color-text-muted);font-size:8px}.cron-grid{display:grid;grid-template-columns:340px minmax(0,1fr);gap:14px}.cron-panel{overflow:hidden;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.panel-head{display:flex;min-height:58px;align-items:center;justify-content:space-between;padding:10px 14px;border-bottom:1px solid var(--color-border)}.panel-head p{margin:0 0 4px;color:var(--color-text-muted);font:8px "SFMono-Regular",Consolas,monospace;letter-spacing:.12em}.panel-head h2{margin:0;font-size:13px}.panel-head>span{color:var(--color-text-muted);font-size:8px}.preset-list{display:grid}.preset-list button{display:flex;min-height:47px;align-items:center;justify-content:space-between;padding:9px 13px;border:0;border-bottom:1px solid var(--color-border);background:transparent;text-align:left}.preset-list button:last-child{border-bottom:0}.preset-list button:hover,.preset-list button.active{background:var(--color-bg-muted)}.preset-list button.active{box-shadow:inset 3px 0 var(--color-accent)}.preset-list span{font-size:9px}.preset-list code{color:var(--color-text-muted);font:9px "SFMono-Regular",Consolas,monospace}.cron-explanation{min-height:125px;padding:18px}.cron-explanation strong{font-size:16px}.cron-explanation p{margin:12px 0 0;color:var(--color-text-muted);font-size:9px;line-height:1.7}.syntax-list{display:grid;grid-template-columns:repeat(2,1fr);border-top:1px solid var(--color-border)}.syntax-list span{padding:10px 14px;border-right:1px solid var(--color-border);border-bottom:1px solid var(--color-border);color:var(--color-text-secondary);font-size:9px}.syntax-list span:nth-child(2n){border-right:0}.syntax-list span:nth-last-child(-n+2){border-bottom:0}.syntax-list code{margin-right:8px;color:var(--color-accent)}.run-list>div{display:grid;grid-template-columns:42px minmax(220px,.8fr) minmax(220px,1.2fr);gap:14px;padding:10px 14px;border-bottom:1px solid var(--color-border);font-size:9px}.run-list>div:last-child{border-bottom:0}.run-list span{color:var(--color-text-muted)}.run-list code{color:var(--color-text-secondary);font-family:"SFMono-Regular",Consolas,monospace}.cron-empty{padding:36px;text-align:center;color:var(--color-text-muted);font-size:9px}.cron-note{margin:0;color:var(--color-text-muted);font-size:8px;line-height:1.7}.cron-note code{color:var(--color-warning-text)}@media(max-width:1000px){.cron-grid{grid-template-columns:1fr}.run-list>div{grid-template-columns:38px minmax(0,1fr)}.run-list code{grid-column:2}}
</style>
