import { ref } from "vue";
import type { ColorTheme, ThemeMode } from "./types";

const THEME_STORAGE_KEY = "zhiyu-env.theme-mode";
const COLOR_THEME_STORAGE_KEY = "zhiyu-env.color-theme";
const DARK_QUERY = "(prefers-color-scheme: dark)";

export const themeMode = ref<ThemeMode>("system");
export const colorTheme = ref<ColorTheme>("classic");
export const resolvedTheme = ref<"light" | "dark">("light");

let mediaQuery: MediaQueryList | null = null;

function isThemeMode(value: string | null): value is ThemeMode {
  return value === "system" || value === "light" || value === "dark";
}

function isColorTheme(value: string | null): value is ColorTheme {
  return (
    value === "classic" ||
    value === "ocean" ||
    value === "forest" ||
    value === "sand" ||
    value === "twilight" ||
    value === "aurora" ||
    value === "graphite" ||
    value === "coral" ||
    value === "sunset" ||
    value === "neon"
  );
}

function resolveTheme(mode: ThemeMode): "light" | "dark" {
  if (mode !== "system") return mode;
  return window.matchMedia(DARK_QUERY).matches ? "dark" : "light";
}

function updateDocument(mode: ThemeMode) {
  const resolved = resolveTheme(mode);
  resolvedTheme.value = resolved;
  document.documentElement.dataset.theme = resolved;
  document.documentElement.dataset.palette = colorTheme.value;
  document.documentElement.style.colorScheme = resolved;
  const background = getComputedStyle(document.documentElement)
    .getPropertyValue("--color-bg-app")
    .trim();
  document
    .querySelector('meta[name="theme-color"]')
    ?.setAttribute(
      "content",
      background || (resolved === "dark" ? "#1d201b" : "#f4f2ec"),
    );
}

export function setThemeMode(mode: ThemeMode, persist = true) {
  themeMode.value = mode;
  updateDocument(mode);
  if (persist) localStorage.setItem(THEME_STORAGE_KEY, mode);
}

export function setColorTheme(theme: ColorTheme, persist = true) {
  colorTheme.value = theme;
  document.documentElement.dataset.palette = theme;
  updateDocument(themeMode.value);
  if (persist) localStorage.setItem(COLOR_THEME_STORAGE_KEY, theme);
}

export function initializeTheme() {
  const stored = localStorage.getItem(THEME_STORAGE_KEY);
  const storedColorTheme = localStorage.getItem(COLOR_THEME_STORAGE_KEY);
  colorTheme.value = isColorTheme(storedColorTheme) ? storedColorTheme : "classic";
  setThemeMode(isThemeMode(stored) ? stored : "system", false);

  mediaQuery = window.matchMedia(DARK_QUERY);
  mediaQuery.addEventListener("change", () => {
    if (themeMode.value === "system") updateDocument("system");
  });
}
