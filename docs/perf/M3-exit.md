# Exit criterion M3

> Status: **to measure by the owner.** This page is the procedure and the empty result table. An
> agent dry run on a synthetic drawing is recorded at the end (no timing claim).

Exit criterion of milestone 3 (`docs/plan/M3.md`): a one person shop goes from customer PDF to a
finished report in one sitting. The owner takes a real drawing of his own with at least 25
inspected characteristics and three parts. Starting with Dimo open and no project, he balloons the
drawing, prints the check sheet, measures the parts (measuring time not counted), enters the
results and exports the inspection report as PDF and XLSX. The criterion is met when:

1. the whole run is done in Dimo without any other program (no spreadsheet, no PDF editor, no
   calculator for limits or pass/fail), in one session without restarting the app;
2. Dimo working time (clock paused while measuring) is at most 60 minutes for 25 characteristics
   and three parts;
3. the report has no error when checked against the drawing and the measured values (number,
   requirement, limits, value, verdict per cell, overall verdict, drawing revision and hash);
4. exporting the same project again gives byte identical PDF and XLSX files.

Unlike [M1](M1-exit.md) there is no reference run: the 60 minutes are an absolute limit, and the
other three conditions are pass or fail checks.

## What counts as another program

Condition 1 is about the work, not about the operating system:

- Not allowed during the run: a spreadsheet, a PDF editor, a calculator (also the one on the
  phone) for limits, deviations or pass/fail, a second tool to number the drawing, a text editor
  for the report.
- Allowed: the system print dialog and the PDF viewer of the operating system **only to print**
  the check sheet. Dimo has no print command in this version, so the exported check sheet PDF is
  opened in the viewer and printed. Write down how long that took (see the laps) and note it in
  the friction table.
- Allowed: the measuring instruments, the paper check sheet and a pen, the Tabellenbuch for
  reading the drawing before the run. Reading the drawing is not timed (see Preparation).
- Allowed outside the clock: everything you use to **check** the result afterwards (condition 3
  and 4), including a calculator, the Tabellenbuch, a spreadsheet program to open the XLSX and
  the terminal for `shasum`.

If you need another program to finish the run, finish it anyway, write down what and why, and
count condition 1 as not met.

## Preparation

1. Pick one real drawing of your own with at least 25 characteristics you would inspect, and three
   parts made to it. Prefer a drawing you have not ballooned in Dimo before. Do not commit the
   drawing, the project or the reports: they may be confidential, and `corpus/` takes drawings
   only with a `PROVENANCE.md` entry (AGENTS.md rule 8). Record it by part number, number of
   sheets, number of inspected characteristics and how many of them are OK / not OK checks
   (threads, notes, surface symbols).
2. Read the drawing before the clock starts and mark what you will inspect, including the hard
   ones. Note the general tolerance of the title block and, for angles without tolerance, the
   length of the shorter leg (Dimo asks for it to apply ISO 2768-1 to angles). This reading is not
   timed. Do not work out any limits on paper.
3. Use a release build on the machine you work on. From the repository root:

   ```sh
   pnpm -C apps/desktop tauri build --no-bundle
   DIMO_PDFIUM_PATH=$PWD/vendor/pdfium/mac-arm64/lib/libpdfium.dylib \
     target/release/dimo-desktop
   ```

   (Windows: `vendor\pdfium\win-x64\bin\pdfium.dll`.) The agent dry run built it this way on an
   M2 MacBook Air in about 10 minutes and the app started. If it does not work, use `pnpm dev` and
   write "debug build" in the record; a debug build is slower, which works against Dimo.
4. Settings (before the clock, they are set once per installation): your company name under
   **Settings > Reports**, your user name, the app language. Leave the report language for the
   Export view.
5. Have the three parts, the measuring instruments, a printer with paper and a stopwatch with lap
   times **and pause** ready (a phone stopwatch pauses with Stop and continues with Start; if
   yours cannot pause, write down the clock time at every pause and resume instead).
6. Close other heavy programs. Silence notifications. Start Dimo and leave it on the start screen
   with no project open.
7. Create an empty folder for the run, for example `~/m3-exit/`, with a subfolder `first/`.

## Procedure

Start the stopwatch when you choose **New project** in Dimo. Do not restart Dimo until the last
export is written (condition 1). Take a lap at the end of each of the four laps below.

### Lap 1: ballooning

1. Create the project from the drawing PDF.
2. Set the general tolerance of the title block: **Settings** (⌘,), section **Tolerances of the
   project**.
3. Balloon every characteristic you marked. Use box select (B and drag around the callout, Enter
   accepts the proposal) where the drawing has text, or click and type the requirement. Enter the
   shorter leg for angles in the proposal card or in the detail panel. Clear **Inspect** for
   reference dimensions if Dimo did not.
4. Check the table: every limit is filled where the drawing gives one; correct what is wrong in
   Dimo, not on paper.
5. Fill **Project and report** in the side panel: part number, part name, customer, order
   reference if you have one, **Revision** (German UI: **Änderungsstand**) as printed in the title
   block, report number, **Report date** with **Today**, inspected by and approved by.
6. Save the project (⌘S) into the run folder.

**Lap** when the project is saved.

### Lap 2: check sheet printed

1. Open the **Measure** view (⌘3), type `3` under **Number** and choose **Add samples** (German
   UI: parts, **Teile**). Change the serials if your parts carry their own.
2. Open the **Export** view (⌘E), choose the report language, **Check sheet**, format **PDF**,
   export into the run folder.
3. Open the exported PDF in the system PDF viewer and print it.

**Lap** when the printed sheet is in your hand. **Pause the clock.**

### Measuring (not timed)

Measure the three parts and write every value and every OK / not OK on the printed check sheet,
one column per part. Do not enter anything in Dimo while the clock is paused, and do not change the
project. If you need Dimo during measuring (for example to look up a limit), resume the clock for
that time.

**Resume the clock** when you sit down at Dimo with the filled check sheet.

### Lap 3: result entry

1. Open the **Measure** view (⌘3).
2. For each part: click the first cell of its column, type each value and press **Enter**; for an
   OK / not OK characteristic press **O** (OK) or **N** (not OK). Set the date of the part in the
   sample panel if you want it on the report.
3. Compare the marks in the grid with the check sheet: a ✗ on the screen that is not a failure on
   the part is a typing error; correct it with **F2**.
4. Save (⌘S).

**Lap** when the last value is entered and the project is saved.

### Lap 4: report exported

1. In the **Export** view choose **Inspection report**, format **PDF**, and export into the run
   folder. Then format **Excel** and export again.
2. If the report goes to a customer, tick **Export as issued** (German UI: **Als ausgegeben
   exportieren**) before both exports and confirm the draft warning with **Export anyway**. The
   draft warning is expected, because all tolerance tables are still drafts (D-43). Say in the
   record whether you exported as issued.

**Stop the clock** when the Excel file is written. The total of the four laps is the Dimo working
time (condition 2).

### Re-export (not timed, condition 4)

Without changing anything in the project:

1. Move the two report files into `first/` (Finder or terminal).
2. Export the inspection report again as PDF and as Excel into the run folder, with the same
   options as before (also **Export as issued** if you used it).
3. Compare in the terminal:

   ```sh
   cd ~/m3-exit
   shasum -a 256 first/*_inspection-report.* *_inspection-report.*
   cmp first/*_inspection-report.pdf *_inspection-report.pdf && echo PDF identical
   cmp first/*_inspection-report.xlsx *_inspection-report.xlsx && echo XLSX identical
   ```

   Each pair must have the same SHA-256. Copy the four hashes into the result table.

Condition 4 holds when no report date is set too, but only on the same calendar day: without a
report date the report prints the date of the last change of the project, and an issued export is
a change. Setting the report date with **Today** in lap 1 keeps the files identical on any later
day.

## Checking the report (not timed, condition 3)

Check the PDF line by line against the drawing and the filled check sheet. Use the Tabellenbuch and
a calculator here if you like; checking time is not part of the run time but write it down. Every
item that is wrong, missing or extra counts as one error.

Header and first page:

- [ ] Company, part number and name, customer, order reference as entered.
- [ ] Revision (**Änderungsstand** in German) equals the title block of the drawing.
- [ ] SHA-256 in full on the first page equals `shasum -a 256 <drawing>.pdf`; the short form in
      the header of every page is its start.
- [ ] Report number and date as entered; "Page n of m" on every page.

Summary and parts:

- [ ] Number of characteristics equals the inspected characteristics you marked; reference
      dimensions are not listed.
- [ ] Number of samples (parts, **Teile**) is 3, with your serials.
- [ ] Counts of pass, fail, not evaluated and missing add up to characteristics times 3, and match
      your own count from the check sheet.
- [ ] Verdict of each part: fail if any of its results fails, pass if all pass, open otherwise.
- [ ] Overall verdict follows the same rule over all parts.

Table, for every row:

- [ ] The number equals the balloon on the drawing at the right callout (compare with the ballooned
      drawing on screen or export it).
- [ ] Requirement text as printed on the drawing.
- [ ] Nominal, lower and upper limit and unit are right for the requirement (work out fits and
      general tolerances with the Tabellenbuch). Limits from a table carry the asterisk; that is
      expected while the tables are drafts.
- [ ] Every value equals the check sheet, with the digits you wrote.
- [ ] Verdict mark per cell: pass inside or on a limit, fail outside, OK and not OK as written.
- [ ] Row verdict in the last column.

End of the report:

- [ ] Draft footnote present if any asterisk is printed; legend of the marks.
- [ ] Signature block: inspected by with the report date, approved by with an empty date.
- [ ] Remarks as entered.

Excel file: open it in a spreadsheet program **after** the run and spot check at least five rows,
the summary and the part verdicts against the PDF. Any difference counts as an error.

## Results

Times in minutes and seconds.

| Lap | Time | Notes |
|---|---|---|
| 1 Ballooning (including project details, saved) | | |
| 2 Check sheet printed (samples, export, print) | | |
| 3 Result entry (saved) | | |
| 4 Report exported (PDF and XLSX) | | |
| **Dimo working time (sum)** | | |
| Measuring (paused, for information) | | |
| Checking the report (for information) | | |

| Item | Value |
|---|---|
| Drawing (part number, not committed) | |
| Sheets | |
| Inspected characteristics (of them OK / not OK) | |
| Parts | 3 |
| Box select / typed / corrected after proposal | |
| Exported as issued | yes / no |
| Errors found in the report (condition 3) | |
| SHA-256 report PDF, first and second export | |
| SHA-256 report XLSX, first and second export | |

Machine and software:

| Item | Value |
|---|---|
| Date | |
| Machine, operating system | |
| Dimo version and build (release or debug), commit | |
| Display size and scale | |
| Printer, PDF viewer used to print | |
| Spreadsheet program used to check the XLSX | |

Conditions:

| Condition | Met | Notes |
|---|---|---|
| 1 Only Dimo, one session without restart | | |
| 2 Working time at most 60 minutes | | |
| 3 No error in the report | | |
| 4 Byte identical PDF and XLSX on re-export | | |

Friction: every moment where you wanted something that Dimo did not offer, and anything that
crashed, froze, looked wrong or made you reach for another program. One row each.

| # | Lap | What happened | Time lost (estimate) | Suggestion |
|---|---|---|---|---|
| 1 | | | | |
| 2 | | | | |
| 3 | | | | |

## Verdict

The exit criterion is met if all four conditions are met. Write the working time per
characteristic as well (working time divided by the number of inspected characteristics), so
drawings of other sizes can be compared later.

Verdict: met / not met / met with these conditions: ______

If it is not met, the laps show where the time goes. Typical causes and where to look:

- Lap 1 long: proposals that need correction (`docs/user/en/ballooning.md`, tolerances), angles
  without a shorter leg, project details typed late.
- Lap 2 long: printing from outside Dimo, sample setup, export options.
- Lap 3 long: keyboard flow in the grid (`docs/user/en/measuring.md`), typing errors found late,
  result types to change.
- Lap 4 long: options asked each time, warnings, file naming.
- Condition 3 failed: say which column was wrong and file it with the row number.
- Condition 4 failed: keep both files and attach them to the issue (they show what differs).

## What to record in STATUS.md

Put these in `docs/plan/STATUS.md`:

1. In the task table, T3.11: state `done` and in the notes: `Exit criterion measured <date>:
   <working time> for <n> characteristics and 3 parts, conditions 1 to 4 <met or which failed>,
   see docs/perf/M3-exit.md`.
2. In the Log, one line: `<date>: M3 exit criterion <met or not met>. <m:ss> working time on <n>
   characteristics, 3 parts (<s> per characteristic). Longest lap: <lap>. Errors: <count>.
   Re-export identical: <yes/no>. Friction: <top three>`.
3. In the decision log, any decision that follows from the result (for example "print command in
   the Export view", or "fill the report date by default").
4. In `docs/plan/M3.md`, tick the acceptance criterion "Owner ran the exit criterion and recorded
   the result in STATUS.md". With a "met" verdict M3 is done and release 0.1 can be prepared; with
   "not met" write the follow up tasks first.

Copy the filled tables into this file and change the status line at the top to the date and the
verdict.

## Agent dry run (2026-10-13)

An agent ran the procedure end to end on a synthetic drawing to check that every step works and
that the checks can be done as written. It drove the real debug build with the development hooks
(`apps/desktop/src-tauri/src/dev.rs`) and a UI script of pointer and key events. Agent speed is
not user speed, so **no timing is claimed**, and nothing here replaces the owner's run.

### Setup

| Item | Value |
|---|---|
| Machine | MacBook Air M2, macOS 27.0.1 |
| Build | debug, `main` at `a922bd9` |
| Drawing | `dimo-synth --seed 7 --count 15` (A4 landscape, 15 callouts; the generator allows at most 15), SHA-256 `65d6ba8c…a22d1` |
| Characteristics | 27 balloons, 25 inspected: the 15 callouts by box select (two of them reference dimensions, so Inspect cleared by Dimo) plus 12 typed after a click at free places (`Ø6 H7`, `20 ±0.1`, `Ø25 h6`, `15 +0.2 -0`, `8 ±0.05`, `Deburr all edges`, `Ra 1.6`, `60`, `Ø12 ±0.02`, `2x45°`, `M8`, `100 ±0.2`) |
| Result types | 21 values, 4 OK / not OK (two threads, the deburr note, the surface symbol) |
| Settings | company name and user name in a throwaway settings folder; general tolerance ISO 2768-1 class m |
| Parts | 3, invented values; expected: part 1 pass, part 2 fails at 14 (`39.974` below `39.975`), part 3 fails at 2 (`40.41` above `40.40`) and at 21 (not OK) |

Replaced by hooks, because a script cannot drive native dialogs: the open dialog of **New
project** (`DIMO_DEV_OPEN`), the save dialogs (`DIMO_DEV_SAVE_AS`, `DIMO_DEV_EXPORT_DIR`). Not
done: printing (no printer step; the check sheet PDF was rendered and looked at instead) and
measuring.

### What was done and what worked

All in one app session, without restart:

1. Settings: general tolerance ISO 2768-1 m. Box select of the 15 callouts: every proposal had
   the expected limits and rule (explicit, general, fit; threads and reference dimensions without
   tolerance). The angle `45°` had no limits until the shorter leg `50` was entered in the
   proposal card; then 45.5 / 44.5 (ISO 2768-1 m, ±0°30'). The two reference dimensions came in
   with **Inspect** cleared.
2. Twelve characteristics typed right after the click, Enter after each.
3. Project and report: part number, part name, customer, revision `B`, report number, **Today**,
   approved by; company name and inspector came from the settings. Save as.
4. Measure view: three samples added. Export view: language German, check sheet PDF exported. Two
   pages, 25 rows in number order, the reference dimensions left out, one empty column per part
   with the serial, header with **Änderungsstand** `B`, full SHA-256, draft asterisks and footnote,
   "Geprüft von" line.
5. Result entry with the keyboard only, one column per part: 25 entries each, `o` and `n` without
   Enter, one value typed with a comma (`22,2`). The verdicts in the app after each part:
   all pass; then fail at 14; then fail at 2 and 21. Overall fail. Every keystroke result was as
   expected (75 results, commit to repaint 24.5 ms mean in the debug build).
6. Inspection report exported as PDF and as Excel (German), then exported again, then as issued
   (PDF twice, then Excel), confirming the draft warning with **Export anyway**. Saved.
7. The whole run was repeated in a second app process from a new project (the first try had left
   the report date empty, see finding 1). Both processes gave the same files.

The report was checked with the checklist above against the truth file of the synthetic drawing
and the invented values: no error. Summary 25 characteristics, 3 parts, 72 pass, 3 fail, 0 not
evaluated, 0 missing, overall **n.i.O.**; parts **i.O.**, **n.i.O.**, **n.i.O.** with counts
25/0, 24/1, 23/2; failing cells boxed with ✗ at 2/part 3, 14/part 2 and 21/part 3; the row
verdicts ✗ in rows 2, 14 and 21; values with the typed digits and a decimal comma (`39,974`,
`6,000`); revision `B`; the full SHA-256 equals `shasum -a 256` of the drawing; signature block
with "Geprüft von Agent 2026-10-13" and "Freigegeben von Owner" without date. The XLSX holds the
same values as number cells, the same verdicts, counts per part and footnote. The project records
three issued reports whose `output_sha256` equal the exported files.

Hashes (`shasum -a 256`), identical for every export in both app processes, normal and issued:

| File | SHA-256 |
|---|---|
| `final_check-sheet.pdf` (German) | `bb866d64c3c355c895d44a6d959ea7b82fc53624e281bbdf09359a04cf90577d` |
| `final_inspection-report.pdf` (German) | `6bcaa587e818e02b8960a50b5b1716dc287057ee47a9bce80cc34af65258f067` |
| `final_inspection-report.xlsx` (German) | `d740cf3c2060ed2008e0e18cb5f211d85e038324fdc2223f999438da41977d0e` |

In the second process the report PDF was written four times and the XLSX three times (new file
times each time); every copy has the hash above. Conditions 3 and 4 hold for this dry run; conditions 1 and 2
need the owner.

### Findings

No bug in the app was found. Friction and points for the owner:

1. **Report date left empty is easy to miss.** Nothing asks for it; without it the report prints
   the date of the last change of the project. Re-export stayed identical in the dry run because
   all exports were on the same day. The procedure therefore sets it with **Today** in lap 1.
   Possible follow up: fill the report date at the first report export, or show the date that will
   be printed next to the empty field.
2. **No print command.** The check sheet has to be opened in the system PDF viewer to print it.
   The procedure allows this as "printing only" under condition 1; the owner decides whether that
   is acceptable or whether M3 needs a print or "open after export" button.
3. **Angles need the shorter leg.** An angle without tolerance gets no limits until the shorter
   leg is entered, and then shows as OK / not OK if you skip it. The owner should note the legs
   while reading the drawing (Preparation, step 2).
4. **`Ra 1.6` typed as text becomes kind `other`,** not a surface texture, with OK / not OK. Fine
   for the report; noted in case the owner expects a surface kind.
5. **Empty order reference** still prints the label "Auftrag" with no value in the PDF header and
   in the XLSX header block. Cosmetic.
6. **Legend differs slightly:** the PDF legend lists "Offen", the XLSX legend does not (open cells
   show `-` in the XLSX). Cosmetic.
7. Development hook only, not a user problem: the UI script's `fill` step finds a field only
   inside its `<label>`; the report date field uses `<label for>`, so the script used **Today**.
