// Spike template: a table with repeated header from JSON data. Not a real report layout.
#let data = json("/data.json")

#set document(title: data.title, date: none)
#set page(paper: "a4", flipped: true, margin: 12mm,
  header: [#data.title #h(1fr) page #context counter(page).display("1 of 1", both: true)])
#set text(font: "Open Sans", size: 8pt, lang: "en")
#show "⌀": "Ø"

#let cols = 5 + data.samples.len()
#table(
  columns: (auto, 1fr, auto, auto, auto) + data.samples.map(_ => auto),
  stroke: 0.4pt,
  table.header(
    [*No.*], [*Requirement*], [*Nominal*], [*Lower*], [*Upper*],
    ..data.samples.map(s => [*#s*]),
  ),
  ..data.rows.map(r => (r.no, r.req, r.nom, r.lo, r.hi, ..r.vals)).flatten()
)
