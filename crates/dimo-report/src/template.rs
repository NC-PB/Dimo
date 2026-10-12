//! Report template sets (FR-EXP-06, M3 decision 10).
//!
//! A template set is a folder with a `template.toml` manifest and Typst files. The bundled set
//! `dimo-default` lives in `data/templates/dimo-default/` and is embedded at compile time, so
//! rendering never reads the file system (NFR-SEC-05).

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::ReportError;

/// Name of the manifest file inside a template set.
pub const MANIFEST: &str = "template.toml";

/// Path of the data file the renderer adds next to the template files. Templates read it with
/// `json("/data.json")`; a template set must not contain a file with this name.
pub const DATA_FILE: &str = "data.json";

/// Files of the bundled `dimo-default` set, embedded at compile time. Every file of the folder
/// must be listed here (checked by a test).
const DIMO_DEFAULT: EmbeddedFiles = &[
    (
        MANIFEST,
        include_bytes!("../../../data/templates/dimo-default/template.toml"),
    ),
    (
        "placeholder.typ",
        include_bytes!("../../../data/templates/dimo-default/placeholder.typ"),
    ),
];

/// Files of a template set compiled into the binary: `(relative path, content)`.
type EmbeddedFiles = &'static [(&'static str, &'static [u8])];

/// Bundled template sets by id.
const BUNDLED: &[(&str, EmbeddedFiles)] = &[("dimo-default", DIMO_DEFAULT)];

/// Ids of the template sets compiled into the binary.
pub fn bundled_ids() -> impl Iterator<Item = &'static str> {
    BUNDLED.iter().map(|(id, _)| *id)
}

/// One document of a template set, for example the check sheet.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentSpec {
    /// Name used to select the document when rendering.
    pub name: String,
    /// Typst file of the set that is compiled for this document.
    pub main: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    id: String,
    version: u32,
    documents: Vec<DocumentSpec>,
}

/// A validated template set held in memory.
#[derive(Debug, Clone)]
pub struct TemplateSet {
    id: String,
    version: u32,
    documents: Vec<DocumentSpec>,
    files: BTreeMap<String, Vec<u8>>,
}

impl TemplateSet {
    /// The bundled template set with this id.
    pub fn bundled(id: &str) -> Result<Self, ReportError> {
        let (_, files) = BUNDLED
            .iter()
            .find(|(bundled, _)| *bundled == id)
            .ok_or_else(|| ReportError::UnknownTemplate(id.to_string()))?;
        Self::from_files(
            files
                .iter()
                .map(|(path, bytes)| ((*path).to_string(), bytes.to_vec())),
        )
    }

    /// Builds a template set from `(relative path, content)` pairs. Paths use `/` and stay
    /// inside the set (no leading `/`, no `.` or `..` segments). One of them must be the
    /// manifest `template.toml`.
    pub fn from_files(
        files: impl IntoIterator<Item = (String, Vec<u8>)>,
    ) -> Result<Self, ReportError> {
        let mut map = BTreeMap::new();
        for (path, bytes) in files {
            check_path(&path)?;
            if path == DATA_FILE {
                return Err(ReportError::InvalidTemplate(format!(
                    "the file name {DATA_FILE} is reserved for the report data"
                )));
            }
            map.insert(path, bytes);
        }
        let manifest = map
            .get(MANIFEST)
            .ok_or_else(|| ReportError::InvalidTemplate(format!("{MANIFEST} is missing")))?;
        let manifest = std::str::from_utf8(manifest)
            .map_err(|_| ReportError::InvalidTemplate(format!("{MANIFEST} is not UTF-8")))?;
        let manifest: Manifest = toml::from_str(manifest)
            .map_err(|err| ReportError::InvalidTemplate(format!("{MANIFEST}: {err}")))?;
        if manifest.id.trim().is_empty() {
            return Err(ReportError::InvalidTemplate("empty template id".into()));
        }
        for document in &manifest.documents {
            if !map.contains_key(&document.main) {
                return Err(ReportError::InvalidTemplate(format!(
                    "document {} names the missing file {}",
                    document.name, document.main
                )));
            }
        }
        Ok(Self {
            id: manifest.id,
            version: manifest.version,
            documents: manifest.documents,
            files: map,
        })
    }

    /// Template id from the manifest.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Template version from the manifest.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Documents in manifest order.
    pub fn documents(&self) -> &[DocumentSpec] {
        &self.documents
    }

    /// The document with this name.
    pub fn document(&self, name: &str) -> Result<&DocumentSpec, ReportError> {
        self.documents
            .iter()
            .find(|d| d.name == name)
            .ok_or_else(|| ReportError::UnknownDocument {
                template: self.id.clone(),
                document: name.to_string(),
            })
    }

    /// All files of the set, by relative path.
    pub fn files(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.files
    }
}

fn check_path(path: &str) -> Result<(), ReportError> {
    let bad = path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..");
    if bad {
        return Err(ReportError::InvalidTemplate(format!(
            "invalid file path in template: {path:?}"
        )));
    }
    Ok(())
}
