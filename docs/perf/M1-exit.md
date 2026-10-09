# Exit criterion M1

> Status: **to measure by the owner.** This page is the procedure and the empty result table.

Exit criterion of milestone 1 (`docs/plan/M1.md`): a real drawing can be ballooned and exported
faster than with a PDF editor and a spreadsheet. The owner measures it with drawings of his choice
and records the result in `docs/plan/STATUS.md`.

Dimo does not read the drawing in this version, so both ways are manual. What Dimo saves is the
second pass: the numbering, the layout of the balloons, the list, the consistency between the
balloons and the list, and the export. The test is whether that adds up to less time.

## What is compared

| | Dimo | Reference |
|---|---|---|
| Place the numbered balloons | Place balloon tool | The PDF editor you use today (stamps, text boxes, shapes) |
| Capture the values | Type the value in the balloon field, fill nominal, deviations and the rest in the table | Type the rows into your spreadsheet |
| Renumber after a change | Automatic | By hand in the drawing and in the sheet |
| Produce the files | Export view: ballooned PDF and CSV or Excel list | Save the PDF from the editor, save the spreadsheet |

Use the tools and templates you would use on a normal working day, including a spreadsheet
template with your usual columns if you have one.

## The finished work must be equal

Both ways end with the same two deliverables, so the times can be compared:

1. A PDF of the drawing with a numbered balloon at every characteristic that gets inspected.
2. A list with one row per balloon and at least these columns: number, requirement text as printed,
   nominal, upper deviation, lower deviation, unit. In Dimo also fill the limits (they are worked
   out for you); in the spreadsheet add them if you normally do.

Decide the list of characteristics before you start the clock (read the drawing, mark what you
will inspect). That reading is the same for both ways and is not timed. Do not leave out the
characteristics you find hard.

## Preparation

1. Pick two real drawings of your own, A and B, of similar size (at least 20 characteristics each).
   Prefer drawings you have not ballooned before. Do not commit them: they may be confidential,
   and `corpus/` takes drawings only with a `PROVENANCE.md` entry (AGENTS.md rule 8). Record each
   by part number, number of sheets and number of characteristics.
2. Use a release build on the machine you work on. From the repository root:

   ```sh
   pnpm -C apps/desktop tauri build --no-bundle
   DIMO_PDFIUM_PATH=$PWD/vendor/pdfium/mac-arm64/lib/libpdfium.dylib \
     target/release/dimo-desktop
   ```

   (Windows: `vendor\pdfium\win-x64\bin\pdfium.dll`.) This build command has not been tried by
   the agent that wrote this page. If it does not work, use `pnpm dev` and write "debug build" in
   the record; a debug build is slower than a release build, which favors the reference.
3. Close other heavy programs. Silence notifications. Have a stopwatch with lap times ready.
4. Cross over the order so that learning does not decide the result: do drawing A with Dimo and
   drawing B with the reference, or the other way round, and say which in the table. If you have a
   third and fourth drawing, do the other combination too.

## Procedure per run

1. Start with the drawing file unopened and the characteristic list decided.
2. Start the stopwatch when you open the drawing in the tool.
3. Take a lap when the **balloons** are all placed, a lap when the **values** are all captured,
   and stop when **both files are saved** in their final place. Dimo: laps at the end of ballooning
   with typed values, at the end of table filling, and when the last export has finished.
   Reference: laps at the end of balloons in the editor, at the end of typing the rows, and at the
   end of saving both files.
4. Do not stop for questions or interruptions; if you must, pause the watch and note why.
5. Check the result against the list: count balloons, compare every number and value of the list
   with the drawing and with the balloons (a number in the wrong place, a typing error, a missing
   characteristic is an error). Count the errors. Checking time is not part of the run time but
   write it down.
6. Write down friction: every moment where you wanted something that Dimo did not offer, and
   anything that crashed, froze or looked wrong.

## Results

Fill one row per run. Times in minutes and seconds.

| Run | Drawing | Characteristics | Way | Order (1st or 2nd) | Balloons | Values | Export or save | Total | Errors found | Notes |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | A | | Dimo | | | | | | | |
| 2 | B | | Reference | | | | | | | |
| 3 | B | | Dimo | | | | | | | |
| 4 | A | | Reference | | | | | | | |

Machine and software:

| Item | Value |
|---|---|
| Date | |
| Machine, operating system | |
| Dimo version and build (release or debug), commit | |
| PDF editor and version | |
| Spreadsheet program and version | |
| Display size and scale | |

## Verdict

The exit criterion is met if, over the runs, Dimo's total time is lower than the reference total
for the same kind of drawing and Dimo's error count is not higher. Write the ratio (Dimo total
divided by reference total) for each pair and the overall verdict:

| Pair | Ratio | Errors Dimo | Errors reference |
|---|---|---|---|
| A and B, first order | | | |
| A and B, crossed | | | |

Verdict: met / not met / met with these conditions: ______

If it is not met, the laps show where the time goes. Typical causes and where to look:

- Balloons lap long: placement flow (`docs/user/en/ballooning.md`), zoom and pan, wrong balloon
  position that you correct afterwards.
- Values lap long: the table has too many columns to fill by hand; consider parsing the typed text
  (M2) before anything else.
- Export lap long: options asked each time, dialogs, file naming.

## What to record in STATUS.md

Put these in `docs/plan/STATUS.md`:

1. In the task table, T1.10: state `done` and in the notes: `Exit criterion measured <date>: Dimo
   <total> versus <total> (ratio <r>), see docs/perf/M1-exit.md`.
2. In the Log, one line: `<date>: M1 exit criterion <met or not met>. Dimo <m:ss>, reference
   <m:ss> on <n> characteristics. Biggest time sinks: <laps>. Friction: <top three>`.
3. In the decision log, any decision that follows from the result (for example "parse typed text
   first in M2", or "reduce table columns for manual entry").
4. In `docs/plan/M1.md`, tick the acceptance criterion "Exit criterion measured by the owner and
   recorded in STATUS.md". With a "met" verdict M1 is done; with "not met" write the follow up
   tasks before starting M2.

Copy the filled tables into this file and change the status line at the top to the date and the
verdict.
