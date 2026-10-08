# 03 Functional requirements

Priority: **M** must (1.0), **S** should (1.0 if possible), **C** could (after 1.0).
Milestone references point to [09 Roadmap](09-roadmap.md). Pain point references point to [02](02-user-pain-points.md).

## FR-DOC Document handling

| ID | Requirement | Prio | MS |
|---|---|---|---|
| FR-DOC-01 | Open PDF drawings with any number of sheets and any sheet size up to A0 and beyond | M | M0 |
| FR-DOC-02 | Open TIFF, PNG and JPEG scans, multi-page TIFF included | M | M5 |
| FR-DOC-03 | Classify each sheet: vector with text layer, vector with outlined text, raster | M | M4 |
| FR-DOC-04 | Smooth pan and zoom with tiled rendering | M | M0 |
| FR-DOC-05 | Rotate sheets, set scale and units per sheet | M | M1 |
| FR-DOC-06 | Detect the drawing frame zone grid automatically, allow manual definition | S | M4 |
| FR-DOC-07 | Store a hash of every imported drawing to guarantee the report matches the exact file | M | M1 |
| FR-DOC-08 | Read title block fields: part number, revision, material, general tolerance, units, projection method | S | M4 |

## FR-BAL Ballooning and numbering

| ID | Requirement | Prio | MS |
|---|---|---|---|
| FR-BAL-01 | Manual placement: click a feature, balloon appears, characteristic row is created and focused | M | M1 |
| FR-BAL-02 | Keyboard driven capture: type value, Enter, place next | M | M1 |
| FR-BAL-03 | Balloon styles: circle, flag, rectangle; size, color, leader on/off; per project defaults | M | M1 |
| FR-BAL-04 | Numbering strategies: sheet then zone, per view, clockwise per view, by type, manual | M | M2 |
| FR-BAL-05 | Preview numbering before applying | S | M2 |
| FR-BAL-06 | Reorder by drag in the list, renumber in one action | M | M1 |
| FR-BAL-07 | Sub-numbering for repeated features (`5.1`, `5.2` for `4X`), or one balloon with quantity, per project setting | M | M2 |
| FR-BAL-08 | Automatic balloon placement avoids overlap with geometry, text and other balloons | S | M4 |
| FR-BAL-09 | Stable internal IDs independent from displayed number | M | M1 |
| FR-BAL-10 | Lock numbering once a report is issued | M | M1 |
| FR-BAL-11 | Insert policy when locked: sub-number, next free number, or letter suffix | M | M2 |
| FR-BAL-12 | Group selection, move, restyle, delete with undo | M | M1 |

## FR-CHR Characteristics

| ID | Requirement | Prio | MS |
|---|---|---|---|
| FR-CHR-01 | Types: linear, diameter, radius, spherical, angle, chamfer, thread, counterbore/countersink, depth, surface texture, geometric tolerance, note, flag note, material/process requirement, other | M | M1 |
| FR-CHR-02 | Fields: nominal, upper and lower deviation, upper and lower limit, unit, quantity, classification (critical, major, minor, key), inspection method, gauge, sampling, comment | M | M1 |
| FR-CHR-03 | Composite characteristics: dimension with attached frame, multi-segment frames | M | M6 |
| FR-CHR-04 | Variable dimensions resolved from a table on the drawing | S | M6 |
| FR-CHR-05 | Notes as characteristics, split per requirement inside a note | M | M6 |
| FR-CHR-06 | Flag notes linked to all features carrying the flag, across sheets | M | M6 |
| FR-CHR-07 | Title block and general notes captured as characteristics when required | M | M6 |
| FR-CHR-08 | Reference and basic dimensions recognized and excluded from inspection by default, overridable | M | M2 |
| FR-CHR-09 | Each characteristic links to its source region (sheet, bounding box, rotation) | M | M1 |
| FR-CHR-10 | Change history per characteristic (who, when, what, source: manual, rule, recognition) | S | M2 |

## FR-REC Recognition and automation

| ID | Requirement | Prio | MS |
|---|---|---|---|
| FR-REC-01 | Use the PDF text layer when present, with glyph positions and rotation | M | M2 |
| FR-REC-02 | Box select: recognize and parse a user selected region | M | M2 |
| FR-REC-03 | Auto detect characteristics on a sheet, view or region | M | M4 |
| FR-REC-04 | OCR for outlined vector text and raster scans, including rotated text | M | M5 |
| FR-REC-05 | Image preprocessing for scans: deskew, denoise, binarize | M | M5 |
| FR-REC-06 | Recognize feature control frames: symbol, tolerance, diameter zone, modifiers, datum references | M | M6 |
| FR-REC-07 | Recognize geometric tolerance, surface texture, welding and datum symbols | S | M6 |
| FR-REC-08 | Recognize decimal comma and decimal point, metric and inch | M | M2 |
| FR-REC-09 | Group tokens into one callout (prefix, nominal, tolerance stack, fit, suffix like `THRU`) | M | M4 |
| FR-REC-10 | Confidence per field and per characteristic | M | M4 |
| FR-REC-11 | Pluggable recognition engines behind one interface | M | M5 |
| FR-REC-12 | User defined symbol sets and token dictionaries | S | M6 |
| FR-REC-13 | Optional local vision language model assist for notes, opt in, never required | C | after 1.0 |

## FR-TOL Tolerance engine

| ID | Requirement | Prio | MS |
|---|---|---|---|
| FR-TOL-01 | Precedence: explicit tolerance > fit designation > drawing specific rule > general tolerance standard > decimal place rule | M | M2 |
| FR-TOL-02 | General tolerances for linear, angular, radius and chamfer dimensions by size range and class | M | M2 |
| FR-TOL-03 | General geometric tolerance classes | S | M6 |
| FR-TOL-04 | ISO fit designations for holes and shafts expanded to limits | M | M2 |
| FR-TOL-05 | Welded construction and thermal cutting general tolerances | S | M6 |
| FR-TOL-06 | Decimal place rules (e.g. `X.X ±0.1`, `X.XX ±0.05`) from the title block | M | M2 |
| FR-TOL-07 | Custom tolerance tables defined by the user as data files | M | M2 |
| FR-TOL-08 | Every derived limit stores its rule and shows a human readable explanation | M | M2 |
| FR-TOL-09 | Unit conversion mm and inch with configurable rounding | S | M2 |

## FR-REV Review and verification

| ID | Requirement | Prio | MS |
|---|---|---|---|
| FR-REV-01 | States: proposed, accepted, verified, rejected | M | M4 |
| FR-REV-02 | Review queue sorted by confidence, lowest first | M | M4 |
| FR-REV-03 | Side by side view: source crop and parsed fields | M | M4 |
| FR-REV-04 | Keyboard review: accept, edit, reject, next | M | M4 |
| FR-REV-05 | Coverage view: highlight drawing text not covered by any characteristic | M | M4 |
| FR-REV-06 | Proposals never overwrite user data without an explicit action | M | M4 |
| FR-REV-07 | Export warns if unverified characteristics remain | M | M4 |

## FR-MEA Measurements

| ID | Requirement | Prio | MS |
|---|---|---|---|
| FR-MEA-01 | Multiple serial numbers or samples per project | M | M3 |
| FR-MEA-02 | Manual result entry in a grid with keyboard navigation | M | M3 |
| FR-MEA-03 | Pass/fail evaluation against limits, color coding | M | M3 |
| FR-MEA-04 | Import results from CSV and spreadsheet with saved column mapping profiles | M | M7 |
| FR-MEA-05 | Import from QIF results | S | M9 |
| FR-MEA-06 | Attribute results (go/no go, visual OK) and variable results | M | M3 |

## FR-EXP Export and reporting

| ID | Requirement | Prio | MS |
|---|---|---|---|
| FR-EXP-01 | Ballooned PDF, original vector content preserved, balloons as vector overlay | M | M1 |
| FR-EXP-02 | First article report forms 1 to 3 structure | M | M7 |
| FR-EXP-03 | Initial sample report structure (European automotive) | M | M7 |
| FR-EXP-04 | Generic inspection report and check sheet | M | M3 |
| FR-EXP-05 | Spreadsheet templates with documented placeholders | M | M7 |
| FR-EXP-06 | Text based report templates rendered to PDF | M | M3 |
| FR-EXP-07 | Template preview with live data | S | M7 |
| FR-EXP-08 | Template packs importable and exportable as files | S | M7 |
| FR-EXP-09 | CSV characteristic list for CMM programming | M | M1 |
| FR-EXP-10 | QIF plans and results export | S | M9 |
| FR-EXP-11 | Deterministic output: same project and version produce identical files | M | M1 |

## FR-RVC Revisions

| ID | Requirement | Prio | MS |
|---|---|---|---|
| FR-RVC-01 | Import a new drawing revision into an existing project | M | M8 |
| FR-RVC-02 | Visual comparison: overlay and changed region highlighting | M | M8 |
| FR-RVC-03 | Carry over unchanged characteristics with numbers, flag changed and removed ones, propose new ones | M | M8 |
| FR-RVC-04 | Reports record the drawing revision and file hash they refer to | M | M3 |

## FR-AUT Automation and integration

| ID | Requirement | Prio | MS |
|---|---|---|---|
| FR-AUT-01 | Command line tool: detect, export, apply template, headless | M | M9 |
| FR-AUT-02 | JSON output of characteristic lists for other systems | M | M9 |
| FR-AUT-03 | Watch folder mode for batch processing | C | after 1.0 |
| FR-AUT-04 | Plugin interface for custom exporters and parsers (sandboxed) | C | after 1.0 |

## FR-SET Settings and profiles

| ID | Requirement | Prio | MS |
|---|---|---|---|
| FR-SET-01 | Customer profiles: numbering strategy, balloon style, templates, tolerance defaults, classification rules | M | M7 |
| FR-SET-02 | Profiles exportable as files to share within a company | M | M7 |
| FR-SET-03 | Organization policy file to disable online features | M | M5 |
| FR-SET-04 | UI languages English and German, extensible | M | M1 |
