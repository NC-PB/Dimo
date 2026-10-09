//! Drawing primitives for the ballooned PDF (FR-EXP-01, D-24, D-33).
//!
//! [`crate::PdfEngine::write_ballooned`] takes the original PDF and a [`BalloonOverlay`] and
//! returns a new PDF with the balloons added as vector graphics. The types here are plain
//! drawing primitives in sheet space (AGENTS.md rule 4: PDF user units, origin top left,
//! y downward, page rotation and crop box applied). They know nothing about characteristics;
//! callers map their domain model to them.
//!
//! # Geometry of a balloon
//!
//! A balloon is drawn inside the box of `width` x `height` around `center`:
//!
//! - [`BalloonShape::Circle`]: the ellipse inscribed in the box, a circle when both are equal.
//! - [`BalloonShape::Rectangle`]: the box.
//! - [`BalloonShape::Flag`]: the box with its left end cut to a point, a tag shape. The point
//!   sits at the middle of the left edge, the two cuts run at 45 degrees to the top and bottom
//!   edges (the cut length is `height / 2`, at most `width / 2`).
//!
//! The outline is stroked centered on the shape edge, as in PDF. The leader runs from where the
//! line from `center` to the anchor leaves the shape to the anchor. No leader is drawn when the
//! anchor lies inside the shape. The text is set on one line in the bundled bold font
//! (Open Sans Bold, see `data/fonts/README.md`), horizontally centered on `center`, with
//! its cap height vertically centered, so digits sit in the middle. Text is always upright on
//! the sheet, also on rotated pages.
//!
//! Drawing order: in page content mode all leaders of a sheet first, then each balloon's
//! shape and text, so leaders never cross a balloon. In annotation mode each balloon is one
//! annotation with its leader, shape and text.

use crate::PdfError;

/// A point in sheet space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetPoint {
    /// x in sheet units, to the right.
    pub x: f64,
    /// y in sheet units, downward.
    pub y: f64,
}

impl SheetPoint {
    /// Creates a point.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// An opaque sRGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    /// Red, 0 to 255.
    pub r: u8,
    /// Green, 0 to 255.
    pub g: u8,
    /// Blue, 0 to 255.
    pub b: u8,
}

impl Rgb {
    /// Black.
    pub const BLACK: Self = Self::new(0, 0, 0);
    /// White.
    pub const WHITE: Self = Self::new(255, 255, 255);

    /// Creates a color from its components.
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// Color and width of a stroked line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    /// Line color.
    pub color: Rgb,
    /// Line width in sheet units, greater than 0.
    pub width: f64,
}

/// The outline shape of a balloon (FR-BAL-03). See the module docs for the exact geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BalloonShape {
    /// Ellipse inscribed in the balloon box (a circle for a square box).
    Circle,
    /// The balloon box.
    Rectangle,
    /// The balloon box with a pointed left end.
    Flag,
}

/// A line from the balloon outline to the anchor on the drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leader {
    /// Where the leader ends, in sheet space.
    pub anchor: SheetPoint,
    /// Color and width of the line.
    pub stroke: Stroke,
}

/// One balloon: shape, fill, outline, text and an optional leader.
#[derive(Debug, Clone, PartialEq)]
pub struct Balloon {
    /// Outline shape.
    pub shape: BalloonShape,
    /// Center of the balloon box in sheet space.
    pub center: SheetPoint,
    /// Width of the balloon box in sheet units, greater than 0.
    pub width: f64,
    /// Height of the balloon box in sheet units, greater than 0.
    pub height: f64,
    /// Fill color, or `None` for a transparent balloon.
    pub fill: Option<Rgb>,
    /// Outline, or `None` for no outline.
    pub outline: Option<Stroke>,
    /// The balloon number, for example `12` or `12.1`; may be empty. Printable ASCII only
    /// (U+0020 to U+007E) except `'` and `` ` ``: PDFium writes the font without an
    /// `/Encoding`, so viewers apply the standard encoding, which differs from ASCII only in
    /// those two.
    pub text: String,
    /// Text color.
    pub text_color: Rgb,
    /// Font size (em size) in sheet units, greater than 0.
    pub text_size: f64,
    /// Leader line, or `None` for leader off.
    pub leader: Option<Leader>,
}

/// The balloons of one sheet.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetBalloons {
    /// Zero based sheet index.
    pub sheet: usize,
    /// Balloons in drawing order.
    pub balloons: Vec<Balloon>,
}

/// A date and time in UTC, written into annotation dates. It must come from the caller (for
/// example the project's last modification), never from the clock (FR-EXP-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PdfDate {
    /// Year, 1 to 9999.
    pub year: u16,
    /// Month, 1 to 12.
    pub month: u8,
    /// Day of the month, 1 to 31.
    pub day: u8,
    /// Hour, 0 to 23.
    pub hour: u8,
    /// Minute, 0 to 59.
    pub minute: u8,
    /// Second, 0 to 59.
    pub second: u8,
}

/// How balloons are written (D-33).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BalloonOutput {
    /// Vector graphics appended to the page content (default, D-33).
    PageContent,
    /// One stamp annotation per balloon, with the balloon as its appearance. PDF viewers can
    /// hide, select or delete them. `date` is written as creation and modification date of
    /// every annotation.
    Annotations {
        /// Creation and modification date of the annotations.
        date: PdfDate,
    },
}

/// Everything [`crate::PdfEngine::write_ballooned`] adds to a PDF.
#[derive(Debug, Clone, PartialEq)]
pub struct BalloonOverlay {
    /// Balloons per sheet. A sheet may appear more than once; sheets without balloons may be
    /// left out.
    pub sheets: Vec<SheetBalloons>,
    /// Page content or annotations.
    pub output: BalloonOutput,
}

fn invalid(message: impl Into<String>) -> PdfError {
    PdfError::InvalidOverlay(message.into())
}

impl BalloonOverlay {
    /// Checks every value before PDFium sees it: finite coordinates, positive sizes and
    /// widths, printable ASCII text, a valid date. Sheet indices are checked against the
    /// document when writing.
    pub fn validate(&self) -> Result<(), PdfError> {
        if let BalloonOutput::Annotations { date } = self.output {
            date.validate()?;
        }
        for sheet in &self.sheets {
            for (index, balloon) in sheet.balloons.iter().enumerate() {
                balloon
                    .validate()
                    .map_err(|e| invalid(format!("sheet {}, balloon {index}: {e}", sheet.sheet)))?;
            }
        }
        Ok(())
    }
}

impl PdfDate {
    /// True if every field is in its range.
    pub fn is_valid(self) -> bool {
        (1..=9999).contains(&self.year)
            && (1..=12).contains(&self.month)
            && (1..=31).contains(&self.day)
            && self.hour < 24
            && self.minute < 60
            && self.second < 60
    }

    fn validate(self) -> Result<(), PdfError> {
        if self.is_valid() {
            Ok(())
        } else {
            Err(invalid(format!("invalid date {self:?}")))
        }
    }
}

impl Balloon {
    fn validate(&self) -> Result<(), String> {
        let point_ok = |p: SheetPoint| p.x.is_finite() && p.y.is_finite();
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if !point_ok(self.center) {
            return Err("center must be finite".to_owned());
        }
        if !positive(self.width) || !positive(self.height) {
            return Err("width and height must be positive".to_owned());
        }
        if !positive(self.text_size) {
            return Err("text size must be positive".to_owned());
        }
        if self.outline.is_some_and(|s| !positive(s.width)) {
            return Err("outline width must be positive".to_owned());
        }
        if let Some(leader) = self.leader
            && (!point_ok(leader.anchor) || !positive(leader.stroke.width))
        {
            return Err("leader needs a finite anchor and a positive width".to_owned());
        }
        if let Some(c) = self
            .text
            .chars()
            .find(|c| !(' '..='~').contains(c) || matches!(c, '\'' | '`'))
        {
            return Err(format!(
                "text may only contain printable ASCII, found {c:?}"
            ));
        }
        Ok(())
    }

    /// The closed outline of the shape in sheet space.
    pub(crate) fn outline_path(&self) -> Vec<PathSegment> {
        let (cx, cy) = (self.center.x, self.center.y);
        let (hw, hh) = (self.width / 2.0, self.height / 2.0);
        match self.shape {
            BalloonShape::Circle => ellipse(cx, cy, hw, hh),
            BalloonShape::Rectangle | BalloonShape::Flag => polygon(&self.polygon()),
        }
    }

    /// Corners of the rectangle or flag, clockwise on the sheet starting top left.
    fn polygon(&self) -> Vec<SheetPoint> {
        let (cx, cy) = (self.center.x, self.center.y);
        let (hw, hh) = (self.width / 2.0, self.height / 2.0);
        let (left, right, top, bottom) = (cx - hw, cx + hw, cy - hh, cy + hh);
        match self.shape {
            BalloonShape::Flag => {
                let cut = hh.min(hw);
                vec![
                    SheetPoint::new(left + cut, top),
                    SheetPoint::new(right, top),
                    SheetPoint::new(right, bottom),
                    SheetPoint::new(left + cut, bottom),
                    SheetPoint::new(left, cy),
                ]
            }
            _ => vec![
                SheetPoint::new(left, top),
                SheetPoint::new(right, top),
                SheetPoint::new(right, bottom),
                SheetPoint::new(left, bottom),
            ],
        }
    }

    /// Where the leader starts: the point where the ray from the center to `anchor` leaves the
    /// shape, or `None` if the anchor is inside the shape (or at the center).
    pub(crate) fn leader_start(&self, anchor: SheetPoint) -> Option<SheetPoint> {
        let (dx, dy) = (anchor.x - self.center.x, anchor.y - self.center.y);
        if dx == 0.0 && dy == 0.0 {
            return None;
        }
        // t is the ray parameter where the outline is crossed: center + t * (dx, dy).
        let t = match self.shape {
            BalloonShape::Circle => {
                let (a, b) = (self.width / 2.0, self.height / 2.0);
                1.0 / ((dx / a).powi(2) + (dy / b).powi(2)).sqrt()
            }
            BalloonShape::Rectangle | BalloonShape::Flag => {
                let corners = self.polygon();
                let mut best = f64::INFINITY;
                for (i, p) in corners.iter().enumerate() {
                    let q = corners[(i + 1) % corners.len()];
                    if let Some(t) = ray_segment(self.center, (dx, dy), *p, q) {
                        best = best.min(t);
                    }
                }
                best
            }
        };
        (t.is_finite() && t < 1.0)
            .then(|| SheetPoint::new(self.center.x + t * dx, self.center.y + t * dy))
    }

    /// Axis aligned box around everything the balloon draws, including half the stroke
    /// widths, as (left, top, right, bottom) in sheet space.
    pub(crate) fn bounds(&self) -> (f64, f64, f64, f64) {
        let outline = self.outline.map_or(0.0, |s| s.width / 2.0);
        let mut left = self.center.x - self.width / 2.0 - outline;
        let mut right = self.center.x + self.width / 2.0 + outline;
        let mut top = self.center.y - self.height / 2.0 - outline;
        let mut bottom = self.center.y + self.height / 2.0 + outline;
        if let Some(leader) = self.leader {
            let w = leader.stroke.width / 2.0;
            left = left.min(leader.anchor.x - w);
            right = right.max(leader.anchor.x + w);
            top = top.min(leader.anchor.y - w);
            bottom = bottom.max(leader.anchor.y + w);
        }
        (left, top, right, bottom)
    }
}

/// Smallest `t` in `(0, inf)` where `origin + t * dir` hits segment `p`-`q`.
fn ray_segment(origin: SheetPoint, dir: (f64, f64), p: SheetPoint, q: SheetPoint) -> Option<f64> {
    let (ex, ey) = (q.x - p.x, q.y - p.y);
    let denom = dir.0 * ey - dir.1 * ex;
    if denom.abs() < 1e-12 {
        return None;
    }
    let (wx, wy) = (p.x - origin.x, p.y - origin.y);
    let t = (wx * ey - wy * ex) / denom;
    let s = (wx * dir.1 - wy * dir.0) / denom;
    (t > 0.0 && (-1e-9..=1.0 + 1e-9).contains(&s)).then_some(t)
}

/// One segment of a path in sheet space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PathSegment {
    MoveTo(SheetPoint),
    LineTo(SheetPoint),
    /// Cubic Bezier with two control points and the end point.
    CurveTo(SheetPoint, SheetPoint, SheetPoint),
    Close,
}

/// Four cubic Bezier arcs approximating an ellipse (radial error below 0.03 percent).
fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64) -> Vec<PathSegment> {
    // Control point distance for a quarter circle: 4/3 * (sqrt(2) - 1).
    const KAPPA: f64 = 0.552_284_749_830_793_4;
    let (kx, ky) = (rx * KAPPA, ry * KAPPA);
    let p = SheetPoint::new;
    vec![
        PathSegment::MoveTo(p(cx + rx, cy)),
        PathSegment::CurveTo(p(cx + rx, cy + ky), p(cx + kx, cy + ry), p(cx, cy + ry)),
        PathSegment::CurveTo(p(cx - kx, cy + ry), p(cx - rx, cy + ky), p(cx - rx, cy)),
        PathSegment::CurveTo(p(cx - rx, cy - ky), p(cx - kx, cy - ry), p(cx, cy - ry)),
        PathSegment::CurveTo(p(cx + kx, cy - ry), p(cx + rx, cy - ky), p(cx + rx, cy)),
        PathSegment::Close,
    ]
}

fn polygon(points: &[SheetPoint]) -> Vec<PathSegment> {
    let mut path = Vec::with_capacity(points.len() + 1);
    for (i, point) in points.iter().enumerate() {
        path.push(if i == 0 {
            PathSegment::MoveTo(*point)
        } else {
            PathSegment::LineTo(*point)
        });
    }
    path.push(PathSegment::Close);
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balloon(shape: BalloonShape) -> Balloon {
        Balloon {
            shape,
            center: SheetPoint::new(100.0, 50.0),
            width: 20.0,
            height: 10.0,
            fill: Some(Rgb::WHITE),
            outline: Some(Stroke {
                color: Rgb::new(0, 0x57, 0xB8),
                width: 1.0,
            }),
            text: "12".to_owned(),
            text_color: Rgb::BLACK,
            text_size: 7.0,
            leader: None,
        }
    }

    fn close(a: SheetPoint, b: SheetPoint) -> bool {
        (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
    }

    #[test]
    fn leader_starts_on_the_outline() {
        let p = SheetPoint::new;
        let circle = balloon(BalloonShape::Circle);
        // Along the axes the ellipse has its radii.
        assert!(close(
            circle.leader_start(p(200.0, 50.0)).unwrap(),
            p(110.0, 50.0)
        ));
        assert!(close(
            circle.leader_start(p(100.0, 0.0)).unwrap(),
            p(100.0, 45.0)
        ));
        let rect = balloon(BalloonShape::Rectangle);
        assert!(close(
            rect.leader_start(p(100.0, 100.0)).unwrap(),
            p(100.0, 55.0)
        ));
        assert!(close(
            rect.leader_start(p(0.0, 50.0)).unwrap(),
            p(90.0, 50.0)
        ));
        // Through a corner.
        assert!(close(
            rect.leader_start(p(120.0, 60.0)).unwrap(),
            p(110.0, 55.0)
        ));
        // The flag point is the left end; above it the cut is 45 degrees.
        let flag = balloon(BalloonShape::Flag);
        assert!(close(
            flag.leader_start(p(0.0, 50.0)).unwrap(),
            p(90.0, 50.0)
        ));
        assert!(close(
            flag.leader_start(p(100.0, 0.0)).unwrap(),
            p(100.0, 45.0)
        ));
    }

    #[test]
    fn no_leader_for_anchor_inside() {
        for shape in [
            BalloonShape::Circle,
            BalloonShape::Rectangle,
            BalloonShape::Flag,
        ] {
            let b = balloon(shape);
            assert_eq!(
                b.leader_start(SheetPoint::new(102.0, 51.0)),
                None,
                "{shape:?}"
            );
            assert_eq!(b.leader_start(b.center), None, "{shape:?}");
        }
    }

    #[test]
    fn outlines_are_closed_and_on_the_box() {
        let flag = balloon(BalloonShape::Flag).outline_path();
        assert_eq!(flag.len(), 6);
        assert_eq!(flag[0], PathSegment::MoveTo(SheetPoint::new(95.0, 45.0)));
        assert_eq!(flag[4], PathSegment::LineTo(SheetPoint::new(90.0, 50.0)));
        assert_eq!(flag[5], PathSegment::Close);
        let circle = balloon(BalloonShape::Circle).outline_path();
        assert_eq!(circle[0], PathSegment::MoveTo(SheetPoint::new(110.0, 50.0)));
        assert_eq!(circle.last(), Some(&PathSegment::Close));
    }

    #[test]
    fn bounds_cover_outline_and_leader() {
        let mut b = balloon(BalloonShape::Circle);
        assert_eq!(b.bounds(), (89.5, 44.5, 110.5, 55.5));
        b.leader = Some(Leader {
            anchor: SheetPoint::new(150.0, 80.0),
            stroke: Stroke {
                color: Rgb::BLACK,
                width: 2.0,
            },
        });
        assert_eq!(b.bounds(), (89.5, 44.5, 151.0, 81.0));
    }

    #[test]
    fn validation() {
        let ok = BalloonOverlay {
            sheets: vec![SheetBalloons {
                sheet: 0,
                balloons: vec![balloon(BalloonShape::Circle)],
            }],
            output: BalloonOutput::PageContent,
        };
        assert!(ok.validate().is_ok());
        let mut bad = ok.clone();
        bad.sheets[0].balloons[0].text = "Ø1".to_owned();
        assert!(matches!(bad.validate(), Err(PdfError::InvalidOverlay(_))));
        bad.sheets[0].balloons[0].text = "1'".to_owned();
        assert!(bad.validate().is_err());
        let mut bad = ok.clone();
        bad.sheets[0].balloons[0].width = f64::NAN;
        assert!(bad.validate().is_err());
        let mut bad = ok.clone();
        bad.sheets[0].balloons[0].outline = Some(Stroke {
            color: Rgb::BLACK,
            width: 0.0,
        });
        assert!(bad.validate().is_err());
        let mut bad = ok;
        bad.output = BalloonOutput::Annotations {
            date: PdfDate {
                year: 2026,
                month: 13,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            },
        };
        assert!(bad.validate().is_err());
    }
}
