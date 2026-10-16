# Projects

A project holds one drawing, its characteristics and balloons, and the log of every change. It is
saved as one `.dimo` file. The drawing is stored inside the file unchanged, so the project still
opens when the original PDF is moved or deleted.

## Create, open, save

| Action | Where | macOS | Windows and Linux |
|---|---|---|---|
| New project from a PDF drawing | File > New project | ⌘N | Ctrl+N |
| Open a project | File > Open project | ⌘O | Ctrl+O |
| Save | Save in the title bar, or File | ⌘S | Ctrl+S |
| Save under a new name | File > Save as… | ⇧⌘S | Ctrl+Shift+S |
| Undo | Undo in the title bar | ⌘Z | Ctrl+Z |
| Redo | Redo in the title bar | ⇧⌘Z | Ctrl+Shift+Z or Ctrl+Y |

**New project** and **Open project** ask for a file with the operating system's file dialog. A
project is made from a PDF drawing; other file types are not offered. A new project has no file
until you save it; the title bar shows "Untitled". The first save asks for a file name and adds
`.dimo` if you leave it out. Undo has no limit while the project is open; the history starts fresh
when you open a project again.

Without an open project DimoMark shows the start screen with **New project from drawing**,
**Open project** and the five most recent projects; one click on a recent project opens it. Right after a new project is created, the pane **Set up this drawing** asks for
the general tolerance, unit and scale, part number and customer; **Skip** closes it, and
**Set up drawing** on the **Drawing** tab opens it again. Opened projects do not show it. Views
and panes without content (no characteristics, no parts, no measured values) say what is missing
and offer a button for the next step.

When a project has unsaved changes, DimoMark asks before it creates or opens another project and
before the window closes or DimoMark quits: **Save**, **Don't save** or **Cancel**.

## What is in a project file

One `.dimo` file holds the drawing as you imported it, the characteristics and balloons, the
settings of the project (balloon style, tolerance rules with the company tables you imported,
numbering strategy), the rotation, unit, scale, zone grid and views of each sheet, the numbering
lock, and the change log: every change with the time and the user name set in
**Options** ([File page, options and About](settings.md)). The file is a ZIP archive, so you can look inside it with any ZIP
program, but only DimoMark should write it.

A project saved by an older DimoMark opens, and a note says it was converted; saving writes the
current format. A project written by a newer DimoMark is refused with a message, so that nothing is
lost; update DimoMark to open it.

## Autosave and recovery

Every change is written to an autosave journal right away, and at the latest every 30 seconds.
The title bar shows the state next to the project name: "saved", "not saved yet" (a new project
without changes), "not saved", "not saved, autosaved", or "Autosave failed" with the reason and a
warning sign if the journal could not be written.

- A saved project keeps its journal next to the file: `part.dimo.journal`. Saving merges it into
  the project file and deletes it.
- A project that was never saved keeps its journal in the DimoMark data folder (for example
  `~/Library/Application Support/io.github.nc-pb.dimo/autosave` on macOS).

If DimoMark or the computer stops unexpectedly, nothing older than 30 seconds is lost:

- Opening the saved project again restores the changes from its journal.
- A project that was never saved is restored the next time DimoMark starts.

A note above the drawing tells you what was restored. Save the project to keep the restored
changes. If you choose "Don't save" when closing, the journal is deleted.

## One window per project

While a project is open, DimoMark keeps a lock file next to it (`part.dimo.lock`). A second DimoMark
window that tries to open the same project gets a message naming the user who has it open. The
lock ends when DimoMark closes, also after a crash, so a leftover lock file never blocks you. The lock
only coordinates DimoMark windows; other programs can still change the file.
