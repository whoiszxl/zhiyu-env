import { ref } from "vue";
import type { ThemeMode } from "./types";

const THEME_STORAGE_KEY = "zhiyu-env.theme-mode";
const DARK_QUERY = "(prefers-color-scheme: dark)";

export const themeMode = ref<ThemeMode>("system");
export const resolvedTheme = ref<"light" | "dark">("light");

let mediaQuery: MediaQueryList | null = null;

function isThemeMode(value: string | null): value is ThemeMode {
  return value === "system" || value === "light" || value === "dark";
}

function resolveTheme(mode: ThemeMode): "light" | "dark" {
  if (mode !== "system") return mode;
  return window.matchMedia(DARK_QUERY).matches ? "dark" : "light";
}

function updateDocument(mode: ThemeMode) {
  const resolved = resolveTheme(mode);
  resolvedTheme.value = resolved;
  document.documentElement.dataset.theme = resolved;
  document.documentElement.style.colorScheme = resolved;
  document
    .querySelector('meta[name="theme-color"]')
    ?.setAttribute("content", resolved === "dark" ? "#1d201b" : "#f4f2ec");
}

export function setThemeMode(mode: ThemeMode, persist = true) {
  themeMode.value = mode;
  updateDocument(mode);
  if (persist) localStorage.setItem(THEME_STORAGE_KEY, mode);
}

export function initializeTheme() {
  const stored = localStorage.getItem(THEME_STORAGE_KEY);
  setThemeMode(isThemeMode(stored) ? stored : "system", false);

  mediaQuery = window.matchMedia(DARK_QUERY);
  mediaQuery.addEventListener("change", () => {
    if (themeMode.value === "system") updateDocument("system");
  });
}
