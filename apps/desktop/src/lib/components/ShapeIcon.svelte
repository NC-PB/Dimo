<script lang="ts">
  import type { BalloonShape } from "$lib/ipc/bindings";

  interface Props {
    shape: BalloonShape;
  }

  let { shape }: Props = $props();
</script>

<!-- Small outline of a balloon shape next to its name; decorative. -->
<svg viewBox="0 0 24 16" class="h-3 w-5" aria-hidden="true">
  {#if shape === "circle"}
    <circle cx="12" cy="8" r="6.5" fill="none" stroke="currentColor" />
  {:else if shape === "flag"}
    <polygon points="8,1.5 22,1.5 22,14.5 8,14.5 2,8" fill="none" stroke="currentColor" />
  {:else}
    <rect x="2" y="1.5" width="20" height="13" fill="none" stroke="currentColor" />
  {/if}
</svg>
