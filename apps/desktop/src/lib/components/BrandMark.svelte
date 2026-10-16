<script lang="ts">
  /**
   * Brand mark (M3 decision 24, mark and wordmark from R.11 concept B "checked dimension"): an
   * orange balloon with a check mark above a dimension line, next to the wordmark "DimoMark" in
   * Space Grotesk with "Mark" in orange. Check and dimension follow the text color, so the mark
   * works in light and dark; the balloon keeps the brand orange. "Mark" uses the strong accent
   * for text contrast. The product name is not translated; `app_title` is the accessible name.
   */
  import { m } from "$lib/i18n";

  interface Props {
    /** Height of the mark in pixels; the name scales with it. */
    size?: number;
  }

  let { size = 22 }: Props = $props();
</script>

<span
  class="brand inline-flex items-center gap-2 text-text"
  style:--brand-size="{size}px"
  role="img"
  aria-label={m.app_title()}
>
  <svg
    class="mark"
    viewBox="160 160 704 704"
    width={size}
    height={size}
    aria-hidden="true"
    focusable="false"
  >
    <circle cx="512" cy="380" r="176" fill="none" stroke="#e85d04" stroke-width="60" />
    <path
      d="M432 384 L492 444 L600 324"
      fill="none"
      stroke="currentColor"
      stroke-width="44"
      stroke-linecap="round"
      stroke-linejoin="round"
    />
    <path
      d="M200 640v200M824 640v200M270 740h484"
      stroke="currentColor"
      stroke-width="28"
      stroke-linecap="round"
    />
    <path d="M214 740l86-44v88zM810 740l-86-44v88z" fill="currentColor" />
  </svg>
  <span class="name" aria-hidden="true">Dimo<span class="text-accent-strong">Mark</span></span>
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
