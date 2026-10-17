//! File helpers of the shell: atomic writes and small JSON stores in the app config directory.
//!
//! [`write_atomic`] writes a file through a temporary file in the target folder and a rename, so
//! a crash or a failed write never leaves half a file. Every write uses its own temporary file
//! name, so two writes of the same file at the same time (two threads, or two app instances
//! sharing the settings) never rename each other's temporary file away.
//!
//! [`JsonStore`] keeps one value of the app (settings, recent projects, later first run and
//! update check state) in memory and in a JSON file. Reading is schema tolerant: fields missing
//! in the file keep their defaults and unknown fields are ignored, so older and newer files
//! still load. A file that cannot be read gives the defaults, with a warning in the log, and is
//! replaced at the next change.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Counter that makes temporary file names unique within the process.
static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

/// A temporary file name in the folder of `path`: `.<name>.<process>.<counter>.tmp`. The leading
/// dot hides it on macOS and Linux while it exists.
fn temporary_path(path: &Path) -> std::io::Result<PathBuf> {
    let name = path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("{} is not a file name", path.display()),
        )
    })?;
    let mut temporary = OsString::from(".");
    temporary.push(name);
    temporary.push(format!(
        ".{}.{}.tmp",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    Ok(path.with_file_name(temporary))
}

/// Writes `bytes` to `path` through a unique temporary file in the same folder and a rename.
/// The folder must exist. On an error the temporary file is removed and `path` is unchanged.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temporary = temporary_path(path)?;
    let result = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .and_then(|mut file| {
            file.write_all(bytes)?;
            file.sync_all()
        })
        .and_then(|()| std::fs::rename(&temporary, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// A value kept in a JSON file by a [`JsonStore`].
pub trait Stored: Serialize + DeserializeOwned + Default {
    /// Repairs a value read from the file, for example by dropping entries that are out of
    /// range. The default keeps it as read.
    #[must_use]
    fn repaired(self) -> Self {
        self
    }
}

/// Reads a stored value from `path`. A missing file gives the defaults; an unreadable or
/// malformed one too, with a warning. Fields missing in the file keep their defaults.
pub fn load_json<T: Stored>(path: &Path) -> T {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return T::default(),
        Err(error) => {
            tracing::warn!("cannot read {}: {error}", path.display());
            return T::default();
        }
    };
    // The file is laid over the defaults, so fields missing in it keep their defaults.
    let parsed = serde_json::from_str::<serde_json::Value>(&text).and_then(|file| {
        let mut merged = serde_json::to_value(T::default())?;
        overlay(&mut merged, file);
        serde_json::from_value::<T>(merged)
    });
    match parsed {
        Ok(value) => value.repaired(),
        Err(error) => {
            tracing::warn!("ignoring malformed {}: {error}", path.display());
            T::default()
        }
    }
}

/// Writes `value` as pretty JSON to `path` with [`write_atomic`], creating the folder first.
pub fn save_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut text = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    text.push('\n');
    write_atomic(path, text.as_bytes())
}

/// Lays `top` over `base`: objects are merged key by key, other values replace.
fn overlay(base: &mut serde_json::Value, top: serde_json::Value) {
    match (base, top) {
        (serde_json::Value::Object(base), serde_json::Value::Object(top)) => {
            for (key, value) in top {
                match base.get_mut(&key) {
                    Some(slot) => overlay(slot, value),
                    None => {
                        base.insert(key, value);
                    }
                }
            }
        }
        (base, top) => *base = top,
    }
}

/// One value of the app in memory and in its JSON file. Without a path (no config directory)
/// the value lives for this run only.
#[derive(Debug)]
pub struct JsonStore<T> {
    path: Option<PathBuf>,
    value: Mutex<T>,
}

impl<T: Stored> JsonStore<T> {
    /// Loads the value from `path` (see [`load_json`]).
    pub fn load(path: Option<PathBuf>) -> Self {
        let value = path.as_deref().map(load_json).unwrap_or_default();
        Self {
            path,
            value: Mutex::new(value),
        }
    }

    /// The file of the store, if it has one.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Runs `read` on the current value.
    pub fn read<R>(&self, read: impl FnOnce(&T) -> R) -> R {
        read(&self.lock())
    }

    /// Changes the value with `change` and writes the file while the value is still locked, so
    /// writes of one store never overtake each other. The change applies to this run even if
    /// the write fails; the write error comes back next to the result of `change`.
    pub fn update<R>(&self, change: impl FnOnce(&mut T) -> R) -> (R, std::io::Result<()>) {
        let mut value = self.lock();
        let result = change(&mut value);
        let written = match &self.path {
            Some(path) => save_json(path, &*value),
            None => Ok(()),
        };
        (result, written)
    }

    fn lock(&self) -> MutexGuard<'_, T> {
        self.value.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<T: Stored + Clone> JsonStore<T> {
    /// A copy of the current value.
    pub fn get(&self) -> T {
        self.lock().clone()
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
    struct Sample {
        name: String,
        nested: Nested,
        items: Vec<u32>,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct Nested {
        a: u32,
        b: bool,
    }

    impl Default for Nested {
        fn default() -> Self {
            Self { a: 7, b: true }
        }
    }

    impl Stored for Sample {
        fn repaired(mut self) -> Self {
            self.items.truncate(3);
            self
        }
    }

    fn names_in(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// NFR-PORT-02: atomic writes in folders with spaces, umlauts and a path over 260
    /// characters (the Windows limit without the verbatim prefix).
    #[test]
    fn atomic_writes_work_in_folders_with_umlauts_and_long_paths() {
        let dir = tempfile::tempdir().unwrap();
        let mut folder = dir.path().join("Zeichnungen Müller");
        // Windows counts the limit in UTF-16 units, not bytes.
        let units = |p: &Path| p.to_string_lossy().encode_utf16().count();
        while units(&folder) < 280 {
            folder.push("Prüfteile der Baugruppe Größe 1");
        }
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("Prüfteil 1.json");
        assert!(units(&path) > 260);
        save_json(&path, &["ä", "ö", "ü"]).unwrap();
        let read: Vec<String> =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(read, ["ä", "ö", "ü"]);
        assert_eq!(names_in(&folder), ["Prüfteil 1.json"]);
    }

    /// The temporary file of an atomic write stays in the folder of the target, also on a
    /// network share: no drive letter is assumed.
    #[cfg(windows)]
    #[test]
    fn temporary_files_of_unc_targets_stay_on_the_share() {
        let temporary = temporary_path(Path::new(r"\\server\share\Prüfteil 1.dimo")).unwrap();
        assert_eq!(temporary.parent(), Some(Path::new(r"\\server\share\")));
        let name = temporary
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        assert!(name.starts_with(".Prüfteil 1.dimo."), "{name}");
        assert!(name.ends_with(".tmp"), "{name}");
    }

    #[test]
    fn atomic_writes_replace_whole_files_and_leave_no_temporary_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        write_atomic(&path, b"first").unwrap();
        write_atomic(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        assert_eq!(names_in(dir.path()), ["a.txt"]);
        let missing = dir.path().join("missing").join("a.txt");
        assert!(write_atomic(&missing, b"x").is_err());
        assert!(write_atomic(Path::new("/"), b"x").is_err(), "no file name");
        assert_eq!(names_in(dir.path()), ["a.txt"]);
    }

    #[test]
    fn temporary_names_are_unique() {
        let path = Path::new("/tmp/settings.json");
        let a = temporary_path(path).unwrap();
        let b = temporary_path(path).unwrap();
        assert_ne!(a, b);
        assert_eq!(a.parent(), path.parent());
        let name = a.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.starts_with(".settings.json.") && a.extension().is_some_and(|e| e == "tmp"),
            "{name}"
        );
    }

    /// The old fixed `.json.tmp` name let one save rename the temporary file of another away,
    /// which then failed with "not found".
    #[test]
    fn concurrent_saves_of_one_file_all_succeed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config").join("store.json");
        std::thread::scope(|scope| {
            for thread in 0..8u32 {
                let path = &path;
                scope.spawn(move || {
                    for round in 0..25u32 {
                        let value = Sample {
                            name: format!("t{thread}r{round}"),
                            items: vec![thread, round],
                            ..Sample::default()
                        };
                        save_json(path, &value).unwrap();
                    }
                });
            }
        });
        let loaded: Sample = load_json(&path);
        assert!(loaded.name.starts_with('t'), "{loaded:?}");
        assert_eq!(names_in(path.parent().unwrap()), ["store.json"]);
    }

    #[test]
    fn stores_serialize_concurrent_updates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store.json");
        let store = JsonStore::<Sample>::load(Some(path.clone()));
        std::thread::scope(|scope| {
            for _ in 0..8 {
                scope.spawn(|| {
                    for _ in 0..10 {
                        let ((), written) = store.update(|s| s.nested.a += 1);
                        written.unwrap();
                    }
                });
            }
        });
        assert_eq!(store.get().nested.a, 87);
        assert_eq!(load_json::<Sample>(&path).nested.a, 87);
        assert_eq!(names_in(dir.path()), ["store.json"]);
    }

    #[test]
    fn loading_is_schema_tolerant() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store.json");
        assert_eq!(load_json::<Sample>(&path), Sample::default(), "missing");
        std::fs::write(&path, "{ broken").unwrap();
        assert_eq!(load_json::<Sample>(&path), Sample::default(), "malformed");
        std::fs::write(&path, r#"{"name": 5}"#).unwrap();
        assert_eq!(load_json::<Sample>(&path), Sample::default(), "wrong type");
        std::fs::write(
            &path,
            r#"{"nested": {"a": 2}, "future": true, "items": [1, 2, 3, 4]}"#,
        )
        .unwrap();
        let loaded = load_json::<Sample>(&path);
        assert_eq!(loaded.nested, Nested { a: 2, b: true }, "nested defaults");
        assert_eq!(loaded.items, [1, 2, 3], "repaired");
        assert_eq!(loaded.name, "");
    }

    #[test]
    fn stores_without_a_path_live_in_memory() {
        let store = JsonStore::<Sample>::load(None);
        assert_eq!(store.path(), None);
        let (len, written) = store.update(|s| {
            s.name = "x".into();
            s.name.len()
        });
        assert_eq!(len, 1);
        assert!(written.is_ok());
        assert_eq!(store.read(|s| s.name.clone()), "x");
    }
}
