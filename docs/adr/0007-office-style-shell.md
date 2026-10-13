# ADR 0007: Office-style application shell

Status: accepted (owner, 2026-10-13)

## Context
D-50 fixed the main window as five views (Drawing, Review, Measure, Export, Settings) switched by
a view store. Testing M3 the owner found that the UI lacks a common thread: shop floor users do
not know what to expect from a button, the general tolerance of a project is buried in Settings,
and Settings mixes project data with personal preferences. Most users know common office
software. General UI guidance (Microsoft ribbon and command guidance, Nielsen Norman Group, ISO
9241-110) recommends a familiar layout, commands with icon and label, document settings in the
document and personal settings in an options page. NFR-UX-01 to NFR-UX-06 apply. The owner
approved mockups on 2026-10-13 (M3 decisions 18 to 29).

## Decision
- One main window, still without a router library, with an office-style frame: title bar (brand
  mark, Save, Undo, Redo, document name and saved state, light/dark switch, help), a tab bar with
  one ribbon row, the work area, a properties pane on the right and a status bar (sheet tabs,
  active tool, general tolerance, zoom).
- Tabs: File (a full page with Info, New, Open, Recent, Save, Save as, Export, Print, Close,
  About and Options), Home, Drawing, Review (placeholder until M4), Measure, Report, View. A view
  store still switches the work area.
- Project settings live on the tabs, in File > Info and in the properties pane. Personal
  settings live in File > Options. The Settings view goes away; Export becomes the Report tab.
- Every command is defined once (label, icon, shortcut, tooltip, enabled state and reason) and
  the ribbon, tooltips and the cheat sheet read from that list (D-52 stays).

## Consequences
- Users find commands where office software has them; settings have one home each.
- The frontend shell is rebuilt (M3 tasks T3.15 to T3.19); Rust business logic is unchanged.
- An icon set and two bundled fonts become frontend assets, with their licenses shown in About.
- The ribbon takes more vertical space than today's toolbar; the drawing area gets smaller on
  small laptop screens. The View tab can hide panes to give the drawing more room.
- Supersedes the view list of D-50 in `docs/spec/12-implementation-defaults.md` (updated in the
  same change to point here). No earlier ADR is superseded.
