---
name: tolerance-table
description: Create or edit tolerance table data files in data/tolerances (ISO 2768-1, ISO 2768-2, ISO 286, ISO 13920, ISO 9013, custom tables) and their test vector files. Use for anything touching tolerance values or the table file format.
---

# Tolerance tables

Rules from D-43 and FR-TOL-*:

- Tables are data files, never hard coded in Rust.
- You draft values from your knowledge of the standards. You **never** mark a table verified.
  Every file you create has `status = "draft"` and no `verified_by` or `verified_date`.
  A hook blocks attempts to set them. The owner verifies against his Tabellenbuch.
- All values are decimal strings in millimetres (`"0.021"`), never floats.
- Size ranges follow the standard exactly: state whether bounds are inclusive or exclusive
  (`over 30 up to and including 120` becomes `min_exclusive = "30"`, `max_inclusive = "120"`).
- Each table names its source: standard number, edition year, table number inside the standard.
  Do not copy explanatory text from the standard. Values and structure only.
- Each table has a test vector file next to it (`<name>.test.toml`) with spot checks:
  input (nominal, class or fit) and expected limits. Add vectors for every range boundary.
  Mark which vectors you derived and leave `checked_by_owner = false`; the owner flips them.
- Hint behavior from the corpus: explicit deviations on a drawing always win. A mismatch with the
  table (inch based CAD fit tables) creates an info hint, never a correction.

When the file format does not exist yet, propose it first (TOML, with a `[table]` header holding
`id`, `standard`, `edition`, `kind`, `unit = "mm"`, `status`, `source`), add a JSON Schema via
`schemars`, and document it in `data/tolerances/README.md`.

After drafting, add the table to "Waiting for the owner" in `docs/plan/STATUS.md` with the list of
values to verify, so the owner can check them in one pass.
