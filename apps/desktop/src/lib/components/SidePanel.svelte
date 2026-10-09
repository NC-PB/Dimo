<script lang="ts">
  import DevPanel from "$lib/components/DevPanel.svelte";
  import SelectionSummary from "$lib/components/SelectionSummary.svelte";
  import SheetProperties from "$lib/components/SheetProperties.svelte";
  import { DEV_TOOLS_ENABLED } from "$lib/dev/dev-tools.svelte";
  import { buildProfileLabel, m } from "$lib/i18n";
  import { appInfo } from "$lib/stores/app-info.svelte";
</script>

<aside
  class="flex w-72 shrink-0 flex-col gap-2 border-l border-border bg-surface p-3 text-sm text-text-muted"
  aria-label={m.side_panel_label()}
>
  <SheetProperties />
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
