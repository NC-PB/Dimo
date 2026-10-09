<script lang="ts">
  import { ToggleGroup } from "bits-ui";
  import ProjectActions from "$lib/components/ProjectActions.svelte";
  import { LOCALE_NAMES, getLocale, locales, m, setLocale, viewLabel } from "$lib/i18n";
  import { VIEWS, view } from "$lib/stores/view.svelte";

  function onLocaleChange(event: Event & { currentTarget: HTMLSelectElement }) {
    const next = locales.find((l) => l === event.currentTarget.value);
    if (next) {
      // Reloads the window so every message is rendered in the new language.
      setLocale(next);
    }
  }
</script>

<header class="flex items-center gap-4 border-b border-border bg-surface px-3 py-2">
  <span class="font-semibold">{m.app_title()}</span>

  <ProjectActions />

  <ToggleGroup.Root
    type="single"
    bind:value={() => view.current, (v) => view.set(v)}
    aria-label={m.view_switch_label()}
    class="flex gap-1"
  >
    {#each VIEWS as v (v)}
      <ToggleGroup.Item
        value={v}
        class="rounded px-3 py-1 text-sm text-text-muted hover:bg-surface-raised data-[state=on]:bg-accent data-[state=on]:text-accent-text"
      >
        {viewLabel(v)}
      </ToggleGroup.Item>
    {/each}
  </ToggleGroup.Root>

  <label class="ml-auto flex items-center gap-2 text-sm text-text-muted">
    {m.language_label()}
    <select
      class="rounded border border-border bg-surface px-2 py-1 text-text"
      value={getLocale()}
      onchange={onLocaleChange}
    >
      {#each locales as l (l)}
        <option value={l}>{LOCALE_NAMES[l]}</option>
      {/each}
    </select>
  </label>
</header>
