<script lang="ts">
  /**
   * Brand mark (M3 decision 24): a rounded square with a crosshair and an orange dot, next to
   * the product name in Space Grotesk. The outline follows the text color, so it works in light
   * and dark; the dot keeps the brand orange.
   */
  import { m } from "$lib/i18n";

  interface Props {
    /** Height of the mark in pixels; the name scales with it. */
    size?: number;
  }

  let { size = 22 }: Props = $props();
</script>

<span class="brand inline-flex items-center gap-2 text-text" style:--brand-size="{size}px">
  <svg
    class="mark"
    viewBox="10 10 44 44"
    width={size}
    height={size}
    aria-hidden="true"
    focusable="false"
  >
    <rect
      x="12"
      y="12"
      width="40"
      height="40"
      rx="8"
      fill="none"
      stroke="currentColor"
      stroke-width="4"
    />
    <path d="M32 12v11M32 41v11M12 32h11M41 32h11" stroke="currentColor" stroke-width="4" />
    <circle cx="32" cy="32" r="6" fill="#e85d04" />
  </svg>
  <span class="name">{m.app_title()}</span>
</span>

<style>
  .mark {
    flex: none;
  }

  .name {
    font-family: var(--font-heading);
    font-weight: 700;
    font-size: calc(var(--brand-size) * 0.8);
    line-height: 1;
    letter-spacing: -0.01em;
  }
</style>
