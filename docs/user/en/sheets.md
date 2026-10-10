# Sheets: rotation, unit and scale

Every sheet of a drawing has its own rotation, unit and scale. You set them in the **Sheet
properties** panel on the right. Each change can be undone and redone like any other change.
The zone grid and the views of a sheet are set in the same panel; they are described in
[Numbering strategies, zones and views](numbering.md).

## Rotation

Rotate a sheet in steps of 90 degrees when a drawing was scanned or exported sideways.

| Action | Panel | Key |
|---|---|---|
| Rotate right (clockwise) | ↻ | R |
| Rotate left (counterclockwise) | ↺ | Shift+R |

The keys R and Shift+R work while the table and text fields do not have the focus.

Rotation only changes how the sheet is shown. Balloons and regions keep their place on the
drawing, and exports are not affected. The view keeps the point in the middle of the window where
it is while the sheet turns. The rotation is saved with the project.

## Unit

Choose **Millimeter (mm)** or **Inch (in)**. This is the unit of the dimensions printed on the
sheet. Dimo never guesses it from the sheet size: a metric drawing on an inch format is common,
so the unit is always your choice. A new sheet starts with millimeter. A nominal value
you type in the table without a unit gets the unit of its sheet (degrees for angles).

## Scale

Choose the scale printed in the title block, for example 1:1, 2:1 or 1:2. The first number is the
length on the drawing, the second is the length on the part: 2:1 enlarges, 1:2 reduces. For a
scale that is not in the list choose **Custom** and enter two whole numbers from 1 to 1,000,000.
A new sheet starts with 1:1. Unit and scale are saved with the project.
