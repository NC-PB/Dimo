/**
 * Row virtualization with TanStack Virtual for Svelte 5 runes (frontend.md: never render all
 * rows). A thin wrapper around `@tanstack/virtual-core`: rows have one fixed height, so nothing
 * is measured, and only the rows in view plus `overscan` are rendered.
 */

import {
  Virtualizer,
  elementScroll,
  observeElementOffset,
  observeElementRect,
  type VirtualItem,
} from "@tanstack/virtual-core";

export interface RowVirtualizerOptions {
  /** Height of one row in CSS px. */
  rowHeight: number;
  /** Rows rendered above and below the visible ones. */
  overscan: number;
  /** Space above the first row inside the scroll element, for example a sticky header. */
  headerHeight: number;
}

export class RowVirtualizer {
  readonly #virtualizer: Virtualizer<HTMLElement, HTMLElement>;
  readonly #options: RowVirtualizerOptions;
  #element: HTMLElement | null = null;
  /** Increased whenever the virtualizer reports a change, so derived values recompute. */
  #version = $state(0);
  /** Plain copy of `#version`, so a change can be signalled inside an effect without reading. */
  #changes = 0;

  #changed(): void {
    this.#changes += 1;
    this.#version = this.#changes;
  }

  constructor(options: RowVirtualizerOptions) {
    this.#options = options;
    this.#virtualizer = new Virtualizer<HTMLElement, HTMLElement>({
      count: 0,
      getScrollElement: () => this.#element,
      estimateSize: () => options.rowHeight,
      overscan: options.overscan,
      scrollMargin: options.headerHeight,
      scrollPaddingStart: options.headerHeight,
      scrollToFn: elementScroll,
      // The visible area without scroll bars, so a row scrolled into view is not hidden
      // under the horizontal scroll bar.
      observeElementRect: (instance, cb) =>
        observeElementRect(instance, (rect) => {
          const element = this.#element;
          cb(
            element === null
              ? rect
              : {
                  width: element.clientWidth || rect.width,
                  height: element.clientHeight || rect.height,
                },
          );
        }),
      observeElementOffset,
      onChange: () => {
        this.#changed();
      },
    });
  }

  /** The rows to render, with their offset from the first row in `start - headerHeight`. */
  readonly items: VirtualItem[] = $derived.by(() => {
    void this.#version;
    return this.#virtualizer.getVirtualItems();
  });

  /** Height of all rows together in CSS px. */
  readonly totalSize: number = $derived.by(() => {
    void this.#version;
    return this.#virtualizer.getTotalSize() - this.#options.headerHeight;
  });

  /** Connects the scroll element. Call from an effect; returns the cleanup for the effect. */
  attach(element: HTMLElement): () => void {
    this.#element = element;
    // `_didMount` and `_willUpdate` are underscore-prefixed in @tanstack/virtual-core but are
    // the documented hooks its framework adapters (React, Vue, Svelte) call; there is no public
    // equivalent for wiring a scroll element by hand. The package version is therefore pinned
    // exactly in package.json: re-check both calls (and the virtualizer tests) on every upgrade.
    const cleanup = this.#virtualizer._didMount();
    this.#virtualizer._willUpdate();
    this.#changed();
    return () => {
      cleanup();
      this.#element = null;
    };
  }

  /** Sets the number of rows. */
  setCount(count: number): void {
    if (count !== this.#virtualizer.options.count) {
      this.#virtualizer.setOptions({ ...this.#virtualizer.options, count });
      this.#virtualizer._willUpdate();
      this.#changed();
    }
  }

  /** Scrolls the row at `index` into view if it is not visible. */
  scrollToIndex(index: number): void {
    if (index >= 0 && index < this.#virtualizer.options.count) {
      this.#virtualizer.scrollToIndex(index, { align: "auto" });
    }
  }

  /** Offset of a row from the top of the first row in CSS px. */
  rowTop(item: VirtualItem): number {
    return item.start - this.#options.headerHeight;
  }
}
