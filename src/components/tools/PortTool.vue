<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { listPortListeners } from "../../api/services";
import type { PortListener } from "../../types";

const REFRESH_INTERVAL_MS = 2000;
const ALL_INTERFACE_ADDRESSES = ["*", "0.0.0.0", "[::]"];

const listeners = ref<PortListener[]>([]);
const query = ref("");
const loading = ref(false);
const error = ref("");

let timer: number | undefined;

const filteredListeners = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  if (!keyword) return listeners.value;
  return listeners.value.filter((listener) =>
    [
      listener.port,
      listener.address,
      listener.pid,
      listener.process,
      listener.managedService,
      listener.commonService,
    ]
      .filter((value) => value !== null)
      .some((value) => String(value).toLowerCase().includes(keyword)),
  );
});

const processCount = computed(
  () => new Set(listeners.value.map((listener) => listener.pid)).size,
);

const publicCount = computed(
  () =>
    listeners.value.filter((listener) =>
      ALL_INTERFACE_ADDRESSES.includes(listener.address),
    ).length,
);

const managedCount = computed(
  () => listeners.value.filter((listener) => listener.managedService).length,
);

function isPublic(address: string): boolean {
  return ALL_INTERFACE_ADDRESSES.includes(address);
}

async function load(silent = false) {
  if (loading.value) return;
  loading.value = true;
  try {
    listeners.value = await listPortListeners();
    error.value = "";
  } catch (cause) {
    // 静默轮询失败不打断界面，手动刷新失败才提示
    if (!silent) error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

onMounted(async () => {
  await load();
  timer = window.setInterval(() => void load(true), REFRESH_INTERVAL_MS);
});

onUnmounted(() => {
  if (timer) window.clearInterval(timer);
});
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo ports">↔</span>
      <div>
        <div class="title-line">
          <h1>端口检查器</h1>
          <span>TCP LISTEN</span>
        </div>
        <p>查看本机正在监听的 TCP 端口，不修改任何进程</p>
      </div>
    </div>
    <div class="header-actions">
      <button
        class="primary"
        type="button"
        :disabled="loading"
        @click="load()"
      >
        <span v-if="loading" class="spinner"></span>
        {{ loading ? "检查中" : "重新检查" }}
      </button>
    </div>
  </header>

  <div v-if="error" class="notice danger">
    <span>{{ error }}</span>
    <button type="button" @click="error = ''">×</button>
  </div>

  <section class="port-tool-page">
    <div class="metric-grid">
      <article class="metric-card">
        <p>LISTENERS</p>
        <strong>{{ listeners.length }}</strong>
        <small>正在监听的地址</small>
      </article>
      <article class="metric-card">
        <p>PROCESSES</p>
        <strong>{{ processCount }}</strong>
        <small>占用端口的进程</small>
      </article>
      <article class="metric-card">
        <p>ZHIYU</p>
        <strong>{{ managedCount }}</strong>
        <small>智屿管理的监听地址</small>
      </article>
      <article class="metric-card">
        <p>ALL INTERFACES</p>
        <strong>{{ publicCount }}</strong>
        <small>监听全部网络接口</small>
      </article>
    </div>

    <div class="port-panel">
      <div class="port-toolbar">
        <div>
          <p>LOCAL PORTS</p>
          <h2>监听端口</h2>
        </div>
        <label>
          筛选
          <input v-model="query" type="search" placeholder="端口、进程或服务" />
        </label>
      </div>

      <div v-if="loading && listeners.length === 0" class="port-empty">
        正在读取本机端口…
      </div>
      <div v-else-if="filteredListeners.length === 0" class="port-empty">
        {{ query ? "没有匹配的监听端口" : "当前没有 TCP 监听端口" }}
      </div>
      <div v-else class="port-table-wrap">
        <table class="port-table">
          <thead>
            <tr>
              <th>端口</th>
              <th>监听地址</th>
              <th>进程</th>
              <th>PID</th>
              <th>归属</th>
              <th>常见用途</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="listener in filteredListeners"
              :key="`${listener.pid}-${listener.address}-${listener.port}`"
            >
              <td><code>{{ listener.port }}</code></td>
              <td>
                <code>{{ listener.address }}:{{ listener.port }}</code>
                <span v-if="isPublic(listener.address)" class="network-badge">
                  全部网卡
                </span>
              </td>
              <td>{{ listener.process || "未知进程" }}</td>
              <td><code>{{ listener.pid }}</code></td>
              <td>
                <span
                  v-if="listener.managedService"
                  class="ownership-badge managed"
                >
                  智屿 · {{ listener.managedService }}
                </span>
                <span v-else class="ownership-badge">外部进程</span>
              </td>
              <td>{{ listener.commonService ?? "—" }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <p class="port-note">
        这里只显示 TCP 监听端口。监听 <code>127.0.0.1</code> 或
        <code>::1</code> 的服务仅供本机访问；“全部网卡”表示局域网设备也可能连接。
      </p>
    </div>
  </section>
</template>
