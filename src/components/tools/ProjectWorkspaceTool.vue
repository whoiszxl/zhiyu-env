<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import {
  listServices,
  runServiceAction,
  runtimeOpenTerminal,
  runtimeOverview,
  runtimeProjectDelete,
  runtimeProjectManifestExport,
  runtimeProjectManifestImport,
  runtimeProjectsList,
  runtimeProjectSave,
} from "../../api/services";
import type {
  RuntimeKind,
  RuntimeOverview,
  RuntimeProject,
  ServiceInfo,
  ServiceKind,
} from "../../types";

const { t } = useI18n();
const projects = ref<RuntimeProject[]>([]);
const services = ref<ServiceInfo[]>([]);
const runtimes = ref<Record<RuntimeKind, RuntimeOverview | null>>({
  go: null,
  java: null,
  rust: null,
  python: null,
  node: null,
});
const selectedId = ref("");
const loading = ref(true);
const busy = ref("");
const error = ref("");
const notice = ref("");
const modalOpen = ref(false);
const draft = ref<RuntimeProject>(emptyProject());
const runtimeKinds: RuntimeKind[] = ["go", "java", "rust", "python", "node"];

const selectedProject = computed(
  () => projects.value.find((project) => project.id === selectedId.value) ?? null,
);
const linkedServices = computed(() =>
  (selectedProject.value?.services ?? [])
    .map((kind) => services.value.find((service) => service.kind === kind))
    .filter((service): service is ServiceInfo => Boolean(service)),
);
const configuredRuntimes = computed(() => {
  const project = selectedProject.value;
  if (!project) return [];
  return runtimeKinds
    .map((kind) => ({ kind, version: runtimeVersion(project, kind) }))
    .filter((item): item is { kind: RuntimeKind; version: string } =>
      Boolean(item.version),
    );
});
const runningCount = computed(
  () => linkedServices.value.filter((service) => service.status === "running").length,
);

function emptyProject(): RuntimeProject {
  return {
    id: "",
    name: "",
    path: "",
    description: "",
    services: [],
    goVersion: null,
    javaVersion: null,
    rustVersion: null,
    pythonVersion: null,
    nodeVersion: null,
    createdAtMillis: 0,
    updatedAtMillis: 0,
  };
}

function runtimeVersion(
  project: RuntimeProject,
  kind: RuntimeKind,
): string | null {
  const key = `${kind}Version` as
    | "goVersion"
    | "javaVersion"
    | "rustVersion"
    | "pythonVersion"
    | "nodeVersion";
  return project[key];
}

function setRuntimeVersion(
  project: RuntimeProject,
  kind: RuntimeKind,
  value: string | null,
) {
  const key = `${kind}Version` as
    | "goVersion"
    | "javaVersion"
    | "rustVersion"
    | "pythonVersion"
    | "nodeVersion";
  project[key] = value;
}

function runtimeName(kind: RuntimeKind) {
  return kind === "node" ? "Node.js" : kind[0].toUpperCase() + kind.slice(1);
}

async function load() {
  loading.value = true;
  try {
    const [projectList, serviceList, ...runtimeList] = await Promise.all([
      runtimeProjectsList(),
      listServices(),
      ...runtimeKinds.map((kind) => runtimeOverview(kind).catch(() => null)),
    ]);
    projects.value = projectList;
    services.value = serviceList;
    runtimeKinds.forEach((kind, index) => {
      runtimes.value[kind] = runtimeList[index];
    });
    if (!projects.value.some((project) => project.id === selectedId.value)) {
      selectedId.value = projects.value[0]?.id ?? "";
    }
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

async function createProject() {
  const path = await open({
    directory: true,
    multiple: false,
    title: t("workspace.chooseDirectory"),
  });
  if (!path) return;
  const name = path.split(/[\\/]/).filter(Boolean).at(-1) || path;
  draft.value = { ...emptyProject(), name, path };
  modalOpen.value = true;
}

function editProject() {
  if (!selectedProject.value) return;
  draft.value = {
    ...selectedProject.value,
    services: [...selectedProject.value.services],
  };
  modalOpen.value = true;
}

async function saveProject() {
  if (busy.value || !draft.value.name.trim() || !draft.value.path) return;
  busy.value = "save";
  try {
    projects.value = await runtimeProjectSave(draft.value);
    const saved = projects.value.find((item) => item.path === draft.value.path);
    selectedId.value = saved?.id ?? selectedId.value;
    modalOpen.value = false;
    notice.value = t("workspace.saved");
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = "";
  }
}

async function importManifest() {
  const path = await open({
    directory: true,
    multiple: false,
    title: t("workspace.importDirectory"),
  });
  if (!path) return;
  busy.value = "import";
  try {
    projects.value = await runtimeProjectManifestImport(path);
    selectedId.value =
      projects.value.find((project) => project.path === path)?.id ??
      projects.value.at(-1)?.id ??
      "";
    notice.value = t("workspace.imported");
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = "";
  }
}

async function exportManifest() {
  if (!selectedProject.value || busy.value) return;
  busy.value = "export";
  try {
    const path = await runtimeProjectManifestExport(selectedProject.value.id);
    notice.value = t("workspace.exported", { path });
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = "";
  }
}

async function removeProject() {
  const project = selectedProject.value;
  if (!project || !confirm(t("workspace.deleteConfirm"))) return;
  try {
    projects.value = await runtimeProjectDelete(project.id);
    selectedId.value = projects.value[0]?.id ?? "";
  } catch (cause) {
    error.value = String(cause);
  }
}

async function runServices(action: "start" | "stop") {
  if (!selectedProject.value || busy.value) return;
  const targets = linkedServices.value.filter((service) =>
    action === "start"
      ? service.status === "stopped"
      : service.status === "running",
  );
  if (!targets.length) return;
  busy.value = action;
  error.value = "";
  const failures: string[] = [];
  for (const service of targets) {
    try {
      await runServiceAction(action, service.kind);
    } catch (cause) {
      failures.push(`${service.name}: ${String(cause)}`);
    }
  }
  services.value = await listServices();
  busy.value = "";
  if (failures.length) {
    error.value = failures.join("\n");
  } else {
    notice.value =
      action === "start"
        ? t("workspace.servicesStarted", { count: targets.length })
        : t("workspace.servicesStopped", { count: targets.length });
  }
}

async function openRuntime(kind: RuntimeKind, version: string) {
  if (!selectedProject.value) return;
  try {
    await runtimeOpenTerminal(kind, selectedProject.value.path, version);
    notice.value = t("workspace.terminalOpened", {
      runtime: runtimeName(kind),
      version,
    });
  } catch (cause) {
    error.value = String(cause);
  }
}

function navigateService(kind: ServiceKind) {
  window.dispatchEvent(
    new CustomEvent("zhiyu:navigate", {
      detail: { type: "service", id: kind },
    }),
  );
}

function handleOpenProject(event: Event) {
  const id = (event as CustomEvent<{ id?: string }>).detail?.id;
  if (id) selectedId.value = id;
}

onMounted(() => {
  window.addEventListener("zhiyu:project-open", handleOpenProject);
  void load();
});
onUnmounted(() =>
  window.removeEventListener("zhiyu:project-open", handleOpenProject),
);
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo workspace-logo">W</span>
      <div>
        <div class="title-line">
          <h1>{{ t("workspace.title") }}</h1>
          <span>LOCAL PROJECTS</span>
        </div>
        <p>{{ t("workspace.subtitle") }}</p>
      </div>
    </div>
    <div class="header-actions">
      <button type="button" :disabled="Boolean(busy)" @click="importManifest">
        {{ t("workspace.importManifest") }}
      </button>
      <button class="primary" type="button" @click="createProject">
        ＋ {{ t("workspace.newProject") }}
      </button>
    </div>
  </header>

  <div v-if="notice" class="workspace-toast success">
    <span>{{ notice }}</span><button type="button" @click="notice = ''">×</button>
  </div>
  <div v-if="error" class="workspace-toast danger">
    <span>{{ error }}</span><button type="button" @click="error = ''">×</button>
  </div>

  <div v-if="loading" class="page-loading">{{ t("common.loading") }}…</div>
  <main v-else class="workspace-layout">
    <aside class="project-list">
      <div class="workspace-section-head">
        <div>
          <small>WORKSPACES</small>
          <h2>{{ t("workspace.projects") }}</h2>
        </div>
        <span>{{ projects.length }}</span>
      </div>
      <button
        v-for="project in projects"
        :key="project.id"
        type="button"
        class="project-item"
        :class="{ active: selectedId === project.id }"
        @click="selectedId = project.id"
      >
        <i>{{ project.name.slice(0, 1).toUpperCase() }}</i>
        <span>
          <strong>{{ project.name }}</strong>
          <small>{{ project.path }}</small>
        </span>
        <em>{{ project.services.length }}</em>
      </button>
      <div v-if="!projects.length" class="workspace-empty compact">
        <strong>{{ t("workspace.noProjects") }}</strong>
        <p>{{ t("workspace.noProjectsHint") }}</p>
        <button type="button" @click="createProject">
          {{ t("workspace.newProject") }}
        </button>
      </div>
    </aside>

    <section v-if="selectedProject" class="workspace-detail">
      <div class="project-hero">
        <div>
          <small>ACTIVE WORKSPACE</small>
          <h2>{{ selectedProject.name }}</h2>
          <p>{{ selectedProject.description || t("workspace.noDescription") }}</p>
          <code>{{ selectedProject.path }}</code>
        </div>
        <div class="project-actions">
          <button type="button" @click="editProject">{{ t("common.edit") }}</button>
          <button type="button" :disabled="Boolean(busy)" @click="exportManifest">
            {{ t("workspace.exportManifest") }}
          </button>
          <button class="danger" type="button" @click="removeProject">
            {{ t("common.delete") }}
          </button>
        </div>
      </div>

      <div class="workspace-metrics">
        <article>
          <small>{{ t("workspace.linkedServices") }}</small>
          <strong>{{ linkedServices.length }}</strong>
          <span>{{ runningCount }} {{ t("workspace.running") }}</span>
        </article>
        <article>
          <small>{{ t("workspace.runtimes") }}</small>
          <strong>{{ configuredRuntimes.length }}</strong>
          <span>{{ t("workspace.isolatedVersions") }}</span>
        </article>
        <article>
          <small>{{ t("workspace.manifest") }}</small>
          <strong>.zhiyu-env.json</strong>
          <span>{{ t("workspace.manifestHint") }}</span>
        </article>
      </div>

      <article class="workspace-panel">
        <div class="workspace-panel-head">
          <div><small>SERVICE STACK</small><h3>{{ t("workspace.services") }}</h3></div>
          <div>
            <button
              type="button"
              :disabled="!linkedServices.some((service) => service.status === 'running') || Boolean(busy)"
              @click="runServices('stop')"
            >
              {{ t("workspace.stopAll") }}
            </button>
            <button
              class="primary"
              type="button"
              :disabled="!linkedServices.some((service) => service.status === 'stopped') || Boolean(busy)"
              @click="runServices('start')"
            >
              {{ t("workspace.startAll") }}
            </button>
          </div>
        </div>
        <div v-if="linkedServices.length" class="dependency-grid">
          <button
            v-for="service in linkedServices"
            :key="service.kind"
            type="button"
            @click="navigateService(service.kind)"
          >
            <i :class="service.status"></i>
            <span><strong>{{ service.name }}</strong><small>v{{ service.version }} · {{ service.port }}</small></span>
            <em>{{ t(`workspace.state.${service.status}`) }}</em>
          </button>
        </div>
        <div v-else class="workspace-empty">
          <p>{{ t("workspace.noServices") }}</p>
          <button type="button" @click="editProject">{{ t("workspace.configure") }}</button>
        </div>
      </article>

      <article class="workspace-panel">
        <div class="workspace-panel-head">
          <div><small>RUNTIME TOOLCHAIN</small><h3>{{ t("workspace.runtimes") }}</h3></div>
        </div>
        <div v-if="configuredRuntimes.length" class="runtime-strip">
          <button
            v-for="runtime in configuredRuntimes"
            :key="runtime.kind"
            type="button"
            @click="openRuntime(runtime.kind, runtime.version)"
          >
            <i>{{ runtimeName(runtime.kind).slice(0, 2) }}</i>
            <span><strong>{{ runtimeName(runtime.kind) }}</strong><code>{{ runtime.version }}</code></span>
            <em>{{ t("workspace.openTerminal") }} ↗</em>
          </button>
        </div>
        <div v-else class="workspace-empty">
          <p>{{ t("workspace.noRuntimes") }}</p>
          <button type="button" @click="editProject">{{ t("workspace.configure") }}</button>
        </div>
      </article>
    </section>
    <section v-else class="workspace-empty main">
      <span>W</span>
      <strong>{{ t("workspace.selectProject") }}</strong>
      <p>{{ t("workspace.selectProjectHint") }}</p>
    </section>
  </main>

  <Teleport to="body">
    <div
      v-if="modalOpen"
      class="workspace-modal-backdrop"
      role="dialog"
      aria-modal="true"
      @click.self="modalOpen = false"
    >
      <form class="workspace-modal" @submit.prevent="saveProject">
        <div class="workspace-modal-head">
          <div><small>PROJECT WORKSPACE</small><h2>{{ t("workspace.projectSettings") }}</h2></div>
          <button type="button" @click="modalOpen = false">×</button>
        </div>
        <div class="workspace-form-grid">
          <label>
            {{ t("workspace.projectName") }}
            <input v-model="draft.name" maxlength="80" />
          </label>
          <label>
            {{ t("workspace.projectPath") }}
            <input v-model="draft.path" readonly />
          </label>
        </div>
        <label>
          {{ t("workspace.description") }}
          <textarea v-model="draft.description" rows="2"></textarea>
        </label>
        <fieldset>
          <legend>{{ t("workspace.services") }}</legend>
          <div class="workspace-check-grid">
            <label v-for="service in services" :key="service.kind">
              <input v-model="draft.services" type="checkbox" :value="service.kind" />
              <span><strong>{{ service.name }}</strong><small>v{{ service.version }}</small></span>
            </label>
          </div>
        </fieldset>
        <fieldset>
          <legend>{{ t("workspace.runtimes") }}</legend>
          <div class="workspace-runtime-grid">
            <label v-for="kind in runtimeKinds" :key="kind">
              <span>{{ runtimeName(kind) }}</span>
              <select
                :value="runtimeVersion(draft, kind) ?? ''"
                @change="setRuntimeVersion(draft, kind, ($event.target as HTMLSelectElement).value || null)"
              >
                <option value="">{{ t("runtime.notConfigured") }}</option>
                <option
                  v-for="version in runtimes[kind]?.versions.filter((item) => item.installed) ?? []"
                  :key="version.version"
                  :value="version.version"
                >
                  {{ version.version }}
                </option>
              </select>
            </label>
          </div>
        </fieldset>
        <div class="workspace-modal-actions">
          <button type="button" @click="modalOpen = false">{{ t("common.cancel") }}</button>
          <button class="primary" type="submit" :disabled="busy === 'save'">
            {{ t("common.save") }}
          </button>
        </div>
      </form>
    </div>
  </Teleport>
</template>

<style scoped>
.workspace-logo{background:#3d6f62}.workspace-toast{position:fixed;z-index:120;right:24px;bottom:24px;display:flex;max-width:520px;gap:14px;padding:12px 14px;border:1px solid var(--color-success-text);background:var(--color-panel);box-shadow:0 12px 40px rgba(0,0,0,.25);color:var(--color-success-text);font-size:9px;white-space:pre-wrap}.workspace-toast.danger{border-color:var(--color-danger-text);color:var(--color-danger-text)}.workspace-toast button{margin-left:auto;border:0;background:transparent;color:inherit}.workspace-layout{display:grid;grid-template-columns:270px minmax(0,1fr);min-height:calc(100vh - 148px);padding:24px 32px 40px;gap:14px}.project-list,.workspace-detail,.workspace-panel,.workspace-empty.main{min-width:0;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.project-list{overflow:hidden}.workspace-section-head,.workspace-panel-head{display:flex;min-height:58px;align-items:center;justify-content:space-between;gap:12px;padding:10px 14px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.workspace-section-head small,.workspace-panel-head small,.project-hero small{color:var(--color-text-muted);font:8px "SFMono-Regular",Consolas,monospace;letter-spacing:.12em}.workspace-section-head h2,.workspace-panel-head h3{margin:4px 0 0;font-size:14px}.workspace-section-head>span{display:grid;min-width:28px;height:24px;place-items:center;border:1px solid var(--color-border);font:9px "SFMono-Regular",monospace}.project-item{display:grid;width:100%;grid-template-columns:34px minmax(0,1fr) auto;align-items:center;gap:10px;padding:12px 14px;border:0;border-bottom:1px solid var(--color-border);background:transparent;color:var(--color-text-primary);text-align:left}.project-item:hover,.project-item.active{background:var(--color-panel-active)}.project-item.active{box-shadow:inset 3px 0 var(--color-accent)}.project-item>i{display:grid;width:30px;height:30px;place-items:center;border-radius:50%;background:var(--color-bg-muted);color:var(--color-accent);font-style:normal;font-weight:700}.project-item>span{display:grid;min-width:0;gap:4px}.project-item strong,.project-item small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.project-item strong{font-size:10px}.project-item small{color:var(--color-text-muted);font-size:8px}.project-item em{padding:3px 6px;border:1px solid var(--color-border);color:var(--color-text-muted);font:normal 8px "SFMono-Regular",monospace}.workspace-detail{display:grid;align-content:start;gap:14px;border:0;background:transparent}.project-hero{display:flex;min-height:112px;align-items:center;justify-content:space-between;gap:20px;padding:18px 20px;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.project-hero>div:first-child{display:grid;min-width:0;gap:5px}.project-hero h2{margin:0;font-size:22px}.project-hero p{margin:0;color:var(--color-text-secondary);font-size:9px}.project-hero code{overflow:hidden;color:var(--color-text-muted);font:8px "SFMono-Regular",monospace;text-overflow:ellipsis;white-space:nowrap}.project-actions,.workspace-panel-head>div:last-child{display:flex;gap:7px}.project-actions button,.workspace-panel button,.workspace-empty button{min-height:30px;padding:6px 10px;border:1px solid var(--color-border-strong);background:var(--color-bg-panel);color:var(--color-text-primary);font-size:9px}.project-actions button.primary,.workspace-panel button.primary{border-color:var(--color-control-primary);background:var(--color-control-primary);color:#fff}.project-actions button.danger{border-color:var(--color-danger-text);color:var(--color-danger-text)}.workspace-metrics{display:grid;grid-template-columns:repeat(3,minmax(0,1fr));border:1px solid var(--color-border);background:var(--color-panel-translucent)}.workspace-metrics article{display:grid;gap:5px;padding:12px 15px;border-right:1px solid var(--color-border)}.workspace-metrics article:last-child{border-right:0}.workspace-metrics small{color:var(--color-text-muted);font-size:8px}.workspace-metrics strong{font:16px "SFMono-Regular",Consolas,monospace}.workspace-metrics span{color:var(--color-text-muted);font-size:8px}.dependency-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr))}.dependency-grid>button{display:grid;grid-template-columns:9px minmax(0,1fr) auto;align-items:center;gap:10px;min-height:60px;border:0;border-right:1px solid var(--color-border);border-bottom:1px solid var(--color-border);background:transparent;text-align:left}.dependency-grid>button:hover,.runtime-strip>button:hover{background:var(--color-panel-active)}.dependency-grid i{width:7px;height:7px;border-radius:50%;background:var(--color-text-muted)}.dependency-grid i.running{background:var(--color-success-text);box-shadow:0 0 8px color-mix(in srgb,var(--color-success-text) 55%,transparent)}.dependency-grid i.crashed,.dependency-grid i.stale_pid{background:var(--color-danger-text)}.dependency-grid span,.runtime-strip span{display:grid;min-width:0;gap:3px}.dependency-grid strong,.runtime-strip strong{font-size:10px}.dependency-grid small,.dependency-grid em,.runtime-strip em{color:var(--color-text-muted);font:normal 8px "SFMono-Regular",monospace}.runtime-strip{display:grid;grid-template-columns:repeat(3,minmax(0,1fr))}.runtime-strip>button{display:grid;grid-template-columns:32px minmax(0,1fr) auto;align-items:center;gap:10px;padding:12px 14px;border:0;border-right:1px solid var(--color-border);border-bottom:1px solid var(--color-border);background:transparent;color:var(--color-text-primary);text-align:left}.runtime-strip>button>i{display:grid;width:30px;height:30px;place-items:center;border-radius:50%;background:var(--color-bg-muted);color:var(--color-accent);font:normal 9px "SFMono-Regular",monospace}.runtime-strip code{color:var(--color-text-muted);font-size:8px}.workspace-empty{display:grid;min-height:110px;place-items:center;align-content:center;gap:8px;padding:18px;color:var(--color-text-muted);text-align:center}.workspace-empty.compact{min-height:190px}.workspace-empty.main{min-height:420px}.workspace-empty.main>span{display:grid;width:54px;height:54px;place-items:center;border:1px solid var(--color-border);border-radius:50%;color:var(--color-accent);font-size:18px}.workspace-empty strong{color:var(--color-text-primary);font-size:11px}.workspace-empty p{max-width:360px;margin:0;font-size:9px;line-height:1.6}.workspace-modal-backdrop{position:fixed;z-index:160;inset:0;display:grid;place-items:center;padding:24px;background:rgba(0,0,0,.5);backdrop-filter:blur(6px)}.workspace-modal{display:grid;width:min(760px,calc(100vw - 48px));max-height:calc(100vh - 48px);gap:14px;overflow:auto;padding:22px;border:1px solid var(--color-border-strong);background:var(--color-panel);box-shadow:0 28px 90px rgba(0,0,0,.35)}.workspace-modal-head{display:flex;align-items:start;justify-content:space-between}.workspace-modal-head small{color:var(--color-text-muted);font:8px "SFMono-Regular",monospace;letter-spacing:.12em}.workspace-modal-head h2{margin:4px 0 0}.workspace-modal-head>button{width:30px;height:30px;border:1px solid var(--color-border);background:transparent;color:var(--color-text-primary)}.workspace-modal>label,.workspace-form-grid label{display:grid;gap:6px;color:var(--color-text-secondary);font-size:9px}.workspace-modal input,.workspace-modal select,.workspace-modal textarea{box-sizing:border-box;width:100%;min-width:0;border:1px solid var(--color-border-strong);background:var(--color-input-bg);color:var(--color-text-primary);font-size:9px}.workspace-modal input,.workspace-modal select{height:34px;padding:0 10px}.workspace-modal textarea{padding:9px 10px;resize:vertical}.workspace-form-grid{display:grid;grid-template-columns:minmax(0,.8fr) minmax(0,1.2fr);gap:12px}.workspace-modal fieldset{margin:0;padding:0;border:1px solid var(--color-border)}.workspace-modal legend{margin-left:10px;padding:0 6px;color:var(--color-text-muted);font:8px "SFMono-Regular",monospace;letter-spacing:.1em}.workspace-check-grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr))}.workspace-check-grid label{display:flex;align-items:center;gap:8px;padding:9px 10px;border-right:1px solid var(--color-border);border-bottom:1px solid var(--color-border);font-size:9px}.workspace-check-grid input{width:14px;height:14px}.workspace-check-grid span{display:grid;gap:2px}.workspace-check-grid small{color:var(--color-text-muted);font-size:7px}.workspace-runtime-grid{display:grid;grid-template-columns:repeat(5,minmax(0,1fr));padding:10px;gap:8px}.workspace-runtime-grid label{display:grid;gap:5px;color:var(--color-text-secondary);font-size:8px}.workspace-modal-actions{display:flex;justify-content:flex-end;gap:8px}.workspace-modal-actions button{min-height:32px;padding:7px 14px;border:1px solid var(--color-border-strong);background:var(--color-bg-panel);color:var(--color-text-primary);font-size:9px}.workspace-modal-actions button.primary{border-color:var(--color-control-primary);background:var(--color-control-primary);color:#fff}@media(max-width:1100px){.workspace-layout{grid-template-columns:220px minmax(0,1fr)}.dependency-grid,.runtime-strip{grid-template-columns:repeat(2,minmax(0,1fr))}.workspace-check-grid{grid-template-columns:repeat(3,minmax(0,1fr))}.workspace-runtime-grid{grid-template-columns:repeat(3,minmax(0,1fr))}}@media(max-width:820px){.workspace-layout{grid-template-columns:1fr}.project-list{max-height:260px;overflow:auto}.workspace-metrics{grid-template-columns:1fr}.workspace-metrics article{border-right:0;border-bottom:1px solid var(--color-border)}}
</style>
