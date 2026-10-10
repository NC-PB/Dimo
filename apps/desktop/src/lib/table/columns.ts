/**
 * Columns of the characteristic table (T1.7, FR-CHR-01, FR-CHR-02).
 *
 * Every value shown comes from Rust as it is stored: numbers are the decimal strings of the
 * project, limits are the ones Rust derived or the user set (rule 5, frontend.md "no business
 * logic"). The table only turns typed text into a `FieldValue`; Rust parses and validates it.
 */

import { classificationLabel, kindLabel, m, ruleLabel, unitLabel } from "$lib/i18n";
import type {
  CharacteristicKind,
  Characteristic,
  Classification,
  FieldValue,
  Unit,
} from "$lib/ipc/bindings";

/** How a cell is edited. */
export type EditorKind =
  /** Not editable (the number follows the placement order). */
  | "none"
  /** Free text. */
  | "text"
  /** Exact decimal, typed as text and parsed by Rust. */
  | "decimal"
  /** Whole number of at least 1, validated by Rust. */
  | "quantity"
  /** Choice from a list (Bits UI select). */
  | "choice"
  /** Check box. */
  | "check";

/**
 * Column IDs. Except `number` and `rule` they are the `field` names of `FieldValue`. `rule` shows
 * the tolerance rule with its markers (T2.8); it changes only through the values or a
 * re-interpretation, never directly.
 */
export type ColumnId =
  | "number"
  | "kind"
  | "requirement_text"
  | "nominal"
  | "upper_dev"
  | "lower_dev"
  | "upper_limit"
  | "lower_limit"
  | "unit"
  | "fit"
  | "rule"
  | "quantity"
  | "classification"
  | "inspection_method"
  | "gauge"
  | "sampling"
  | "frequency"
  | "comment"
  | "inspect";

export interface ColumnSpec {
  id: ColumnId;
  editor: EditorKind;
  /** Width in CSS px. */
  size: number;
  /** Translated header text. */
  label: () => string;
  /** Numbers are right aligned so decimal points line up better. */
  numeric?: boolean;
}

export const COLUMNS: readonly ColumnSpec[] = [
  { id: "number", editor: "none", size: 64, label: () => m.col_number(), numeric: true },
  { id: "kind", editor: "choice", size: 128, label: () => m.col_kind() },
  { id: "requirement_text", editor: "text", size: 200, label: () => m.col_requirement_text() },
  { id: "nominal", editor: "decimal", size: 88, label: () => m.col_nominal(), numeric: true },
  { id: "upper_dev", editor: "decimal", size: 88, label: () => m.col_upper_dev(), numeric: true },
  { id: "lower_dev", editor: "decimal", size: 88, label: () => m.col_lower_dev(), numeric: true },
  {
    id: "upper_limit",
    editor: "decimal",
    size: 96,
    label: () => m.col_upper_limit(),
    numeric: true,
  },
  {
    id: "lower_limit",
    editor: "decimal",
    size: 96,
    label: () => m.col_lower_limit(),
    numeric: true,
  },
  { id: "unit", editor: "choice", size: 64, label: () => m.col_unit() },
  { id: "fit", editor: "text", size: 64, label: () => m.col_fit() },
  { id: "rule", editor: "none", size: 232, label: () => m.col_rule() },
  { id: "quantity", editor: "quantity", size: 56, label: () => m.col_quantity(), numeric: true },
  { id: "classification", editor: "choice", size: 112, label: () => m.col_classification() },
  { id: "inspection_method", editor: "text", size: 120, label: () => m.col_inspection_method() },
  { id: "gauge", editor: "text", size: 120, label: () => m.col_gauge() },
  { id: "sampling", editor: "text", size: 96, label: () => m.col_sampling() },
  { id: "frequency", editor: "text", size: 96, label: () => m.col_frequency() },
  { id: "comment", editor: "text", size: 200, label: () => m.col_comment() },
  { id: "inspect", editor: "check", size: 64, label: () => m.col_inspect() },
];

/** Index of the column the table focuses for "edit this characteristic" (`focusRequest`). */
export const REQUIREMENT_COLUMN = COLUMNS.findIndex((c) => c.id === "requirement_text");

export function isEditable(column: ColumnSpec): boolean {
  return column.editor !== "none";
}

/** The stored value of a cell as text, unchanged. Empty when not set. */
export function rawText(c: Characteristic, id: ColumnId): string {
  switch (id) {
    case "number":
      return c.number;
    case "kind":
      return c.kind;
    case "requirement_text":
      return c.requirement_text;
    case "nominal":
    case "upper_dev":
    case "lower_dev":
    case "upper_limit":
    case "lower_limit":
    case "fit":
      return c[id] ?? "";
    case "unit":
      return c.unit ?? NO_UNIT;
    case "rule":
      return c.derivation?.rule.rule ?? "";
    case "quantity":
      return String(c.quantity);
    case "classification":
      return c.classification;
    case "inspection_method":
      return c.inspection.method;
    case "gauge":
    case "sampling":
    case "frequency":
      return c.inspection[id];
    case "comment":
      return c.comment;
    case "inspect":
      return c.inspect ? "true" : "false";
  }
}

/** The text a cell shows. Choices are translated; "none" classification shows nothing. */
export function displayText(c: Characteristic, id: ColumnId): string {
  switch (id) {
    case "kind":
      return kindLabel(c.kind);
    case "unit":
      return c.unit === null ? "" : unitLabel(c.unit);
    case "classification":
      return c.classification === "none" ? "" : classificationLabel(c.classification);
    case "rule":
      return c.derivation === null ? "" : ruleLabel(c.derivation.rule);
    case "inspect":
      return "";
    default:
      return rawText(c, id);
  }
}

/** Choice value that stands for "no unit" (`null`). */
export const NO_UNIT = "none";

/** One option of a choice cell. */
export interface ChoiceOption {
  /** Value as Rust sends it; {@link NO_UNIT} stands for a `null` unit. */
  value: string;
  label: string;
}

export const KINDS: readonly CharacteristicKind[] = [
  "linear",
  "diameter",
  "radius",
  "spherical_radius",
  "angle",
  "chamfer",
  "thread",
  "counterbore",
  "countersink",
  "depth",
  "surface_texture",
  "geometric",
  "note",
  "flag_note",
  "material_process",
  "other",
];

const CLASSIFICATIONS: readonly Classification[] = ["none", "critical", "major", "minor", "key"];

const UNITS: readonly (Unit | null)[] = [null, "mm", "in", "deg"];

/** The options of a choice column, translated. */
export function choiceOptions(id: ColumnId): ChoiceOption[] {
  switch (id) {
    case "kind":
      return KINDS.map((k) => ({ value: k, label: kindLabel(k) }));
    case "classification":
      return CLASSIFICATIONS.map((k) => ({ value: k, label: classificationLabel(k) }));
    case "unit":
      return UNITS.map((u) => ({ value: u ?? NO_UNIT, label: unitLabel(u) }));
    default:
      return [];
  }
}

function isKind(value: string): value is CharacteristicKind {
  return (KINDS as readonly string[]).includes(value);
}

function isClassification(value: string): value is Classification {
  return (CLASSIFICATIONS as readonly string[]).includes(value);
}

function isUnit(value: string): value is Unit {
  return value === "mm" || value === "in" || value === "deg";
}

/**
 * The `FieldValue` for text typed into a cell, or `null` if the text cannot be sent (an empty
 * quantity, an unknown choice). Decimals and quantities are not checked here: Rust parses them
 * and its error is shown in the cell (rule 5). Surrounding white space is removed from numbers.
 */
export function fieldValue(id: ColumnId, text: string): FieldValue | null {
  switch (id) {
    case "number":
    case "rule":
    case "inspect":
      return null;
    case "nominal":
    case "upper_dev":
    case "lower_dev":
    case "upper_limit":
    case "lower_limit": {
      const value = text.trim();
      return { field: id, value: value === "" ? null : value };
    }
    case "fit": {
      const value = text.trim();
      return { field: "fit", value: value === "" ? null : value };
    }
    case "quantity": {
      const value = text.trim();
      if (value === "") {
        return null;
      }
      // Only plain digits are read as a number. Anything else (`1e3`, `0x10`, `1.0`, `-1`,
      // `abc`) goes to Rust as `NaN`, which is sent as `null`; Rust refuses it. A range check
      // (zero, above u32) is Rust's too.
      return { field: "quantity", value: /^\d+$/.test(value) ? Number(value) : Number.NaN };
    }
    case "kind":
      return isKind(text) ? { field: "kind", value: text } : null;
    case "classification":
      return isClassification(text) ? { field: "classification", value: text } : null;
    case "unit":
      if (text === NO_UNIT) {
        return { field: "unit", value: null };
      }
      return isUnit(text) ? { field: "unit", value: text } : null;
    case "requirement_text":
    case "inspection_method":
    case "gauge":
    case "sampling":
    case "frequency":
    case "comment":
      return { field: id, value: text };
  }
}
