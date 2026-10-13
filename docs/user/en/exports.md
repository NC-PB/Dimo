# Exports

## Exporting from the app

Open the **Report** tab (⌘E on macOS, Ctrl+E elsewhere). Its commands choose the check sheet or
the inspection report, the format, the language and **Open after export**; **Save copy** exports
the chosen document, **Issue report** exports it as issued. Below them every export has a card.
The tab needs an open project. Each export asks where to save the file and then runs in the
background; you can keep working meanwhile. The file contains the project as it was when you
clicked the button. A progress bar and then the saved file name appear below the export, or the
reason why it failed. While a report export runs, **Cancel export** stops it; no file is
written then. Dimo stops between the steps of the export (prepare, render, write), so a cancel
can take a moment to take effect.

The view has two groups: **Reports** (check sheet and inspection report) and **Drawing and
characteristic list** (ballooned PDF, CSV and Excel list).

The suggested file names follow the project file (or the drawing, for a project that was never
saved): `part_ballooned.pdf`, `part_characteristics.csv`, `part_characteristics.xlsx`,
`part_check-sheet.pdf` or `.xlsx`, and `part_inspection-report.pdf` or `.xlsx`.

Options. All but **Export as issued** are remembered for the next start, as is the format chosen for each
report:

- **Language**: English or German headers in the CSV and Excel files, and the language of the
  reports, independent of the language of the app.
- **Balloons in the PDF**: as part of the page (default), or as annotations that PDF viewers can
  show, hide or delete.
- **Blank result columns**: how many empty result columns a check sheet gets when the project
  has no samples, 1 to 10 (default 3).
- **Open the file after exporting**: when an export has finished and the file is written, Dimo
  opens it with the program your computer uses for that file type, for example your PDF viewer
  for a check sheet you want to print. Off by default. It applies to every export in the view.
  Dimo only opens the file it just wrote, and only if its name still ends in the extension of the
  export (`.pdf`, `.csv` or `.xlsx`). If no program is set up for the type, the file is still
  saved and the export shows a notice.
- **Export as issued**: locks the numbering. Use it for drawings and reports that go to a
  customer. While locked, numbers never change, deleted numbers are not reused, a new
  characteristic gets the highest number ever used plus one, and rows of the table cannot be
  moved. The lock is recorded in the change log. A report exported as issued is also recorded
  in the project (see [Issued reports](#issued-reports)). This box is not remembered and is off
  at every start. While the numbering is locked the box says so; reports exported as issued are
  still recorded.

Dimo has no unlock button in this version. The lock is a change like any other, so ⌘Z undoes it
as long as it is the last change you made, for example right after the export. Anything you
change after the export goes on top of it in the history.

## Check sheet and inspection report

Both reports list the inspected characteristics in number order: characteristics with
**Inspect** cleared (reference and basic dimensions) and rejected ones are left out. Each row
shows number, requirement, nominal, limits, unit, the rule that gave the limits, method and
gauge. The header on every page shows the company, part, customer, order, drawing file,
revision, the start of the SHA-256 of the drawing file (in full on the first page), report
number, date and page. The values come from [Project and report details](project-details.md).
A header field that is empty prints neither its label nor a value.

- **Check sheet**: empty result columns to print and fill in by hand, and a line for the
  inspector. With samples in the project there is one column per sample with its serial; without
  samples the number of blank result columns set in the options, with a note to write the serial
  of each part in the column head. The check sheet shows no verdicts.
- **Inspection report**: a summary (overall verdict, number of characteristics and samples, and
  counts of pass, fail, not evaluated and missing), a sample block with one line per sample (serial,
  date, inspector, gauges, comment, its own counts and verdict), the table of results with the
  mark of every verdict and the verdict of every row, a legend of the marks, remarks, and the
  signature block. How verdicts are found is explained in [Entering results](measuring.md).

The signature block has two lines, **Inspected by** and **Approved by**, each with name, date and
an empty field for a handwritten signature. The names come from the report fields. The date of
"Inspected by" is the report date; the date of "Approved by" stays empty to be written by hand.
There are no digital signatures. If the overall verdict is open, the report says that not every
result is entered and evaluated. The failing cells and the failing samples have a bold border.

Choose **PDF** or **Excel** under **Format** of each report. Both contain the same values.

- **PDF**: A4 landscape, header on every page, table header repeated on every page. With more than
  six samples the table is printed in blocks of six sample columns, each block on a new page, with
  the characteristic columns repeated and a heading such as "Samples 7 to 12 of 12".
- **Excel**: one sheet named after the report. Values are number cells with the digits you typed;
  the mark (✓ ✗ ○) is part of the cell format, so the cell stays a number. Verdicts and OK / not OK
  results are text with the mark. Cells have a colored fill as well (green, red, yellow, grey for
  open). Columns A and B are frozen. The sheet is set up for A4 landscape with the header rows
  repeated on every printed page; with more than six samples it prints at 60 percent with a page
  break after every six sample columns, and the characteristic columns repeat. The inspection
  report carries the counts of pass, fail, not evaluated and missing for every sample, as the PDF.

Words on the reports follow the report language of the export (English or German), not the
language of the app. In German the verdicts read **i.O.** and **n.i.O.**, in English **Pass** and
**Fail**.

Limits from tolerance tables that are still drafts are marked in the rule column with an asterisk
and the report says in a footnote that these limits are from a draft table and not yet verified.

The date printed on a report is the report date of the project. If it is empty when you export
a check sheet or an inspection report, Dimo enters today's date (the date of your computer) as
the report date first, as one change you can undo with ⌘Z, and prints it. Later exports print
the stored date, so the same project gives the same files, byte for byte, on any day. To use
another date, enter it under **Report date** in [Project and report details](project-details.md)
before you export. If you clear the date again, the next report export fills in the date of that
day anew. The ballooned PDF and the lists do not use the report date.

### Issued reports

With **Export as issued** a report export does three things in one step, after the file is
written:

1. The numbering is locked, if it is not locked yet.
2. An empty company name is filled in from **Options**, an empty "inspected by" with your
   user name and an empty report date with today's date. The report already shows these
   values. A normal export prints the project as it is and leaves the other empty fields empty;
   only the report date is entered then too, as described above.
3. The project records the issued report: which report, the template and its version, the file
   format, the drawing revision and its SHA-256, the SHA-256 of the written file and the export
   time. The export time is kept in the project only, it is not printed in the report.

Recording is a change of the project. If the report date was empty, the same step also stores
today's date as the report date, so the report shows it and later exports print it as well.

⌘Z undoes all of it at once. Below the export Dimo confirms "Recorded as issued". If you changed
the project while the export ran, the file is written but nothing is recorded, because the file
no longer shows the project; export again.

Before an issued export Dimo warns when limits come from draft tables, when results are missing
or not evaluated (inspection report), or when limits or result types changed after the last
issued report ([When limits change](measuring.md#when-limits-change)). You can export anyway or
go back. The warnings never prevent the export. Both reports can be issued, as PDF and as Excel;
every issued file is recorded on its own.

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
| Rule | Regel | Rule that gave the limits: `explicit`, `fit`, `drawing_rule`, `general`, `decimal_rule`, `custom_table`, `no_tolerance_defined` or `manual` (entered by hand); empty if none is recorded ([Tolerances](tolerances.md)) |
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
