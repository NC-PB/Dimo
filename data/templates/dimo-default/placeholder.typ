// PLACEHOLDER document (T3.2), replaced by the real report layouts in T3.6.
// Data: { "title": text, "columns": [text], "rows": [[text]] }. All values are text from Rust;
// the template never computes with numbers and never decides pass or fail.
#let data = json("/data.json")
#let lang = sys.inputs.at("lang", default: "en")
#let label = (
  en: (page: "Page", of: "of", note: "Placeholder template"),
  de: (page: "Seite", of: "von", note: "Platzhaltervorlage"),
).at(lang)

#set document(title: data.title)
#set page(
  paper: "a4",
  flipped: true,
  margin: 12mm,
  header: [
    #data.title #h(1fr)
    #label.page #context counter(page).display() #label.of #context counter(page).final().first()
  ],
)
#set text(font: "Open Sans", size: 8pt, lang: lang)
// Open Sans has no U+2300; the diameter sign prints as Ø, which looks the same on drawings.
#show "⌀": "Ø"

#text(weight: "bold", size: 11pt)[#label.note]

#table(
  columns: data.columns.len(),
  stroke: 0.4pt,
  table.header(..data.columns.map(c => strong(c))),
  ..data.rows.flatten(),
)
