import { createApp } from "vue";
import App from "./App.vue";
import { initializeTheme } from "./theme";
import { initializeUiScale } from "./display";
import "./theme.css";
import "./styles.css";

initializeTheme();
initializeUiScale();
createApp(App).mount("#app");
