import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { getLocale } from "./lib/i18n";
import { applyStartupSettings, settingsStore } from "./lib/stores/settings.svelte";
import { themeController } from "./lib/theme/theme";

const target = document.getElementById("app");
if (!target) {
  throw new Error("missing #app element in index.html");
}

// Follow the system theme until the stored settings are read.
themeController.apply("system");
// The stored language and theme apply before the first render (T1.9).
await settingsStore.load();
applyStartupSettings(settingsStore.current);
document.documentElement.lang = getLocale();

export default mount(App, { target });
