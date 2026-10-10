# Tolerance tables

Data files of the tolerance engine (`crates/dimo-tolerance`). Values never live in code (D-43).
Shipped tables are embedded into the app at build time; users can add their own tables in the
same format (FR-TOL-07).

| File | Table | Status |
|---|---|---|
| `iso-2768-1.toml` | ISO 2768-1:1989 general tolerances, tables 1, 2 and 3 | draft |
| `iso-286.toml` | ISO 286-1:2010 standard tolerance grades IT01 to IT18 and fundamental deviations up to 3150 mm, limits cross-checked with ISO 286-2:2010 | draft |

## Verification (D-43)

Every table starts as `status = "draft"`. The values are drafted by the agent from its knowledge
of the standards and must be checked by the owner against his Tabellenbuch. Only the owner
changes the status and then also records his name and the date (field names in the schema).
Agents never do this; a hook blocks it.

- Draft tables work in development builds. The UI marks limits derived from them with a badge.
- `scripts/check-table-status.sh` is the release gate. It fails with `DIMO_RELEASE=1` or on a tag
  build while any table here is a draft, and only warns otherwise. `check.sh` and CI run it.

## File format

TOML, schema `docs/schema/tolerance-table.schema.json` (generated from the Rust types in
`crates/dimo-tolerance/src/format.rs`). All values are decimal strings such as `"0.021"`, never
TOML numbers. Unknown keys are errors.

```toml
[table]
id = "iso-2768-1"          # unique across shipped and user tables, lowercase and '-'
title = "ISO 2768-1 general tolerances"
standard = "ISO 2768-1"    # optional for company tables
edition = "1989"           # optional
kind = "general"           # general, fit or custom
unit = "mm"                # unit of all size ranges
version = 1                # increase with every change of a value
status = "draft"
source = "ISO 2768-1:1989, tables 1, 2 and 3"

[[part]]                   # one or more range tables
id = "linear"              # unique in the file, lowercase and '_'
title = "Linear dimensions"  # optional
source = "table 1"         # table number in the source
kind = "symmetric"         # see "Part kinds"
applies_to = "linear"      # symmetric parts: linear, radius_chamfer or angular
range_of = "nominal"       # optional: nominal (default) or shorter_leg (angles)
value_unit = "mm"          # mm, or arcmin for angular parts
columns = ["f", "m", "c", "v"]
rows = [
  { min_inclusive = "0.5", max_inclusive = "3", values = ["0.05", "0.1", "0.2", "-"] },
  { min_exclusive = "3", max_inclusive = "6", values = ["0.05", "0.1", "0.3", "0.5"] },
]
```

### Rows and ranges

Each row has at most one lower bound (`min_inclusive` or `min_exclusive`) and at most one upper
bound (`max_inclusive` or `max_exclusive`). Write the bounds exactly as the source states them:
"over 30 up to and including 120" is `min_exclusive = "30"`, `max_inclusive = "120"`.

- Rows are ascending and contiguous: each row starts where the previous one ends, and the bound
  belongs to exactly one of the two rows.
- Only the first row may omit the lower bound (it then starts above zero). Only the last row
  may omit the upper bound (open ended).
- `values` has one entry per column. `"-"` means the source defines no value there.
- Sizes outside all rows have no value.

### Part kinds

| Kind | Values | Columns | Used by |
|---|---|---|---|
| `symmetric` | tolerance `t`, deviations are `+t` and `-t` | classes, e.g. `f`, `m` | general and custom tables |
| `standard_tolerance` | standard tolerance grade | `IT01`, `IT0`, `IT1` to `IT18` | fit tables |
| `fundamental_deviation` | fundamental deviation, signed; needs `feature` (`shaft`, `hole`) and `deviation` (`upper`, `lower`) | deviation letters | fit tables |
| `delta` | delta values for holes | `IT3` to `IT8` | fit tables |
| `override` | fundamental deviation that replaces the computed one | tolerance classes, e.g. `M6` | fit tables |

General and custom tables hold only `symmetric` parts, at most one per `applies_to`. Fit tables
hold no `symmetric` part.

Angular values are in minutes of arc (`value_unit = "arcmin"`, 60 is one degree), because values
such as 20' have no exact decimal in degrees.

### Fit tables (ISO 286, FR-TOL-04)

A fit table (`kind = "fit"`) has one `standard_tolerance` part, one `delta` part, four
`fundamental_deviation` parts (shaft upper, shaft lower, hole upper, hole lower) and optionally
one `override` part. Values are in mm, converted from the micrometres of the standard. The
fundamental deviation parts share one row grid with the intermediate steps of the standard, so
columns without intermediate steps repeat their value; a split row at 1 mm marks the classes
that the standard does not use up to 1 mm.

The engine combines the values with the rules of the ISO system
(`crates/dimo-tolerance/src/fit.rs`). Column names of the fundamental deviation parts:

| Part | Columns | Meaning |
|---|---|---|
| shaft, upper | `a` to `h`, `cd`, `ef`, `fg` | es; ei = es - IT |
| shaft, lower | `j5`, `j6`, `j7`, `j8` | ei of j for that grade |
| shaft, lower | `k` | ei of k for IT4 to IT7 |
| shaft, lower | `k_other` | ei of k for the other grades |
| shaft, lower | `m` to `zc` | ei; es = ei + IT |
| hole, lower | `A` to `H`, `CD`, `EF`, `FG` | EI; ES = EI + IT |
| hole, upper | `J6`, `J7`, `J8` | ES of J for that grade |
| hole, upper | `K`, `M`, `N` | ES for grades up to IT8 before adding delta |
| hole, upper | `K_above_IT8`, `M_above_IT8`, `N_above_IT8` | ES for grades above IT8 |
| hole, upper | `P` to `ZC` | ES for grades above IT7; delta is added up to IT7 |

js and JS have no column: their deviations are exactly +IT/2 and -IT/2, also when that gives
half micrometres (ISO 286-2:2010). `override` columns name a tolerance class and replace its
fundamental deviation in the given range (M6 from 250 to 315 mm). The intermediate deviations
`CD`, `EF`, `FG` and `cd`, `ef`, `fg` are defined up to 50 mm.

Above 500 mm the standard defines no IT01, IT0 and no delta values, and gives K, M, N and P to U
without delta; the delta part therefore holds zero there. a to c, A to C, j, J, v to zc and V
to ZC end at 500 mm.

### Custom tables (FR-TOL-07)

A company table uses `kind = "custom"`, any id that is not taken, and `symmetric` parts. Name the
company document in `source`. Put the file into the user tables folder; Dimo validates it on
load and refuses it with the file name and the line or the part and row of the problem. A
project that uses a custom table stores a copy, so it opens with the same limits elsewhere.

## Test vectors

Each table has a test vector file `<id>.test.toml` next to it. `cargo test -p dimo-tolerance`
runs every vector. General tables use `general` vectors, fit tables `fit` vectors.

```toml
table = "iso-2768-1"
general = [
  { applies_to = "linear", class = "m", size = "30", value = "0.2", derived_by = "agent", checked_by_owner = false },
  { applies_to = "linear", class = "v", size = "0.5", value = "-", derived_by = "agent", checked_by_owner = false },
]
```

```toml
table = "iso-286"
fit = [
  { fit = "H7", nominal = "30", upper = "0.021", lower = "0", derived_by = "agent", checked_by_owner = false, note = "corpus truth c03" },
]
```

- `value = "-"` (or `upper = "-"`, `lower = "-"`) expects no value (size outside the table or
  no value defined). `upper` and `lower` are deviations in mm. `note` is optional.
- Vectors cover every range bound: on the bound and just above it.
- `derived_by = "agent"` marks vectors written by the agent. The owner sets
  `checked_by_owner = true` per vector after comparing it with the printed table.

## Adding a shipped table

1. Write the table and its test vector file here, `status = "draft"`.
2. Add the file to `SHIPPED` in `crates/dimo-tolerance/src/load.rs` (a test checks the list).
3. Add it to the table above and to "Waiting for the owner" in `docs/plan/STATUS.md` with the
   values to verify.
