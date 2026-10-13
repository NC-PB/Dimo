<script lang="ts">
  import { m } from "$lib/i18n";
  import type { SheetInfo } from "$lib/ipc/bindings";
  import { previewTiles } from "./preview";

  interface Props {
    /** Content hash of the preview document from `report_preview`. */
    doc: string;
    /** Zero based page index (the sheet of the tile route). */
    index: number;
    /** Number of pages, for the label. */
    count: number;
    /** Page size in sheet units. */
    page: SheetInfo;
    /** Width on screen in CSS px. */
    width: number;
  }

  let { doc, index, count, page, width }: Props = $props();

  /**
   * One page of the report preview (T3.19): the PDF page as PDFium renders it, through the tile
   * route like a drawing, on white paper in both themes.
   */
  // Rust's `f64` may arrive as `null` (not finite); such a page shows empty.
  const size = $derived({ width: page.width ?? 0, height: page.height ?? 0 });
  const height = $derived(size.width > 0 ? (width * size.height) / size.width : 0);
  const dpr = typeof window === "undefined" ? 1 : window.devicePixelRatio || 1;
  const tiles = $derived(previewTiles(doc, index, size, width, dpr));
</script>

<div
  class="paper relative shrink-0 overflow-hidden"
  style:width="{width}px"
  style:height="{height}px"
  role="img"
  aria-label={m.report_preview_page({ page: String(index + 1), count: String(count) })}
  data-testid="preview-page"
>
  {#each tiles as tile (tile.key)}
    <img
      class="absolute block max-w-none select-none"
      src={tile.url}
      alt=""
      draggable="false"
      style:left="{tile.left}px"
      style:top="{tile.top}px"
      style:width="{tile.width}px"
      style:height="{tile.height}px"
    />
  {/each}
</div>

<style>
  /* The page stays paper colored in both themes (M3 decision 24). */
  .paper {
    background: #ffffff;
    box-shadow:
      0 2px 4px rgb(16 24 32 / 8%),
      0 16px 48px rgb(16 24 32 / 14%);
  }
</style>
