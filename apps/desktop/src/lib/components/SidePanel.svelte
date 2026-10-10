<script lang="ts">
  import DevPanel from "$lib/components/DevPanel.svelte";
  import NumberingPanel from "$lib/components/NumberingPanel.svelte";
  import SelectionSummary from "$lib/components/SelectionSummary.svelte";
  import SheetProperties from "$lib/components/SheetProperties.svelte";
  import { DEV_TOOLS_ENABLED } from "$lib/dev/dev-tools.svelte";
  import { buildProfileLabel, m } from "$lib/i18n";
  import { appInfo } from "$lib/stores/app-info.svelte";
</script>

<aside
  class="flex min-h-0 w-72 shrink-0 flex-col gap-2 overflow-y-auto border-l border-border bg-surface p-3 text-sm text-text-muted"
  aria-label={m.side_panel_label()}
>
  <SheetProperties />
  <NumberingPanel />
  <SelectionSummary />
  {#if DEV_TOOLS_ENABLED}
    <DevPanel />
  {/if}
  {#if appInfo.current}
    {@const info = appInfo.current}
    <p class="text-xs">
      {m.app_version({ version: info.version })}, {buildProfileLabel(info.build_profile)}
    </p>
  {/if}
</aside>
