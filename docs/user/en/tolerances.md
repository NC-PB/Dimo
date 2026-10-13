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

### Angles: the shorter leg

The general tolerance for an angle (ISO 2768-1) depends on the length of its shorter leg, which is
not written on the drawing. For an angle without a written tolerance, type the length of the
shorter leg in the field **Shorter leg** on the proposal card or in the side panel, tab
**Tolerance**. The field is in the unit of the sheet (mm or inch). Dimo then sets the limits of the
angle, and the explanation names the leg, its range and the limits. Without a leg the angle gets
no limits and a note. The leg is stored with the characteristic and its unit, so **Re-interpret
selected** and a retyped requirement text keep using it, also after the sheet unit changes.
Limits you entered by hand stay. Empty the field to remove the leg.

## Tolerance settings of the project

Open **Settings** (⌘,) while a project is open. The section **Tolerances of the project** holds
the rules of this project. They are stored in the project file, and every change is one undo step.

- **General tolerance**: the table and class for dimensions without a tolerance, for example
  ISO 2768-1 class m. The choice lists the general tables and the custom tables of the
  project; fit tables are not offered here.
- **Drawing rule (custom table)**: a custom table of the project that applies before the general
  tolerance.
- **Decimal place rules**: one tolerance per number of decimal places, for example 2 places ±0.05.
  Type the places and the tolerance and choose **Add rule**. Each number of places has one rule,
  and the tolerance is plus and minus the value you type. A rule applies to sizes written as
  decimals without a tolerance, by the number of decimal places as written (`12.50` has two). You
  can change the tolerance of a rule in its field, and remove a rule with the ✕ button.
- **Rounding after unit conversion**: decimal places for limits that have to be converted to mm
  or inch, for example a size written in inch under a general tolerance, whose table is in mm.
  The defaults are 3 places for mm and 4 for inch. Limits are rounded inward, so they are never
  wider than the table, and the explanation says that a conversion took place. Whole numbers
  from 0 to 10 are accepted.
- **Custom tables**: **Import table…** reads a table file of your own (TOML, in the format of the
  tables shipped with Dimo). Dimo checks the whole file before it is used. If it is refused, the
  reason is shown with the line of the file where Dimo can tell it. The file format is described
  in `data/tolerances/README.md` of the Dimo sources, next to the tables shipped with Dimo. An
  imported table is copied
  into the project, so the project does not need the file afterwards, and it is kept after a
  crash even before you save. Importing a table with the same id again replaces it, and the
  settings that use it switch to the new version.

Changing the settings does not change the limits of characteristics that are already in the
project by itself. New characteristics, box select and typed values use the new settings. After a
change of the general tolerance, Dimo offers to update the existing characteristics it applies
to, see below.

## Update after a change of the general tolerance

When you change the general tolerance or its class, a notice under it says how many
characteristics have no tolerance yet or still have the previous general tolerance, and offers
**Update n** and **Not now**. The notice appears only if at least one characteristic would
change; other settings never show it.

- **Update n** reads exactly these characteristics again with the new general tolerance, as one
  step of undo. **Undo** restores their previous limits.
- **Not now** closes the notice and changes nothing. You can still use **Re-interpret selected**
  later.

The update considers only characteristics whose rule is **No tolerance defined** or a general
tolerance (of a shipped or a custom table), and characteristics whose text was never read.
Limits entered by hand, written tolerances, fits, drawing rules, decimal place rules and texts
that are not a dimension never change. A characteristic that stays without tolerance, for
example a thread or a reference dimension, is not counted. If you remove the general tolerance,
the notice offers to remove the previous general tolerance from the characteristics that still
have it.

## Re-interpret after a change

To apply other new settings to existing characteristics, select them (in the table or on the drawing)
and choose **Re-interpret selected** in the side panel. Dimo reads the requirement text of each
one again with the current settings and sets nominal, unit, deviations, limits, fit and rule. All
of it is one step of undo.

- Characteristics with limits entered by hand are kept as they are.
- Characteristics whose text is empty or is not a dimension are left unchanged.
- Kind, quantity and the **Inspect** box stay as you set them. A characteristic whose text was
  never read before gets them from the text.
- If you changed the **Requirement** text in the table, re-interpreting reads the new text.

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

Some notes are only shown on the card of a box select and are not stored: a fit pair such as
`H7/g6` names a hole and a shaft, so Dimo gives no limits and asks you to capture them as two
characteristics. The other notes of the table are stored with the characteristic:

- The written deviations differ from the fit table; the written ones are used.
- The fit is not in the fit table, so there are no limits from it.
- The size is outside the ranges of a table, so that rule gave no limits.

## Limits you enter yourself

When you type a nominal value, a deviation or a limit in the table and the limits change because
of it, the rule becomes **Entered by hand**. These limits are kept when you re-interpret, and
the explanation only says that they were entered by hand. If you delete every deviation and limit, the rule is
removed and the characteristic can be read again.

## Draft tables

The tables shipped with Dimo are drafts until their values have been checked against the printed
standard. Limits from a draft table work like any other but carry the **Draft** marker in the
table, in the side panel and on the box select card. A custom table you import can be a draft too.
The settings show the marker next to a table that is a draft. Check limits from a draft table
against your own standard before you rely on them. A table stops being a draft only when its
file is marked as verified by the person who checked each value; Dimo never changes that itself.

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
