# Entering results

The **Measure** tab (⌘3) is where you enter what you measured on the parts. Rows are the
characteristics to inspect, one column per part (sample). Dimo compares every value with the
limits and marks it pass or fail. Samples, results and the verdicts are saved in the project and
end up in the [inspection report](exports.md).

## Samples

A sample is one part that you measure. A new project has no samples. Type how many parts you
measure in **Number** (1 to 100 at a time) and choose **Add parts** on the Measure tab. A project can hold up to
500 samples. The serials continue from the highest plain number (`1`, `2`, `3`); the date is empty
and the inspector is your user name from **Options**.

Click the header or a cell of a sample column to make it the active sample. The pane on the
right shows its serial, date, inspector, gauges and comment; change them there. Below the fields
it counts the results of that part: pass, not OK, not evaluated and missing. Project and report
details are in **File > Info**. A serial is
trimmed, must not be empty and must be unique in the project (upper and lower case count as
different). Serials may be any text, for example `A-17`. **Move left** and **Move right** change
the column order, which is also the order in the reports. **Delete part** deletes the column;
when the sample has results, Dimo asks first, because they are deleted with it. Every change is
undone with ⌘Z. The tab also has **Paste values** and **Clear cell** for the active cell,
**Result of this row** for the active row, and **Only missing values** and **Only not OK** to
show fewer rows; the row you are in stays shown.

## The grid

Only characteristics with **Inspect** switched on are listed, in number order. Reference and
basic dimensions and rejected characteristics are left out, in the grid as in both reports. The
left columns stay in place when you scroll sideways: number, requirement, nominal, lower and upper
limit, unit, result type and a summary of the row. Limits from a draft tolerance table carry the
draft marker. A characteristic with only an upper or only a lower limit shows the other limit
cell empty.

| Column | Meaning |
|---|---|
| Result type | **Auto** (a value if the characteristic has a limit, else OK / not OK), **Value** or **OK / not OK** |
| Summary | How many results of the row pass, fail, are not evaluated or are missing |
| Sample header | Serial of the part and the same counts for that part |

The line above the grid shows the overall verdict of all parts, and why Dimo did not accept the
last entry. The status bar at the bottom shows the totals of all parts and the overall verdict.

## The Measure bar

The bar above the grid works like the formula bar of a spreadsheet. It names the active cell
(characteristic number and part), shows the requirement with its limits and the tolerance rule,
and has a **Value** field with the value of the active cell. Next to it Dimo says the verdict in
words, for example "pass, on the lower limit" or "not OK, above the upper limit".

You can type a value in the cell or in the **Value** field; both show the same text while you
type. **Enter** in the field stores the value and moves to the next characteristic, as in the
grid; **Shift+Enter** goes up and **Escape** cancels. Clicking the field while you type in a cell
carries on the same entry in the field.

A triangle next to the number means the limits or the result type of that characteristic changed
after the last issued report ([When limits change](#when-limits-change)).

A characteristic with a quantity such as `4X` and one balloon has one result per part: enter the
worst value. If you want one value per feature, number the features separately with
sub-numbers ([Numbering strategies, zones and views](numbering.md)).

## Typing results

The grid is made for measuring one part from top to bottom without the mouse.

1. Click the first cell of the part, or move there with the arrow keys.
2. Type the value. Typing starts the entry; a point or a comma are both fine (`12.05`, `12,05`),
   and the digits you type are kept (`12.10` stays `12.10`). A minus sign can be a hyphen or the
   minus sign `−`.
3. Press **Enter**. Dimo stores the value and moves to the next characteristic.

**Shift+Enter** goes up instead, **Tab** and **Shift+Tab** confirm and go to the next or the
previous part; past the last part Tab leaves the grid for the sample panel. The arrow keys move
without changing anything, **F2** edits the value in the cell, **Delete** clears it and
**Escape** cancels what you are typing. When Dimo does not accept a value (for example letters in
a value cell), the cell opens again and the line above the grid says why. Dimo always stores the
value exactly as typed and never rounds it.

Values are entered in the unit of the characteristic, shown in the **Unit** column. Dimo does not
convert: type millimeters for a millimeter characteristic, inches for an inch characteristic and
decimal degrees for an angle (degrees and minutes are not accepted yet).

### OK / not OK

For a characteristic with the result type **OK / not OK**, press **O** or **+** for OK and **N**
or **X** for not OK; Dimo stores it and moves down. **Space** switches between OK and not OK.

These words are accepted too, in any case: `ok`, `o`, `+`, `io`, `i.O.` for OK and `not ok`,
`not_ok`, `nok`, `n`, `x`, `nio`, `n.i.O.` for not OK. The German `i.O.` and `n.i.O.` therefore
work in an English app and the other way round. Since **O**, **N** and **X** alone already set
the result, press **F2** first to type a longer word such as `n.i.O.`. The same words work in a
pasted block. A number in an OK / not OK cell, or a word in a value cell, is refused.

## Pasting a block

Copy a block of values from a spreadsheet or a measuring program, click the cell where the block
starts and press ⌘V. Rows of the block go down the characteristics, columns go across the parts.
The whole block is one step: ⌘Z removes it again. An empty cell in the block clears the result
in that cell. If one value does not fit, nothing is pasted.
If the block reaches past the last characteristic or the last part, Dimo says so and pastes
nothing; add samples first.

## Marks

Every verdict has its own shape, so the grid can be read without colors and on a black and white
printout. The reports use the same marks.

| Mark | Verdict | Meaning |
|---|---|---|
| ✓ | Pass | Within the limits (a value equal to a limit passes), or OK |
| ✗ with a bold border | Fail | Outside a limit, or not OK |
| ○ | Not evaluated | A value for a characteristic without limits |
| empty cell | Missing | No result yet |

For a whole part, a row or the report there are three verdicts: **fail** if any result fails,
**pass** if every inspected characteristic has a pass, and **open** otherwise (something is
missing or not evaluated, or there are no results at all).

## How Dimo decides

Dimo compares the entered value with the limits as they are now. Nothing about the verdict is
stored, so it always follows the current limits.

- Limits are inclusive: a value equal to a limit passes. The comparison is exact, so `12` and
  `12.000` are the same value. The value is never rounded, and no measurement uncertainty is
  taken into account.
- With only an upper limit (for example `MAX`) or only a lower limit (`MIN`), only that limit is
  checked.
- A value for a characteristic without any limit is **not evaluated** (○). It does not fail the
  part, but the part and the report cannot be **pass** while such a result exists.
- For OK / not OK, OK passes and not OK fails. Limits are ignored.
- No result is **missing**.
- Limits from a draft tolerance table are used like any other limits. The grid and the reports
  mark them as drafts ([Tolerances](tolerances.md)).

## Result type

Dimo picks the result type by itself (**Auto**): a value for characteristics with limits,
OK / not OK for threads, notes and other characteristics without limits. Change it in the
**Result type** column of the grid or in the detail panel of the drawing view. The first result
fixes the type: when you enter the first result of an **Auto** characteristic, Dimo sets the type
to value or OK / not OK, so later changes of the limits do not turn a value characteristic into
an OK / not OK one. Clearing results does not undo that.

Dimo never converts results. While a characteristic has results of one type, it refuses to change
it to the other type, and it also refuses to set **Auto** again. The line above the grid says to
clear the results first. Clear them with **Delete** in each cell, then change the type.

## When limits change

Limits change when you edit the nominal, a deviation or a limit, read a requirement again, change
the general tolerance or the rules of the project, or edit a characteristic in the table.

- The results stay. Dimo does not delete or convert them.
- The verdicts follow the new limits at once, in the grid and in every report you export from then
  on. A value that passed can fail now, and the other way round.
- A characteristic that already has results keeps its result type (see above).
- A report that was exported before is a file, and it does not change. If the new limits matter,
  export the report again, as issued if it goes to a customer.
- A triangle next to the number in the grid shows every characteristic whose limits or result type
  differ from the state at the last issued report. Limits count as different when the value or
  the way it is written differs (`12` against `12.0`). The marker is gone when the change is
  undone or the old limits are entered again. A characteristic that you added after the last issued
  report is not marked. Before the first issued report, nothing is marked. Dimo also warns about
  such changes before the next issued export ([Exports](exports.md)).
- If you delete a characteristic, its results are deleted in the same step, without a question,
  and ⌘Z brings both back. Deleting a sample asks first when it has results. Deleting characteristics that have results asks first too: Dimo says how many results go with them, and ⌘Z brings them back.
