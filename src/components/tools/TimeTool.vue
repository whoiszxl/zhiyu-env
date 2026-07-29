<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";

const TIME_ZONES = [
  ["local", "本地时区"],
  ["UTC", "UTC"],
  ["Asia/Shanghai", "亚洲 / 上海"],
  ["Asia/Tokyo", "亚洲 / 东京"],
  ["America/New_York", "美国 / 纽约"],
  ["Europe/London", "欧洲 / 伦敦"],
] as const;

const now = ref(Date.now());
const timestampInput = ref(String(Date.now()));
const timestampUnit = ref<"auto" | "seconds" | "milliseconds">("auto");
const selectedZone = ref("local");
const dateInput = ref(toDatetimeLocal(new Date()));
const resultDate = ref<Date | null>(new Date());
const error = ref("");
let timer: number | undefined;

const nowSeconds = computed(() => Math.floor(now.value / 1000));
const zoneLabel = computed(
  () => TIME_ZONES.find(([value]) => value === selectedZone.value)?.[1] ?? selectedZone.value,
);

function toDatetimeLocal(date: Date): string {
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 19);
}

function format(date: Date, zone = selectedZone.value): string {
  return new Intl.DateTimeFormat("zh-CN", {
    dateStyle: "full",
    timeStyle: "medium",
    hour12: false,
    timeZone: zone === "local" ? undefined : zone,
  }).format(date);
}

function parseTimestamp() {
  error.value = "";
  const raw = timestampInput.value.trim();
  if (!/^-?\d+(\.\d+)?$/.test(raw)) {
    resultDate.value = null;
    error.value = "请输入有效的 Unix 时间戳";
    return;
  }
  const value = Number(raw);
  const milliseconds =
    timestampUnit.value === "seconds" ||
    (timestampUnit.value === "auto" && Math.abs(value) < 100_000_000_000)
      ? value * 1000
      : value;
  const date = new Date(milliseconds);
  if (Number.isNaN(date.getTime())) {
    resultDate.value = null;
    error.value = "时间戳超出可表示范围";
    return;
  }
  resultDate.value = date;
}

function parseDate() {
  error.value = "";
  const date = new Date(dateInput.value);
  if (Number.isNaN(date.getTime())) {
    resultDate.value = null;
    error.value = "请输入有效的日期时间";
    return;
  }
  resultDate.value = date;
  timestampInput.value = String(date.getTime());
  timestampUnit.value = "milliseconds";
}

function useNow() {
  const date = new Date();
  timestampInput.value = String(date.getTime());
  dateInput.value = toDatetimeLocal(date);
  resultDate.value = date;
  error.value = "";
}

async function copy(value: string | number) {
  await navigator.clipboard.writeText(String(value));
}

onMounted(() => {
  timer = window.setInterval(() => (now.value = Date.now()), 1000);
});

onUnmounted(() => {
  if (timer) window.clearInterval(timer);
});
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo time">T</span>
      <div>
        <div class="title-line"><h1>时间与时间戳</h1><span>TIME TOOL</span></div>
        <p>Unix 时间戳、日期时间和常用时区快速互转</p>
      </div>
    </div>
    <div class="header-actions"><button type="button" @click="useNow">使用当前时间</button></div>
  </header>

  <div v-if="error" class="notice danger">
    <span>{{ error }}</span><button type="button" @click="error = ''">×</button>
  </div>

  <section class="time-page">
    <div class="current-time">
      <div><p>LOCAL TIME</p><strong>{{ format(new Date(now), "local") }}</strong></div>
      <button type="button" @click="copy(nowSeconds)"><span>UNIX 秒</span><code>{{ nowSeconds }}</code></button>
      <button type="button" @click="copy(now)"><span>UNIX 毫秒</span><code>{{ now }}</code></button>
    </div>

    <div class="time-grid">
      <article class="time-panel">
        <div class="panel-title"><p>TIMESTAMP → DATE</p><h2>时间戳转日期</h2></div>
        <div class="time-form">
          <label>Unix 时间戳<input v-model="timestampInput" spellcheck="false" @keyup.enter="parseTimestamp" /></label>
          <label>输入单位<select v-model="timestampUnit"><option value="auto">自动判断</option><option value="seconds">秒</option><option value="milliseconds">毫秒</option></select></label>
          <button class="primary" type="button" @click="parseTimestamp">转换</button>
        </div>
        <p class="time-hint">自动判断：少于 12 位按秒处理，其余按毫秒处理。</p>
      </article>

      <article class="time-panel">
        <div class="panel-title"><p>DATE → TIMESTAMP</p><h2>日期转时间戳</h2></div>
        <div class="time-form date-form">
          <label>本地日期时间<input v-model="dateInput" type="datetime-local" step="1" /></label>
          <button class="primary" type="button" @click="parseDate">转换</button>
        </div>
        <p class="time-hint">日期输入按当前系统时区解释。</p>
      </article>
    </div>

    <article class="time-panel result-panel">
      <div class="panel-title result-title">
        <div><p>CONVERSION RESULT</p><h2>转换结果</h2></div>
        <label>显示时区<select v-model="selectedZone"><option v-for="[value, label] in TIME_ZONES" :key="value" :value="value">{{ label }}</option></select></label>
      </div>
      <div v-if="resultDate" class="result-grid">
        <button type="button" @click="copy(Math.floor(resultDate.getTime() / 1000))"><span>Unix 秒</span><code>{{ Math.floor(resultDate.getTime() / 1000) }}</code></button>
        <button type="button" @click="copy(resultDate.getTime())"><span>Unix 毫秒</span><code>{{ resultDate.getTime() }}</code></button>
        <button type="button" @click="copy(resultDate.toISOString())"><span>ISO 8601</span><code>{{ resultDate.toISOString() }}</code></button>
        <button type="button" @click="copy(format(resultDate))"><span>{{ zoneLabel }}</span><code>{{ format(resultDate) }}</code></button>
      </div>
      <div v-else class="time-empty">输入时间戳或日期后进行转换</div>
    </article>

    <article class="time-panel">
      <div class="panel-title"><p>TIME ZONES</p><h2>同一时刻的时区对照</h2></div>
      <div class="zone-list">
        <div v-for="[value, label] in TIME_ZONES.slice(1)" :key="value">
          <span>{{ label }}</span><code>{{ format(resultDate ?? new Date(now), value) }}</code>
        </div>
      </div>
    </article>
  </section>
</template>

<style scoped>
.time-page{display:grid;gap:14px;padding:24px 32px 36px}.current-time{display:grid;grid-template-columns:minmax(280px,1fr) 220px 240px;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.current-time>div,.current-time>button{min-height:72px;padding:14px 18px;border:0;border-right:1px solid var(--color-border);background:transparent;text-align:left}.current-time>*:last-child{border-right:0}.current-time p,.panel-title p{margin:0 0 5px;color:var(--color-text-muted);font:8px/1.2 "SFMono-Regular",Consolas,monospace;letter-spacing:.12em}.current-time strong{font-size:15px}.current-time span,.result-grid span{display:block;margin-bottom:6px;color:var(--color-text-muted);font-size:8px}.current-time code,.result-grid code{font:10px/1.4 "SFMono-Regular",Consolas,monospace}.time-grid{display:grid;grid-template-columns:1fr 1fr;gap:14px}.time-panel{overflow:hidden;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.panel-title{padding:13px 16px;border-bottom:1px solid var(--color-border)}.panel-title h2{margin:0;font-size:14px}.time-form{display:grid;grid-template-columns:minmax(180px,1fr) 130px auto;align-items:end;gap:10px;padding:16px}.time-form.date-form{grid-template-columns:minmax(220px,1fr) auto}.time-form label,.result-title label{display:grid;gap:5px;color:var(--color-text-secondary);font-size:9px}.time-form input,.time-form select,.result-title select{height:34px;padding:0 10px;font-size:10px}.time-form input{font-family:"SFMono-Regular",Consolas,monospace}.time-form button{min-height:34px}.time-hint{margin:0;padding:0 16px 14px;color:var(--color-text-muted);font-size:8px}.result-title{display:flex;align-items:center;justify-content:space-between;gap:20px}.result-title select{min-width:150px}.result-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr))}.result-grid button{min-height:68px;padding:13px 16px;border:0;border-right:1px solid var(--color-border);border-bottom:1px solid var(--color-border);background:transparent;text-align:left}.result-grid button:nth-child(2n){border-right:0}.result-grid button:nth-last-child(-n+2){border-bottom:0}.result-grid code{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.zone-list>div{display:grid;grid-template-columns:150px minmax(0,1fr);gap:16px;padding:11px 16px;border-bottom:1px solid var(--color-border);font-size:9px}.zone-list>div:last-child{border-bottom:0}.zone-list span{color:var(--color-text-muted)}.zone-list code{font-family:"SFMono-Regular",Consolas,monospace}.time-empty{padding:38px;text-align:center;color:var(--color-text-muted);font-size:9px}@media(max-width:1050px){.current-time{grid-template-columns:1fr 1fr}.current-time>div{grid-column:1/-1;border-bottom:1px solid var(--color-border)}.time-grid{grid-template-columns:1fr}}
</style>
