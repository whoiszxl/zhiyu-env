<script setup lang="ts">
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  ref,
  watch,
} from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { FitAddon } from "@xterm/addon-fit";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import {
  connectSshTerminal,
  deleteSshProfile,
  disconnectSshTerminal,
  listSshProfiles,
  previewSshHostKey,
  resizeSshTerminal,
  saveSshProfile,
  testSshConnection,
  trustSshHostKey,
  writeSshTerminal,
} from "../../api/tools";
import type {
  SshHostKey,
  SshProfile,
  SshTerminalEvent,
} from "../../types";

const props = withDefaults(defineProps<{ visible?: boolean }>(), {
  visible: true,
});

function emptyProfile(): SshProfile {
  return {
    id: "",
    name: "",
    host: "",
    port: 22,
    username: "",
    identityFile: "",
    authMethod: "key",
    createdAtMillis: 0,
    updatedAtMillis: 0,
  };
}

const profiles = ref<SshProfile[]>([]);
const selectedId = ref("");
const draft = ref<SshProfile>(emptyProfile());
const loading = ref(true);
const saving = ref(false);
const deleting = ref(false);
const checkingFingerprint = ref(false);
const trustingFingerprint = ref(false);
const testing = ref(false);
const error = ref("");
const notice = ref("");
const fingerprint = ref<SshHostKey | null>(null);
const modalOpen = ref(false);
const passwordDraft = ref("");
const passwordVisible = ref(false);
const sessionPasswords = ref<Record<string, string>>({});
const storedIdleTimeout = Number(
  window.localStorage.getItem("zhiyu.ssh.idle-timeout-minutes"),
);
const idleTimeoutMinutes = ref(
  [0, 10, 30, 60, 120].includes(storedIdleTimeout)
    ? storedIdleTimeout
    : 30,
);
const profileListCollapsed = ref(
  window.localStorage.getItem("zhiyu.ssh.profile-list-collapsed") === "true",
);
const profileListMotion = ref<"" | "opening" | "closing">("");
const terminalElement = ref<HTMLDivElement | null>(null);
const terminalSessionId = ref("");
const terminalConnecting = ref(false);
const terminalStatus = ref<"disconnected" | "connecting" | "connected">(
  "disconnected",
);
let terminal: Terminal | null = null;
let fitAddon: FitAddon | null = null;
let terminalUnlisten: UnlistenFn | null = null;
let terminalResizeObserver: ResizeObserver | null = null;
let profileListTimer: number | null = null;
let idleDisconnectTimer: number | null = null;

const selectedProfile = computed(
  () => profiles.value.find((profile) => profile.id === selectedId.value) ?? null,
);
const canConnect = computed(
  () => Boolean(selectedId.value) && !saving.value && !deleting.value,
);
const isDirty = computed(() => {
  const selected = selectedProfile.value;
  if (!selected) {
    return Boolean(
      draft.value.name ||
        draft.value.host ||
        draft.value.username ||
        draft.value.identityFile,
    );
  }
  return JSON.stringify(selected) !== JSON.stringify(draft.value);
});
const canSaveModal = computed(
  () =>
    isDirty.value ||
    (draft.value.authMethod === "password" &&
      passwordDraft.value.length > 0),
);

async function loadProfiles(preferredId?: string) {
  loading.value = true;
  error.value = "";
  try {
    profiles.value = await listSshProfiles();
    const target =
      profiles.value.find((profile) => profile.id === preferredId) ??
      profiles.value.find((profile) => profile.id === selectedId.value) ??
      profiles.value[0];
    if (target) {
      selectProfile(target);
    } else {
      resetDraft();
    }
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

function selectProfile(profile: SshProfile) {
  selectedId.value = profile.id;
  draft.value = { ...profile };
  fingerprint.value = null;
  error.value = "";
  notice.value = "";
}

async function switchProfile(profile: SshProfile) {
  if (profile.id === selectedId.value) return;
  if (terminalSessionId.value) {
    await disconnectTerminal();
  }
  selectProfile(profile);
  terminal?.reset();
  writeTerminalWelcome();
}

function persistProfileListState() {
  window.localStorage.setItem(
    "zhiyu.ssh.profile-list-collapsed",
    String(profileListCollapsed.value),
  );
}

function fitTerminalAfterLayout() {
  window.requestAnimationFrame(() => {
    window.requestAnimationFrame(() => {
      fitAddon?.fit();
      terminal?.focus();
    });
  });
}

function clearIdleDisconnectTimer() {
  if (idleDisconnectTimer !== null) {
    window.clearTimeout(idleDisconnectTimer);
    idleDisconnectTimer = null;
  }
}

function resetIdleDisconnectTimer() {
  clearIdleDisconnectTimer();
  if (!terminalSessionId.value || idleTimeoutMinutes.value === 0) return;
  idleDisconnectTimer = window.setTimeout(() => {
    idleDisconnectTimer = null;
    if (!terminalSessionId.value) return;
    terminal?.writeln(
      `\r\n\x1b[33m[已闲置 ${idleTimeoutMinutes.value} 分钟，智屿自动断开 SSH 连接]\x1b[0m`,
    );
    void disconnectTerminal();
  }, idleTimeoutMinutes.value * 60 * 1000);
}

function updateIdleTimeout() {
  window.localStorage.setItem(
    "zhiyu.ssh.idle-timeout-minutes",
    String(idleTimeoutMinutes.value),
  );
  resetIdleDisconnectTimer();
}

async function toggleProfileList() {
  if (profileListMotion.value) return;
  if (profileListCollapsed.value) {
    profileListCollapsed.value = false;
    profileListMotion.value = "opening";
    persistProfileListState();
    await nextTick();
    window.requestAnimationFrame(() => {
      window.requestAnimationFrame(() => {
        profileListMotion.value = "";
      });
    });
    fitTerminalAfterLayout();
    return;
  }

  profileListMotion.value = "closing";
  profileListTimer = window.setTimeout(() => {
    profileListCollapsed.value = true;
    profileListMotion.value = "";
    profileListTimer = null;
    persistProfileListState();
    fitTerminalAfterLayout();
  }, 150);
}

function resetDraft() {
  selectedId.value = "";
  draft.value = emptyProfile();
  fingerprint.value = null;
  error.value = "";
  notice.value = "";
}

function openCreateModal() {
  draft.value = emptyProfile();
  passwordDraft.value = "";
  passwordVisible.value = false;
  modalOpen.value = true;
}

function openEditModal() {
  const profile = selectedProfile.value;
  if (!profile) return;
  draft.value = { ...profile };
  passwordDraft.value = sessionPasswords.value[profile.id] ?? "";
  passwordVisible.value = false;
  modalOpen.value = true;
}

function closeModal() {
  if (saving.value || deleting.value) return;
  modalOpen.value = false;
  passwordDraft.value = "";
  if (selectedProfile.value) {
    draft.value = { ...selectedProfile.value };
  }
}

async function chooseIdentityFile() {
  const selected = await open({
    multiple: false,
    title: "选择 SSH 私钥",
    defaultPath: draft.value.identityFile || undefined,
  });
  if (typeof selected === "string") {
    draft.value.identityFile = selected;
  }
}

async function saveProfile() {
  if (saving.value) return;
  saving.value = true;
  error.value = "";
  notice.value = "";
  try {
    if (terminalSessionId.value) {
      await disconnectTerminal();
    }
    const saved = await saveSshProfile({ ...draft.value });
    if (saved.authMethod === "password" && passwordDraft.value) {
      sessionPasswords.value[saved.id] = passwordDraft.value;
    } else if (saved.authMethod === "key") {
      delete sessionPasswords.value[saved.id];
    }
    await loadProfiles(saved.id);
    modalOpen.value = false;
    passwordDraft.value = "";
    notice.value = "连接配置已安全保存在本机";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    saving.value = false;
  }
}

async function removeProfile() {
  if (!draft.value.id || deleting.value) return;
  if (!window.confirm(`确定删除连接“${draft.value.name}”吗？私钥文件不会被删除。`)) {
    return;
  }
  deleting.value = true;
  error.value = "";
  try {
    if (terminalSessionId.value) {
      await disconnectTerminal();
    }
    await deleteSshProfile(draft.value.id);
    delete sessionPasswords.value[draft.value.id];
    modalOpen.value = false;
    await loadProfiles();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    deleting.value = false;
  }
}

async function inspectFingerprint() {
  if (!canConnect.value || checkingFingerprint.value) return;
  checkingFingerprint.value = true;
  error.value = "";
  notice.value = "";
  fingerprint.value = null;
  try {
    fingerprint.value = await previewSshHostKey(selectedId.value);
  } catch (cause) {
    error.value = String(cause);
  } finally {
    checkingFingerprint.value = false;
  }
}

async function trustFingerprint() {
  if (!fingerprint.value || trustingFingerprint.value) return;
  trustingFingerprint.value = true;
  error.value = "";
  try {
    fingerprint.value = await trustSshHostKey(
      selectedId.value,
      fingerprint.value.fingerprint,
    );
    notice.value = "主机指纹已写入智屿独立的 known_hosts";
    fingerprint.value = null;
  } catch (cause) {
    error.value = String(cause);
  } finally {
    trustingFingerprint.value = false;
  }
}

function activePassword(): string | undefined | null {
  const profile = selectedProfile.value;
  if (profile?.authMethod !== "password") return undefined;
  const password = sessionPasswords.value[profile.id];
  if (password) return password;
  error.value = "请输入本次会话使用的 SSH 密码";
  openEditModal();
  return null;
}

async function testConnection() {
  if (!canConnect.value || testing.value) return;
  testing.value = true;
  error.value = "";
  notice.value = "";
  try {
    const password = activePassword();
    if (password === null) return;
    const result = await testSshConnection(
      selectedId.value,
      password,
    );
    if (result.success) {
      notice.value = "SSH 连接成功";
    } else {
      error.value = result.stderr || "SSH 连接失败";
    }
  } catch (cause) {
    error.value = String(cause);
  } finally {
    testing.value = false;
  }
}

function writeTerminalWelcome() {
  if (!terminal) return;
  terminal.writeln("\x1b[38;2;114;180;133m智屿 SSH 交互终端\x1b[0m");
  terminal.writeln(
    selectedProfile.value
      ? `点击“连接终端”进入 ${selectedProfile.value.name}。`
      : "请先选择或创建一个 SSH 连接。",
  );
  terminal.writeln("");
}

function decodeTerminalData(value: string): Uint8Array {
  const binary = window.atob(value);
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

async function initializeTerminal() {
  if (!terminalElement.value || terminal) return;
  terminal = new Terminal({
    cursorBlink: true,
    cursorStyle: "block",
    convertEol: true,
    fontFamily:
      '"SFMono-Regular", "SF Mono", Menlo, Monaco, Consolas, monospace',
    fontSize: 13,
    lineHeight: 1.35,
    scrollback: 3000,
    allowTransparency: false,
    theme: {
      background: "#171b17",
      foreground: "#dce3da",
      cursor: "#79bd8b",
      cursorAccent: "#171b17",
      selectionBackground: "#45644d",
      black: "#171b17",
      brightBlack: "#697168",
      green: "#79bd8b",
      brightGreen: "#9bd5a8",
      red: "#d66b55",
      brightRed: "#eb8874",
      yellow: "#d5aa60",
      brightYellow: "#e8c37f",
      blue: "#78a9d2",
      brightBlue: "#96c2e5",
    },
  });
  fitAddon = new FitAddon();
  terminal.loadAddon(fitAddon);
  terminal.open(terminalElement.value);
  fitAddon.fit();
  terminal.onData((data) => {
    const sessionId = terminalSessionId.value;
    if (!sessionId) return;
    resetIdleDisconnectTimer();
    void writeSshTerminal(sessionId, data).catch((cause) => {
      error.value = String(cause);
    });
  });
  terminalElement.value.addEventListener("pointerdown", resetIdleDisconnectTimer);
  terminalElement.value.addEventListener("wheel", resetIdleDisconnectTimer, {
    passive: true,
  });
  terminal.onResize(({ cols, rows }) => {
    const sessionId = terminalSessionId.value;
    if (!sessionId) return;
    void resizeSshTerminal(sessionId, cols, rows).catch(() => undefined);
  });
  terminalResizeObserver = new ResizeObserver(() => {
    if (!props.visible || !terminalElement.value?.offsetParent) return;
    window.requestAnimationFrame(() => fitAddon?.fit());
  });
  terminalResizeObserver.observe(terminalElement.value);
  terminalUnlisten = await listen<SshTerminalEvent>(
    "ssh-terminal-event",
    ({ payload }) => {
      if (payload.sessionId !== terminalSessionId.value || !terminal) return;
      if (payload.event === "data") {
        terminal.write(decodeTerminalData(payload.data));
      } else if (payload.event === "error") {
        terminal.writeln(`\r\n\x1b[31m终端错误：${payload.data}\x1b[0m`);
      } else {
        clearIdleDisconnectTimer();
        terminalSessionId.value = "";
        terminalStatus.value = "disconnected";
        terminal.writeln("\r\n\x1b[90m[SSH 连接已断开]\x1b[0m");
      }
    },
  );
  writeTerminalWelcome();
}

async function connectTerminal() {
  const profile = selectedProfile.value;
  if (!profile || terminalConnecting.value || terminalSessionId.value) return;
  terminalConnecting.value = true;
  terminalStatus.value = "connecting";
  error.value = "";
  notice.value = "";
  await nextTick();
  fitAddon?.fit();
  const sessionId = `terminal-${crypto.randomUUID().replaceAll("-", "")}`;
  terminalSessionId.value = sessionId;
  terminal?.reset();
  terminal?.writeln(
    `\x1b[90m正在连接 ${profile.username}@${profile.host}:${profile.port}…\x1b[0m`,
  );
  try {
    await connectSshTerminal(
      sessionId,
      profile.id,
      Math.max(40, terminal?.cols ?? 80),
      Math.max(10, terminal?.rows ?? 24),
    );
    if (terminalSessionId.value === sessionId) {
      terminalStatus.value = "connected";
      terminal?.focus();
      resetIdleDisconnectTimer();
    }
  } catch (cause) {
    terminalSessionId.value = "";
    terminalStatus.value = "disconnected";
    error.value = String(cause);
    terminal?.writeln(`\r\n\x1b[31m连接失败：${String(cause)}\x1b[0m`);
  } finally {
    terminalConnecting.value = false;
  }
}

async function disconnectTerminal() {
  const sessionId = terminalSessionId.value;
  if (!sessionId) return;
  clearIdleDisconnectTimer();
  terminalSessionId.value = "";
  terminalStatus.value = "disconnected";
  try {
    await disconnectSshTerminal(sessionId);
  } catch (cause) {
    error.value = String(cause);
  }
  terminal?.writeln("\r\n\x1b[90m[SSH 连接已断开]\x1b[0m");
}

onMounted(() => {
  void loadProfiles();
  void nextTick().then(initializeTerminal);
});

watch(
  () => props.visible,
  (visible) => {
    if (!visible) return;
    resetIdleDisconnectTimer();
    void nextTick().then(() => {
      fitTerminalAfterLayout();
    });
  },
);

onBeforeUnmount(() => {
  const sessionId = terminalSessionId.value;
  terminalSessionId.value = "";
  if (sessionId) {
    void disconnectSshTerminal(sessionId);
  }
  terminalResizeObserver?.disconnect();
  terminalUnlisten?.();
  terminal?.dispose();
  clearIdleDisconnectTimer();
  if (profileListTimer !== null) {
    window.clearTimeout(profileListTimer);
  }
});
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo ssh">&gt;_</span>
      <div>
        <div class="title-line">
          <h1>SSH 连接管理</h1>
          <span>SECURE SHELL</span>
        </div>
        <p>安全直连远程服务器，支持密钥和密码认证</p>
      </div>
    </div>
  </header>

  <div v-if="notice" class="notice">
    <span>{{ notice }}</span>
    <button type="button" @click="notice = ''">×</button>
  </div>
  <div v-if="error" class="notice danger">
    <span>{{ error }}</span>
    <button type="button" @click="error = ''">×</button>
  </div>

  <section
    class="ssh-page"
    :class="{
      'list-collapsed': profileListCollapsed,
      'list-opening': profileListMotion === 'opening',
      'list-closing': profileListMotion === 'closing',
    }"
  >
    <aside class="ssh-profile-list" :aria-hidden="profileListCollapsed">
      <div class="ssh-panel-heading">
        <div><p>CONNECTIONS</p><h2>服务器</h2></div>
        <div class="ssh-panel-actions">
          <button
            type="button"
            title="新建连接"
            aria-label="新建连接"
            @click="openCreateModal"
          >
            <svg viewBox="0 0 20 20" aria-hidden="true">
              <path d="M10 4v12M4 10h12" />
            </svg>
          </button>
          <button
            type="button"
            title="隐藏服务器列表"
            aria-label="隐藏服务器列表"
            :disabled="Boolean(profileListMotion)"
            @click="toggleProfileList"
          >
            <svg viewBox="0 0 20 20" aria-hidden="true">
              <rect x="2.5" y="3" width="15" height="14" rx="1.5" />
              <path d="M7 3v14M13 7l-3 3 3 3" />
            </svg>
          </button>
        </div>
      </div>
      <div v-if="loading" class="ssh-list-empty">正在读取本地连接…</div>
      <div v-else-if="profiles.length === 0" class="ssh-list-empty">
        <span>&gt;_</span>
        <strong>还没有连接</strong>
        <small>点击右上角加号创建</small>
      </div>
      <template v-else>
        <button
          v-for="profile in profiles"
          :key="profile.id"
          type="button"
          class="ssh-profile-item"
          :class="{ selected: selectedId === profile.id }"
          @click="switchProfile(profile)"
        >
          <span>{{ profile.name.slice(0, 1).toUpperCase() }}</span>
          <div>
            <strong>{{ profile.name }}</strong>
            <small>{{ profile.username }}@{{ profile.host }}:{{ profile.port }}</small>
          </div>
          <i></i>
        </button>
      </template>
      <div class="ssh-local-note">
        <strong>本地安全存储</strong>
        <small>私钥内容不会进入智屿；密码仅保留在当前应用会话内。</small>
      </div>
    </aside>

    <div class="ssh-workspace">
      <button
        v-if="profileListCollapsed"
        type="button"
        class="ssh-list-restore"
        title="展开服务器列表"
        aria-label="展开服务器列表"
        :disabled="Boolean(profileListMotion)"
        @click="toggleProfileList"
      >
        <svg viewBox="0 0 20 20" aria-hidden="true">
          <rect x="2.5" y="3" width="15" height="14" rx="1.5" />
          <path d="M7 3v14M10 7l3 3-3 3" />
        </svg>
      </button>
      <article v-if="selectedProfile" class="ssh-active-card">
        <div class="ssh-active-identity">
          <span>{{ selectedProfile.name.slice(0, 1).toUpperCase() }}</span>
          <div>
            <p>ACTIVE CONNECTION</p>
            <h2>{{ selectedProfile.name }}</h2>
            <small>
              {{ selectedProfile.username }}@{{ selectedProfile.host }}:{{
                selectedProfile.port
              }}
            </small>
          </div>
        </div>
        <div class="ssh-active-auth">
          <span>{{ selectedProfile.authMethod === "password" ? "密码" : "密钥" }}</span>
          <small v-if="selectedProfile.authMethod === 'password'">
            {{
              sessionPasswords[selectedProfile.id]
                ? "本次会话已输入"
                : "需要输入密码"
            }}
          </small>
          <small v-else>
            {{ selectedProfile.identityFile ? "指定私钥" : "ssh-agent / 默认密钥" }}
          </small>
        </div>
        <div class="ssh-active-actions">
          <button
            type="button"
            title="编辑连接"
            aria-label="编辑连接"
            @click="openEditModal"
          >
            ✎
          </button>
          <button
            type="button"
            title="核对主机指纹"
            aria-label="核对主机指纹"
            :disabled="checkingFingerprint"
            @click="inspectFingerprint"
          >
            <span v-if="checkingFingerprint" class="spinner"></span>
            <span v-else>⌁</span>
          </button>
          <button
            type="button"
            title="测试连接"
            aria-label="测试连接"
            :disabled="testing"
            @click="testConnection"
          >
            <span v-if="testing" class="spinner"></span>
            <span v-else>✓</span>
          </button>
        </div>
      </article>
      <article v-else class="ssh-active-empty">
        <div>
          <span>&gt;_</span>
          <strong>创建一个 SSH 连接开始使用</strong>
          <small>支持密钥认证和仅在当前会话保存的密码认证</small>
        </div>
        <button type="button" class="primary" @click="openCreateModal">
          ＋ 新建连接
        </button>
      </article>

      <article v-if="fingerprint" class="ssh-fingerprint-card">
        <div class="ssh-fingerprint-icon">⌁</div>
        <div>
          <p>HOST KEY VERIFICATION</p>
          <h3>请通过服务器控制台或管理员核对指纹</h3>
          <code>{{ fingerprint.fingerprint }}</code>
          <small
            >{{ fingerprint.keyType }} · {{ selectedProfile?.host }}:{{
              selectedProfile?.port
            }}</small
          >
        </div>
        <button
          type="button"
          :disabled="trustingFingerprint"
          @click="trustFingerprint"
        >
          <span v-if="trustingFingerprint" class="spinner"></span>
          已核对，信任此主机
        </button>
      </article>

      <article class="ssh-terminal-card">
        <div class="ssh-terminal-toolbar">
          <div class="ssh-terminal-heading">
            <i :class="terminalStatus"></i>
            <div>
              <p>INTERACTIVE TERMINAL</p>
              <h2>交互终端</h2>
            </div>
          </div>
          <code v-if="selectedProfile">
            {{ selectedProfile.username }}@{{ selectedProfile.host }}:{{
              selectedProfile.port
            }}
          </code>
          <span v-else>未选择连接</span>
          <label class="ssh-idle-timeout" title="超过所选时间未操作终端时自动断开">
            <span>闲置断开</span>
            <select
              v-model.number="idleTimeoutMinutes"
              aria-label="SSH 闲置自动断开时间"
              @change="updateIdleTimeout"
            >
              <option :value="10">10 分钟</option>
              <option :value="30">30 分钟</option>
              <option :value="60">1 小时</option>
              <option :value="120">2 小时</option>
              <option :value="0">永不断开</option>
            </select>
          </label>
          <button
            v-if="terminalStatus === 'disconnected'"
            type="button"
            class="primary"
            :disabled="!selectedProfile || terminalConnecting"
            @click="connectTerminal"
          >
            <span v-if="terminalConnecting" class="spinner"></span>
            {{ terminalConnecting ? "连接中" : "连接终端" }}
          </button>
          <button
            v-else
            type="button"
            class="disconnect"
            :disabled="terminalStatus === 'connecting'"
            @click="disconnectTerminal"
          >
            断开
          </button>
        </div>
        <div ref="terminalElement" class="ssh-terminal-host"></div>
        <footer>
          <span>支持 Tab 补全、方向键、Ctrl+C 和交互式程序</span>
          <span v-if="selectedProfile?.authMethod === 'password'">
            密码请直接在 SSH 提示符中输入，输入内容不会显示或落盘
          </span>
        </footer>
      </article>
    </div>
  </section>

  <Teleport to="body">
    <div
      v-if="modalOpen"
      class="ssh-modal-backdrop"
      role="presentation"
      @mousedown.self="closeModal"
    >
      <section
        class="ssh-profile-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="ssh-profile-modal-title"
      >
        <header>
          <div>
            <p>SSH CONNECTION</p>
            <h2 id="ssh-profile-modal-title">
              {{ draft.id ? "编辑 SSH 连接" : "新建 SSH 连接" }}
            </h2>
          </div>
          <button type="button" aria-label="关闭" @click="closeModal">×</button>
        </header>
        <div v-if="error" class="ssh-modal-error">{{ error }}</div>

        <div class="ssh-auth-options" role="radiogroup" aria-label="认证方式">
          <button
            type="button"
            :class="{ selected: draft.authMethod === 'key' }"
            @click="draft.authMethod = 'key'"
          >
            <span>⌁</span>
            <div><strong>密钥认证</strong><small>私钥或 ssh-agent</small></div>
          </button>
          <button
            type="button"
            :class="{ selected: draft.authMethod === 'password' }"
            @click="draft.authMethod = 'password'"
          >
            <span>••</span>
            <div><strong>密码认证</strong><small>仅保留当前会话</small></div>
          </button>
        </div>

        <div class="ssh-form modal-form">
          <label class="wide">
            <span>连接名称</span>
            <input v-model="draft.name" maxlength="50" placeholder="例如：开发服务器" />
          </label>
          <label class="host">
            <span>主机地址</span>
            <input
              v-model="draft.host"
              autocomplete="off"
              spellcheck="false"
              placeholder="192.168.1.10 或 server.example.com"
            />
          </label>
          <label class="port">
            <span>端口</span>
            <input v-model.number="draft.port" type="number" min="1" max="65535" />
          </label>
          <label>
            <span>用户名</span>
            <input
              v-model="draft.username"
              autocomplete="off"
              spellcheck="false"
              placeholder="deploy"
            />
          </label>
          <label v-if="draft.authMethod === 'key'" class="wide">
            <span>私钥文件 <small>可留空，使用 ssh-agent 和默认密钥</small></span>
            <div class="ssh-key-field">
              <input
                v-model="draft.identityFile"
                readonly
                placeholder="~/.ssh/id_ed25519"
              />
              <button type="button" @click="chooseIdentityFile">选择</button>
              <button
                v-if="draft.identityFile"
                type="button"
                title="清除私钥路径"
                @click="draft.identityFile = ''"
              >
                ×
              </button>
            </div>
          </label>
          <label v-else class="wide">
            <span>SSH 密码 <small>不会写入 profiles.json</small></span>
            <div class="ssh-password-field">
              <input
                v-model="passwordDraft"
                :type="passwordVisible ? 'text' : 'password'"
                autocomplete="new-password"
                maxlength="1024"
                placeholder="输入本次会话使用的密码"
              />
              <button type="button" @click="passwordVisible = !passwordVisible">
                {{ passwordVisible ? "隐藏" : "显示" }}
              </button>
            </div>
            <small class="ssh-password-note">
              关闭智屿后密码自动清除，下次连接时需要重新输入。
            </small>
          </label>
        </div>

        <footer>
          <button
            v-if="draft.id"
            type="button"
            class="danger"
            :disabled="deleting"
            @click="removeProfile"
          >
            删除连接
          </button>
          <span></span>
          <button type="button" @click="closeModal">取消</button>
          <button
            type="button"
            class="primary"
            :disabled="saving || !canSaveModal"
            @click="saveProfile"
          >
            <span v-if="saving" class="spinner"></span>
            {{ saving ? "保存中" : "保存并使用" }}
          </button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.ssh-page {
  display: grid;
  min-height: calc(100vh - 112px);
  grid-template-columns: 250px minmax(0, 1fr);
}

.ssh-page.list-collapsed {
  grid-template-columns: 0 minmax(0, 1fr);
}

.ssh-profile-list {
  position: relative;
  display: flex;
  min-height: 0;
  flex-direction: column;
  border-right: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
  transform: translate3d(0, 0, 0);
  transition:
    transform 150ms cubic-bezier(0.22, 1, 0.36, 1),
    opacity 120ms ease-out;
  will-change: transform, opacity;
}

.ssh-page.list-opening .ssh-profile-list,
.ssh-page.list-closing .ssh-profile-list {
  opacity: 0;
  transform: translate3d(-100%, 0, 0);
}

.ssh-page.list-collapsed .ssh-profile-list {
  overflow: hidden;
  border-right: 0;
  opacity: 0;
  pointer-events: none;
  visibility: hidden;
}

.ssh-panel-heading,
.ssh-card-heading {
  display: flex;
  min-height: 62px;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--color-border);
}

.ssh-panel-heading p,
.ssh-card-heading p,
.ssh-fingerprint-card p {
  margin: 0 0 4px;
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.12em;
}

.ssh-panel-heading h2,
.ssh-card-heading h2 {
  margin: 0;
  font-size: 13px;
}

.ssh-panel-actions {
  display: flex;
  align-items: center;
  gap: 5px;
}

.ssh-panel-actions button,
.ssh-list-restore {
  display: grid;
  width: 26px;
  height: 26px;
  padding: 0;
  place-items: center;
  border: 1px solid var(--color-border-strong);
  background: var(--color-bg-elevated);
  color: var(--color-text-secondary);
  cursor: pointer;
}

.ssh-panel-actions button:hover,
.ssh-list-restore:hover {
  border-color: var(--color-accent);
  color: var(--color-accent);
}

.ssh-panel-actions button:disabled,
.ssh-list-restore:disabled {
  cursor: default;
  opacity: 0.5;
}

.ssh-panel-actions svg,
.ssh-list-restore svg {
  width: 13px;
  height: 13px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.5;
}

.ssh-profile-item {
  position: relative;
  display: grid;
  grid-template-columns: 34px minmax(0, 1fr) 7px;
  align-items: center;
  gap: 10px;
  min-height: 64px;
  padding: 10px 14px;
  border: 0;
  border-bottom: 1px solid var(--color-border);
  background: transparent;
  color: var(--color-text-primary);
  text-align: left;
  cursor: pointer;
}

.ssh-profile-item:hover {
  background: var(--color-hover);
}

.ssh-profile-item.selected {
  background: var(--color-selected);
}

.ssh-profile-item.selected::before {
  position: absolute;
  inset: 0 auto 0 0;
  width: 3px;
  background: var(--color-accent);
  content: "";
}

.ssh-profile-item > span {
  display: grid;
  width: 34px;
  height: 34px;
  place-items: center;
  border-radius: 9px;
  background: #36473f;
  color: #d9e6dd;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 12px;
  font-weight: 700;
}

.ssh-profile-item strong,
.ssh-profile-item small {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ssh-profile-item strong {
  font-size: 10px;
}

.ssh-profile-item small {
  margin-top: 4px;
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
}

.ssh-profile-item > i {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--color-text-muted);
}

.ssh-list-empty {
  display: grid;
  min-height: 180px;
  place-items: center;
  align-content: center;
  gap: 6px;
  color: var(--color-text-muted);
  text-align: center;
  font-size: 9px;
}

.ssh-list-empty > span {
  color: var(--color-accent);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 24px;
}

.ssh-list-empty small {
  font-size: 8px;
}

.ssh-local-note {
  display: grid;
  gap: 5px;
  margin-top: auto;
  padding: 15px;
  border-top: 1px solid var(--color-border);
  color: var(--color-text-secondary);
}

.ssh-local-note strong {
  font-size: 9px;
}

.ssh-local-note small {
  color: var(--color-text-muted);
  font-size: 8px;
  line-height: 1.55;
}

.ssh-workspace {
  position: relative;
  display: grid;
  align-content: start;
  gap: 14px;
  min-width: 0;
  padding: 22px 26px 34px;
  container-type: inline-size;
}

.ssh-list-restore {
  position: absolute;
  z-index: 3;
  top: 22px;
  left: 0;
  width: 21px;
  height: 28px;
  border-left: 0;
  border-radius: 0 5px 5px 0;
  background: var(--color-bg-panel);
}

.ssh-config-card,
.ssh-terminal-card,
.ssh-fingerprint-card,
.ssh-active-card,
.ssh-active-empty {
  border: 1px solid var(--color-border);
  background: var(--color-panel-translucent);
}

.ssh-active-card {
  display: grid;
  grid-template-columns: minmax(160px, 1fr) minmax(90px, auto) auto;
  align-items: center;
  gap: 12px;
  min-height: 64px;
  padding: 9px 11px;
}

.ssh-active-identity {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 12px;
}

.ssh-active-identity > span {
  display: grid;
  width: 38px;
  height: 38px;
  flex: 0 0 auto;
  place-items: center;
  border-radius: 10px;
  background: #36473f;
  color: #d9e7de;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 14px;
  font-weight: 700;
}

.ssh-active-identity p {
  margin: 0 0 3px;
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 7px;
  letter-spacing: 0.12em;
}

.ssh-active-identity h2 {
  margin: 0;
  font-size: 12px;
}

.ssh-active-identity small {
  display: block;
  overflow: hidden;
  margin-top: 4px;
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ssh-active-auth {
  display: grid;
  min-width: 90px;
  gap: 4px;
  padding-left: 12px;
  border-left: 1px solid var(--color-border);
}

.ssh-active-auth > span {
  color: var(--color-text-secondary);
  font-size: 9px;
  font-weight: 600;
}

.ssh-active-auth small {
  color: var(--color-text-muted);
  font-size: 8px;
}

.ssh-active-actions {
  display: flex;
  flex: 0 0 auto;
  gap: 6px;
}

.ssh-active-actions button {
  display: inline-flex;
  width: 30px;
  height: 30px;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: 1px solid var(--color-border-strong);
  background: var(--color-bg-panel);
  color: var(--color-text-secondary);
  cursor: pointer;
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 12px;
}

.ssh-active-actions button:hover {
  border-color: var(--color-accent);
  color: var(--color-accent);
}

.ssh-active-empty > button {
  display: inline-flex;
  min-width: 68px;
  align-items: center;
  justify-content: center;
  gap: 5px;
  padding: 7px 10px;
  border: 1px solid var(--color-border-strong);
  background: var(--color-bg-panel);
  cursor: pointer;
  font-size: 8px;
}

.ssh-active-actions button:disabled {
  cursor: default;
  opacity: 0.55;
}

.ssh-active-empty {
  display: flex;
  min-height: 90px;
  align-items: center;
  justify-content: space-between;
  padding: 16px 18px;
}

.ssh-active-empty > div {
  display: grid;
  grid-template-columns: 38px auto;
  gap: 4px 12px;
  align-items: center;
}

.ssh-active-empty > div > span {
  grid-row: 1 / 3;
  color: var(--color-accent);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 20px;
}

.ssh-active-empty strong {
  font-size: 10px;
}

.ssh-active-empty small {
  color: var(--color-text-muted);
  font-size: 8px;
}

.ssh-card-actions {
  display: flex;
  gap: 7px;
}

.ssh-card-actions button,
.ssh-connection-actions button,
.ssh-fingerprint-card > button {
  display: inline-flex;
  min-width: 80px;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 7px 11px;
  border: 1px solid var(--color-border-strong);
  background: var(--color-bg-panel);
  cursor: pointer;
  font-size: 9px;
}

.ssh-card-actions button:disabled,
.ssh-connection-actions button:disabled,
.ssh-fingerprint-card > button:disabled {
  cursor: default;
  opacity: 0.55;
}

.ssh-card-actions .primary,
.ssh-fingerprint-card > button {
  border-color: var(--color-control-primary);
  background: var(--color-control-primary);
  color: #f7f7f2;
}

.ssh-card-actions .danger {
  color: var(--color-danger-text);
}

.ssh-form {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 112px minmax(150px, 0.65fr);
  gap: 14px;
  padding: 16px;
}

.ssh-form label {
  display: grid;
  min-width: 0;
  gap: 7px;
}

.ssh-form label.wide {
  grid-column: 1 / -1;
}

.ssh-form label.host {
  grid-column: 1;
}

.ssh-form label.port {
  grid-column: 2;
}

.ssh-form label > span {
  color: var(--color-text-secondary);
  font-size: 9px;
  font-weight: 600;
}

.ssh-form label > span small {
  margin-left: 6px;
  color: var(--color-text-muted);
  font-size: 8px;
  font-weight: 400;
}

.ssh-form input {
  min-width: 0;
  height: 34px;
  padding: 0 10px;
  border: 1px solid var(--color-border);
  outline: 0;
  background: var(--color-bg-elevated);
  color: var(--color-text-primary);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 9px;
}

.ssh-form input:focus {
  border-color: var(--color-accent);
  box-shadow: 0 0 0 2px var(--color-focus);
}

.ssh-key-field {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 70px 34px;
}

.ssh-key-field input {
  border-right: 0;
}

.ssh-key-field button {
  border: 1px solid var(--color-border);
  background: var(--color-bg-panel);
  cursor: pointer;
  font-size: 9px;
}

.ssh-key-field button + button {
  border-left: 0;
}

.ssh-password-field {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 64px;
}

.ssh-password-field input {
  border-right: 0;
}

.ssh-password-field button {
  border: 1px solid var(--color-border);
  background: var(--color-bg-panel);
  cursor: pointer;
  font-size: 8px;
}

.ssh-password-note {
  color: var(--color-warning-text);
  font-size: 8px;
  line-height: 1.5;
}

.ssh-connection-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 20px;
  padding: 12px 16px;
  border-top: 1px solid var(--color-border);
}

.ssh-connection-actions > div {
  display: flex;
  gap: 7px;
}

.ssh-connection-actions p {
  margin: 0;
  color: var(--color-text-muted);
  font-size: 8px;
}

.ssh-fingerprint-card {
  display: grid;
  grid-template-columns: 44px minmax(0, 1fr) auto;
  align-items: center;
  gap: 14px;
  padding: 14px 16px;
  border-color: var(--color-warning);
  background: var(--color-warning-surface);
}

.ssh-fingerprint-icon {
  display: grid;
  width: 42px;
  height: 42px;
  place-items: center;
  border: 1px solid var(--color-warning);
  border-radius: 50%;
  color: var(--color-warning-text);
  font-size: 20px;
}

.ssh-fingerprint-card h3 {
  margin: 0 0 8px;
  font-size: 10px;
}

.ssh-fingerprint-card code {
  display: block;
  overflow: hidden;
  color: var(--color-warning-text);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 10px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ssh-fingerprint-card small {
  display: block;
  margin-top: 5px;
  color: var(--color-text-muted);
  font-size: 8px;
}

.ssh-terminal-card {
  min-width: 0;
  overflow: hidden;
}

.ssh-terminal-toolbar {
  display: grid;
  min-height: 58px;
  grid-template-columns: minmax(150px, 1fr) minmax(0, auto) auto auto;
  align-items: center;
  gap: 14px;
  padding: 9px 12px;
  border-bottom: 1px solid var(--color-border);
}

.ssh-terminal-heading {
  display: flex;
  min-width: 0;
  align-items: center;
  gap: 10px;
}

.ssh-terminal-heading > i {
  width: 8px;
  height: 8px;
  flex: 0 0 auto;
  border-radius: 50%;
  background: var(--color-text-muted);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--color-text-muted) 14%, transparent);
}

.ssh-terminal-heading > i.connecting {
  background: var(--color-warning);
}

.ssh-terminal-heading > i.connected {
  background: var(--color-success);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--color-success) 16%, transparent);
}

.ssh-terminal-heading p {
  margin: 0 0 3px;
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 7px;
  letter-spacing: 0.12em;
}

.ssh-terminal-heading h2 {
  margin: 0;
  font-size: 12px;
}

.ssh-terminal-toolbar > code,
.ssh-terminal-toolbar > span {
  overflow: hidden;
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ssh-idle-timeout {
  display: inline-flex;
  height: 28px;
  align-items: center;
  gap: 6px;
  color: var(--color-text-muted);
  font-size: 8px;
  white-space: nowrap;
}

.ssh-idle-timeout select {
  height: 28px;
  padding: 0 22px 0 8px;
  border: 1px solid var(--color-border);
  outline: 0;
  background: var(--color-bg-elevated);
  color: var(--color-text-secondary);
  cursor: pointer;
  font-size: 8px;
}

.ssh-idle-timeout select:focus {
  border-color: var(--color-accent);
  box-shadow: 0 0 0 2px var(--color-focus);
}

.ssh-terminal-toolbar > button {
  min-width: 70px;
  height: 30px;
  padding: 0 11px;
  border: 1px solid var(--color-border-strong);
  background: var(--color-bg-panel);
  color: var(--color-text-primary);
  cursor: pointer;
  font-size: 8px;
}

.ssh-terminal-toolbar > button.primary {
  border-color: #4d7658;
  background: #3f6349;
  color: #f0f4ee;
}

.ssh-terminal-toolbar > button.disconnect {
  border-color: var(--color-danger);
  color: var(--color-danger-text);
}

.ssh-terminal-toolbar > button:disabled {
  cursor: default;
  opacity: 0.52;
}

.ssh-terminal-host {
  height: clamp(320px, calc(100vh - 390px), 610px);
  min-height: 320px;
  padding: 10px 8px 8px 12px;
  background: #171b17;
}

.ssh-terminal-host :deep(.xterm) {
  height: 100%;
}

.ssh-terminal-host :deep(.xterm-viewport) {
  scrollbar-color: #4c554b #171b17;
  scrollbar-width: thin;
}

.ssh-terminal-card > footer {
  display: flex;
  min-height: 30px;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  padding: 6px 12px;
  border-top: 1px solid #30362f;
  background: #171b17;
  color: #7e887d;
  font-size: 7px;
}

.ssh-modal-backdrop {
  position: fixed;
  z-index: 10000;
  inset: 0;
  display: grid;
  padding: 28px;
  place-items: center;
  background: rgba(13, 16, 13, 0.58);
  backdrop-filter: blur(7px);
}

.ssh-profile-modal {
  width: min(700px, calc(100vw - 56px));
  max-height: calc(100vh - 56px);
  overflow: auto;
  border: 1px solid var(--color-border-strong);
  border-radius: 12px;
  background: var(--color-bg-panel);
  color: var(--color-text-primary);
  box-shadow: 0 28px 80px rgba(8, 12, 9, 0.34);
}

.ssh-profile-modal > header {
  display: flex;
  min-height: 72px;
  align-items: center;
  justify-content: space-between;
  padding: 15px 18px;
  border-bottom: 1px solid var(--color-border);
}

.ssh-profile-modal > header p {
  margin: 0 0 5px;
  color: var(--color-text-muted);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 8px;
  letter-spacing: 0.14em;
}

.ssh-profile-modal > header h2 {
  margin: 0;
  font-size: 16px;
}

.ssh-profile-modal > header > button {
  display: grid;
  width: 30px;
  height: 30px;
  place-items: center;
  border: 1px solid var(--color-border);
  border-radius: 50%;
  background: var(--color-bg-muted);
  color: var(--color-text-secondary);
  cursor: pointer;
}

.ssh-auth-options {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
  padding: 16px 16px 0;
}

.ssh-modal-error {
  margin: 14px 16px 0;
  padding: 9px 11px;
  border: 1px solid var(--color-danger);
  border-radius: 6px;
  background: var(--color-danger-surface);
  color: var(--color-danger-text);
  font-size: 9px;
  line-height: 1.5;
}

.ssh-auth-options > button {
  display: grid;
  min-width: 0;
  grid-template-columns: 36px minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  padding: 10px;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-bg-elevated);
  color: var(--color-text-primary);
  text-align: left;
  cursor: pointer;
}

.ssh-auth-options > button.selected {
  border-color: var(--color-accent);
  box-shadow: 0 0 0 2px var(--color-focus);
}

.ssh-auth-options > button > span {
  display: grid;
  width: 36px;
  height: 34px;
  place-items: center;
  border-radius: 7px;
  background: var(--color-bg-muted);
  color: var(--color-accent);
  font-family: "SFMono-Regular", Consolas, monospace;
  font-size: 13px;
  font-weight: 700;
}

.ssh-auth-options strong,
.ssh-auth-options small {
  display: block;
}

.ssh-auth-options strong {
  font-size: 10px;
}

.ssh-auth-options small {
  margin-top: 4px;
  color: var(--color-text-muted);
  font-size: 8px;
}

.ssh-profile-modal .modal-form {
  padding: 16px;
}

.ssh-profile-modal > footer {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 13px 16px;
  border-top: 1px solid var(--color-border);
  background: var(--color-bg-muted);
}

.ssh-profile-modal > footer > span {
  flex: 1;
}

.ssh-profile-modal > footer button {
  display: inline-flex;
  min-width: 78px;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 8px 12px;
  border: 1px solid var(--color-border-strong);
  background: var(--color-bg-panel);
  color: var(--color-text-primary);
  cursor: pointer;
  font-size: 9px;
}

.ssh-profile-modal > footer button.primary,
.ssh-active-empty > button.primary {
  border-color: var(--color-control-primary);
  background: var(--color-control-primary);
  color: #f7f7f2;
}

.ssh-profile-modal > footer button.danger {
  color: var(--color-danger-text);
}

.ssh-profile-modal > footer button:disabled {
  cursor: default;
  opacity: 0.55;
}

@media (max-width: 980px) {
  .ssh-page {
    grid-template-columns: 210px minmax(0, 1fr);
  }

  .ssh-form {
    grid-template-columns: minmax(0, 1fr) 90px;
  }

  .ssh-form label:not(.host, .port, .wide) {
    grid-column: 1 / -1;
  }

  .ssh-fingerprint-card {
    grid-template-columns: 38px minmax(0, 1fr);
  }

  .ssh-fingerprint-card > button {
    grid-column: 2;
    justify-self: start;
  }

  .ssh-active-card {
    grid-template-columns: minmax(180px, 1fr) auto;
  }

  .ssh-active-auth {
    display: none;
  }
}

@container (max-width: 650px) {
  .ssh-active-card {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .ssh-active-auth {
    display: none;
  }

  .ssh-terminal-toolbar {
    grid-template-columns: minmax(0, 1fr) auto auto;
  }

  .ssh-terminal-toolbar > code,
  .ssh-terminal-toolbar > span {
    display: none;
  }

  .ssh-terminal-card > footer {
    justify-content: flex-start;
  }

  .ssh-terminal-card > footer span + span {
    display: none;
  }
}
</style>
