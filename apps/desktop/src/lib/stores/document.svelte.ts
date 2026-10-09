import { commands, type CommandError, type DocumentInfo } from "$lib/ipc/bindings";
import type { Size } from "$lib/viewport/view-math";

type OpenResult =
  { status: "ok"; data: DocumentInfo | null } | { status: "error"; error: CommandError };

/**
 * The open drawing as the viewport needs it, and the sheet on screen. View state only: the
 * document itself lives in Rust (ADR 0001).
 */
export class DocumentStore {
  current = $state<DocumentInfo | null>(null);
  sheet = $state(0);
  error = $state<CommandError | null>(null);
  busy = $state(false);

  /** Number of sheets of the open document, zero without one. */
  get sheetCount(): number {
    return this.current?.sheets.length ?? 0;
  }

  /** Size of the sheet on screen in sheet units, or `null` without a document. */
  get sheetSize(): Size | null {
    const info = this.current?.sheets[this.sheet];
    if (!info) {
      return null;
    }
    return { width: info.width ?? 0, height: info.height ?? 0 };
  }

  /** Asks Rust to show the file dialog and open the chosen PDF. Cancel keeps the current one. */
  async openWithDialog(
    open: () => Promise<OpenResult> = commands.openDocumentDialog,
  ): Promise<void> {
    if (this.busy) {
      return;
    }
    this.busy = true;
    try {
      const result = await open();
      if (result.status === "error") {
        this.error = result.error;
      } else if (result.data) {
        this.show(result.data);
      }
    } catch (e) {
      this.error = { kind: "io", message: String(e) };
    } finally {
      this.busy = false;
    }
  }

  /** Shows a document that Rust opened, starting at its first sheet. */
  show(info: DocumentInfo): void {
    this.current = info;
    this.sheet = 0;
    this.error = null;
  }

  /** Switches to sheet `index` if the document has it. */
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
