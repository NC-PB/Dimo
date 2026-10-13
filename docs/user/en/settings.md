# File page, options and About

The **File** tab opens a full page. On the left are the project commands and the panes of the
page, on the right the pane you chose. **Back** (or any other tab) returns to where you were.

| Item | What it does |
|---|---|
| **Info** | The data saved with the project, see below |
| **New project** | Starts a project from a PDF drawing (⌘N) |
| **Open project** | Shows the recent projects and **Open project** for any other file (⌘O) |
| **Save**, **Save as…** | Save the project ([Projects](projects.md)) |
| **Export** | Opens the **Report** tab ([Exports](exports.md)) |
| **Print** | Not in this version: save a copy on the Report tab and print it from your PDF viewer |
| **Close project** | Closes the project and keeps Dimo open. Dimo asks first if there are unsaved changes |
| **About Dimo** | Version and licenses, see below |
| **Options** | Your personal settings (⌘,), see below |

Settings have one home each. Settings of a project are saved in the project file and travel with
it: the general tolerance (Home and Drawing tabs), the other tolerance rules and the balloon style
(Drawing tab), and the part, drawing and report data (File > Info and the side panel). Your
personal settings are under **Options** and are never saved in a project.

## Info

**Project info** shows what is saved with the open project. Every change is one step of undo.

- **Part**: part number, part name, customer and order reference.
- **Tolerances**: the general tolerance, the decimal place rules and the company tables of the
  project. **Change on the Drawing tab** takes you to the place where you change them.
- **Drawing**: the drawing file with its number of sheets, the revision, and the SHA-256
  fingerprint of the file with **Copy**.
- **Progress**: the number of characteristics and of characteristics to inspect, how many parts
  have a result for every characteristic, the overall result, and the last report exported as
  issued.
- **Report**: report number, company, inspected by, approved by and the report date (**Today**
  fills in the date). Details: [Project and report details](project-details.md).

Without an open project, Info says so and offers **New project** and **Open project**.

## Recent projects

**Open project** on the File page lists the projects you opened or saved under a new name last,
newest first, at most ten. One click opens a project; with unsaved changes Dimo asks first. A
project whose file was moved, renamed or is on a drive that is not connected is shown greyed and
crossed out with "File not found". Remove an entry with ✕ (the file itself is not touched).

Dimo keeps only the paths of the files, in a small file next to the settings in your user's Dimo
configuration folder. Nothing leaves your computer.

## Options

Open **Options** at the bottom of the File page or with ⌘, (Ctrl+, on Windows and Linux).
Changes apply at once and are kept for the next start. These settings belong to you, not to a
project: they apply to every project and are stored in a small settings file in your user's Dimo
configuration folder.

### Appearance

- **Theme**: same as the system, light or dark. The moon or sun button in the title bar switches
  between light and dark too.
- **Language**: same as the system, English or German. The window reloads when the language
  changes; the open project stays open.

### You and your company

- **User name**: recorded with every change of a project. Leave it empty to use the user name
  of the operating system, which the field shows in grey.
- **Company name**: printed on reports. New projects start with it in their report data. Existing
  projects keep theirs ([Project and report details](project-details.md)). A report exported as
  issued fills it into a project whose company name is empty ([Exports](exports.md)).

### Export defaults

The language of reports and column headers, the file format of the check sheet and of the
inspection report, the number of blank result columns, how the PDF carries the balloons, and
whether Dimo opens a file after exporting it. Dimo remembers what you chose last on the Report
tab; you can change it here or there ([Exports](exports.md)).

## Project settings on the Drawing tab

- **General tolerance**: a field on the Home and Drawing tabs ([Tolerances](tolerances.md)).
- **Decimal rules…**: a small window with the decimal place rules and the rounding after unit
  conversion.
- **Company tables**: a small window with the custom tables of the project, **Import table…**
  and the drawing rule.
- **Project balloon style**: a small window with shape, size, line width, outline color and
  leader line of all balloons of the project that have no style of their own. To give single
  balloons their own style, select them in the drawing and use **Style** (S).

Each change in these windows applies at once and is one step of undo. **Close** closes the window.

## About Dimo

**About Dimo** shows the version of Dimo, its license (Apache License 2.0) and the NOTICE file.
It also lists every font and icon set that comes with Dimo (Open Sans, Space Grotesk, JetBrains
Mono and the Lucide icons) with its copyright notice and full license text. The texts are part of
the app, so they show without an internet connection. License texts are shown in their original
English. The licenses of the other open source libraries Dimo is built with are in the file
`THIRD_PARTY_LICENSES` that comes with every release.
