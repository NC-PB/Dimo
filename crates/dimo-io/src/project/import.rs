//! Importing a drawing file (FR-DOC-07).
//!
//! `dimo-io` does not open PDFs: the crate rules allow it only `dimo-core`. The caller (the
//! desktop shell or the CLI) opens the bytes with `dimo-pdf` and passes what it found per sheet
//! together with the hash `dimo-pdf` computed. `dimo-cli/tests/project_import.rs` shows the
//! glue:
//!
//! ```text
//! let pdf = engine.open(bytes.clone())?;
//! let sheets: Vec<SheetInfo> = (0..pdf.sheet_count())
//!     .map(|i| SheetInfo { size: Size { width, height } of pdf.sheet_size(i)?,
//!                          kind: pdf.analyze_sheet(i)?.kind })
//!     .collect();
//! let hash = Sha256Hex::parse(&pdf.content_hash().to_hex())?;
//! let drawing = import_drawing(bytes, "part.pdf", &hash, &sheets, &mut env)?;
//! ```
//!
//! The hash check ties the sheet data to the stored bytes: if the bytes handed in differ from
//! the bytes that were analyzed, the import fails instead of storing a mismatched pair.

use dimo_core::{
    DrawingRevision, Environment, RevisionId, Sha256Hex, Sheet, SheetId, SheetKind, Size,
};

use super::error::ProjectError;
use super::{Limits, sha256};

/// What the PDF reader found on one sheet, in page order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetInfo {
    /// Size in PDF user units, unrotated.
    pub size: Size,
    /// How the text is stored.
    pub kind: SheetKind,
}

/// A drawing ready to become part of a project: the revision and the file bytes stored as
/// `drawings/<sha256>.pdf`.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedDrawing {
    /// Revision with new IDs, file name, hash, import time and sheets with default view
    /// settings (unit mm, scale 1:1, D-20).
    pub revision: DrawingRevision,
    /// The file, unchanged.
    pub bytes: Vec<u8>,
}

/// Turns a drawing file into a revision.
///
/// `analyzed_hash` is the SHA-256 the PDF reader computed for the bytes it analyzed;
/// `file_name` may be a full path, only the last component is kept. IDs and the import time
/// come from `env`. The revision label starts empty.
pub fn import_drawing(
    bytes: Vec<u8>,
    file_name: &str,
    analyzed_hash: &Sha256Hex,
    sheets: &[SheetInfo],
    env: &mut dyn Environment,
) -> Result<ImportedDrawing, ProjectError> {
    let limit = Limits::DEFAULT.max_drawing;
    if bytes.len() as u64 > limit {
        return Err(ProjectError::TooLarge {
            name: file_name.to_owned(),
            limit,
        });
    }
    let actual = sha256(&bytes);
    if actual != *analyzed_hash {
        return Err(ProjectError::HashMismatch {
            expected: analyzed_hash.clone(),
            actual,
        });
    }
    if sheets.is_empty() {
        return Err(ProjectError::InvalidDrawing(
            "the drawing has no sheets".into(),
        ));
    }
    let valid = |v: f64| v.is_finite() && v > 0.0;
    if let Some(index) = sheets
        .iter()
        .position(|s| !valid(s.size.width) || !valid(s.size.height))
    {
        return Err(ProjectError::InvalidDrawing(format!(
            "sheet {} has no valid size",
            index + 1
        )));
    }
    let name = file_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(file_name)
        .to_owned();

    let revision_id = RevisionId::from_uuid(env.new_uuid());
    let mut sheet_list = Vec::with_capacity(sheets.len());
    for (index, info) in sheets.iter().enumerate() {
        let index = u32::try_from(index)
            .map_err(|_| ProjectError::InvalidDrawing("too many sheets".into()))?;
        sheet_list.push(Sheet::new(
            SheetId::from_uuid(env.new_uuid()),
            index,
            info.size,
            info.kind,
        ));
    }
    Ok(ImportedDrawing {
        revision: DrawingRevision {
            id: revision_id,
            label: String::new(),
            file_name: name,
            sha256: actual,
            imported_at: env.now(),
            sheets: sheet_list,
        },
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use dimo_core::FixedEnvironment;

    use super::*;

    const A4: SheetInfo = SheetInfo {
        size: Size {
            width: 595.0,
            height: 842.0,
        },
        kind: SheetKind::VectorText,
    };

    #[test]
    fn import_keeps_bytes_and_hash() {
        let bytes = b"%PDF-1.7 test".to_vec();
        let hash = sha256(&bytes);
        let mut env = FixedEnvironment::new();
        let drawing =
            import_drawing(bytes.clone(), "/tmp/x/part.pdf", &hash, &[A4, A4], &mut env).unwrap();
        assert_eq!(drawing.bytes, bytes);
        assert_eq!(drawing.revision.sha256, hash);
        assert_eq!(drawing.revision.file_name, "part.pdf");
        assert_eq!(drawing.revision.sheets.len(), 2);
        assert_eq!(drawing.revision.sheets[1].index, 1);
        assert_eq!(
            drawing.revision.imported_at.as_str(),
            "2026-01-01T00:00:00Z"
        );
    }

    #[test]
    fn import_refuses_bytes_that_were_not_analyzed() {
        let mut env = FixedEnvironment::new();
        let other = sha256(b"other");
        let error = import_drawing(b"%PDF".to_vec(), "a.pdf", &other, &[A4], &mut env);
        assert!(matches!(error, Err(ProjectError::HashMismatch { .. })));
    }

    #[test]
    fn import_refuses_missing_or_invalid_sheets() {
        let mut env = FixedEnvironment::new();
        let bytes = b"%PDF".to_vec();
        let hash = sha256(&bytes);
        let error = import_drawing(bytes.clone(), "a.pdf", &hash, &[], &mut env);
        assert!(matches!(error, Err(ProjectError::InvalidDrawing(_))));
        let flat = SheetInfo {
            size: Size {
                width: 0.0,
                height: 842.0,
            },
            ..A4
        };
        let error = import_drawing(bytes, "a.pdf", &hash, &[A4, flat], &mut env);
        assert!(matches!(error, Err(ProjectError::InvalidDrawing(m)) if m.contains("sheet 2")));
    }

    #[test]
    fn windows_paths_keep_only_the_file_name() {
        let mut env = FixedEnvironment::new();
        let bytes = b"%PDF".to_vec();
        let hash = sha256(&bytes);
        let drawing =
            import_drawing(bytes, r"C:\drawings\part 7.pdf", &hash, &[A4], &mut env).unwrap();
        assert_eq!(drawing.revision.file_name, "part 7.pdf");
    }
}
