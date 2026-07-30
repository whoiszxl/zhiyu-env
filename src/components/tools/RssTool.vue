<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, writeTextFile } from "@tauri-apps/plugin-fs";
import { useI18n } from "vue-i18n";
import {
  rssEntriesList,
  rssEntryRead,
  rssEntryStar,
  rssExportOpml,
  rssFeedAdd,
  rssFeedDelete,
  rssFeedRefresh,
  rssFeedsList,
  rssFeedUpdate,
  rssImportOpml,
  rssMarkAllRead,
} from "../../api/services";
import {
  RSS_RECOMMENDATIONS,
  type RssRecommendation,
  type RssRecommendationCategory,
} from "../../data/rssRecommendations";
import type { RssEntry, RssFeed } from "../../types";
import RssAiPanel from "./RssAiPanel.vue";

type EntryFilter = "all" | "unread" | "starred";
type RecommendationFilter = "all" | RssRecommendationCategory;

const { t, locale } = useI18n();
const feeds = ref<RssFeed[]>([]);
const entries = ref<RssEntry[]>([]);
const selectedFeedId = ref<number | null>(null);
const selectedEntryId = ref<number | null>(null);
const filter = ref<EntryFilter>("all");
const search = ref("");
const loading = ref(true);
const refreshing = ref(false);
const notice = ref("");
const error = ref("");
const addOpen = ref(false);
const settingsOpen = ref(false);
const newFeedUrl = ref("");
const recommendationFilter = ref<RecommendationFilter>("all");
const recommendationSearch = ref("");
const adding = ref(false);
const readingMode = ref(false);
const aiReaderOpen = ref(false);
const editTitle = ref("");
const editInterval = ref(30);
const editEnabled = ref(true);
let searchTimer: ReturnType<typeof setTimeout> | null = null;
let syncTimer: ReturnType<typeof setInterval> | null = null;

const selectedFeed = computed(
  () => feeds.value.find((feed) => feed.id === selectedFeedId.value) ?? null,
);
const selectedEntry = computed(
  () => entries.value.find((entry) => entry.id === selectedEntryId.value) ?? null,
);
const selectedContent = computed(() => {
  const entry = selectedEntry.value;
  return decodeLegacyEntities(
    entry?.content || entry?.summary || t("rss.noContent"),
  );
});
const selectedEntryIndex = computed(() =>
  entries.value.findIndex((entry) => entry.id === selectedEntryId.value),
);
const hasPreviousEntry = computed(() => selectedEntryIndex.value > 0);
const hasNextEntry = computed(
  () =>
    selectedEntryIndex.value >= 0 &&
    selectedEntryIndex.value < entries.value.length - 1,
);
const totalUnread = computed(() =>
  feeds.value.reduce((sum, feed) => sum + feed.unreadCount, 0),
);
const recommendations = computed(() => {
  const query = recommendationSearch.value.trim().toLocaleLowerCase();
  const subscribed = new Set(feeds.value.map((feed) => feed.feedUrl));
  return RSS_RECOMMENDATIONS.filter((feed) => {
    if (recommendationFilter.value !== "all" && feed.category !== recommendationFilter.value) {
      return false;
    }
    if (query && !`${feed.name} ${feed.url}`.toLocaleLowerCase().includes(query)) {
      return false;
    }
    return !subscribed.has(feed.url);
  });
});

function clearMessages() {
  notice.value = "";
  error.value = "";
}

function chooseRecommendation(feed: RssRecommendation) {
  newFeedUrl.value = feed.url;
}

function decodeLegacyEntities(value: string) {
  return value
    .replaceAll("&#x27;", "'")
    .replaceAll("&#X27;", "'")
    .replaceAll("&#39;", "'")
    .replaceAll("&apos;", "'")
    .replaceAll("&quot;", '"')
    .replaceAll("&amp;", "&");
}

async function loadFeeds() {
  feeds.value = await rssFeedsList();
  if (
    selectedFeedId.value !== null &&
    !feeds.value.some((feed) => feed.id === selectedFeedId.value)
  ) {
    selectedFeedId.value = null;
  }
}

async function loadEntries() {
  entries.value = await rssEntriesList(
    selectedFeedId.value ?? undefined,
    filter.value,
    search.value.trim() || undefined,
  );
  if (
    selectedEntryId.value !== null &&
    !entries.value.some((entry) => entry.id === selectedEntryId.value)
  ) {
    selectedEntryId.value = null;
  }
}

async function loadAll() {
  loading.value = true;
  try {
    await loadFeeds();
    await loadEntries();
    clearMessages();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
}

function chooseFeed(id: number | null) {
  selectedFeedId.value = id;
  selectedEntryId.value = null;
  readingMode.value = false;
}

async function chooseEntry(entry: RssEntry) {
  selectedEntryId.value = entry.id;
  readingMode.value = true;
  if (!entry.isRead) {
    entry.isRead = true;
    await rssEntryRead(entry.id, true);
    const feed = feeds.value.find((item) => item.id === entry.feedId);
    if (feed && feed.unreadCount > 0) feed.unreadCount -= 1;
  }
}

function closeReading() {
  aiReaderOpen.value = false;
  readingMode.value = false;
}

async function navigateEntry(direction: -1 | 1) {
  const target = entries.value[selectedEntryIndex.value + direction];
  if (target) await chooseEntry(target);
}

function handleReadingShortcut(event: KeyboardEvent) {
  if (!readingMode.value || !selectedEntry.value) return;
  if (
    event.target instanceof HTMLInputElement ||
    event.target instanceof HTMLTextAreaElement ||
    event.target instanceof HTMLSelectElement
  ) {
    return;
  }
  if (event.key === "Escape") {
    closeReading();
  } else if (event.key === "j" || event.key === "ArrowDown") {
    event.preventDefault();
    navigateEntry(1);
  } else if (event.key === "k" || event.key === "ArrowUp") {
    event.preventDefault();
    navigateEntry(-1);
  }
}

async function addFeed() {
  if (!newFeedUrl.value.trim() || adding.value) return;
  adding.value = true;
  try {
    const result = await rssFeedAdd(newFeedUrl.value);
    await loadFeeds();
    selectedFeedId.value = result.feedId;
    await loadEntries();
    newFeedUrl.value = "";
    recommendationSearch.value = "";
    addOpen.value = false;
    notice.value = t("rss.added", { count: result.added });
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  } finally {
    adding.value = false;
  }
}

async function refreshFeeds() {
  if (refreshing.value || feeds.value.length === 0) return;
  refreshing.value = true;
  let added = 0;
  const targets = selectedFeed.value ? [selectedFeed.value] : feeds.value.filter((feed) => feed.enabled);
  try {
    for (const feed of targets) {
      try {
        const result = await rssFeedRefresh(feed.id);
        added += result.added;
      } catch (cause) {
        error.value = String(cause);
      }
    }
    await loadFeeds();
    await loadEntries();
    if (!error.value) notice.value = t("rss.refreshed", { count: added });
  } finally {
    refreshing.value = false;
  }
}

async function toggleStar(entry: RssEntry) {
  entry.isStarred = !entry.isStarred;
  try {
    await rssEntryStar(entry.id, entry.isStarred);
    if (filter.value === "starred" && !entry.isStarred) await loadEntries();
  } catch (cause) {
    entry.isStarred = !entry.isStarred;
    error.value = String(cause);
  }
}

async function toggleRead(entry: RssEntry) {
  entry.isRead = !entry.isRead;
  try {
    await rssEntryRead(entry.id, entry.isRead);
    await loadFeeds();
    if (filter.value === "unread" && entry.isRead) await loadEntries();
  } catch (cause) {
    entry.isRead = !entry.isRead;
    error.value = String(cause);
  }
}

async function markAllRead() {
  try {
    await rssMarkAllRead(selectedFeedId.value ?? undefined);
    await Promise.all([loadFeeds(), loadEntries()]);
    notice.value = t("rss.markedRead");
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  }
}

function openSettings() {
  if (!selectedFeed.value) return;
  editTitle.value = selectedFeed.value.title;
  editInterval.value = selectedFeed.value.refreshIntervalMinutes;
  editEnabled.value = selectedFeed.value.enabled;
  settingsOpen.value = true;
}

async function saveFeedSettings() {
  if (!selectedFeed.value) return;
  try {
    await rssFeedUpdate(selectedFeed.value.id, {
      title: editTitle.value,
      refreshIntervalMinutes: editInterval.value,
      enabled: editEnabled.value,
    });
    settingsOpen.value = false;
    await loadFeeds();
    notice.value = t("rss.settingsSaved");
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  }
}

async function deleteFeed() {
  if (!selectedFeed.value || !confirm(t("rss.deleteConfirm"))) return;
  try {
    await rssFeedDelete(selectedFeed.value.id);
    settingsOpen.value = false;
    selectedFeedId.value = null;
    selectedEntryId.value = null;
    await Promise.all([loadFeeds(), loadEntries()]);
    notice.value = t("rss.deleted");
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  }
}

async function importOpml() {
  const path = await open({
    title: t("rss.importOpml"),
    multiple: false,
    filters: [{ name: "OPML", extensions: ["opml", "xml"] }],
  });
  if (!path) return;
  try {
    const result = await rssImportOpml(await readTextFile(path));
    await loadAll();
    notice.value = t("rss.imported", {
      imported: result.imported,
      skipped: result.skipped,
    });
  } catch (cause) {
    error.value = String(cause);
  }
}

async function exportOpml() {
  const path = await save({
    title: t("rss.exportOpml"),
    defaultPath: "zhiyu-rss.opml",
    filters: [{ name: "OPML", extensions: ["opml"] }],
  });
  if (!path) return;
  try {
    await writeTextFile(path, await rssExportOpml());
    notice.value = t("rss.exported");
    error.value = "";
  } catch (cause) {
    error.value = String(cause);
  }
}

async function openOriginal(entry: RssEntry) {
  if (!entry.link) return;
  try {
    await invoke("open_url", { url: entry.link });
  } catch (cause) {
    error.value = String(cause);
  }
}

function formatDate(value: number | null) {
  if (!value) return "";
  return new Intl.DateTimeFormat(locale.value, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(value));
}

watch([selectedFeedId, filter], () => {
  loadEntries().catch((cause) => (error.value = String(cause)));
});

watch(search, () => {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    loadEntries().catch((cause) => (error.value = String(cause)));
  }, 250);
});

onMounted(async () => {
  await loadAll();
  window.addEventListener("keydown", handleReadingShortcut);
  syncTimer = setInterval(async () => {
    try {
      await loadFeeds();
      await loadEntries();
    } catch {
      // 后台同步失败不打断当前阅读。
    }
  }, 60_000);
});

onUnmounted(() => {
  window.removeEventListener("keydown", handleReadingShortcut);
  if (searchTimer) clearTimeout(searchTimer);
  if (syncTimer) clearInterval(syncTimer);
});
</script>

<template>
  <header class="detail-header rss-header">
    <div class="detail-identity">
      <span class="service-logo rss">R</span>
      <div>
        <div class="title-line">
          <h1>{{ t("rss.title") }}</h1>
          <span>RSS · ATOM · JSON FEED</span>
        </div>
        <p>{{ t("rss.subtitle") }}</p>
      </div>
    </div>
    <div class="header-actions">
      <button type="button" @click="importOpml">{{ t("rss.import") }}</button>
      <button type="button" @click="exportOpml">{{ t("rss.export") }}</button>
      <button type="button" :disabled="refreshing || feeds.length === 0" @click="refreshFeeds">
        <span v-if="refreshing" class="spinner"></span>
        {{ refreshing ? t("rss.refreshing") : t("common.refresh") }}
      </button>
      <button class="primary" type="button" @click="addOpen = true">＋ {{ t("rss.subscribe") }}</button>
    </div>
  </header>

  <div v-if="notice" class="notice rss-notice">
    <span>{{ notice }}</span><button type="button" @click="notice = ''">×</button>
  </div>
  <div v-if="error" class="notice danger rss-notice">
    <span>{{ error }}</span><button type="button" @click="error = ''">×</button>
  </div>

  <section
    class="rss-page"
    :class="{
      'reading-mode': readingMode && selectedEntry,
      'ai-reader-open': aiReaderOpen && selectedEntry,
    }"
  >
    <aside class="rss-feeds">
      <div class="rss-pane-title">
        <div><small>SUBSCRIPTIONS</small><strong>{{ t("rss.feeds") }}</strong></div>
        <button v-if="selectedFeed" type="button" :title="t('rss.feedSettings')" @click="openSettings">•••</button>
      </div>
      <button
        class="rss-feed-item"
        :class="{ active: selectedFeedId === null }"
        type="button"
        @click="chooseFeed(null)"
      >
        <span class="rss-feed-mark all">◎</span>
        <span><strong>{{ t("rss.allArticles") }}</strong><small>{{ feeds.length }} {{ t("rss.feedsUnit") }}</small></span>
        <b v-if="totalUnread">{{ totalUnread }}</b>
      </button>
      <button
        v-for="feed in feeds"
        :key="feed.id"
        class="rss-feed-item"
        :class="{ active: selectedFeedId === feed.id, failed: feed.lastError }"
        type="button"
        @click="chooseFeed(feed.id)"
      >
        <span class="rss-feed-mark">{{ feed.title.slice(0, 1).toUpperCase() }}</span>
        <span>
          <strong>{{ feed.title }}</strong>
          <small v-if="feed.lastError">{{ t("rss.refreshFailed") }}</small>
          <small v-else>{{ feed.entryCount }} {{ t("rss.articlesUnit") }}</small>
        </span>
        <b v-if="feed.unreadCount">{{ feed.unreadCount }}</b>
      </button>
      <div v-if="!feeds.length && !loading" class="rss-empty-feed">
        <span>RSS</span>
        <strong>{{ t("rss.noFeeds") }}</strong>
        <p>{{ t("rss.noFeedsHint") }}</p>
        <button class="primary" type="button" @click="addOpen = true">{{ t("rss.addFirst") }}</button>
      </div>
    </aside>

    <div class="rss-entries">
      <div class="rss-entry-toolbar">
        <div class="rss-filters">
          <button v-for="value in (['all', 'unread', 'starred'] as EntryFilter[])" :key="value" type="button" :class="{ active: filter === value }" @click="filter = value">
            {{ t(`rss.filter.${value}`) }}
          </button>
        </div>
        <button type="button" :disabled="entries.length === 0" @click="markAllRead">{{ t("rss.markAllRead") }}</button>
      </div>
      <label class="rss-search">
        <span>⌕</span>
        <input v-model="search" :placeholder="t('rss.search')" />
      </label>
      <div v-if="loading" class="panel-state">{{ t("common.loading") }}…</div>
      <div v-else-if="entries.length === 0" class="panel-state">{{ t("rss.noArticles") }}</div>
      <div v-else class="rss-entry-list">
        <button
          v-for="entry in entries"
          :key="entry.id"
          class="rss-entry-item"
          :class="{ active: selectedEntryId === entry.id, unread: !entry.isRead }"
          type="button"
          @click="chooseEntry(entry)"
        >
          <i></i>
          <span class="rss-entry-source">{{ entry.feedTitle }}</span>
          <strong>{{ entry.title }}</strong>
          <p>{{ entry.summary || entry.content }}</p>
          <time>{{ formatDate(entry.publishedAtMillis) }}</time>
          <span v-if="entry.isStarred" class="rss-star">★</span>
        </button>
      </div>
    </div>

    <article class="rss-reader">
      <div v-if="!selectedEntry" class="rss-reader-empty">
        <span>R</span>
        <strong>{{ t("rss.selectArticle") }}</strong>
        <p>{{ t("rss.selectArticleHint") }}</p>
      </div>
      <template v-else>
        <div class="rss-reader-toolbar">
          <div class="rss-reader-navigation">
            <button v-if="readingMode" type="button" @click="closeReading">
              ← {{ t("rss.backToList") }}
            </button>
            <button type="button" :disabled="!hasPreviousEntry" :title="t('rss.shortcutPrevious')" @click="navigateEntry(-1)">
              ↑ {{ t("rss.previousArticle") }}
            </button>
            <button type="button" :disabled="!hasNextEntry" :title="t('rss.shortcutNext')" @click="navigateEntry(1)">
              ↓ {{ t("rss.nextArticle") }}
            </button>
          </div>
          <span class="rss-reader-spacer"></span>
          <button type="button" @click="toggleRead(selectedEntry)">
            {{ selectedEntry.isRead ? t("rss.markUnread") : t("rss.markRead") }}
          </button>
          <button type="button" :class="{ starred: selectedEntry.isStarred }" @click="toggleStar(selectedEntry)">
            {{ selectedEntry.isStarred ? "★" : "☆" }} {{ t("rss.star") }}
          </button>
          <button
            type="button"
            class="rss-ai-trigger"
            :class="{ active: aiReaderOpen }"
            @click="aiReaderOpen = !aiReaderOpen"
          >
            ✦ {{ t("rss.aiReader.button") }}
          </button>
          <button class="primary" type="button" :disabled="!selectedEntry.link" @click="openOriginal(selectedEntry)">
            {{ t("rss.openOriginal") }} ↗
          </button>
        </div>
        <div class="rss-reader-body">
          <div class="rss-reader-document">
            <div class="rss-reader-meta">
              <span>{{ selectedEntry.feedTitle }}</span>
              <time>{{ formatDate(selectedEntry.publishedAtMillis) }}</time>
            </div>
            <h2>{{ selectedEntry.title }}</h2>
            <p v-if="selectedEntry.author" class="rss-author">{{ t("rss.byAuthor", { author: selectedEntry.author }) }}</p>
            <div class="rss-content">{{ selectedContent }}</div>
            <footer class="rss-reader-footer">
              <button type="button" :disabled="!hasNextEntry" @click="navigateEntry(1)">
                {{ t("rss.readNext") }} →
              </button>
            </footer>
          </div>
        </div>
      </template>
    </article>
    <RssAiPanel
      v-if="aiReaderOpen && selectedEntry"
      :entry="selectedEntry"
      @close="aiReaderOpen = false"
    />
  </section>

  <div v-if="addOpen" class="rss-modal-backdrop" role="dialog" aria-modal="true" @click.self="addOpen = false">
    <form class="rss-modal rss-add-modal" @submit.prevent="addFeed">
      <div class="rss-modal-heading"><div><small>NEW SUBSCRIPTION</small><h2>{{ t("rss.addTitle") }}</h2></div><button type="button" @click="addOpen = false">×</button></div>
      <p>{{ t("rss.addHint") }}</p>
      <label>{{ t("rss.feedUrl") }}<input v-model="newFeedUrl" autofocus placeholder="https://example.com/feed.xml" spellcheck="false" /></label>
      <details class="rss-recommendations">
        <summary>
          <span><b>{{ t("rss.recommendations") }}</b><small>{{ t("rss.recommendationsHint") }}</small></span>
          <i>⌄</i>
        </summary>
        <div class="rss-recommendation-controls">
          <select v-model="recommendationFilter" :aria-label="t('rss.recommendationCategory')">
            <option value="all">{{ t("rss.recommendationCategories.all") }}</option>
            <option value="chinese">{{ t("rss.recommendationCategories.chinese") }}</option>
            <option value="programming">{{ t("rss.recommendationCategories.programming") }}</option>
            <option value="ai">{{ t("rss.recommendationCategories.ai") }}</option>
            <option value="engineering">{{ t("rss.recommendationCategories.engineering") }}</option>
          </select>
          <input v-model="recommendationSearch" :placeholder="t('rss.searchRecommendations')" />
        </div>
        <div class="rss-recommendation-list">
          <button
            v-for="feed in recommendations"
            :key="feed.url"
            type="button"
            class="rss-recommendation-item"
            :class="{ selected: newFeedUrl === feed.url }"
            @click="chooseRecommendation(feed)"
          >
            <span><strong>{{ feed.name }}</strong><small>{{ feed.url }}</small></span>
            <b :class="feed.source">{{ t(`rss.recommendationSources.${feed.source}`) }}</b>
          </button>
          <p v-if="recommendations.length === 0">{{ t("rss.noRecommendations") }}</p>
        </div>
        <p class="rss-community-hint">{{ t("rss.communityFeedHint") }}</p>
      </details>
      <div class="rss-modal-actions"><button type="button" @click="addOpen = false">{{ t("common.cancel") }}</button><button class="primary" type="submit" :disabled="adding || !newFeedUrl.trim()"><span v-if="adding" class="spinner"></span>{{ adding ? t("rss.validating") : t("rss.subscribe") }}</button></div>
    </form>
  </div>

  <div v-if="settingsOpen && selectedFeed" class="rss-modal-backdrop" role="dialog" aria-modal="true" @click.self="settingsOpen = false">
    <form class="rss-modal" @submit.prevent="saveFeedSettings">
      <div class="rss-modal-heading"><div><small>FEED SETTINGS</small><h2>{{ t("rss.feedSettings") }}</h2></div><button type="button" @click="settingsOpen = false">×</button></div>
      <label>{{ t("rss.feedName") }}<input v-model="editTitle" /></label>
      <label>{{ t("rss.refreshInterval") }}<select v-model.number="editInterval"><option :value="5">5 {{ t("rss.minutes") }}</option><option :value="15">15 {{ t("rss.minutes") }}</option><option :value="30">30 {{ t("rss.minutes") }}</option><option :value="60">60 {{ t("rss.minutes") }}</option><option :value="180">180 {{ t("rss.minutes") }}</option></select></label>
      <label class="rss-check"><input v-model="editEnabled" type="checkbox" />{{ t("rss.autoRefresh") }}</label>
      <p v-if="selectedFeed.lastError" class="rss-feed-error">{{ selectedFeed.lastError }}</p>
      <div class="rss-modal-actions"><button class="danger" type="button" @click="deleteFeed">{{ t("rss.deleteFeed") }}</button><span></span><button type="button" @click="settingsOpen = false">{{ t("common.cancel") }}</button><button class="primary" type="submit">{{ t("common.save") }}</button></div>
    </form>
  </div>
</template>

<style scoped>
.rss-header{gap:20px}.rss-header .header-actions{flex-wrap:wrap}.rss-notice{margin:14px 32px 0}.rss-page{display:grid;grid-template-columns:220px minmax(290px,350px) minmax(360px,1fr);height:calc(100vh - 132px);min-height:540px;padding:24px 32px 36px;gap:14px;min-width:0}.rss-feeds,.rss-entries,.rss-reader{min-width:0;overflow:hidden;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.rss-feeds,.rss-entries{display:flex;flex-direction:column}.rss-page button,.rss-modal button{display:inline-flex;min-height:30px;align-items:center;justify-content:center;gap:6px;padding:7px 11px;border:1px solid var(--color-border-strong);background:var(--color-bg-panel);color:var(--color-text-primary);font-size:9px;cursor:pointer;transition:border-color 140ms ease,background-color 140ms ease,color 140ms ease}.rss-page button:hover:not(:disabled),.rss-modal button:hover:not(:disabled){border-color:var(--color-text-muted);background:var(--color-bg-muted)}.rss-page button.primary,.rss-modal button.primary{border-color:var(--color-control-primary);background:var(--color-control-primary);color:#fff}.rss-page button.danger,.rss-modal button.danger{border-color:var(--color-danger-text);color:var(--color-danger-text)}.rss-pane-title,.rss-entry-toolbar{display:flex;min-height:58px;align-items:center;justify-content:space-between;padding:0 14px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.rss-pane-title div{display:grid;gap:4px}.rss-pane-title small,.rss-modal-heading small{color:var(--color-text-muted);font:8px/1.2 "SFMono-Regular",Consolas,monospace;letter-spacing:.12em}.rss-pane-title strong{font-size:13px}.rss-pane-title>button{width:30px;min-height:30px;padding:0}.rss-feed-item{position:relative!important;display:grid!important;grid-template-columns:30px minmax(0,1fr) auto!important;align-items:center!important;justify-content:stretch!important;gap:9px!important;width:100%;min-height:0!important;padding:11px 12px!important;border:0!important;border-bottom:1px solid var(--color-border)!important;background:transparent!important;text-align:left}.rss-feed-item:hover{background:var(--color-bg-muted)!important}.rss-feed-item.active{background:var(--color-panel-active)!important;box-shadow:inset 3px 0 var(--color-accent)}.rss-feed-item.failed{box-shadow:inset 3px 0 var(--color-danger)}.rss-feed-mark{display:grid;width:28px;height:28px;place-items:center;border-radius:50%;background:color-mix(in srgb,var(--color-accent) 18%,var(--color-panel));color:var(--color-accent);font:700 10px/1 "SFMono-Regular",Consolas,monospace}.rss-feed-mark.all{color:var(--color-text)}.rss-feed-item>span:nth-child(2){display:grid;min-width:0;gap:3px}.rss-feed-item strong,.rss-feed-item small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.rss-feed-item strong{font-size:10px}.rss-feed-item small{color:var(--color-text-muted);font-size:8px}.rss-feed-item b{min-width:19px;padding:3px 5px;border-radius:10px;background:var(--color-accent);color:var(--color-on-accent);font:8px/1 "SFMono-Regular",Consolas,monospace;text-align:center}.rss-empty-feed,.rss-reader-empty{display:grid;place-items:center;align-content:center;gap:8px;flex:1;padding:24px;text-align:center;color:var(--color-text-muted)}.rss-empty-feed>span,.rss-reader-empty>span{display:grid;width:44px;height:44px;place-items:center;border:1px solid var(--color-border);border-radius:50%;background:var(--color-bg-muted);font:700 14px/1 "SFMono-Regular",Consolas,monospace}.rss-empty-feed strong,.rss-reader-empty strong{color:var(--color-text);font-size:12px}.rss-empty-feed p,.rss-reader-empty p{max-width:240px;margin:0 0 6px;font-size:9px;line-height:1.6}.rss-entry-toolbar{padding:0 10px}.rss-entry-toolbar>button{padding:7px 8px;font-size:8px}.rss-filters{display:flex;gap:4px}.rss-filters button{min-width:46px;border-color:transparent;background:transparent;font-size:8px}.rss-filters button.active{border-color:var(--color-border-strong);background:var(--color-bg-panel);color:var(--color-accent);box-shadow:inset 0 -2px var(--color-accent)}.rss-search{display:flex;align-items:center;gap:7px;margin:10px;border:1px solid var(--color-border);background:var(--color-input)}.rss-search:focus-within{border-color:var(--color-border-strong)}.rss-search span{padding-left:10px;color:var(--color-text-muted)}.rss-search input{width:100%;height:34px;padding:0 10px 0 0;border:0;outline:0;background:transparent;font-size:9px}.rss-entry-list{overflow:auto}.rss-entry-item{position:relative!important;display:grid!important;justify-content:stretch!important;width:100%;min-height:0!important;gap:5px!important;padding:13px 34px 13px 17px!important;border:0!important;border-bottom:1px solid var(--color-border)!important;background:transparent!important;text-align:left}.rss-entry-item:hover{background:var(--color-bg-muted)!important}.rss-entry-item.active{background:var(--color-panel-active)!important;box-shadow:inset 3px 0 var(--color-accent)}.rss-entry-item>i{position:absolute;top:18px;left:7px;width:5px;height:5px;border-radius:50%;background:transparent}.rss-entry-item.unread>i{background:var(--color-accent)}.rss-entry-source{overflow:hidden;color:var(--color-accent);font:8px/1.2 "SFMono-Regular",Consolas,monospace;text-overflow:ellipsis;white-space:nowrap}.rss-entry-item strong{display:-webkit-box;overflow:hidden;font-size:11px;line-height:1.35;-webkit-box-orient:vertical;-webkit-line-clamp:2}.rss-entry-item:not(.unread) strong{color:var(--color-text-secondary);font-weight:500}.rss-entry-item p{display:-webkit-box;overflow:hidden;margin:0;color:var(--color-text-muted);font-size:8px;line-height:1.5;-webkit-box-orient:vertical;-webkit-line-clamp:2}.rss-entry-item time{color:var(--color-text-muted);font:7px/1.2 "SFMono-Regular",Consolas,monospace}.rss-star{position:absolute;top:13px;right:12px;color:#d49a42;font-size:11px}.rss-reader{display:flex;flex-direction:column}.rss-reader-toolbar{display:flex;gap:7px;min-height:58px;align-items:center;padding:0 14px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.rss-reader-navigation{display:flex;gap:6px}.rss-page:not(.reading-mode) .rss-reader-navigation{display:none}.rss-reader-spacer{flex:1}.rss-reader-toolbar button{font-size:8px}.rss-reader-toolbar .starred{color:#d49a42}.rss-reader-body{overflow:auto;padding:30px clamp(24px,4vw,58px) 56px}.rss-reader-document{width:100%;max-width:780px;margin:0 auto}.rss-reader-meta{display:flex;justify-content:space-between;gap:20px;color:var(--color-text-muted);font:8px/1.4 "SFMono-Regular",Consolas,monospace}.rss-reader-body h2{margin:14px 0 10px;font-size:clamp(20px,2vw,30px);line-height:1.25;letter-spacing:-.035em}.rss-author{margin:0 0 24px;color:var(--color-text-muted);font-size:9px}.rss-content{color:var(--color-text-secondary);font-size:11px;line-height:1.9;white-space:pre-wrap}.rss-reader-footer{display:flex;justify-content:flex-end;margin-top:42px;padding-top:18px;border-top:1px solid var(--color-border)}.rss-page.reading-mode{grid-template-columns:minmax(0,1fr)}.rss-page.reading-mode .rss-feeds,.rss-page.reading-mode .rss-entries{display:none}.rss-page.reading-mode .rss-reader{grid-column:1/-1}.rss-page.reading-mode .rss-reader-body{padding-top:42px}.rss-page.reading-mode .rss-reader-document{max-width:820px}.rss-page.reading-mode .rss-content{font-size:12px;line-height:2}.rss-modal-backdrop{position:fixed;z-index:120;inset:0;display:grid;place-items:center;padding:24px;background:rgba(0,0,0,.48);backdrop-filter:blur(5px)}.rss-modal{display:grid;width:min(500px,calc(100vw - 48px));gap:16px;padding:22px;border:1px solid var(--color-border-strong);background:var(--color-panel);box-shadow:0 24px 80px rgba(0,0,0,.32)}.rss-modal-heading{display:flex;align-items:start;justify-content:space-between}.rss-modal-heading h2{margin:4px 0 0;font-size:20px}.rss-modal-heading>button{width:30px;height:30px;padding:0}.rss-modal>p{margin:0;color:var(--color-text-muted);font-size:9px;line-height:1.6}.rss-modal>label{display:grid;gap:7px;color:var(--color-text-secondary);font-size:9px}.rss-modal input,.rss-modal select{box-sizing:border-box;width:100%;height:38px;padding:0 11px}.rss-modal .rss-check{display:flex;align-items:center}.rss-modal .rss-check input{width:15px;height:15px}.rss-modal-actions{display:flex;align-items:center;justify-content:flex-end;gap:8px;padding-top:4px}.rss-modal-actions>span{flex:1}.rss-feed-error{padding:10px!important;border:1px solid color-mix(in srgb,var(--color-danger) 40%,transparent);color:var(--color-danger)!important}.panel-state{display:grid;flex:1;place-items:center;color:var(--color-text-muted);font-size:9px}@media(max-width:1180px){.rss-page:not(.reading-mode){grid-template-columns:190px minmax(270px,330px) minmax(320px,1fr);padding-inline:18px;gap:10px}}@media(max-width:900px){.rss-page:not(.reading-mode){grid-template-columns:190px 1fr}.rss-page:not(.reading-mode) .rss-reader{display:none}.rss-reader-navigation button:not(:first-child){display:none}}

.rss-add-modal{width:min(620px,calc(100vw - 48px));max-height:calc(100vh - 48px);overflow:auto}.rss-recommendations{border:1px solid var(--color-border);background:var(--color-bg-muted)}.rss-recommendations>summary{display:flex;min-height:48px;align-items:center;justify-content:space-between;padding:0 13px;cursor:pointer;list-style:none}.rss-recommendations>summary::-webkit-details-marker{display:none}.rss-recommendations>summary span{display:grid;gap:3px}.rss-recommendations>summary b{font-size:10px}.rss-recommendations>summary small{color:var(--color-text-muted);font-size:8px;font-weight:400}.rss-recommendations>summary i{color:var(--color-text-muted);font-style:normal;transition:transform 140ms ease}.rss-recommendations[open]>summary i{transform:rotate(180deg)}.rss-recommendation-controls{display:grid;grid-template-columns:160px minmax(0,1fr);gap:8px;padding:10px 12px;border-top:1px solid var(--color-border)}.rss-recommendation-controls select,.rss-recommendation-controls input{height:34px}.rss-recommendation-list{display:grid;max-height:260px;overflow:auto;border-top:1px solid var(--color-border)}.rss-modal .rss-recommendation-item{display:grid;grid-template-columns:minmax(0,1fr) auto;width:100%;min-height:52px;align-items:center;justify-content:stretch;padding:8px 12px;border:0;border-bottom:1px solid var(--color-border);background:transparent;text-align:left}.rss-modal .rss-recommendation-item:hover{background:var(--color-hover)}.rss-modal .rss-recommendation-item.selected{background:var(--color-panel-active);box-shadow:inset 3px 0 var(--color-accent)}.rss-recommendation-item>span{display:grid;min-width:0;gap:4px}.rss-recommendation-item strong,.rss-recommendation-item small{overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.rss-recommendation-item strong{font-size:9px}.rss-recommendation-item small{color:var(--color-text-muted);font:7px/1.2 "SFMono-Regular",Consolas,monospace}.rss-recommendation-item>b{padding:3px 6px;border:1px solid var(--color-border-strong);border-radius:10px;color:var(--color-text-muted);font-size:7px}.rss-recommendation-item>b.community{border-color:color-mix(in srgb,var(--color-warning) 55%,var(--color-border));color:var(--color-warning)}.rss-recommendation-list>p{padding:20px;text-align:center}.rss-community-hint{margin:0;padding:8px 12px;color:var(--color-text-muted);font-size:7px;line-height:1.5}@media(max-width:640px){.rss-recommendation-controls{grid-template-columns:1fr}}

.rss-reader-toolbar .rss-ai-trigger{color:var(--color-accent)}.rss-reader-toolbar .rss-ai-trigger.active{border-color:var(--color-accent);background:color-mix(in srgb,var(--color-accent) 10%,var(--color-bg-panel))}.rss-page.reading-mode.ai-reader-open{grid-template-columns:minmax(0,1fr) minmax(360px,420px)}.rss-page.reading-mode.ai-reader-open .rss-reader{grid-column:1}.rss-page.reading-mode.ai-reader-open .rss-ai-panel{grid-column:2}@media(max-width:1050px){.rss-page.reading-mode.ai-reader-open{grid-template-columns:minmax(0,1fr) minmax(330px,380px);padding-inline:14px}}@media(max-width:820px){.rss-page.reading-mode.ai-reader-open{position:relative;grid-template-columns:1fr}.rss-page.reading-mode.ai-reader-open .rss-ai-panel{position:absolute;z-index:20;inset:0 0 0 auto;width:min(420px,100%);box-shadow:-18px 0 50px rgba(0,0,0,.2)}}
</style>
