# Looking at a drawing

The **Drawing** view is where you work. It shows the sheet in the middle, the characteristic table
under it ([Characteristic table](characteristics.md)) and a side panel on the right. The buttons
**Drawing**, **Review**, **Measure**, **Export** and **Settings** at the top switch the view. The
Drawing view is also ⌘1. **Review** and **Measure** are placeholders in this version.

## Zoom

| Action | How |
|---|---|
| Zoom in or out around the pointer | Mouse wheel, or pinch on the trackpad |
| Zoom in or out around the center | **+** and **-** (also the buttons in the toolbar) |
| Fit the whole sheet into the window | **0**, or **Fit sheet** in the toolbar |

The toolbar shows the zoom as a percentage of the printed size: 100 % means the drawing is as
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

A PDF with several pages has one sheet per page. When there is more than one sheet, the toolbar
shows a selector ("Sheet 2 of 5") with arrows. **PgUp** and **PgDn** switch sheets too. A sheet is
fitted into the window the first time you show it. Selecting a characteristic in the table whose
balloon is on another sheet switches to that sheet and shows the balloon.

Every sheet has its own rotation, unit and scale ([Sheets](sheets.md)). Rotating a sheet turns the
view: balloons and their numbers stay upright and exports are not affected.

## Side panel

- **Sheet properties**: rotation, unit and scale of the sheet on screen.
- **Selection**: the values of the selected characteristic, or the list of selected
  characteristics. The panel only shows them; edit values in the table.
- The version of Dimo at the bottom.

## Table height

Drag the line between the drawing and the table to give one of them more room. With the keyboard,
focus the line and press ↑ or ↓. Dimo remembers the height.

## If a drawing does not open

Dimo shows a message above the drawing, for example that the file could not be opened. Only PDF
files can be used as drawings in this version. If the message says that the PDF engine is
missing, the installation is incomplete; install Dimo again.
