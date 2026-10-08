# test_drawing_1

## Facts

- Created by the owner in Onshape, exported as PDF (producer string: ODA PDF Export v24.7)
- 1 sheet, 1584 x 1224 pt = 22 x 17 inch (ANSI C size), zone grid A to D and 1 to 4
- Metric drawing with ISO fit designations, despite the inch sheet format. Units must never be inferred from the sheet size.
- Optional content groups (layers): Visible, TITLE_BLOCK, BORDER_FRAME, BORDER_ZONES, CENTERMARKS, ANNOTATION_TEXT
- **Sheet kind: `vector_text`.** 74 text objects in an embedded subset font (NotoSans, Type0, Identity-H encoding; `/ToUnicode` is a name instead of a CMap stream, see Pitfall). All callouts are real text.

## Metadata and privacy check (2026-10-08)

- Info dictionary: only Producer and CreationDate. No Author, Title, Subject or Keywords.
- No XMP metadata, no embedded files, no annotations, no form fields, no JavaScript, no links.
- No names, e-mail addresses or file paths in any stream.
- Title block fields NAME, DATE, DRAWN, CHECKED, APPROVED, MATERIAL, TITLE, DWG NO. are empty.
- Result: no personal information. Safe to publish.

## Pitfall: text layer not found by every library

macOS PDFKit reported 0 characters for this page, although the content stream holds 74 text objects with a ToUnicode map. Our first analysis wrongly classified the sheet as outlined text because of this. Consequence: sheet classification (FR-DOC-03) must inspect the content stream itself (text operators, fonts) and must not rely on a single library's text extraction. This file is a regression test for that.

Likely cause (found in T0.10): the font's `/ToUnicode` entry is the name `/Identity-H` instead of a CMap stream, which the PDF specification does not allow. The CIDs equal the Unicode code points (for example `0x2212` for the minus sign, `0xD8` for `Ø`), so decoding them as UTF-16 code units gives the correct text. Text extraction must tolerate this.

## Text content (from the content stream)

Title block notes: `UNLESS OTHERWISE SPECIFIED,` `DIMENSIONS ARE IN MILLIMETERS`, `ANGULAR = ± °` (value left empty), `SURFACE FINISH` (empty), `DO NOT SCALE DRAWING`, `BREAK ALL SHARP EDGES AND REMOVE BURRS`, `THIRD ANGLE PROJECTION`, `SIZE C`, `SCALE 1:1`, `SHEET 1 of 1`.

Callouts:

| Text | Notes |
|---|---|
| `Ø30 H7` with stacked `+0.0203` / `-0` | Fit plus explicit deviations |
| `Ø8 f7` with stacked `-0.0127` / `-0.0279` | Fit plus explicit deviations, appears 3 times |
| `Ø8 h6`, `Ø8 d9`, `Ø8 c10` | Fit only |
| `R15 H7` | Radius with a fit designation (unusual, must still parse) |
| `100` with stacked `+0` / `−0.6` | Unicode minus U+2212 |
| `90.0°` with stacked `+0.0°` / `−0.1°` | Angular tolerance, stacked |
| `90.0°` | Untoleranced angle |
| `50±0.1`, `12.39±0.1` | Symmetric tolerance |
| `200`, `100`, `50`, `40`, `30`, `20`, `31.05`, `52.61`, `71.04`, `86.16` | Untoleranced |

## Findings for the spec

1. **Explicit deviations differ from ISO 286.** `Ø8 f7` shows −0.0127 / −0.0279, ISO 286 gives −0.013 / −0.028. `Ø30 H7` shows +0.0203, ISO 286 gives +0.021. The four decimal values are exact inch conversions (0.0005 in = 0.0127 mm), so the CAD tool uses inch based fit tables. Rule: explicit deviations on the drawing win, a mismatch with the ISO table is shown as an info hint, never silently corrected.
2. **No general tolerance declared.** The title block has no linear general tolerance and an empty angular value. Untoleranced dimensions such as `200` or `31.05` therefore have no tolerance. The tolerance engine must flag them for review ("no tolerance defined on drawing") instead of applying a default standard.
3. **Mixed minus signs.** Hyphen `-` and Unicode minus `−` appear on the same sheet. The parser accepts both.
4. **Stacked deviations are separate text objects** placed right of the callout. Token grouping must attach them by geometry.
5. **Repeated identical callouts** (`Ø8 f7` three times) are separate characteristics unless marked with a quantity.

## OCR baseline (for comparison only)

macOS Vision fast mode on a 300 dpi render found 29 of the 74 text items. It read `Ø` as `0` (`08f7`, `030 H7`), split `Ø8 h6`, merged `R15 H7` incorrectly, and read `+0.0°` as `+O.O°`. Accurate mode failed with a runtime error in this environment. Useful later as an OCR test by ignoring the text layer.

## Ground truth

`truth/test_drawing_1.truth.json` (T0.10): 24 characteristics, regions measured from the content stream. Draft until the owner has reviewed it; entries with a `review_note` need a decision.
