//! Parse errors as values (docs/dev/rust.md: no panics, errors carry a position).

/// A callout that does not match the grammar. A parse error lowers the confidence of a
/// proposal instead of failing silently (spec 08 stage 6, FR-REC-08).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("cannot read callout at byte {position}: expected {expected}")]
pub struct ParseError {
    /// Byte offset into the parsed text where parsing stopped.
    pub position: usize,
    /// What the grammar expected at that position, e.g. `"lower deviation"`.
    pub expected: String,
}
