//! Layout analysis, token grouping, characteristic proposals with confidence, and balloon
//! placement (spec 08).
//!
//! T2.6 adds box select: [`box_select`] turns the text inside a box drawn by the user into
//! [`Proposal`](dimo_core::Proposal)s (FR-REC-01, FR-REC-02), and [`read_callout`] reads one
//! callout text the same way for text typed into the value field (M2 decision 5). Both parse
//! with `dimo-notation` and interpret through the [`Interpreter`] seam; the tolerance engine is
//! plugged in by the app, [`CalloutOnly`] is the fallback. Nothing here changes a project:
//! proposals become characteristics only through an accept command (ADR 0006).

pub mod box_select;
pub mod interpret;
pub mod read;

pub use box_select::{BoxSelectContext, box_select};
pub use interpret::{CALLOUT_ONLY_ENGINE, CalloutOnly, Interpretation, Interpreter};
pub use read::{PARSER_ENGINE, Reading, read_callout};
