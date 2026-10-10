//! The `.dimo` project file (data model 07, ADR 0004, NFR-REL-01, NFR-REL-03, NFR-SEC-05).
//!
//! # Layout
//!
//! ```text
//! project.dimo            ZIP container, or a folder with the same entries (folder mode)
//!   manifest.json         format name, schema version, app version, created, modified
//!   project.json          dimo_core::Project, schema in docs/schema/project.schema.json
//!   audit.jsonl           one dimo_core::AuditEntry per line, append only
//!   tolerances/<id>.toml  custom tolerance tables used by the project (M2 decision 4)
//!   drawings/<sha256>.pdf original drawing files, byte identical to the import (FR-DOC-07)
//! ```
//!
//! `profile.toml` and `templates/` from the data model are added together with profiles and
//! issued reports, with a schema version increase.
//!
//! # Determinism
//!
//! The same [`ProjectFile`] always gives the same bytes (FR-EXP-11, rule 11): entries in the
//! order above with tables sorted by id and drawings sorted by hash, JSON from the Rust types
//! (no maps with random order), every ZIP entry stored uncompressed with the manifest's
//! `modified` time, fixed permissions and host system. `modified` is the time of the last audit
//! entry, so saving an unchanged project after loading it gives identical bytes.
//!
//! # Modules
//!
//! - [`ProjectFile`]: load and save in either layout, with migrations and drawing hash checks.
//! - [`import`]: a drawing's bytes and sheet data become a [`DrawingRevision`](dimo_core::DrawingRevision).
//! - [`ProjectSession`]: an open project with its autosave journal ([`crate::journal`]).

mod container;
mod error;
pub mod import;
mod manifest;
pub mod migrate;
mod session;

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use dimo_core::{AuditEntry, Project, Sha256Hex, Timestamp};
use sha2::{Digest as _, Sha256};

pub use container::Limits;
pub(crate) use container::write_atomic;
pub use error::ProjectError;
pub use import::{ImportedDrawing, SheetInfo, import_drawing};
pub use manifest::Manifest;
pub use session::{OpenReport, ProjectSession};

/// The `format` value in every manifest.
pub const FORMAT_NAME: &str = "dimo-project";
/// The schema version this build writes and the highest it reads (NFR-REL-03).
pub const SCHEMA_VERSION: u32 = 2;
/// File extension of project files, without the dot.
pub const EXTENSION: &str = "dimo";
/// Version of the Dimo build, written to the manifest.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Entry name of the manifest.
pub const MANIFEST: &str = "manifest.json";
/// Entry name of the project data.
pub const PROJECT: &str = "project.json";
/// Entry name of the audit log.
pub const AUDIT: &str = "audit.jsonl";
/// Folder of the drawing files.
pub const DRAWINGS_DIR: &str = "drawings";
/// Folder of the custom tolerance tables (M2 decision 4).
pub const TOLERANCES_DIR: &str = "tolerances";

/// How a project is stored on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Layout {
    /// One ZIP file (the default).
    #[default]
    Zip,
    /// A folder with the same entries, for version control and debugging.
    Folder,
}

impl Layout {
    /// Folder mode if `path` is an existing folder, else ZIP.
    pub fn of(path: &Path) -> Self {
        if path.is_dir() {
            Self::Folder
        } else {
            Self::Zip
        }
    }
}

/// SHA-256 of bytes as hex (FR-DOC-07).
pub fn sha256(bytes: &[u8]) -> Sha256Hex {
    Sha256Hex::from_bytes(&Sha256::digest(bytes).into())
}

/// The JSON Schema of `project.json`, as published in `docs/schema/project.schema.json`
/// (ADR 0004). Generated from [`Project`], keys sorted, pretty printed with a final newline.
pub fn schema_json() -> String {
    // Serializing a `Value` cannot fail: all keys are strings.
    let mut text = serde_json::to_string_pretty(&dimo_core::project_schema()).unwrap_or_default();
    text.push('\n');
    text
}

/// Everything stored in a project file.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectFile {
    /// When the project was created (manifest `created`).
    pub created: Timestamp,
    /// The project (`project.json`).
    pub project: Project,
    /// The audit log, oldest first (`audit.jsonl`).
    pub audit: Vec<AuditEntry>,
    /// Drawing files by their SHA-256. Only drawings of a revision are saved.
    pub drawings: BTreeMap<Sha256Hex, Vec<u8>>,
    /// Custom tolerance table files (TOML) by their SHA-256. Only tables listed in the tolerance
    /// settings are saved, as `tolerances/<id>.toml` (M2 decision 4, FR-TOL-07).
    pub tables: BTreeMap<Sha256Hex, Vec<u8>>,
}

/// A project read from disk.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedProject {
    /// The content, migrated to [`SCHEMA_VERSION`].
    pub file: ProjectFile,
    /// The manifest as stored.
    pub manifest: Manifest,
}

impl LoadedProject {
    /// The schema version the file had before migration, if it was older.
    pub fn migrated_from(&self) -> Option<u32> {
        (self.manifest.schema_version != SCHEMA_VERSION).then_some(self.manifest.schema_version)
    }
}

impl ProjectFile {
    /// A project file without audit entries and drawings.
    pub fn new(project: Project, created: Timestamp) -> Self {
        Self {
            created,
            project,
            audit: Vec::new(),
            drawings: BTreeMap::new(),
            tables: BTreeMap::new(),
        }
    }

    /// Adds a drawing file and returns its hash. Adding the same bytes twice keeps one copy.
    pub fn insert_drawing(&mut self, bytes: Vec<u8>) -> Sha256Hex {
        let hash = sha256(&bytes);
        self.drawings.entry(hash.clone()).or_insert(bytes);
        hash
    }

    /// The drawing file with this hash.
    pub fn drawing(&self, hash: &Sha256Hex) -> Option<&[u8]> {
        self.drawings.get(hash).map(Vec::as_slice)
    }

    /// Adds a custom tolerance table file and returns its hash, for a
    /// [`CustomTable`](dimo_core::CustomTable) entry of the tolerance settings. The bytes are
    /// stored as given; `dimo-tolerance` validates them (FR-TOL-07).
    pub fn insert_table(&mut self, bytes: Vec<u8>) -> Sha256Hex {
        let hash = sha256(&bytes);
        self.tables.entry(hash.clone()).or_insert(bytes);
        hash
    }

    /// The custom tolerance table file with this hash.
    pub fn table(&self, hash: &Sha256Hex) -> Option<&[u8]> {
        self.tables.get(hash).map(Vec::as_slice)
    }

    /// The manifest written for this content.
    pub fn manifest(&self) -> Manifest {
        Manifest {
            format: FORMAT_NAME.to_owned(),
            schema_version: SCHEMA_VERSION,
            app_version: APP_VERSION.to_owned(),
            created: self.created.clone(),
            modified: self
                .audit
                .last()
                .map_or_else(|| self.created.clone(), |entry| entry.timestamp.clone()),
        }
    }

    /// The entries in container order. Fails if a revision's drawing is missing.
    fn entries(&self) -> Result<Vec<container::Entry<'_>>, ProjectError> {
        let json = |name: &str, result: serde_json::Result<Vec<u8>>| {
            result
                .map(|mut bytes| {
                    bytes.push(b'\n');
                    bytes
                })
                .map_err(|e| ProjectError::json(name, &e))
        };
        let manifest = json(MANIFEST, serde_json::to_vec_pretty(&self.manifest()))?;
        let project = json(PROJECT, serde_json::to_vec_pretty(&self.project))?;
        let mut audit = Vec::new();
        for entry in &self.audit {
            serde_json::to_writer(&mut audit, entry).map_err(|e| ProjectError::json(AUDIT, &e))?;
            audit.push(b'\n');
        }
        let mut entries = vec![
            (MANIFEST.to_owned(), Cow::Owned(manifest)),
            (PROJECT.to_owned(), Cow::Owned(project)),
            (AUDIT.to_owned(), Cow::Owned(audit)),
        ];
        // Sorted by id (the settings keep them sorted, ids unique).
        let mut tables: Vec<&dimo_core::CustomTable> = self
            .project
            .settings
            .tolerance
            .custom_tables
            .iter()
            .collect();
        tables.sort_by(|a, b| a.table.id.cmp(&b.table.id));
        for table in tables {
            let name = format!("{TOLERANCES_DIR}/{}.toml", table.table.id);
            if !dimo_core::project::is_table_id(&table.table.id)
                || entries.iter().any(|(n, _)| *n == name)
            {
                return Err(ProjectError::InvalidTable(table.table.id.clone()));
            }
            let bytes = self
                .tables
                .get(&table.sha256)
                .ok_or_else(|| ProjectError::MissingTable(table.table.id.clone()))?;
            entries.push((name, Cow::Borrowed(bytes.as_slice())));
        }
        let hashes: BTreeSet<&Sha256Hex> =
            self.project.revisions.iter().map(|r| &r.sha256).collect();
        for hash in hashes {
            let bytes = self
                .drawings
                .get(hash)
                .ok_or_else(|| ProjectError::MissingDrawing(hash.clone()))?;
            entries.push((
                format!("{DRAWINGS_DIR}/{hash}.pdf"),
                Cow::Borrowed(bytes.as_slice()),
            ));
        }
        Ok(entries)
    }

    /// The ZIP container as bytes. Deterministic (see the module docs).
    pub fn to_zip(&self) -> Result<Vec<u8>, ProjectError> {
        container::write_zip(
            &self.entries()?,
            container::zip_time(&self.manifest().modified),
        )
        .map_err(ProjectError::Container)
    }

    /// Reads a ZIP container from memory with the default [`Limits`].
    pub fn from_zip(bytes: &[u8]) -> Result<LoadedProject, ProjectError> {
        Self::from_zip_with_limits(bytes, &Limits::DEFAULT)
    }

    /// Reads a ZIP container from memory.
    pub fn from_zip_with_limits(
        bytes: &[u8],
        limits: &Limits,
    ) -> Result<LoadedProject, ProjectError> {
        parse(container::read_zip(bytes, limits)?)
    }

    /// Reads a project folder (folder mode) with the default [`Limits`].
    pub fn from_folder(dir: &Path) -> Result<LoadedProject, ProjectError> {
        parse(container::read_folder(dir, &Limits::DEFAULT)?)
    }

    /// Reads a project from a ZIP file or a project folder.
    pub fn load(path: &Path) -> Result<LoadedProject, ProjectError> {
        match Layout::of(path) {
            Layout::Folder => Self::from_folder(path),
            Layout::Zip => {
                let limits = Limits::DEFAULT;
                let meta = fs::metadata(path).map_err(|e| ProjectError::io(path, e))?;
                if meta.len() > limits.max_total {
                    return Err(ProjectError::TooLarge {
                        name: path.display().to_string(),
                        limit: limits.max_total,
                    });
                }
                let bytes = fs::read(path).map_err(|e| ProjectError::io(path, e))?;
                Self::from_zip_with_limits(&bytes, &limits)
            }
        }
    }

    /// Writes the project. ZIP files are replaced atomically; in folder mode each entry is.
    pub fn save(&self, path: &Path, layout: Layout) -> Result<(), ProjectError> {
        match layout {
            Layout::Zip => container::write_atomic(path, &self.to_zip()?),
            Layout::Folder => container::write_folder(path, &self.entries()?),
        }
    }
}

/// Parses raw entries: manifest check, migration, types, drawing hashes.
fn parse(mut raw: container::RawEntries) -> Result<LoadedProject, ProjectError> {
    let manifest = manifest::parse(&raw.manifest.ok_or(ProjectError::MissingEntry(MANIFEST))?)?;
    let project_bytes = raw.project.ok_or(ProjectError::MissingEntry(PROJECT))?;
    let mut project: serde_json::Value =
        serde_json::from_slice(&project_bytes).map_err(|e| ProjectError::json(PROJECT, &e))?;
    let audit_bytes = raw.audit.unwrap_or_default();
    let audit_text = std::str::from_utf8(&audit_bytes).map_err(|e| ProjectError::Json {
        entry: AUDIT.to_owned(),
        message: e.to_string(),
    })?;
    let mut audit = Vec::new();
    for (index, line) in audit_text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value = serde_json::from_str(line)
            .map_err(|e| ProjectError::json(format!("{AUDIT} line {}", index + 1), &e))?;
        audit.push(value);
    }

    migrate::migrate(manifest.schema_version, &mut project, &mut audit)?;

    let project: Project =
        serde_json::from_value(project).map_err(|e| ProjectError::json(PROJECT, &e))?;
    let audit = audit
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            serde_json::from_value(value)
                .map_err(|e| ProjectError::json(format!("{AUDIT} entry {}", index + 1), &e))
        })
        .collect::<Result<Vec<AuditEntry>, _>>()?;

    let mut drawings = BTreeMap::new();
    for revision in &project.revisions {
        let hash = &revision.sha256;
        if drawings.contains_key(hash) {
            continue;
        }
        let bytes = raw
            .drawings
            .remove(hash)
            .ok_or_else(|| ProjectError::MissingDrawing(hash.clone()))?;
        let actual = sha256(&bytes);
        if actual != *hash {
            return Err(ProjectError::HashMismatch {
                expected: hash.clone(),
                actual,
            });
        }
        drawings.insert(hash.clone(), bytes);
    }

    let mut tables = BTreeMap::new();
    for table in &project.settings.tolerance.custom_tables {
        let id = &table.table.id;
        let bytes = raw
            .tables
            .remove(id)
            .ok_or_else(|| ProjectError::MissingTable(id.clone()))?;
        let actual = sha256(&bytes);
        if actual != table.sha256 {
            return Err(ProjectError::TableHashMismatch {
                id: id.clone(),
                expected: table.sha256.clone(),
                actual,
            });
        }
        tables.insert(actual, bytes);
    }

    Ok(LoadedProject {
        file: ProjectFile {
            created: manifest.created.clone(),
            project,
            audit,
            drawings,
            tables,
        },
        manifest,
    })
}
