//! Report templating and PDF rendering of inspection reports (D-30, FR-EXP-06).
//!
//! [`render_pdf`] compiles one document of a [`TemplateSet`] with Typst inside a sandboxed world
//! (NFR-SEC-05) and returns the PDF bytes. Rust passes every value already formatted as text in
//! one JSON data file, so templates never compute with numbers or decide pass or fail
//! (M3 decision 10). The output is deterministic (FR-EXP-11, rule 11): no clock, the PDF
//! identifier is a hash of the inputs, and the creation date comes from the request.

mod template;
mod world;

use std::fmt::{self, Write as _};
use std::ops::Range;

use sha2::{Digest, Sha256};
use typst::WorldExt;
use typst::diag::{Severity, SourceDiagnostic};
use typst::foundations::{Datetime, Smart};
use typst_layout::PagedDocument;
use typst_pdf::{PdfOptions, Timestamp};

pub use template::{DATA_FILE, DocumentSpec, MANIFEST, TemplateSet, bundled_ids};

use world::SandboxWorld;

/// Report language (D-32), chosen per export independent of the UI language. Templates see it
/// as `sys.inputs.lang`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    /// English.
    En,
    /// German.
    De,
}

impl Language {
    /// Two letter code passed to the template (`en`, `de`).
    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::De => "de",
        }
    }
}

/// A calendar date (from the project, never from the clock).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReportDate {
    /// Year, for example 2026.
    pub year: i32,
    /// Month 1 to 12.
    pub month: u8,
    /// Day 1 to 31.
    pub day: u8,
}

/// What to render.
#[derive(Debug, Clone, Copy)]
pub struct RenderRequest<'a> {
    /// Name of the document in the template manifest.
    pub document: &'a str,
    /// The report data, written to `/data.json` for the template.
    pub data: &'a serde_json::Value,
    /// Report language.
    pub language: Language,
    /// Creation date written into the PDF metadata; `None` writes no date.
    pub date: Option<ReportDate>,
}

/// A Typst error or warning with its location in the template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Path of the template file, `None` when the message has no location.
    pub file: Option<String>,
    /// Line, 1 based.
    pub line: Option<usize>,
    /// Column in characters, 1 based.
    pub column: Option<usize>,
    /// Message from Typst.
    pub message: String,
    /// Hints from Typst.
    pub hints: Vec<String>,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (&self.file, self.line, self.column) {
            (Some(file), Some(line), Some(column)) => write!(f, "{file}:{line}:{column}: ")?,
            (Some(file), _, _) => write!(f, "{file}: ")?,
            _ => {}
        }
        f.write_str(&self.message)?;
        for hint in &self.hints {
            write!(f, " (hint: {hint})")?;
        }
        Ok(())
    }
}

/// Errors of report rendering.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    /// No bundled template set has this id.
    #[error("unknown report template {0}")]
    UnknownTemplate(String),
    /// The template set has no document with this name.
    #[error("report template {template} has no document {document}")]
    UnknownDocument {
        /// Template id.
        template: String,
        /// Requested document name.
        document: String,
    },
    /// The template set is malformed (manifest, paths).
    #[error("invalid report template: {0}")]
    InvalidTemplate(String),
    /// The date is not a valid calendar date.
    #[error("invalid report date {0:?}")]
    InvalidDate(ReportDate),
    /// The data could not be serialized.
    #[error("report data: {0}")]
    Data(#[from] serde_json::Error),
    /// Typst could not compile the template; the first error is shown.
    #[error("report template error: {}", first(.0))]
    Compile(Vec<Diagnostic>),
    /// Typst could not write the PDF.
    #[error("PDF export failed: {}", first(.0))]
    Pdf(Vec<Diagnostic>),
}

fn first(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .first()
        .map_or_else(|| "unknown error".to_string(), ToString::to_string)
}

/// Renders one document of a template set to PDF bytes.
///
/// Same template, request and app version give byte identical output (FR-EXP-11). Warnings
/// from Typst are logged with `tracing`; errors are returned with file and line.
pub fn render_pdf(
    template: &TemplateSet,
    request: &RenderRequest<'_>,
) -> Result<Vec<u8>, ReportError> {
    let document = template.document(request.document)?;
    let timestamp = request
        .date
        .map(|date| {
            Datetime::from_ymd(date.year, date.month, date.day)
                .map(Timestamp::new_utc)
                .ok_or(ReportError::InvalidDate(date))
        })
        .transpose()?;
    let data = serde_json::to_vec(request.data)?;
    let ident = document_ident(template, request, &data);
    let world = SandboxWorld::new(template, &document.main, data, request.language.code())
        .ok_or_else(|| ReportError::InvalidTemplate("a file path is not valid in Typst".into()))?;

    let warned = typst::compile::<PagedDocument>(&world);
    for warning in &warned.warnings {
        tracing::warn!(target: "dimo_report", "{}", to_diagnostic(&world, warning));
    }
    let result = warned
        .output
        .map_err(|errors| ReportError::Compile(to_diagnostics(&world, &errors)))
        .and_then(|doc| {
            let options = PdfOptions {
                ident: Smart::Custom(ident),
                creator: Smart::Custom(Some("Dimo".into())),
                timestamp,
                ..PdfOptions::default()
            };
            typst_pdf::pdf(&doc, &options)
                .map_err(|errors| ReportError::Pdf(to_diagnostics(&world, &errors)))
        });
    // Bound the memory of Typst's global cache across many exports (as the Typst CLI does).
    typst::comemo::evict(10);
    result
}

/// Stable PDF document identifier: SHA-256 over template id and version, document, language,
/// date and data, so the same report always gets the same identifier.
fn document_ident(template: &TemplateSet, request: &RenderRequest<'_>, data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    for part in [
        template.id().as_bytes(),
        &template.version().to_le_bytes(),
        request.document.as_bytes(),
        request.language.code().as_bytes(),
        format!("{:?}", request.date).as_bytes(),
        data,
    ] {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize().iter().fold(String::new(), |mut hex, b| {
        let _ = write!(hex, "{b:02x}");
        hex
    })
}

fn to_diagnostics(world: &SandboxWorld, diagnostics: &[SourceDiagnostic]) -> Vec<Diagnostic> {
    diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| to_diagnostic(world, d))
        .collect()
}

fn to_diagnostic(world: &SandboxWorld, diagnostic: &SourceDiagnostic) -> Diagnostic {
    let id = diagnostic.span.id();
    let file = id.map(|id| id.vpath().get_without_slash().to_string());
    let position = id
        .zip(world.range(diagnostic.span))
        .and_then(|(id, range)| line_column(world, id, range));
    Diagnostic {
        file,
        line: position.map(|(line, _)| line + 1),
        column: position.map(|(_, column)| column + 1),
        message: diagnostic.message.to_string(),
        hints: diagnostic.hints.iter().map(|h| h.v.to_string()).collect(),
    }
}

fn line_column(
    world: &SandboxWorld,
    id: typst::syntax::FileId,
    range: Range<usize>,
) -> Option<(usize, usize)> {
    use typst::World;
    world
        .source(id)
        .ok()?
        .lines()
        .byte_to_line_column(range.start)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The characters reports need (M3 T3.2) are in both bundled fonts; U+2300 is not and is
    /// mapped to Ø by the templates.
    #[test]
    fn bundled_fonts_cover_report_characters() {
        let fonts = &world::FONTS.0;
        assert_eq!(fonts.len(), 2);
        for font in fonts {
            for c in "Ø±°′″×−äöüÄÖÜß0123456789.,".chars() {
                assert!(
                    font.info().coverage.contains(c as u32),
                    "{:?} lacks {c:?}",
                    font.info().family
                );
            }
            assert!(!font.info().coverage.contains('⌀' as u32));
        }
    }

    #[test]
    fn every_file_of_the_bundled_folder_is_embedded() {
        // Read at run time so a shared target directory does not bake in a stale path.
        let dir = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
        let root = std::path::Path::new(&dir).join("../../data/templates/dimo-default");
        let mut on_disk: Vec<String> = std::fs::read_dir(&root)
            .expect("template folder")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        on_disk.sort();
        let set = TemplateSet::bundled("dimo-default").expect("bundled set");
        let embedded: Vec<String> = set.files().keys().cloned().collect();
        assert_eq!(on_disk, embedded);
    }
}
