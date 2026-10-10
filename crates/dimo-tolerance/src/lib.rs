//! Tolerance engine: fit tables, general tolerance tables and limit derivation with exact decimal numbers. Data driven and pure.
//!
//! Tolerance values live in data files (`data/tolerances/`), never in code (D-43). This crate
//! holds the file format ([`format`]), validation and range lookup ([`table`]), loading of
//! shipped and user tables ([`load`]), the ISO 286 tolerance class designations
//! ([`designation`]) and their expansion to deviations ([`fit`]). All values are `rust_decimal::Decimal` (AGENTS.md rule 5).
//!
//! [`interpret`] turns a parsed callout into nominal, deviations, limits and a structured
//! derivation with the precedence of FR-TOL-01, using a [`ToleranceContext`] built from the
//! project tolerance settings and a [`TableSet`]. [`explain`] renders a derivation as English
//! or German text (FR-TOL-08).

pub mod context;
pub mod designation;
pub mod error;
pub mod explain;
pub mod fit;
pub mod format;
pub mod interpret;
pub mod load;
pub mod table;

pub use context::{ContextError, DEFAULT_FIT_TABLE, ToleranceContext, import_custom_table};
pub use designation::{Fit, Grade};
pub use error::TableError;
pub use explain::{ExplainValues, Language, explain};
pub use fit::{FitError, FitLimits};
pub use format::{AppliesTo, Feature, RangeOf, Status, TableKind, ValueUnit};
pub use interpret::{Interpretation, Measures, Note, interpret, interpret_with};
pub use load::{SHIPPED, TableSet, load_custom_table, load_dir, shipped_tables};
pub use table::{Bound, GeneralTolerance, LookupError, Range, Table};
