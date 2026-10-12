# Report templates

Report template sets for PDF reports (FR-EXP-06, D-30, D-32). Dimo ships one set,
`dimo-default`, embedded into the binary at compile time. Custom template sets (M7) use the same
folder layout and the same data format.

## Template set layout

```
dimo-default/
  template.toml          id, version, label file, documents
  labels.toml            label texts, one table per language ([en], [de])
  common.typ             page setup, header, marks and table parts shared by both documents
  check-sheet.typ        document "check-sheet"
  inspection-report.typ  document "inspection-report"
```

`template.toml`:

| Key | Meaning |
|---|---|
| `id` | Template id, recorded with every issued report |
| `version` | Template version, increased whenever the output changes |
| `labels` | Optional. Label file of the set, a TOML table per language with the same keys |
| `[[documents]]` | `name` (selected when rendering) and `main` (the Typst file compiled for it) |

A document is a Typst file. It is compiled in a sandbox: it can read the
files of its own set, the data file `/data.json` and the bundled Open Sans fonts. Files outside
the set, packages, plugins and the clock are refused (NFR-SEC-05). The report language is
`sys.inputs.lang` (`en` or `de`). `data.json` is a reserved file name.

Rules for templates (AGENTS.md rules 2 and 5):

- Print the values as they come. Every printed value is already text, formatted by Dimo.
- Never compute with measured values or limits, never decide pass or fail. Verdicts come from
  Dimo.
- Every verdict needs a shape as well as a color, so a black and white print reads correctly
  (NFR-UX-05). Open Sans has no check mark, cross or circle glyphs; `common.typ` draws them.
- Open Sans has no `⌀` (U+2300); map it to `Ø` with a show rule.

## The two documents

Both are A4 landscape with a header on every page: company, document title, part number and
name, customer, order reference, drawing file name, revision, the first 16 hex digits of the
drawing SHA-256, report number, report date and page x of y. The full SHA-256 is printed on the
first page (FR-RVC-04, FR-DOC-07).

- **Check sheet**: the characteristics to inspect with empty result columns, for printing. One
  column per sample with its serial, or blank columns (1 to 10, default 3) when the project has
  no samples yet.
- **Inspection report**: summary (overall verdict and the number of results per verdict), one
  line per sample (serial, date, inspector, gauges, comment, counts, verdict), the
  characteristic table with the results and their marks, remarks and the signature block
  (inspected by and approved by, each with name, date and an empty field for a handwritten
  signature).

The characteristic columns are number, requirement, nominal, lower and upper limit, unit, rule
(with `*` for limits from a draft tolerance table and a footnote), method and gauge. With more
than six samples the table repeats in column blocks of up to six samples, each starting on a new
page with the characteristic columns repeated. Table header rows repeat on every page.

Marks: pass is a check mark, fail a cross with a bold cell frame, not evaluated a hollow circle,
an open group verdict a short bar, a missing result an empty cell.

## Data format

Dimo writes one JSON object to `/data.json`. Its JSON Schema is
[`docs/schema/report-data.schema.json`](../../docs/schema/report-data.schema.json), generated
from `dimo_report::ReportModel`; a test fails when the two differ. `format_version` is
increased when a field is removed or changes meaning; added fields keep the version.

All values are strings unless noted. Empty text means not set.

- Numbers keep the decimals they were typed with (`12.10` stays `12.10`) and use the decimal
  separator of the report language: point in English, comma in German.
- Dates are `YYYY-MM-DD` in both languages.
- Verdicts, rules, kinds, attribute results and classifications are stable codes in English
  snake case. The template translates them with its label file.

### Top level

| Field | Content |
|---|---|
| `format_version` | Number, currently `1` |
| `document` | `check_sheet` or `inspection_report` |
| `language` | `en` or `de` |
| `header` | Page header, see below |
| `summary` | Overall result, see below |
| `samples` | Result columns in order, see below |
| `column_blocks` | At least one block of sample columns, see below |
| `rows` | Measured characteristics in number order, see below |
| `remarks` | Remarks of the report, line breaks as `\n` |
| `signatures` | `inspected_by` and `approved_by`, each `{ name, date }` |
| `flags` | `draft_limits`, `open_results`, `blank_samples` (booleans) |

Only characteristics that are inspected and not rejected are rows (M3 decision 5).

### `header`

`company`, `part_number`, `part_name`, `customer`, `order_reference`, `drawing_file`, `revision`,
`sha256` (64 hex digits), `sha256_short` (first 16), `report_number`, `report_date`.

The report date is the date in the report details, else the modification date of the project
file, else empty. It is never the current date (FR-EXP-11).

### `summary`

| Field | Content |
|---|---|
| `verdict` | `pass`, `fail` or `open` |
| `counts` | `{ pass, fail, not_evaluated, missing }`, number of results per verdict as text |
| `characteristics` | Number of rows |
| `samples` | Number of samples |

### `samples[]`

| Field | Content |
|---|---|
| `index` | Position from 1 |
| `serial`, `date`, `inspector`, `gauges`, `comment` | As entered |
| `blank` | Boolean, true for a blank check sheet column (no samples in the project) |
| `verdict`, `counts` | Verdict and counts of this part, as in `summary` |

### `column_blocks[]`

`from` and `to` (numbers) index `samples` and each row's `cells`, `to` exclusive; `first` and
`last` are the printed positions. A report without samples has one block with `from = to = 0`.

### `rows[]`

| Field | Content |
|---|---|
| `number` | Balloon number, such as `12` or `12.1` |
| `kind` | Characteristic kind code, such as `diameter` or `thread` |
| `requirement` | Text as on the drawing |
| `quantity` | Number of features |
| `nominal`, `lower_limit`, `upper_limit` | Formatted numbers or empty |
| `upper_deviation`, `lower_deviation` | With sign (`+0.1`, `-0.2`, `0`) or empty |
| `unit` | `mm`, `in`, `°` or empty |
| `fit` | Fit designation or empty |
| `rule` | `explicit`, `fit`, `drawing_rule`, `general`, `decimal_rule`, `custom_table`, `no_tolerance_defined`, `manual`, or `null` |
| `rule_detail` | Table and class or parameter without words, such as `ISO 2768-1 m`, `ISO 286 H7`, `±0.05`; else empty |
| `draft` | Boolean: limits from a draft tolerance table (D-43) |
| `method`, `gauge` | Inspection method and gauge |
| `classification` | `critical`, `major`, `minor`, `key` or `none` |
| `result_type` | `variable` or `attribute` |
| `verdict`, `counts` | Verdict and counts of this characteristic over all samples |
| `cells` | One per entry of `samples`, see below |

### `cells[]`

| Field | Content |
|---|---|
| `type` | `none` (no result, and every cell of a check sheet), `variable` or `attribute` |
| `value` | Formatted number (`variable`), `ok` or `not_ok` (`attribute`), else empty |
| `verdict` | `pass`, `fail`, `not_evaluated` or `missing` |
| `comment` | Comment of the result |
