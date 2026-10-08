//! Domain model of a Dimo project: characteristics, balloons, commands with undo and redo, numbering and validation. Pure logic without IO.
//!
//! Stub created in T0.1. T0.10 added the shared vocabulary (sheet kind, characteristic kind,
//! unit, tolerance rule, sheet space geometry, exact decimals) and the corpus truth format.

pub mod characteristic;
pub mod decimal;
pub mod geometry;
pub mod sheet;
pub mod truth;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {
        assert_eq!(env!("CARGO_PKG_NAME"), "dimo-core");
    }
}
