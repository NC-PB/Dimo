// Shared parts of the dimo-default report documents (M3 decisions 9, 12 and 15).
//
// Data: `/data.json`, the report model from Rust (data/templates/README.md). Every printed value
// is already text; this file only lays it out. It never computes with measured numbers and never
// decides pass or fail (AGENTS.md rules 2 and 5).

#let data = json("/data.json")
#let lang = sys.inputs.at("lang", default: "en")
#let L = toml("/labels.toml").at(lang)

// Colors. Every verdict also has its own shape, so a black and white print stays readable
// (NFR-UX-05).
#let ink = black
#let muted = luma(85)
#let rule-gray = luma(150)
#let head-fill = luma(232)
#let pass-color = rgb("#1b7a3a")
#let fail-color = rgb("#b3261e")
#let open-color = luma(70)

// Column widths shared by both documents (A4 landscape, 273 mm between the margins).
#let w-number = 9mm
#let w-value = 15mm
#let w-unit = 10mm
#let w-rule = 19mm
#let w-method = 25mm

// Verdict marks drawn as shapes: Open Sans has no check, cross or circle glyphs (T3.2).
#let mark-box(size, body) = box(width: size, height: size, baseline: 12%, body)

#let pass-mark(size: 7pt, paint: pass-color) = mark-box(size, curve(
  stroke: (paint: paint, thickness: 0.16 * size, cap: "round", join: "round"),
  curve.move((0.05 * size, 0.55 * size)),
  curve.line((0.38 * size, 0.88 * size)),
  curve.line((0.95 * size, 0.12 * size)),
))

#let fail-mark(size: 7pt, paint: fail-color) = mark-box(size, {
  let s = (paint: paint, thickness: 0.16 * size, cap: "round")
  place(line(start: (0.1 * size, 0.1 * size), end: (0.9 * size, 0.9 * size), stroke: s))
  place(line(start: (0.9 * size, 0.1 * size), end: (0.1 * size, 0.9 * size), stroke: s))
})

#let circle-mark(size: 7pt, paint: open-color) = mark-box(size, place(
  dx: 0.1 * size,
  dy: 0.1 * size,
  circle(radius: 0.4 * size, stroke: 0.12 * size + paint),
))

// An open group verdict: a short bar, distinct from the three cell marks.
#let open-mark(size: 7pt, paint: open-color) = mark-box(size, place(
  line(start: (0.1 * size, 0.5 * size), end: (0.9 * size, 0.5 * size), stroke: (
    paint: paint,
    thickness: 0.16 * size,
    cap: "round",
  )),
))

// Mark of a cell verdict (pass, fail, not_evaluated, missing) or a group verdict (pass, fail,
// open).
#let mark(verdict, size: 7pt) = {
  if verdict == "pass" { pass-mark(size: size) }
  else if verdict == "fail" { fail-mark(size: size) }
  else if verdict == "not_evaluated" { circle-mark(size: size) }
  else if verdict == "open" { open-mark(size: size) }
}

// Mark followed by the verdict word.
#let verdict-text(verdict, size: 7pt) = [#mark(verdict, size: size) #h(0.2em) #L.verdicts.at(verdict)]

// Small label above a value.
// An empty value prints no label either; the cell keeps its place so the layout stays stable.
#let field(label, value) = block(spacing: 0pt)[
  #text(size: 5.5pt, fill: muted, if value == "" { "" } else { label }) \
  #text(size: 7.5pt, value)
]

// Page header: company, document title, part, customer, order, drawing, revision, short hash,
// report number and date, page x of y (decision 12).
#let page-header(title) = context {
  let h = data.header
  let part = (h.part_number, h.part_name).filter(s => s != "").join("  ")
  grid(
    columns: (1fr, auto),
    align: (left + bottom, right + bottom),
    text(size: 10pt, weight: "bold", h.company),
    text(size: 13pt, weight: "bold", title),
  )
  v(-1.5mm)
  line(length: 100%, stroke: 0.6pt + ink)
  v(-1.5mm)
  grid(
    columns: (1.6fr, 1.1fr, 1.1fr, 1.6fr, 0.8fr, 1fr, 0.9fr, 0.7fr, 0.6fr),
    column-gutter: 2.5mm,
    field(L.part, if part == none { "" } else { part }),
    field(L.customer, h.customer),
    field(L.order, h.order_reference),
    field(L.drawing, h.drawing_file),
    field(L.revision, h.revision),
    field(L.sha256, h.sha256_short),
    field(L.report_number, h.report_number),
    field(L.report_date, h.report_date),
    field(L.page, [#counter(page).display() #L.of #counter(page).final().first()]),
  )
  v(-1mm)
  line(length: 100%, stroke: 0.3pt + rule-gray)
}

// Page setup of a report document: A4 landscape, header on every page.
#let report(title, body) = {
  set document(title: title, author: data.header.company)
  set page(
    paper: "a4",
    flipped: true,
    margin: (left: 12mm, right: 12mm, top: 30mm, bottom: 12mm),
    header-ascent: 3mm,
    header: page-header(title),
  )
  // No ligatures: copied and extracted text keeps "fi" and "ff" as two letters.
  set text(font: "Open Sans", size: 7pt, lang: lang, ligatures: false)
  set par(leading: 0.45em)
  set table(stroke: 0.4pt + rule-gray, inset: (x: 1.2mm, y: 1mm))
  // Open Sans has no U+2300; the diameter sign prints as Ø, which looks the same on drawings.
  show "⌀": "Ø"
  // Full drawing hash on the first page (FR-DOC-07, decision 12).
  text(size: 6.5pt)[#text(fill: muted, L.sha256_full): #data.header.sha256]
  v(1.5mm)
  body
}

// Bold frame around the cell in column `x`, row `y` of a table, for a failing verdict
// (decision 9). Table lines take priority over cell strokes, so all four sides stay bold even
// where a neighbor cell draws its own border.
#let bold-frame(x, y, stroke: 1.2pt + ink) = (
  table.hline(y: y, start: x, end: x + 1, stroke: stroke),
  table.hline(y: y + 1, start: x, end: x + 1, stroke: stroke),
  table.vline(x: x, start: y, end: y + 1, stroke: stroke),
  table.vline(x: x + 1, start: y, end: y + 1, stroke: stroke),
)

// Header cell of a table.
#let th(body, align: left) = table.cell(fill: head-fill, align: align + bottom, text(
  weight: "bold",
  body,
))

// The characteristic columns that every table block repeats: number, requirement, nominal,
// limits, unit, rule with its marker, method and gauge.
#let fixed-widths = (w-number, 1fr, w-value, w-value, w-value, w-unit, w-rule, w-method)

#let fixed-headers = (
  th(L.number),
  th(L.requirement),
  th(L.nominal, align: right),
  th(L.lower_limit, align: right),
  th(L.upper_limit, align: right),
  th(L.unit),
  th(L.rule),
  th(L.method),
)

#let rule-cell(row) = {
  if row.rule != none {
    L.rules.at(row.rule)
    if row.draft { super[\*] }
    if row.rule_detail != "" { linebreak(); text(size: 6pt, fill: muted, row.rule_detail) }
  }
}

#let fixed-cells(row) = (
  text(weight: "bold", row.number),
  row.requirement,
  table.cell(align: right, row.nominal),
  table.cell(align: right, row.lower_limit),
  table.cell(align: right, row.upper_limit),
  row.unit,
  rule-cell(row),
  {
    row.method
    if row.method != "" and row.gauge != "" { linebreak() }
    row.gauge
  },
)

// Heading of a column block when the samples need more than one (decision 12).
#let block-heading(block) = {
  if data.column_blocks.len() > 1 {
    text(weight: "bold", size: 8pt)[#L.samples #block.first #L.samples_to #block.last #L.of #data.samples.len()]
    v(1mm)
  }
}

// Footnotes under the characteristic table.
#let notes() = {
  if data.flags.draft_limits {
    v(1.5mm)
    text(size: 6.5pt)[#super[\*] #L.draft_note]
  }
}
