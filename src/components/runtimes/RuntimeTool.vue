<script setup lang="ts">
import { computed, inject, onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useI18n } from "vue-i18n";
import {
  runtimeDiagnose,
  runtimeInstall,
  runtimeOpenTerminal,
  runtimeOverview,
  runtimeProjectDelete,
  runtimeProjectsList,
  runtimeProjectSave,
  runtimeSelect,
  runtimeSetGoProxy,
  runtimeUninstall,
} from "../../api/services";
import type {
  RuntimeDiagnostic,
  RuntimeKind,
  RuntimeOverview,
  RuntimeProject,
  RuntimeVersionInfo,
} from "../../types";
import { INSTALL_TASK_KEY } from "../../tools/types";
import { formatBytes } from "../../utils/format";

const props = defineProps<{ kind: RuntimeKind }>();
const { t } = useI18n();
const installTasks = inject(INSTALL_TASK_KEY);
const overview = ref<RuntimeOverview | null>(null);
const projects = ref<RuntimeProject[]>([]);
const activeTab = ref<"overview" | "versions" | "projects" | "environment">("overview");
const loading = ref(true);
const busyVersion = ref("");
const versionTarget = ref("");
const error = ref("");
const notice = ref("");
const diagnostic = ref<RuntimeDiagnostic | null>(null);
const diagnosing = ref(false);
const projectModalOpen = ref(false);
const projectDraft = ref<RuntimeProject>(emptyProject());
const savingProject = ref(false);
const goProxyDraft = ref("");

const runtimeMeta: Record<RuntimeKind, { name: string; icon: string }> = {
  go: { name: "Go", icon: "G" },
  java: { name: "Java", icon: "J" },
  rust: { name: "Rust", icon: "R" },
  python: { name: "Python", icon: "Py" },
  node: { name: "Node.js", icon: "N" },
};
const runtimeName = computed(() => runtimeMeta[props.kind].name);
const icon = computed(() => runtimeMeta[props.kind].icon);
const selectedVersion = computed(() =>
  overview.value?.versions.find((version) => version.selected),
);
const installedVersions = computed(
  () => overview.value?.versions.filter((version) => version.installed) ?? [],
);
const selectedTargetVersion = computed(
  () =>
    overview.value?.versions.find(
      (version) => version.version === versionTarget.value,
    ) ?? null,
);
const projectRuntimeVersion = computed<string | null>({
  get: () => projectVersion(projectDraft.value),
  set: (version) => setProjectVersion(projectDraft.value, version),
});

function supportLabel(version: RuntimeVersionInfo): string {
  const key = `runtime.support.${props.kind}${version.series.replace(".", "")}`;
  const translated = t(key);
  return translated === key ? version.supportLabel : translated;
}

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

function projectVersion(project: RuntimeProject): string | null {
  switch (props.kind) {
    case "go":
      return project.goVersion;
    case "java":
      return project.javaVersion;
    case "rust":
      return project.rustVersion;
    case "python":
      return project.pythonVersion;
    case "node":
      return project.nodeVersion;
  }
}

function setProjectVersion(
  project: RuntimeProject,
  version: string | null,
): void {
  switch (props.kind) {
    case "go":
      project.goVersion = version;
      break;
    case "java":
      project.javaVersion = version;
      break;
    case "rust":
      project.rustVersion = version;
      break;
    case "python":
      project.pythonVersion = version;
      break;
    case "node":
      project.nodeVersion = version;
      break;
  }
}

async function load() {
  loading.value = true;
  try {
    [overview.value, projects.value] = await Promise.all([
      runtimeOverview(props.kind),
      runtimeProjectsList(),
    ]);
    goProxyDraft.value = overview.value.goProxy;
    if (
      !versionTarget.value ||
      !overview.value.versions.some(
        (version) => version.version === versionTarget.value,
      )
    ) {
      versionTarget.value =
        overview.value.selectedVersion ??
        overview.value.versions.find((version) => version.recommended)
          ?.version ??
        overview.value.versions[0]?.version ??
        "";
    }
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

async function install(version: RuntimeVersionInfo) {
  if (busyVersion.value || !installTasks) return;
  busyVersion.value = version.version;
  const operationId = installTasks.start(
    `runtime-${props.kind}`,
    t("runtime.installTitle", {
      runtime: runtimeName.value,
      version: version.version,
    }),
  );
  try {
    overview.value = await runtimeInstall(
      props.kind,
      version.version,
      operationId,
    );
    installTasks.succeed(operationId);
    notice.value = t("runtime.installed", { version: version.version });
    error.value = "";
  } catch (cause) {
    installTasks.fail(operationId, cause);
    error.value = String(cause);
  } finally {
    busyVersion.value = "";
  }
}

async function selectVersion(version: RuntimeVersionInfo) {
  if (busyVersion.value) return;
  busyVersion.value = version.version;
  try {
    overview.value = await runtimeSelect(props.kind, version.version);
    notice.value = t("runtime.selected", { version: version.version });
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busyVersion.value = "";
  }
}

async function activateTargetVersion() {
  const version = selectedTargetVersion.value;
  if (!version || busyVersion.value || version.selected) return;
  error.value = "";
  if (!version.installed) {
    await install(version);
    if (error.value) return;
  }
  if (overview.value?.selectedVersion !== version.version) {
    await selectVersion(version);
  }
}

async function uninstall(version: RuntimeVersionInfo) {
  if (
    busyVersion.value ||
    !confirm(t("runtime.uninstallConfirm", { version: version.version }))
  ) {
    return;
  }
  busyVersion.value = version.version;
  try {
    const wasSelected = version.selected;
    overview.value = await runtimeUninstall(props.kind, version.version);
    if (wasSelected && overview.value.selectedVersion) {
      versionTarget.value = overview.value.selectedVersion;
    }
    notice.value = t("runtime.uninstalled", { version: version.version });
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busyVersion.value = "";
  }
}

async function diagnose() {
  if (!overview.value?.selectedVersion || diagnosing.value) return;
  diagnosing.value = true;
  try {
    diagnostic.value = await runtimeDiagnose(props.kind);
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    diagnosing.value = false;
  }
}

async function openTerminal(projectPath?: string, version?: string | null) {
  try {
    await runtimeOpenTerminal(props.kind, projectPath, version ?? undefined);
    notice.value = t("runtime.terminalOpened");
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  }
}

async function copyEnvironment() {
  if (!overview.value?.environment.length) return;
  const exports = overview.value.environment
    .map((item) => {
      if (item.key === "PATH") {
        const prefix = item.value.replace(/:\$PATH$/, "");
        return `export PATH='${prefix.replaceAll("'", "'\"'\"'")}':"$PATH"`;
      }
      return `export ${item.key}='${item.value.replaceAll("'", "'\"'\"'")}'`;
    })
    .join("\n");
  await navigator.clipboard.writeText(exports);
  notice.value = t("runtime.environmentCopied");
}

async function saveGoProxy() {
  if (props.kind !== "go" || !goProxyDraft.value.trim()) return;
  try {
    overview.value = await runtimeSetGoProxy(goProxyDraft.value);
    notice.value = t("runtime.proxySaved");
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  }
}

async function addProject() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: t("runtime.chooseProject"),
  });
  if (!selected) return;
  const name = selected.split("/").filter(Boolean).at(-1) || selected;
  projectDraft.value = { ...emptyProject(), name, path: selected };
  setProjectVersion(
    projectDraft.value,
    overview.value?.selectedVersion ?? null,
  );
  projectModalOpen.value = true;
}

function editProject(project: RuntimeProject) {
  projectDraft.value = { ...project };
  projectModalOpen.value = true;
}

async function saveProject() {
  if (
    savingProject.value ||
    !projectDraft.value.name.trim() ||
    !projectDraft.value.path
  ) {
    return;
  }
  savingProject.value = true;
  try {
    projects.value = await runtimeProjectSave(projectDraft.value);
    projectModalOpen.value = false;
    notice.value = t("runtime.projectSaved");
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    savingProject.value = false;
  }
}

async function deleteProject(project: RuntimeProject) {
  if (!confirm(t("runtime.deleteProjectConfirm"))) return;
  try {
    projects.value = await runtimeProjectDelete(project.id);
  } catch (cause) {
    error.value = String(cause);
  }
}

onMounted(load);
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo runtime-logo" :class="kind">{{ icon }}</span>
      <div>
        <div class="title-line">
          <h1>{{ runtimeName }} {{ t("runtime.environment") }}</h1>
          <span>LOCAL SDK</span>
        </div>
        <p>{{ t(`runtime.${kind}.subtitle`) }}</p>
      </div>
    </div>
    <div class="header-actions">
      <button
        type="button"
        :disabled="!overview?.selectedVersion"
        @click="diagnose"
      >
        <span v-if="diagnosing" class="spinner"></span>
        {{ t("runtime.diagnose") }}
      </button>
      <button
        class="primary"
        type="button"
        :disabled="!overview?.selectedVersion"
        @click="openTerminal()"
      >
        {{ t("runtime.openTerminal") }}
      </button>
    </div>
  </header>

  <div v-if="notice" class="notice runtime-notice">
    <span>{{ notice }}</span><button type="button" @click="notice = ''">×</button>
  </div>
  <div v-if="error" class="notice danger runtime-notice">
    <span>{{ error }}</span><button type="button" @click="error = ''">×</button>
  </div>

  <nav class="detail-tabs runtime-tabs">
    <button
      v-for="tab in (['overview', 'versions', 'projects', 'environment'] as const)"
      :key="tab"
      type="button"
      :class="{ active: activeTab === tab }"
      @click="activeTab = tab"
    >
      {{ t(`runtime.tabs.${tab}`) }}
    </button>
  </nav>

  <div v-if="loading" class="page-loading">{{ t("common.loading") }}…</div>
  <section v-else-if="overview" class="runtime-page">
    <template v-if="activeTab === 'overview'">
      <div class="runtime-metrics">
        <article>
          <small>{{ t("runtime.currentVersion") }}</small>
          <strong>{{ overview.selectedVersion ?? "—" }}</strong>
          <span>{{ selectedVersion ? supportLabel(selectedVersion) : t("runtime.notInstalled") }}</span>
        </article>
        <article>
          <small>{{ t("runtime.installedVersions") }}</small>
          <strong>{{ overview.installedCount }}</strong>
          <span>{{ overview.versions.length }} {{ t("runtime.available") }}</span>
        </article>
        <article>
          <small>{{ t("runtime.diskUsage") }}</small>
          <strong>{{ formatBytes(overview.totalDiskBytes) }}</strong>
          <span>~/.devbox/runtimes/{{ kind }}</span>
        </article>
        <article>
          <small>{{ t("runtime.platform") }}</small>
          <strong>{{ overview.compatible ? "READY" : "N/A" }}</strong>
          <span>{{ overview.platformLabel }}</span>
        </article>
      </div>

      <div class="runtime-overview-grid">
        <article class="runtime-panel">
          <div class="runtime-panel-head">
            <div><small>ACTIVE RUNTIME</small><h2>{{ t("runtime.activeRuntime") }}</h2></div>
            <button type="button" @click="activeTab = 'versions'">{{ t("runtime.manageVersions") }}</button>
          </div>
          <div v-if="selectedVersion" class="active-runtime-card">
            <span class="service-logo runtime-logo" :class="kind">{{ icon }}</span>
            <div>
              <strong>{{ runtimeName }} {{ selectedVersion.series }}</strong>
              <code>{{ selectedVersion.version }}</code>
              <p>{{ selectedVersion.executablePath }}</p>
            </div>
            <button class="primary" type="button" @click="openTerminal()">{{ t("runtime.openTerminal") }}</button>
          </div>
          <div v-else class="runtime-empty">
            <strong>{{ t("runtime.noActiveVersion") }}</strong>
            <p>{{ t("runtime.noActiveHint") }}</p>
            <button class="primary" type="button" @click="activeTab = 'versions'">{{ t("runtime.installRuntime") }}</button>
          </div>
        </article>

        <article class="runtime-panel">
          <div class="runtime-panel-head">
            <div><small>DIAGNOSTICS</small><h2>{{ t("runtime.diagnosticResult") }}</h2></div>
            <span v-if="diagnostic" class="runtime-health" :class="{ ok: diagnostic.success }">
              {{ diagnostic.success ? t("runtime.healthy") : t("runtime.abnormal") }}
            </span>
          </div>
          <pre v-if="diagnostic">{{ diagnostic.output }}</pre>
          <div v-else class="runtime-empty compact">
            <p>{{ t("runtime.diagnosticHint") }}</p>
            <button type="button" :disabled="!overview.selectedVersion" @click="diagnose">{{ t("runtime.runDiagnostic") }}</button>
          </div>
        </article>
      </div>

      <article v-if="kind === 'go'" class="runtime-panel proxy-panel">
        <div class="runtime-panel-head">
          <div><small>GOPROXY</small><h2>{{ t("runtime.go.proxyTitle") }}</h2></div>
        </div>
        <div class="proxy-controls">
          <select v-model="goProxyDraft">
            <option value="https://proxy.golang.org,direct">proxy.golang.org · Official</option>
            <option value="https://goproxy.cn,direct">goproxy.cn · China</option>
            <option value="https://goproxy.io,direct">goproxy.io</option>
            <option value="direct">direct</option>
          </select>
          <input v-model="goProxyDraft" spellcheck="false" />
          <button class="primary" type="button" @click="saveGoProxy">{{ t("common.save") }}</button>
        </div>
      </article>
    </template>

    <div
      v-else-if="activeTab === 'versions'"
      class="redis-version-manager runtime-version-manager"
    >
      <div class="redis-version-head">
        <div>
          <p>VERSION MANAGER</p>
          <h2>{{ runtimeName }} {{ t("runtime.versionManager") }}</h2>
        </div>
        <span>{{ t("runtime.isolatedRuntime") }} · {{ overview.platformLabel }}</span>
      </div>

      <div class="redis-version-grid">
        <button
          v-for="version in overview.versions"
          :key="version.version"
          type="button"
          :class="{
            selected: versionTarget === version.version,
            active: version.selected,
            legacy: version.legacy,
          }"
          :disabled="Boolean(busyVersion)"
          @click="versionTarget = version.version"
        >
          <span class="redis-version-radio"></span>
          <span class="redis-version-copy">
            <strong>{{ runtimeName }} {{ version.series }}</strong>
            <small>v{{ version.version }}</small>
          </span>
          <span class="redis-version-badges">
            <i v-if="version.selected">{{ t("runtime.currentBadge") }}</i>
            <i v-else-if="version.installed">{{ t("runtime.installedBadge") }}</i>
            <i v-if="version.recommended" class="recommended">
              {{ t("runtime.recommended") }}
            </i>
          </span>
          <em>
            {{ supportLabel(version) }}
            <template v-if="version.installed">
              · {{ formatBytes(version.diskBytes) }}
            </template>
          </em>
        </button>
      </div>

      <div class="redis-version-footer">
        <p>{{ t("runtime.versionNote") }}</p>
        <div>
          <button
            v-if="selectedTargetVersion?.installed"
            type="button"
            class="version-remove-button"
            :disabled="Boolean(busyVersion)"
            @click="uninstall(selectedTargetVersion)"
          >
            {{ t("runtime.uninstall") }}
          </button>
          <button
            type="button"
            :disabled="
              !selectedTargetVersion ||
              selectedTargetVersion.selected ||
              !selectedTargetVersion.compatible ||
              Boolean(busyVersion)
            "
            @click="activateTargetVersion"
          >
            <span v-if="busyVersion" class="spinner"></span>
            {{
              selectedTargetVersion?.selected
                ? t("runtime.currentBadge")
                : busyVersion
                  ? t("common.loading")
                  : selectedTargetVersion?.installed
                    ? t("runtime.useVersion")
                    : t("runtime.installAndUse")
            }}
          </button>
        </div>
      </div>
    </div>

    <article v-else-if="activeTab === 'projects'" class="runtime-panel projects-panel">
      <div class="runtime-panel-head">
        <div><small>PROJECT PROFILES</small><h2>{{ t("runtime.projectProfiles") }}</h2></div>
        <button class="primary" type="button" @click="addProject">＋ {{ t("runtime.addProject") }}</button>
      </div>
      <div v-if="projects.length" class="runtime-project-list">
        <div v-for="project in projects" :key="project.id" class="runtime-project-row">
          <span class="project-mark">{{ project.name.slice(0, 1).toUpperCase() }}</span>
          <div>
            <strong>{{ project.name }}</strong>
            <small>{{ project.path }}</small>
          </div>
          <code>{{ runtimeName }} {{ projectVersion(project) ?? "—" }}</code>
          <button type="button" :disabled="!projectVersion(project)" @click="openTerminal(project.path, projectVersion(project))">{{ t("runtime.terminal") }}</button>
          <button type="button" @click="editProject(project)">{{ t("common.edit") }}</button>
          <button class="danger" type="button" @click="deleteProject(project)">×</button>
        </div>
      </div>
      <div v-else class="runtime-empty tall">
        <strong>{{ t("runtime.noProjects") }}</strong>
        <p>{{ t("runtime.noProjectsHint") }}</p>
        <button class="primary" type="button" @click="addProject">{{ t("runtime.addProject") }}</button>
      </div>
    </article>

    <article v-else class="runtime-panel environment-panel">
      <div class="runtime-panel-head">
        <div><small>ENVIRONMENT</small><h2>{{ t("runtime.environmentPreview") }}</h2></div>
        <button type="button" :disabled="!overview.environment.length" @click="copyEnvironment">{{ t("runtime.copyExports") }}</button>
      </div>
      <div v-if="overview.environment.length" class="environment-list">
        <div v-for="variable in overview.environment" :key="variable.key">
          <strong>{{ variable.key }}</strong><code>{{ variable.value }}</code>
        </div>
      </div>
      <div v-else class="runtime-empty tall">
        <p>{{ t("runtime.installBeforeEnvironment") }}</p>
      </div>
      <p class="environment-safety">{{ t("runtime.environmentSafety") }}</p>
    </article>
  </section>

  <div v-if="projectModalOpen" class="runtime-modal-backdrop" role="dialog" aria-modal="true" @click.self="projectModalOpen = false">
    <form class="runtime-modal" @submit.prevent="saveProject">
      <div class="runtime-modal-head"><div><small>PROJECT PROFILE</small><h2>{{ t("runtime.projectSettings") }}</h2></div><button type="button" @click="projectModalOpen = false">×</button></div>
      <label>{{ t("runtime.projectName") }}<input v-model="projectDraft.name" /></label>
      <label>{{ t("runtime.projectPath") }}<input v-model="projectDraft.path" readonly /></label>
      <label>{{ runtimeName }}<select v-model="projectRuntimeVersion"><option :value="null">{{ t("runtime.notConfigured") }}</option><option v-for="version in installedVersions" :key="version.version" :value="version.version">{{ version.version }}</option></select></label>
      <div class="runtime-modal-actions"><button type="button" @click="projectModalOpen = false">{{ t("common.cancel") }}</button><button class="primary" type="submit" :disabled="savingProject">{{ t("common.save") }}</button></div>
    </form>
  </div>
</template>

<style scoped>
.runtime-notice{margin-bottom:0}.runtime-tabs{height:44px}.runtime-page{display:grid;gap:14px;padding:24px 32px 38px}.runtime-metrics{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));border:1px solid var(--color-border);background:var(--color-panel-translucent)}.runtime-metrics article{display:grid;min-width:0;gap:5px;padding:14px 16px;border-right:1px solid var(--color-border)}.runtime-metrics article:last-child{border-right:0}.runtime-metrics small,.runtime-panel-head small{color:var(--color-text-muted);font:8px/1.2 "SFMono-Regular",Consolas,monospace;letter-spacing:.11em}.runtime-metrics strong{overflow:hidden;font-size:20px;text-overflow:ellipsis;white-space:nowrap}.runtime-metrics span{overflow:hidden;color:var(--color-text-muted);font-size:8px;text-overflow:ellipsis;white-space:nowrap}.runtime-overview-grid{display:grid;grid-template-columns:1.25fr .75fr;gap:14px}.runtime-panel{min-width:0;overflow:hidden;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.runtime-panel-head{display:flex;min-height:58px;align-items:center;justify-content:space-between;gap:16px;padding:10px 14px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.runtime-panel-head h2{margin:4px 0 0;font-size:14px}.runtime-panel-head>span{color:var(--color-text-muted);font-size:9px}.runtime-panel button,.runtime-modal button{min-height:30px;padding:7px 11px;border:1px solid var(--color-border-strong);background:var(--color-bg-panel);color:var(--color-text-primary);font-size:9px}.runtime-panel button.primary,.runtime-modal button.primary{border-color:var(--color-control-primary);background:var(--color-control-primary);color:#fff}.runtime-panel button.danger{border-color:var(--color-danger-text);color:var(--color-danger-text)}.active-runtime-card{display:grid;grid-template-columns:48px minmax(0,1fr) auto;align-items:center;gap:14px;padding:22px}.active-runtime-card>div{display:grid;min-width:0;gap:4px}.active-runtime-card strong{font-size:14px}.active-runtime-card code{font:10px "SFMono-Regular",Consolas,monospace}.active-runtime-card p{overflow:hidden;margin:2px 0 0;color:var(--color-text-muted);font-size:8px;text-overflow:ellipsis;white-space:nowrap}.runtime-empty{display:grid;min-height:120px;place-items:center;align-content:center;gap:8px;padding:20px;color:var(--color-text-muted);text-align:center}.runtime-empty.compact{min-height:104px}.runtime-empty.tall{min-height:280px}.runtime-empty strong{color:var(--color-text);font-size:12px}.runtime-empty p{max-width:440px;margin:0;font-size:9px;line-height:1.6}.runtime-panel pre{min-height:104px;max-height:220px;margin:0;overflow:auto;padding:14px;color:var(--color-text-secondary);font:9px/1.6 "SFMono-Regular",Consolas,monospace;white-space:pre-wrap}.runtime-health{padding:4px 7px;border:1px solid var(--color-danger-text);color:var(--color-danger-text)!important}.runtime-health.ok{border-color:var(--color-success-text);color:var(--color-success-text)!important}.proxy-controls{display:grid;grid-template-columns:190px minmax(0,1fr) auto;gap:9px;padding:14px}.proxy-controls select,.proxy-controls input{min-width:0;height:34px;padding:0 10px}.runtime-version-grid{display:grid;grid-template-columns:repeat(3,minmax(0,1fr))}.runtime-version-card{position:relative;display:grid;min-height:175px;gap:10px;padding:18px;border-right:1px solid var(--color-border);border-bottom:1px solid var(--color-border)}.runtime-version-card:nth-child(3n){border-right:0}.runtime-version-card.selected{background:var(--color-panel-active);box-shadow:inset 0 -3px var(--color-accent)}.runtime-version-title{display:flex;align-items:center;gap:10px}.runtime-version-title i{width:10px;height:10px;border:2px solid var(--color-text-muted);border-radius:50%}.runtime-version-card.selected .runtime-version-title i{border:3px solid var(--color-accent)}.runtime-version-title>div{display:grid;gap:4px}.runtime-version-title strong{font-size:13px}.runtime-version-title code{color:var(--color-text-muted);font:9px "SFMono-Regular",Consolas,monospace}.runtime-version-card>p{margin:0;color:var(--color-text-muted);font-size:9px}.runtime-version-card>small{color:var(--color-text-muted);font-size:8px}.version-badges{display:flex;flex-wrap:wrap;gap:5px}.version-badges span{padding:3px 6px;border:1px solid var(--color-success-text);color:var(--color-success-text);font-size:7px}.version-badges .recommended{border-color:var(--color-warning-text);color:var(--color-warning-text)}.runtime-version-actions{display:flex;align-items:end;gap:7px;margin-top:auto}.runtime-version-note,.environment-safety{margin:0;padding:13px 16px;color:var(--color-text-muted);font-size:8px}.runtime-project-list{display:grid}.runtime-project-row{display:grid;grid-template-columns:34px minmax(200px,1fr) 130px auto auto 34px;align-items:center;gap:10px;padding:11px 14px;border-bottom:1px solid var(--color-border)}.project-mark{display:grid;width:30px;height:30px;place-items:center;border-radius:50%;background:var(--color-bg-muted);color:var(--color-accent);font-weight:700}.runtime-project-row>div{display:grid;min-width:0;gap:3px}.runtime-project-row strong,.runtime-project-row small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.runtime-project-row strong{font-size:10px}.runtime-project-row small{color:var(--color-text-muted);font-size:8px}.runtime-project-row code{font:9px "SFMono-Regular",Consolas,monospace}.environment-list>div{display:grid;grid-template-columns:140px minmax(0,1fr);gap:12px;padding:11px 14px;border-bottom:1px solid var(--color-border)}.environment-list strong{color:var(--color-accent);font:9px "SFMono-Regular",Consolas,monospace}.environment-list code{overflow:hidden;font:9px "SFMono-Regular",Consolas,monospace;text-overflow:ellipsis;white-space:nowrap}.runtime-modal-backdrop{position:fixed;z-index:130;inset:0;display:grid;place-items:center;padding:24px;background:rgba(0,0,0,.48);backdrop-filter:blur(5px)}.runtime-modal{display:grid;width:min(520px,calc(100vw - 48px));gap:14px;padding:22px;border:1px solid var(--color-border-strong);background:var(--color-panel);box-shadow:0 24px 80px rgba(0,0,0,.32)}.runtime-modal-head{display:flex;align-items:start;justify-content:space-between}.runtime-modal-head small{color:var(--color-text-muted);font:8px "SFMono-Regular",Consolas,monospace;letter-spacing:.12em}.runtime-modal-head h2{margin:4px 0 0}.runtime-modal-head button{width:30px;padding:0}.runtime-modal label{display:grid;gap:6px;color:var(--color-text-secondary);font-size:9px}.runtime-modal input,.runtime-modal select{box-sizing:border-box;width:100%;height:36px;padding:0 10px}.runtime-modal-actions{display:flex;justify-content:flex-end;gap:8px}.runtime-logo.go{background:#087c91}.runtime-logo.java{background:#b74b37}@media(max-width:1100px){.runtime-metrics{grid-template-columns:repeat(2,1fr)}.runtime-metrics article:nth-child(2){border-right:0}.runtime-overview-grid{grid-template-columns:1fr}.runtime-version-grid{grid-template-columns:1fr}.runtime-version-card{border-right:0}.runtime-project-row{grid-template-columns:34px minmax(160px,1fr) 110px auto auto 34px}}
</style>
