<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

export interface CommandPaletteItem {
  id: string;
  label: string;
  hint: string;
  group: string;
  icon: string;
  keywords?: string;
  shortcut?: string;
  danger?: boolean;
}

const props = defineProps<{
  open: boolean;
  items: CommandPaletteItem[];
  busyId?: string;
}>();
const emit = defineEmits<{
  close: [];
  select: [item: CommandPaletteItem];
}>();
const { t } = useI18n();
const query = ref("");
const activeIndex = ref(0);
const input = ref<HTMLInputElement | null>(null);

const filteredItems = computed(() => {
  const terms = query.value
    .trim()
    .toLocaleLowerCase()
    .split(/\s+/)
    .filter(Boolean);
  if (!terms.length) return props.items;
  return props.items.filter((item) => {
    const haystack =
      `${item.label} ${item.hint} ${item.group} ${item.keywords ?? ""}`.toLocaleLowerCase();
    return terms.every((term) => haystack.includes(term));
  });
});

const groupedItems = computed(() => {
  const groups: Array<{ name: string; items: CommandPaletteItem[] }> = [];
  for (const item of filteredItems.value) {
    const current = groups.at(-1);
    if (current?.name === item.group) current.items.push(item);
    else groups.push({ name: item.group, items: [item] });
  }
  return groups;
});

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    query.value = "";
    activeIndex.value = 0;
    await nextTick();
    input.value?.focus();
  },
);
watch(filteredItems, () => {
  activeIndex.value = Math.min(activeIndex.value, filteredItems.value.length - 1);
  if (activeIndex.value < 0) activeIndex.value = 0;
});

function move(offset: number) {
  const length = filteredItems.value.length;
  if (!length) return;
  activeIndex.value = (activeIndex.value + offset + length) % length;
  nextTick(() =>
    document
      .querySelector(`[data-command-index="${activeIndex.value}"]`)
      ?.scrollIntoView({ block: "nearest" }),
  );
}

function select(item = filteredItems.value[activeIndex.value]) {
  if (!item || props.busyId) return;
  emit("select", item);
}

function flatIndex(item: CommandPaletteItem) {
  return filteredItems.value.findIndex((candidate) => candidate.id === item.id);
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="command-backdrop"
      role="dialog"
      aria-modal="true"
      :aria-label="t('commandPalette.title')"
      @mousedown.self="emit('close')"
      @keydown.esc.prevent="emit('close')"
      @keydown.down.prevent="move(1)"
      @keydown.up.prevent="move(-1)"
      @keydown.enter.prevent="select()"
    >
      <section class="command-palette">
        <div class="command-search">
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <circle cx="10.8" cy="10.8" r="6.4" />
            <path d="m15.7 15.7 4.1 4.1" />
          </svg>
          <input
            ref="input"
            v-model="query"
            :placeholder="t('commandPalette.placeholder')"
            autocomplete="off"
            spellcheck="false"
          />
          <kbd>ESC</kbd>
        </div>
        <div class="command-results">
          <template v-if="filteredItems.length">
            <div v-for="group in groupedItems" :key="group.name" class="command-group">
              <p>{{ group.name }}</p>
              <button
                v-for="item in group.items"
                :key="item.id"
                type="button"
                :data-command-index="flatIndex(item)"
                :class="{
                  active: activeIndex === flatIndex(item),
                  danger: item.danger,
                }"
                @mouseenter="activeIndex = flatIndex(item)"
                @click="select(item)"
              >
                <i>{{ item.icon }}</i>
                <span><strong>{{ item.label }}</strong><small>{{ item.hint }}</small></span>
                <em v-if="busyId === item.id">{{ t("common.loading") }}…</em>
                <kbd v-else-if="item.shortcut">{{ item.shortcut }}</kbd>
                <b v-else>↵</b>
              </button>
            </div>
          </template>
          <div v-else class="command-empty">
            <span>⌘</span>
            <strong>{{ t("commandPalette.noResults") }}</strong>
            <small>{{ t("commandPalette.noResultsHint") }}</small>
          </div>
        </div>
        <footer>
          <span><kbd>↑</kbd><kbd>↓</kbd> {{ t("commandPalette.navigate") }}</span>
          <span><kbd>↵</kbd> {{ t("commandPalette.run") }}</span>
          <span><kbd>esc</kbd> {{ t("commandPalette.close") }}</span>
          <em>{{ filteredItems.length }} {{ t("commandPalette.commands") }}</em>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.command-backdrop{position:fixed;z-index:220;inset:0;display:flex;justify-content:center;align-items:flex-start;padding:11vh 24px 24px;background:rgba(5,8,12,.48);backdrop-filter:blur(7px)}.command-palette{width:min(680px,calc(100vw - 48px));overflow:hidden;border:1px solid var(--color-border-strong);background:var(--color-panel);box-shadow:0 30px 100px rgba(0,0,0,.42)}.command-search{display:grid;grid-template-columns:24px minmax(0,1fr) auto;align-items:center;gap:10px;padding:12px 14px;border-bottom:1px solid var(--color-border)}.command-search svg{width:19px;fill:none;stroke:var(--color-text-muted);stroke-width:1.6}.command-search input{height:34px;padding:0;border:0!important;outline:0!important;background:transparent!important;box-shadow:none!important;color:var(--color-text-primary);font-size:13px}.command-search kbd,.command-palette footer kbd,.command-group button kbd{padding:3px 5px;border:1px solid var(--color-border);border-bottom-color:var(--color-border-strong);background:var(--color-bg-muted);color:var(--color-text-muted);font:7px "SFMono-Regular",Consolas,monospace}.command-results{max-height:min(58vh,520px);overflow:auto;padding:7px}.command-group>p{margin:0;padding:8px 9px 5px;color:var(--color-text-muted);font:7px "SFMono-Regular",Consolas,monospace;letter-spacing:.13em;text-transform:uppercase}.command-group button{display:grid;width:100%;grid-template-columns:32px minmax(0,1fr) auto;align-items:center;gap:10px;padding:8px 9px;border:0;background:transparent;color:var(--color-text-primary);text-align:left}.command-group button.active{background:var(--color-panel-active);box-shadow:inset 2px 0 var(--color-accent)}.command-group button.danger strong{color:var(--color-danger-text)}.command-group button>i{display:grid;width:29px;height:29px;place-items:center;border:1px solid var(--color-border);border-radius:50%;background:var(--color-bg-muted);color:var(--color-accent);font:normal 9px "SFMono-Regular",Consolas,monospace}.command-group button>span{display:grid;min-width:0;gap:3px}.command-group button strong,.command-group button small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.command-group button strong{font-size:10px}.command-group button small{color:var(--color-text-muted);font-size:8px}.command-group button>b,.command-group button>em{color:var(--color-text-muted);font:normal 8px "SFMono-Regular",Consolas,monospace}.command-empty{display:grid;min-height:230px;place-items:center;align-content:center;gap:7px;color:var(--color-text-muted);text-align:center}.command-empty>span{display:grid;width:42px;height:42px;place-items:center;border:1px solid var(--color-border);border-radius:50%;color:var(--color-accent)}.command-empty strong{color:var(--color-text-primary);font-size:11px}.command-empty small{font-size:8px}.command-palette footer{display:flex;align-items:center;gap:14px;padding:8px 14px;border-top:1px solid var(--color-border);background:var(--color-bg-muted);color:var(--color-text-muted);font-size:7px}.command-palette footer span{display:flex;align-items:center;gap:4px}.command-palette footer em{margin-left:auto;font:normal 7px "SFMono-Regular",Consolas,monospace}
</style>
