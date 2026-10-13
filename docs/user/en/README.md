# Dimo user guide

Dimo numbers every characteristic of an engineering drawing that has to be inspected with a
balloon, keeps the list of characteristics, takes the values you measured on your parts and
decides pass or fail. It exports a check sheet, an inspection report, a ballooned PDF and a
characteristic list. Everything stays on your computer.

This version is for **assisted ballooning**: you drag a box around a dimension and Dimo reads its
PDF text, or you click a feature and type the value as it is printed. Dimo reads nominal value,
fit and written tolerances from the text, works out the limits (written tolerances first, then
the ISO 286 fit table, then the general tolerance and the rules of the project), explains where
they come from, and takes care of numbering, the list, saving and the exports. Dimo does not
search the drawing by itself yet. Without a general tolerance in the project, a dimension without
tolerance is marked "No tolerance defined"; choose one in the **General tolerance** field of the
**Home** or **Drawing** tab, or enter its limits in the characteristic table.

## Pages

| Page | Content |
|---|---|
| [Looking at a drawing](drawing-view.md) | Zoom, move, sheets, the side panel |
| [Ballooning a drawing](ballooning.md) | Place, select, move, style and delete balloons |
| [Characteristic table](characteristics.md) | Values, selecting, editing, changing the order |
| [Sheets: rotation, unit and scale](sheets.md) | Properties of every sheet |
| [Numbering strategies, zones and views](numbering.md) | Number by zone, view or type, preview, sub-numbers, locked numbering |
| [Project and report details](project-details.md) | Part, customer, revision, drawing hash, report header and approval fields |
| [Entering results](measuring.md) | Samples, the measurement grid, keyboard entry, pasting, pass/fail marks, result types |
| [Projects](projects.md) | New, open, save, autosave, recovery |
| [Exports](exports.md) | Check sheet and inspection report, ballooned PDF, CSV and Excel list, issued drawings and reports |
| [Tolerances](tolerances.md) | Rules, general tolerance, custom tables, explanations, draft tables, history |
| [File page, options and About](settings.md) | Project info, recent projects, options (theme, language, user name, company name, export defaults), project settings of the Drawing tab, licenses |
| [Keyboard shortcuts](shortcuts.md) | Every key, for macOS and for Windows and Linux |

Press **?** in the app at any time to see the keyboard shortcuts.

## Getting started: balloon your first drawing

This walkthrough takes a PDF drawing from the first click to the exported files. It uses every
feature of this version once, in the order you normally need them. Keys are written for macOS;
on Windows and Linux use Ctrl instead of ⌘ and Alt instead of ⌥.

### 1. Start a project from a drawing

Start Dimo and choose **New project** (⌘N). Pick the PDF of the drawing. Dimo copies the drawing
into the project, so the project does not depend on the original file afterwards, and shows the
first sheet fitted into the window. A drawing with several sheets (pages) shows one tab per sheet
in the status bar at the bottom.

The project has no file yet. The title bar says "Untitled"; you save it in step 9, but Dimo already
keeps a recoverable copy of everything you do ([Projects](projects.md)).

If the sheet is sideways, set its rotation, unit and scale in **Sheet properties** on the right
now, as in step 8. Setting the unit and the scale early saves you from forgetting them.

### 2. Look around

Zoom with the mouse wheel or a pinch on the trackpad (the point under the pointer stays where it
is), or with **+** and **-**. **0** fits the whole sheet into the window again. Move the drawing
with the arrow keys, with the middle mouse button, or by holding Space and dragging. PgUp and PgDn
switch sheets. Details: [Looking at a drawing](drawing-view.md).

### 3. Choose the general tolerance of the project

Most drawings print a general tolerance in the title block, for example ISO 2768-m. Dimo does not
read the title block yet, so tell it once: on the **Home** tab (or the **Drawing** tab), choose
**ISO 2768-1** with the class from the drawing in the **General tolerance** field, for example
"ISO 2768-1 general tolerances m". From now on a dimension without a tolerance gets its limits from
that table. Without a general tolerance such a dimension is marked **No tolerance defined** and
has no limits. The tables shipped with Dimo carry the marker **Draft** until their values have
been verified. Details: [Tolerances](tolerances.md).

### 4. Place balloons and capture the values

1. Press **B** (the **Place balloon** tool).
2. Drag a box around the dimension text. A card shows what Dimo read: kind, nominal value,
   limits, the rule the limits come from and an explanation. Press **Enter** to accept it.
   Or click the dimension or note, type the value as printed, for example `Ø8 f7`, and
   press **Enter**. Dimo reads typed text the same way as a box.
3. Press **Escape** to discard a card you do not want.
4. Drag or click the next feature. You can start typing right after the click or the box.

The balloons are numbered 1, 2, 3 in the order you place them. If you made a mistake, press ⌘Z.
Press **V** to go back to the **Select** tool. Details: [Ballooning a drawing](ballooning.md).
Click a balloon or a row to see the rule, the explanation and the history of a characteristic in
the side panel ([Tolerances](tolerances.md)).

### 5. Complete the values in the characteristic table

The table under the drawing has one row per balloon. The text read or typed is in the
**Requirement** column, and kind, nominal, fit and printed tolerances are already filled in.
Complete what the inspection needs: **Kind**, **Nominal**, **Upper dev.** and **Lower dev.**
(Dimo computes **Upper limit** and **Lower limit**), **Unit**, **Fit**, **Qty**, **Class**, and the
columns for method, gauge, sampling and frequency. Double click a cell or start typing, confirm
with Enter, move on with Tab. Clear **Inspect** for reference and basic dimensions that are not
measured. Numbers are typed with a point, and the digits you type are the digits that are stored
and exported (`90.0` stays `90.0`). Details: [Characteristic table](characteristics.md).

Click a row to see its balloon in the drawing, and click a balloon to see its row.

### 6. Change the order and the numbering

Drag the handle ⠿ of a row, or select rows and press ⌥↑ or ⌥↓. All characteristics are numbered
again, and the balloons on the drawing follow. One ⌘Z puts everything back.

To number by zone, by view or by type, use the **Numbering** panel in the side panel: choose a
strategy, tick **Preview new numbers** to see the result on the drawing and in the table, and
choose **Apply numbering**. Zone grid and views are drawn by hand in **Sheet properties**.
Details: [Numbering strategies, zones and views](numbering.md).

### 7. Style the balloons

Select balloons and press **S**: shape (circle, flag, rectangle), leader line, size and outline
color. The style of the whole project is **Project balloon style** on the **Drawing** tab. Status
is shown by shape as well as color. Details: [Ballooning a drawing](ballooning.md),
[File page, options and About](settings.md).

### 8. Set rotation, unit and scale of the sheet

In **Sheet properties** on the right: rotate the sheet in steps of 90 degrees (**R** and
**Shift+R**), choose millimeter or inch, and the scale of the title block. Each sheet has its own
values. Details: [Sheets](sheets.md).

### 9. Save

Choose **Save** (⌘S). Because the project has no file yet, Dimo asks for a name and location and
writes one `.dimo` file that contains the drawing, the characteristics, the balloons and the log
of all changes. From then on ⌘S saves into that file. **Save as** (⇧⌘S) saves under another name.

Between saves, Dimo writes every change to an autosave journal. After a crash or a power cut,
opening the project again (or starting Dimo, for a project that was never saved) restores the
work, with a note above the drawing. When you close the window or create or open another
project with unsaved changes, Dimo asks first. Details: [Projects](projects.md).

### 10. Enter the results

Open the **Measure** tab (⌘3). Type how many parts you measure under **Number** and choose
**Add parts**. Click the first cell of a part, type the value, press **Enter**, and carry on down
the column: Dimo stores the value, moves to the next characteristic and marks the result with ✓
(pass), ✗ (fail) or ○ (value without limits). For threads and notes press **O** for OK or **N**
for not OK. A block copied from a spreadsheet pastes with ⌘V. Limits are inclusive, and Dimo
decides pass or fail for you. Details: [Entering results](measuring.md).

### 11. Export

First fill in the part, drawing revision and report fields in **Project and report**
([Project and report details](project-details.md)). Then open the **Report** tab (⌘E).

- **Check sheet** (PDF or Excel): the characteristics with empty result columns, to print before
  you measure.
- **Inspection report** (PDF or Excel): summary, samples, results with verdicts and the signature
  block.
- **Ballooned PDF**: the drawing with the balloons drawn in. The original file is not changed.
- **Characteristic list (CSV)** and **(Excel)**: one row per characteristic, in balloon order,
  with the rule that gave each limit.

Pick the language of the reports and column headers and how the PDF carries the balloons. Each
export asks where to save the file and runs in the background; **Cancel export** stops a report
export. For a drawing or report that goes to a customer, tick **Export as issued** first: it locks
the numbering, so numbers never change afterwards, even if you add or delete characteristics, and a
report is recorded in the project. Details: [Exports](exports.md).

### 12. Options

In **File** > **Options** (⌘,) choose the theme (light, dark or as the system), the language of
the app, the user name that is written into the change log of each project, the company name for
reports and the export defaults. Options are kept for the next start and belong to you, not to a
project. The project itself (part, tolerances, report header, progress) is summed up in
**File** > **Info**, and **File** > **Open project** lists your recent projects. Details:
[File page, options and About](settings.md).

### What next

- Open the project later with **Open project** (⌘O) and carry on. Every change you made is recorded
  in the change log inside the project.
- Enter the measured values of your parts in the **Measure** view (⌘3), see
  [Entering results](measuring.md).
- If limits change after you entered results, the results stay and the verdicts follow the new
  limits ([Entering results](measuring.md#when-limits-change)).
- Not yet in this version: searching the drawing automatically. The **Review** view shows a note
  that it comes in a later version.
