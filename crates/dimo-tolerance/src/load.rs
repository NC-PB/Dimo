//! Loading shipped and user tables (T2.2, D-43, FR-TOL-07).
//!
//! Shipped tables are embedded into the binary at compile time. User tables are read from a
//! folder. Every table is validated on load ([`Table::parse`]) and ids must be unique across
//! all tables of a [`TableSet`].

use std::collections::HashMap;
use std::path::Path;

use crate::error::TableError;
use crate::table::Table;

/// A table file shipped with Dimo, embedded at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShippedTable {
    /// File name in `data/tolerances/`.
    pub file_name: &'static str,
    /// The file content.
    pub text: &'static str,
}

/// All shipped tables. A new table file in `data/tolerances/` must be added here; a test
/// checks that the list matches the folder.
pub const SHIPPED: &[ShippedTable] = &[
    ShippedTable {
        file_name: "iso-2768-1.toml",
        text: include_str!("../../../data/tolerances/iso-2768-1.toml"),
    },
    ShippedTable {
        file_name: "iso-286.toml",
        text: include_str!("../../../data/tolerances/iso-286.toml"),
    },
];

/// Origin prefix of shipped tables in error messages and [`Table::origin`].
pub const SHIPPED_ORIGIN: &str = "shipped:";

/// Parse and validate all shipped tables.
pub fn shipped_tables() -> Result<Vec<Table>, TableError> {
    SHIPPED
        .iter()
        .map(|s| Table::parse(&format!("{SHIPPED_ORIGIN}{}", s.file_name), s.text))
        .collect()
}

/// Whether a file name is a table file: `*.toml` but not a test vector file `*.test.toml`.
pub fn is_table_file_name(name: &str) -> bool {
    let toml = Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("toml"));
    toml && !name.to_ascii_lowercase().ends_with(".test.toml")
}

/// Read and validate all table files in a folder, sorted by file name. A missing folder
/// holds no tables. Subfolders and other files are ignored.
pub fn load_dir(dir: &Path) -> Result<Vec<Table>, TableError> {
    let io = |origin: &Path, e: std::io::Error| TableError::Io {
        origin: origin.display().to_string(),
        message: e.to_string(),
    };
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(io(dir, e)),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry.map_err(|e| io(dir, e))?.path();
        let is_table = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(is_table_file_name);
        if is_table && path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    paths
        .iter()
        .map(|path| {
            let text = std::fs::read_to_string(path).map_err(|e| io(path, e))?;
            Table::parse(&path.display().to_string(), &text)
        })
        .collect()
}

/// A set of tables with unique ids.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TableSet {
    tables: Vec<Table>,
}

impl TableSet {
    /// Build a set, refusing duplicate ids.
    pub fn new(tables: Vec<Table>) -> Result<Self, TableError> {
        let mut seen: HashMap<&str, &str> = HashMap::new();
        for table in &tables {
            if let Some(first) = seen.insert(table.id(), table.origin()) {
                return Err(TableError::Duplicate {
                    id: table.id().to_owned(),
                    first: first.to_owned(),
                    second: table.origin().to_owned(),
                });
            }
        }
        Ok(Self { tables })
    }

    /// The shipped tables only.
    pub fn shipped() -> Result<Self, TableError> {
        Self::new(shipped_tables()?)
    }

    /// The shipped tables plus the user tables in `dir`. A user table may not reuse the id of
    /// a shipped table.
    pub fn with_user_dir(dir: &Path) -> Result<Self, TableError> {
        let mut tables = shipped_tables()?;
        tables.extend(load_dir(dir)?);
        Self::new(tables)
    }

    /// Add one more table, e.g. a custom table stored in a project. Refuses a duplicate id.
    pub fn add(&mut self, table: Table) -> Result<(), TableError> {
        if let Some(first) = self.get(table.id()) {
            return Err(TableError::Duplicate {
                id: table.id().to_owned(),
                first: first.origin().to_owned(),
                second: table.origin().to_owned(),
            });
        }
        self.tables.push(table);
        Ok(())
    }

    /// A table by id.
    pub fn get(&self, id: &str) -> Option<&Table> {
        self.tables.iter().find(|t| t.id() == id)
    }

    /// All tables in load order.
    pub fn tables(&self) -> &[Table] {
        &self.tables
    }

    /// Tables that are still drafts (D-43): limits derived from them carry a warning badge.
    pub fn drafts(&self) -> impl Iterator<Item = &Table> {
        self.tables.iter().filter(|t| t.is_draft())
    }
}
