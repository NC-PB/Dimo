<script lang="ts">
  import DrawingToolbar from "$lib/components/DrawingToolbar.svelte";
  import Shortcuts from "$lib/components/Shortcuts.svelte";
  import SidePanel from "$lib/components/SidePanel.svelte";
  import Toolbar from "$lib/components/Toolbar.svelte";
  import Viewport from "$lib/components/Viewport.svelte";
  import { DEV_TOOLS_ENABLED, devTools } from "$lib/dev/dev-tools.svelte";
  import { appInfo } from "$lib/stores/app-info.svelte";
  import { documentStore } from "$lib/stores/document.svelte";
  import { viewport } from "$lib/stores/viewport.svelte";
  import { onMount } from "svelte";

  let shortcutsOpen = $state(false);

  onMount(() => {
    void appInfo.load();
    if (DEV_TOOLS_ENABLED) {
      void devTools.startup(documentStore, viewport);
    }
  });
</script>

<div class="flex h-full flex-col">
  <Toolbar />
  <div class="flex min-h-0 flex-1">
    <div class="flex min-w-0 flex-1 flex-col">
      <DrawingToolbar
        onShowShortcuts={() => {
          shortcutsOpen = true;
        }}
      />
      <Viewport />
    </div>
    <SidePanel />
  </div>
</div>
<Shortcuts bind:open={shortcutsOpen} />
