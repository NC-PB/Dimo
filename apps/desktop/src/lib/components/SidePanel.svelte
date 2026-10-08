<script lang="ts">
  import { buildProfileLabel, m } from "$lib/i18n";
  import { appInfo } from "$lib/stores/app-info.svelte";
</script>

<aside
  class="flex w-72 shrink-0 flex-col border-l border-border bg-surface p-3 text-sm text-text-muted"
  aria-label={m.side_panel_label()}
>
  <p class="flex-1">{m.side_panel_empty()}</p>
  {#if appInfo.current}
    {@const info = appInfo.current}
    <p class="text-xs">
      {m.app_version({ version: info.version })}, {buildProfileLabel(info.build_profile)}
    </p>
  {/if}
</aside>
