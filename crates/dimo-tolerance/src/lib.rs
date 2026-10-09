//! Tolerance engine: fit tables, general tolerance tables and limit derivation with exact decimal numbers. Data driven and pure.
//!
//! Tolerance values live in data files (`data/tolerances/`), never in code (D-43). This crate
//! holds the file format ([`format`]), validation and range lookup ([`table`]), loading of
//! shipped and user tables ([`load`]) and the ISO 286 tolerance class designations
//! ([`designation`]). All values are `rust_decimal::Decimal` (AGENTS.md rule 5).

pub mod designation;
pub mod error;
pub mod format;
pub mod load;
pub mod table;

pub use designation::{Fit, Grade};
pub use error::TableError;
pub use format::{AppliesTo, Feature, RangeOf, Status, TableKind, ValueUnit};
pub use load::{SHIPPED, TableSet, load_dir, shipped_tables};
pub use table::{Bound, GeneralTolerance, LookupError, Range, Table};
