# Entering results

The **Measure** view (⌘3) is where you enter what you measured on the parts. Rows are the
characteristics to inspect, one column per part (sample). Dimo compares every value with the
limits and marks it pass or fail.

## Samples

A new project has no samples. Type how many parts you measure in **Number** and choose **Add
samples**. The serials continue from the highest plain number (`1`, `2`, `3`); the inspector is
your user name from the settings.

Click the header or a cell of a sample column to make it the active sample. The panel on the
right shows its serial, date, inspector, gauges and comment; change them there. A serial must not
be empty and must be unique in the project. **Move left** and **Move right** change the column
order. **Delete sample** deletes the column; when the sample has results, Dimo asks first,
because they are deleted with it. Every change is undone with ⌘Z.

## The grid

Only characteristics with **Inspect** switched on are listed, in number order. Reference and
basic dimensions and rejected characteristics are left out. The left columns stay in place when
you scroll sideways: number, requirement, nominal, lower and upper limit, unit, result type and a
summary of the row. Limits from a draft tolerance table carry the draft marker.

| Column | Meaning |
|---|---|
| Result type | **Auto** (a value if the characteristic has a limit, else OK / not OK), **Value** or **OK / not OK** |
| Summary | How many results of the row pass, fail, are not evaluated or are missing |
| Sample header | Serial of the part and the same counts for that part |

The line above the grid shows the overall verdict of all parts, and why Dimo did not accept the
last entry.

A triangle next to the number means the limits or the result type of that characteristic changed
after the last issued report. The results stay; their verdicts follow the new limits.

## Typing results

The grid is made for measuring one part from top to bottom without the mouse.

1. Click the first cell of the part, or move there with the arrow keys.
2. Type the value. Typing starts the entry; a point or a comma are both fine (`12.05`, `12,05`),
   and the digits you type are kept (`12.10` stays `12.10`).
3. Press **Enter**. Dimo stores the value and moves to the next characteristic.

**Shift+Enter** goes up instead, **Tab** and **Shift+Tab** confirm and go to the next or the
previous part. The arrow keys move without changing anything, **F2** edits the value in the cell,
**Delete** clears it and **Escape** cancels what you are typing. When Dimo does not accept a value
(for example letters in a value cell), the cell opens again and the line above the grid says
why.

For a characteristic with the result type **OK / not OK**, press **O** or **+** for OK and **N**
or **X** for not OK; Dimo stores it and moves down. **Space** switches between OK and not OK. You
can also type the words `ok`, `not ok`, `i.O.` or `n.i.O.`.

Values are entered in the unit of the characteristic, shown in the **Unit** column.

## Pasting a block

Copy a block of values from a spreadsheet or a measuring program, click the cell where the block
starts and press ⌘V. Rows of the block go down the characteristics, columns go across the parts.
The whole block is one step: ⌘Z removes it again. If one value does not fit, nothing is pasted.
If the block reaches past the last characteristic or the last part, Dimo says so and pastes
nothing; add samples first.

## Marks

Every verdict has its own shape, so the grid can be read without colors and on a black and white
printout.

| Mark | Verdict | Meaning |
|---|---|---|
| ✓ | Pass | Within the limits (a value equal to a limit passes), or OK |
| ✗ with a bold border | Fail | Outside a limit, or not OK |
| ○ | Not evaluated | A value for a characteristic without limits |
| empty cell | Missing | No result yet |

A part passes when every characteristic passes, fails when one fails, and is open otherwise.

## Result type

Dimo picks the result type by itself (**Auto**): a value for characteristics with limits,
OK / not OK for threads, notes and other characteristics without limits. Change it in the
**Result type** column of the grid or in the detail panel of the drawing view. The first result
fixes the type, so later changes of the limits do not turn a value characteristic into an OK / not
OK one. While a characteristic has results of one type, Dimo does not change it to the other type;
clear the results first.
