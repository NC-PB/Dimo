//! Sandboxed Typst world and render function of the spike.
//!
//! Kept in its own file so the binary size measurement can include it into the desktop app with
//! `#[path]` without touching any crate (see RESULTS.md, "Binary size").

use std::collections::HashMap;

use typst::diag::{FileError, FileResult, PackageError, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime, Duration, Smart};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;

const FONT_REGULAR: &[u8] = include_bytes!("../../../data/fonts/OpenSans-Regular.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../../data/fonts/OpenSans-Bold.ttf");

/// A world with an in-memory file table and the bundled fonts. Nothing else: no file system,
/// no packages, no clock.
pub struct SandboxWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    main: FileId,
    files: HashMap<FileId, Bytes>,
}

fn project_id(path: &str) -> FileId {
    let vpath = VirtualPath::new(path).expect("valid virtual path");
    RootedPath::new(VirtualRoot::Project, vpath).intern()
}

impl SandboxWorld {
    /// `main_source` becomes `/main.typ`, `data_json` becomes `/data.json`.
    pub fn new(main_source: &str, data_json: &str) -> Self {
        let fonts: Vec<Font> = [FONT_REGULAR, FONT_BOLD]
            .into_iter()
            .flat_map(|data| Font::iter(Bytes::new(data)))
            .collect();
        let book = FontBook::from_fonts(&fonts);
        let main = project_id("/main.typ");
        let mut files = HashMap::new();
        files.insert(main, Bytes::from_string(main_source.to_string()));
        files.insert(
            project_id("/data.json"),
            Bytes::from_string(data_json.to_string()),
        );
        Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            fonts,
            main,
            files,
        }
    }
}

impl World for SandboxWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }
    fn main(&self) -> FileId {
        self.main
    }
    fn source(&self, id: FileId) -> FileResult<Source> {
        let bytes = self.file(id)?;
        let text = bytes.as_str().map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, text.to_string()))
    }
    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let VirtualRoot::Package(spec) = id.root() {
            return Err(FileError::Package(PackageError::Other(Some(
                format!("packages are not available in reports: {spec:?}").into(),
            ))));
        }
        self.files.get(&id).cloned().ok_or_else(|| {
            FileError::Other(Some(
                format!(
                    "file {} is not part of the template",
                    id.vpath().get_with_slash()
                )
                .into(),
            ))
        })
    }
    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

fn describe(errors: &[SourceDiagnostic]) -> String {
    errors
        .iter()
        .map(|e| e.message.to_string())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Compiles the world and exports a PDF with a fixed identifier and no timestamp.
/// Returns the PDF bytes and the page count.
pub fn render(world: &SandboxWorld, ident: &str) -> Result<(Vec<u8>, usize), String> {
    let warned = typst::compile::<PagedDocument>(world);
    for w in &warned.warnings {
        eprintln!("warning: {}", w.message);
    }
    let doc = warned.output.map_err(|e| describe(&e))?;
    let options = PdfOptions {
        ident: Smart::Custom(ident.to_string()),
        creator: Smart::Custom(Some("Dimo spike".into())),
        timestamp: None,
        ..PdfOptions::default()
    };
    let pdf = typst_pdf::pdf(&doc, &options).map_err(|e| describe(&e))?;
    Ok((pdf, doc.pages().len()))
}
