import { getCurrentWebview } from "@tauri-apps/api/webview";
import type { UiScale } from "./types";

const UI_SCALE_STORAGE_KEY = "zhiyu-env.ui-scale";
const SUPPORTED_SCALES: UiScale[] = [90, 100, 110, 120];

function parseUiScale(value: string | null): UiScale {
  const scale = Number(value);
  return SUPPORTED_SCALES.includes(scale as UiScale)
    ? (scale as UiScale)
    : 100;
}

export function applyUiScale(scale: UiScale, persist = true) {
  if (persist) {
    localStorage.setItem(UI_SCALE_STORAGE_KEY, String(scale));
  }
  try {
    void getCurrentWebview()
      .setZoom(scale / 100)
      .catch(() => {
        // 普通浏览器预览没有原生 Webview，使用 CSS zoom 保持可预览。
        document.documentElement.style.zoom = String(scale / 100);
      });
  } catch {
    document.documentElement.style.zoom = String(scale / 100);
  }
}

export function initializeUiScale() {
  applyUiScale(
    parseUiScale(localStorage.getItem(UI_SCALE_STORAGE_KEY)),
    false,
  );
}
