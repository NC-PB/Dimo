// Check sheet of the dimo-default template set (D-30, M3 decision 12): the characteristics to
// inspect with empty result columns, printed before measuring. One column per sample with its
// serial, or blank columns when the project has no samples yet.

#import "common.typ": *

#show: report.with(L.check_sheet)

#let result-width = 24mm

#for (i, block) in data.column_blocks.enumerate() {
  if i > 0 { pagebreak(weak: true) }
  block-heading(block)
  let samples = data.samples.slice(block.from, block.to)
  table(
    columns: fixed-widths + (result-width,) * samples.len(),
    table.header(
      ..fixed-headers,
      ..samples.map(s => th(align: center)[
        #text(weight: "regular", fill: muted, size: 6pt)[#L.part #s.index] \
        #if s.blank { v(3.5mm) } else { s.serial }
      ]),
    ),
    ..data.rows.map(row => (
      ..fixed-cells(row),
      // Room for a handwritten value.
      ..samples.map(_ => box(height: 5mm)),
    )).flatten(),
  )
  if data.rows.len() == 0 { text(fill: muted, L.no_rows) }
}

#notes()
#if data.flags.blank_samples {
  v(1.5mm)
  text(size: 6.5pt, L.blank_note)
}
