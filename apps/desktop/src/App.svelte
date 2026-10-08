<script lang="ts">
  import SidePanel from "$lib/components/SidePanel.svelte";
  import Toolbar from "$lib/components/Toolbar.svelte";
  import Viewport from "$lib/components/Viewport.svelte";
  import { appInfo } from "$lib/stores/app-info.svelte";
  import { onMount } from "svelte";

  onMount(() => {
    void appInfo.load();
  });
</script>

<div class="flex h-full flex-col">
  <Toolbar />
  <div class="flex min-h-0 flex-1">
    <Viewport />
    <SidePanel />
  </div>
</div>
