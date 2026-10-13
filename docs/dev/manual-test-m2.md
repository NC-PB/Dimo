# Manual test script, milestone 2

For what `scripts/e2e-smoke.sh` cannot reach: the look of the proposal card and the markers, the
feel of box select on a real drawing, native dialogs for the custom table import, and the user
guide in both languages ([testing.md](testing.md)). It follows the same idea as
[manual-test-m1.md](manual-test-m1.md) and takes about 60 minutes. Run it before the M2 sign off
and after changes to box select, the tolerance engine, numbering or the side panel. Record the
result in the table at the end and put the notes in the pull request or in `docs/plan/STATUS.md`.

All tolerance tables are still drafts (D-43). Every limit derived from a table therefore carries
the **Draft** marker in this script; that is expected.

## Before you start

1. Run `./scripts/check.sh` and `./scripts/e2e-smoke.sh`. Both must pass first.
2. Start the app with `pnpm dev` (a debug build) or from an installed build. Note which.
3. Drawings:
   - **A**: `corpus/drawings/test_drawing_1.pdf` (24 callouts, no general tolerance declared).
   - **B**: a synthetic drawing, made with
     `cargo run -p dimo-synth -- --seed 7 --count 6`. It writes `target/synth/synth_7.pdf` and
     the truth file `target/synth/synth_7.truth.json` (general tolerance ISO 2768-1 m, a thread,
     a limit dimension, a symmetric tolerance, a radius, two reference dimensions, one of them
     rotated by 90 degrees).
   - **C**: one real drawing of your own with vector text (for the exit criterion in section 11).
4. A custom table file for section 5. Save this text as `shop-table.toml` in a folder you can
   delete afterwards:

   ```toml
   [table]
   id = "shop-table"
   title = "Shop general tolerances"
   kind = "custom"
   unit = "mm"
   version = 1
   status = "draft"
   source = "Example shop standard"

   [[part]]
   id = "linear"
   source = "example"
   kind = "symmetric"
   applies_to = "linear"
   value_unit = "mm"
   columns = ["a"]
   rows = [
     { min_inclusive = "0.5", max_inclusive = "100", values = ["0.02"] },
   ]
   ```

5. Put the files you create in a folder you can delete afterwards.

Write down: date, tester, operating system and version, Dimo version and build, display scale.

## 1. Box select on drawing A (FR-REC-01, FR-REC-02)

Create a new project from drawing A. Press **B** (Place balloon). Drag a box tightly around each
text below, look at the card, then press **Enter**.

| # | Text on the drawing | Expected on the card |
|---|---|---|
| 1.1 | `Ø30 H7 +0.0203 -0` (stacked) | Requirement `Ø30 H7 +0.0203 -0`, kind Diameter, nominal 30 mm, fit H7, limits 30.0203 and 30, rule **Explicit (on the drawing)**, a triangle marker, and a note that the printed deviations differ from the fit table |
| 1.2 | `Ø8 f7` | Limits 7.987 and 7.972, rule **Fit table**, marker **Draft** |
| 1.3 | `Ø8 f7` with printed `-0.0127 -0.0279` (stacked below) | Limits 7.9873 and 7.9721, rule Explicit, a note about the fit table |
| 1.4 | `100` with `+0` and `−0.6` | Limits 100 and 99.4, rule Explicit |
| 1.5 | `12.39±0.1` | Limits 12.49 and 12.29, rule Explicit |
| 1.6 | `R15 H7` | Kind Radius, limits 15.018 and 15.000, rule Fit table |
| 1.7 | `50` | Rule **No tolerance defined**, no limits, a circle with a bar |
| 1.8 | `BREAK ALL SHARP EDGES AND REMOVE BURRS` | The card says the text is not read as a dimension; accepting stores the text only, kind Note |

After each accept:

| # | Step | Expected |
|---|---|---|
| 1.9 | Look at the drawing | The balloon sits up and right of the box, the leader points at the box corner, the new characteristic is selected and its row is shown |
| 1.10 | Look at the table | The row has the values of the card, **Rule** shows the same rule and markers |
| 1.11 | Press ⌘Z once | The characteristic and its balloon are gone, all in one step |
| 1.12 | Press ⇧⌘Z | They are back |

## 2. The proposal card (FR-REC-01, ADR 0006)

| # | Step | Expected |
|---|---|---|
| 2.1 | Drag a box around `Ø8 c10`, do not press anything | The card opens next to the box, the box is drawn dashed, the cursor is in the requirement field with the text selected |
| 2.2 | Press **Escape** | The card closes, nothing is added, the undo button is unchanged |
| 2.3 | Drag the box again, then click **Discard** | Same as 2.2 |
| 2.4 | Drag the box again, change the requirement text to `Ø8 h6` and press Tab | The card reads it again: nominal 8, limits 8.000 and 7.991, rule Fit table, explanation changed |
| 2.5 | Change the kind in the list, then press **Enter** | One characteristic is added with the text `Ø8 h6` and the kind you chose |
| 2.6 | Drag a box, then drag a second box somewhere else | The first card closes without adding anything, the second opens |
| 2.7 | Drag a box and, without waiting, type `Ø9 H7` and press **Enter** | The typed text replaces the text of the card and is accepted; no key is lost. Repeat ten times quickly |
| 2.8 | Drag a box and immediately press **Escape** | The card does not stay open |
| 2.9 | Drag one box around the three stacked lines `Ø30 H7`, `+0.0203`, `-0` | One proposal (stacked tolerances are joined) |
| 2.10 | Drag one box around two dimensions above each other on the drawing, for example `31.05` and `52.61` | A card with two proposals, one for each, and the text "2 proposals". **Enter** adds both, one ⌘Z removes both |
| 2.11 | Drag a box over an empty part of the sheet | No card; a balloon is placed and its value field opens, as with a click |
| 2.12 | Drag a box with the Select tool (**V**) | No card; it selects balloons (Shift) or moves the drawing |
| 2.13 | Drag a box around text while the sheet is rotated (R) | The card appears next to the box on the screen, its content is the same |
| 2.14 | Read the card with a screen reader or the keyboard only | Fields can be reached with Tab, Enter accepts, Escape discards, the notes are read |

## 3. Typed callouts and type-ahead (FR-BAL-02, M2 decision 5)

| # | Step | Expected |
|---|---|---|
| 3.1 | Press **B**, click a feature, type `Ø30 H7 +0.0203 -0` without pause and press **Enter** | Same values as 1.1: limits 30.0203 and 30, rule Explicit |
| 3.2 | Click, type `Ø8 f7`, Enter | Limits 7.987 and 7.972, rule Fit table |
| 3.3 | Click, type `12,5±0,1` (comma), Enter | Nominal 12.5, limits 12.6 and 12.4, rule Explicit |
| 3.4 | Click, type `(42)`, Enter | Reference dimension, no limits, **Inspect** cleared, marker **(Ref)** |
| 3.5 | Click, type `[25]`, Enter | Basic dimension, **Basic** marker, **Inspect** cleared |
| 3.6 | Click, type `4X Ø6.6 THRU`, Enter | Quantity 4, kind Diameter |
| 3.7 | Click, type `.250`, Enter (no general tolerance yet) | Inch notation: nominal 0.250, unit in, no limits |
| 3.8 | Click, type `hello there`, Enter | Stored as requirement text only, kind stays Other, no limits |
| 3.9 | Click, press **Escape** at once | The field closes, the balloon stays, ⌘Z removes it |
| 3.10 | Place 30 balloons quickly with click, type, Enter | No character is lost, no lag, focus returns to the drawing after each Enter |
| 3.11 | Select balloon 1, press Enter, retype `Ø8 h6`, Enter | Values are read again and replace the old ones |
| 3.12 | Edit the **Requirement** cell in the table | Only the text changes; the limits stay until you re-interpret (section 5) |

## 4. Tolerance settings and rules (FR-TOL-01, FR-TOL-02, FR-TOL-06, FR-TOL-09)

Use a new project from drawing A with the characteristics 1.7 (`50`), a `31.05` and a `30`
placed by box select, so they have no tolerance.

| # | Step | Expected |
|---|---|---|
| 4.1 | Open the **Drawing** tab (the project settings left the Settings view in M3, ADR 0007) | The group **Tolerances** shows **General tolerance**, **Decimal rules…** and **Company tables**; the language of the labels follows the app |
| 4.2 | Set **General tolerance** to ISO 2768-1 and class m | A **Draft** marker appears next to the table |
| 4.3 | Go back to the drawing (⌘1) | The limits of the three characteristics are unchanged (no automatic update) |
| 4.4 | Select the three rows and choose **Update limits** (Home tab) | The line below says "Limits updated: 3". `50` gets 50.3 and 49.7, `31.05` gets 31.35 and 30.75, `30` gets 30.2 and 29.8, rule **General tolerance**, marker Draft |
| 4.5 | Press ⌘Z once | All three are back to no limits in one step |
| 4.6 | Press ⇧⌘Z, then type limits by hand into one of them (`50.1` upper, `49.9` lower) | The rule becomes **Entered by hand** |
| 4.7 | Select all three and re-interpret again | "Kept because entered by hand: 1" |
| 4.8 | Set **General tolerance** to None, add a decimal place rule: places 2, tolerance 0.05 | The rule is listed; adding the same number of places again is refused |
| 4.9 | Select `31.05` and `30` (not the hand edited one) and re-interpret | `31.05` gets 31.10 and 31.00, rule **Decimal places rule**; `30` has no decimals, so it loses its limits and gets **No tolerance defined** |
| 4.10 | Set the general tolerance back to ISO 2768-1 m and re-interpret `31.05` | The general tolerance wins over the decimal rule: 31.35 and 30.75 |
| 4.11 | Type `.250` into a new balloon | Nominal 0.250 in, limits 0.2578 and 0.2422; the explanation says the values were converted from mm to in and rounded to 4 decimal places |
| 4.12 | Change **Decimal places in inch** to 3, re-interpret that row | Limits 0.257 and 0.243 |
| 4.13 | Enter `x` in a rounding field | A message says to enter a whole number from 0 to 10; nothing changes |
| 4.14 | Box select `Ø30 H7 +0.0203 -0` and `Ø8 f7` with the general tolerance set | The fit and the printed limits still win; the general tolerance is not used |
| 4.15 | Save, close, open the project | General tolerance, decimal rule and rounding are as you left them |
| 4.16 | Box select the angle `90.0°` with ISO 2768-1 m set | The card shows a field **Shorter leg (mm)** and a note that the shorter leg is unknown; no limits |
| 4.17 | Type `40` into **Shorter leg** and leave the field | Limits 90.5 and 89.5 (±0.5° for a leg over 10 up to 50 mm, class m); the explanation names the leg 40 mm, its range and the limits. Compare with part `angular` of `data/tolerances/iso-2768-1.toml` (draft, D-43) |
| 4.18 | Accept, set the general class to c and re-interpret the angle | Limits 91.0 and 89.0 (±1°, class c for the same range); the side panel still shows the leg 40 |
| 4.19 | In the side panel type `abc` into **Shorter leg**, then empty the field | `abc` is refused with a message and nothing changes; the empty field removes the leg, and the angle has no limits again |
| 4.20 | Set the sheet unit to inch, select the angle and retype its requirement text `90.0°` | The leg stays 40 mm (shown beside the field) and the limits stay; set the unit back to mm |

## 5. Explanations, markers, history and custom tables (FR-TOL-07, FR-TOL-08, FR-CHR-08, FR-CHR-10)

| # | Step | Expected |
|---|---|---|
| 5.1 | Select a characteristic with rule Fit table | The side panel shows **Characteristic N**, tab **Tolerance**: rule, limits with unit, explanation naming the table, the fit, the size range and the deviations |
| 5.2 | Switch the app language to German and look at the same panel | The explanation, rule names and markers are German with decimal comma; switch back |
| 5.3 | Look at the markers in table and panel for each case: Draft, no tolerance, hint, reference, basic | Each has its own shape (dashed box, circle with bar, triangle, rounded box, square box); none relies on color alone. Point at the triangle: the note appears |
| 5.4 | Tick and clear **Inspect this characteristic** for a reference dimension | The **Inspect** cell in the table follows; re-interpreting keeps your choice |
| 5.5 | Open the tab **History** for a characteristic you edited and accepted | Newest first; each entry has time, user and source: **By hand** for your edits, **Recognition** for the accepted box select, **By rule** after a re-interpretation or renumbering; undo and redo appear as "(Undone)" and "(Redone)" |
| 5.6 | Use the arrow keys on the tab buttons | The tab changes |
| 5.7 | In **Company tables** (Drawing tab) choose **Import table…** and cancel the dialog | Nothing changes, no message |
| 5.8 | Import `shop-table.toml` | A native dialog opens; afterwards "Imported shop-table v1. It is saved with the project." and the table is listed under **Custom tables** with the Draft marker |
| 5.9 | Set **Drawing rule (custom table)** to Shop general tolerances, class a, and re-interpret a characteristic without hand entered limits, for example `30` | Limits 30.02 and 29.98, rule **Drawing rule**; with ISO 2768-1 m still set as general tolerance the drawing rule wins |
| 5.10 | Import a copy of the file in which `version = 1` is changed to `version = one` | The file is refused with its name and "line 6"; the project is unchanged |
| 5.11 | Import the first file again | It replaces the old table; the settings keep working |
| 5.12 | After 5.8, with a project that was never saved, force quit Dimo (kill the process), then start Dimo | The unsaved project is restored with a note; the custom table and the setting that uses it are still there |
| 5.13 | Save, move the `.dimo` file to another computer or folder without the table file, open it | The custom table is part of the project; the limits are the same |
| 5.14 | Try to import a copy of the file with the id changed to `iso-286` | Refused: shipped tables cannot be replaced |

## 6. Drawing B: synthetic drawing, rotated text (FR-REC-02)

Create a new project from `target/synth/synth_7.pdf`. In **General tolerance** (Home or Drawing tab) set ISO 2768-1 class m (the
drawing declares it). Box select each callout:

| # | Text | Expected |
|---|---|---|
| 6.1 | `M12x1,75-6g` | Kind Thread, nominal 12, no limits |
| 6.2 | `39.4 +1.00 0` | Limits 40.40 and 39.4, rule Explicit |
| 6.3 | `48.3±0.10` | Limits 48.40 and 48.20, rule Explicit |
| 6.4 | `R22,3` | Kind Radius, limits 23.3 and 21.3, rule General tolerance |
| 6.5 | `(212.3)`, written vertically | Read correctly in its own direction, Reference, **Inspect** cleared, no limits |
| 6.6 | `(141.9)` | Reference dimension as 6.5 |
| 6.7 | Compare every result with `target/synth/synth_7.truth.json` | Same kind, nominal and limits |

## 7. Numbering strategies (FR-BAL-04, FR-BAL-05, D-21)

Use drawing A with at least 12 characteristics placed in no particular order.

| # | Step | Expected |
|---|---|---|
| 7.1 | Look at the **Numbering** panel in a new project | Strategy "Sheet, zone, reading order" |
| 7.2 | Pick **By type**, tick **Preview new numbers** | Dashed ghost numbers appear next to the balloons and as an arrow and number in the **No** column; changed numbers are bold and colored, unchanged ones italic and grey; the panel counts the changes |
| 7.3 | Place or delete a balloon with the preview on | The ghosts and the count follow at once |
| 7.4 | Choose **Apply numbering** | The table is sorted, the balloons carry the new numbers, the preview is off; one ⌘Z restores everything |
| 7.5 | Compare the applied numbers with the preview | Identical |
| 7.6 | Try each strategy once with the preview | **Manual** shows no change; the others reorder as described in the user guide |
| 7.7 | Open an older project (made before M2) | Strategy **Manual**, order unchanged |
| 7.8 | Save, close, open | The strategy is kept |

### Zone grid and views

| # | Step | Expected |
|---|---|---|
| 7.9 | In **Sheet properties**, **Zone grid**, choose **Add zone grid** | A dashed grid 10 mm inside the sheet edge with 8 columns (1 to 8) and 6 rows (A to F), labels on the sheet |
| 7.10 | Choose **Draw frame** and drag along the inner edge of the drawing frame | The grid follows the rectangle; Esc during the drag cancels |
| 7.11 | Change columns to 4 and the column labels to **A, B, C from the right** | Labels follow; type `X, Y, Z` into the label field: the count of labels sets the columns |
| 7.12 | Apply **Sheet, zone, reading order** with the preview | Numbers go zone by zone, rows of zones top to bottom, left to right inside a row |
| 7.13 | **Remove zone grid** | The grid is gone; the preview reads the whole sheet in reading order |
| 7.14 | **Draw view** around two views, name them, apply **Per view, clockwise** with the preview | Views in the order drawn, clockwise from 12 o'clock around each view center, balloons outside all views last |
| 7.15 | Remove a view with ×, undo with ⌘Z | The view returns |
| 7.16 | Rotate the sheet (R) and preview again | Numbers follow the sheet as shown on screen |
| 7.17 | Save, close, open | Zone grid and views are kept |

## 8. Repeated features and locked inserts (FR-BAL-07, FR-BAL-11, D-22, D-23)

| # | Step | Expected |
|---|---|---|
| 8.1 | Box select any dimension, change the requirement text on the card to `4X Ø6.6 THRU`, press Tab and Enter, with **Repeated features (4X)** on **One balloon with quantity** | One characteristic with quantity 4 (column **Qty**), one number |
| 8.2 | Set **Repeated features (4X)** to **Sub-numbers (5.1, 5.2)** and do the same again | Four characteristics numbered like 7.1 to 7.4, quantity 1 each, with stacked balloons; one ⌘Z removes all four |
| 8.3 | Type `4X Ø6.6` into a balloon with sub-numbers on | One characteristic with quantity 4 (typed values are not split) |
| 8.4 | Edit the requirement of one copy | It gets a plain number of its own |
| 8.5 | Switch back to **One balloon with quantity** | The copies get plain numbers, they stay separate |
| 8.6 | Export as issued (**Export** view, tick **Export as issued**, export the CSV) | Numbering is locked; the **Numbering** panel says so and hides strategy and **Apply numbering** |
| 8.7 | Set **Added while locked** to **Next free number**, place a balloon | Highest number ever used plus one |
| 8.8 | Set it to **Sub-number (12.1)**, select balloon 3 and place a balloon | The new one is 3.1 and it appears in the table right after 3; place another: 3.2 |
| 8.9 | Set it to **Letter suffix (12A)**, select balloon 3, place a balloon | 3A, then 3B |
| 8.10 | Select a balloon and box select a dimension | The accepted characteristic follows the selected one by the same policy |
| 8.11 | Delete a sub-numbered characteristic, then add another after the same balloon | The deleted number is not given again |
| 8.12 | Press ⌘Z right after the issued export | The lock is undone; **Apply numbering** is offered again |

## 9. Exports (FR-EXP-09)

| # | Step | Expected |
|---|---|---|
| 9.1 | Export the CSV and open it in a spreadsheet program | 21 columns; the column **Rule** follows **Fit**; values such as `explicit`, `fit`, `general`, `manual`; empty if no rule |
| 9.2 | Export with German headers | The column is called **Regel**, values stay English |
| 9.3 | Export the Excel file | Same values; sub-numbers such as `3.1` and `3A` are text, plain numbers are number cells |
| 9.4 | Export twice from the same project | Byte identical files |
| 9.5 | Reference and basic dimensions | Present with `Inspect` = `no` |

## 10. Shortcuts, language and help

| # | Step | Expected |
|---|---|---|
| 10.1 | Press **?** and read the list | It matches `docs/user/en/shortcuts.md`; Enter and Escape for the card are listed in the user guide section "Box select card and typed values" |
| 10.2 | Switch to German and go through sections 1 to 8 quickly | All texts are German, nothing is clipped; the explanation of the same characteristic is German |
| 10.3 | Read `docs/user/en/README.md` and follow its "Getting started" with a drawing | Every step works as written, every label matches the app |
| 10.4 | Read the pages for ballooning, tolerances and numbering in German | Terms match the app labels |

## 11. Exit criterion M2 on a real drawing

M2 asks for correct limits for 99 percent of common callouts on a vector PDF. The evaluation
tests (`cargo run -p dimo-cli -- eval box-select`) measure this on the corpus and synthetic
drawings. This section is the confirmation on a real drawing of your choice (drawing C).

1. Set the general tolerance of the drawing in **General tolerance** (Home or Drawing tab).
2. Box select every dimension of the drawing with the Place balloon tool and press **Enter** (fix
   the text on the card where needed, and count that as a miss).
3. Compare the limits of every characteristic with the printed values and your standard tables.
   Only count linear, diameter, radius, chamfer and angle dimensions, symmetric, asymmetric,
   one sided and limit tolerances, fits, metric threads and reference dimensions (see M2.md,
   T2.9).

| Item | Value |
|---|---|
| Drawing (part number, sheets) | |
| Callouts tried | |
| Correct without editing | |
| Wrong or no limits (list them) | |
| Share correct | |
| Time for the whole drawing | |

Record the result in `docs/plan/STATUS.md` under T2.9. The criterion is met at 99 percent or more.

## Result

| Area | Pass | Notes |
|---|---|---|
| 1 Box select on drawing A | | |
| 2 Proposal card | | |
| 3 Typed callouts and type-ahead | | |
| 4 Tolerance settings and rules | | |
| 5 Explanations, markers, history, custom tables | | |
| 6 Synthetic drawing, rotated text | | |
| 7 Numbering, zones and views | | |
| 8 Repeated features and locked inserts | | |
| 9 Exports | | |
| 10 Shortcuts, language, help | | |
| 11 Exit criterion on a real drawing | | |

Report every failure as an issue with the step number, what happened and the platform.
