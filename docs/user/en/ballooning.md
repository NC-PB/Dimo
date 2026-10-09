# Ballooning a drawing

A balloon is the numbered mark on the drawing that belongs to one characteristic. You place
balloons in the drawing view with the **Place balloon** tool. Every action below can be undone
with ⌘Z (Ctrl+Z on Windows and Linux) and redone with ⇧⌘Z (Ctrl+Shift+Z or Ctrl+Y).

## Tools

| Tool | Key | What the left mouse button does on the drawing |
|---|---|---|
| Select | V | Click selects a balloon, drag moves balloons, drag on empty drawing pans |
| Place balloon | B | Click or drag places a new balloon, drag on a balloon moves it |

In both tools you can move the drawing with the middle mouse button, or by holding Space while
you drag. Zooming and sheets are in [Looking at a drawing](drawing-view.md).

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

The field stores the text as written, in the **Requirement** column of the characteristic table.
Dimo does not split it into nominal value and tolerances in this version; you fill nominal,
deviations, kind and the rest in the table ([Characteristic table](characteristics.md)).

## Balloon numbers

While numbering is not locked, the balloons are numbered 1, 2, 3 and so on in the order of the
characteristic table, without gaps. A new balloon gets the next number. Deleting a balloon or
moving a row in the table numbers all of them again.

After an export as issued ([Exports](exports.md)) the numbering is locked. Numbers then never
change: a new balloon gets the highest number ever used plus one, a deleted number is not used
again, and rows cannot be moved.

## Select balloons

| Action | Mouse | Key |
|---|---|---|
| Select one balloon | Click it | |
| Add or remove a balloon | Shift+click or ⌘+click (Ctrl+click) | |
| Select several by area | Shift+drag on empty drawing, balloons whose center is inside are added | |
| Select all balloons of the sheet | | ⌘A (Ctrl+A) |
| Clear the selection | Click on empty drawing (Select tool) | Escape |

Selected balloons get a thick orange ring and a small square handle at the end of their leader.

## Move balloons and leaders

- Drag a selected balloon to move all selected balloons together. Drag an unselected balloon to
  select and move only that one. The leader ends stay on the drawing.
- Drag the square handle at the end of a leader to point the leader somewhere else.
- One drag is one undo step, however many balloons it moves.
- With the keyboard: Shift and an arrow key move the selected balloons 1 mm (on the printed
  sheet) in that direction on screen. Each press is one undo step. The arrow keys alone pan.

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

Balloons you place by hand are accepted. Proposed and rejected come with the automatic recognition
of a later version; the file formats and exports already handle them (rejected characteristics get
no balloon in the exported PDF and are left out of the lists).

All keys are in [Keyboard shortcuts](shortcuts.md) and in the cheat sheet, press **?**.
