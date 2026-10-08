//! Conversion from PDF page space to sheet space (AGENTS.md rule 4).
//!
//! PDFium reports character and object geometry in page space: PDF user units, y upward, before
//! the crop box and the page `/Rotate` are applied. Sheet space is the displayed page: origin at
//! the top left of the page bounding box (crop box intersected with media box), y downward, page
//! rotation applied. This module mirrors PDFium's display matrix (`CPDF_Page::GetDisplayMatrix`)
//! so that geometry from text extraction lines up with rendered tiles. The `sheet_space_*` and
//! `text_runs_follow_*` tests check both against each other.

use crate::geometry::SheetRect;

/// Maps page space points to sheet space for one page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PageToSheet {
    left: f64,
    bottom: f64,
    right: f64,
    top: f64,
    /// Page `/Rotate` in clockwise quarter turns, 0 to 3.
    quarter_turns: u8,
}

impl PageToSheet {
    /// `bbox` is the page bounding box as (left, bottom, right, top) in page space;
    /// `quarter_turns` the clockwise page rotation in quarter turns.
    pub(crate) fn new(bbox: (f64, f64, f64, f64), quarter_turns: u8) -> Self {
        let (left, bottom, right, top) = bbox;
        Self {
            left: left.min(right),
            bottom: bottom.min(top),
            right: left.max(right),
            top: bottom.max(top),
            quarter_turns: quarter_turns % 4,
        }
    }

    /// Maps one page space point to sheet space.
    pub(crate) fn point(&self, x: f64, y: f64) -> (f64, f64) {
        match self.quarter_turns {
            0 => (x - self.left, self.top - y),
            // Shown rotated 90 degrees clockwise: the bottom left corner of the page becomes
            // the top left corner of the sheet.
            1 => (y - self.bottom, x - self.left),
            2 => (self.right - x, y - self.bottom),
            _ => (self.top - y, self.right - x),
        }
    }

    /// Maps an axis aligned page space rectangle (left, bottom, right, top) to the axis aligned
    /// sheet space rectangle covering it.
    pub(crate) fn rect(&self, left: f64, bottom: f64, right: f64, top: f64) -> SheetRect {
        let (x0, y0) = self.point(left, bottom);
        let (x1, y1) = self.point(right, top);
        SheetRect::new(x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs())
    }

    /// Converts a counterclockwise angle in page space (degrees) to the counterclockwise angle
    /// seen on the sheet, normalized to `[0, 360)`.
    pub(crate) fn angle(&self, page_degrees: f64) -> f64 {
        normalize_degrees(page_degrees - 90.0 * f64::from(self.quarter_turns))
    }
}

/// Normalizes an angle in degrees to `[0, 360)`.
pub(crate) fn normalize_degrees(degrees: f64) -> f64 {
    // Adding 0.0 turns -0.0 into 0.0.
    let a = degrees.rem_euclid(360.0) + 0.0;
    // rem_euclid can return 360.0 for tiny negative inputs due to rounding.
    if a >= 360.0 { 0.0 } else { a }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BBOX: (f64, f64, f64, f64) = (100.0, 50.0, 500.0, 750.0);

    #[test]
    fn unrotated_moves_origin_to_top_left() {
        let t = PageToSheet::new(BBOX, 0);
        assert_eq!(t.point(100.0, 750.0), (0.0, 0.0));
        assert_eq!(t.point(500.0, 50.0), (400.0, 700.0));
        assert_eq!(t.angle(30.0), 30.0);
    }

    #[test]
    fn rotations_map_corners_like_pdfium() {
        // Each rotation maps the page corner that ends up top left to (0, 0) and the opposite
        // corner to (sheet width, sheet height).
        let cases = [
            (1, (100.0, 50.0), (500.0, 750.0), (700.0, 400.0)),
            (2, (500.0, 50.0), (100.0, 750.0), (400.0, 700.0)),
            (3, (500.0, 750.0), (100.0, 50.0), (700.0, 400.0)),
        ];
        for (turns, top_left, bottom_right, size) in cases {
            let t = PageToSheet::new(BBOX, turns);
            assert_eq!(t.point(top_left.0, top_left.1), (0.0, 0.0), "{turns}");
            assert_eq!(t.point(bottom_right.0, bottom_right.1), size, "{turns}");
        }
    }

    #[test]
    fn rect_is_normalized() {
        let t = PageToSheet::new(BBOX, 1);
        let r = t.rect(110.0, 60.0, 130.0, 70.0);
        assert_eq!(r, SheetRect::new(10.0, 10.0, 10.0, 20.0));
    }

    #[test]
    fn angles_follow_page_rotation() {
        assert_eq!(PageToSheet::new(BBOX, 1).angle(0.0), 270.0);
        assert_eq!(PageToSheet::new(BBOX, 2).angle(90.0), 270.0);
        assert_eq!(PageToSheet::new(BBOX, 3).angle(0.0), 90.0);
        assert_eq!(normalize_degrees(-1e-20), 0.0);
        assert_eq!(normalize_degrees(720.0), 0.0);
        assert!(normalize_degrees(-0.0).is_sign_positive());
    }
}
