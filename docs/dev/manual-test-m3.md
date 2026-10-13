# Manual test script, milestone 3

For what `scripts/e2e-smoke.sh` cannot reach: the real clipboard paste in the measurement grid
(the automated tests send a synthetic paste event), the look of the grid marks, the four XLSX
reports in a spreadsheet program, the printed layout of the PDF reports, the German wording, and
the user guide ([testing.md](testing.md)). It follows the same idea as
[manual-test-m2.md](manual-test-m2.md) and takes about 75 minutes. Run it before the M3 sign off
and after changes to the grid, the evaluation, the report templates or the report export. Record
the result in the table at the end and put the notes in the pull request or in
`docs/plan/STATUS.md`.

All tolerance tables are still drafts (D-43). Every limit derived from a table therefore carries
the **Draft** marker in the grid and an asterisk plus a footnote in the reports; that is expected.

## Before you start

1. Run `./scripts/check.sh` and `./scripts/e2e-smoke.sh`. Both must pass first.
2. Start the app with `pnpm dev` (a debug build) or from an installed build. Note which. Use the
   operating system's own clipboard, not a remote desktop, for section 5.
3. Drawing: `corpus/drawings/test_drawing_1.pdf` (no general tolerance declared).
4. A spreadsheet program that can open `.xlsx` files (Excel, LibreOffice Calc or Numbers) and one
   that can edit cells to copy a block from (the same one is fine). Note which and its version.
5. Put the files you create in a folder you can delete afterwards.

Write down: date, tester, operating system and version, Dimo version and build, spreadsheet
program and version, display scale.

## 0. The test project

Create a new project from the drawing. Place balloons with the **Place balloon** tool (B), typing
the requirement text after each click, in this order. Leave the settings alone, so no general
tolerance applies.

| # | Type | Expected limits (upper / lower) |
|---|---|---|
| 1 | `Ø10 ±0.1` | 10.1 / 9.9 |
| 2 | `20 ±0.05` | 20.05 / 19.95 |
| 3 | `100 +0 -0.6` | 100 / 99.4 |
| 4 | `7.5`, then type `7.5` into **Upper limit** of the table and leave **Lower limit** empty | 7.5 / none (one sided) |
| 5 | `50` (rule "No tolerance defined") | none |
| 6 | `Deburr all edges` | none |
| 7 | `(42)` | none, **Inspect** cleared |
| 8 | `Ø8 f7` | 7.987 / 7.972, marker **Draft** |

In **Project and report** (side panel) enter part number `T-100`, part name `Test part`,
customer `Test customer`, revision `A`. Save the project as `m3-test.dimo`.

## 1. Samples (FR-MEA-01)

Open the **Measure** view (⌘3).

| # | Step | Expected |
|---|---|---|
| 1.1 | Look at the view | Seven rows (1 to 6 and 8): the reference dimension 7 is not listed. No sample columns; a note says to add samples |
| 1.2 | Type `3` under **Number** and choose **Add samples** | Three columns `1`, `2`, `3`; the sample panel shows the active sample with inspector = your user name, date empty |
| 1.3 | Type `100` under **Number**, add, then undo with ⌘Z | 100 samples added in one step, one undo removes them all. A number above 100 is refused |
| 1.4 | Click the header of sample 2, change the serial to `1` in the panel | Refused with a message; the field shows `2` again |
| 1.5 | Change the serial of sample 3 to ` A-17 ` (with spaces) | Stored as `A-17`. Change it back to `3` |
| 1.6 | Enter a date, gauges and a comment for sample 1 | They stay after you click another sample and back |
| 1.7 | Choose **Move right** for sample 1 | Order is 2, 1, 3 and the active column follows. ⌘Z restores it |
| 1.8 | Add one more sample, enter nothing, choose **Delete sample** | Deleted at once (no results). ⌘Z brings it back, ⇧⌘Z deletes it again; three samples remain |

## 2. Entering results with the keyboard (FR-MEA-02, NFR-UX-01)

First set the **Result type** of row 5 (`50`, no limits) to **Value** in the grid: with **Auto** a
characteristic without limits takes OK / not OK, and a number would be refused.

Click the first cell of sample 1. Use the keyboard only from now on. Type each value and press
**Enter**.

| # | Sample 1 | Sample 2 | Sample 3 |
|---|---|---|---|
| Row 1 | `10.00` | `10,1` | `9.89` |
| Row 2 | `20.05` | `20.06` | `20,00` |
| Row 3 | `99.4` | `99.39` | `100.0` |
| Row 4 | `7.5` | `7.4` | `7.51` |
| Row 5 | `50.2` | `49` | `50` |
| Row 6 | `o` | `n` | `o` |
| Row 8 | `7.980` | `7.987` | `7.971` |

For samples 2 and 3 click the first cell of the column and repeat. In row 6 the keys **O** and
**N** set the result at once (no Enter needed).

| # | Step | Expected |
|---|---|---|
| 2.1 | Sample 1 after typing, check each cell | ✓ in every cell except row 5 (○). The values read as typed (`10.00`, `7.980`) |
| 2.2 | Sample 2 | ✓ ✗ ✗ ✓ ○ ✗ ✓ (rows 1 to 6, 8). The ✗ cells have a bold border and bold text |
| 2.3 | Sample 3 | ✗ ✓ ✓ ✗ ○ ✓ ✗. `20,00` shows as `20.00`, `100.0` stays `100.0` |
| 2.4 | Sample headers | Sample 1: 6 pass, 0 fail, 1 not evaluated, 0 missing. Samples 2 and 3: 3 pass, 3 fail, 1 not evaluated, 0 missing |
| 2.5 | Row summaries | Row 1: 2 pass 1 fail; row 2: 2 pass 1 fail; row 3: 2 pass 1 fail; row 4: 2 pass 1 fail; row 5: 3 not evaluated; row 6: 2 pass 1 fail; row 8: 2 pass 1 fail |
| 2.6 | Line above the grid | Overall verdict: fail |
| 2.7 | Press **Shift+Enter**, **Tab**, **Shift+Tab** and the arrow keys in the grid | Shift+Enter commits and goes up, Tab and Shift+Tab go to the next and previous sample, arrows move without editing. Tab past the last sample leaves the grid |
| 2.8 | Move to a filled cell, press **F2**, change the value, press **Escape** | The old value stays. Press **Delete**: the result is cleared (empty cell), ⌘Z restores it |
| 2.9 | Type `abc` in a value cell and press Enter | The cell opens again; the line above the grid says it is not a number. Nothing is stored |
| 2.10 | Type `5` in a row 6 cell and press Enter | The cell opens again; the line above the grid says to type o or + for OK, n or x for not OK. Nothing is stored |
| 2.11 | Click the row 6 cell of sample 1 (OK) and press **Space** twice | Not OK, then OK again |
| 2.12 | Click the row 6 cell of sample 2. Press **F2**, type `n.i.O.`, Enter. Click the cell again and do the same with `ok`, then `nio`, then `io` | Not OK, OK, not OK, OK; each is stored without error |
| 2.13 | Make the final values equal to the table above again | Same marks and counts as 2.1 to 2.4 |

## 3. Pass and fail rules (FR-MEA-03)

Use the values of section 2 and check them against the limits yourself.

| # | Check | Expected |
|---|---|---|
| 3.1 | Row 1, sample 2: 10.1 equals the upper limit | ✓ (limits are inclusive) |
| 3.2 | Row 2, sample 1 and row 3, sample 1: values equal to a limit | ✓ |
| 3.3 | Row 2, sample 2: 20.06 | ✗ (just outside) |
| 3.4 | Row 4 (one sided, upper limit 7.5 only): 7.5 and 7.4 pass, 7.51 fails | ✓ ✓ ✗. Enter `0` in sample 2 and it also passes (no lower limit); restore `7.4` |
| 3.5 | Row 5 (no limits) | ○ in every cell; not counted as a failure; sample 1 is not pass (see the summary in section 8) |
| 3.6 | Type `20.050` in row 2, sample 1 | Still ✓ and shown as `20.050` (the exact value is compared, the typed digits are kept). Restore `20.05` |
| 3.7 | Look at the grid in greyscale (macOS: System Settings > Accessibility > Display > Color filters > Grayscale) | ✓, ✗ with border, ○ and empty cells can still be told apart |
| 3.8 | Switch the app to dark mode | Same marks, readable |

## 4. Result types and refusals (FR-MEA-06)

| # | Step | Expected |
|---|---|---|
| 4.1 | Look at the **Result type** column | Rows 1 to 4 and 8: **Value** (they were **Auto** and the first result fixed the type); row 5: **Value** (you set it); row 6: **OK / not OK** (fixed by the first result) |
| 4.2 | Click the drawing view and select characteristic 4; in the detail panel look at **Result type** | The same value as in the grid; change it in either place and the other follows |
| 4.3 | In the grid set row 6 to **Value** | Refused: the result type is not changed and the line above the grid says to clear the results first. The rows keep their results |
| 4.4 | In the grid set row 1 to **Auto** | Refused with the same message (results exist, T3.9). The type stays **Value** |
| 4.5 | Clear the three results of row 6 with **Delete**, set the type to **Value** | Allowed. Set it back to **OK / not OK**, then enter `o`, `n`, `o` again |
| 4.6 | Add a note characteristic with no results yet: place a balloon and type `Check the thread`. Look at its **Result type** | **Auto (OK / not OK)**, because there are no limits |
| 4.7 | In the table give that characteristic limits (Upper limit `5`, Lower limit `4`) before any result | The grid shows **Auto (Value)**, because **Auto** follows the limits while no result exists |
| 4.8 | Enter `4.5` in its first cell, then remove both limits in the table | The type shows **Value** (the first result fixed it); the cell shows ○. ⌘Z twice restores the limits. Then delete the characteristic with the Delete button (its results go with it, undo restores both) |

## 5. Pasting a block with the real clipboard (FR-MEA-02)

This is the step the automated tests cannot cover. Add three samples so that the project has
samples 4, 5 and 6 (serials continue). In the spreadsheet program format the cells as text (so
that `+` stays a plus sign and is not read as a formula), type this block into seven rows and
three columns (separate cells, not text with spaces) and select it:

| | A | B | C |
|---|---|---|---|
| 1 | `10` | `10,05` | `9.8` |
| 2 | `20` | `20` | `20` |
| 3 | `100` | `99.5` | `99.9` |
| 4 | `7` | `7.1` | `7.2` |
| 5 | `49` | `50` | `51` |
| 6 | `ok` | `n` | `+` |
| 7 | `7.98` | `7.98` | `7.98` |

Copy it with ⌘C (Ctrl+C elsewhere).

| # | Step | Expected |
|---|---|---|
| 5.1 | Click row 1 of sample 4 in Dimo, press **⌘V** (Ctrl+V) | 21 results appear at once. Sample 4: ✓ in every cell except row 5 (○); sample 5: ✗ in row 6; sample 6: ✗ in row 1 |
| 5.2 | The values | `10,05` is stored as `10.05`; the other values read as pasted |
| 5.3 | Press ⌘Z once | All 21 results disappear in one step |
| 5.4 | Press ⇧⌘Z | All 21 are back |
| 5.5 | Click row 1 of sample 5 and paste again | Nothing is pasted; a message says the block does not fit (3 columns from sample 5) |
| 5.6 | Click row 4 of sample 4 and paste | Nothing is pasted; the message says the block does not fit (7 rows from row 4) |
| 5.7 | In the spreadsheet change one cell to `abc`, copy, paste at row 1 of sample 4 | Nothing is pasted, a message says a value does not fit; no partial result |
| 5.8 | Copy a block of two rows and two columns with one empty cell, paste it over filled cells | The result in the cell that matches the empty cell is cleared, the others are set; one ⌘Z undoes it (do that now) |
| 5.9 | If the app has an **Edit** menu, try Edit > Paste with the same block | Note whether it works (information only) |
| 5.10 | Copy a single value, click a cell and paste | That cell is set; ⌘Z restores it |
| 5.11 | Paste while a cell is being edited (after F2), then press Escape | No other cell changes (information only: note what the editor shows) |

Leave samples 4 to 6 in the project and add six more samples (serials 7 to 12, without results)
for sections 8 to 10.

## 6. Project and report details (FR-RVC-04, FR-DOC-07)

| # | Step | Expected |
|---|---|---|
| 6.1 | Open **Settings**, enter the company name `Test Works`, close the project and create a new one from the drawing | The new project's **Company** field shows `Test Works` and **Inspected by** shows your user name; the project is not marked as changed |
| 6.2 | Open `m3-test.dimo` again | Company and inspector of this project are what they were (an empty company stays empty). Opening never changes them |
| 6.3 | In `m3-test.dimo` enter company `Test Works`, inspected by `A. Tester`, approved by `B. Boss`, report number `R-1`, remarks `Test run` | Each field saves when you leave it; each is one undo step |
| 6.4 | Choose **Today** next to the report date, then undo (⌘Z), then leave the field empty | **Today** enters the date of your computer once; it does not change by itself later. After the undo the field is empty again; leave it empty, step 9 covers the export |
| 6.5 | Check the SHA-256 and **Copy** | The value equals `shasum -a 256` of the drawing file |
| 6.6 | Save, close, reopen | Revision, report number, date and all fields are kept |

## 7. Check sheet (FR-EXP-04)

Open the **Export** view (⌘E). Do not tick **Export as issued** in sections 7 to 10.

| # | Step | Expected |
|---|---|---|
| 7.1 | Report language English, **Check sheet**, format **PDF**, export | File `m3-test_check-sheet.pdf` is suggested. A4 landscape, header with company, part, customer, drawing file, revision A, report number, date, SHA-256 start and "Page 1 of n"; the full SHA-256 on the first page |
| 7.2 | The table | The seven characteristics in number order with requirement, nominal, limits, unit, rule, method; the reference dimension 7 is missing; one empty column per sample (12), serials in the head; no verdicts; an "Inspected by" line. The asterisk and footnote mark balloon 8 as draft. More than six samples: the table is printed in two blocks, "Samples 1 to 6 of 12" and "Samples 7 to 12 of 12" |
| 7.3 | Print it in black and white or use the print preview | Readable, empty columns usable for handwriting |
| 7.4 | Export the check sheet as **Excel** | See section 10 |
| 7.5 | Create a new project without samples and export its check sheet with **Blank result columns** set to 4 | Four empty columns and a note to write the serial of each part in the head |

## 8. Inspection report as PDF (FR-EXP-04, FR-MEA-03, NFR-UX-05)

Export the inspection report of `m3-test.dimo` as **PDF**, English.

| # | Check | Expected |
|---|---|---|
| 8.1 | Header and first page | As the check sheet, title "Inspection report", report number `R-1`, the date you set, full SHA-256 |
| 8.2 | Summary | Overall result **Fail** with the fail mark; 7 characteristics, 12 samples; 28 pass, 8 fail, 6 not evaluated, 42 missing (samples 1 to 6 have results, samples 7 to 12 none; recount if you changed values) |
| 8.3 | Sample block | One line per sample with serial, date, inspector, gauges, comment, its counts and verdict: samples 1 and 4 **Open**, samples 2, 3, 5 and 6 **Fail**, samples 7 to 12 **Open**; the failing samples have a bold border |
| 8.4 | Results table | Marks next to the values; failing cells have a bold border; a last column with the verdict of each row |
| 8.5 | Column blocks | Two blocks of six sample columns, each starting on a new page, with the characteristic columns and the table header repeated |
| 8.6 | Legend | Lists the marks for pass, fail, not evaluated, open and "empty: missing" |
| 8.7 | Draft footnote | Balloon 8 shows an asterisk after its rule; the footnote says the limits come from a draft table |
| 8.8 | Signature block | **Inspected by** `A. Tester` with the report date, **Approved by** `B. Boss` with an empty date, empty fields for signatures. Remarks `Test run` above |
| 8.9 | Open note | Absent: the note that not every result is entered and evaluated appears only when the overall verdict is open, and here it is Fail. To see it, export a project without samples (section 11.9) |
| 8.10 | Print in black and white | Pass, fail, not evaluated and open can be told apart by shape |

Then export the same report in **German**.

| # | Check | Expected |
|---|---|---|
| 8.11 | Words | Title **Prüfbericht**, verdicts **i.O.** and **n.i.O.**, OK results **i.O.**, **Nicht bewertet**, **Fehlt**, **Offen**; decimal comma in values and limits (`20,05`), dates as `YYYY-MM-DD` |
| 8.12 | Umlauts and signs | `Ø`, `±`, `ä`, `ö`, `ü`, `ß` print correctly, nothing is replaced by a box |
| 8.13 | Read it as a German speaker | Wording is natural; list what is not in section 14 |

## 9. Report dates and identical files (FR-EXP-11)

| # | Step | Expected |
|---|---|---|
| 9.1 | Export the English inspection report PDF and Excel twice to two folders and compare with `cmp` | Byte identical |
| 9.2 | With the **Report date** empty, export the PDF | The header shows today's date, and the **Report date** field in the side panel now shows it too. One ⌘Z empties the field again |
| 9.3 | Export again, also after changing the clock or on the next day if you can | Same date, byte identical file |
| 9.4 | Type another date (for example `2026-01-05`) as report date and export | The header shows the date you typed; a later export does not replace it |

## 10. The four XLSX files in a spreadsheet program

Export the **check sheet** and the **inspection report** as **Excel**, each in English and in
German. Open every file in the spreadsheet program and keep the program's default settings.

| # | Check | Expected |
|---|---|---|
| 10.1 | Open each of the four files | No repair or recovery prompt, no warning about the file format or content |
| 10.2 | Sheet name | One sheet, named after the document (English or German) |
| 10.3 | Header block | Company, part, customer, order, drawing, revision, full SHA-256, report number, date |
| 10.4 | Marks | ✓, ✗ and ○ render as symbols (no boxes or question marks); open shows `-`. Cells have fills: green, red, yellow, grey for open. Failing cells are bold with a border |
| 10.5 | Numbers | `10.00` and `7.980` are number cells (right aligned, selectable in a sum) and show the digits as typed; select one and look at the formula bar |
| 10.6 | OK results | Text such as `✓ OK` and `✗ Not OK` (German `✓ i.O.`, `✗ n.i.O.`) |
| 10.7 | Inspection report summary | Overall result, characteristics, samples, counts of pass, fail, not evaluated and missing |
| 10.8 | Sample block | Date, inspector, gauges, comment, counts and verdict for each sample, as in the PDF (T3.9) |
| 10.9 | Freeze | Columns A and B stay when you scroll right; the header rows do not freeze |
| 10.10 | Print preview of the inspection report | A4 landscape. The table header rows repeat on every page; columns A to H (the characteristic columns) repeat on every page; the 12 samples are printed as two blocks of six sample columns (a page break after sample 6), at about 60 percent |
| 10.11 | Print preview of the check sheet | Same page setup |
| 10.12 | Header and footer of the printout | Company, title, part, report number, date and page numbers |
| 10.13 | Compare cell by cell with the PDF | Same values, marks and verdicts |
| 10.14 | Save the file again from the spreadsheet program | No error (a smoke check that the structure is standard) |

## 11. Issued reports (D-23, FR-BAL-10)

Use a copy of the project. Clear **Company**, **Inspected by** and **Approved by** in the copy
first, so the fill is visible.

| # | Step | Expected |
|---|---|---|
| 11.1 | In the **Export** view tick **Export as issued** and export the inspection report as PDF | Before it starts Dimo warns that some limits come from draft tables; **Back** cancels, **Export anyway** continues |
| 11.2 | After the export | Below the export: recorded as issued, numbering locked |
| 11.3 | Open the PDF | **Inspected by** shows your user name and the company shows the name from **Settings**, as the empty fields were filled for the issued export |
| 11.4 | Look at **Project and report** | The company and inspected by fields now hold these values |
| 11.5 | Press ⌘Z once | The lock, the filled fields and the record are undone together; the **Export as issued** hint no longer says locked |
| 11.6 | Press ⇧⌘Z, save, then run `unzip -p m3-test-copy.dimo project.json \| jq '.issued_reports'` | One entry: kind `inspection_report`, template `dimo-default` version 1, the revision, the SHA-256 of the drawing, `output_sha256` equal to `shasum -a 256` of the exported file, format `pdf`, an export time |
| 11.7 | Export the same report as issued again as Excel | A second entry with format `xlsx`; warnings as before |
| 11.8 | A normal export (**Export as issued** off) with empty company in the project | Company stays empty on the report; nothing is recorded; numbering is untouched |
| 11.9 | New project with one characteristic `Ø10 ±0.1` and no samples, export the inspection report as issued | The warning also says results are missing or not evaluated and the overall result is open; no draft warning |
| 11.10 | Export the check sheet as issued | Warnings only for drafts; recorded with kind `check_sheet` |
| 11.11 | Change the project while an export is running (use the slow export of section 13) | The file is written, nothing is recorded, the message says the project changed; export again |

## 12. When limits change (decisions 2 and 16)

Start from the issued copy of section 11 (at least one issued report recorded, results entered).

| # | Step | Expected |
|---|---|---|
| 12.1 | Look at the Measure view right after the issued export | No triangle next to any number |
| 12.2 | Change the **Lower dev.** of characteristic 1 from -0.1 to -0.2 in the table | The limits change to 10.1 / 9.8; the results stay; the marks are recomputed at once (sample 3, `9.89`, was ✗ and is now ✓); a triangle appears next to number 1 with the hint "Limits or result type changed after the last issued report" |
| 12.3 | Look at the other rows | No other change; no triangle on other numbers |
| 12.4 | Press ⌘Z until the lower deviation is -0.1 again | `9.89` is ✗ again and the triangle is gone |
| 12.5 | Type `20.050` into **Upper limit** of characteristic 2 (it was `20.05`) | The marks do not change, but the triangle appears, because the written limit differs; ⌘Z removes it |
| 12.6 | Add a new characteristic after the issued report | It has no triangle |
| 12.7 | Remove the limits of characteristic 3 (clear both limit cells) | The results of row 3 stay and show ○; the type stays **Value**; a triangle appears next to number 3 |
| 12.8 | With the change of 12.7 still in place, start an inspection report export as issued | The warning says limits or result types changed after the last issued report; choose **Back**, then press ⌘Z until the limits of characteristic 3 and the marks are back |
| 12.9 | Delete characteristic 1 with the toolbar button | No question; its results go with it in the same step. ⌘Z restores the characteristic, its balloon and its results |
| 12.10 | Delete a sample that has results | Dimo asks first, naming the number of results; **Keep** leaves it, **Delete with results** removes it; ⌘Z restores both |

## 13. Cancel (NFR-PERF-06)

A normal export finishes in well under a second, so use the debug build with a delay:
`DIMO_DEV_EXPORT_DELAY_MS=2000 pnpm dev`.

| # | Step | Expected |
|---|---|---|
| 13.1 | Start an inspection report export (PDF) and choose **Cancel export** within the first two seconds | A message says the export was cancelled and no file was written; the target folder has no new file and no temporary file |
| 13.2 | Start an export, wait for it to finish | File written, name shown |
| 13.3 | Cancel after about 3 to 5 seconds | The export is cancelled before the write and leaves no file, or, if the write already started, the file is complete; never a half written file |
| 13.4 | Cancel an issued export | Nothing is recorded, the numbering is not locked, ⌘Z does not undo anything from it |

## 14. German UI and wording review

Switch the app to German and go through the Measure view, the **Project and report** panel, the
Export view and the report warnings. Compare each term with the user guide in `docs/user/de/`.
The user guide uses "du".

| # | Check | Expected |
|---|---|---|
| 14.1 | The view and the guide | Terms in the app and in `docs/user/de/measuring.md`, `exports.md` and `project-details.md` are the same |
| 14.2 | Teil and Teile | The app and the German reports both say **Teil** and **Teile** for samples (owner decision 2026-10-13): panel, grid, sample bar, messages, column heads, block heading ("Teile 7 bis 12 von 12") |
| 14.3 | Änderungsstand | The panel field and the German reports both say **Änderungsstand** for the drawing revision |
| 14.4 | Ausgegeben and Freigegeben | The export option reads **Als ausgegeben exportieren** (issued) and the signature row **Freigegeben von** (approved by); the two words no longer clash. The warning dialog, the notes under the export and the Measure view hint say "ausgegeben" too |
| 14.5 | Gut/Schlecht-Merkmal | Used in the shortcut cheat sheet for the OK / not OK result type; the column says **i.O. / n.i.O.** |
| 14.6 | Grenze and Grenzmaß | Columns say **Untere Grenze** and **Obere Grenze**; running texts say "Grenzmaße" |
| 14.7 | Refusal messages | Texts such as "Dieses Merkmal hat Ergebnisse der anderen Art. Lösche sie zuerst." read well; the grid messages (not a number, paste does not fit) are complete German sentences |
| 14.8 | Keyboard help | Press **?**: the grid section is German and lists the paste key for your platform |
| 14.9 | Clipping | Nothing is clipped in the grid header, the sample panel or the export warnings |

## 15. User guide

| # | Step | Expected |
|---|---|---|
| 15.1 | Read `docs/user/en/README.md` and follow "Getting started" with the drawing, including steps 10 and 11 | Every step works as written and every label matches the app |
| 15.2 | Read `docs/user/en/measuring.md`, `exports.md` and `project-details.md` | What they say about samples, rules, marks, result types, limits changes, reports and issued reports matches what you saw in sections 1 to 13 |
| 15.3 | Read the German pages | Same, and the German words match the app |
| 15.4 | Look for anything the guide does not say that you needed | List it |

## 16. Exit criterion M3

The timed procedure with a real drawing of your own (at least 25 inspected characteristics and
three parts) is in [docs/perf/M3-exit.md](../perf/M3-exit.md). Run it after this script.

## Result

| Area | Pass | Notes |
|---|---|---|
| 0 Test project | | |
| 1 Samples | | |
| 2 Keyboard entry | | |
| 3 Pass and fail rules | | |
| 4 Result types | | |
| 5 Paste with the real clipboard | | |
| 6 Project and report details | | |
| 7 Check sheet | | |
| 8 Inspection report PDF | | |
| 9 Report dates, identical files | | |
| 10 XLSX in a spreadsheet program | | |
| 11 Issued reports | | |
| 12 When limits change | | |
| 13 Cancel | | |
| 14 German wording | | |
| 15 User guide | | |

Report every failure as an issue with the step number, what happened and the platform.
