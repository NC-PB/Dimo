import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";
import { getLocale } from "./lib/i18n";

document.documentElement.lang = getLocale();

const target = document.getElementById("app");
if (!target) {
  throw new Error("missing #app element in index.html");
}

export default mount(App, { target });
