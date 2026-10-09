import type { DocumentInfo } from "$lib/ipc/bindings";
import type { Size } from "$lib/viewport/view-math";

/**
 * The drawing on screen and which of its sheets is shown. View state only: the drawing belongs
 * to the open project, whose store calls {@link DocumentStore.show} when it changes (ADR 0001).
 */
export class DocumentStore {
  current = $state.raw<DocumentInfo | null>(null);
  sheet = $state(0);

  /** Number of sheets of the drawing, zero without one. */
  get sheetCount(): number {
    return this.current?.sheets.length ?? 0;
  }

  /** Size of the sheet on screen in sheet units, or `null` without a drawing. */
  get sheetSize(): Size | null {
    const info = this.current?.sheets[this.sheet];
    if (!info) {
      return null;
    }
    return { width: info.width ?? 0, height: info.height ?? 0 };
  }

  /** Shows a drawing from its first sheet, or nothing. */
  show(info: DocumentInfo | null): void {
    this.current = info;
    this.sheet = 0;
  }

  /** Switches to sheet `index` if the drawing has it. */
  setSheet(index: number): void {
    if (Number.isInteger(index) && index >= 0 && index < this.sheetCount) {
      this.sheet = index;
    }
  }

  nextSheet(): void {
    this.setSheet(this.sheet + 1);
  }

  previousSheet(): void {
    this.setSheet(this.sheet - 1);
  }
}

export const documentStore = new DocumentStore();
