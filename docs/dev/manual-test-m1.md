# Manual test script, milestone 1

For what `scripts/e2e-smoke.sh` cannot reach: native dialogs, window events, the feel of gestures,
and how files look in other programs ([testing.md](testing.md)). It takes about 30 minutes.
Run it before a release and after changes to dialogs, window events, viewport gestures or the
exports. Record the result in the table at the end and put the notes in the pull request or in
`docs/plan/STATUS.md`.

## Before you start

1. Run `./scripts/check.sh` and `./scripts/e2e-smoke.sh`. Both must pass first.
2. Start the app with `pnpm dev` (a debug build) or from an installed build. Note which.
3. Use `corpus/drawings/test_drawing_1.pdf`. For the multi sheet and large cases create the stress
   PDF with `cargo run -p dimo-synth -- --stress --seed 1 --sheets 50 --out target/perf`.
4. Put the files you create in a folder you can delete afterwards.

Write down: date, tester, operating system and version, Dimo version and build, display scale.

## 1. File dialogs (FR-DOC-07, FR-EXP-01)

| # | Step | Expected |
|---|---|---|
| 1.1 | Choose **New project** | A file dialog opens that offers PDF files |
| 1.2 | Cancel the dialog | Nothing changes, no error message |
| 1.3 | Choose a text file or a folder if the dialog allows it | No project is made; if a message appears it is understandable |
| 1.4 | Choose the corpus drawing | The sheet is shown fitted; the toolbar says "Untitled" |
| 1.5 | Press ⌘S (Ctrl+S) | A save dialog opens with the drawing name and `.dimo` suggested |
| 1.6 | Type a name without extension and save | The file is `<name>.dimo`; the toolbar shows the name and "Saved" |
| 1.7 | Choose **Save as** and save under a second name | Both files exist; the toolbar shows the second name |
| 1.8 | Choose **Open project**, cancel, then open the first file | Cancel changes nothing; the file opens with the same characteristics |
| 1.9 | In the Export view start each of the three exports and cancel the dialog | No file, no error, the export row shows no result |
| 1.10 | Start each export and save | Suggested names are `<project>_ballooned.pdf`, `<project>_characteristics.csv`, `<project>_characteristics.xlsx`; the saved file name appears under the export |
| 1.11 | Try to save an export into a read only folder | The export row shows a failure with a reason; the project is unchanged |

## 2. Window close and quit (NFR-REL-01)

| # | Step | Expected |
|---|---|---|
| 2.1 | Place a balloon, then close the window | A question "Save changes?" with Save, Don't save, Cancel |
| 2.2 | Choose Cancel | The window stays, nothing is lost |
| 2.3 | Close again and choose Save | The project is saved (file dialog if it has no file), the window closes |
| 2.4 | Start Dimo and open the project | The balloon is there |
| 2.5 | Change something, quit with ⌘Q (Alt+F4 or the menu elsewhere), choose Don't save | Dimo ends; open the project again: the change is gone and there is no recovery note |
| 2.6 | With unsaved changes choose **New project** or **Open project** | The same question appears before the dialog |
| 2.7 | Close the window of a project without changes | No question |

## 3. Crash and recovery (NFR-REL-01, D-28)

| # | Step | Expected |
|---|---|---|
| 3.1 | Open a saved project, place two balloons, force quit Dimo (Activity Monitor, or kill the process) | |
| 3.2 | Start Dimo and open the project | The two balloons are there; a note says changes were restored |
| 3.3 | Create a new project, place two balloons, force quit | |
| 3.4 | Start Dimo | The unsaved project is restored with the note; saving keeps it |
| 3.5 | Look for `<project>.dimo.journal` next to a saved project after saving | The file is gone |
| 3.6 | Start a second Dimo instance and open a project that the first one has open | A message that the project is open in another window, with the user name |
| 3.7 | Force quit the first instance, then open the project in the second | It opens; the leftover `.lock` file does not block it |

## 4. Gestures and feel (FR-DOC-05, NFR-PERF-02)

| # | Step | Expected |
|---|---|---|
| 4.1 | Zoom with the mouse wheel | Zooms around the pointer, steady, not too fast or slow |
| 4.2 | Pinch on a trackpad | Zooms around the fingers, smoothly; the page itself does not zoom |
| 4.3 | Two finger scroll on a trackpad | Note what happens (it zooms in this version); decide if that is acceptable |
| 4.4 | Zoom to the minimum and maximum | Stops at the limits, nothing breaks |
| 4.5 | Pan with the middle button, with Space and left drag, with the arrow keys | All move the drawing; the cursor shows a hand |
| 4.6 | Press 0, then +, then - | Fit, zoom in, zoom out |
| 4.7 | Zoom in to 800 % on a dimension text | Text stays sharp; tiles appear without gaps or seams |
| 4.8 | Rotate the sheet (R, Shift+R) | The view turns; balloons and tiles turn together, numbers stay upright |
| 4.9 | Place 30 balloons quickly (B, click, type, Enter) | No lag; focus returns to the drawing after each Enter |
| 4.10 | Open the 50 sheet stress PDF and page through with PgDn | Each sheet shows within 2 seconds (NFR-PERF-01); memory stays below 1.5 GiB (NFR-PERF-03) |

## 5. Balloons and table by hand (FR-BAL, FR-CHR)

| # | Step | Expected |
|---|---|---|
| 5.1 | Drag a box around a dimension with the Place balloon tool | A balloon appears up and right of the box; the leader points at the box corner |
| 5.2 | Place a balloon near the sheet edge | It moves to the other side so it stays on the sheet |
| 5.3 | Drag a balloon, then drag its leader handle | Both move; one undo step each |
| 5.4 | Shift drag a box over several balloons, then Shift+arrow | They are selected and move 1 mm per press |
| 5.5 | Select balloons, press S, change shape, size, color, leader | Each change applies at once to all selected balloons |
| 5.6 | Drag a row by the handle ⠿ to another place, also near the top and bottom edge of the table | The row moves; the table scrolls while dragging; all numbers change; ⌘Z restores |
| 5.7 | Resize the table with the mouse and with the keyboard | Height changes; it is remembered after a restart |
| 5.8 | Edit every kind of cell: text, number, list, check box | Each can be edited with the mouse and with the keyboard only |
| 5.9 | Type `1,5` in Nominal | The cell stays open and a message says to use a point |
| 5.10 | Select a row whose balloon is far outside the view, then one on another sheet | The balloon is centered; the other sheet is shown |

## 6. Look of the exports in other programs (FR-EXP-01, FR-EXP-09)

| # | Step | Expected |
|---|---|---|
| 6.1 | Open the ballooned PDF in two PDF viewers | Balloons are where they are in the app, same shape, size and color; numbers readable and upright |
| 6.2 | Export with balloons as annotations and open it | The balloons can be hidden and shown in the viewer's annotation list |
| 6.3 | Print the ballooned PDF at 100 % | Balloons are 7 mm (or the chosen size) on paper |
| 6.4 | Open the CSV in a spreadsheet program | Columns are as in the user guide; `Ø` and German umlauts are right (import as UTF-8 if the program asks) |
| 6.5 | Open the XLSX | One sheet, bold frozen header, filters; `90.0` shows as `90.0`; numbers are numbers |
| 6.6 | Export with German headers | German column names, English values (`diameter`, `yes`) |
| 6.7 | Export as issued, then try to move a row and place a balloon | Moving is refused with the lock message; the new balloon gets the highest number plus one |
| 6.8 | Press ⌘Z right after an issued export | The lock is undone, rows can move again |

## 7. Settings and appearance (D-50, D-51, FR-SET-04)

| # | Step | Expected |
|---|---|---|
| 7.1 | Switch the theme to dark, light, system | Applies at once; the drawing area, table and dialogs follow; survives a restart |
| 7.2 | Change the OS theme while Dimo runs with "same as the system" | Dimo follows |
| 7.3 | Switch the language with a project open | The window reloads, the project stays, texts are in the new language |
| 7.4 | Change the user name, change something, save, look in the project's `audit.jsonl` | The new name is in the new entries |
| 7.5 | Look at every view in both languages | No clipped or English leftover texts in German |
| 7.6 | Use the app with the keyboard only (Tab, Enter, Space, arrows) | Every control can be reached and has a visible focus |

## Result

| Area | Pass | Notes |
|---|---|---|
| 1 File dialogs | | |
| 2 Close and quit | | |
| 3 Crash and recovery | | |
| 4 Gestures and feel | | |
| 5 Balloons and table | | |
| 6 Exports in other programs | | |
| 7 Settings and appearance | | |

Report every failure as an issue with the step number, what happened and the platform.
