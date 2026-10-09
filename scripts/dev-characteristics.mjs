#!/usr/bin/env node
// Writes a `DIMO_DEV_SCRIPT` file that adds many characteristics with balloons to sheet 0 in one
// undo step, for checking the characteristic table with a large list (T1.7). Debug builds only.
// Plain Node, no dependencies. Values are made up and deterministic.
//
// Usage:
//   node scripts/dev-characteristics.mjs 1000 target/dev/chars-1000.json
//   DIMO_DEV_OPEN=corpus/drawings/test_drawing_1.pdf \
//   DIMO_DEV_SCRIPT=$PWD/target/dev/chars-1000.json \
//   VITE_DIMO_DEV_TABLE_CHECK=1 pnpm -C apps/desktop tauri dev
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

const count = Number(process.argv[2] ?? 1000);
const out = process.argv[3];
if (!Number.isInteger(count) || count < 1 || !out) {
  console.error("usage: node scripts/dev-characteristics.mjs <count> <output.json>");
  process.exit(2);
}

const KINDS = ["linear", "diameter", "radius", "angle", "chamfer", "thread", "depth", "note"];
const CLASSES = ["none", "none", "major", "minor", "critical", "key"];
// Balloons on a grid inside the smallest common sheet (A4 landscape, 842 x 595 units).
const COLUMNS = 40;
const ROWS = 25;

const commands = Array.from({ length: count }, (_, i) => {
  const x = 30 + (i % COLUMNS) * 19.5;
  const y = 40 + (Math.floor(i / COLUMNS) % ROWS) * 21;
  const nominal = `${String(5 + (i % 95))}.${String(i % 10)}`;
  const kind = KINDS[i % KINDS.length];
  return {
    type: "add_characteristic",
    sheet: "$SHEET0",
    position: { x, y },
    anchor: { x: x + 6, y: y + 8 },
    region: null,
    values: [
      { field: "kind", value: kind },
      { field: "requirement_text", value: `${kind === "diameter" ? "Ø" : ""}${nominal}` },
      { field: "nominal", value: nominal },
      { field: "upper_dev", value: "0.1" },
      { field: "lower_dev", value: "-0.05" },
      { field: "classification", value: CLASSES[i % CLASSES.length] },
      { field: "inspection_method", value: i % 3 === 0 ? "CMM" : "caliper" },
      { field: "comment", value: i % 7 === 0 ? "generated for the table check" : "" },
    ],
  };
});

mkdirSync(dirname(out), { recursive: true });
writeFileSync(out, `${JSON.stringify([{ execute: { type: "batch", commands } }])}\n`);
console.log(`wrote ${String(count)} characteristics to ${out}`);
