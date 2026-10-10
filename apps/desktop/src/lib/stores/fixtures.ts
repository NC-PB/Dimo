/**
 * Project data for store tests (T1.5). Shapes as Rust sends them; values are arbitrary.
 */

import type {
  Balloon,
  Change,
  Characteristic,
  Project,
  ProjectLoaded,
  ProjectStatus,
} from "$lib/ipc/bindings";

export const SHEET_ID = "00000000-0000-4000-8000-000000000001";
export const REVISION_ID = "00000000-0000-4000-8000-000000000002";
export const DOC_HASH = "a".repeat(64);

export function characteristic(id: string, number: number): Characteristic {
  return {
    id,
    number: String(number),
    kind: "other",
    requirement_text: "",
    nominal: null,
    unit: null,
    upper_dev: null,
    lower_dev: null,
    upper_limit: null,
    lower_limit: null,
    fit: null,
    derivation: null,
    quantity: 1,
    classification: "none",
    inspection: { method: "", gauge: "", sampling: "", frequency: "" },
    inspect: true,
    status: "accepted",
    origin: "manual",
    sources: [],
    comment: "",
  };
}

export function balloon(id: string, char: string): Balloon {
  return {
    id,
    characteristic: char,
    sheet: SHEET_ID,
    position: { x: 100, y: 100 },
    anchor: { x: 120, y: 110 },
    style: {
      shape: null,
      size_mm: null,
      outline_mm: null,
      outline_color: null,
      fill_color: null,
      text_color: null,
      leader: null,
    },
  };
}

export function emptyProject(): Project {
  return {
    info: { part_number: "", part_name: "", customer: "", order_reference: "" },
    revisions: [
      {
        id: REVISION_ID,
        label: "",
        file_name: "part.pdf",
        sha256: DOC_HASH,
        imported_at: "2026-10-09T10:00:00Z",
        sheets: [
          {
            id: SHEET_ID,
            index: 0,
            size: { width: 841.89, height: 595.276 },
            rotation: "deg0",
            kind: "vector_text",
            raster_dpi: null,
            unit: "mm",
            scale: { drawing: 1, actual: 1 },
            zone_grid: null,
            views: [],
          },
        ],
      },
    ],
    current_revision: REVISION_ID,
    characteristics: [],
    balloons: [],
    numbering: { lock: null },
    settings: {
      balloon_style: {
        shape: "circle",
        size_mm: null,
        outline_mm: null,
        outline_color: "#0057B8",
        fill_color: "#FFFFFF",
        text_color: "#000000",
        leader: true,
      },
      numbering: {
        strategy: "sheet_zone",
        multi_instance: "quantity",
        insert_when_locked: "next_free",
      },
      tolerance: {
        general: null,
        drawing_rule: null,
        decimal_rules: [],
        unit_rounding: { mm_places: 3, inch_places: 4 },
        custom_tables: [],
      },
    },
  };
}

export function status(overrides: Partial<ProjectStatus> = {}): ProjectStatus {
  return {
    file_name: null,
    modified: false,
    can_undo: false,
    can_redo: false,
    autosave: { state: "clean" },
    ...overrides,
  };
}

export function loaded(session: number, project: Project | null = emptyProject()): ProjectLoaded {
  return {
    session,
    snapshot:
      project === null
        ? null
        : {
            revision: 0,
            project,
            drawing: {
              doc: DOC_HASH,
              name: "part.pdf",
              sheets: [{ width: 841.89, height: 595.276 }],
            },
            status: status(),
          },
    notice: null,
  };
}

/** What Rust sends for adding characteristic `id` with balloon `b-<id>`, and its undo. */
export function addChanges(id: string, number: number, index: number): Change[] {
  return [
    { type: "characteristic_inserted", index, characteristic: characteristic(id, number) },
    { type: "balloon_inserted", index, balloon: balloon(`b-${id}`, id) },
  ];
}

export function undoAddChanges(id: string, number: number, index: number): Change[] {
  return [
    { type: "balloon_removed", index, balloon: balloon(`b-${id}`, id) },
    { type: "characteristic_removed", index, characteristic: characteristic(id, number) },
  ];
}
