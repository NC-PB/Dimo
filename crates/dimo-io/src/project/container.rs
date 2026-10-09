//! The two storage layouts of a project: a ZIP file and a plain folder (ADR 0004).
//!
//! Both hold the same entries. Reading checks every name and size before any content is used
//! (NFR-SEC-05): names that could leave the container (zip slip), symbolic links, encrypted
//! entries and anything above [`Limits`] are refused. Entries outside the known layout are
//! ignored and dropped on the next save. Nothing in a project is ever executed.
//!
//! Writing is deterministic (FR-EXP-11): entries in a fixed order, all stored uncompressed,
//! one timestamp derived from the project, fixed permissions and host system.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;

use dimo_core::{Sha256Hex, Timestamp};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, System, ZipArchive, ZipWriter};

use super::error::ProjectError;
use super::{AUDIT, DRAWINGS_DIR, MANIFEST, PROJECT};

/// A named entry to write, in container order.
pub(crate) type Entry<'a> = (String, Cow<'a, [u8]>);

/// Size limits applied when reading a project (NFR-SEC-05).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Most entries in a ZIP container, directories included.
    pub max_entries: usize,
    /// Largest `manifest.json`.
    pub max_manifest: u64,
    /// Largest `project.json`.
    pub max_project: u64,
    /// Largest `audit.jsonl`.
    pub max_audit: u64,
    /// Largest single drawing.
    pub max_drawing: u64,
    /// Largest sum of all entries read.
    pub max_total: u64,
}

const MIB: u64 = 1024 * 1024;

impl Limits {
    /// Limits for real projects: far above what a shop produces, far below what exhausts
    /// memory.
    pub const DEFAULT: Self = Self {
        max_entries: 10_000,
        max_manifest: MIB,
        max_project: 256 * MIB,
        max_audit: 1024 * MIB,
        max_drawing: 1024 * MIB,
        max_total: 4096 * MIB,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Entries read from a container, before parsing.
#[derive(Debug, Default)]
pub(crate) struct RawEntries {
    pub manifest: Option<Vec<u8>>,
    pub project: Option<Vec<u8>>,
    pub audit: Option<Vec<u8>>,
    pub drawings: BTreeMap<Sha256Hex, Vec<u8>>,
}

/// What an entry name stands for.
enum Kind {
    Manifest,
    Project,
    Audit,
    Drawing(Sha256Hex),
    Other,
}

fn classify(name: &str) -> Kind {
    match name {
        MANIFEST => Kind::Manifest,
        PROJECT => Kind::Project,
        AUDIT => Kind::Audit,
        _ => name
            .strip_prefix(DRAWINGS_DIR)
            .and_then(|rest| rest.strip_prefix('/'))
            .and_then(|file| file.strip_suffix(".pdf"))
            .and_then(|hex| Sha256Hex::parse(hex).ok())
            .map_or(Kind::Other, Kind::Drawing),
    }
}

/// A relative path of plain components: no root, drive, backslash, `.` or `..` (zip slip).
pub(crate) fn is_safe_name(name: &str) -> bool {
    let path = name.strip_suffix('/').unwrap_or(name);
    !path.is_empty()
        && path.len() <= 1024
        && !path.contains(['\\', ':', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

impl RawEntries {
    /// Stores an entry, checking its limit. Unknown names are ignored.
    fn insert(&mut self, name: &str, data: Vec<u8>) -> Result<(), ProjectError> {
        let duplicate = || ProjectError::DuplicateEntry(name.to_owned());
        let slot = match classify(name) {
            Kind::Manifest => &mut self.manifest,
            Kind::Project => &mut self.project,
            Kind::Audit => &mut self.audit,
            Kind::Drawing(hash) => {
                return match self.drawings.insert(hash, data) {
                    None => Ok(()),
                    Some(_) => Err(duplicate()),
                };
            }
            Kind::Other => return Ok(()),
        };
        if slot.replace(data).is_some() {
            return Err(duplicate());
        }
        Ok(())
    }
}

/// The limit of a known entry, `None` for entries that are skipped.
fn limit_of(name: &str, limits: &Limits) -> Option<u64> {
    match classify(name) {
        Kind::Manifest => Some(limits.max_manifest),
        Kind::Project => Some(limits.max_project),
        Kind::Audit => Some(limits.max_audit),
        Kind::Drawing(_) => Some(limits.max_drawing),
        Kind::Other => None,
    }
}

/// Reads at most `limit` bytes, failing if there are more (sizes in headers can lie).
fn read_limited(reader: impl Read, name: &str, limit: u64) -> Result<Vec<u8>, ProjectError> {
    let mut data = Vec::new();
    reader
        .take(limit.saturating_add(1))
        .read_to_end(&mut data)
        .map_err(|e| ProjectError::Container(format!("{name}: {e}")))?;
    if data.len() as u64 > limit {
        return Err(too_large(name, limit));
    }
    Ok(data)
}

fn too_large(name: &str, limit: u64) -> ProjectError {
    ProjectError::TooLarge {
        name: name.to_owned(),
        limit,
    }
}

/// Adds `len` to the running total, failing above the total limit.
fn count(total: &mut u64, len: usize, limits: &Limits) -> Result<(), ProjectError> {
    *total = total.saturating_add(len as u64);
    if *total > limits.max_total {
        return Err(too_large("project", limits.max_total));
    }
    Ok(())
}

/// Reads the entries of a ZIP container held in memory.
pub(crate) fn read_zip(bytes: &[u8], limits: &Limits) -> Result<RawEntries, ProjectError> {
    let container = |e: zip::result::ZipError| ProjectError::Container(e.to_string());
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(container)?;
    if archive.len() > limits.max_entries {
        return Err(ProjectError::TooManyEntries(limits.max_entries));
    }
    let mut raw = RawEntries::default();
    let mut total = 0;
    for index in 0..archive.len() {
        let file = archive.by_index_raw(index).map_err(container)?;
        let name = file.name().to_owned();
        if !is_safe_name(&name) {
            return Err(ProjectError::UnsafeEntry(name));
        }
        if file.encrypted() {
            return Err(ProjectError::Encrypted(name));
        }
        if file.is_symlink() {
            return Err(ProjectError::Symlink(name));
        }
        if file.is_dir() {
            continue;
        }
        let Some(limit) = limit_of(&name, limits) else {
            continue;
        };
        if file.size() > limit {
            return Err(too_large(&name, limit));
        }
        drop(file);
        let file = archive.by_index(index).map_err(container)?;
        let data = read_limited(file, &name, limit)?;
        count(&mut total, data.len(), limits)?;
        raw.insert(&name, data)?;
    }
    Ok(raw)
}

/// Writes entries as a ZIP container: stored, in the given order, all with timestamp `time`.
pub(crate) fn write_zip(entries: &[Entry<'_>], time: DateTime) -> Result<Vec<u8>, String> {
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(time)
        .unix_permissions(0o644)
        .system(System::Unix);
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer
            .start_file(name.as_str(), options)
            .map_err(|e| e.to_string())?;
        writer.write_all(data).map_err(|e| e.to_string())?;
    }
    let cursor = writer.finish().map_err(|e| e.to_string())?;
    Ok(cursor.into_inner())
}

/// ZIP timestamp from a project timestamp. DOS time has two second resolution and covers
/// 1980 to 2107; times outside fall back to 1980-01-01.
pub(crate) fn zip_time(timestamp: &Timestamp) -> DateTime {
    let text = timestamp.as_str();
    let number = |range: std::ops::Range<usize>| -> Option<u16> { text.get(range)?.parse().ok() };
    let small = |range| number(range).and_then(|n| u8::try_from(n).ok());
    let parts = (
        number(0..4),
        small(5..7),
        small(8..10),
        small(11..13),
        small(14..16),
        small(17..19),
    );
    if let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = parts {
        DateTime::from_date_and_time(year, month, day, hour, minute, second).unwrap_or_default()
    } else {
        DateTime::default()
    }
}

/// Reads a regular file of at most `limit` bytes. Symbolic links are refused.
fn read_file(path: &Path, name: &str, limit: u64) -> Result<Option<Vec<u8>>, ProjectError> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ProjectError::io(path, e)),
    };
    if meta.file_type().is_symlink() {
        return Err(ProjectError::Symlink(name.to_owned()));
    }
    if !meta.is_file() {
        return Ok(None);
    }
    if meta.len() > limit {
        return Err(too_large(name, limit));
    }
    let file = fs::File::open(path).map_err(|e| ProjectError::io(path, e))?;
    read_limited(file, name, limit).map(Some)
}

/// Reads the entries of a project folder (folder mode).
pub(crate) fn read_folder(dir: &Path, limits: &Limits) -> Result<RawEntries, ProjectError> {
    let mut raw = RawEntries::default();
    let mut total = 0;
    for name in [MANIFEST, PROJECT, AUDIT] {
        if let Some(limit) = limit_of(name, limits)
            && let Some(data) = read_file(&dir.join(name), name, limit)?
        {
            count(&mut total, data.len(), limits)?;
            raw.insert(name, data)?;
        }
    }
    let drawings = dir.join(DRAWINGS_DIR);
    match fs::symlink_metadata(&drawings) {
        Ok(meta) if meta.file_type().is_symlink() => {
            return Err(ProjectError::Symlink(DRAWINGS_DIR.to_owned()));
        }
        Ok(meta) if meta.is_dir() => {
            let listing = fs::read_dir(&drawings).map_err(|e| ProjectError::io(&drawings, e))?;
            let mut names = Vec::new();
            for entry in listing {
                let entry = entry.map_err(|e| ProjectError::io(&drawings, e))?;
                if let Some(file) = entry.file_name().to_str() {
                    names.push(format!("{DRAWINGS_DIR}/{file}"));
                }
                if names.len() > limits.max_entries {
                    return Err(ProjectError::TooManyEntries(limits.max_entries));
                }
            }
            names.sort();
            for name in names {
                if let Some(limit) = limit_of(&name, limits)
                    && let Some(data) = read_file(&dir.join(&name), &name, limit)?
                {
                    count(&mut total, data.len(), limits)?;
                    raw.insert(&name, data)?;
                }
            }
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(ProjectError::io(&drawings, e)),
    }
    Ok(raw)
}

/// Replaces `path` with `data` atomically: a temporary file in the same folder, flushed to
/// disk, then renamed over the target. A crash leaves the old or the new file, never a mix.
pub(crate) fn write_atomic(path: &Path, data: &[u8]) -> Result<(), ProjectError> {
    let dir = match path.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    let io = |e| ProjectError::io(path, e);
    let mut temp = tempfile::NamedTempFile::new_in(dir).map_err(io)?;
    temp.write_all(data).map_err(io)?;
    temp.as_file().sync_all().map_err(io)?;
    temp.persist(path).map_err(|e| io(e.error))?;
    Ok(())
}

/// Writes entries into a project folder and removes drawings that are no longer part of it.
pub(crate) fn write_folder(dir: &Path, entries: &[Entry<'_>]) -> Result<(), ProjectError> {
    let drawings = dir.join(DRAWINGS_DIR);
    fs::create_dir_all(&drawings).map_err(|e| ProjectError::io(&drawings, e))?;
    // Drawings first and the manifest last, so a folder with a new manifest is complete.
    for (name, data) in entries.iter().rev() {
        write_atomic(&dir.join(name), data)?;
    }
    let listing = fs::read_dir(&drawings).map_err(|e| ProjectError::io(&drawings, e))?;
    for entry in listing {
        let entry = entry.map_err(|e| ProjectError::io(&drawings, e))?;
        let Some(file) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        let name = format!("{DRAWINGS_DIR}/{file}");
        if matches!(classify(&name), Kind::Drawing(_)) && !entries.iter().any(|(n, _)| *n == name) {
            fs::remove_file(entry.path()).map_err(|e| ProjectError::io(entry.path(), e))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsafe_names_are_detected() {
        for name in [
            "manifest.json",
            "drawings/",
            "drawings/abc.pdf",
            "templates/report v1.typ",
        ] {
            assert!(is_safe_name(name), "{name}");
        }
        for name in [
            "",
            "/",
            "/etc/passwd",
            "../project.json",
            "drawings/../../x",
            "./manifest.json",
            "a//b",
            "C:/x",
            "c:x",
            "drawings\\x.pdf",
            "a\0b",
        ] {
            assert!(!is_safe_name(name), "{name}");
        }
    }

    #[test]
    fn zip_time_comes_from_the_timestamp() {
        let time = zip_time(&Timestamp::parse("2026-10-09T14:03:21.250Z").unwrap());
        assert_eq!(
            (time.year(), time.month(), time.day()),
            (2026, 10, 9),
            "date"
        );
        assert_eq!((time.hour(), time.minute(), time.second()), (14, 3, 20));
        let early = zip_time(&Timestamp::parse("1970-01-01T00:00:00Z").unwrap());
        assert_eq!(early, DateTime::default());
    }
}
