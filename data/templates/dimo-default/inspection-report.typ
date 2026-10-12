// Inspection report of the dimo-default template set (D-30, M3 decisions 9, 12 and 15): what
// was inspected, against which requirements, on which parts, with which results and verdicts,
// and who inspected and approved it (ISO 9001 clause 8.6). Verdicts come from Rust; this file
// only prints them with their marks.

#import "common.typ": *

#show: report.with(L.inspection_report)

#let result-width = 18mm
#let verdict-width = 14mm

// Summary block: overall verdict and the number of cells per verdict.
#let s = data.summary
#block(breakable: false, width: 100%)[
  #text(weight: "bold", size: 8pt, L.summary)
  #v(-1mm)
  #table(
    columns: (40mm, 22mm, 22mm, 22mm, 22mm, 22mm, 22mm),
    align: (left + horizon, right, right, right, right, right, right),
    th(L.overall),
    th(L.characteristics, align: right),
    th(L.samples, align: right),
    th(verdict-text("pass"), align: right),
    th(verdict-text("fail"), align: right),
    th(verdict-text("not_evaluated"), align: right),
    th(L.verdicts.missing, align: right),
    text(size: 10pt, weight: "bold", verdict-text(s.verdict, size: 9pt)),
    s.characteristics,
    s.samples,
    s.counts.pass,
    s.counts.fail,
    s.counts.not_evaluated,
    s.counts.missing,
    ..if s.verdict == "fail" { bold-frame(0, 1) } else { () },
  )
  #if data.flags.open_results { text(size: 6.5pt, L.open_note) }
]

// Bold frames of failing samples in the sample table: the header is row 0, so sample i is row
// i + 1; the verdict is the last of 11 columns.
#let sample-frames() = {
  let frames = ()
  for (i, p) in data.samples.enumerate() {
    if p.verdict == "fail" { frames += bold-frame(10, i + 1) }
  }
  frames
}

// Sample block: one line per part with its counts and verdict.
#if data.samples.len() > 0 {
  v(2mm)
  text(weight: "bold", size: 8pt, L.samples)
  v(-1mm)
  table(
    columns: (9mm, 30mm, 18mm, 32mm, 1fr, 1fr, 14mm, 14mm, 14mm, 14mm, 24mm),
    align: (left, left, left, left, left, left, right, right, right, right, left),
    table.header(
      th("#"),
      th(L.serial),
      th(L.date),
      th(L.inspector),
      th(L.gauges),
      th(L.comment),
      th(mark("pass"), align: right),
      th(mark("fail"), align: right),
      th(mark("not_evaluated"), align: right),
      th(L.verdicts.missing, align: right),
      th(L.result),
    ),
    ..data.samples.map(p => (
      p.index,
      text(weight: "bold", p.serial),
      p.date,
      p.inspector,
      p.gauges,
      p.comment,
      p.counts.pass,
      p.counts.fail,
      p.counts.not_evaluated,
      p.counts.missing,
      verdict-text(p.verdict),
    )).flatten(),
    ..sample-frames(),
  )
}

// A result cell: value or attribute word with the verdict mark. A failing cell also gets a
// bold frame (decision 9, see the table below).
#let result-cell(c) = {
  let value = if c.type == "attribute" { L.attributes.at(c.value) } else { c.value }
  table.cell(align: right, if c.type == "none" { [] } else [#value #h(0.3em) #mark(c.verdict)])
}

// Bold frames of failing cells in one column block: the header is row 0, the sample columns
// follow the characteristic columns.
#let cell-frames(block) = {
  let frames = ()
  for (r, row) in data.rows.enumerate() {
    for (j, c) in row.cells.slice(block.from, block.to).enumerate() {
      if c.verdict == "fail" { frames += bold-frame(fixed-widths.len() + j, r + 1) }
    }
  }
  frames
}

// Characteristic table, in blocks of sample columns.
#v(2mm)
#for (i, block) in data.column_blocks.enumerate() {
  if i > 0 { pagebreak(weak: true) }
  block-heading(block)
  let samples = data.samples.slice(block.from, block.to)
  table(
    columns: fixed-widths + (result-width,) * samples.len() + (verdict-width,),
    table.header(
      ..fixed-headers,
      ..samples.map(p => th(align: right)[
        #text(weight: "regular", fill: muted, size: 6pt)[#L.part #p.index] \
        #p.serial
      ]),
      th(L.result, align: center),
    ),
    ..data.rows.map(row => (
      ..fixed-cells(row),
      ..row.cells.slice(block.from, block.to).map(result-cell),
      table.cell(align: center, mark(row.verdict)),
    )).flatten(),
    ..cell-frames(block),
  )
  if data.rows.len() == 0 { text(fill: muted, L.no_rows) }
}

#notes()

// Legend of the marks, so a black and white print reads without color.
#v(1.5mm)
#text(size: 6.5pt)[
  #L.marks: #h(0.4em)
  #mark("pass") #L.verdicts.pass #h(0.8em)
  #mark("fail") #L.verdicts.fail #h(0.8em)
  #mark("not_evaluated") #L.verdicts.not_evaluated #h(0.8em)
  #mark("open") #L.verdicts.open #h(0.8em)
  #L.missing_mark
]

// Remarks.
#if data.remarks != "" {
  v(3mm)
  block(breakable: false, width: 100%)[
    #text(weight: "bold", size: 8pt, L.remarks) \
    #data.remarks
  ]
}

// Signature block: inspected by and approved by, each with name, date and an empty field for
// a handwritten signature (decision 15).
#v(4mm)
#block(breakable: false, width: 100%)[
  #text(weight: "bold", size: 8pt, L.approval)
  #v(-1mm)
  #table(
    columns: (32mm, 60mm, 28mm, 1fr),
    rows: (auto, 12mm, 12mm),
    align: left + horizon,
    th(""),
    th(L.name),
    th(L.date),
    th(L.signature),
    text(weight: "bold", L.inspected_by),
    data.signatures.inspected_by.name,
    data.signatures.inspected_by.date,
    [],
    text(weight: "bold", L.approved_by),
    data.signatures.approved_by.name,
    data.signatures.approved_by.date,
    [],
  )
]
