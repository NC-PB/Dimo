# Characteristic table

The table under the drawing lists every characteristic of the project, in balloon number order.
Drag the line above the table to change its height (or focus it and use ↑ and ↓).

## Columns

| Column | Content |
|---|---|
| No | Balloon number. Drag the handle ⠿ to move the characteristic |
| Kind | Linear, diameter, radius, spherical radius, angle, chamfer, thread, counterbore, countersink, depth, surface texture, geometric tolerance, note, flag note, material or process, other |
| Requirement | Text as it appears on the drawing |
| Nominal | Nominal value |
| Upper dev., Lower dev. | Deviations, with sign |
| Upper limit, Lower limit | Limits. Dimo works them out from nominal and deviations; you can also type them |
| Unit | mm, in, ° or none. A nominal without a unit gets the unit of the sheet (degrees for angles) |
| Fit | Fit designation such as `H7` |
| Qty | Number of features the characteristic stands for, at least 1 |
| Class | Critical, major, minor, key, or empty when not classified |
| Method, Gauge, Sampling, Frequency | How the characteristic is inspected, free text |
| Comment | Free text |
| Inspect | Cleared for reference and basic dimensions |

Numbers are shown with exactly the digits stored, so `90.0` stays `90.0`.

## Selecting

Selecting a row puts a thick ring around its balloon on the drawing, and clicking a balloon
selects its row and scrolls it into view. If the balloon is on another sheet, that sheet is shown
with the balloon in the middle; a balloon outside the visible part of the drawing is moved into
the middle as well.

- Click a row to select it. ⌘ click (Ctrl click) adds or removes a row, Shift click selects a
  range.
- ↑ and ↓ move through the rows, Shift ↑ and Shift ↓ extend the selection, ⌘A (Ctrl+A) selects
  all.

## Editing

Double click a cell, press Enter or F2, or start typing. Enter confirms, Escape cancels, Tab
confirms and moves to the next column, ↑ and ↓ confirm and move to the row above or below.
Kind, unit and class open a list. Space switches Inspect.

Type numbers with a point as decimal separator, for example `-0.05`. If Dimo cannot use a value,
the cell stays open and the line above the table says why. Every change can be undone with ⌘Z
(Ctrl+Z).

## Changing the order

Drag the handle ⠿ in the No column, or select rows and press ⌥↑ or ⌥↓ (Alt+↑ or Alt+↓). Several
selected rows move together and keep their order. All characteristics are renumbered in the same
step, so one undo puts everything back.

While numbering is locked, for example after a report was issued, the order cannot change. The
line above the table shows who locked the numbering and when.

## Keyboard

The keyboard shortcut list (`?`) has a section for the table. While the table has the focus,
the arrow keys, Enter, Escape, Space and ⌘A (Ctrl+A) act on the table, and single letter keys such
as R, B or S never act on the drawing. Shortcuts with ⌘ (Ctrl), such as save and undo, work
everywhere.
