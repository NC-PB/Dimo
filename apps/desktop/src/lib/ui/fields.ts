/**
 * Form fields and buttons shared by panes, ribbons and dialogs (MF F8): one class string per
 * kind, colors only through the theme tokens (D-51), and one way to send an edited field to
 * Rust and show what the project then holds.
 */

/** The look of a text, number, date or choice field. */
const FIELD = "min-h-8 rounded-md border border-border-strong bg-surface px-2 py-1 text-text";

/** Class of a form field; `extra` adds width, text size or alignment. */
export function fieldClass(extra = ""): string {
  return extra === "" ? FIELD : `${FIELD} ${extra}`;
}

const BUTTON = "min-h-8 rounded-md px-3 py-1 text-sm";

/** Classes of dialog and form buttons: the main action, the others and a destructive one. */
export const buttonClass = {
  primary: `${BUTTON} bg-accent-strong text-accent-ink`,
  secondary: `${BUTTON} border border-border text-text hover:bg-hover`,
  danger: `${BUTTON} bg-danger text-accent-ink`,
} as const;

/** A text field or text area whose change goes to Rust. */
type Editable = HTMLInputElement | HTMLTextAreaElement;

/**
 * Sends the text of an edited field and then shows what the project holds (MF F8). `stored`
 * reads the value from the project: Rust's trimmed text after the change, or the old value when
 * Rust refused it (the reason is shown elsewhere). Nothing is sent when the text equals the
 * stored value.
 */
export async function commitField(
  input: Editable,
  stored: () => string | undefined,
  send: (value: string) => Promise<unknown>,
): Promise<void> {
  const value = input.value;
  if (value === stored()) {
    return;
  }
  await send(value);
  input.value = stored() ?? "";
}
