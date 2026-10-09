import { paraglideVitePlugin } from "@inlang/paraglide-js";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vitest/config";

// Tauri expects a fixed port and must see Rust errors in the terminal.
export default defineConfig(({ mode }) => ({
  plugins: [
    paraglideVitePlugin({
      project: "./project.inlang",
      outdir: "./src/lib/paraglide",
      strategy: ["localStorage", "preferredLanguage", "baseLocale"],
    }),
    svelte(),
    tailwindcss(),
  ],
  resolve: {
    // Root relative, resolved by Vite from the project root.
    alias: { $lib: "/src/lib" },
    // Component tests mount Svelte in a simulated browser (happy-dom), so they need the browser
    // build of Svelte, not the server one.
    ...(mode === "test" ? { conditions: ["browser"] } : {}),
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  test: {
    include: ["src/**/*.test.ts"],
  },
}));
