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

When a project has unsaved changes, Dimo asks before it creates or opens another project and
before the window closes or Dimo quits: **Save**, **Don't save** or **Cancel**.

## What is in a project file

One `.dimo` file holds the drawing as you imported it, the characteristics and balloons, the
settings of the project (balloon style, tolerance rules with the custom tables you imported,
numbering strategy), the rotation, unit, scale, zone grid and views of each sheet, the numbering
lock, and the change log: every change with the time and the user name set in
[Settings](settings.md). The file is a ZIP archive, so you can look inside it with any ZIP
program, but only Dimo should write it.

A project saved by an older Dimo opens, and a note says it was converted; saving writes the
current format. A project written by a newer Dimo is refused with a message, so that nothing is
lost; update Dimo to open it.

## Autosave and recovery

Every change is written to an autosave journal right away, and at the latest every 30 seconds.
The title bar shows the state next to the project name: "saved", "not saved", "not saved,
autosaved", or "autosave failed" with a warning sign if the journal could not be written.

- A saved project keeps its journal next to the file: `part.dimo.journal`. Saving merges it into
  the project file and deletes it.
- A project that was never saved keeps its journal in the Dimo data folder (for example
  `~/Library/Application Support/io.github.nc-pb.dimo/autosave` on macOS).

If Dimo or the computer stops unexpectedly, nothing older than 30 seconds is lost:

- Opening the saved project again restores the changes from its journal.
- A project that was never saved is restored the next time Dimo starts.

A note above the drawing tells you what was restored. Save the project to keep the restored
changes. If you choose "Don't save" when closing, the journal is deleted.

## One window per project

While a project is open, Dimo keeps a lock file next to it (`part.dimo.lock`). A second Dimo
window that tries to open the same project gets a message naming the user who has it open. The
lock ends when Dimo closes, also after a crash, so a leftover lock file never blocks you. The lock
only coordinates Dimo windows; other programs can still change the file.
