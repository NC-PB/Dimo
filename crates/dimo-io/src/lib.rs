//! Project container, migrations, and imports and exports of inspection data.
//!
//! - [`export`]: characteristic list as CSV and XLSX (T1.4).
//! - [`project`]: the `.dimo` project file (ZIP or folder), schema versions and migrations,
//!   drawing import, and [`project::ProjectSession`] for an open project (T1.3).
//! - [`journal`]: the autosave journal and crash recovery (NFR-REL-01, D-28).

pub mod export;
pub mod journal;
pub mod project;
