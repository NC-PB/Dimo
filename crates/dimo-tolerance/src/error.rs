//! Errors of loading tolerance tables (D-43, FR-TOL-07).

/// Why a table could not be loaded. Every variant names the file (`origin`) so the user can
/// fix it; parse errors include the line and column.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TableError {
    /// The file could not be read.
    #[error("{origin}: cannot read the file: {message}")]
    Io {
        /// File name or path.
        origin: String,
        /// Operating system message.
        message: String,
    },
    /// The file is not valid TOML or does not match the format.
    #[error("{origin}: {message}")]
    Parse {
        /// File name or path.
        origin: String,
        /// Parser message with line and column.
        message: String,
    },
    /// The file matches the format but breaks a rule (see [`crate::Table::parse`]).
    #[error("{origin}: {location}: {message}")]
    Invalid {
        /// File name or path.
        origin: String,
        /// `table`, or the part and row.
        location: String,
        /// What is wrong.
        message: String,
    },
    /// Two tables have the same id.
    #[error("table id {id:?} is defined twice: in {first} and in {second}")]
    Duplicate {
        /// The id.
        id: String,
        /// Origin of the first table.
        first: String,
        /// Origin of the second table.
        second: String,
    },
}
