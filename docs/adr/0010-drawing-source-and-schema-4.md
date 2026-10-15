# ADR 0010: Drawing source type and project schema 4

Status: proposed

## Context
Release 0.1 opens photos and scans as drawings (R01 decisions 7 to 11, FR-DOC-02): JPEG, PNG and
TIFF, stored unchanged with their SHA-256 as fingerprint, turned into a sheet by settings that
can be changed later (rotation, fine angle, four corners, trim, paper format, clean up). Until
schema 3 every revision was a PDF: files were stored as `drawings/<sha256>.pdf`, the tile service
rendered only PDFium documents, and the desktop shell opened and analyzed the PDF itself to
create a project. The code review of 2026-10-14 also found two hash types for the same value.
MF decision 1: F1 and F4 share one schema increase, so 0.1 needs no second migration for images.

## Decision
- `DrawingRevision` gets `source`: `{"type": "pdf"}` or `{"type": "image", "format": "jpeg" |
  "png" | "tiff", "pages": [ImageEdit, ...]}` with one edit per page, in sheet order (a TIFF has
  one sheet per page). `ImageEdit` holds quarter turns, a fine angle in degrees, sheet corners
  in pixels of the stored page, a trim as fractions from 0 to 1 of the page after rotation and
  corner correction (independent of any intermediate pixel size), a paper (standard size and
  orientation, or a custom size in mm) and a clean up flag. Persisted numbers are exact
  decimals (rule 5). Image revisions are part of the format now and become creatable in R.19.
- The stored drawing is named by its source: `drawings/<sha256>.<ext>` with `pdf`, `jpg`, `png`
  or `tiff`. Schema version 4; the migration from 3 makes every revision a PDF source.
- `Change::RevisionChanged` (revision ID, source before and after) and `Command::SetImageEdit`
  (revision, page, edit) change the image settings of a page as one undo step; refused for PDF
  revisions. Sheet sizes and balloon positions that follow from an edit are derived in Rust
  (R.20), never in the frontend.
- `dimo_pdf::RasterSource` (kind, content hash, render key, sheet count, sheet size in sheet
  space, render region at a zoom, sheet kind) is what the tile service renders. The PDF
  document implements it; image sources will too. The render key names a source in tile URLs,
  the service and both caches: the content hash for a PDF, the hash plus the edit settings for
  an image, so changed settings never get old tiles. Cache keys also include the source kind.
- `dimo_io::project::create_from_drawing_file` creates every new project: it sniffs the file
  type, opens it through a `RasterOpener` (the tile service in the app) and imports it. `dimo-io`
  may therefore depend on `dimo-pdf`. The shell no longer calls PDFium to create a project.
- One hash type, `dimo_core::Sha256Hex` (32 bytes, written as 64 lowercase hex digits as before).
- Field level update commands (MF F4, same schema): `update_project_info`,
  `update_report_info`, `update_sheet` and `update_sample` carry a list of
  `{"field": ..., "value": ...}` entries instead of a whole record or one option per field;
  single tolerance settings change with `update_tolerance_settings`, decimal place rules with
  `add_decimal_rule` and `remove_decimal_rule`. The migration from 3 rewrites these commands in
  the audit log (also inside batches, undo and redo) with every field the old command set, so
  the conversion is lossless; the changes keep their whole before and after states.

## Consequences
- Version 3 projects open and save as version 4; PDF projects export byte identical files.
  Older Dimo builds refuse version 4 files (NFR-REL-04).
- Image support in 0.1 needs a decoder and an image `RasterSource` with its render key, no
  format change and no change to the tile service.
- `dimo-io` builds against `dimo-pdf` (no new third party crate); tests use a fake opener.
- Migrations that change `ImageEdit` must also patch the sources inside `revision_changed`
  audit changes.
- Refines ADR 0004 (drawing file names).
- Supersedes ADR 0005 in part: PDFium stays the only renderer and text source for PDF drawings;
  tiles of image drawings are rendered in Rust from the image, in the same sheet space. Owner:
  add "Superseded in part by ADR 0010 (image drawings)" to the status line of ADR 0005.
