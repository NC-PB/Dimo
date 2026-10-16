//! The log file of the app (D-06, basis of "Copy diagnostics" in R.10).
//!
//! A small `tracing` subscriber writes one line per event to [`LOG_FILE`] in the app log folder:
//!
//! - macOS: `~/Library/Logs/io.github.nc-pb.dimo/dimo.log`
//! - Windows: `%LOCALAPPDATA%\io.github.nc-pb.dimo\logs\dimo.log`
//! - Linux: `~/.local/share/io.github.nc-pb.dimo/logs/dimo.log`
//!
//! Debug builds started with `DIMO_DEV_HOME` write to `logs/dimo.log` below that folder instead.
//!
//! The file is capped at [`MAX_LOG_BYTES`]: when it is full it becomes `dimo.log.1` (replacing
//! the older one) and a new file starts; lines are cut at [`MAX_LINE_BYTES`], so the logs take
//! at most about twice the cap. Release builds log `info` and above; debug builds also log
//! `debug` events of the DimoMark crates and mirror every line to the terminal. Nothing leaves the
//! computer (rule 9). The lines hold what the code logs: errors, warnings, the start line and
//! file paths. Code must not log drawing or project content; error texts of libraries (Typst
//! diagnostics, panics) are logged as they are and are checked again for R.10.
//!
//! The subscriber is written here on top of `tracing` itself instead of `tracing-subscriber`, so
//! no new crate is needed (rule 10). It does not track spans: only events are written.

use std::fmt::Write as _;
use std::fs::File;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use tracing::field::{Field, Visit};
use tracing::level_filters::LevelFilter;
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

/// File name of the log in the app log folder.
pub const LOG_FILE: &str = "dimo.log";

/// Size at which the log file is rotated, 1 MiB.
pub const MAX_LOG_BYTES: u64 = 1024 * 1024;

/// Longest line written; longer messages are cut, so one event cannot fill the log.
pub const MAX_LINE_BYTES: usize = 8 * 1024;

thread_local! {
    /// True while this thread writes an event.
    static IN_EVENT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Clears [`IN_EVENT`] when an event is written, also by a panic.
struct InEvent;

impl Drop for InEvent {
    fn drop(&mut self) {
        IN_EVENT.with(|busy| busy.set(false));
    }
}

/// What a [`FileLog`] writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LogLevel {
    /// Most detailed level of every crate.
    pub all: Level,
    /// Most detailed level of the DimoMark crates (targets starting with `dimo`).
    pub dimo: Level,
    /// Also write every line to standard error.
    pub mirror: bool,
}

impl LogLevel {
    /// `info` in release builds; debug builds add `debug` for the DimoMark crates and mirror the
    /// lines to the terminal.
    pub const fn for_build() -> Self {
        if cfg!(debug_assertions) {
            Self {
                all: Level::INFO,
                dimo: Level::DEBUG,
                mirror: true,
            }
        } else {
            Self {
                all: Level::INFO,
                dimo: Level::INFO,
                mirror: false,
            }
        }
    }

    fn enabled(self, metadata: &Metadata<'_>) -> bool {
        let level = if metadata.target().starts_with("dimo") {
            self.dimo
        } else {
            self.all
        };
        *metadata.level() <= level
    }
}

/// The open log file and how much it holds.
#[derive(Debug)]
struct Output {
    file: Option<File>,
    written: u64,
}

/// A `tracing` subscriber that appends events to a size capped log file.
#[derive(Debug)]
pub struct FileLog {
    path: PathBuf,
    max_bytes: u64,
    level: LogLevel,
    output: Mutex<Output>,
}

impl FileLog {
    /// A log appending to `path`, rotated at `max_bytes`. Creates the folder. Fails if the file
    /// cannot be opened.
    pub fn open(path: &Path, max_bytes: u64, level: LogLevel) -> std::io::Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let file = append(path)?;
        let written = file.metadata()?.len();
        Ok(Self {
            path: path.to_path_buf(),
            max_bytes,
            level,
            output: Mutex::new(Output {
                file: Some(file),
                written,
            }),
        })
    }

    /// The log file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends one line, rotating first if it would pass the cap. Write errors are dropped:
    /// logging must never fail the app.
    fn write_line(&self, line: &str) {
        let mut output = self.output.lock().unwrap_or_else(PoisonError::into_inner);
        let len = line.len() as u64;
        if output.written > 0 && output.written + len > self.max_bytes {
            output.file = None;
            let _ = std::fs::rename(&self.path, rotated(&self.path));
            output.written = 0;
        }
        if output.file.is_none() {
            output.file = append(&self.path).ok();
        }
        if let Some(file) = &mut output.file
            && file.write_all(line.as_bytes()).is_ok()
        {
            output.written += len;
        }
    }
}

fn append(path: &Path) -> std::io::Result<File> {
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
}

/// The file an old log moves to: `dimo.log.1`.
pub fn rotated(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".1");
    PathBuf::from(name)
}

/// Collects the message and the fields of an event into one line.
#[derive(Default)]
struct Line {
    message: String,
    fields: String,
}

impl Visit for Line {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else {
            let _ = write!(self.fields, " {}={value:?}", field.name());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value:?}");
        } else {
            let _ = write!(self.fields, " {}={value:?}", field.name());
        }
    }
}

impl Subscriber for FileLog {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.level.enabled(metadata)
    }

    fn max_level_hint(&self) -> Option<LevelFilter> {
        Some(LevelFilter::from_level(self.level.all.max(self.level.dimo)))
    }

    fn new_span(&self, _span: &Attributes<'_>) -> Id {
        // Spans are not tracked; every span gets the same id.
        Id::from_u64(1)
    }

    fn record(&self, _span: &Id, _values: &Record<'_>) {}

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        // An event raised while this thread writes one (a panic inside the write reaching the
        // panic hook) is dropped: logging it would lock the output twice.
        if IN_EVENT.with(|busy| busy.replace(true)) {
            return;
        }
        let _done = InEvent;
        let metadata = event.metadata();
        let mut line = Line::default();
        event.record(&mut line);
        let text = format!(
            "{} {:>5} {}: {}{}",
            chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ"),
            metadata.level(),
            metadata.target(),
            line.message,
            line.fields
        );
        // One event is one line, also when the message has line breaks.
        let mut text = text.replace(['\n', '\r'], " ");
        if text.len() > MAX_LINE_BYTES {
            let mut end = MAX_LINE_BYTES;
            while !text.is_char_boundary(end) {
                end -= 1;
            }
            text.truncate(end);
            text.push_str(" [cut]");
        }
        text.push('\n');
        if self.level.mirror {
            // Not `eprint!`, which panics when standard error is closed.
            let _ = std::io::stderr().write_all(text.as_bytes());
        }
        self.write_line(&text);
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

/// Installs the log for the whole process, writing to [`LOG_FILE`] in `dir`, and logs panics.
/// Returns the log file, or `None` if it could not be opened or a log is already installed.
pub fn init(dir: &Path) -> Option<PathBuf> {
    let path = dir.join(LOG_FILE);
    let log = match FileLog::open(&path, MAX_LOG_BYTES, LogLevel::for_build()) {
        Ok(log) => log,
        Err(error) => {
            #[allow(clippy::print_stderr, reason = "there is no log to write to")]
            {
                eprintln!("cannot open the log file {}: {error}", path.display());
            }
            return None;
        }
    };
    tracing::subscriber::set_global_default(log).ok()?;
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        tracing::error!("panic: {info}");
        previous(info);
    }));
    tracing::info!(
        "DimoMark {} ({:?}) on {} {}",
        env!("CARGO_PKG_VERSION"),
        crate::ipc::BuildProfile::current(),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    const QUIET: LogLevel = LogLevel {
        all: Level::INFO,
        dimo: Level::DEBUG,
        mirror: false,
    };

    #[test]
    fn events_are_written_as_lines_with_level_and_target() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("logs").join(LOG_FILE);
        let log = FileLog::open(&path, MAX_LOG_BYTES, QUIET).unwrap();
        assert_eq!(log.path(), path);
        tracing::subscriber::with_default(log, || {
            tracing::warn!("cannot write {}: {}", "/x/settings.json", "denied");
            tracing::info!(count = 3, "two\nlines");
            tracing::debug!("detail of a dimo crate");
            tracing::debug!(target: "other_crate", "noise of another crate");
            tracing::trace!("too detailed");
            tracing::info!("{}", "ü".repeat(MAX_LINE_BYTES));
        });
        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 4, "{text}");
        assert!(lines[3].ends_with(" [cut]"), "long lines are cut");
        assert!(lines[3].len() <= MAX_LINE_BYTES + 6);
        assert!(
            lines[0].ends_with(
                " WARN dimo_desktop::logging::tests: cannot write /x/settings.json: denied"
            ),
            "{}",
            lines[0]
        );
        assert!(lines[1].ends_with("two lines count=3"), "{}", lines[1]);
        assert!(lines[2].contains("DEBUG"), "{}", lines[2]);
    }

    #[test]
    fn full_logs_rotate_to_one_older_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOG_FILE);
        std::fs::write(&path, "x".repeat(150)).unwrap();
        let log = FileLog::open(&path, 200, QUIET).unwrap();
        tracing::subscriber::with_default(log, || {
            for i in 0..20 {
                tracing::info!("line {i}");
            }
        });
        let current = std::fs::read_to_string(&path).unwrap();
        let older = std::fs::read_to_string(rotated(&path)).unwrap();
        assert!(current.len() <= 200 && older.len() <= 200, "capped");
        assert!(current.contains("line 19"), "{current}");
        assert!(!older.starts_with("xxx"), "the oldest content is gone");
        assert_eq!(rotated(&path), dir.path().join("dimo.log.1"));
    }
}
