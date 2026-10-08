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

## Text extraction with PDFium (T0.6, 2026-10-09)

Measured with PDFium chromium/7881 through `dimo-pdf` (`Document::analyze_sheet`, `Document::text_runs`). Regression tests: `crates/dimo-pdf/tests/text.rs`.

- **Classification: `vector_text`.** The page content has 74 text objects, all visible, one font (`NotoSans`, embedded subset), no images, 407 page objects in total. PDFium decodes 380 non space characters, none of them unmapped.
- **Runs.** The 74 text objects become 72 runs. Only the title block `1 `, ` of `, `1` (three objects on one baseline) merge into `1 of 1`. Trailing spaces in objects such as `Ø30 H7 ` are trimmed.
- **Callouts against the truth file.** For each of the 24 characteristics, the runs whose center lies in the truth region (33 runs in total) joined in reading order give exactly the `requirement_text`. Every run box lies within 0.5 pt of its truth region. The run box is PDFium's loose character box (advance width, font ascent to descent), the same construction as the truth regions.
- **Stacked deviations are separate runs.** They use the same font size as the callout (12.05 pt) and start directly after the trailing space of the main text, about 0.26 em after its last glyph. They differ by baseline: the upper deviation sits about 0.47 em above, the lower about 0.5 em below the main baseline. Merging must therefore check the baseline, spacing alone would join them.
- **Symbols are preserved:** `Ø` U+00D8, `°` U+00B0, `±` U+00B1, the Unicode minus U+2212 in `−0.6` and `−0.1°`, and the hyphen minus in `-0`, `-0.0127`, `-0.0279`.
- **Order.** PDFium's character order follows the content stream almost everywhere; one of the two `D` zone labels comes after the first callouts. Runs keep PDFium's order; nothing should depend on it.

### `/Identity-H` as `/ToUnicode`

The font is a Type0 font with `/Encoding /Identity-H` and `/ToUnicode /Identity-H` (a name, not a CMap stream). The descendant `CIDFontType2` has `CIDSystemInfo` Adobe-Identity-0 and a `CIDToGIDMap` stream that maps each CID to the glyph of the subset font, for example CID 0x2212 to glyph 48. The CIDs are Unicode code points.

PDFium ignores a `/ToUnicode` entry that is not a stream. For an Adobe-Identity CID font without a usable map it then uses the character code itself as the Unicode value. Synthetic PDFs confirm this (test `identity_h_without_to_unicode_passes_codes_through`): a missing `/ToUnicode` and the name `/Identity-H` give identical text, a real CMap stream takes precedence. With `CIDSystemInfo` Adobe-Japan1 instead, PDFium maps the same codes through the Japan1 table and returns different, wrong characters.

Consequences:

1. This drawing extracts correctly only because the exporter writes Unicode code points as CIDs. An exporter that writes glyph ids as CIDs with the same invalid `/ToUnicode` produces wrong text without any error.
2. Classification therefore counts unmapped characters (control characters, private use, U+FFFD, zero) of the visible text. Sheets whose text is mostly unmapped go the OCR path (test `undecodable_text_is_classified_for_ocr`). Glyph ids that happen to land on printable letters are not detected; checking a sample of runs against OCR of the rendered sheet would catch that (later milestone).
3. A likely reason why other libraries reported zero characters for this sheet: they reject the invalid `/ToUnicode` entry and give up instead of falling back to the character codes.

### Tooling notes

- PDFium does not expose font dictionaries, so `dimo-pdf` cannot see the `/ToUnicode` entry itself. The page objects (PDFium's parsed content stream: text objects with render mode and font, paths, images, form XObjects) plus the unmapped share answer the question that matters for the recognition path. The values above were read with pypdf 6.14.2 for this note only.
- lopdf was evaluated for reading the raw content stream and font dictionaries and not added: version 0.45 pulls in about 19 crates including encryption and compression code, and it would add a second PDF parser whose view can differ from what PDFium renders. Revisit if a corpus drawing needs the font dictionaries for classification.
- pdfium-render 0.9.4 pitfalls: `PdfPageTextChar::angle_degrees` is clockwise, and `scaled_font_size` multiplies by the matrix entry `d` only, which is 0 for text rotated by 90 degrees. `dimo-pdf` computes rotation and size from the character matrix. `FPDFText_HasUnicodeMapError` exists but is only reachable through `unsafe` binding calls, which the workspace forbids.
