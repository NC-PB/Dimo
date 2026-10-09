# Projects

A project holds one drawing, its characteristics and balloons, and the log of every change. It is
saved as one `.dimo` file. The drawing is stored inside the file unchanged, so the project still
opens when the original PDF is moved or deleted.

## Create, open, save

| Action | Toolbar | macOS | Windows and Linux |
|---|---|---|---|
| New project from a PDF drawing | New project | ⌘N | Ctrl+N |
| Open a project | Open project | ⌘O | Ctrl+O |
| Save | Save | ⌘S | Ctrl+S |
| Save under a new name | Save as | ⇧⌘S | Ctrl+Shift+S |
| Undo | ↶ | ⌘Z | Ctrl+Z |
| Redo | ↷ | ⇧⌘Z | Ctrl+Shift+Z or Ctrl+Y |

A new project has no file until you save it; the toolbar shows "Untitled". The first save asks
for a file name. Undo has no limit while the project is open; the history starts fresh when you
open a project again.

When a project has unsaved changes, Dimo asks before it creates or opens another project and
before the window closes: save, don't save, or cancel.

## Autosave and recovery

Every change is written to an autosave journal right away, and at the latest every 30 seconds.
The toolbar shows the state: "Saved", "Unsaved changes, autosaved", or a warning if the journal
could not be written.

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
