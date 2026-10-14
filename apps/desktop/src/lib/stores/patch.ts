import type {
  Balloon,
  Change,
  Characteristic,
  MeasurementResult,
  Project,
  ResultCell,
  Sheet,
} from "$lib/ipc/bindings";

/**
 * A patch that does not fit the project in the store, for example after a missed event. The
 * store then fetches the full state from Rust again.
 */
export class PatchMismatch extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PatchMismatch";
  }
}

function insertAt<T>(list: readonly T[], index: number, item: T): T[] {
  if (index < 0 || index > list.length) {
    throw new PatchMismatch(`insert index ${String(index)} out of range`);
  }
  return [...list.slice(0, index), item, ...list.slice(index)];
}

function removeAt<T extends { id: string }>(list: readonly T[], index: number, id: string): T[] {
  if (list[index]?.id !== id) {
    throw new PatchMismatch(`no item ${id} at index ${String(index)}`);
  }
  return [...list.slice(0, index), ...list.slice(index + 1)];
}

function replaceById<T extends { id: string }>(list: readonly T[], item: T): T[] {
  const index = list.findIndex((x) => x.id === item.id);
  if (index < 0) {
    throw new PatchMismatch(`unknown item ${item.id}`);
  }
  return list.map((x, i) => (i === index ? item : x));
}

function reorder<T extends { id: string }>(list: readonly T[], order: readonly string[]): T[] {
  const byId = new Map(list.map((x) => [x.id, x]));
  const result = order.map((id) => byId.get(id));
  if (result.length !== list.length || result.some((x) => x === undefined)) {
    throw new PatchMismatch("new order is no permutation");
  }
  return result as T[];
}

function replaceSheet(project: Project, sheet: Sheet): Project {
  let found = false;
  const revisions = project.revisions.map((revision) => {
    if (!revision.sheets.some((s) => s.id === sheet.id)) {
      return revision;
    }
    found = true;
    return { ...revision, sheets: replaceById(revision.sheets, sheet) };
  });
  if (!found) {
    throw new PatchMismatch(`unknown sheet ${sheet.id}`);
  }
  return { ...project, revisions };
}

/**
 * Results in the order Rust keeps them: by sample order, then by characteristic placement order
 * (M3 decision 1). A copy of Rust's order, so the store equals the project Rust holds.
 */
function sortResults(project: Project): Project {
  if (project.results.length === 0) {
    return project;
  }
  const samples = new Map(project.samples.map((s, i) => [s.id, i]));
  const chars = new Map(project.characteristics.map((c, i) => [c.id, i]));
  const key = (r: MeasurementResult): [number, number] => [
    samples.get(r.sample) ?? Infinity,
    chars.get(r.characteristic) ?? Infinity,
  ];
  const results = [...project.results].sort((a, b) => {
    const [as, ac] = key(a);
    const [bs, bc] = key(b);
    return as - bs || ac - bc;
  });
  return results.every((r, i) => r === project.results[i]) ? project : { ...project, results };
}

function sameCell(result: MeasurementResult, cell: ResultCell): boolean {
  return result.sample === cell.sample && result.characteristic === cell.characteristic;
}

function changeResult(
  project: Project,
  cell: ResultCell,
  before: MeasurementResult | null,
  after: MeasurementResult | null,
): Project {
  const index = project.results.findIndex((r) => sameCell(r, cell));
  if (index >= 0 !== (before !== null)) {
    throw new PatchMismatch(`result of ${cell.sample}/${cell.characteristic} differs`);
  }
  if (after === null) {
    return { ...project, results: project.results.filter((_, i) => i !== index) };
  }
  if (index >= 0) {
    return { ...project, results: project.results.map((r, i) => (i === index ? after : r)) };
  }
  return sortResults({ ...project, results: [...project.results, after] });
}

function applyChange(project: Project, change: Change): Project {
  switch (change.type) {
    case "characteristic_inserted":
      return {
        ...project,
        characteristics: insertAt(project.characteristics, change.index, change.characteristic),
      };
    case "characteristic_removed":
      return {
        ...project,
        characteristics: removeAt(project.characteristics, change.index, change.characteristic.id),
      };
    case "characteristic_changed":
      return {
        ...project,
        characteristics: replaceById(project.characteristics, change.after),
      };
    case "balloon_inserted":
      return { ...project, balloons: insertAt(project.balloons, change.index, change.balloon) };
    case "balloon_removed":
      return {
        ...project,
        balloons: removeAt(project.balloons, change.index, change.balloon.id),
      };
    case "balloon_changed":
      return { ...project, balloons: replaceById(project.balloons, change.after) };
    case "order_changed":
      return sortResults({
        ...project,
        characteristics: reorder(project.characteristics, change.after),
      });
    case "sheet_changed":
      return replaceSheet(project, change.after);
    case "numbering_changed":
      return { ...project, numbering: change.after };
    case "settings_changed":
      return { ...project, settings: change.after };
    case "info_changed":
      return { ...project, info: change.after };
    case "sample_inserted":
      return { ...project, samples: insertAt(project.samples, change.index, change.sample) };
    case "sample_removed":
      return { ...project, samples: removeAt(project.samples, change.index, change.sample.id) };
    case "sample_changed":
      return { ...project, samples: replaceById(project.samples, change.after) };
    case "sample_order_changed":
      return sortResults({ ...project, samples: reorder(project.samples, change.after) });
    case "result_changed":
      return changeResult(project, change.cell, change.before, change.after);
    case "report_info_changed":
      return { ...project, report_info: change.after };
    case "revision_label_changed": {
      if (!project.revisions.some((r) => r.id === change.revision)) {
        throw new PatchMismatch(`unknown revision ${change.revision}`);
      }
      return {
        ...project,
        revisions: project.revisions.map((r) =>
          r.id === change.revision ? { ...r, label: change.after } : r,
        ),
      };
    }
    case "revision_changed":
      return { ...project, revisions: replaceById(project.revisions, change.after) };
    case "issued_report_inserted":
      return {
        ...project,
        issued_reports: insertAt(project.issued_reports, change.index, change.report),
      };
    case "issued_report_removed": {
      const reports = project.issued_reports;
      if (change.index < 0 || change.index >= reports.length) {
        throw new PatchMismatch(`no issued report at index ${String(change.index)}`);
      }
      return {
        ...project,
        issued_reports: reports.filter((_, i) => i !== change.index),
      };
    }
  }
}

/**
 * Applies the changes of a patch from Rust, in order, without touching `project`. Only the
 * lists and items that changed get new objects, so unchanged rows and balloons keep their
 * identity (keyed lists and table memoization stay cheap). This is a mechanical copy of what
 * Rust already decided; no rule is evaluated here.
 */
export function applyChanges(project: Project, changes: readonly Change[]): Project {
  return changes.reduce(applyChange, project);
}

/** Characteristics by ID, for lookups from balloons and selection. Built once per project. */
export function characteristicsById(project: Project | null): ReadonlyMap<string, Characteristic> {
  return new Map(project?.characteristics.map((c) => [c.id, c]) ?? []);
}

/** Balloons grouped by sheet ID, in placement order. Built once per project. */
export function balloonsBySheet(project: Project | null): ReadonlyMap<string, readonly Balloon[]> {
  const map = new Map<string, Balloon[]>();
  for (const balloon of project?.balloons ?? []) {
    const list = map.get(balloon.sheet);
    if (list) {
      list.push(balloon);
    } else {
      map.set(balloon.sheet, [balloon]);
    }
  }
  return map;
}
