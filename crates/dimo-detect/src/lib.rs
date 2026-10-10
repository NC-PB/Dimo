//! Layout analysis, token grouping, characteristic proposals with confidence, and balloon
//! placement (spec 08).
//!
//! T2.6 adds box select: [`box_select`] turns the text inside a box drawn by the user into
//! [`Proposal`](dimo_core::Proposal)s (FR-REC-01, FR-REC-02), and [`read_callout`] reads one
//! callout text the same way for text typed into the value field (M2 decision 5). Both parse
//! with `dimo-notation` and interpret through the [`Interpreter`] seam: [`ToleranceEngine`]
//! (the `dimo-tolerance` engine) in the app, [`CalloutOnly`] as fallback. Nothing here changes a
//! project: proposals become characteristics only through an accept command (ADR 0006).
//!
//! T2.9 adds [`evaluation`]: box select on the truth regions of corpus and synthetic drawings,
//! compared with the truth and counted per callout category (M2 exit criterion).

pub mod box_select;
pub mod evaluation;
pub mod interpret;
pub mod read;

pub use box_select::{BoxSelectContext, Proposed, box_select, box_select_with_notes};
pub use interpret::{
    CALLOUT_ONLY_ENGINE, CalloutOnly, Interpretation, Interpreter, TOLERANCE_ENGINE,
    ToleranceEngine,
};
pub use read::{PARSER_ENGINE, Reading, read_callout};
