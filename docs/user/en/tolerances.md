# Tolerances

Dimo works out the upper and lower limit of every dimension it reads, records which rule gave
them, and explains that rule in words. This page describes the rules, the tolerance settings of a
project, the markers in the table, and the change history of a characteristic.

## Where limits come from

Dimo tries these rules in this order and takes the first one that gives limits:

1. **Explicit**: a tolerance written on the drawing, such as `±0.1`, `+0.2 -0.1`, two limits, `MIN`
   or `MAX`. A written tolerance always wins.
2. **Fit table**: a fit such as `H7` without written deviations, expanded with the ISO 286 table.
3. **Drawing rule**: a custom table of the project chosen as drawing rule.
4. **General tolerance**: the general tolerance standard and class of the project, by size range.
5. **Decimal places rule**: a tolerance for the number of decimal places the size is written with.

If no rule applies, the characteristic gets the rule **No tolerance defined** and no limits; check
it and type its limits in the table. Limits you type or change yourself get the rule **Entered by
hand**. Dimo never changes them again on its own.

When a fit is written together with deviations that differ from the fit table, the written
deviations are used and the characteristic gets a note.

## Tolerance settings of the project

Open **Settings** (⌘,) while a project is open. The section **Tolerances of the project** holds
the rules of this project. They are stored in the project file, and every change is one undo step.

- **General tolerance**: the table and class for dimensions without a tolerance, for example
  ISO 2768-1 class m. Fit tables are not offered here.
- **Drawing rule (custom table)**: a custom table of the project that applies before the general
  tolerance.
- **Decimal place rules**: one tolerance per number of decimal places, for example 2 places ±0.05.
  Type the places and the tolerance and choose **Add rule**. Each number of places has one rule.
- **Rounding after unit conversion**: decimal places for values converted to mm or inch.
- **Custom tables**: **Import table…** reads a table file of your own (TOML, in the format of the
  tables shipped with Dimo). Dimo checks the whole file before it is used. If it is refused, the
  reason is shown with the line of the file where Dimo can tell it. An imported table is copied
  into the project, so the project does not need the file afterwards, and it is kept after a
  crash even before you save. Importing a table with the same id again replaces it, and the
  settings that use it switch to the new version.

Changing the settings does not change the limits of characteristics that are already in the
project. New characteristics, box select and typed values use the new settings.

## Re-interpret after a change

To apply new settings to existing characteristics, select them (in the table or on the drawing)
and choose **Re-interpret selected** in the side panel. Dimo reads the requirement text of each
one again with the current settings and sets nominal, unit, deviations, limits, fit and rule. All
of it is one step of undo.

- Characteristics with limits entered by hand are kept as they are.
- Characteristics whose text is empty or is not a dimension are left unchanged.
- Kind, quantity and the **Inspect** box stay as you set them. A characteristic whose text was
  never read before gets them from the text.

The line under the button says how many were read again, kept or not readable.

## Rule, explanation and markers

The **Rule** column of the characteristic table shows the rule of each characteristic. Select a
characteristic to see its rule, its limits and the explanation in the side panel, tab
**Tolerance**. The explanation names the table, the class and the size range the value came from,
in the language of the app. The rule is also exported in the CSV and Excel list
([Exports](exports.md)).

Markers next to the rule. Each one has its own shape, so they can be told apart without color:

| Marker | Meaning |
|---|---|
| **Draft** in a dashed box | The limits come from a table whose values are not verified yet |
| Circle with a bar | No tolerance defined: no limits. Check this characteristic |
| Triangle with ! | A note, for example written deviations that differ from the fit table. Point at it to read the note |
| **(Ref)** with round ends | Reference dimension: no limits |
| **Basic** in a square frame | Basic (theoretically exact) dimension: no limits |

## Draft tables

The tables shipped with Dimo are drafts until their values have been checked against the printed
standard. Limits from a draft table work like any other but carry the **Draft** marker in the
table, in the side panel and on the box select card. A custom table you import can be a draft too.

## Reference and basic dimensions

Reference dimensions (in parentheses or with `REF`) and basic dimensions (in a frame) get no
limits and are not inspected: their **Inspect** box starts cleared. You can tick it in the table
or in the side panel if you want to measure them anyway; re-interpreting keeps your choice.

## History of a characteristic

The tab **History** in the side panel lists every change of the selected characteristic, newest
first, from the change log of the project. Each entry shows when, who, and the source of the
change:

- **By hand**: you changed this characteristic.
- **By rule**: a rule changed it, for example renumbering after a delete or a re-interpretation.
- **Recognition**: it came from an accepted box select card.

The entry says which values changed, or that the characteristic was created or removed. Undo and
redo are listed as entries of their own.
