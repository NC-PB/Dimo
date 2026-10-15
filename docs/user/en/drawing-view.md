# Looking at a drawing

The drawing is where you work. It shows the sheet in the middle, the characteristic table under it
([Characteristic table](characteristics.md)) and the properties pane on the right.

## The window

- **Title bar**: Save, Undo and Redo, the name of the project with "saved" or "not saved", the
  light or dark switch and **Help** (the list of keyboard shortcuts).
- **Tabs**: **File** (project info, new, open with the recent projects, save, save as, close, options, about), **Home** (place and edit balloons,
  numbering, general tolerance), **Drawing** (rotation, unit, scale, zone grid, views,
  tolerances, project info), **Review** (a placeholder in this version), **Measure**
  ([Entering results](measuring.md)), **Report** ([Exports](exports.md)) and **View** (zoom,
  show or hide the table and the properties pane, theme). Each tab shows its commands in one row
  under the tabs. Hold the pointer over a command to see what it does, its shortcut and, when it
  is grey, why it cannot be used now. In a narrow window the last groups move into **More**.
- **Status bar** at the bottom: the sheets, the active tool, the number of characteristics and
  of selected ones (on the Measure tab the totals of all parts), the general tolerance, unit and
  scale, and the zoom.

Home and Drawing both show the drawing (⌘1). ⌘2 shows Review, ⌘3 Measure, ⌘E Report.

## Zoom

| Action | How |
|---|---|
| Zoom in or out around the pointer | Mouse wheel, or pinch on the trackpad |
| Zoom in or out around the center | **+** and **-**, the buttons on the View tab or the zoom in the status bar |
| Fit the whole sheet into the window | **0**, or **Fit sheet** on the View tab |

The status bar shows the zoom as a percentage of the printed size: 100 % means the drawing is as
large on the screen as it would be on paper. Balloons keep their printed size, so they grow and
shrink with the zoom.

## Move the drawing

| Action | How |
|---|---|
| Move in small steps | Arrow keys |
| Drag the drawing | Middle mouse button, or hold Space and drag with the left button |
| Drag the drawing with the Select tool | Drag on empty drawing |

The arrow keys move the view only while the drawing, not a text field or the table, has the
keyboard focus. Shift with an arrow key moves the selected balloons instead
([Ballooning a drawing](ballooning.md)). Single keys such as **+**, **-**, **0**, **R**, **B**,
**V** and **S** do not act on the drawing while the table has the focus.

## Sheets

A PDF with several pages has one sheet per page. The status bar shows one tab per sheet ("Sheet
2"); click one to show it. **PgUp** and **PgDn** switch sheets too. A sheet is
fitted into the window the first time you show it. Selecting a characteristic in the table whose
balloon is on another sheet switches to that sheet and shows the balloon.

Every sheet has its own rotation, unit and scale ([Sheets](sheets.md)). Rotating a sheet turns the
view: balloons and their numbers stay upright and exports are not affected.

## Properties pane

The pane on the right follows the selection. Its header says what it shows; the × hides the pane
(**View > Properties** shows it again). Click the heading of a section to fold or unfold it; Dimo
remembers which sections you folded.

- **Nothing selected**: a summary of the project (part, drawing, general tolerance, number of
  characteristics and sheets) with **Edit in File > Info**, where these values are changed
  ([Project and report details](project-details.md)), and a hint how to start.
- **One characteristic**: **Requirement** (text, kind, quantity), **Tolerance** (rule, limits, the
  shorter leg of an angle, the explanation and **Update limits**), **Inspection** (inspect, result
  type, method, gauge) and **History** ([Tolerances](tolerances.md)). Each change is one undo
  step, like an edit in the table.
- **Several characteristics**: the list, **Update limits** for all of them, and **Yes** or **No**
  to inspect all of them.
- **Zone grid** and **Draw view** on the Drawing tab open a section to edit the grid or the list of
  views until you close it with its × ([Numbering strategies, zones and views](numbering.md)).
- While **Preview numbers** is on, a section counts the numbers that change.

Rotation, unit and scale of a sheet are on the Drawing tab ([Sheets](sheets.md)), the numbering
order on the Home tab.

## Table height

Drag the line between the drawing and the table to give one of them more room. With the keyboard,
focus the line and press ↑ or ↓. Dimo remembers the height.

## If a drawing does not open

Dimo shows a message above the drawing, for example that the file could not be opened. Only PDF
files can be used as drawings in this version. If the message says that the PDF component is
missing, the installation is incomplete; install Dimo again.

Error messages are shown in the language of the app. Below a message, **Details** opens the
technical text (file path, system error) to add to a bug report.
