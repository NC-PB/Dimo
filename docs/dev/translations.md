# Adding a UI language

Dimo ships English and German and is built so that more UI languages can be added without code
changes beyond one table entry (FR-SET-04). Every visible string of the app goes through
[Paraglide](https://inlang.com/m/gerre34r/library-inlang-paraglideJs) messages.

## Where the texts live

| What | Where |
|---|---|
| UI messages, one file per language | `apps/desktop/messages/<tag>.json` |
| List of languages | `apps/desktop/project.inlang/settings.json` (`locales`) |
| Language names in the language menu | `LOCALE_NAMES` in `apps/desktop/src/lib/i18n.ts` |
| User guide | `docs/user/<tag>/` |

`<tag>` is a language tag such as `fr` or `pt-BR`. The generated code in
`apps/desktop/src/lib/paraglide/` is rebuilt from these files and is not committed.

## Steps

1. Add the tag to `locales` in `apps/desktop/project.inlang/settings.json`.
2. Copy `apps/desktop/messages/en.json` to `apps/desktop/messages/<tag>.json` and translate every
   value. Keep the keys and the `{placeholders}` unchanged. English is the base language: a
   missing key falls back to it, but the test below fails, so translate all of them.
3. Add the language's own name to `LOCALE_NAMES` in `apps/desktop/src/lib/i18n.ts`, written in
   that language (for example `fr: "Français"`), so users find it whatever language is shown.
   TypeScript refuses to compile until the entry exists.
4. Update the expected list in `apps/desktop/src/lib/i18n.test.ts`.
5. Run the checks:

   ```sh
   pnpm -C apps/desktop check    # compiles the messages, type checks
   pnpm -C apps/desktop test     # every message file has the same keys as en.json
   pnpm -C apps/desktop tauri dev
   ```

   In the app open File > Options, choose the language and look through every view. The window
   reloads when the language changes.

6. Optional: translate the user guide into `docs/user/<tag>/`. Copy `docs/user/en/shortcuts.md`
   and add the table header words of the language to `WORDS` in
   `apps/desktop/src/lib/shortcuts-docs.test.ts`; the test then checks the shortcut tables and
   prints the correct ones when they differ.

## Wording

- Short, plain words of the shop floor. Use the terms of the standards for the language
  (ISO 1101, ISO 286, ISO 2768 in their national editions) for tolerances and characteristics.
- Address the user the same way throughout (the German texts use "du").
- Never copy texts or terms from other inspection or ballooning software (clean room policy,
  `docs/spec/00-clean-room-policy.md`).

## Glossary

Every label, tooltip, message and page of the user guide uses these words (T3.20). A new text
uses a term from this list or adds it here first, in both languages. The words are shop
language: no developer terms in the UI (the data model may keep its own names, for example
`sample` for a part, `attribute` for the OK / not OK result type). German addresses the user
with "du"; short hints may use the infinitive ("Leer lassen"), never "Sie".

### Window and navigation

| English | German | Notes |
|---|---|---|
| title bar | Titelleiste | Brand mark, Save, Undo, Redo, document name, light/dark, help |
| tab | Reiter | Never "Register" or "Registerkarte" |
| ribbon | Menüband | The row of commands under the tabs |
| group (of the ribbon) | Gruppe | Label under the commands, for example "Numbering" |
| More | Mehr | Menu for ribbon groups that do not fit |
| File, Home, Drawing, Review, Measure, Report, View | Datei, Start, Zeichnung, Überprüfen, Messen, Bericht, Ansicht | Tab names (ADR 0007). "Prüfen" is the Inspect switch, not the Review tab |
| File page | Seite Datei | The full page of the File tab |
| Info, Open, Export, Options, About Dimo | Info, Öffnen, Exportieren, Optionen, Über Dimo | Items of the File page |
| project info | Projektinfo | File > Info, the one home of part, drawing and report data |
| options | Optionen | Personal settings: theme, language, user name, company name, export defaults |
| properties pane | Eigenschaftenbereich | Right pane that follows the selection |
| pane | Bereich | Any side pane; "Bereich ausblenden" |
| status bar | Statusleiste | Sheet tabs, active tool, totals, general tolerance, zoom |
| start screen | Startseite | Shown without an open project |
| setup pane, Set up drawing | Bereich "Zeichnung einrichten", Zeichnung einrichten | Opens after a new project |
| Measure bar | Messleiste | Bar above the measurement grid: active cell, limits, value |
| report preview | Berichtsvorschau | |
| Quick access | Schnellzugriff | Accessible name of the title bar buttons |
| appearance, theme | Darstellung, Farbschema | Light, dark, same as the system: Hell, Dunkel, Wie das System |
| keyboard shortcuts | Tastenkürzel | |

### Commands

| English | German |
|---|---|
| New project, Open project, Save, Save as…, Close project | Neues Projekt, Projekt öffnen, Speichern, Speichern unter…, Projekt schließen |
| Undo, Redo, undo step | Rückgängig, Wiederholen, Rückgängig-Schritt |
| Select, Place balloon | Auswählen, Ballon setzen |
| Edit value, Delete | Wert bearbeiten, Löschen |
| Balloon style, Project balloon style | Ballonstil, Ballonstil des Projekts |
| Renumber, Preview numbers, Numbering options… | Neu nummerieren, Nummern-Vorschau, Nummerierung… |
| Update limits | Grenzmaße aktualisieren |
| Rotate left, Rotate right | Nach links drehen, Nach rechts drehen |
| Zone grid, Draw view | Zonenraster, Ansicht zeichnen |
| Decimal rules…, Company tables, Import table… | Dezimalregeln…, Firmentabellen, Tabelle importieren… |
| Add parts, Move left, Move right, Delete part | Teile hinzufügen, Nach links, Nach rechts, Teil löschen |
| Paste values, Clear cell | Werte einfügen, Zelle leeren |
| Only missing values, Only failed | Nur fehlende Werte, Nur n.i.O. |
| Save copy, Issue report, Print | Kopie speichern, Bericht ausgeben, Drucken |
| Export as issued | Als ausgegeben exportieren |
| Open after export | Nach dem Export öffnen |
| Zoom in, Zoom out, Fit sheet | Vergrößern, Verkleinern, Blatt einpassen |
| Accept, Discard (proposal card) | Übernehmen, Verwerfen |
| Skip, Start ballooning | Überspringen, Ballonieren starten |

### Drawing and characteristics

| English | German | Notes |
|---|---|---|
| drawing | Zeichnung | |
| sheet | Blatt | A page of the PDF |
| unit, scale | Einheit, Maßstab | Millimeter (mm), Inch / Zoll (in) |
| title block | Schriftfeld | |
| revision | Änderungsstand | Owner decision 2026-10-13 |
| balloon (plural balloons) | Ballon (plural Ballons) | Never "Ballone" |
| balloon the drawing, ballooning | Zeichnung ballonieren, Ballonieren | |
| leader line | Bezugslinie | |
| zone, zone grid | Zone, Zonenraster | |
| view (of the drawing) | Ansicht | Same word as the View tab, as in office software |
| characteristic | Merkmal | |
| characteristic table | Merkmalstabelle | Table under the drawing |
| characteristic list | Merkmalsliste | The CSV or Excel export |
| requirement | Anforderung | The text as printed on the drawing |
| kind | Art | Linear, diameter, ... |
| nominal | Nennmaß | |
| upper deviation, lower deviation | Oberes Abmaß, Unteres Abmaß | |
| limits | Grenzmaße | |
| upper limit, lower limit | Obere Grenze, Untere Grenze | Column heads, also in reports |
| fit, fit table | Passung, Passungstabelle | ISO 286 |
| quantity | Anzahl | Never "Menge" |
| class (of a characteristic) | Klasse | Critical, major, minor, key |
| inspect, Inspect (switch) | prüfen, Prüfen | Whether a characteristic is measured |
| inspection, inspection method, gauge | Prüfung, Prüfmethode, Prüfmittel | |
| sampling, frequency | Stichprobe, Häufigkeit | Inspection planning columns, not parts |
| comment | Kommentar | Of a characteristic or a part |
| remarks | Bemerkungen | Free text of the report header |
| proposal | Vorschlag | What Dimo read from a box, before Accept |
| reference dimension, basic dimension | Hilfsmaß, theoretisch genaues Maß | |
| history | Verlauf | |

### Tolerances

| English | German | Notes |
|---|---|---|
| tolerance | Toleranz | |
| general tolerance | Allgemeintoleranz | ISO 2768-1 classes fine, medium, coarse, very coarse: fein, mittel, grob, sehr grob |
| rule | Regel | Where the limits come from |
| explanation | Erklärung | |
| explicit (on the drawing) | Explizit (auf der Zeichnung) | |
| drawing rule | Zeichnungsregel | A company table that applies before the general tolerance |
| decimal rule | Dezimalregel | Tolerance by number of decimal places |
| company table | Firmentabelle | A tolerance table of the user; never "custom table" in the UI |
| No tolerance defined | Keine Toleranz festgelegt | |
| entered by hand | Von Hand eingegeben | |
| draft (table) | Entwurf | Values not verified yet: noch nicht verifiziert |
| shorter leg | Kürzerer Schenkel | Angles, ISO 2768-1 |

### Parts and results

| English | German | Notes |
|---|---|---|
| part | Teil (plural Teile) | One measured piece. Never "sample" in the UI |
| serial number | Seriennummer | Short "Serial" only as a report column head |
| measurement grid | Messwerttabelle | Shortcut names start with "Grid:" / "Messwerte:" |
| active cell, active row | Aktive Zelle, aktive Zeile | |
| block (of pasted values) | Werteblock, Block | Never "Bereich" (that is a pane) |
| result | Ergebnis | What is entered for a characteristic and a part |
| measured value, value | Messwert, Wert | |
| result type | Ergebnisart | Value / OK / not OK: Messwert / i.O. / n.i.O.; Auto: Automatisch |
| OK, not OK | i.O., n.i.O. | Entered results of the result type OK / not OK |
| pass, fail | i.O., n.i.O. | Verdict of a value against its limits |
| not evaluated | nicht bewertet | A value without limits |
| missing | fehlt (plural fehlen) | No result yet |
| open | offen | Overall verdict while results are missing or not evaluated |
| verdict, overall | Bewertung, Gesamt / Gesamtergebnis | |
| inspector | Prüfer | Of a part |

### Reports and exports

| English | German | Notes |
|---|---|---|
| report | Bericht | |
| check sheet | Prüfblatt | Empty result columns, filled in by hand |
| inspection report | Prüfbericht | |
| issue a report, issued report | Bericht ausgeben, ausgegebener Bericht | Locks the numbering and records the report |
| numbering is locked | Die Nummerierung ist gesperrt | |
| ballooned PDF | PDF mit Ballons | |
| Excel workbook | Excel-Datei / Excel-Arbeitsmappe | Labels say "Excel", not "XLSX" |
| report header | Berichtskopf | |
| report number, report date | Berichtsnummer, Berichtsdatum | |
| inspected by, approved by | Geprüft von, Freigegeben von | |
| company, company name | Firma, Firmenname | |
| part number, part name, customer, order reference | Teilenummer, Teilebezeichnung, Kunde, Auftragsnummer | |
| blank result columns | Leere Ergebnisspalten | |

## What is not a UI language

- Export and report languages are chosen per export, independent of the UI language (D-32).
  The headers of the characteristic list are defined in Rust
  (`crates/dimo-io/src/export/columns.rs`, `Language`); a new export language needs a change
  there and in `ReportLanguage` of `apps/desktop/src-tauri/src/settings.rs`.
- Enumeration values in exports (`linear`, `diameter`, ...) are stable identifiers and are never
  translated.
