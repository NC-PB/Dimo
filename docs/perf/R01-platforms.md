# Release 0.1: platform tests

> Status: **to test by the owner.** This page is the checklist and the empty result tables for
> R.12 (Windows specifics, D-10, D-11, NFR-PORT-02). R.14 fills in the results and adds the
> macOS and Linux runs.

Builds under test: the MSI (`DimoMark_<version>_x64_en-US.msi`) and the portable zip
(`DimoMark_<version>_x64_portable.zip`) of the same release candidate. Test each on:

- Windows 10 22H2, x64
- Windows 11 (current release), x64

Use a normal user account without administrator rights unless a step says otherwise. Use a
drawing from the corpus or one of your own.

## What the agent checked before

- CI (windows-2025): `tests/paths.rs` creates, saves, changes, reopens and exports a project in
  `Zeichnungen Müller\Kunde Ärger & Söhne`, in a folder with paths over 260 characters, and on
  `\\localhost\C$\...` if that share exists on the runner (otherwise the test prints SKIPPED).
- CI (windows-2025): unit tests for UNC paths in the recent list, the journal and lock paths,
  atomic writes and the opener.
- The WebView2 check finds the runtime on the runner. The message itself is only shown on a PC
  without WebView2, so step P2 below is the only test of it.

## Checklist

### MSI installer

| # | Step | Expected |
|---|---|---|
| M1 | Run the MSI (needs administrator rights). | Installs without error. On a PC without WebView2 the bootstrapper downloads and installs it. |
| M2 | Start DimoMark from the start menu. | The start screen appears, no console window. |
| M3 | About pane. | Version matches the MSI, license list opens. |
| M4 | Uninstall in Settings > Apps. | DimoMark is removed; projects and settings of the user stay. |

### Portable zip

| # | Step | Expected |
|---|---|---|
| P1 | Unzip into a folder of your user (for example `Downloads\DimoMark`), start `DimoMark.exe`. | Starts without administrator rights, start screen appears. |
| P2 | On a PC or VM without WebView2 (Windows 10 without Edge updates, or with the runtime uninstalled), start `DimoMark.exe`. | A message box "DimoMark: WebView2 missing / WebView2 fehlt" in English and German with the download page and the MSI hint. OK closes DimoMark; no crash dialog, no process left in the task manager. |
| P3 | Move `DimoMark.exe` without the `pdfium` folder and start it. | Starts; opening a drawing fails with a message (PDFium missing), no crash. |

### Paths (both builds)

| # | Step | Expected |
|---|---|---|
| F1 | New project from a drawing stored in `Zeichnungen Müller\Prüfzeichnung 1.pdf`, save as `Zeichnungen Müller\Prüfteil 1.dimo`. | Title bar and recent list show `Prüfteil 1.dimo` with the umlauts. |
| F2 | Close DimoMark, start again, open the project from the recent list. | Opens; the path in the list is the one you chose (no `\\?\`). |
| F3 | Export check sheet, inspection report, ballooned PDF, CSV and Excel into the same folder, open each with **Open after export**. | Every file is written and opens. |
| F4 | A folder path longer than 260 characters (nest folders in Explorer), repeat F1 to F3. | Save, open and export work. If the file dialog or opening the export fails, note it, switch on "Enable Win32 long paths" (gpedit or registry, administrator) and try again. |
| F5 | A network drive with a letter (`Z:\`), repeat F1 to F3. | Works; the recent list shows `Z:\...`. |
| F6 | The same share through its network path (`\\server\share\...`, type it into the file dialog), repeat F1 to F3. | Works; the recent list shows `\\server\share\...`, no `\\?\UNC\`. |
| F7 | On the share: make a change, wait 30 seconds, pull the network cable or switch off Wi-Fi, make another change, then save. | "Autosave failed" in the title bar, saving fails with a message, the window keeps the work. **Save as** onto the local disk works. |
| F8 | Open the same project on the share from a second PC while the first has it open. | The second PC gets the "in use" message with the user name of the first. |

## Results

| Step | Windows 10 22H2 | Windows 11 | Notes |
|---|---|---|---|
| M1 | | | |
| M2 | | | |
| M3 | | | |
| M4 | | | |
| P1 | | | |
| P2 | | | |
| P3 | | | |
| F1 | | | |
| F2 | | | |
| F3 | | | |
| F4 | | | |
| F5 | | | |
| F6 | | | |
| F7 | | | |
| F8 | | | |
