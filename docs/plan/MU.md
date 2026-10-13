# MU Self-explanatory UI

Goal: before release 0.1, the UI is settled. A shop floor user who knows common office software
finds every M0 to M3 feature without help and knows what a button does before clicking it. The
look belongs to the same family as cnc-master.xyz and peterbuilds.dev.

Mockups: the design canvas "Dimo UI redesign" (owner's claude.ai artifacts). Research: general UI
sources only (Microsoft ribbon and command guidance, Fluent 2 toolbar guidance, Nielsen Norman
Group, ISO 9241-110, WCAG 2.2). No ballooning or inspection product was looked at (rule 1).

**Exit criterion:** a shop floor person who has not used Dimo does one real job (drawing to
issued report, at least 10 characteristics, 2 parts) while the owner watches without helping.
Met when the person finishes, every point where they hesitated longer than about 10 seconds is
recorded, and every recorded point is either fixed or logged as a decision. Procedure in
`docs/perf/MU-exit.md` (TU.9). The M3 exit run (`docs/perf/M3-exit.md`) follows this milestone
on the new UI.

Requirements in scope: NFR-UX-01 to NFR-UX-06, D-50 (changed, see decision 1), D-51, D-52,
FR-TOL-01 (general tolerance applied to existing characteristics), FR-BAL-01 and FR-BAL-02 (tool
behaviour).

Out of scope: new features, the Review view content (M4), touch or tablet layout, new languages,
changes to Rust business logic beyond the small commands named below.

---

## Decisions

Owner decisions of 2026-10-13 unless marked otherwise.

1. **Office-style shell.** Title bar (brand mark, Save, Undo, Redo, document name with saved
   state, light/dark switch, help), a tab bar and one ribbon row, the work area, a properties
   pane on the right, a status bar with sheet tabs, active tool, general tolerance and zoom.
   Tabs: **File** (full page), **Home**, **Drawing**, **Review**, **Measure**, **Report**,
   **View**. Review stays as the existing placeholder (shown, says it comes in a later version).
   This replaces the D-50 view list (Drawing, Review, Measure, Export, Settings): a new ADR
   supersedes that line (TU.0).
2. **Place balloon stays on** until Esc, right click or Select. While on: the ribbon button is
   highlighted, the cursor is a crosshair, a bar on the drawing says "Placing balloons. Click a
   dimension, or drag a box around it. [Stop (Esc)]", and the status bar names the tool.
3. **Order:** this milestone comes before the owner's M3 exit run.
4. **Project settings move into the project, personal settings into File > Options.** General
   tolerance on the Home and Drawing tabs and in File > Info; sheet, zones, views, decimal rules
   and company tables on the Drawing tab; part, drawing and report header data in File > Info
   and the properties pane. Theme, language, user name and company name go to File > Options.
   The Settings view goes away.
5. **General tolerance applied to existing characteristics.** After the general tolerance
   changes, Dimo offers "n characteristics have no tolerance yet. Give them the general
   tolerance now? [Update n] [Not now]". Update re-reads only characteristics whose rule is
   "no tolerance defined" or a general tolerance; hand entered limits never change. One undo
   step. "Update limits" on the Home tab does the same for the selection.
6. **Setup pane after opening a drawing.** A right pane asks for the general tolerance (large
   choices: none, ISO 2768-1 f, m, c, v), unit and scale, and part number and customer
   (optional). Skip is always possible; the Drawing tab holds the same settings later.
7. **Brand family (owner request).** Colors from the cnc-master site tokens: light ground
   `#f5f6f8`, raised `#ffffff`, sunken `#e6e9ed`, text `#1a222c` / `#46545f`, borders
   `#dee3e9` / `#c3ccd5`; dark ground `#0c1016`, raised `#141a23`, sunken `#080b0f`, text
   `#e7ecf2` / `#a8b4c1`, borders `#232d3a` / `#35414f`. One accent, orange: `#e85d04` (light)
   and `#ff7a1a` (dark) for marks and focus; filled buttons use `#c2410c` with white text (light)
   and `#ff8f40` with dark text `#1a1004` (dark), so text contrast stays at least 4.5:1. Pass,
   fail and not evaluated keep green, red and grey plus their shapes, never orange. The drawing
   sheet stays paper colored in both themes. Headings in Space Grotesk, values in a monospace
   face. Brand mark: the cnc-master rounded square with crosshair and orange dot, next to the
   word Dimo. Light, dark and "follow the system" as today (D-51).
8. **Fonts are bundled, never loaded from the network** (rule 9). Space Grotesk and the
   monospace face need an OFL or similar license and an entry in `data/fonts/README.md`.
   **Owner: confirm** that OFL fonts count as "similar" under rule 10 (Open Sans, already
   shipped, is Apache 2.0).
9. **Icons:** one open source icon set with a permissive license (MIT, ISC or similar), pinned,
   at least 14 days old, chosen in TU.3. Every command shows an icon and a text label; icon only
   only for universally known actions (save, undo, redo, zoom, close) and always with a tooltip
   and an accessible name.
10. **Tooltips** name the command, the shortcut, one sentence of what it does and, when
    disabled, why. Commands that do not apply are disabled, not hidden, so the ribbon does not
    jump. Alt shows key letters on tabs and commands (KeyTips) as a later step inside TU.3 if
    time allows.
11. **Sizes:** buttons and other targets at least 32 px high (40 px for primary actions), body
    text 14 px, contrast checked in both themes, no fixed width labels (German text is up to
    twice as long).

---

## Tasks

Waves as in M2 and M3. Plan mode for tasks that touch more than one crate or add a dependency.

### TU.0 ADR for the new shell (wave 1)

- New ADR 0007 "Office-style application shell" superseding the D-50 view list, through the
  `new-adr` skill. Update spec 12 D-50 to point to it.

Acceptance:
- [ ] ADR accepted by the owner

### TU.1 Bug fixes (wave 1)

- Place balloon toggle: clicking the active button returns to Select instead of leaving the
  store on "place" (`BalloonTools.svelte` toggle group maps an empty value to "select"). Test.
- General tolerance applied to existing characteristics (decision 5): a Rust query that counts
  the characteristics an update would change, the prompt in the frontend, the update through the
  existing re-interpret path in one undo step. Tests in `dimo-detect` and `src-tauri`.

Acceptance:
- [ ] Clicking an active Place balloon button stops placing (component test)
- [ ] Changing the general tolerance offers the update; accepting changes only characteristics
      without tolerance or with a general tolerance; undo restores them

### TU.2 Theme tokens and brand (wave 1)

- Replace the colors in `apps/desktop/src/lib/theme/tokens.css` with decision 7, keep the token
  names, add the missing ones (sunken, accent strong, accent soft, hover). Bundle the fonts
  (decision 8). Brand mark component. Light, dark and system stay.
- Contrast check of every token pair in both themes (a test that computes the ratios).

Acceptance:
- [ ] All text and control token pairs reach WCAG AA in both themes (test)
- [ ] Screenshots of every view in light and dark reviewed by the owner

### TU.3 App shell, tabs and ribbon (wave 2, after TU.0 and TU.2)

- Title bar, tab bar, ribbon component (groups, large and small buttons, fields), status bar
  with sheet tabs, active tool, general tolerance and zoom slider. Icon set (decision 9).
- Command registry: one place that lists every command with label, icon, shortcut, tooltip
  text, enabled state and reason; ribbon, tooltips and the cheat sheet read from it (D-52).
- Tab contents as in the mockups: Home (tools, balloon, numbering, tolerance), Drawing (sheet,
  zones and views, tolerances, project), Measure (parts, values, row, show), Report (document,
  options, output), View (zoom, fit, panes, theme).

Acceptance:
- [ ] Every command of M0 to M3 is reachable from a tab or File, with icon, label and tooltip
- [ ] Keyboard: every command has a shortcut or is reachable by Tab; the cheat sheet lists all
- [ ] Component tests for ribbon, disabled reasons and tab switching

### TU.4 File view and Options (wave 2)

- File page: Info (project data, tolerances summary, progress), New, Open, Recent, Save, Save as,
  Export, Print, Close, Options. Recent projects list (stored per user by Rust).
- Options: theme, language, user name, company name, export defaults. The Settings view is
  removed; its project parts move per decision 4.

Acceptance:
- [ ] No project setting is in Options, no personal setting is saved in the project
- [ ] Recent list opens a project in one click

### TU.5 Guidance (wave 2)

- Start screen without a project (new from drawing, open, recent, three steps).
- Setup pane after a new project (decision 6).
- An empty state with a clear next step and a button in every view and pane.
- The placing bar and cursor of decision 2.

Acceptance:
- [ ] Every view without content says what to do next and offers the button for it
- [ ] Setup pane can be skipped and reopened from the Drawing tab

### TU.6 Properties pane (wave 3)

- One right pane that follows the selection: characteristic (requirement, tolerance with
  explanation, inspection, history), part (Measure), report checks (Report), nothing selected
  (project summary). Sections can be folded; the table stays at the bottom of the Drawing view.

Acceptance:
- [ ] Pane content follows the selection in every view; no setting exists in two places

### TU.7 Measure and Report layout (wave 3)

- Measure: the bar above the grid with active row, limits and value; part details in the pane;
  totals in the status bar.
- Report: document choice, options, live preview, Save copy, Print, Issue report with the
  "before you issue" checks in the pane. Print opens the system print dialog through the
  existing open after export path or a print command (decide in task, no new capability beyond
  need).

Acceptance:
- [ ] Report preview shows the document that Save copy writes
- [ ] Measure keyboard flow unchanged (existing tests pass)

### TU.8 Wording and German pass (wave 3)

- One glossary for English and German (shop language, no developer terms), all labels and
  tooltips checked against it, German length check in the ribbon at 1280 px width.
- User guide screenshots and texts updated for the new shell.

Acceptance:
- [ ] Glossary in `docs/dev/translations.md`; no label outside it
- [ ] User guide en and de describe the new shell; smoke test passes twice

### TU.9 Exit test with a shop floor user (wave 4)

- Write `docs/perf/MU-exit.md` (procedure, observation sheet), run it with one person, fix or log
  every finding.

Acceptance:
- [ ] Procedure written and run, findings in STATUS.md
