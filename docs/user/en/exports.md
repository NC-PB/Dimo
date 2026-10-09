# Exports

## Characteristic list (CSV and XLSX)

The characteristic list has one row per characteristic, in display number order. It is meant
for programming a coordinate measuring machine (CSV) and for working in a spreadsheet (XLSX).
Both files have the same columns.

Characteristics with the status `rejected` are not exported. Reference and basic dimensions are
exported with `Inspect` set to `no`, so you can filter them out.

You choose the language of the column headers per export: English or German. The values do not
change with the language.

### Columns

| Column (English) | Column (German) | Content |
|---|---|---|
| No | Nr | Display number (the number in the balloon) |
| Kind | Art | `linear`, `diameter`, `radius`, `spherical_radius`, `angle`, `chamfer`, `thread`, `counterbore`, `countersink`, `depth`, `surface_texture`, `geometric`, `note`, `flag_note`, `material_process`, `other` |
| Requirement | Anforderung | Text as it appears on the drawing |
| Nominal | Nennmaß | Nominal value |
| Upper deviation | Oberes Abmaß | Upper deviation, with sign |
| Lower deviation | Unteres Abmaß | Lower deviation, with sign |
| Upper limit | Obere Grenze | Upper limit, absolute value |
| Lower limit | Untere Grenze | Lower limit, absolute value |
| Unit | Einheit | `mm`, `in` or `deg` |
| Fit | Passung | Fit designation such as `H7` |
| Quantity | Anzahl | Number of features the characteristic stands for (`4X` gives 4) |
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
