//! `manifest.json`: format name, schema version, app version, created and modified (data model
//! 07, project file format).

use dimo_core::Timestamp;
use serde::{Deserialize, Serialize};

use super::error::ProjectError;
use super::{FORMAT_NAME, SCHEMA_VERSION};

/// Content of `manifest.json`.
///
/// `format` and `schema_version` keep their names and meaning in every version, so any Dimo
/// can tell whether it may read a file (NFR-REL-04).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Manifest {
    /// Always [`FORMAT_NAME`].
    pub format: String,
    /// Version of the `project.json` and `audit.jsonl` layout (NFR-REL-03).
    pub schema_version: u32,
    /// Version of the Dimo build that wrote the file.
    pub app_version: String,
    /// When the project was created.
    pub created: Timestamp,
    /// Time of the last audited change, or `created` if there is none. Derived from the
    /// project, so saving an unchanged project gives identical bytes (FR-EXP-11).
    pub modified: Timestamp,
}

/// The part of the manifest that is stable across versions.
#[derive(Deserialize)]
struct ManifestHead {
    format: String,
    schema_version: u32,
}

/// Checks format and version, then parses the manifest of a file with `schema_version`.
///
/// Returns the manifest and the schema version found.
pub(crate) fn parse(bytes: &[u8]) -> Result<Manifest, ProjectError> {
    let head: ManifestHead = serde_json::from_slice(bytes)
        .map_err(|e| ProjectError::NotAProject(format!("{}: {e}", super::MANIFEST)))?;
    if head.format != FORMAT_NAME {
        return Err(ProjectError::NotAProject(format!(
            "format is {:?}, expected {FORMAT_NAME:?}",
            head.format
        )));
    }
    if head.schema_version > SCHEMA_VERSION {
        return Err(ProjectError::NewerVersion {
            found: head.schema_version,
            supported: SCHEMA_VERSION,
        });
    }
    if head.schema_version == 0 {
        return Err(ProjectError::UnknownVersion(0));
    }
    // The manifest layout has not changed since version 1. A future change migrates here.
    serde_json::from_slice(bytes).map_err(|e| ProjectError::json(super::MANIFEST, &e))
}
