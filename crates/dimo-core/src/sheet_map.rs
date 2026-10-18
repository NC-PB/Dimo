//! How sheet geometry follows an image edit (R.20, R01 decision 10, ADR 0010).
//!
//! An image edit turns, straightens, corrects, trims or resizes the sheet of a photo. The
//! balloons, regions, views and zone grid placed on the old sheet must stay on their features,
//! so they move with a projective map from the old sheet to the new one. Rust derives the map
//! from the image (`dimo_pdf::ImageSource::sheet_map`) and sends it in
//! [`Command::SetImageEdit`](crate::Command::SetImageEdit); this module only applies it. Floats
//! are fine: these are sheet positions, never tolerances (rule 5).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::geometry::{OrientedBox, Point, Rect, Size};

/// A projective map of the plane, a 3 x 3 matrix in row order acting on `(x, y, 1)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(transparent)]
pub struct Homography(pub [f64; 9]);

#[allow(
    clippy::many_single_char_names,
    reason = "matrix entries and coordinates read as in the formulas"
)]
impl Homography {
    /// The map that leaves every point where it is.
    pub const IDENTITY: Self = Self([1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]);

    /// `x' = a x + b y + c`, `y' = d x + e y + f`.
    pub const fn affine(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self([a, b, c, d, e, f, 0.0, 0.0, 1.0])
    }

    /// Moves every point by `(dx, dy)`.
    pub const fn translate(dx: f64, dy: f64) -> Self {
        Self::affine(1.0, 0.0, dx, 0.0, 1.0, dy)
    }

    /// Scales by `sx` and `sy` about the origin.
    pub const fn scale(sx: f64, sy: f64) -> Self {
        Self::affine(sx, 0.0, 0.0, 0.0, sy, 0.0)
    }

    /// `self` after `first`: the map that applies `first`, then `self`.
    #[must_use]
    pub fn after(&self, first: &Self) -> Self {
        let (a, b) = (&self.0, &first.0);
        let mut out = [0.0; 9];
        for row in 0..3 {
            for col in 0..3 {
                out[row * 3 + col] = (0..3).map(|k| a[row * 3 + k] * b[k * 3 + col]).sum();
            }
        }
        Self(out)
    }

    /// The inverse map, `None` if the matrix is singular or not finite.
    pub fn inverse(&self) -> Option<Self> {
        let [a, b, c, d, e, f, g, h, i] = self.0;
        let co = [
            e * i - f * h,
            c * h - b * i,
            b * f - c * e,
            f * g - d * i,
            a * i - c * g,
            c * d - a * f,
            d * h - e * g,
            b * g - a * h,
            a * e - b * d,
        ];
        let det = a * co[0] + b * co[3] + c * co[6];
        if !det.is_finite() || det.abs() < 1e-300 {
            return None;
        }
        let inv = Self(co.map(|v| v / det));
        inv.0.iter().all(|v| v.is_finite()).then_some(inv)
    }

    /// Where `p` goes; `None` for a point the map sends to infinity (or behind the horizon).
    pub fn apply(&self, p: Point) -> Option<Point> {
        let m = &self.0;
        let w = m[6] * p.x + m[7] * p.y + m[8];
        if w.abs() < 1e-12 || !w.is_finite() {
            return None;
        }
        let x = (m[0] * p.x + m[1] * p.y + m[2]) / w;
        let y = (m[3] * p.x + m[4] * p.y + m[5]) / w;
        (x.is_finite() && y.is_finite()).then_some(Point { x, y })
    }

    /// The map that sends the corners of a quadrilateral (clockwise from the top left) to
    /// `(0, 0)`, `(width, 0)`, `(width, height)` and `(0, height)`. `None` if three corners
    /// lie on one line.
    pub fn quad_to_rect(quad: [Point; 4], width: f64, height: f64) -> Option<Self> {
        let unit_to_quad = square_to_quad(quad)?;
        let to_unit = unit_to_quad.inverse()?;
        Some(Self::scale(width, height).after(&to_unit))
    }
}

/// The map from the unit square (clockwise from `(0, 0)`) to `quad` (Heckbert's closed form).
fn square_to_quad(q: [Point; 4]) -> Option<Homography> {
    let [p0, p1, p2, p3] = q;
    let sx = p0.x - p1.x + p2.x - p3.x;
    let sy = p0.y - p1.y + p2.y - p3.y;
    let (dx1, dy1) = (p1.x - p2.x, p1.y - p2.y);
    let (dx2, dy2) = (p3.x - p2.x, p3.y - p2.y);
    let det = dx1 * dy2 - dx2 * dy1;
    if det.abs() < 1e-12 {
        return None;
    }
    let g = (sx * dy2 - dx2 * sy) / det;
    let h = (dx1 * sy - sx * dy1) / det;
    // Square corners (0,0) (1,0) (1,1) (0,1) go to p0 p1 p2 p3.
    Some(Homography([
        p1.x - p0.x + g * p1.x,
        p3.x - p0.x + h * p3.x,
        p0.x,
        p1.y - p0.y + g * p1.y,
        p3.y - p0.y + h * p3.y,
        p0.y,
        g,
        h,
        1.0,
    ]))
}

/// How far outside the sheet, in sheet units, a mapped point still counts as on it.
pub const ON_SHEET_TOLERANCE: f64 = 1e-6;

/// The sheet an image edit gives and how positions on the old sheet move onto it (R.20).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct SheetMap {
    /// Size of the new sheet in sheet units.
    pub size: Size,
    /// Resolution of the kept pixels at the new sheet size.
    pub raster_dpi: Option<u32>,
    /// From the old sheet to the new sheet.
    pub map: Homography,
}

impl SheetMap {
    /// True if the size is positive and finite and the map can be inverted.
    pub fn is_valid(&self) -> bool {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        positive(self.size.width) && positive(self.size.height) && self.map.inverse().is_some()
    }

    /// Where a point of the old sheet lands, `None` if the map loses it.
    pub fn point(&self, p: Point) -> Option<Point> {
        self.map.apply(p)
    }

    /// True if `p` lies on the new sheet, edges included, up to [`ON_SHEET_TOLERANCE`] (a point
    /// put on the edge by an earlier edit stays on the sheet despite rounding).
    pub fn on_sheet(&self, p: Point) -> bool {
        let t = ON_SHEET_TOLERANCE;
        (-t..=self.size.width + t).contains(&p.x) && (-t..=self.size.height + t).contains(&p.y)
    }

    /// `p` moved onto the new sheet: mapped, then clamped to the sheet edges. A point the map
    /// loses goes to the sheet center.
    pub fn point_on_sheet(&self, p: Point) -> Point {
        match self.point(p) {
            Some(q) => Point {
                x: q.x.clamp(0.0, self.size.width),
                y: q.y.clamp(0.0, self.size.height),
            },
            None => Rect::full(self.size).center(),
        }
    }

    /// The bounding box of the mapped rectangle, cut to the new sheet. `None` if nothing of
    /// it is left on the sheet.
    pub fn rect(&self, r: Rect) -> Option<Rect> {
        let corners = [
            r.origin,
            Point {
                x: r.origin.x + r.size.width,
                y: r.origin.y,
            },
            Point {
                x: r.origin.x + r.size.width,
                y: r.origin.y + r.size.height,
            },
            Point {
                x: r.origin.x,
                y: r.origin.y + r.size.height,
            },
        ];
        let mapped: Option<Vec<Point>> = corners.into_iter().map(|c| self.point(c)).collect();
        let mapped = mapped?;
        let left = mapped
            .iter()
            .map(|p| p.x)
            .fold(f64::INFINITY, f64::min)
            .max(0.0);
        let top = mapped
            .iter()
            .map(|p| p.y)
            .fold(f64::INFINITY, f64::min)
            .max(0.0);
        let right = mapped
            .iter()
            .map(|p| p.x)
            .fold(f64::NEG_INFINITY, f64::max)
            .min(self.size.width);
        let bottom = mapped
            .iter()
            .map(|p| p.y)
            .fold(f64::NEG_INFINITY, f64::max)
            .min(self.size.height);
        let out = Rect::new(left, top, right - left, bottom - top);
        out.is_valid().then_some(out)
    }

    /// The oriented box moved with the map: its center maps, its sides follow the mapped axes
    /// through the center. `None` if the map loses it.
    pub fn oriented_box(&self, b: OrientedBox) -> Option<OrientedBox> {
        let rad = b.angle.to_radians();
        // Counterclockwise as seen on the sheet, with y downward.
        let ux = Point {
            x: rad.cos(),
            y: -rad.sin(),
        };
        let uy = Point {
            x: rad.sin(),
            y: rad.cos(),
        };
        let at = |dir: Point, len: f64| Point {
            x: b.center.x + dir.x * len,
            y: b.center.y + dir.y * len,
        };
        let center = self.point(b.center)?;
        let (w, h) = (b.size.width / 2.0, b.size.height / 2.0);
        let x0 = self.point(at(ux, -w))?;
        let x1 = self.point(at(ux, w))?;
        let y0 = self.point(at(uy, -h))?;
        let y1 = self.point(at(uy, h))?;
        let len = |a: Point, b: Point| ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        let width = len(x0, x1);
        let height = len(y0, y1);
        let mut angle = (-(x1.y - x0.y)).atan2(x1.x - x0.x).to_degrees();
        if angle <= -180.0 {
            angle += 360.0;
        }
        let out = OrientedBox {
            center,
            size: Size { width, height },
            angle,
        };
        out.is_valid().then_some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    fn close(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
    }

    #[test]
    fn composition_and_inverse() {
        let m = Homography::translate(5.0, -2.0).after(&Homography::scale(2.0, 3.0));
        assert!(close(m.apply(pt(1.0, 1.0)).unwrap(), pt(7.0, 1.0)));
        let back = m.inverse().unwrap();
        assert!(close(back.apply(pt(7.0, 1.0)).unwrap(), pt(1.0, 1.0)));
        assert!(Homography::scale(0.0, 1.0).inverse().is_none());
    }

    #[test]
    fn quad_corners_go_to_the_rectangle() {
        let quad = [
            pt(10.0, 20.0),
            pt(110.0, 10.0),
            pt(120.0, 90.0),
            pt(5.0, 100.0),
        ];
        let m = Homography::quad_to_rect(quad, 50.0, 40.0).unwrap();
        let want = [pt(0.0, 0.0), pt(50.0, 0.0), pt(50.0, 40.0), pt(0.0, 40.0)];
        for (q, w) in quad.into_iter().zip(want) {
            assert!(close(m.apply(q).unwrap(), w), "{q:?}");
        }
        let line = [pt(0.0, 0.0), pt(1.0, 1.0), pt(2.0, 2.0), pt(3.0, 3.0)];
        assert!(Homography::quad_to_rect(line, 1.0, 1.0).is_none());
    }

    #[test]
    fn rects_are_cut_to_the_sheet_and_boxes_follow_a_quarter_turn() {
        // A quarter turn clockwise of a 100 x 50 sheet: (x, y) -> (50 - y, x).
        let map = SheetMap {
            size: Size {
                width: 50.0,
                height: 100.0,
            },
            raster_dpi: Some(200),
            map: Homography::affine(0.0, -1.0, 50.0, 1.0, 0.0, 0.0),
        };
        let r = map.rect(Rect::new(10.0, 5.0, 20.0, 10.0)).unwrap();
        assert!(close(r.origin, pt(35.0, 10.0)));
        assert!((r.size.width - 10.0).abs() < 1e-9 && (r.size.height - 20.0).abs() < 1e-9);
        assert!(map.rect(Rect::new(-500.0, 0.0, 10.0, 10.0)).is_none());
        let b = OrientedBox {
            center: pt(20.0, 10.0),
            size: Size {
                width: 8.0,
                height: 2.0,
            },
            angle: 0.0,
        };
        let turned = map.oriented_box(b).unwrap();
        assert!(close(turned.center, pt(40.0, 20.0)));
        assert!((turned.angle + 90.0).abs() < 1e-9, "{}", turned.angle);
        assert!((turned.size.width - 8.0).abs() < 1e-9);
        assert!(map.on_sheet(pt(50.0, 0.0)) && !map.on_sheet(pt(50.1, 0.0)));
        assert!(close(map.point_on_sheet(pt(0.0, 500.0)), pt(0.0, 0.0)));
    }
}
