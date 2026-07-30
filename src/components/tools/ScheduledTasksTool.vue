<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import {
  cancelScheduledTask,
  deleteScheduledTask,
  listScheduledTaskHistory,
  listScheduledTasks,
  runScheduledTask,
  saveScheduledTask,
  toggleScheduledTask,
} from "../../api/scheduledTasks";
import type {
  ScheduledTask,
  ScheduledTaskInput,
  ScheduledTaskRun,
} from "../../types";

type Filter = "all" | "active" | "paused";

const { t, locale } = useI18n();
const tasks = ref<ScheduledTask[]>([]);
const history = ref<ScheduledTaskRun[]>([]);
const selectedTaskId = ref<number | null>(null);
const expandedRunId = ref<number | null>(null);
const filter = ref<Filter>("all");
const loading = ref(true);
const saving = ref(false);
const runningIds = ref(new Set<number>());
const modalOpen = ref(false);
const error = ref("");
const toast = ref("");
let refreshTimer: number | undefined;
let toastTimer: number | undefined;

const form = reactive<ScheduledTaskInput>({
  id: null,
  name: "",
  scheduleKind: "cron",
  cronExpression: "0 9 * * MON-FRI",
  intervalMinutes: 60,
  command: "",
  workingDirectory: "",
  timeoutSeconds: 60,
  enabled: true,
});

const filteredTasks = computed(() => {
  if (filter.value === "active") return tasks.value.filter((task) => task.enabled);
  if (filter.value === "paused") return tasks.value.filter((task) => !task.enabled);
  return tasks.value;
});
const selectedTask = computed(
  () => tasks.value.find((task) => task.id === selectedTaskId.value) ?? null,
);
const enabledCount = computed(() => tasks.value.filter((task) => task.enabled).length);
const runningCount = computed(
  () => tasks.value.filter((task) => task.running || runningIds.value.has(task.id)).length,
);
const failedCount = computed(
  () => tasks.value.filter((task) => task.lastStatus && task.lastStatus !== "success").length,
);
const taskNames = computed(
  () => new Map(tasks.value.map((task) => [task.id, task.name])),
);

function notify(message: string) {
  toast.value = message;
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => (toast.value = ""), 2600);
}

async function refresh(silent = false) {
  if (!silent) loading.value = true;
  try {
    const [nextTasks, nextHistory] = await Promise.all([
      listScheduledTasks(),
      listScheduledTaskHistory(null, 80),
    ]);
    tasks.value = nextTasks;
    history.value = nextHistory;
    if (
      selectedTaskId.value !== null &&
      !nextTasks.some((task) => task.id === selectedTaskId.value)
    ) {
      selectedTaskId.value = null;
    }
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

function resetForm(task?: ScheduledTask) {
  Object.assign(form, task
    ? {
        id: task.id,
        name: task.name,
        scheduleKind: task.scheduleKind,
        cronExpression: task.cronExpression,
        intervalMinutes: task.intervalMinutes,
        command: task.command,
        workingDirectory: task.workingDirectory,
        timeoutSeconds: task.timeoutSeconds,
        enabled: task.enabled,
      }
    : {
        id: null,
        name: "",
        scheduleKind: "cron",
        cronExpression: "0 9 * * MON-FRI",
        intervalMinutes: 60,
        command: "",
        workingDirectory: "",
        timeoutSeconds: 60,
        enabled: true,
      });
  error.value = "";
  modalOpen.value = true;
}

async function chooseDirectory() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: t("scheduledTasks.form.workingDirectory"),
  });
  if (typeof selected === "string") form.workingDirectory = selected;
}

async function save() {
  if (saving.value) return;
  saving.value = true;
  error.value = "";
  try {
    const saved = await saveScheduledTask({ ...form });
    modalOpen.value = false;
    selectedTaskId.value = saved.id;
    await refresh(true);
    notify(t("scheduledTasks.saveSuccess"));
  } catch (cause) {
    error.value = String(cause);
  } finally {
    saving.value = false;
  }
}

async function toggle(task: ScheduledTask) {
  error.value = "";
  try {
    await toggleScheduledTask(task.id, !task.enabled);
    await refresh(true);
  } catch (cause) {
    error.value = String(cause);
  }
}

async function runNow(task: ScheduledTask) {
  if (runningIds.value.has(task.id)) return;
  runningIds.value = new Set([...runningIds.value, task.id]);
  error.value = "";
  try {
    const run = await runScheduledTask(task.id);
    expandedRunId.value = run.id;
    notify(
      run.status === "success"
        ? t("scheduledTasks.runSuccess")
        : t("scheduledTasks.runFailed"),
    );
    await refresh(true);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    const next = new Set(runningIds.value);
    next.delete(task.id);
    runningIds.value = next;
  }
}

async function remove(task: ScheduledTask) {
  if (!window.confirm(t("scheduledTasks.deleteConfirm"))) return;
  error.value = "";
  try {
    await deleteScheduledTask(task.id);
    await refresh(true);
  } catch (cause) {
    error.value = String(cause);
  }
}

function dateTime(value: number | null) {
  if (!value) return t("scheduledTasks.never");
  return new Intl.DateTimeFormat(locale.value, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(value);
}

function duration(value: number) {
  return value < 1000 ? `${value} ms` : `${(value / 1000).toFixed(1)} s`;
}

function scheduleLabel(task: ScheduledTask) {
  return task.scheduleKind === "cron"
    ? task.cronExpression
    : `${task.intervalMinutes} min`;
}

function statusLabel(status: ScheduledTaskRun["status"]) {
  if (status === "success") return t("scheduledTasks.success");
  if (status === "timed_out") return t("scheduledTasks.timedOut");
  if (status === "cancelled") return t("scheduledTasks.cancelled");
  return t("scheduledTasks.failedStatus");
}

async function cancelRun(task: ScheduledTask) {
  error.value = "";
  try {
    await cancelScheduledTask(task.id);
    notify(t("scheduledTasks.cancelRequested"));
  } catch (cause) {
    error.value = String(cause);
  }
}

onMounted(() => {
  void refresh();
  refreshTimer = window.setInterval(() => void refresh(true), 5000);
});
onBeforeUnmount(() => {
  window.clearInterval(refreshTimer);
  window.clearTimeout(toastTimer);
});
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo tasks">⌁</span>
      <div>
        <div class="title-line">
          <h1>{{ t("scheduledTasks.title") }}</h1>
          <span>LOCAL SCHEDULER</span>
        </div>
        <p>{{ t("scheduledTasks.subtitle") }}</p>
      </div>
    </div>
    <button class="primary" type="button" @click="resetForm()">
      ＋ {{ t("scheduledTasks.newTask") }}
    </button>
  </header>

  <main class="scheduled-page">
    <section class="scheduled-stats">
      <article><small>TOTAL</small><strong>{{ tasks.length }}</strong><span>{{ t("scheduledTasks.total") }}</span></article>
      <article><small>ACTIVE</small><strong>{{ enabledCount }}</strong><span>{{ t("scheduledTasks.enabled") }}</span></article>
      <article><small>RUNNING</small><strong>{{ runningCount }}</strong><span>{{ t("scheduledTasks.running") }}</span></article>
      <article :class="{ warning: failedCount > 0 }"><small>FAILED</small><strong>{{ failedCount }}</strong><span>{{ t("scheduledTasks.failed") }}</span></article>
    </section>

    <div v-if="error" class="scheduled-error">
      <span>{{ error }}</span><button type="button" @click="error = ''">×</button>
    </div>

    <section class="scheduled-panel">
      <header class="scheduled-panel-head">
        <div><small>SCHEDULES</small><h2>{{ t("scheduledTasks.tasks") }}</h2></div>
        <div class="scheduled-filters">
          <button v-for="item in (['all','active','paused'] as Filter[])" :key="item" :class="{ active: filter === item }" @click="filter = item">
            {{ t(`scheduledTasks.${item}`) }}
          </button>
        </div>
      </header>

      <div v-if="loading" class="scheduled-empty"><span class="spinner"></span></div>
      <div v-else-if="filteredTasks.length === 0" class="scheduled-empty">
        <i>⌁</i><strong>{{ t("scheduledTasks.emptyTitle") }}</strong>
        <p>{{ t("scheduledTasks.emptyHint") }}</p>
        <button class="primary" type="button" @click="resetForm()">{{ t("scheduledTasks.newTask") }}</button>
      </div>
      <div v-else class="scheduled-list">
        <article
          v-for="task in filteredTasks"
          :key="task.id"
          :class="{ selected: selectedTaskId === task.id, disabled: !task.enabled }"
          @click="selectedTaskId = task.id"
        >
          <button class="schedule-toggle" :aria-label="task.enabled ? t('scheduledTasks.pause') : t('scheduledTasks.enable')" @click.stop="toggle(task)">
            <i :class="{ on: task.enabled }"></i>
          </button>
          <div class="schedule-identity">
            <strong>{{ task.name }}</strong>
            <code>{{ scheduleLabel(task) }}</code>
          </div>
          <div class="schedule-command"><span>COMMAND</span><code>{{ task.command }}</code></div>
          <div class="schedule-time"><span>{{ t("scheduledTasks.nextRun") }}</span><strong>{{ task.enabled ? dateTime(task.nextRunAtMillis) : "—" }}</strong></div>
          <div class="schedule-time"><span>{{ t("scheduledTasks.lastRun") }}</span><strong>{{ dateTime(task.lastRunAtMillis) }}</strong></div>
          <span class="schedule-status" :class="task.lastStatus || 'idle'">{{ task.runCount }}</span>
          <div class="schedule-actions">
            <button @click.stop="task.running || runningIds.has(task.id) ? cancelRun(task) : runNow(task)">
              <span v-if="task.running || runningIds.has(task.id)" class="spinner"></span>
              {{ task.running || runningIds.has(task.id) ? t("scheduledTasks.cancelRun") : t("scheduledTasks.runNow") }}
            </button>
            <button @click.stop="resetForm(task)">{{ t("scheduledTasks.edit") }}</button>
            <button class="danger" @click.stop="remove(task)">×</button>
          </div>
        </article>
      </div>
    </section>

    <section class="scheduled-panel history-panel">
      <header class="scheduled-panel-head">
        <div><small>EXECUTION LOG</small><h2>{{ t("scheduledTasks.history") }}</h2></div>
        <span v-if="selectedTask">{{ selectedTask.name }}</span>
      </header>
      <div v-if="history.filter((run) => !selectedTaskId || run.taskId === selectedTaskId).length === 0" class="history-empty">
        {{ t("scheduledTasks.emptyHistory") }}
      </div>
      <div v-else class="run-history">
        <article
          v-for="run in history.filter((item) => !selectedTaskId || item.taskId === selectedTaskId)"
          :key="run.id"
          :class="{ expanded: expandedRunId === run.id }"
        >
          <button class="run-summary" @click="expandedRunId = expandedRunId === run.id ? null : run.id">
            <i :class="run.status"></i>
            <strong>{{ taskNames.get(run.taskId) || `#${run.taskId}` }}</strong>
            <span>{{ run.trigger === "manual" ? t("scheduledTasks.manual") : t("scheduledTasks.scheduled") }}</span>
            <time>{{ dateTime(run.startedAtMillis) }}</time>
            <code>{{ duration(run.durationMillis) }}</code>
            <em :class="run.status">{{ statusLabel(run.status) }}</em>
            <b>{{ expandedRunId === run.id ? "−" : "+" }}</b>
          </button>
          <div v-if="expandedRunId === run.id" class="run-output">
            <header><span>{{ t("scheduledTasks.output") }}</span><code>{{ t("scheduledTasks.exitCode") }}: {{ run.exitCode ?? "—" }}</code></header>
            <pre>{{ run.output || t("scheduledTasks.noOutput") }}</pre>
          </div>
        </article>
      </div>
    </section>
  </main>

  <div v-if="modalOpen" class="scheduled-modal-backdrop" @mousedown.self="modalOpen = false">
    <form class="scheduled-modal" @submit.prevent="save">
      <header>
        <div><small>LOCAL SCHEDULER</small><h2>{{ t(form.id ? "scheduledTasks.form.editTitle" : "scheduledTasks.form.createTitle") }}</h2></div>
        <button type="button" @click="modalOpen = false">×</button>
      </header>
      <div class="scheduled-form">
        <label class="wide">{{ t("scheduledTasks.form.name") }}<input v-model="form.name" required maxlength="80" :placeholder="t('scheduledTasks.form.namePlaceholder')" /></label>
        <fieldset class="wide">
          <legend>{{ t("scheduledTasks.form.schedule") }}</legend>
          <div class="schedule-kind">
            <button type="button" :class="{ active: form.scheduleKind === 'cron' }" @click="form.scheduleKind = 'cron'">{{ t("scheduledTasks.form.cron") }}</button>
            <button type="button" :class="{ active: form.scheduleKind === 'interval' }" @click="form.scheduleKind = 'interval'">{{ t("scheduledTasks.form.interval") }}</button>
          </div>
          <label v-if="form.scheduleKind === 'cron'">{{ t("scheduledTasks.form.cronExpression") }}<input v-model="form.cronExpression" required spellcheck="false" /><small>{{ t("scheduledTasks.form.cronHint") }}</small></label>
          <label v-else>{{ t("scheduledTasks.form.intervalMinutes") }}<input v-model.number="form.intervalMinutes" type="number" min="1" max="43200" required /><small>{{ t("scheduledTasks.form.intervalHint") }}</small></label>
        </fieldset>
        <label class="wide">{{ t("scheduledTasks.form.command") }}<textarea v-model="form.command" required rows="4" spellcheck="false" :placeholder="t('scheduledTasks.form.commandPlaceholder')"></textarea></label>
        <label class="wide">{{ t("scheduledTasks.form.workingDirectory") }}
          <div class="directory-input"><input v-model="form.workingDirectory" spellcheck="false" :placeholder="t('scheduledTasks.form.workingDirectoryHint')" /><button type="button" @click="chooseDirectory">{{ t("scheduledTasks.form.choose") }}</button></div>
        </label>
        <label>{{ t("scheduledTasks.form.timeout") }}<div class="number-unit"><input v-model.number="form.timeoutSeconds" type="number" min="1" max="3600" required /><span>{{ t("scheduledTasks.form.seconds") }}</span></div></label>
        <label class="enabled-check"><input v-model="form.enabled" type="checkbox" /><span>{{ t("scheduledTasks.form.enabled") }}</span></label>
        <p class="wide safety-note">◇ {{ t("scheduledTasks.form.safety") }}</p>
      </div>
      <footer>
        <button type="button" @click="modalOpen = false">{{ t("scheduledTasks.form.cancel") }}</button>
        <button class="primary" type="submit" :disabled="saving">{{ saving ? t("scheduledTasks.form.saving") : t("scheduledTasks.form.save") }}</button>
      </footer>
    </form>
  </div>

  <Transition name="scheduled-toast"><div v-if="toast" class="scheduled-toast">✓ {{ toast }}</div></Transition>
</template>

<style scoped>
.scheduled-page{display:grid;gap:13px;padding:22px 30px 42px}.scheduled-stats{display:grid;grid-template-columns:repeat(4,1fr);border:1px solid var(--color-border);background:var(--color-panel-translucent)}.scheduled-stats article{display:grid;grid-template-columns:1fr auto;gap:2px 10px;min-height:58px;align-content:center;padding:8px 14px;border-right:1px solid var(--color-border)}.scheduled-stats article:last-child{border-right:0}.scheduled-stats small,.scheduled-panel-head small,.scheduled-modal small{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace;letter-spacing:.13em}.scheduled-stats strong{grid-row:1/3;grid-column:2;font:22px "SFMono-Regular",monospace}.scheduled-stats span{color:var(--color-text-muted);font-size:8px}.scheduled-stats .warning strong{color:var(--color-warning-text)}.scheduled-error{display:flex;align-items:center;justify-content:space-between;padding:9px 12px;border:1px solid var(--color-danger-text);background:var(--color-danger-bg);color:var(--color-danger-text);font-size:8px}.scheduled-error button{border:0;background:transparent;color:inherit}.scheduled-panel{border:1px solid var(--color-border);background:var(--color-panel-translucent)}.scheduled-panel-head{display:flex;min-height:53px;align-items:center;justify-content:space-between;padding:7px 13px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.scheduled-panel-head h2{margin:4px 0 0;font-size:12px}.scheduled-panel-head>span{color:var(--color-text-muted);font-size:8px}.scheduled-filters{display:flex}.scheduled-filters button{height:28px;padding:0 11px;border:1px solid var(--color-border);border-right:0;background:transparent;color:var(--color-text-muted);font-size:8px}.scheduled-filters button:last-child{border-right:1px solid var(--color-border)}.scheduled-filters button.active{background:var(--color-bg-elevated);color:var(--color-accent);box-shadow:inset 0 -2px var(--color-accent)}.scheduled-empty{display:grid;min-height:190px;place-items:center;align-content:center;gap:7px;color:var(--color-text-muted)}.scheduled-empty i{display:grid;width:44px;height:44px;place-items:center;border:1px solid var(--color-border);border-radius:50%;font-style:normal;font-size:18px}.scheduled-empty strong{font-size:11px}.scheduled-empty p{margin:0 0 4px;font-size:8px}.scheduled-list article{display:grid;grid-template-columns:28px minmax(125px,.7fr) minmax(190px,1.3fr) 95px 95px 34px auto;align-items:center;gap:10px;min-height:64px;padding:0 12px;border-bottom:1px solid var(--color-border);transition:background .14s ease}.scheduled-list article:last-child{border-bottom:0}.scheduled-list article:hover,.scheduled-list article.selected{background:var(--color-bg-muted)}.scheduled-list article.selected{box-shadow:inset 3px 0 var(--color-accent)}.scheduled-list article.disabled{opacity:.62}.schedule-toggle{display:grid;width:27px;height:18px;place-items:center;border:0;background:transparent}.schedule-toggle i{position:relative;width:24px;height:13px;border:1px solid var(--color-border-strong);border-radius:9px;background:var(--color-bg-muted)}.schedule-toggle i:after{position:absolute;top:2px;left:2px;width:7px;height:7px;border-radius:50%;background:var(--color-text-muted);content:"";transition:transform .15s ease}.schedule-toggle i.on:after{transform:translateX(11px);background:var(--color-success-text)}.schedule-identity,.schedule-command,.schedule-time{display:grid;min-width:0;gap:4px}.schedule-identity strong{font-size:9px}.schedule-identity code,.schedule-command code{overflow:hidden;color:var(--color-text-muted);font:7px "SFMono-Regular",monospace;text-overflow:ellipsis;white-space:nowrap}.schedule-command span,.schedule-time span{color:var(--color-text-muted);font:6px "SFMono-Regular",monospace;letter-spacing:.08em}.schedule-time strong{font-size:7px;white-space:nowrap}.schedule-status{display:grid;width:25px;height:18px;place-items:center;border:1px solid var(--color-border);border-radius:9px;color:var(--color-text-muted);font:7px "SFMono-Regular",monospace}.schedule-status.success{border-color:var(--color-success-text);color:var(--color-success-text)}.schedule-status.failed,.schedule-status.timed_out{border-color:var(--color-danger-text);color:var(--color-danger-text)}.schedule-actions{display:flex;gap:5px}.schedule-actions button{height:27px;padding:0 8px;border:1px solid var(--color-border-strong);background:var(--color-bg-panel);font-size:7px;white-space:nowrap}.schedule-actions button.danger{width:27px;padding:0;color:var(--color-danger-text)}.history-empty{padding:38px;text-align:center;color:var(--color-text-muted);font-size:8px}.run-history article{border-bottom:1px solid var(--color-border)}.run-history article:last-child{border-bottom:0}.run-summary{display:grid;width:100%;grid-template-columns:12px minmax(140px,1fr) 55px 100px 55px 60px 14px;align-items:center;gap:10px;min-height:43px;padding:0 13px;border:0;background:transparent;text-align:left}.run-summary:hover{background:var(--color-bg-muted)}.run-summary i{width:6px;height:6px;border-radius:50%;background:var(--color-text-muted)}.run-summary i.success{background:var(--color-success-text)}.run-summary i.failed,.run-summary i.timed_out{background:var(--color-danger-text)}.run-summary strong{font-size:8px}.run-summary span,.run-summary time{color:var(--color-text-muted);font-size:7px}.run-summary code{font:7px "SFMono-Regular",monospace}.run-summary em{font-style:normal;font-size:7px}.run-summary em.success{color:var(--color-success-text)}.run-summary em.failed,.run-summary em.timed_out{color:var(--color-danger-text)}.run-summary b{text-align:center}.run-output{border-top:1px solid var(--color-border);background:#111713}.run-output header{display:flex;justify-content:space-between;padding:7px 12px;border-bottom:1px solid rgba(255,255,255,.08);color:#8f9d93;font-size:7px}.run-output pre{box-sizing:border-box;max-height:260px;margin:0;padding:12px;overflow:auto;color:#d7ded8;font:8px/1.65 "SFMono-Regular",Consolas,monospace;white-space:pre-wrap}.scheduled-modal-backdrop{position:fixed;z-index:1200;inset:0;display:grid;place-items:center;padding:35px;background:rgba(7,10,9,.68);backdrop-filter:blur(8px)}.scheduled-modal{width:min(680px,calc(100vw - 70px));max-height:calc(100vh - 70px);overflow:auto;border:1px solid var(--color-border-strong);background:var(--color-bg-panel);box-shadow:0 24px 70px rgba(0,0,0,.3)}.scheduled-modal>header{display:flex;min-height:70px;align-items:center;justify-content:space-between;padding:0 18px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.scheduled-modal h2{margin:5px 0 0;font-size:16px}.scheduled-modal>header button{width:30px;height:30px;border:1px solid var(--color-border);border-radius:50%;background:transparent}.scheduled-form{display:grid;grid-template-columns:1fr 1fr;gap:15px;padding:18px}.scheduled-form label{display:grid;gap:6px;color:var(--color-text-secondary);font-size:8px}.scheduled-form .wide{grid-column:1/-1}.scheduled-form input:not([type=checkbox]),.scheduled-form textarea{box-sizing:border-box;width:100%;border:1px solid var(--color-border-strong);background:var(--color-bg-elevated);color:var(--color-text-primary);font:9px "SFMono-Regular",monospace;outline:0}.scheduled-form input:not([type=checkbox]){height:35px;padding:0 10px}.scheduled-form textarea{min-height:82px;padding:10px;resize:vertical;line-height:1.6}.scheduled-form input:focus,.scheduled-form textarea:focus{border-color:var(--color-accent);box-shadow:inset 0 0 0 1px var(--color-accent)}.scheduled-form fieldset{margin:0;padding:12px;border:1px solid var(--color-border)}.scheduled-form legend{padding:0 5px;color:var(--color-text-secondary);font-size:8px}.schedule-kind{display:flex;margin-bottom:10px}.schedule-kind button{height:29px;padding:0 12px;border:1px solid var(--color-border);border-right:0;background:transparent;font-size:8px}.schedule-kind button:last-child{border-right:1px solid var(--color-border)}.schedule-kind button.active{background:var(--color-bg-muted);color:var(--color-accent);box-shadow:inset 0 -2px var(--color-accent)}.scheduled-form label small{color:var(--color-text-muted);font-size:7px}.directory-input,.number-unit{display:flex}.directory-input input,.number-unit input{min-width:0}.directory-input button,.number-unit span{display:grid;min-width:68px;place-items:center;border:1px solid var(--color-border-strong);border-left:0;background:var(--color-bg-muted);font-size:8px}.enabled-check{display:flex!important;align-items:center;align-self:end;gap:8px;height:35px}.enabled-check input{width:14px;height:14px}.safety-note{margin:0;padding:8px 10px;border:1px solid var(--color-border);background:var(--color-bg-muted);color:var(--color-text-muted);font-size:7px}.scheduled-modal>footer{display:flex;justify-content:flex-end;gap:8px;padding:12px 18px;border-top:1px solid var(--color-border);background:var(--color-bg-muted)}.scheduled-modal>footer button{height:32px;padding:0 15px}.scheduled-toast{position:fixed;z-index:1400;right:22px;bottom:22px;padding:10px 14px;border:1px solid var(--color-success-text);background:var(--color-bg-panel);color:var(--color-success-text);box-shadow:0 10px 30px rgba(0,0,0,.25);font-size:8px}.scheduled-toast-enter-active,.scheduled-toast-leave-active{transition:opacity .16s ease,transform .16s ease}.scheduled-toast-enter-from,.scheduled-toast-leave-to{opacity:0;transform:translateY(8px)}@media(max-width:1150px){.scheduled-list article{grid-template-columns:28px minmax(130px,.8fr) minmax(160px,1.2fr) 88px 34px auto}.schedule-time:nth-of-type(5){display:none}}@media(max-width:850px){.scheduled-stats{grid-template-columns:repeat(2,1fr)}.scheduled-stats article:nth-child(2){border-right:0}.scheduled-list article{grid-template-columns:28px 1fr auto}.schedule-command,.schedule-time,.schedule-status{display:none}.scheduled-form{grid-template-columns:1fr}.scheduled-form>*{grid-column:1!important}}
</style>
