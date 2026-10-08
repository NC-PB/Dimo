//! Disk cache of encoded tiles in the user cache directory, keyed by file hash.
//!
//! Layout below the directory given to the service (the app passes
//! `<user cache dir>/<app id>/tiles`):
//!
//! ```text
//! <RENDER_VERSION>/<content hash>/last_used
//! <RENDER_VERSION>/<content hash>/<sheet>/<zoom>/<x>_<y>.png
//! ```
//!
//! Tiles are never stored in the project (07 Data model, ADR 0004). `RENDER_VERSION` changes
//! whenever rendering or encoding changes, so stale tiles are never served; directories of
//! other versions are deleted when the cache opens. The cache is pruned to its byte budget at
//! the same time, least recently used documents first (`last_used` marker mtime).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::hash::ContentHash;

use super::TileKey;
use super::encode::PNG_SIGNATURE;

/// Version of rendering plus encoding. Bump it when tile pixels or bytes change for the same
/// input (PDFium release, render flags, encoder settings).
pub(crate) const RENDER_VERSION: &str = "v1-pdfium7881-png";

/// Name of the per document marker file whose mtime records the last use.
const LAST_USED: &str = "last_used";

/// The tile disk cache. All methods are safe to call from several threads: files are written
/// to a temporary name and renamed into place.
#[derive(Debug)]
pub(crate) struct DiskCache {
    /// `<base>/<RENDER_VERSION>`.
    root: PathBuf,
}

impl DiskCache {
    /// Opens (creates) the cache below `base`.
    pub(crate) fn open(base: &Path) -> io::Result<Self> {
        let root = base.join(RENDER_VERSION);
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    fn doc_dir(&self, doc: &ContentHash) -> PathBuf {
        self.root.join(doc.to_hex())
    }

    fn tile_path(&self, key: &TileKey) -> PathBuf {
        self.doc_dir(&key.doc)
            .join(key.sheet.to_string())
            .join(key.zoom.to_string())
            .join(format!("{}_{}.png", key.x, key.y))
    }

    /// The cached tile, or `None` when it is missing or not a PNG file.
    pub(crate) fn read(&self, key: &TileKey) -> Option<Vec<u8>> {
        let bytes = fs::read(self.tile_path(key)).ok()?;
        bytes.starts_with(&PNG_SIGNATURE).then_some(bytes)
    }

    /// Stores a tile atomically (temporary file, then rename).
    pub(crate) fn write(&self, key: &TileKey, bytes: &[u8]) -> io::Result<()> {
        let path = self.tile_path(key);
        let dir = path
            .parent()
            .ok_or_else(|| io::Error::other("tile path has no parent"))?;
        fs::create_dir_all(dir)?;
        let tmp = dir.join(format!(
            ".{}_{}.{:?}.tmp",
            key.x,
            key.y,
            std::thread::current().id()
        ));
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &path).inspect_err(|_| {
            let _ = fs::remove_file(&tmp);
        })
    }

    /// Records that a document was used now, so pruning keeps it longer.
    pub(crate) fn touch(&self, doc: &ContentHash) -> io::Result<()> {
        let dir = self.doc_dir(doc);
        fs::create_dir_all(&dir)?;
        let file = fs::File::create(dir.join(LAST_USED))?;
        file.set_modified(SystemTime::now())
    }

    /// Deletes directories of other render versions below `base`, then whole documents, least
    /// recently used first, until the cache holds at most `budget` bytes. Documents used after
    /// `started` (opened while pruning runs) are never deleted. Returns the bytes left.
    pub(crate) fn prune(&self, budget: u64, started: SystemTime) -> io::Result<u64> {
        if let Some(base) = self.root.parent() {
            for entry in fs::read_dir(base)? {
                let entry = entry?;
                if entry.file_name() != RENDER_VERSION && entry.file_type()?.is_dir() {
                    tracing::debug!(dir = %entry.path().display(), "removing stale tile cache");
                    fs::remove_dir_all(entry.path())?;
                }
            }
        }
        let mut docs = Vec::new();
        let mut total = 0;
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let path = entry.path();
            let used = fs::metadata(path.join(LAST_USED))
                .or_else(|_| entry.metadata())
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let size = dir_size(&path)?;
            total += size;
            docs.push((used, size, path));
        }
        docs.sort();
        for (used, size, path) in docs {
            if total <= budget {
                break;
            }
            if used > started {
                continue;
            }
            tracing::debug!(dir = %path.display(), size, "pruning tile cache");
            fs::remove_dir_all(&path)?;
            total -= size;
        }
        Ok(total)
    }
}

fn dir_size(dir: &Path) -> io::Result<u64> {
    let mut size = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            size += dir_size(&entry.path())?;
        } else if kind.is_file() {
            size += entry.metadata()?.len();
        }
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn key(doc: &[u8], x: u32) -> TileKey {
        TileKey {
            doc: ContentHash::of(doc),
            sheet: 1,
            zoom: -1,
            x,
            y: 2,
        }
    }

    fn png(size: usize) -> Vec<u8> {
        let mut bytes = PNG_SIGNATURE.to_vec();
        bytes.resize(size, 0);
        bytes
    }

    #[test]
    fn write_then_read() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let k = key(b"a", 0);
        assert!(cache.read(&k).is_none());
        cache.write(&k, &png(100)).unwrap();
        assert_eq!(cache.read(&k).unwrap(), png(100));
        let expected = dir
            .path()
            .join(RENDER_VERSION)
            .join(k.doc.to_hex())
            .join("1/-1/0_2.png");
        assert!(expected.is_file());
    }

    #[test]
    fn non_png_file_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let k = key(b"a", 0);
        cache.write(&k, b"garbage").unwrap();
        assert!(cache.read(&k).is_none());
    }

    #[test]
    fn prune_removes_stale_versions_and_oldest_documents() {
        let dir = tempfile::tempdir().unwrap();
        let stale = dir.path().join("v0-old");
        fs::create_dir_all(&stale).unwrap();
        fs::write(stale.join("x.png"), png(10)).unwrap();

        let cache = DiskCache::open(dir.path()).unwrap();
        for (doc, age) in [(b"old", 300), (b"mid", 200), (b"new", 100)] {
            cache.write(&key(doc, 0), &png(1000)).unwrap();
            cache.touch(&ContentHash::of(doc)).unwrap();
            let marker = cache.doc_dir(&ContentHash::of(doc)).join(LAST_USED);
            let file = fs::File::options().write(true).open(marker).unwrap();
            file.set_modified(SystemTime::now() - Duration::from_secs(age))
                .unwrap();
        }
        let left = cache.prune(2000, SystemTime::now()).unwrap();
        assert_eq!(left, 2000);
        assert!(!stale.exists());
        assert!(cache.read(&key(b"old", 0)).is_none());
        assert!(cache.read(&key(b"mid", 0)).is_some());
        assert!(cache.read(&key(b"new", 0)).is_some());
    }

    #[test]
    fn prune_keeps_documents_used_after_start() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::open(dir.path()).unwrap();
        let started = SystemTime::now() - Duration::from_secs(60);
        cache.write(&key(b"a", 0), &png(1000)).unwrap();
        cache.touch(&ContentHash::of(b"a")).unwrap();
        let left = cache.prune(0, started).unwrap();
        assert_eq!(left, 1000);
        assert!(cache.read(&key(b"a", 0)).is_some());
    }
}
