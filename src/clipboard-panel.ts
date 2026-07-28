import { createApp } from "vue";
import ClipboardQuickPanel from "./components/ClipboardQuickPanel.vue";
import { initializeTheme } from "./theme";
import "./theme.css";

initializeTheme();
createApp(ClipboardQuickPanel).mount("#clipboard-panel");
