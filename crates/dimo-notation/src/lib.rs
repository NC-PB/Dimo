//! Parser for dimension and tolerance callouts. Pure, parse errors are values with a position.
//!
//! [`parse_callout`] turns the text of one callout (`4X Ø6.6 THRU`, `Ø30 H7 +0.0203 -0`,
//! `M8x1.25-6H`, `(42)`) into a [`Callout`]; [`Callout::to_canonical`] prints it back
//! (spec 08 stage 6, FR-REC-08). Reference and basic markers are recognized so that the
//! caller can set `inspect = false` (D-25, FR-CHR-08). Inch notation is recognized and marked
//! (D-20). All numbers are `rust_decimal::Decimal` (rule 5).
//!
//! The callout describes what is written, not the limits. `dimo-tolerance` interprets it.

mod callout;
mod error;
mod parser;

pub use callout::{Callout, Fit, Kind, Number, NumberForm, Suffix, Tolerance, ToleranceClass};
pub use error::ParseError;
pub use parser::parse_callout;
