# File page, options and About

The **File** tab opens a full page. On the left are the project commands and the panes of the
page, on the right the pane you chose. **Back** (or any other tab) returns to where you were.

| Item | What it does |
|---|---|
| **Info** | The data saved with the project, see below |
| **New project** | Starts a project from a PDF drawing, a photo or a scan (⌘N) |
| **Open project** | Shows the recent projects and **Open project** for any other file (⌘O) |
| **Save**, **Save as…** | Save the project ([Projects](projects.md)) |
| **Export** | Opens the **Report** tab ([Exports](exports.md)) |
| **Print** | Opens the report chosen on the Report tab in your PDF viewer, where you print it ([Exports](exports.md)) |
| **Close project** | Closes the project and keeps DimoMark open. DimoMark asks first if there are unsaved changes |
| **About DimoMark** | Version and licenses, see below |
| **Options** | Your personal settings (⌘,), see below |

The version of DimoMark stands at the bottom of the list on the left.

Settings have one home each. Settings of a project are saved in the project file and travel with
it: the general tolerance (Home and Drawing tabs), the other tolerance rules and the balloon style
(Drawing tab), and the part, drawing and report data (File > Info). Your
personal settings are under **Options** and are never saved in a project.

## Info

**Project info** shows what is saved with the open project. Every change is one step of undo.

- **Part**: part number, part name, customer and order reference.
- **Tolerances**: the **General tolerance** field (the same as on the Home and Drawing tabs),
  and the decimal place rules and company tables of the project. **Change on the Drawing tab**
  takes you to the place where you change the rules.
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
newest first, at most ten. One click opens a project; with unsaved changes DimoMark asks first. A
project whose file was moved, renamed or is on a drive that is not connected is shown greyed and
crossed out with "File not found". Remove an entry with ✕ (the file itself is not touched).

DimoMark keeps only the paths of the files, in a small file next to the settings in your user's DimoMark
configuration folder. Nothing leaves your computer.

## Options

Open **Options** at the bottom of the File page or with ⌘, (Ctrl+, on Windows and Linux).
Changes apply at once and are kept for the next start. These settings belong to you, not to a
project: they apply to every project and are stored in a small settings file in your user's DimoMark
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
whether DimoMark opens a file after exporting it. DimoMark remembers what you chose last on the Report
tab; you can change it here or there ([Exports](exports.md)).

## Project settings on the Drawing tab

- **General tolerance**: a field on the Home and Drawing tabs and in File > Info
  ([Tolerances](tolerances.md)).
- **Decimal rules…**: a small window with the decimal place rules and the rounding after unit
  conversion.
- **Company tables**: a small window with the company tables of the project, **Import table…**
  and the drawing rule.
- **Project balloon style**: a small window with shape, size, line width, outline color and
  leader line of all balloons of the project that have no style of their own. To give single
  balloons their own style, select them in the drawing and use **Style** (S).

Each change in these windows applies at once and is one step of undo. **Close** closes the window.

## About DimoMark

**About DimoMark** shows the version of DimoMark, its license (Apache License 2.0) and the NOTICE file.
It also lists every font and icon set that comes with DimoMark (Open Sans, Space Grotesk, JetBrains
Mono and the Lucide icons) with its copyright notice and full license text. The texts are part of
the app, so they show without an internet connection. License texts are shown in their original
English.

Under **Open source libraries** follows the list of all other parts DimoMark is built with: the Rust
crates of the program, the packages of the user interface and the PDF library PDFium. Each is listed
with its version and copyright notice, grouped by license, and each license text is shown once.
The last part lists the system libraries that the Linux AppImage brings with it (GTK, WebKitGTK and
others from Ubuntu 22.04), each with its Debian copyright file; the other builds do not contain
them.
**Copy** puts the whole list on the clipboard. The same list comes with every installer as the file
`THIRD_PARTY_LICENSES` in the `licenses` folder of the installed app; the portable build has it next
to `DimoMark.exe`.

## Log file

DimoMark writes messages such as warnings and errors (for example a failed autosave or export, or a
settings file that cannot be read) and the version it started with to a log file on your computer.
It holds messages and file paths, not the content of drawings or projects, and nothing is sent
anywhere. When it reaches 1 MB it is renamed to `dimo.log.1` and a new file starts. If you report
a problem, the log helps to find the cause:

- macOS: `~/Library/Logs/io.github.nc-pb.dimo/dimo.log`
- Windows: `%LOCALAPPDATA%\io.github.nc-pb.dimo\logs\dimo.log`
- Linux: `~/.local/share/io.github.nc-pb.dimo/logs/dimo.log`

**Copy diagnostics** in **About DimoMark** puts a short text on the clipboard for a bug report: the
version of DimoMark, the operating system, your options and the last 200 lines of the log file. Your
name, the company name and your home folder are replaced (`<user>`, `<company>`, `~`). Nothing is
sent; paste the text where you report the problem.

## Check for update

DimoMark never looks for updates by itself. **Check for update** in **About DimoMark** asks GitHub
once for the latest published version, only when you click it. The request goes to
`api.github.com` and carries nothing but what the request itself needs: the DimoMark version as
the program name (`DimoMark/0.1.0`). Nothing is downloaded or installed.

- A newer version: DimoMark shows its number and **Open release page** opens the page in your
  browser. Download and install it yourself, as you did the first time.
- The same version: "You have the latest version".
- No internet connection, or GitHub cannot be reached: a message, nothing else happens.

DimoMark uses a proxy server if one of the usual environment variables is set (`ALL_PROXY`,
`HTTPS_PROXY`, `HTTP_PROXY`, also in lower case; `NO_PROXY` lists hosts without proxy). A network
that inspects HTTPS with its own certificate makes the check fail: DimoMark trusts only the root
certificates built into it.

### Turning the check off for an organization

An administrator can hide the button for every user of a computer with a policy file:

- macOS: `/Library/Application Support/DimoMark/policy.json`
- Windows: `%ProgramData%\DimoMark\policy.json` (usually `C:\ProgramData\DimoMark\policy.json`)
- Linux: `/etc/dimomark/policy.json`

The file contains:

```json
{ "update_check": false }
```

`true` or a file without `update_check` shows the button. A file that is not valid JSON also hides
the button, and the reason is written to the log file. DimoMark reads the file when **About
DimoMark** opens and again before the request.
