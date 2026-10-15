<script lang="ts">
  import BrandMark from "$lib/components/BrandMark.svelte";
  import { autosaveFailedText, autosaveLabel, m } from "$lib/i18n";
  import { icons } from "$lib/icons";
  import { projectStore } from "$lib/stores/project.svelte";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import { effectiveTheme } from "$lib/theme/theme";
  import CommandButton from "./CommandButton.svelte";

  /**
   * Title bar inside the window (ADR 0007): brand mark, Save, Undo, Redo, the document name with
   * its saved state, the light/dark switch and help. Icon only buttons are limited to these
   * well known actions; each has a tooltip and an accessible name (decision 26).
   */
  const status = $derived(projectStore.status);
  const name = $derived(status?.file_name ?? m.untitled_project());

  /** Saved state in words, with a shape for the failed autosave (NFR-UX-05). */
  const saved = $derived.by(() => {
    if (status === null) {
      return null;
    }
    if (status.autosave.state === "failed") {
      // The diagnostic text goes into the tooltip, under "Details" (MF F3).
      return {
        text: autosaveFailedText(status.autosave.reason),
        failed: true,
        title: `${autosaveFailedText(status.autosave.reason)}\n${m.error_details()}: ${status.autosave.message}`,
      };
    }
    if (!status.modified) {
      // A new project without changes has no file yet: it is not saved.
      return {
        text: status.file_name === null ? m.doc_not_saved_yet() : m.doc_saved(),
        failed: false,
      };
    }
    return {
      text: status.autosave.state === "journaled" ? m.doc_autosaved() : m.doc_not_saved(),
      failed: false,
    };
  });

  /** The switch shows where it goes: the moon in light, the sun in dark. */
  let systemDark = $state(
    typeof window !== "undefined" && typeof window.matchMedia === "function"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
      : false,
  );
  const dark = $derived(effectiveTheme(settingsStore.current.theme, systemDark) === "dark");

  $effect(() => {
    if (typeof window.matchMedia !== "function") {
      return;
    }
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => {
      systemDark = query.matches;
    };
    query.addEventListener("change", update);
    return () => {
      query.removeEventListener("change", update);
    };
  });
</script>

<header class="flex h-11 shrink-0 items-center gap-1 border-b border-border bg-surface px-3">
  <div class="mr-2.5">
    <BrandMark />
  </div>
  <div class="flex items-center gap-0.5" role="toolbar" aria-label={m.quick_access_label()}>
    <CommandButton id="save" size="icon" />
    <CommandButton id="undo" size="icon" />
    <CommandButton id="redo" size="icon" />
  </div>
  <div
    class="min-w-0 flex-1 truncate text-center text-[13px] text-text-muted"
    role="status"
    aria-live="polite"
    data-testid="document-name"
  >
    {#if saved === null}
      {m.doc_none()}
    {:else}
      <strong class="font-semibold text-text">{name}</strong>
      <span aria-hidden="true"> · </span>
      <span
        class={saved.failed ? "text-danger" : ""}
        title={"title" in saved
          ? saved.title
          : status
            ? autosaveLabel(status.autosave, status.file_name !== null)
            : undefined}
        data-saved={!status?.modified}
      >
        {#if saved.failed}<span aria-hidden="true">⚠ </span>{/if}{saved.text}
      </span>
    {/if}
  </div>
  <CommandButton id="toggle_theme" size="icon" icon={dark ? icons.themeLight : icons.themeDark} />
  <CommandButton id="help" size="icon" />
</header>
