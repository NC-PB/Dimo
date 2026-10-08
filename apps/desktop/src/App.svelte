<script lang="ts">
  import SidePanel from "$lib/components/SidePanel.svelte";
  import Toolbar from "$lib/components/Toolbar.svelte";
  import Viewport from "$lib/components/Viewport.svelte";
</script>

<div class="flex h-full flex-col">
  <Toolbar />
  <div class="flex min-h-0 flex-1">
    <Viewport />
    <SidePanel />
  </div>
</div>
