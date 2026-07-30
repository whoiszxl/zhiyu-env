<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { generateQrCode } from "../../api/tools";
import type { QrCodeOptions, QrCodeResult } from "../../types";

type ContentType = "text" | "url" | "wifi" | "email" | "phone" | "sms" | "contact";
type HistoryItem = {
  id: string;
  type: ContentType;
  content: string;
  label: string;
  createdAt: number;
};
interface DetectedBarcode { rawValue: string }
interface BarcodeDetectorInstance { detect(source: Blob): Promise<DetectedBarcode[]> }
interface BarcodeDetectorConstructor {
  new (options: { formats: string[] }): BarcodeDetectorInstance;
}

const { t } = useI18n();
const HISTORY_KEY = "zhiyu.qrcode.history.v2";
const contentType = ref<ContentType>("url");
const content = ref("https://github.com/");
const options = reactive<QrCodeOptions>({
  content: "",
  errorCorrection: "M",
  size: 480,
  foreground: "#1f231d",
  background: "#ffffff",
  quietZone: true,
});
const fields = reactive({
  url: "https://github.com/",
  ssid: "",
  wifiPassword: "",
  wifiSecurity: "WPA",
  wifiHidden: false,
  email: "",
  emailSubject: "",
  emailBody: "",
  phone: "",
  smsMessage: "",
  contactName: "",
  contactOrganization: "",
  contactPhone: "",
  contactEmail: "",
});
const result = ref<QrCodeResult | null>(null);
const generating = ref(false);
const error = ref("");
const scanning = ref(false);
const scanSupported = ref(false);
const copied = ref("");
const fileInput = ref<HTMLInputElement | null>(null);
const history = ref<HistoryItem[]>([]);
let generateTimer: number | undefined;

const contentTypes: Array<{ id: ContentType; icon: string }> = [
  { id: "text", icon: "T" },
  { id: "url", icon: "↗" },
  { id: "wifi", icon: "⌁" },
  { id: "email", icon: "@" },
  { id: "phone", icon: "☎" },
  { id: "sms", icon: "✉" },
  { id: "contact", icon: "◉" },
];
const colorPresets = [
  { foreground: "#1f231d", background: "#ffffff" },
  { foreground: "#0b2b40", background: "#f5f0df" },
  { foreground: "#173f35", background: "#f3eee4" },
  { foreground: "#3c2450", background: "#f7eff8" },
  { foreground: "#6b1f28", background: "#fff5ed" },
  { foreground: "#ffffff", background: "#172033" },
];
const contentBytes = computed(() => new TextEncoder().encode(content.value).length);
const capacityPercent = computed(() => Math.min(100, (contentBytes.value / 4096) * 100));
const contentValid = computed(() => contentBytes.value > 0 && contentBytes.value <= 4096);
const currentTypeLabel = computed(() => t(`qr.types.${contentType.value}`));

function escapeWifi(value: string) {
  return value.replace(/([\\;,:"'])/g, "\\$1");
}

function buildStructuredContent() {
  switch (contentType.value) {
    case "url":
      content.value = fields.url.trim();
      break;
    case "wifi":
      content.value = fields.ssid
        ? `WIFI:T:${fields.wifiSecurity};S:${escapeWifi(fields.ssid)};P:${escapeWifi(fields.wifiPassword)};H:${fields.wifiHidden ? "true" : "false"};;`
        : "";
      break;
    case "email": {
      if (!fields.email.trim()) {
        content.value = "";
        break;
      }
      const query = new URLSearchParams();
      if (fields.emailSubject) query.set("subject", fields.emailSubject);
      if (fields.emailBody) query.set("body", fields.emailBody);
      content.value = `mailto:${fields.email.trim()}${query.size ? `?${query}` : ""}`;
      break;
    }
    case "phone":
      content.value = fields.phone.trim() ? `tel:${fields.phone.trim()}` : "";
      break;
    case "sms":
      content.value = fields.phone.trim()
        ? `SMSTO:${fields.phone.trim()}:${fields.smsMessage}`
        : "";
      break;
    case "contact":
      content.value = fields.contactName.trim()
        ? [
            "BEGIN:VCARD",
            "VERSION:3.0",
            `FN:${fields.contactName.trim()}`,
            fields.contactOrganization ? `ORG:${fields.contactOrganization}` : "",
            fields.contactPhone ? `TEL:${fields.contactPhone}` : "",
            fields.contactEmail ? `EMAIL:${fields.contactEmail}` : "",
            "END:VCARD",
          ].filter(Boolean).join("\n")
        : "";
      break;
  }
}

function switchType(type: ContentType) {
  contentType.value = type;
  if (type === "text") content.value = "";
  else buildStructuredContent();
}

async function generate(saveHistory = false) {
  if (generating.value || !contentValid.value) return;
  generating.value = true;
  error.value = "";
  try {
    result.value = await generateQrCode({ ...options, content: content.value });
    if (saveHistory) remember();
  } catch (cause) {
    result.value = null;
    error.value = String(cause);
  } finally {
    generating.value = false;
  }
}

function scheduleGenerate() {
  window.clearTimeout(generateTimer);
  if (!contentValid.value) {
    result.value = null;
    return;
  }
  generateTimer = window.setTimeout(() => void generate(false), 240);
}

function remember() {
  const label = contentType.value === "url"
    ? fields.url
    : contentType.value === "wifi"
      ? fields.ssid
      : contentType.value === "contact"
        ? fields.contactName
        : content.value.slice(0, 56);
  const item: HistoryItem = {
    id: `${Date.now()}-${content.value.length}`,
    type: contentType.value,
    content: content.value,
    label: label || currentTypeLabel.value,
    createdAt: Date.now(),
  };
  history.value = [
    item,
    ...history.value.filter((entry) => entry.content !== item.content),
  ].slice(0, 12);
  localStorage.setItem(HISTORY_KEY, JSON.stringify(history.value));
}

function useHistory(item: HistoryItem) {
  contentType.value = "text";
  content.value = item.content;
}

function clearHistory() {
  history.value = [];
  localStorage.removeItem(HISTORY_KEY);
}

function applyPreset(preset: { foreground: string; background: string }) {
  options.foreground = preset.foreground;
  options.background = preset.background;
}

function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function downloadSvg() {
  if (result.value) {
    downloadBlob(new Blob([result.value.svg], { type: "image/svg+xml" }), "zhiyu-qrcode.svg");
  }
}

async function pngBlob() {
  if (!result.value) return null;
  const svgUrl = URL.createObjectURL(new Blob([result.value.svg], { type: "image/svg+xml" }));
  try {
    const image = new Image();
    image.src = svgUrl;
    await image.decode();
    const canvas = document.createElement("canvas");
    canvas.width = options.size;
    canvas.height = options.size;
    const context = canvas.getContext("2d");
    if (!context) throw new Error(t("qr.errors.canvas"));
    if (options.background !== "transparent") {
      context.fillStyle = options.background;
      context.fillRect(0, 0, canvas.width, canvas.height);
    }
    context.drawImage(image, 0, 0, canvas.width, canvas.height);
    return await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  } finally {
    URL.revokeObjectURL(svgUrl);
  }
}

async function downloadPng() {
  try {
    const blob = await pngBlob();
    if (!blob) throw new Error(t("qr.errors.png"));
    downloadBlob(blob, "zhiyu-qrcode.png");
  } catch (cause) {
    error.value = String(cause);
  }
}

async function copySvg() {
  if (!result.value) return;
  await navigator.clipboard.writeText(result.value.svg);
  flashCopied("svg");
}

async function copyContent() {
  await navigator.clipboard.writeText(content.value);
  flashCopied("content");
}

async function copyPng() {
  try {
    const blob = await pngBlob();
    if (!blob || typeof ClipboardItem === "undefined") throw new Error(t("qr.errors.clipboard"));
    await navigator.clipboard.write([new ClipboardItem({ "image/png": blob })]);
    flashCopied("png");
  } catch (cause) {
    error.value = String(cause);
  }
}

function flashCopied(value: string) {
  copied.value = value;
  window.setTimeout(() => {
    if (copied.value === value) copied.value = "";
  }, 1200);
}

async function scanFile(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  const Detector = (window as unknown as { BarcodeDetector?: BarcodeDetectorConstructor }).BarcodeDetector;
  if (!Detector) {
    error.value = t("qr.errors.scanUnsupported");
    return;
  }
  scanning.value = true;
  error.value = "";
  try {
    const codes = await new Detector({ formats: ["qr_code"] }).detect(file);
    if (!codes.length) throw new Error(t("qr.errors.notFound"));
    contentType.value = "text";
    content.value = codes[0].rawValue;
    await generate(true);
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  } finally {
    scanning.value = false;
  }
}

watch(
  fields,
  () => {
    if (contentType.value !== "text") buildStructuredContent();
  },
  { deep: true },
);
watch(
  [content, () => options.errorCorrection, () => options.size, () => options.foreground, () => options.background, () => options.quietZone],
  scheduleGenerate,
);

onMounted(() => {
  scanSupported.value = "BarcodeDetector" in (window as unknown as Record<string, unknown>);
  try {
    const stored = JSON.parse(localStorage.getItem(HISTORY_KEY) || "[]");
    history.value = Array.isArray(stored) ? stored.slice(0, 12) : [];
  } catch {
    history.value = [];
  }
  void generate(false);
});
onBeforeUnmount(() => window.clearTimeout(generateTimer));
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo qrcode">▦</span>
      <div>
        <div class="title-line"><h1>{{ t("qr.title") }}</h1><span>LOCAL QR STUDIO</span></div>
        <p>{{ t("qr.subtitle") }}</p>
      </div>
    </div>
    <div class="header-actions">
      <input ref="fileInput" class="hidden-file" type="file" accept="image/*" @change="scanFile" />
      <button type="button" :disabled="scanning || !scanSupported" :title="scanSupported ? t('qr.scanHint') : t('qr.errors.scanUnsupported')" @click="fileInput?.click()">
        {{ scanning ? t("qr.scanning") : t("qr.scan") }}
      </button>
      <button class="primary" type="button" :disabled="generating || !contentValid" @click="generate(true)">
        <span v-if="generating" class="spinner"></span>{{ generating ? t("qr.generating") : t("qr.generate") }}
      </button>
    </div>
  </header>

  <div v-if="error" class="notice danger"><span>{{ error }}</span><button type="button" @click="error = ''">×</button></div>

  <main class="qr-page">
    <nav class="type-tabs">
      <button v-for="item in contentTypes" :key="item.id" type="button" :class="{ active: contentType === item.id }" @click="switchType(item.id)">
        <i>{{ item.icon }}</i><span>{{ t(`qr.types.${item.id}`) }}</span>
      </button>
    </nav>

    <section class="qr-workspace">
      <article class="editor-column">
        <header class="section-head">
          <div><small>CONTENT</small><h2>{{ t("qr.content") }}</h2></div>
          <span :class="{ danger: contentBytes > 4096 }">{{ contentBytes }} / 4096 B</span>
        </header>

        <div class="content-editor">
          <textarea v-if="contentType === 'text'" v-model="content" spellcheck="false" :placeholder="t('qr.placeholders.text')"></textarea>
          <div v-else-if="contentType === 'url'" class="form-fields single">
            <label>{{ t("qr.fields.url") }}<input v-model="fields.url" type="url" placeholder="https://example.com" /></label>
          </div>
          <div v-else-if="contentType === 'wifi'" class="form-fields">
            <label class="wide">{{ t("qr.fields.ssid") }}<input v-model="fields.ssid" /></label>
            <label>{{ t("qr.fields.security") }}<select v-model="fields.wifiSecurity"><option>WPA</option><option>WEP</option><option value="nopass">{{ t("qr.fields.noPassword") }}</option></select></label>
            <label>{{ t("qr.fields.password") }}<input v-model="fields.wifiPassword" type="password" :disabled="fields.wifiSecurity === 'nopass'" /></label>
            <label class="check"><input v-model="fields.wifiHidden" type="checkbox" />{{ t("qr.fields.hiddenNetwork") }}</label>
          </div>
          <div v-else-if="contentType === 'email'" class="form-fields">
            <label class="wide">{{ t("qr.fields.email") }}<input v-model="fields.email" type="email" /></label>
            <label class="wide">{{ t("qr.fields.subject") }}<input v-model="fields.emailSubject" /></label>
            <label class="wide">{{ t("qr.fields.body") }}<textarea v-model="fields.emailBody"></textarea></label>
          </div>
          <div v-else-if="contentType === 'phone' || contentType === 'sms'" class="form-fields">
            <label class="wide">{{ t("qr.fields.phone") }}<input v-model="fields.phone" type="tel" placeholder="+86 138 0000 0000" /></label>
            <label v-if="contentType === 'sms'" class="wide">{{ t("qr.fields.message") }}<textarea v-model="fields.smsMessage"></textarea></label>
          </div>
          <div v-else class="form-fields">
            <label>{{ t("qr.fields.name") }}<input v-model="fields.contactName" /></label>
            <label>{{ t("qr.fields.organization") }}<input v-model="fields.contactOrganization" /></label>
            <label>{{ t("qr.fields.phone") }}<input v-model="fields.contactPhone" type="tel" /></label>
            <label>{{ t("qr.fields.email") }}<input v-model="fields.contactEmail" type="email" /></label>
          </div>
        </div>

        <div class="encoded-preview">
          <header><small>{{ t("qr.encoded") }}</small><button type="button" :disabled="!content" @click="copyContent">{{ copied === "content" ? t("common.copied") : t("common.copy") }}</button></header>
          <code>{{ content || t("qr.encodedEmpty") }}</code>
          <i><b :style="{ width: `${capacityPercent}%` }"></b></i>
        </div>

        <section class="style-panel">
          <header><small>STYLE & RELIABILITY</small><h3>{{ t("qr.style") }}</h3></header>
          <div class="option-grid">
            <label>{{ t("qr.errorCorrection") }}<select v-model="options.errorCorrection"><option value="L">L · 7%</option><option value="M">M · 15%</option><option value="Q">Q · 25%</option><option value="H">H · 30%</option></select></label>
            <label>{{ t("qr.exportSize") }}<select v-model.number="options.size"><option :value="240">240 px</option><option :value="320">320 px</option><option :value="480">480 px</option><option :value="640">640 px</option><option :value="1024">1024 px</option></select></label>
            <label>{{ t("qr.foreground") }}<span class="color-field"><input v-model="options.foreground" type="color" /><code>{{ options.foreground }}</code></span></label>
            <label>{{ t("qr.background") }}<span class="color-field"><input v-model="options.background" type="color" :disabled="options.background === 'transparent'" /><code>{{ options.background === "transparent" ? t("qr.transparent") : options.background }}</code></span></label>
          </div>
          <div class="style-actions">
            <label><input v-model="options.quietZone" type="checkbox" />{{ t("qr.quietZone") }}</label>
            <label><input type="checkbox" :checked="options.background === 'transparent'" @change="options.background = ($event.target as HTMLInputElement).checked ? 'transparent' : '#ffffff'" />{{ t("qr.transparentBackground") }}</label>
          </div>
          <div class="color-presets">
            <button v-for="preset in colorPresets" :key="`${preset.foreground}-${preset.background}`" type="button" :title="`${preset.foreground} / ${preset.background}`" :style="{ background: preset.background }" @click="applyPreset(preset)"><i :style="{ background: preset.foreground }"></i></button>
          </div>
          <p>{{ t("qr.reliabilityNote") }}</p>
        </section>
      </article>

      <aside class="preview-column">
        <section class="preview-card">
          <header class="section-head">
            <div><small>LIVE PREVIEW</small><h2>{{ t("qr.preview") }}</h2></div>
            <span v-if="result">V{{ result.version }} · {{ result.modules }}×{{ result.modules }}</span>
          </header>
          <div class="qr-preview" :class="{ transparent: options.background === 'transparent' }">
            <div v-if="result" class="svg-holder" v-html="result.svg"></div>
            <div v-else class="preview-empty"><b>▦</b><span>{{ t("qr.previewEmpty") }}</span></div>
          </div>
          <div class="export-bar">
            <button type="button" :disabled="!result" @click="copyPng">{{ copied === "png" ? t("common.copied") : t("qr.copyPng") }}</button>
            <button type="button" :disabled="!result" @click="copySvg">{{ copied === "svg" ? t("common.copied") : t("qr.copySvg") }}</button>
            <button type="button" :disabled="!result" @click="downloadSvg">{{ t("qr.exportSvg") }}</button>
            <button class="primary" type="button" :disabled="!result" @click="downloadPng">{{ t("qr.exportPng") }}</button>
          </div>
        </section>

        <section class="history-card">
          <header class="section-head">
            <div><small>RECENT</small><h2>{{ t("qr.history") }}</h2></div>
            <button v-if="history.length" type="button" @click="clearHistory">{{ t("common.clear") }}</button>
          </header>
          <div v-if="history.length" class="history-list">
            <button v-for="item in history" :key="item.id" type="button" @click="useHistory(item)">
              <i>{{ contentTypes.find((type) => type.id === item.type)?.icon }}</i>
              <span><strong>{{ item.label }}</strong><small>{{ t(`qr.types.${item.type}`) }} · {{ new Date(item.createdAt).toLocaleString() }}</small></span>
            </button>
          </div>
          <div v-else class="history-empty">{{ t("qr.historyEmpty") }}</div>
        </section>
      </aside>
    </section>
  </main>
</template>

<style scoped>
.hidden-file{display:none}.qr-page{display:grid;gap:12px;padding:20px 28px 36px}.type-tabs{display:grid;grid-template-columns:repeat(7,1fr);border:1px solid var(--color-border);background:var(--color-panel-translucent)}.type-tabs button{display:flex;min-height:50px;align-items:center;justify-content:center;gap:7px;border:0;border-right:1px solid var(--color-border);background:transparent;color:var(--color-text-muted);font-size:8px}.type-tabs button:last-child{border-right:0}.type-tabs button:hover{background:var(--color-bg-muted);color:var(--color-text-primary)}.type-tabs button.active{box-shadow:inset 0 -2px var(--color-accent);background:var(--color-selected-bg);color:var(--color-text-primary)}.type-tabs i{display:grid;width:23px;height:23px;place-items:center;border:1px solid var(--color-border);border-radius:50%;font:8px "SFMono-Regular",monospace;font-style:normal}.type-tabs .active i{border-color:var(--color-accent);color:var(--color-accent)}.qr-workspace{display:grid;grid-template-columns:minmax(460px,1.06fr) minmax(390px,.94fr);gap:12px}.editor-column,.preview-card,.history-card{min-width:0;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.preview-column{display:grid;align-content:start;gap:12px}.section-head{display:flex;min-height:52px;align-items:center;justify-content:space-between;gap:10px;padding:8px 13px;border-bottom:1px solid var(--color-border);background:var(--color-bg-muted)}.section-head small,.style-panel>header small{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace;letter-spacing:.11em}.section-head h2,.style-panel h3{margin:4px 0 0;font-size:10px}.section-head>span{color:var(--color-text-muted);font:7px "SFMono-Regular",monospace}.section-head>span.danger{color:var(--color-danger-text)}.content-editor{min-height:176px}.content-editor>textarea{box-sizing:border-box;width:100%;min-height:176px;padding:13px;border:0;background:transparent;color:var(--color-text-primary);font:8px/1.7 "SFMono-Regular",monospace;resize:vertical}.form-fields{display:grid;grid-template-columns:1fr 1fr;gap:10px;padding:13px}.form-fields.single{grid-template-columns:1fr}.form-fields label{display:grid;gap:5px;color:var(--color-text-muted);font-size:7px}.form-fields label.wide{grid-column:1/-1}.form-fields label.check{display:flex;align-items:center;gap:6px}.form-fields input:not([type=checkbox]),.form-fields select{box-sizing:border-box;width:100%;height:32px;padding:0 8px;font-size:8px}.form-fields textarea{box-sizing:border-box;width:100%;min-height:72px;padding:8px;font-size:8px;resize:vertical}.encoded-preview{border-top:1px solid var(--color-border);background:var(--color-code-bg)}.encoded-preview header{display:flex;align-items:center;justify-content:space-between;padding:6px 11px}.encoded-preview header small{color:var(--color-text-muted);font:6px "SFMono-Regular",monospace}.encoded-preview header button{height:22px;padding:0 7px;border:0;background:transparent;font-size:6px}.encoded-preview>code{display:block;min-height:38px;max-height:70px;overflow:auto;padding:0 11px 8px;color:var(--color-text-secondary);font:6px/1.55 "SFMono-Regular",monospace;overflow-wrap:anywhere}.encoded-preview>i{display:block;height:2px;background:var(--color-border)}.encoded-preview>i>b{display:block;height:100%;background:var(--color-accent);transition:width .2s}.style-panel{border-top:1px solid var(--color-border)}.style-panel>header{padding:10px 13px}.option-grid{display:grid;grid-template-columns:repeat(4,1fr);border-top:1px solid var(--color-border);border-bottom:1px solid var(--color-border)}.option-grid>label{display:grid;gap:5px;padding:9px;border-right:1px solid var(--color-border);color:var(--color-text-muted);font-size:7px}.option-grid>label:last-child{border-right:0}.option-grid select{height:29px;padding:0 7px;font-size:7px}.color-field{display:flex;height:28px;align-items:center;border:1px solid var(--color-border);background:var(--color-input-bg)}.color-field input{width:32px;height:100%;padding:2px;border:0;background:transparent}.color-field code{overflow:hidden;padding:0 5px;font-size:6px;text-overflow:ellipsis;white-space:nowrap}.style-actions{display:flex;gap:18px;padding:9px 12px;color:var(--color-text-secondary);font-size:7px}.style-actions label{display:flex;align-items:center;gap:6px}.color-presets{display:flex;gap:6px;padding:0 12px 9px}.color-presets button{display:grid;width:28px;height:28px;padding:0;place-items:center;border:1px solid var(--color-border);border-radius:50%}.color-presets i{width:12px;height:12px;border-radius:3px}.style-panel>p{margin:0;padding:8px 12px;border-top:1px solid var(--color-border);color:var(--color-text-muted);font-size:7px;line-height:1.55}.qr-preview{display:grid;min-height:360px;place-items:center;padding:24px;background:#f3f1eb}.qr-preview.transparent{background-color:#f1f1ed;background-image:linear-gradient(45deg,#d8d8d3 25%,transparent 25%),linear-gradient(-45deg,#d8d8d3 25%,transparent 25%),linear-gradient(45deg,transparent 75%,#d8d8d3 75%),linear-gradient(-45deg,transparent 75%,#d8d8d3 75%);background-position:0 0,0 8px,8px -8px,-8px 0;background-size:16px 16px}.svg-holder{width:min(310px,100%);line-height:0;filter:drop-shadow(0 8px 18px rgba(0,0,0,.1))}.svg-holder :deep(svg){display:block;width:100%;height:auto}.preview-empty{display:grid;justify-items:center;gap:9px;color:#777;font-size:8px}.preview-empty b{font-size:28px}.export-bar{display:flex;justify-content:flex-end;gap:6px;padding:9px 11px;border-top:1px solid var(--color-border)}.export-bar button{height:28px;padding:0 8px;font-size:7px}.history-card .section-head button{height:24px;padding:0 7px;border:0;background:transparent;font-size:6px}.history-list{max-height:230px;overflow:auto}.history-list button{display:grid;width:100%;grid-template-columns:28px minmax(0,1fr);align-items:center;gap:8px;padding:8px 10px;border:0;border-bottom:1px solid var(--color-border);background:transparent;color:var(--color-text-primary);text-align:left}.history-list button:hover{background:var(--color-bg-muted)}.history-list i{display:grid;width:24px;height:24px;place-items:center;border:1px solid var(--color-border);border-radius:50%;color:var(--color-accent);font:7px "SFMono-Regular",monospace;font-style:normal}.history-list strong,.history-list small{display:block;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.history-list strong{font-size:7px}.history-list small{margin-top:3px;color:var(--color-text-muted);font-size:6px}.history-empty{display:grid;min-height:100px;place-items:center;color:var(--color-text-muted);font-size:7px}@media(max-width:1100px){.qr-page{padding:16px}.qr-workspace{grid-template-columns:1fr}.type-tabs{grid-template-columns:repeat(4,1fr)}.type-tabs button:nth-child(4){border-right:0}.option-grid{grid-template-columns:repeat(2,1fr)}.option-grid>label:nth-child(2){border-right:0}.qr-preview{min-height:310px}}
</style>
