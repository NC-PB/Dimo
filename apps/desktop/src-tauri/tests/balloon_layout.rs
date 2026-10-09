//! Balloon sizes from Rust for the viewport test (T1.9, D-24, FR-BAL-03).
//!
//! The ballooned PDF sizes balloons with `BalloonStyle::layout` (through
//! `dimo_pdf::project_overlay`). The viewport computes them in TypeScript from the generated
//! `BALLOON_METRICS`. This test writes the Rust layout of a few styles and texts to
//! `apps/desktop/src/lib/viewport/balloon-layout.fixture.json`; `balloons.test.ts` checks that
//! the viewport gets the same numbers. Like the bindings test it rewrites the file and fails
//! when it was out of date: review the diff and commit it.

#![allow(clippy::expect_used)] // Test code (rust.md).

use std::fs;
use std::path::Path;

use dimo_core::{BalloonShape, BalloonStyle, Color};
use serde_json::json;

fn style(shape: BalloonShape, size_mm: f64, outline_mm: f64) -> BalloonStyle {
    BalloonStyle {
        shape,
        size_mm,
        outline_mm,
        outline_color: Color::parse("#0057B8").expect("color"),
        ..BalloonStyle::default()
    }
}

#[test]
fn viewport_fixture_matches_the_export_layout() {
    use BalloonShape::{Circle, Flag, Rectangle};
    let cases = [
        (style(Circle, 7.0, 0.35), "1"),
        (style(Circle, 7.0, 0.35), "12"),
        (style(Circle, 7.0, 0.35), "1234"),
        (style(Circle, 5.0, 0.25), "999"),
        (style(Circle, 7.0, 0.35), "12.1"),
        (style(Rectangle, 7.0, 0.35), "1"),
        (style(Rectangle, 10.0, 0.5), "1234"),
        (style(Flag, 7.0, 0.35), "1"),
        (style(Flag, 8.0, 0.35), "123"),
        (style(Flag, 12.0, 0.7), "12345"),
    ];
    let fixture: Vec<_> = cases
        .iter()
        .map(|(style, text)| json!({ "style": style, "text": text, "layout": style.layout(text) }))
        .collect();
    let mut generated = serde_json::to_string_pretty(&fixture).expect("fixture json");
    generated.push('\n');

    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../src/lib/viewport/balloon-layout.fixture.json");
    let committed = fs::read_to_string(&path).unwrap_or_default();
    if committed != generated {
        fs::write(&path, &generated).expect("write fixture");
        panic!(
            "{} was out of date and has been regenerated. Review and commit it.",
            path.display()
        );
    }
}
