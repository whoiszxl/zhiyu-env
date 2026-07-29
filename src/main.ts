import { createApp } from "vue";
import App from "./App.vue";
import { initializeTheme } from "./theme";
import { initializeUiScale } from "./display";
import { i18n, initializeI18n } from "./i18n";
import { toolI18nDirective } from "./i18n/toolUi";
import "./theme.css";
import "./styles.css";

initializeTheme();
initializeUiScale();
await initializeI18n();
createApp(App).use(i18n).directive("tool-i18n", toolI18nDirective).mount("#app");
