//! How an [`ImageEdit`] turns the pixels of a page into a sheet (R.20, ADR 0010).
//!
//! Every edit is one projective map ([`Homography`]) from the upright stored page (pixels,
//! origin top left, after the EXIF or TIFF orientation) to sheet units. It is built in the
//! order of the edit (`dimo_core::source` module docs):
//!
//! 1. Pixels to physical size: the stated scan resolution, or 297 mm on the long side
//!    (R.19). Everything after this step is in sheet units, so a trim keeps the scale.
//! 2. Correction. With corners: the four corners (stored pixels) go straight onto a rectangle
//!    whose sides are the means of the opposite quadrilateral sides; the quarter turns only
//!    name the corners. Without corners: the quarter turns, then the fine angle clockwise about
//!    the center onto the bounding box of the turned page (nothing is cut off; the trim
//!    removes the white corners).
//! 3. Trim: the kept part, as fractions of the corrected page.
//! 4. Paper: the trimmed page stretched onto the paper; without one the size stays.
//!
//! The math is plain IEEE arithmetic with a fixed sine and cosine series, no platform math
//! library, so every machine gets the same map and the same pixels (rule 11).

use dimo_core::{
    Homography, ImageEdit, ImagePoint, Point, Rotation, SheetCorners, Size, UNITS_PER_MM,
};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use super::{DEFAULT_LONG_SIDE_MM, MAX_DPI, MAX_SCAN_DPI, MIN_SCAN_DPI};
use crate::error::PdfError;

/// Sheet units per inch.
pub(super) const UNITS_PER_INCH: f64 = 72.0;

/// Size and stated resolution of an upright page, what the geometry needs from the pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PageInfo {
    pub width: u32,
    pub height: u32,
    /// Pixels per inch horizontally and vertically, if the file states them.
    pub dpi: Option<(f64, f64)>,
}

/// The sheet an edit gives a page and where its pixels go (R.20).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageGeometry {
    /// Sheet size in sheet units.
    pub sheet: Size,
    /// Resolution of the kept pixels at the sheet size, pixels per inch.
    pub raster_dpi: u32,
    /// Block size the page is reduced by to keep at most [`MAX_DPI`] (1 keeps every pixel).
    pub block: u32,
    /// From upright page pixels to sheet units.
    pub to_sheet: Homography,
}

/// Which step of the edit an editor view shows (R.20).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditStage {
    /// The page with its quarter turns only, where the corner handles are placed.
    Corners,
    /// The corrected page without trim and paper, where the trim rectangle is drawn.
    Trim,
    /// The whole edit: the sheet as it will be.
    Result,
}

impl EditStage {
    /// The part of `edit` this stage shows.
    pub fn edit(self, edit: &ImageEdit) -> ImageEdit {
        match self {
            Self::Corners => ImageEdit {
                rotation: edit.rotation,
                ..ImageEdit::default()
            },
            Self::Trim => ImageEdit {
                trim: None,
                paper: None,
                ..edit.clone()
            },
            Self::Result => edit.clone(),
        }
    }
}

/// Sheet units per pixel, horizontally and vertically (R.19 rule).
pub(super) fn units_per_pixel(info: &PageInfo) -> (f64, f64) {
    match info.dpi {
        Some((x, y)) if is_scan_dpi(x) && is_scan_dpi(y) => {
            (UNITS_PER_INCH / x, UNITS_PER_INCH / y)
        }
        _ => {
            let long = f64::from(info.width.max(info.height).max(1));
            let s = DEFAULT_LONG_SIDE_MM * UNITS_PER_MM / long;
            (s, s)
        }
    }
}

fn is_scan_dpi(dpi: f64) -> bool {
    (MIN_SCAN_DPI..=MAX_SCAN_DPI).contains(&dpi)
}

/// The geometry of a page with `edit`. The edit must be valid (`ImageEdit::validate`).
pub(super) fn page_geometry(info: &PageInfo, edit: &ImageEdit) -> Result<PageGeometry, PdfError> {
    let (sx, sy) = units_per_pixel(info);
    let physical = Homography::scale(sx, sy);
    let (correct, corrected) = correction(info, edit, sx, sy)?;
    let (trim, trimmed) = match &edit.trim {
        Some(t) => {
            let (l, t_, r, b) = (num(t.left), num(t.top), num(t.right), num(t.bottom));
            (
                Homography::translate(-l * corrected.width, -t_ * corrected.height),
                Size {
                    width: (r - l) * corrected.width,
                    height: (b - t_) * corrected.height,
                },
            )
        }
        None => (Homography::IDENTITY, corrected),
    };
    if !(trimmed.width > 0.0 && trimmed.height > 0.0) {
        return Err(invalid("the trimmed page has no size"));
    }
    let (fit, sheet) = match &edit.paper {
        Some(paper) => {
            let sheet = paper.sheet_size();
            (
                Homography::scale(sheet.width / trimmed.width, sheet.height / trimmed.height),
                sheet,
            )
        }
        None => (Homography::IDENTITY, trimmed),
    };
    let to_sheet = fit.after(&trim).after(&correct).after(&physical);
    // Mean pixels per sheet unit over the kept part.
    let pixels = trimmed.width * trimmed.height / (sx * sy);
    let per_unit = (pixels / (sheet.width * sheet.height)).sqrt();
    let full_dpi = per_unit * UNITS_PER_INCH;
    let block = to_u32((full_dpi / MAX_DPI - 1e-9).ceil()).max(1);
    Ok(PageGeometry {
        sheet,
        raster_dpi: to_u32((full_dpi / f64::from(block)).round()),
        block,
        to_sheet,
    })
}

/// Map from upright page pixels to an editor stage and the stage's sheet size.
pub(super) fn stage_geometry(
    info: &PageInfo,
    edit: &ImageEdit,
    stage: EditStage,
) -> Result<PageGeometry, PdfError> {
    page_geometry(info, &stage.edit(edit))
}

/// The correction (step 2) in sheet units, from page pixels already scaled by `sx`, `sy`,
/// and the size of the corrected page.
fn correction(
    info: &PageInfo,
    edit: &ImageEdit,
    sx: f64,
    sy: f64,
) -> Result<(Homography, Size), PdfError> {
    let w = f64::from(info.width) * sx;
    let h = f64::from(info.height) * sy;
    if let Some(corners) = &edit.corners {
        let quad = corners.points().map(|p| Point {
            x: num(p.x) * sx,
            y: num(p.y) * sy,
        });
        let [tl, tr, br, bl] = quad;
        let width = f64::midpoint(dist(tl, tr), dist(bl, br));
        let height = f64::midpoint(dist(tl, bl), dist(tr, br));
        if !(width > 0.0 && height > 0.0) {
            return Err(invalid("the corners enclose no area"));
        }
        let map = Homography::quad_to_rect(quad, width, height)
            .ok_or_else(|| invalid("three corners lie on one line"))?;
        return Ok((map, Size { width, height }));
    }
    let (turn, w1, h1) = quarter_turn(edit.rotation, w, h);
    let degrees = num(edit.fine_angle);
    if degrees == 0.0 {
        return Ok((
            turn,
            Size {
                width: w1,
                height: h1,
            },
        ));
    }
    let (s, c) = sin_cos_degrees(degrees);
    let width = w1 * c.abs() + h1 * s.abs();
    let height = w1 * s.abs() + h1 * c.abs();
    // Clockwise on the sheet (y downward).
    let rotate = Homography::affine(c, -s, 0.0, s, c, 0.0);
    let map = Homography::translate(width / 2.0, height / 2.0)
        .after(&rotate)
        .after(&Homography::translate(-w1 / 2.0, -h1 / 2.0))
        .after(&turn);
    Ok((map, Size { width, height }))
}

/// Clockwise quarter turns of a `w` by `h` page and the size after them.
fn quarter_turn(rotation: Rotation, w: f64, h: f64) -> (Homography, f64, f64) {
    match rotation {
        Rotation::Deg0 => (Homography::IDENTITY, w, h),
        Rotation::Deg90 => (Homography::affine(0.0, -1.0, h, 1.0, 0.0, 0.0), h, w),
        Rotation::Deg180 => (Homography::affine(-1.0, 0.0, w, 0.0, -1.0, h), w, h),
        Rotation::Deg270 => (Homography::affine(0.0, 1.0, 0.0, -1.0, 0.0, w), h, w),
    }
}

/// The page corners in stored pixels, named as the sheet reads after `rotation` (the
/// editor's starting corners when none are found).
pub(super) fn page_corners(info: &PageInfo, rotation: Rotation) -> SheetCorners {
    let (w, h) = (f64::from(info.width), f64::from(info.height));
    corners_named(
        [
            Point { x: 0.0, y: 0.0 },
            Point { x: w, y: 0.0 },
            Point { x: w, y: h },
            Point { x: 0.0, y: h },
        ],
        rotation,
    )
}

/// Corners found on the upright page (clockwise from its top left) named as the sheet reads
/// after `rotation`: after one clockwise turn the sheet's top left is the page's bottom left.
pub(super) fn corners_named(page: [Point; 4], rotation: Rotation) -> SheetCorners {
    let turns = match rotation {
        Rotation::Deg0 => 0,
        Rotation::Deg90 => 1,
        Rotation::Deg180 => 2,
        Rotation::Deg270 => 3,
    };
    let at = |i: usize| image_point(page[(i + 4 - turns) % 4]);
    SheetCorners {
        top_left: at(0),
        top_right: at(1),
        bottom_right: at(2),
        bottom_left: at(3),
    }
}

/// A pixel position as an exact decimal with two places.
pub(super) fn image_point(p: Point) -> ImagePoint {
    let dec = |v: f64| {
        Decimal::from_f64_retain(v.max(0.0))
            .unwrap_or_default()
            .round_dp(2)
            .normalize()
    };
    ImagePoint {
        x: dec(p.x),
        y: dec(p.y),
    }
}

fn dist(a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    (dx * dx + dy * dy).sqrt()
}

fn num(value: Decimal) -> f64 {
    value.to_f64().unwrap_or(0.0)
}

fn invalid(message: &str) -> PdfError {
    PdfError::ImageEdit(message.to_owned())
}

/// A whole number of 0 or more as `u32`, saturating; 0 for NaN.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(super) fn to_u32(value: f64) -> u32 {
    // Float to int `as` casts saturate and turn NaN into 0.
    value as u32
}

/// Sine and cosine of an angle of at most 45 degrees either way, by a fixed series: the same
/// bits on every platform (the platform `sin` may differ in the last place).
pub(super) fn sin_cos_degrees(degrees: f64) -> (f64, f64) {
    let x = degrees * (std::f64::consts::PI / 180.0);
    let x2 = x * x;
    let (mut sin, mut cos) = (0.0, 0.0);
    let (mut s_term, mut c_term) = (x, 1.0);
    for n in 0..12_u32 {
        sin += s_term;
        cos += c_term;
        let k = f64::from(2 * n + 2);
        s_term *= -x2 / (k * (k + 1.0));
        c_term *= -x2 / ((k - 1.0) * k);
    }
    (sin, cos)
}

#[cfg(test)]
mod tests {
    use dimo_core::{Orientation, Paper, PaperSize, Trim};

    use super::*;

    fn dec(text: &str) -> Decimal {
        text.parse().unwrap()
    }

    fn photo() -> PageInfo {
        PageInfo {
            width: 4000,
            height: 3000,
            dpi: Some((72.0, 72.0)),
        }
    }

    fn close(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6
    }

    fn at(g: &PageGeometry, x: f64, y: f64) -> Point {
        g.to_sheet.apply(Point { x, y }).unwrap()
    }

    #[test]
    fn series_matches_known_values() {
        let (s, c) = sin_cos_degrees(30.0);
        assert!((s - 0.5).abs() < 1e-15 && (c - 0.75_f64.sqrt()).abs() < 1e-15);
        let (s, c) = sin_cos_degrees(-45.0);
        assert!((s + 0.5_f64.sqrt()).abs() < 1e-15 && (c - 0.5_f64.sqrt()).abs() < 1e-15);
        assert_eq!(sin_cos_degrees(0.0), (0.0, 1.0));
    }

    #[test]
    fn default_edit_keeps_the_r19_sheet() {
        let g = page_geometry(&photo(), &ImageEdit::default()).unwrap();
        let long = 297.0 * UNITS_PER_MM;
        assert!((g.sheet.width - long).abs() < 1e-9);
        assert!((g.sheet.height - long * 0.75).abs() < 1e-9);
        assert_eq!((g.block, g.raster_dpi), (2, 171));
        assert!(close(
            at(&g, 4000.0, 3000.0),
            Point {
                x: g.sheet.width,
                y: g.sheet.height
            }
        ));
        let scan = PageInfo {
            width: 600,
            height: 300,
            dpi: Some((200.0, 200.0)),
        };
        let g = page_geometry(&scan, &ImageEdit::default()).unwrap();
        assert!((g.sheet.width - 216.0).abs() < 1e-9 && (g.sheet.height - 108.0).abs() < 1e-9);
        assert_eq!((g.block, g.raster_dpi), (1, 200));
    }

    #[test]
    fn quarter_turn_sends_the_top_left_to_the_top_right() {
        let edit = ImageEdit {
            rotation: Rotation::Deg90,
            ..ImageEdit::default()
        };
        let g = page_geometry(&photo(), &edit).unwrap();
        assert!(g.sheet.height > g.sheet.width);
        assert!(close(
            at(&g, 0.0, 0.0),
            Point {
                x: g.sheet.width,
                y: 0.0
            }
        ));
        for rotation in [Rotation::Deg180, Rotation::Deg270] {
            let edit = ImageEdit {
                rotation,
                ..ImageEdit::default()
            };
            let g = page_geometry(&photo(), &edit).unwrap();
            let corner = at(&g, 0.0, 0.0);
            let want = if rotation == Rotation::Deg180 {
                Point {
                    x: g.sheet.width,
                    y: g.sheet.height,
                }
            } else {
                Point {
                    x: 0.0,
                    y: g.sheet.height,
                }
            };
            assert!(close(corner, want), "{rotation:?}");
        }
    }

    #[test]
    fn fine_angle_turns_about_the_center_onto_the_bounding_box() {
        let edit = ImageEdit {
            fine_angle: dec("5"),
            ..ImageEdit::default()
        };
        let g = page_geometry(&photo(), &edit).unwrap();
        let center = at(&g, 2000.0, 1500.0);
        assert!(close(
            center,
            Point {
                x: g.sheet.width / 2.0,
                y: g.sheet.height / 2.0
            }
        ));
        // Clockwise: the top left corner moves right onto the top edge of the box.
        let tl = at(&g, 0.0, 0.0);
        assert!(tl.y.abs() < 1e-6 && tl.x > 0.0, "{tl:?}");
        let base = page_geometry(&photo(), &ImageEdit::default()).unwrap();
        assert!(g.sheet.width > base.sheet.width && g.sheet.height > base.sheet.height);
    }

    #[test]
    fn corners_straighten_a_skewed_sheet() {
        let edit = ImageEdit {
            corners: Some(SheetCorners {
                top_left: image_point(Point { x: 400.0, y: 300.0 }),
                top_right: image_point(Point {
                    x: 3600.0,
                    y: 200.0,
                }),
                bottom_right: image_point(Point {
                    x: 3800.0,
                    y: 2900.0,
                }),
                bottom_left: image_point(Point {
                    x: 200.0,
                    y: 2700.0,
                }),
            }),
            paper: Some(Paper::Standard {
                size: PaperSize::A4,
                orientation: Orientation::Landscape,
            }),
            ..ImageEdit::default()
        };
        let g = page_geometry(&photo(), &edit).unwrap();
        assert!((g.sheet.width - 297.0 * UNITS_PER_MM).abs() < 1e-9);
        assert!(close(at(&g, 400.0, 300.0), Point { x: 0.0, y: 0.0 }));
        assert!(close(
            at(&g, 3800.0, 2900.0),
            Point {
                x: g.sheet.width,
                y: g.sheet.height
            }
        ));
        assert!(close(
            at(&g, 200.0, 2700.0),
            Point {
                x: 0.0,
                y: g.sheet.height
            }
        ));
    }

    #[test]
    fn trim_keeps_the_scale_and_paper_stretches() {
        let trim = Some(Trim {
            left: dec("0.25"),
            top: dec("0"),
            right: dec("0.75"),
            bottom: dec("0.5"),
        });
        let edit = ImageEdit {
            trim,
            ..ImageEdit::default()
        };
        let base = page_geometry(&photo(), &ImageEdit::default()).unwrap();
        let g = page_geometry(&photo(), &edit).unwrap();
        assert!((g.sheet.width - base.sheet.width / 2.0).abs() < 1e-9);
        assert!(close(at(&g, 1000.0, 0.0), Point { x: 0.0, y: 0.0 }));
        assert_eq!(g.raster_dpi, base.raster_dpi);
        let on_a4 = ImageEdit {
            trim,
            paper: Some(Paper::Standard {
                size: PaperSize::A4,
                orientation: Orientation::Portrait,
            }),
            ..ImageEdit::default()
        };
        let g = page_geometry(&photo(), &on_a4).unwrap();
        assert!(close(
            at(&g, 3000.0, 1500.0),
            Point {
                x: g.sheet.width,
                y: g.sheet.height
            }
        ));
    }

    #[test]
    fn corners_are_named_after_the_quarter_turns() {
        let info = PageInfo {
            width: 10,
            height: 20,
            dpi: None,
        };
        let c = page_corners(&info, Rotation::Deg90);
        assert_eq!(c.top_left, image_point(Point { x: 0.0, y: 20.0 }));
        assert_eq!(c.top_right, image_point(Point { x: 0.0, y: 0.0 }));
        assert_eq!(c.bottom_right, image_point(Point { x: 10.0, y: 0.0 }));
        // Corners of a turned page give the same sheet as the quarter turn.
        let turned = ImageEdit {
            rotation: Rotation::Deg90,
            ..ImageEdit::default()
        };
        let by_corners = ImageEdit {
            corners: Some(c),
            ..turned.clone()
        };
        let a = page_geometry(&info, &turned).unwrap();
        let b = page_geometry(&info, &by_corners).unwrap();
        assert!((a.sheet.width - b.sheet.width).abs() < 1e-9);
        let p = Point { x: 3.0, y: 7.0 };
        assert!(close(
            a.to_sheet.apply(p).unwrap(),
            b.to_sheet.apply(p).unwrap()
        ));
    }
}
