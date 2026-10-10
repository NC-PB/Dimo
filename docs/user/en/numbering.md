# Numbering strategies, zones and views

Dimo numbers the balloons in the order of the characteristic table. The **Numbering** panel on
the right puts that order in a pattern: sheet by sheet and zone by zone, view by view, clockwise
around each view, or by type. You see the new numbers before they change, and applying them is
one step that ⌘Z takes back.

## Zone grid

Most drawing frames are divided into zones, with letters along one edge and numbers along the
other, for example `B3`. Dimo does not read the frame by itself yet, so you set the grid once per
sheet in **Sheet properties**, under **Zone grid**:

1. Choose **Draw frame** and drag a rectangle along the inner edge of the drawing frame. Esc
   cancels. (**Add zone grid** puts a frame 10 mm inside the sheet edge instead, which you can draw
   again later.)
2. Set the number of **Columns** and **Rows**.
3. Choose how the labels are written: for columns 1, 2, 3 or A, B, C from the left or from the
   right, for rows A, B, C or 1, 2, 3 from the top or from the bottom.
4. If the frame uses other labels, type them in the label fields, separated by commas, left to
   right and top to bottom as printed. The number of labels sets the number of columns and rows.

The grid is drawn on the sheet as dashed lines with its labels. **Remove zone grid** deletes it.

## Views

The strategies per view need the views of the drawing. Under **Views** choose **Draw view** and
drag a rectangle around a view, for example the front view or a section. Draw the views in the
order you want them numbered. Each view can get a name such as `A-A` or `Detail B`; the × button
removes it. Views are drawn on the sheet as dotted rectangles.

## Strategies

A balloon belongs to the place of its leader end, the point on the feature. Everything is read as
the sheet is shown on screen, so a rotated sheet is read the way you see it.

| Strategy | Order |
|---|---|
| Sheet, zone, reading order | Sheet by sheet, zone by zone (rows of zones top to bottom, left to right), inside a zone in reading order. Without a zone grid the whole sheet is read in reading order. This is the default for new projects. |
| Per view, reading order | View by view in the order you drew them, reading order inside each view. Balloons outside all views come last on their sheet, in reading order. |
| Per view, clockwise | View by view, clockwise around the middle of the view, starting at 12 o'clock. Balloons outside all views come last. |
| By type | Grouped by kind (linear, diameter, radius, angle and so on), each kind in sheet and zone order. |
| Manual (current order) | Keeps the order of the table. Change it by dragging rows. |

**Reading order** means top to bottom, then left to right. Balloons whose leader ends are less
than one balloon diameter apart in height count as one line and are read left to right. A point
inside two views, for example a detail drawn inside a larger view, belongs to the smaller one.
A point outside the zone frame belongs to the nearest zone.

## Preview and apply

1. Pick the strategy in the **Numbering** panel. A short text explains it.
2. Tick **Preview new numbers**. Each balloon gets its new number in a dashed box next to it, and
   the table shows it next to the current number. Numbers that change are drawn bold and in
   color, numbers that stay are thin and grey. The panel counts how many numbers change.
3. Choose **Apply numbering**. The table is sorted in the new order, all balloons get their new
   numbers, and the strategy is saved with the project. ⌘Z undoes the whole step.

The preview follows every change you make while it is on, so it always shows what applying would
do. New balloons are still added at the end with the next number; apply the strategy again when
you are done placing them.

## Repeated features

A callout such as `4X Ø5` stands for several features. **Repeated features (4X)** sets how they
are numbered:

| Setting | Result |
|---|---|
| One balloon with quantity | One characteristic with quantity 4 and one number. This is the default. |
| Sub-numbers (5.1, 5.2) | One characteristic per feature, numbered 5.1, 5.2, 5.3, 5.4, with stacked balloons. |

With sub-numbers, a callout with a quantity read from the drawing becomes one characteristic per
feature when you accept it. Every strategy keeps those characteristics together. Switching back to
**One balloon with quantity** gives them plain numbers again; they stay separate characteristics.
If you change the kind or the requirement of one of them, it gets a plain number of its own.
While numbering is locked, and for more than 100 features, a callout stays one characteristic
with its quantity.

## Locked numbering

After an export as issued the numbering is locked ([Exports](exports.md)). Numbers then never
change and **Apply numbering** is not offered. **Added while locked** sets the number of a
characteristic you add after that:

| Setting | Example |
|---|---|
| Next free number | After 12 comes 13 (default). |
| Sub-number (12.1) | With balloon 12 selected, the new one is 12.1, then 12.2. |
| Letter suffix (12A) | With balloon 12 selected, the new one is 12A, then 12B. |

Sub-numbers and letters follow the selected balloon, so select the balloon the new feature
belongs to before you place it. With nothing selected they follow the highest number.

A number given while locked is never given again, even if you delete its characteristic.
