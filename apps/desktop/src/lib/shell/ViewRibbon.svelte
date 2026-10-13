<script lang="ts">
  import { m, themeLabel } from "$lib/i18n";
  import type { Theme } from "$lib/ipc/bindings";
  import { settingsStore } from "$lib/stores/settings.svelte";
  import CommandButton from "./CommandButton.svelte";
  import { FIELD_CLASS, LABEL_CLASS } from "./field";
  import Ribbon from "./Ribbon.svelte";

  /**
   * View tab (ADR 0007): zoom, the panes around the drawing and the theme (D-51). The panes can
   * be hidden to give the drawing more room on small screens.
   */
  const THEMES: readonly Theme[] = ["system", "light", "dark"];
</script>

{#snippet zoom()}
  <CommandButton id="zoom_in" size="large" />
  <CommandButton id="zoom_out" size="large" />
  <CommandButton id="fit" size="large" />
{/snippet}

{#snippet panes()}
  <CommandButton id="show_table" size="large" />
  <CommandButton id="show_side_panel" size="large" />
{/snippet}

{#snippet look()}
  <label class={LABEL_CLASS} title={m.field_theme_desc()}>
    {m.theme_label()}
    <select
      class="{FIELD_CLASS} min-w-40"
      data-field="theme"
      value={settingsStore.current.theme}
      onchange={(e) => {
        const theme = THEMES.find((t) => t === e.currentTarget.value);
        if (theme) {
          void settingsStore.update({ theme });
        }
      }}
    >
      {#each THEMES as theme (theme)}
        <option value={theme}>{themeLabel(theme)}</option>
      {/each}
    </select>
  </label>
{/snippet}

<Ribbon
  groups={[
    { id: "zoom", label: m.ribbon_group_zoom(), content: zoom },
    { id: "panes", label: m.ribbon_group_panes(), content: panes },
    { id: "look", label: m.ribbon_group_look(), content: look },
  ]}
/>
