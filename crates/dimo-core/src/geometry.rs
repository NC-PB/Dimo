//! Sheet space geometry (AGENTS.md rule 4).
//!
//! All geometry is in sheet space: PDF user units (1/72 inch), origin at the top left corner of
//! the sheet, x to the right, y downwards, one coordinate system per sheet. Floats are fine here
//! because these values describe positions on the sheet, never tolerances (rule 5).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A point in sheet space.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Point {
    /// Distance from the left sheet edge in PDF user units.
    pub x: f64,
    /// Distance from the top sheet edge in PDF user units.
    pub y: f64,
}

/// A width and height in PDF user units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Size {
    /// Horizontal extent; for an oriented box, along its own x axis before rotation.
    pub width: f64,
    /// Vertical extent; for an oriented box, along its own y axis before rotation.
    pub height: f64,
}

/// Axis aligned rectangle in sheet space, for example a drawing frame or a view (M2 decision 1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Rect {
    /// Top left corner.
    pub origin: Point,
    /// Width and height.
    pub size: Size,
}

impl Rect {
    /// Creates a rectangle from its top left corner and size.
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            origin: Point { x, y },
            size: Size { width, height },
        }
    }

    /// The rectangle covering a whole sheet of `size`.
    pub const fn full(size: Size) -> Self {
        Self::new(0.0, 0.0, size.width, size.height)
    }

    /// True if the origin is finite and both sides are finite and positive.
    pub fn is_valid(&self) -> bool {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        self.origin.x.is_finite()
            && self.origin.y.is_finite()
            && positive(self.size.width)
            && positive(self.size.height)
    }

    /// The center point.
    pub fn center(&self) -> Point {
        Point {
            x: self.origin.x + self.size.width / 2.0,
            y: self.origin.y + self.size.height / 2.0,
        }
    }
}

/// Oriented bounding box in sheet space: center, size and rotation (data model `SourceRegion`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct OrientedBox {
    /// Center of the box.
    pub center: Point,
    /// Size of the box before rotation.
    pub size: Size,
    /// Rotation in degrees, counterclockwise as seen on the sheet, in the range (-180, 180].
    /// `0` is text read left to right, `90` is text read bottom to top.
    pub angle: f64,
}

impl OrientedBox {
    /// True if center and angle are finite and both sides are finite and positive. The one
    /// region rule of new characteristics and of box select (MF F2).
    pub fn is_valid(&self) -> bool {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        self.center.x.is_finite()
            && self.center.y.is_finite()
            && self.angle.is_finite()
            && positive(self.size.width)
            && positive(self.size.height)
    }

    /// The four corners in sheet space.
    pub fn corners(&self) -> [Point; 4] {
        // Counterclockwise as seen on the sheet means clockwise in the y down frame.
        let (sin, cos) = (-self.angle).to_radians().sin_cos();
        let (hw, hh) = (self.size.width / 2.0, self.size.height / 2.0);
        [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)].map(|(dx, dy)| Point {
            x: self.center.x + dx * cos - dy * sin,
            y: self.center.y + dx * sin + dy * cos,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, x: f64, y: f64) -> bool {
        (a.x - x).abs() < 1e-9 && (a.y - y).abs() < 1e-9
    }

    #[test]
    fn rect_from_corner_and_size() {
        let r = Rect::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!((r.origin.x, r.origin.y), (10.0, 20.0));
        assert_eq!((r.size.width, r.size.height), (30.0, 40.0));
        let full = Rect::full(Size {
            width: 595.0,
            height: 842.0,
        });
        assert_eq!(full, Rect::new(0.0, 0.0, 595.0, 842.0));
    }

    #[test]
    fn regions_need_finite_values_and_a_positive_size() {
        let good = OrientedBox {
            center: Point { x: 10.0, y: 20.0 },
            size: Size {
                width: 4.0,
                height: 2.0,
            },
            angle: 30.0,
        };
        assert!(good.is_valid());
        let mut flat = good;
        flat.size.height = 0.0;
        let mut far = good;
        far.center.x = f64::INFINITY;
        let mut turned = good;
        turned.angle = f64::NAN;
        for bad in [flat, far, turned] {
            assert!(!bad.is_valid(), "{bad:?}");
        }
    }

    #[test]
    fn corners_of_an_unrotated_box() {
        let b = OrientedBox {
            center: Point { x: 10.0, y: 20.0 },
            size: Size {
                width: 4.0,
                height: 2.0,
            },
            angle: 0.0,
        };
        let c = b.corners();
        assert!(close(c[0], 8.0, 19.0));
        assert!(close(c[2], 12.0, 21.0));
    }

    #[test]
    fn corners_of_a_box_rotated_counterclockwise() {
        // Text read bottom to top: the box's own x axis points up the sheet (negative y).
        let b = OrientedBox {
            center: Point { x: 0.0, y: 0.0 },
            size: Size {
                width: 4.0,
                height: 2.0,
            },
            angle: 90.0,
        };
        let c = b.corners();
        // Start of the text (left middle of the unrotated box) moves to the bottom.
        let start = Point {
            x: f64::midpoint(c[0].x, c[3].x),
            y: f64::midpoint(c[0].y, c[3].y),
        };
        assert!(close(start, 0.0, 2.0));
    }
}
