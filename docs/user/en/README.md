# Dimo user guide

Dimo numbers every characteristic of an engineering drawing that has to be inspected with a
balloon, keeps the list of characteristics, and exports a ballooned PDF and a characteristic list
for inspection. Everything stays on your computer.

This version is for **manual ballooning**: you click a feature on the drawing, type the value as it
is printed, and Dimo takes care of numbering, the list, saving and the exports. Dimo does not read
the drawing by itself yet. It also does not split the typed text into nominal value and tolerances;
you enter those in the characteristic table, and Dimo works out the limits from them.

## Pages

| Page | Content |
|---|---|
| [Looking at a drawing](drawing-view.md) | Zoom, move, sheets, the side panel |
| [Ballooning a drawing](ballooning.md) | Place, select, move, style and delete balloons |
| [Characteristic table](characteristics.md) | Values, selecting, editing, changing the order |
| [Sheets: rotation, unit and scale](sheets.md) | Properties of every sheet |
| [Numbering strategies, zones and views](numbering.md) | Number by zone, view or type, preview, sub-numbers, locked numbering |
| [Projects](projects.md) | New, open, save, autosave, recovery |
| [Exports](exports.md) | Ballooned PDF, CSV and Excel list, issued drawings |
| [Settings](settings.md) | Theme, language, user name, balloon style |
| [Keyboard shortcuts](shortcuts.md) | Every key, for macOS and for Windows and Linux |

Press **?** in the app at any time to see the keyboard shortcuts.

## Getting started: balloon your first drawing

This walkthrough takes a PDF drawing from the first click to the exported files. It uses every
feature of this version once, in the order you normally need them. Keys are written for macOS;
on Windows and Linux use Ctrl instead of ⌘ and Alt instead of ⌥.

### 1. Start a project from a drawing

Start Dimo and choose **New project** (⌘N). Pick the PDF of the drawing. Dimo copies the drawing
into the project, so the project does not depend on the original file afterwards, and shows the
first sheet fitted into the window. A drawing with several sheets (pages) shows a sheet selector
in the toolbar.

The project has no file yet. The toolbar says "Untitled"; you save it in step 8, but Dimo already
keeps a recoverable copy of everything you do ([Projects](projects.md)).

If the sheet is sideways, set its rotation, unit and scale in **Sheet properties** on the right
now, as in step 7. Setting the unit and the scale early saves you from forgetting them.

### 2. Look around

Zoom with the mouse wheel or a pinch on the trackpad (the point under the pointer stays where it
is), or with **+** and **-**. **0** fits the whole sheet into the window again. Move the drawing
with the arrow keys, with the middle mouse button, or by holding Space and dragging. PgUp and PgDn
switch sheets. Details: [Looking at a drawing](drawing-view.md).

### 3. Place balloons and capture the values

1. Press **B** (the **Place balloon** tool).
2. Click the dimension or note you want to inspect, or drag a box around its text.
3. A small field opens next to the new balloon. Type the value as printed, for example `Ø8 f7`,
   and press **Enter**.
4. Click the next feature.

The balloons are numbered 1, 2, 3 in the order you place them. If you made a mistake, press ⌘Z.
Press **V** to go back to the **Select** tool. Details: [Ballooning a drawing](ballooning.md).

### 4. Complete the values in the characteristic table

The table under the drawing has one row per balloon. The typed text is in the **Requirement**
column. Fill in what the inspection needs: **Kind**, **Nominal**, **Upper dev.** and **Lower dev.**
(Dimo computes **Upper limit** and **Lower limit**), **Unit**, **Fit**, **Qty**, **Class**, and the
columns for method, gauge, sampling and frequency. Double click a cell or start typing, confirm
with Enter, move on with Tab. Clear **Inspect** for reference and basic dimensions that are not
measured. Numbers are typed with a point, and the digits you type are the digits that are stored
and exported (`90.0` stays `90.0`). Details: [Characteristic table](characteristics.md).

Click a row to see its balloon in the drawing, and click a balloon to see its row.

### 5. Change the order

Drag the handle ⠿ of a row, or select rows and press ⌥↑ or ⌥↓. All characteristics are numbered
again, and the balloons on the drawing follow. One ⌘Z puts everything back.

### 6. Style the balloons

Select balloons and press **S**: shape (circle, flag, rectangle), leader line, size and outline
color. The style of the whole project is in **Settings**. Status is shown by shape as well as
color. Details: [Ballooning a drawing](ballooning.md), [Settings](settings.md).

### 7. Set rotation, unit and scale of the sheet

In **Sheet properties** on the right: rotate the sheet in steps of 90 degrees (**R** and
**Shift+R**), choose millimeter or inch, and the scale of the title block. Each sheet has its own
values. Details: [Sheets](sheets.md).

### 8. Save

Choose **Save** (⌘S). Because the project has no file yet, Dimo asks for a name and location and
writes one `.dimo` file that contains the drawing, the characteristics, the balloons and the log
of all changes. From then on ⌘S saves into that file. **Save as** (⇧⌘S) saves under another name.

Between saves, Dimo writes every change to an autosave journal. After a crash or a power cut,
opening the project again (or starting Dimo, for a project that was never saved) restores the
work, with a note above the drawing. When you close the window or create or open another
project with unsaved changes, Dimo asks first. Details: [Projects](projects.md).

### 9. Export

Open the **Export** view (⌘E).

- **Ballooned PDF**: the drawing with the balloons drawn in. The original file is not changed.
- **Characteristic list (CSV)** and **(Excel)**: one row per characteristic, in balloon order.

Pick the language of the column headers and how the PDF carries the balloons. Each export asks
where to save the file and runs in the background. For a drawing that goes to a customer, tick
**Export as issued** first: it locks the numbering, so numbers never change afterwards, even if
you add or delete characteristics. Details: [Exports](exports.md).

### 10. Settings

In **Settings** (⌘,) choose the theme (light, dark or as the system), the language of the app, and
the user name that is written into the change log of each project. Settings are kept for the next
start. Details: [Settings](settings.md).

### What next

- Open the project later with **Open project** (⌘O) and carry on. Every change you made is recorded
  in the change log inside the project.
- Not yet in this version: reading the drawing automatically, parsing tolerances from the typed
  text, inspection results and reports. The **Review** and **Measure** views show a note that they
  come in a later version.
