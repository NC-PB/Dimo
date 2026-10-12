/**
 * View helpers of the measurement grid (T3.5). Nothing here decides a value or a verdict: texts
 * go to Rust as typed (`set_results`), Rust parses them by the result type and computes the
 * verdicts (rule 2, M3 decisions 2 and 7).
 */

import type { CharId, MeasurementResult, ResultEntry, SampleId } from "$lib/ipc/bindings";

/** Key of a cell in lookup maps. */
export function cellKey(sample: SampleId, characteristic: CharId): string {
  return `${sample}|${characteristic}`;
}

/** Results by cell key. */
export function resultsByCell(
  results: readonly MeasurementResult[],
): ReadonlyMap<string, MeasurementResult> {
  return new Map(results.map((r) => [cellKey(r.sample, r.characteristic), r]));
}

/**
 * Text that puts an existing result into the editor: the value as stored (scale kept), or the
 * attribute word Rust reads back (`ok`, `not ok`).
 */
export function editText(result: MeasurementResult | undefined): string {
  if (result === undefined) {
    return "";
  }
  if (result.value.type === "variable") {
    return result.value.value;
  }
  return result.value.value === "ok" ? "ok" : "not ok";
}

/** Text Space sends on an attribute cell: OK on an empty or not OK cell, else not OK. */
export function toggledText(result: MeasurementResult | undefined): string {
  return result?.value.type === "attribute" && result.value.value === "ok" ? "not ok" : "ok";
}

/**
 * Cells of a pasted block: lines separated by line breaks, cells by tabs, as spreadsheets copy
 * them. One line break at the end is dropped; an empty text is no block.
 */
export function parseBlock(text: string): string[][] {
  const trimmed = text.replace(/(\r\n|\n|\r)$/, "");
  if (trimmed === "") {
    return [];
  }
  return trimmed.split(/\r\n|\n|\r/).map((line) => line.split("\t"));
}

/** The block does not fit into the grid from the active cell on. */
export interface BlockTooBig {
  rows: number;
  columns: number;
}

/**
 * The `set_results` entries of a pasted block whose top left cell is row `row`, sample column
 * `col`. Lines go down the rows, cells go right across the samples. A block that would reach
 * past the last row or sample is refused as a whole, so nothing is dropped silently.
 */
export function pasteEntries(
  block: readonly (readonly string[])[],
  rows: readonly CharId[],
  samples: readonly SampleId[],
  row: number,
  col: number,
): ResultEntry[] | BlockTooBig {
  const width = Math.max(0, ...block.map((line) => line.length));
  if (row < 0 || col < 0 || row + block.length > rows.length || col + width > samples.length) {
    return { rows: block.length, columns: width };
  }
  const entries: ResultEntry[] = [];
  block.forEach((line, r) => {
    line.forEach((text, c) => {
      const characteristic = rows[row + r];
      const sample = samples[col + c];
      if (characteristic !== undefined && sample !== undefined) {
        entries.push({ sample, characteristic, text, comment: null });
      }
    });
  });
  return entries;
}
