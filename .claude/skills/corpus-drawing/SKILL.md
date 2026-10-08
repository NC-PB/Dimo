---
name: corpus-drawing
description: Add or document a test drawing in corpus/ (provenance, metadata privacy check, notes, ground truth *.truth.json). Use when the user provides a new drawing or when writing truth files.
---

# Corpus drawing

The agent never adds drawings on its own and never downloads drawings. The owner provides the file;
`corpus/drawings/` is write protected for the agent, so ask the owner to copy it there.

For a new drawing the agent then:

1. Computes SHA-256 (`shasum -a 256`).
2. Runs a metadata and privacy check: info dictionary, XMP, embedded files, annotations, form fields,
   JavaScript, links, names, e-mail addresses or file paths in streams, title block fields.
   Report anything personal to the owner before continuing.
3. Adds a row to `corpus/PROVENANCE.md`: file, SHA-256 (short form like existing rows), origin,
   license, date. Origin must be "own drawing" or a source with an explicit permissive license.
4. Writes `corpus/notes/<name>.md` following `corpus/notes/test_drawing_1.md`: facts, privacy check,
   sheet kind with evidence from the content stream, text content, findings for the spec, pitfalls.
5. Writes `corpus/truth/<name>.truth.json` once the truth schema exists (task T0.10). Expected limits
   come from what the drawing states plus the tolerance rules, never from the recognizer output.
   Mark uncertain entries and list them for owner review in STATUS.md.

Findings that change requirements go into the spec or a new ADR, not only into the notes.
