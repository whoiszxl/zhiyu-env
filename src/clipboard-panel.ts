import { createApp } from "vue";
import ClipboardQuickPanel from "./components/ClipboardQuickPanel.vue";
import { initializeTheme } from "./theme";
import { i18n, initializeI18n } from "./i18n";
import { toolI18nDirective } from "./i18n/toolUi";
import "./theme.css";

initializeTheme();
await initializeI18n();
createApp(ClipboardQuickPanel)
  .use(i18n)
  .directive("tool-i18n", toolI18nDirective)
  .mount("#clipboard-panel");
