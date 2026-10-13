/** Classes of the fields embedded in the ribbon (T3.15): label above, control at least 32 px. */
export const LABEL_CLASS = "flex flex-col gap-0.5 text-xs text-text-muted";

export const FIELD_CLASS =
  "min-h-8 rounded-md border border-border-strong bg-surface px-2 text-[13px] text-text disabled:opacity-45";

/** Tooltip of a ribbon field: what it sets and, when disabled, why (decision 27). */
export function fieldTitle(description: string, reason: string | null): string {
  return reason === null ? description : `${description}\n${reason}`;
}
