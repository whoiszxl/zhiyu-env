<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { generateQrCode } from "../../api/tools";
import type { QrCodeResult } from "../../types";

interface DetectedBarcode {
  rawValue: string;
}

interface BarcodeDetectorInstance {
  detect(source: Blob): Promise<DetectedBarcode[]>;
}

interface BarcodeDetectorConstructor {
  new (options: { formats: string[] }): BarcodeDetectorInstance;
}

const content = ref("https://github.com/");
const errorCorrection = ref("M");
const size = ref(320);
const result = ref<QrCodeResult | null>(null);
const generating = ref(false);
const error = ref("");
const scanning = ref(false);
const scanSupported = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);
const contentBytes = computed(() => new TextEncoder().encode(content.value).length);

async function generate() {
  if (generating.value) return;
  generating.value = true;
  error.value = "";
  try {
    result.value = await generateQrCode(content.value, errorCorrection.value, size.value);
  } catch (cause) {
    result.value = null;
    error.value = String(cause);
  } finally {
    generating.value = false;
  }
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
  if (!result.value) return;
  downloadBlob(new Blob([result.value.svg], { type: "image/svg+xml" }), "zhiyu-qrcode.svg");
}

async function downloadPng() {
  if (!result.value) return;
  const svgUrl = URL.createObjectURL(
    new Blob([result.value.svg], { type: "image/svg+xml" }),
  );
  try {
    const image = new Image();
    image.src = svgUrl;
    await image.decode();
    const canvas = document.createElement("canvas");
    canvas.width = size.value;
    canvas.height = size.value;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("无法创建图片画布");
    context.fillStyle = "#ffffff";
    context.fillRect(0, 0, canvas.width, canvas.height);
    context.drawImage(image, 0, 0, canvas.width, canvas.height);
    const blob = await new Promise<Blob | null>((resolve) =>
      canvas.toBlob(resolve, "image/png"),
    );
    if (!blob) throw new Error("PNG 导出失败");
    downloadBlob(blob, "zhiyu-qrcode.png");
  } catch (cause) {
    error.value = String(cause);
  } finally {
    URL.revokeObjectURL(svgUrl);
  }
}

async function copySvg() {
  if (result.value) await navigator.clipboard.writeText(result.value.svg);
}

async function scanFile(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  const Detector = (
    window as unknown as { BarcodeDetector?: BarcodeDetectorConstructor }
  ).BarcodeDetector;
  if (!Detector) {
    error.value = "当前系统 WebView 不支持二维码图片识别";
    return;
  }
  scanning.value = true;
  error.value = "";
  try {
    const codes = await new Detector({ formats: ["qr_code"] }).detect(file);
    if (!codes.length) throw new Error("图片中没有识别到二维码");
    content.value = codes[0].rawValue;
    await generate();
  } catch (cause) {
    error.value = cause instanceof Error ? cause.message : String(cause);
  } finally {
    scanning.value = false;
  }
}

onMounted(() => {
  scanSupported.value =
    "BarcodeDetector" in (window as unknown as Record<string, unknown>);
  void generate();
});
</script>

<template>
  <header class="detail-header">
    <div class="detail-identity">
      <span class="service-logo qrcode">▦</span>
      <div>
        <div class="title-line"><h1>QR Code 工具</h1><span>LOCAL QR</span></div>
        <p>本地生成二维码，并在系统支持时识别二维码图片</p>
      </div>
    </div>
    <div class="header-actions">
      <input ref="fileInput" class="hidden-file" type="file" accept="image/*" @change="scanFile" />
      <button type="button" :disabled="scanning || !scanSupported" :title="scanSupported ? '选择二维码图片' : '当前系统不支持图片识别'" @click="fileInput?.click()">{{ scanning ? "识别中" : "识别图片" }}</button>
    </div>
  </header>

  <div v-if="error" class="notice danger"><span>{{ error }}</span><button type="button" @click="error = ''">×</button></div>

  <section class="qr-page">
    <div class="qr-layout">
      <article class="qr-panel editor-panel">
        <div class="panel-head"><div><p>CONTENT</p><h2>二维码内容</h2></div><span>{{ contentBytes }} / 4096 bytes</span></div>
        <textarea v-model="content" spellcheck="false" placeholder="文本、URL、邮箱或其他内容"></textarea>
        <div class="qr-options">
          <label>纠错等级<select v-model="errorCorrection"><option value="L">L · 约 7%</option><option value="M">M · 约 15%</option><option value="Q">Q · 约 25%</option><option value="H">H · 约 30%</option></select></label>
          <label>导出尺寸<select v-model.number="size"><option :value="240">240 px</option><option :value="320">320 px</option><option :value="480">480 px</option><option :value="640">640 px</option><option :value="1024">1024 px</option></select></label>
          <button class="primary" type="button" :disabled="generating" @click="generate">{{ generating ? "生成中" : "生成二维码" }}</button>
        </div>
        <p class="qr-note">内容只在本机编码，不会上传。纠错等级越高，二维码越密集，但污损后的可恢复能力更强。</p>
      </article>

      <article class="qr-panel preview-panel">
        <div class="panel-head"><div><p>PREVIEW</p><h2>二维码预览</h2></div><span v-if="result">{{ result.modules }} × {{ result.modules }} 模块</span></div>
        <div class="qr-preview">
          <div v-if="result" class="svg-holder" v-html="result.svg"></div>
          <span v-else>输入内容后生成二维码</span>
        </div>
        <div class="export-bar">
          <button type="button" :disabled="!result" @click="copySvg">复制 SVG</button>
          <button type="button" :disabled="!result" @click="downloadSvg">导出 SVG</button>
          <button class="primary" type="button" :disabled="!result" @click="downloadPng">导出 PNG</button>
        </div>
      </article>
    </div>
  </section>
</template>

<style scoped>
.hidden-file{display:none}.qr-page{padding:24px 32px 36px}.qr-layout{display:grid;grid-template-columns:minmax(360px,1fr) minmax(380px,.9fr);gap:14px}.qr-panel{overflow:hidden;border:1px solid var(--color-border);background:var(--color-panel-translucent)}.panel-head{display:flex;min-height:62px;align-items:center;justify-content:space-between;gap:14px;padding:11px 15px;border-bottom:1px solid var(--color-border)}.panel-head p{margin:0 0 4px;color:var(--color-text-muted);font:8px "SFMono-Regular",Consolas,monospace;letter-spacing:.12em}.panel-head h2{margin:0;font-size:14px}.panel-head>span{color:var(--color-text-muted);font-size:8px}.editor-panel textarea{display:block;width:100%;min-height:280px;padding:14px 15px;resize:vertical;border:0;outline:0;background:transparent;font:10px/1.7 "SFMono-Regular",Consolas,monospace}.qr-options{display:grid;grid-template-columns:1fr 1fr auto;align-items:end;gap:10px;padding:13px 15px;border-top:1px solid var(--color-border);background:var(--color-bg-muted)}.qr-options label{display:grid;gap:5px;color:var(--color-text-secondary);font-size:8px}.qr-options select{height:32px;padding:0 8px;font-size:9px}.qr-options button{min-height:32px}.qr-note{margin:0;padding:11px 15px;color:var(--color-text-muted);font-size:8px;line-height:1.65}.qr-preview{display:grid;min-height:410px;place-items:center;padding:24px;background:#f1f0eb;color:#777;font-size:9px}.svg-holder{width:min(330px,100%);line-height:0}.svg-holder :deep(svg){display:block;width:100%;height:auto}.export-bar{display:flex;justify-content:flex-end;gap:7px;padding:10px 13px;border-top:1px solid var(--color-border)}.export-bar button{min-height:31px;padding:0 11px;font-size:9px}@media(max-width:1000px){.qr-layout{grid-template-columns:1fr}.qr-options{grid-template-columns:1fr 1fr}.qr-options button{grid-column:1/-1}}
</style>
