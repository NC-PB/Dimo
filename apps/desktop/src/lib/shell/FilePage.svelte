<script lang="ts">
  import { command, type CommandId } from "$lib/commands.svelte";
  import { m } from "$lib/i18n";
  import CommandButton from "./CommandButton.svelte";

  /**
   * The File page until T3.16 builds Info, Recent and Options (ADR 0007): the project commands
   * and the way to the settings page.
   */
  const ITEMS: readonly CommandId[] = [
    "new_project",
    "open_project",
    "save",
    "save_as",
    "settings",
    "help",
  ];
</script>

<div
  id="tab-panel"
  class="flex min-h-0 flex-1 flex-col gap-4 overflow-auto bg-bg p-6 text-sm text-text"
  role="tabpanel"
  aria-labelledby="tab-file"
>
  <h1 class="text-lg font-semibold">{m.file_title()}</h1>
  <ul class="flex max-w-xl flex-col gap-1 rounded-lg border border-border bg-surface p-2">
    {#each ITEMS as id (id)}
      <li class="flex flex-wrap items-center gap-x-3">
        <CommandButton {id} />
        <span class="text-xs text-text-muted">{command(id).description()}</span>
      </li>
    {/each}
  </ul>
  <p class="max-w-xl text-xs text-text-muted">{m.file_hint()}</p>
</div>
