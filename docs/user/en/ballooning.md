# Ballooning a drawing

A balloon is the numbered mark on the drawing that belongs to one characteristic. You place
balloons in the drawing view with the **Place balloon** tool. Every action below can be undone
with ⌘Z (Ctrl+Z on Windows and Linux) and redone with ⇧⌘Z (Ctrl+Shift+Z or Ctrl+Y).

## Tools

| Tool | Key | What the left mouse button does on the drawing |
|---|---|---|
| Select | V | Click selects a balloon, drag moves balloons, drag on empty drawing pans |
| Place balloon | B | Click or drag places a new balloon, drag on a balloon moves it |

In both tools you can pan with the middle mouse button, or by holding Space while you drag.
The mouse wheel and the trackpad zoom as before.

## Place balloons

1. Press **B** or choose **Place balloon** in the toolbar.
2. Click the feature you want to inspect, or drag a box around its dimension text.
   The balloon appears up and to the right of it, with the next free number, and its leader
   points to where you clicked (or to the corner of the box). Near the sheet edge it goes to the
   other side.
3. A small field opens next to the balloon. Type the value as it is printed on the drawing,
   for example `Ø8 f7` or `100 ±0.2`.
4. Press **Enter**. The value is stored and the tool stays active, so you click the next
   feature right away.

So the fastest way through a drawing is: click, type, Enter, click, type, Enter.

- **Escape** in the field closes it without storing the text. The balloon stays; ⌘Z removes it.
- Clicking somewhere else also stores what you typed.
- The box you drag is stored with the characteristic as its source region, so you can find later
  where the value came from. A plain click stores no region.
- **Enter** on a selected balloon, or a double click on it, opens the field again.

The field stores the text as written. Nominal value, tolerances and the kind of characteristic
are filled in the characteristic table.

## Select balloons

| Action | Mouse | Key |
|---|---|---|
| Select one balloon | Click it | |
| Add or remove a balloon | Shift+click or ⌘+click (Ctrl+click) | |
| Select several by area | Shift+drag on empty drawing, balloons whose center is inside are added | |
| Select all balloons of the sheet | | ⌘A (Ctrl+A) |
| Clear the selection | Click on empty drawing | Escape |

Selected balloons get a thick orange ring and a small square handle at the end of their leader.

## Move balloons and leaders

- Drag a selected balloon to move all selected balloons together. Drag an unselected balloon to
  select and move only that one. The leader ends stay on the drawing.
- Drag the square handle at the end of a leader to point the leader somewhere else.
- One drag is one undo step, however many balloons it moves.

## Change the style

Select balloons and press **S** or choose **Style** in the toolbar. Each choice applies at once to
all selected balloons:

- **Shape**: circle, flag or rectangle.
- **Leader line** on or off.
- **Size** in millimeters on the printed sheet. The default is 7 mm.
- **Outline color**, each color with its name.
- **Use project default** removes the changes, so the balloons follow the project style again.

Balloons are drawn on the drawing at the size they will have on the printed sheet, so they grow
and shrink with the zoom and always stay at their place on the drawing. On a rotated sheet the
numbers stay upright.

## Delete balloons

Select balloons and press **Delete** or **Backspace**, or choose **Delete** in the toolbar. The
characteristics of the balloons are deleted with them. While numbering is not locked the other
balloons are numbered again from 1 without gaps.

## How status is shown

Status is never shown by color alone:

| Status | Look |
|---|---|
| Accepted, verified | Solid outline in the balloon color |
| Proposed | Dashed outline |
| Rejected | Dashed grey outline, number in grey, struck through |

All keys are also listed in the cheat sheet, press **?**.
