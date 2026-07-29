import { createI18n } from "vue-i18n";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { AppLocale } from "../types";
import enUS from "./locales/en-US";
import zhCN from "./locales/zh-CN";

const LOCALE_STORAGE_KEY = "zhiyu.environment.locale";
type ResolvedLocale = Exclude<AppLocale, "system">;

function isAppLocale(value: string | null): value is AppLocale {
  return value === "system" || value === "zh-CN" || value === "en-US";
}

export function resolveLocale(locale: AppLocale): ResolvedLocale {
  if (locale !== "system") return locale;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en-US";
}

export function storedLocale(): AppLocale {
  const locale = localStorage.getItem(LOCALE_STORAGE_KEY);
  return isAppLocale(locale) ? locale : "zh-CN";
}

export const i18n = createI18n({
  legacy: false,
  locale: "zh-CN",
  fallbackLocale: "zh-CN",
  messages: { "zh-CN": zhCN, "en-US": enUS },
  missingWarn: import.meta.env.DEV,
  fallbackWarn: import.meta.env.DEV,
});

export async function setAppLocale(
  preference: AppLocale,
  persist = true,
): Promise<ResolvedLocale> {
  const locale = resolveLocale(preference);
  i18n.global.locale.value = locale;
  document.documentElement.lang = locale;
  const currentWindow = getCurrentWindow();
  const isClipboardPanel = currentWindow.label === "clipboard-panel";
  await currentWindow
    .setTitle(
      locale === "en-US"
        ? isClipboardPanel
          ? "Clipboard"
          : "Zhiyu"
        : isClipboardPanel
          ? "剪贴板"
          : "智屿",
    )
    .catch(() => undefined);
  window.dispatchEvent(
    new CustomEvent("zhiyu:locale-changed", { detail: { locale } }),
  );
  if (persist) localStorage.setItem(LOCALE_STORAGE_KEY, preference);
  return locale;
}

export async function initializeI18n() {
  await setAppLocale(storedLocale(), false);
}
