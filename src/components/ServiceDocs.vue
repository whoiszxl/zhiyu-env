<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { ServiceKind } from "../types";
import {
  buildServiceDocs,
  serviceDocTagline,
  type DocCodeSample,
} from "../docs";

const props = defineProps<{
  kind: ServiceKind;
  port: number;
  serviceName: string;
}>();

const COPY_FEEDBACK_MS = 1400;

const chapters = computed(() => buildServiceDocs(props.kind, props.port));
const tagline = computed(() => serviceDocTagline(props.kind));

const activeChapterId = ref(chapters.value[0]?.id ?? "");
const activeChapter = computed(
  () =>
    chapters.value.find((chapter) => chapter.id === activeChapterId.value) ??
    chapters.value[0],
);

/** 每个代码示例组当前选中的语言标签，键为 章节id:块序号。 */
const activeSampleLabel = ref<Record<string, string>>({});
const copiedKey = ref("");
let copyTimer: number | undefined;

// 切换服务时回到第一章，避免停留在上一个服务的章节上
watch(
  () => props.kind,
  () => {
    activeChapterId.value = chapters.value[0]?.id ?? "";
    activeSampleLabel.value = {};
  },
);

function sampleLabels(samples: DocCodeSample[]): string[] {
  return [...new Set(samples.map((sample) => sample.label))];
}

function currentLabel(groupKey: string, samples: DocCodeSample[]): string {
  return activeSampleLabel.value[groupKey] ?? sampleLabels(samples)[0] ?? "";
}

function visibleSamples(
  groupKey: string,
  samples: DocCodeSample[],
): DocCodeSample[] {
  const label = currentLabel(groupKey, samples);
  return samples.filter((sample) => sample.label === label);
}

function selectLabel(groupKey: string, label: string) {
  activeSampleLabel.value = { ...activeSampleLabel.value, [groupKey]: label };
}

async function copyCode(key: string, code: string) {
  try {
    await navigator.clipboard.writeText(code);
    copiedKey.value = key;
    window.clearTimeout(copyTimer);
    copyTimer = window.setTimeout(() => {
      copiedKey.value = "";
    }, COPY_FEEDBACK_MS);
  } catch {
    // 剪贴板不可用时静默降级，用户仍可手动选中复制
    copiedKey.value = "";
  }
}
</script>

<template>
  <div class="docs-panel">
    <nav class="docs-toc">
      <p class="docs-toc-label">CONTENTS</p>
      <button
        v-for="chapter in chapters"
        :key="chapter.id"
        type="button"
        :class="{ active: chapter.id === activeChapter?.id }"
        @click="activeChapterId = chapter.id"
      >
        <strong>{{ chapter.navLabel }}</strong>
        <small>{{ chapter.navHint }}</small>
      </button>
    </nav>

    <article v-if="activeChapter" class="docs-body">
      <header class="docs-head">
        <p>{{ serviceName }} · 使用文档</p>
        <h2>{{ activeChapter.title }}</h2>
        <span>{{ tagline }}</span>
      </header>

      <p class="docs-intro">{{ activeChapter.intro }}</p>

      <template v-for="(block, index) in activeChapter.blocks" :key="index">
        <p v-if="block.kind === 'text'" class="docs-text">{{ block.value }}</p>

        <ul v-else-if="block.kind === 'list'" class="docs-list">
          <li v-for="(item, i) in block.items" :key="i">{{ item }}</li>
        </ul>

        <div v-else-if="block.kind === 'callout'" class="docs-callout" :class="block.tone">
          <strong>{{ block.title }}</strong>
          <p>{{ block.value }}</p>
        </div>

        <div v-else-if="block.kind === 'table'" class="docs-table-wrap">
          <table class="docs-table">
            <thead>
              <tr>
                <th v-for="(cell, i) in block.head" :key="i">{{ cell }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(row, r) in block.rows" :key="r">
                <td v-for="(cell, c) in row" :key="c">{{ cell }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <figure v-else-if="block.kind === 'code'" class="docs-code">
          <figcaption>
            <span>{{ block.caption ?? block.lang }}</span>
            <button
              type="button"
              @click="copyCode(`${activeChapter.id}:${index}`, block.code)"
            >
              {{ copiedKey === `${activeChapter.id}:${index}` ? "已复制" : "复制" }}
            </button>
          </figcaption>
          <pre><code>{{ block.code }}</code></pre>
        </figure>

        <div v-else-if="block.kind === 'samples'" class="docs-samples">
          <div class="docs-sample-tabs">
            <button
              v-for="label in sampleLabels(block.samples)"
              :key="label"
              type="button"
              :class="{
                active:
                  currentLabel(`${activeChapter.id}:${index}`, block.samples) === label,
              }"
              @click="selectLabel(`${activeChapter.id}:${index}`, label)"
            >
              {{ label }}
            </button>
          </div>

          <figure
            v-for="(sample, s) in visibleSamples(
              `${activeChapter.id}:${index}`,
              block.samples,
            )"
            :key="`${sample.label}-${s}`"
            class="docs-code"
          >
            <figcaption>
              <span>{{ sample.caption ?? sample.lang }}</span>
              <button
                type="button"
                @click="copyCode(`${activeChapter.id}:${index}:${s}`, sample.code)"
              >
                {{
                  copiedKey === `${activeChapter.id}:${index}:${s}`
                    ? "已复制"
                    : "复制"
                }}
              </button>
            </figcaption>
            <pre><code>{{ sample.code }}</code></pre>
          </figure>
        </div>
      </template>
    </article>
  </div>
</template>
