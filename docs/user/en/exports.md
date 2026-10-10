# Exports

## Exporting from the app

Open the **Export** view (the **Export** button in the top bar, or ⌘E on macOS, Ctrl+E elsewhere).
It needs an open project. Each export asks where to save the file and then runs in the
background; you can keep working meanwhile. The file contains the project as it was when you
clicked the button. A progress bar and then the saved file name appear below the export, or the
reason why it failed.

The suggested file names follow the project file (or the drawing, for a project that was never
saved): `part_ballooned.pdf`, `part_characteristics.csv` and `part_characteristics.xlsx`.

Options. The first two are remembered for the next start:

- **Column headers**: English or German headers in the CSV and Excel files, independent of the
  language of the app.
- **Balloons in the PDF**: as part of the page (default), or as annotations that PDF viewers can
  show, hide or delete.
- **Export as issued**: locks the numbering before exporting. Use it for drawings that go to a
  customer. While locked, numbers never change, deleted numbers are not reused, a new
  characteristic gets the highest number ever used plus one, and rows of the table cannot be
  moved. The lock is recorded in the change log. This box is not remembered and is off at every
  start. While the numbering is locked the box is disabled and says so.

Dimo has no unlock button in this version. The lock is a change like any other, so ⌘Z undoes it
as long as it is the last change you made, for example right after the export. Anything you
change after the export goes on top of it in the history.

## Ballooned PDF

A copy of the drawing with the balloons drawn in, as vector graphics. The original drawing file
is not changed. Balloons look as in the app: same shape, size, colors and leader lines.
Characteristics with the status `rejected` get no balloon in the PDF.

The same project gives the same PDF file, byte for byte. In annotation mode the date of the
annotations is the time of the last change of the project, not the time of the export.

## Characteristic list (CSV and XLSX)

The characteristic list has one row per characteristic, in display number order. It is meant
for programming a coordinate measuring machine (CSV) and for working in a spreadsheet (XLSX).
Both files have the same columns.

Characteristics with the status `rejected` are not exported. Characteristics for which you cleared
**Inspect**, such as reference and basic dimensions, are exported with `Inspect` set to `no`, so you
can filter them out.

You choose the language of the column headers per export: English or German. The values do not
change with the language.

### Columns

| Column (English) | Column (German) | Content |
|---|---|---|
| No | Nr | Display number (the number in the balloon), e.g. `12`, or `12.1` and `12A` for characteristics added to a locked numbering. Plain numbers are number cells in XLSX, the others text |
| Kind | Art | `linear`, `diameter`, `radius`, `spherical_radius`, `angle`, `chamfer`, `thread`, `counterbore`, `countersink`, `depth`, `surface_texture`, `geometric`, `note`, `flag_note`, `material_process`, `other` |
| Requirement | Anforderung | Text as it appears on the drawing |
| Nominal | Nennmaß | Nominal value |
| Upper deviation | Oberes Abmaß | Upper deviation, with sign |
| Lower deviation | Unteres Abmaß | Lower deviation, with sign |
| Upper limit | Obere Grenze | Upper limit, absolute value |
| Lower limit | Untere Grenze | Lower limit, absolute value |
| Unit | Einheit | `mm`, `in` or `deg` |
| Fit | Passung | Fit designation such as `H7` |
| Quantity | Anzahl | Number of features the characteristic stands for, from the Qty column |
| Classification | Klassifizierung | `critical`, `major`, `minor` or `key`; empty if not classified |
| Inspection method | Prüfmethode | Free text |
| Gauge | Prüfmittel | Free text |
| Sampling | Stichprobe | Free text |
| Frequency | Häufigkeit | Free text |
| Inspect | Prüfen | `yes` or `no` |
| Status | Status | `proposed`, `accepted` or `verified` |
| Sheet | Blatt | Page number of the sheet, starting at 1. Empty if the characteristic has no region and no balloon |
| Comment | Kommentar | Free text |

A value that is not set is an empty field or an empty cell.

### Numbers are exact

Nominals, deviations and limits are written with exactly the digits stored in the project.
`90.0` stays `90.0` and `30.0203` stays `30.0203`. The decimal separator is always a point.

- CSV: plain text such as `-0.2`. Nothing is rounded or converted.
- XLSX: a value with up to 15 significant digits is a number cell, shown with the same number
  of decimal places as stored. A value with more digits is written as text, because a
  spreadsheet number cannot hold it exactly. Such cells are left aligned.

### CSV format

UTF-8 without byte order mark, comma as separator, one header row, `\n` line ends. A field is
put in double quotes if it contains a comma, a quote or a line break; quotes inside are doubled
(RFC 4180). Requirement texts and comments are written as they are. If you open the CSV in a
spreadsheet program, check texts that start with `=`, `+`, `-` or `@` before you save it again.

### XLSX format

One worksheet (`Characteristics` or `Merkmale`) with a bold header row, a frozen header and
column filters.

### Identical files

The same project, version and options give byte identical files. The XLSX contains no creation
time, user name or other value that depends on when or where you export.
