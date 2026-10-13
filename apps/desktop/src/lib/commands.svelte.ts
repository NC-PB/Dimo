/**
 * The command registry (ADR 0007, M3 decisions 26 and 27, D-52). Every command of the ribbon and
 * the title bar is defined here once: label, icon, keyboard shortcut, tooltip sentence, why it is
 * disabled, and what it does. The ribbon, the title bar, the tooltips and the shortcut handler
 * read from this list; the cheat sheet lists the keys of `shortcuts.ts`, which stays the only
 * place where key combinations are defined. A command names its shortcut by action, it never
 * repeats the keys.
 *
 * Commands only call stores, and the stores send Rust commands (rule 2): nothing here computes
 * limits, numbers or verdicts.
 */

import { tick } from "svelte";
import { m } from "$lib/i18n";
import { icons, type Icon } from "$lib/icons";
import { measureUi } from "$lib/measure/measure-ui.svelte";
import { parts } from "$lib/measure/parts.svelte";
import { rotateShownSheet, shownSheet } from "$lib/sheet-properties";
import {
  formatCombo,
  isMacPlatform,
  SHORTCUTS,
  shortcutKeys,
  type ShortcutAction,
} from "$lib/shortcuts";
import { balloonTools } from "$lib/stores/balloon-tools.svelte";
import { documentStore } from "$lib/stores/document.svelte";
import { exportStore } from "$lib/stores/export.svelte";
import { numberingStore } from "$lib/stores/numbering.svelte";
import { projectStore } from "$lib/stores/project.svelte";
import { selection } from "$lib/stores/selection.svelte";
import { settingsStore } from "$lib/stores/settings.svelte";
import { sheetRegions } from "$lib/stores/sheet-regions.svelte";
import { toleranceStore } from "$lib/stores/tolerance.svelte";
import { view } from "$lib/stores/view.svelte";
import { viewport } from "$lib/stores/viewport.svelte";
import type { ReportKind } from "$lib/ipc/bindings";

export interface CommandDef {
  /** Short label (Paraglide message). */
  label: () => string;
  icon: Icon;
  /** One sentence of what the command does, for the tooltip. */
  description: () => string;
  /** The shortcut action of `shortcuts.ts` that runs this command, if any. */
  shortcut?: ShortcutAction;
  /** Why the command cannot run now, `null` when it can. Disabled commands stay visible. */
  disabled?: () => string | null;
  /** For on/off commands: whether it is on. */
  pressed?: () => boolean;
  /** Runs the command. Returns `false` when it did nothing, so a key keeps its usual meaning. */
  run: () => unknown;
}

// Reasons, checked in this order by the commands that need them.
const needProject = (): string | null => (projectStore.isOpen ? null : m.reason_no_project());

const needIdle = (): string | null => needProject() ?? (projectStore.busy ? m.reason_busy() : null);

const needSelection = (): string | null =>
  needProject() ?? (selection.isEmpty ? m.reason_no_selection() : null);

const needDrawing = (): string | null =>
  needProject() ??
  (documentStore.current === null
    ? m.reason_no_sheet()
    : view.current !== "drawing"
      ? m.reason_not_drawing()
      : null);

const needSheet = (): string | null =>
  needProject() ?? (shownSheet() === undefined ? m.reason_no_sheet() : null);

const needPart = (): string | null =>
  needProject() ?? (parts.sample === undefined ? m.reason_no_part() : null);

const needCell = (): string | null =>
  needProject() ??
  (measureUi.activeRow === null || measureUi.activeSample === null ? m.reason_no_part() : null);

function reportBusy(kind: ReportKind): string | null {
  return (
    needProject() ??
    (exportStore.busy(exportStore.reportFormat(kind)) ? m.reason_export_running() : null)
  );
}

/** Shows the drawing with its side panel and scrolls to an element of the panel. */
async function revealInSidePanel(selector: string): Promise<void> {
  view.set("drawing");
  view.showSidePanel = true;
  await tick();
  document.querySelector(selector)?.scrollIntoView({ block: "start", behavior: "smooth" });
}

/** Opens the settings page at a section, until the Drawing tab gets these settings (T3.16). */
async function openSettings(id: string): Promise<void> {
  view.set("settings");
  await tick();
  document.getElementById(id)?.scrollIntoView({ block: "start", behavior: "smooth" });
}

/** The theme in effect, read from the document root (`theme.ts` sets it). */
function shownTheme(): "light" | "dark" {
  return typeof document !== "undefined" && document.documentElement.dataset.theme === "dark"
    ? "dark"
    : "light";
}

export const COMMANDS = {
  // Title bar and File.
  save: {
    label: () => m.cmd_save(),
    icon: icons.save,
    description: () => m.cmd_save_desc(),
    shortcut: "save_project",
    disabled: needIdle,
    run: () => projectStore.save(),
  },
  save_as: {
    label: () => m.cmd_save_as(),
    icon: icons.saveAs,
    description: () => m.cmd_save_as_desc(),
    shortcut: "save_project_as",
    disabled: needIdle,
    run: () => projectStore.saveAs(),
  },
  new_project: {
    label: () => m.cmd_new(),
    icon: icons.newProject,
    description: () => m.cmd_new_desc(),
    shortcut: "new_project",
    disabled: () => (projectStore.busy ? m.reason_busy() : null),
    run: () => projectStore.newProject(),
  },
  open_project: {
    label: () => m.cmd_open(),
    icon: icons.openProject,
    description: () => m.cmd_open_desc(),
    shortcut: "open_project",
    disabled: () => (projectStore.busy ? m.reason_busy() : null),
    run: () => projectStore.open(),
  },
  undo: {
    label: () => m.cmd_undo(),
    icon: icons.undo,
    description: () => m.cmd_undo_desc(),
    shortcut: "undo",
    disabled: () => needProject() ?? (projectStore.canUndo ? null : m.reason_nothing_to_undo()),
    run: () => projectStore.undo(),
  },
  redo: {
    label: () => m.cmd_redo(),
    icon: icons.redo,
    description: () => m.cmd_redo_desc(),
    shortcut: "redo",
    disabled: () => needProject() ?? (projectStore.canRedo ? null : m.reason_nothing_to_redo()),
    run: () => projectStore.redo(),
  },
  toggle_theme: {
    label: () => m.cmd_theme(),
    icon: icons.themeDark,
    description: () => m.cmd_theme_desc(),
    run: () => settingsStore.update({ theme: shownTheme() === "dark" ? "light" : "dark" }),
  },
  help: {
    label: () => m.cmd_help(),
    icon: icons.help,
    description: () => m.cmd_help_desc(),
    shortcut: "show_shortcuts",
    run: () => {
      view.shortcutsOpen = !view.shortcutsOpen;
    },
  },
  settings: {
    label: () => m.cmd_settings(),
    icon: icons.settings,
    description: () => m.cmd_settings_desc(),
    shortcut: "show_settings",
    run: () => {
      view.set("settings");
    },
  },

  // Home: tools, balloon, numbering, tolerance.
  select_tool: {
    label: () => m.cmd_select(),
    icon: icons.select,
    description: () => m.cmd_select_desc(),
    shortcut: "select_tool",
    disabled: needSheet,
    pressed: () => balloonTools.tool === "select",
    run: () => {
      balloonTools.setTool("select");
    },
  },
  place_tool: {
    label: () => m.cmd_place(),
    icon: icons.place,
    description: () => m.cmd_place_desc(),
    shortcut: "place_tool",
    disabled: needSheet,
    pressed: () => balloonTools.tool === "place",
    // Clicking the active tool again stops it (T3.13).
    run: () => {
      balloonTools.setTool(balloonTools.tool === "place" ? "select" : "place");
    },
  },
  edit_value: {
    label: () => m.cmd_edit_value(),
    icon: icons.editValue,
    description: () => m.cmd_edit_value_desc(),
    shortcut: "edit_value",
    disabled: () => needSelection() ?? (view.current !== "drawing" ? m.reason_not_drawing() : null),
    run: () => balloonTools.editPrimary(),
  },
  balloon_style: {
    label: () => m.cmd_balloon_style(),
    icon: icons.balloonStyle,
    description: () => m.cmd_balloon_style_desc(),
    shortcut: "restyle",
    disabled: needSelection,
    run: () => {
      // The style picker opens under its ribbon button, which is on Home.
      if (view.tab !== "home") {
        view.setTab("home");
      }
      balloonTools.styleOpen = !balloonTools.styleOpen;
    },
  },
  delete_selection: {
    label: () => m.cmd_delete(),
    icon: icons.delete,
    description: () => m.cmd_delete_desc(),
    shortcut: "delete_selection",
    disabled: needSelection,
    run: () => balloonTools.deleteSelection(),
  },
  renumber: {
    label: () => m.cmd_renumber(),
    icon: icons.renumber,
    description: () => m.cmd_renumber_desc(),
    disabled: () =>
      needIdle() ??
      (numberingStore.locked
        ? m.reason_numbering_locked()
        : projectStore.characteristics.length === 0
          ? m.reason_no_characteristics()
          : null),
    run: () => numberingStore.apply(),
  },
  preview_numbers: {
    label: () => m.cmd_preview_numbers(),
    icon: icons.previewNumbers,
    description: () => m.cmd_preview_numbers_desc(),
    disabled: () => needProject() ?? (numberingStore.locked ? m.reason_numbering_locked() : null),
    pressed: () => numberingStore.previewing,
    run: () => {
      numberingStore.setPreviewing(!numberingStore.previewing);
    },
  },
  update_limits: {
    label: () => m.cmd_update_limits(),
    icon: icons.updateLimits,
    description: () => m.cmd_update_limits_desc(),
    disabled: () => needSelection() ?? (toleranceStore.busy ? m.reason_busy() : null),
    run: () => toleranceStore.reinterpret(selection.ids),
  },

  // Drawing: sheet, zones and views, tolerances, project.
  rotate_left: {
    label: () => m.rotate_left(),
    icon: icons.rotateLeft,
    description: () => m.cmd_rotate_left_desc(),
    shortcut: "rotate_left",
    disabled: needSheet,
    run: () => rotateShownSheet(-1),
  },
  rotate_right: {
    label: () => m.rotate_right(),
    icon: icons.rotateRight,
    description: () => m.cmd_rotate_right_desc(),
    shortcut: "rotate_right",
    disabled: needSheet,
    run: () => rotateShownSheet(1),
  },
  zone_grid: {
    label: () => m.cmd_zone_grid(),
    icon: icons.zoneGrid,
    description: () => m.cmd_zone_grid_desc(),
    disabled: needSheet,
    pressed: () => sheetRegions.tool === "zone_frame",
    run: () => {
      view.set("drawing");
      sheetRegions.toggle("zone_frame");
    },
  },
  draw_view: {
    label: () => m.cmd_draw_view(),
    icon: icons.drawView,
    description: () => m.cmd_draw_view_desc(),
    disabled: needSheet,
    pressed: () => sheetRegions.tool === "view",
    run: () => {
      view.set("drawing");
      sheetRegions.toggle("view");
    },
  },
  decimal_rules: {
    label: () => m.cmd_decimal_rules(),
    icon: icons.decimalRules,
    description: () => m.cmd_decimal_rules_desc(),
    disabled: needProject,
    run: () => openSettings("settings-decimal-rules"),
  },
  company_tables: {
    label: () => m.cmd_company_tables(),
    icon: icons.companyTables,
    description: () => m.cmd_company_tables_desc(),
    disabled: needProject,
    run: () => openSettings("settings-custom-tables"),
  },
  project_info: {
    label: () => m.cmd_project_info(),
    icon: icons.projectInfo,
    description: () => m.cmd_project_info_desc(),
    disabled: needProject,
    run: () => revealInSidePanel('[aria-labelledby$="-project-details-title"]'),
  },

  // Measure: parts, values, show.
  add_parts: {
    label: () => m.cmd_add_parts(),
    icon: icons.addParts,
    description: () => m.cmd_add_parts_desc(),
    disabled: needIdle,
    run: () => parts.add(),
  },
  move_part_left: {
    label: () => m.cmd_move_part_left(),
    icon: icons.moveLeft,
    description: () => m.cmd_move_part_left_desc(),
    disabled: () => needPart() ?? (parts.index <= 0 ? m.reason_first_part() : null),
    run: () => parts.moveBy(-1),
  },
  move_part_right: {
    label: () => m.cmd_move_part_right(),
    icon: icons.moveRight,
    description: () => m.cmd_move_part_right_desc(),
    disabled: () =>
      needPart() ?? (parts.index >= parts.samples.length - 1 ? m.reason_last_part() : null),
    run: () => parts.moveBy(1),
  },
  delete_part: {
    label: () => m.cmd_delete_part(),
    icon: icons.delete,
    description: () => m.cmd_delete_part_desc(),
    disabled: needPart,
    run: () => parts.remove(),
  },
  paste_values: {
    label: () => m.cmd_paste_values(),
    icon: icons.paste,
    description: () => m.cmd_paste_values_desc(),
    shortcut: "grid_paste",
    disabled: needCell,
    run: () => measureUi.paste(m.cmd_paste_hint({ keys: shortcutKeys("grid_paste") })),
  },
  clear_cell: {
    label: () => m.cmd_clear_cell(),
    icon: icons.clearCell,
    description: () => m.cmd_clear_cell_desc(),
    shortcut: "grid_clear",
    disabled: needCell,
    run: () => measureUi.clear(),
  },
  only_missing: {
    label: () => m.cmd_only_missing(),
    icon: icons.onlyMissing,
    description: () => m.cmd_only_missing_desc(),
    disabled: needProject,
    pressed: () => measureUi.onlyMissing,
    run: () => {
      measureUi.onlyMissing = !measureUi.onlyMissing;
    },
  },
  only_failed: {
    label: () => m.cmd_only_failed(),
    icon: icons.onlyFailed,
    description: () => m.cmd_only_failed_desc(),
    disabled: needProject,
    pressed: () => measureUi.onlyFailed,
    run: () => {
      measureUi.onlyFailed = !measureUi.onlyFailed;
    },
  },

  // Report: document, options, output.
  check_sheet: {
    label: () => m.export_check_sheet_title(),
    icon: icons.checkSheet,
    description: () => m.cmd_check_sheet_desc(),
    disabled: needProject,
    pressed: () => exportStore.document === "check_sheet",
    run: () => {
      exportStore.document = "check_sheet";
    },
  },
  inspection_report: {
    label: () => m.export_inspection_report_title(),
    icon: icons.inspectionReport,
    description: () => m.cmd_inspection_report_desc(),
    disabled: needProject,
    pressed: () => exportStore.document === "inspection_report",
    run: () => {
      exportStore.document = "inspection_report";
    },
  },
  open_after_export: {
    label: () => m.cmd_open_after(),
    icon: icons.openAfter,
    description: () => m.cmd_open_after_desc(),
    pressed: () => settingsStore.current.export.open_after_export,
    run: () => {
      const prefs = settingsStore.current.export;
      return settingsStore.update({
        export: { ...prefs, open_after_export: !prefs.open_after_export },
      });
    },
  },
  save_copy: {
    label: () => m.cmd_save_copy(),
    icon: icons.saveCopy,
    description: () => m.cmd_save_copy_desc(),
    disabled: () => reportBusy(exportStore.document),
    run: () => exportStore.startReport(exportStore.document, false),
  },
  issue_report: {
    label: () => m.cmd_issue_report(),
    icon: icons.issueReport,
    description: () => m.cmd_issue_report_desc(),
    disabled: () => reportBusy(exportStore.document),
    run: () => {
      view.set("export"); // The warnings before issuing are shown on the report page.
      return exportStore.startReport(exportStore.document, true);
    },
  },

  // View: zoom, panes.
  zoom_in: {
    label: () => m.zoom_in(),
    icon: icons.zoomIn,
    description: () => m.cmd_zoom_in_desc(),
    shortcut: "zoom_in",
    disabled: needDrawing,
    run: () => {
      viewport.zoomIn();
    },
  },
  zoom_out: {
    label: () => m.zoom_out(),
    icon: icons.zoomOut,
    description: () => m.cmd_zoom_out_desc(),
    shortcut: "zoom_out",
    disabled: needDrawing,
    run: () => {
      viewport.zoomOut();
    },
  },
  fit: {
    label: () => m.fit_sheet(),
    icon: icons.fit,
    description: () => m.cmd_fit_desc(),
    shortcut: "fit",
    disabled: needDrawing,
    run: () => {
      viewport.fit();
    },
  },
  show_table: {
    label: () => m.cmd_show_table(),
    icon: icons.table,
    description: () => m.cmd_show_table_desc(),
    pressed: () => view.showTable,
    run: () => {
      view.showTable = !view.showTable;
    },
  },
  show_side_panel: {
    label: () => m.cmd_show_side_panel(),
    icon: icons.sidePanel,
    description: () => m.cmd_show_side_panel_desc(),
    pressed: () => view.showSidePanel,
    run: () => {
      view.showSidePanel = !view.showSidePanel;
    },
  },
} satisfies Record<string, CommandDef>;

export type CommandId = keyof typeof COMMANDS;

export const COMMAND_IDS = Object.keys(COMMANDS) as CommandId[];

export function command(id: CommandId): CommandDef {
  return COMMANDS[id];
}

/** Why command `id` cannot run now, `null` when it can. */
export function disabledReason(id: CommandId): string | null {
  return command(id).disabled?.() ?? null;
}

/** Display text of the shortcut of command `id`, empty without one. */
export function commandKeys(id: CommandId, mac: boolean = isMacPlatform()): string {
  const action = command(id).shortcut;
  const combo = action === undefined ? undefined : SHORTCUTS.find((s) => s.action === action);
  return combo?.keys[0] === undefined ? "" : formatCombo(combo.keys[0], mac);
}

/**
 * Tooltip of command `id` (decision 27): the label with the shortcut, one sentence of what it
 * does and, when disabled, why.
 */
export function commandTooltip(id: CommandId, mac: boolean = isMacPlatform()): string {
  const c = command(id);
  const keys = commandKeys(id, mac);
  const reason = disabledReason(id);
  return [keys === "" ? c.label() : `${c.label()} (${keys})`, c.description(), reason]
    .filter((line): line is string => line !== null && line !== "")
    .join("\n");
}

/** Runs command `id` unless it is disabled. Returns whether it did something. */
export function runCommand(id: CommandId): boolean {
  if (disabledReason(id) !== null) {
    return false;
  }
  const result = command(id).run();
  if (result instanceof Promise) {
    void result;
    return true;
  }
  return result !== false;
}

/** The command a shortcut action runs, if any. */
export function commandForShortcut(action: ShortcutAction): CommandId | undefined {
  return COMMAND_IDS.find((id) => command(id).shortcut === action);
}
