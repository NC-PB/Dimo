# ADR 0009: Three milestones after release 0.1 instead of six

Status: accepted

## Context
The roadmap planned six milestones after 0.1: M4 auto detection on vector PDFs, M5 OCR, M6
geometric tolerances and notes, M7 advanced reports, M8 revisions, M9 automation and interchange.
The owner wants to move faster. The code review of 2026-10-14 showed that these milestones share
code in three groups: the recognition pipeline (token grouping, parsing, proposals, review queue),
machine learning models (one ONNX runtime, model files, image preprocessing), and the import and
export layer on the project model.

## Decision
- M4 Automatic recognition: old M4 plus the parts of old M6 that work on PDF text (feature control
  frames, basic and reference dimensions, notes, flag notes, variable tables, composite
  characteristics, user symbol sets).
- M5 Scans and machine learning: old M5 plus the symbol classifier from old M6 (FR-REC-07).
- M6 Interchange, advanced reports and revisions: old M7, M8 and M9.
- Opening images and phone photos for manual work (FR-DOC-02 without OCR) moves into release 0.1.
- A foundation milestone MF comes before release 0.1 work (`docs/plan/MF.md`).
- Each merged milestone keeps the exit criteria of its parts and ships as its own release.

## Consequences
- Each recognition stage and each import or export path is built once, inside the milestone that
  owns its shared code; fewer handovers between milestones.
- Milestones get larger; releases after each one (0.2, 0.3, 0.4) keep a steady pace for users.
- M6 is broad; revisions can run as a parallel track inside it.
- Requirement milestone tags in `docs/spec/03-functional-requirements.md`, D-31 and the roadmap in
  `docs/spec/09-roadmap.md` are updated in the same change. Earlier milestone plans (M2, M3) keep
  their historic references.
