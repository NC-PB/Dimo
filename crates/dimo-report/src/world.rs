//! Sandboxed Typst world (NFR-SEC-01, NFR-SEC-05, M3 decision 11).
//!
//! Serves exactly the files of one template set, the data file and the bundled fonts from
//! memory. There is no file system access, no package resolver (so no downloads) and no clock:
//! `import`, `include`, `read`, `image`, `plugin` and friends can only reach these files.

use std::collections::HashMap;
use std::sync::LazyLock;

use typst::diag::{FileError, FileResult, PackageError};
use typst::foundations::{Bytes, Datetime, Dict, Duration, Str, Value};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

use crate::template::{DATA_FILE, TemplateSet};

/// Open Sans Regular and Bold (`data/fonts/README.md`), the only fonts reports can use.
const FONT_FILES: &[&[u8]] = &[
    include_bytes!("../../../data/fonts/OpenSans-Regular.ttf"),
    include_bytes!("../../../data/fonts/OpenSans-Bold.ttf"),
];

/// Parsed bundled fonts and their book, shared by all renders.
pub(crate) static FONTS: LazyLock<(Vec<Font>, LazyHash<FontBook>)> = LazyLock::new(|| {
    let fonts: Vec<Font> = FONT_FILES
        .iter()
        .flat_map(|data| Font::iter(Bytes::new(*data)))
        .collect();
    let book = LazyHash::new(FontBook::from_fonts(&fonts));
    (fonts, book)
});

/// File id of a path in the project root of the virtual file system.
fn project_id(path: &str) -> Option<FileId> {
    let vpath = VirtualPath::new(format!("/{path}")).ok()?;
    Some(RootedPath::new(VirtualRoot::Project, vpath).intern())
}

/// The world of one render.
pub(crate) struct SandboxWorld {
    library: LazyHash<Library>,
    main: FileId,
    files: HashMap<FileId, Bytes>,
}

impl SandboxWorld {
    /// `main` is the path of the document's Typst file inside the template set; `lang` is
    /// visible to the template as `sys.inputs.lang`.
    pub(crate) fn new(
        template: &TemplateSet,
        main: &str,
        data_json: Vec<u8>,
        lang: &str,
    ) -> Option<Self> {
        let mut files = HashMap::new();
        for (path, bytes) in template.files() {
            files.insert(project_id(path)?, Bytes::new(bytes.clone()));
        }
        files.insert(project_id(DATA_FILE)?, Bytes::new(data_json));
        let mut inputs = Dict::new();
        inputs.insert(Str::from("lang"), Value::Str(Str::from(lang)));
        Some(Self {
            library: LazyHash::new(Library::builder().with_inputs(inputs).build()),
            main: project_id(main)?,
            files,
        })
    }
}

impl World for SandboxWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &FONTS.1
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
                format!("packages are not available in report templates ({spec:?})").into(),
            ))));
        }
        self.files.get(&id).cloned().ok_or_else(|| {
            FileError::Other(Some(
                format!(
                    "{} is not a file of the report template",
                    id.vpath().get_with_slash()
                )
                .into(),
            ))
        })
    }

    fn font(&self, index: usize) -> Option<Font> {
        FONTS.0.get(index).cloned()
    }

    /// No clock (rule 11): dates come from the report data.
    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}
