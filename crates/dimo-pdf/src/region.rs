//! Text and frames inside a region of a sheet, for box select (T2.6, FR-REC-01, spec 08
//! stages 2 and 4).
//!
//! The user draws an [`OrientedBox`] around a callout. [`RegionText`] holds what the PDF has
//! there:
//!
//! - **Characters** whose box center lies inside the region, in content order. A box that cuts
//!   through a run keeps only the characters it covers, so a neighbouring dimension that shares
//!   a text object is left out.
//! - **Runs** merged from those characters with the rules of [`crate::text`], each with a tight
//!   oriented box in its reading direction. Rotated text (dimensions aligned to a vertical
//!   dimension line) gets a box rotated with it.
//! - **Framed** runs: a closed rectangle drawn as a vector path tightly around the run marks a
//!   basic (theoretically exact) dimension (M2 decision 6, D-25). "Tightly" means the rectangle
//!   is aligned with the run, contains it, and leaves at most [`FRAME_MARGIN_EM`] around it.
//!
//! The geometry functions are pure and tested without PDFium; [`region_text`] collects the
//! characters and rectangles from a loaded page.

use dimo_core::geometry::{OrientedBox, Point, Size};
use pdfium_render::prelude::{
    PdfPage, PdfPageObject, PdfPageObjectCommon, PdfPageObjectsCommon, PdfPathSegmentType,
    PdfPathSegments,
};

use crate::PdfError;
use crate::geometry::SheetRect;
use crate::page_space::PageToSheet;
use crate::text::{
    Glyph, TextRun, across, along, corners, glyph, merge_runs_indexed, page_to_sheet, text_error,
};

/// Largest space between a run and a frame around it, in em of the run's font size.
pub const FRAME_MARGIN_EM: f64 = 1.2;
/// How far the run may stick out of its frame, in em (loose character boxes include the
/// font's full ascent and descent, which some frames cut).
pub const FRAME_OVERLAP_EM: f64 = 0.25;
/// Largest angle between a frame and the run's reading direction, in degrees.
pub const FRAME_ANGLE_DEG: f64 = 2.0;

/// One character inside a region, in sheet space.
#[derive(Debug, Clone, PartialEq)]
pub struct TextChar {
    /// The character as extracted (U+FFFD when the PDF has no Unicode value for it).
    pub ch: char,
    /// Loose box (advance width, font ascent to descent), axis aligned on the sheet.
    pub bbox: SheetRect,
    /// Rotation of the baseline in degrees, counterclockwise on the sheet, in `[0, 360)`.
    pub rotation: f64,
    /// Font size in sheet units.
    pub font_size: f64,
}

/// A run of text inside a region.
#[derive(Debug, Clone, PartialEq)]
pub struct RegionRun {
    /// The run as [`crate::Document::text_runs`] would report it for these characters.
    pub run: TextRun,
    /// Tight box in the run's reading direction: `angle` is the run rotation in `(-180, 180]`,
    /// width along the baseline, height from ascent to descent.
    pub frame: OrientedBox,
    /// A closed rectangle path lies tightly around the run (basic dimension, M2 decision 6).
    pub framed: bool,
}

/// What a region of a sheet contains. See the module docs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RegionText {
    /// Characters with their center inside the region, in content order. Whitespace included.
    pub chars: Vec<TextChar>,
    /// Runs built from `chars`, in content order.
    pub runs: Vec<RegionRun>,
}

/// Collects the text and frames of `page` inside `region` (sheet space).
pub(crate) fn region_text(
    page: &PdfPage<'_>,
    region: &OrientedBox,
) -> Result<RegionText, PdfError> {
    let to_sheet = page_to_sheet(page)?;
    let text = page.text().map_err(text_error)?;
    let glyphs: Vec<Glyph> = text
        .chars()
        .iter()
        .filter_map(|ch| glyph(&ch, &to_sheet))
        .filter(|g| contains(region, rect_center(g.bbox)))
        .collect();
    let rects = if glyphs.is_empty() {
        Vec::new()
    } else {
        closed_rectangles(page, &to_sheet, &aabb(region))
    };
    Ok(select(&glyphs, &rects))
}

/// Builds the region result from the characters inside the region and the closed rectangles
/// near it (each as four corners in sheet space).
pub(crate) fn select(glyphs: &[Glyph], rects: &[[(f64, f64); 4]]) -> RegionText {
    let chars = glyphs
        .iter()
        .map(|g| TextChar {
            ch: g.ch,
            bbox: g.bbox,
            rotation: g.rotation,
            font_size: g.font_size,
        })
        .collect();
    let runs = merge_runs_indexed(glyphs)
        .into_iter()
        .map(|(run, range)| {
            let frame = run_frame(&run, &glyphs[range]);
            let framed = rects
                .iter()
                .any(|r| is_tight_frame(&frame, run.font_size, r));
            RegionRun { run, frame, framed }
        })
        .collect();
    RegionText { chars, runs }
}

/// Unit vector of the reading direction for a rotation in degrees (sheet space, y down).
fn direction(rotation: f64) -> (f64, f64) {
    let rad = rotation.to_radians();
    (rad.cos(), -rad.sin())
}

/// Tight oriented box around the visible characters of a run in its reading direction.
fn run_frame(run: &TextRun, glyphs: &[Glyph]) -> OrientedBox {
    let dir = direction(run.rotation);
    let mut a = (f64::INFINITY, f64::NEG_INFINITY);
    let mut c = (f64::INFINITY, f64::NEG_INFINITY);
    for g in glyphs.iter().filter(|g| !g.ch.is_whitespace()) {
        for p in corners(g.bbox) {
            let (u, v) = (along(dir, p), across(dir, p));
            a = (a.0.min(u), a.1.max(u));
            c = (c.0.min(v), c.1.max(v));
        }
    }
    if !a.0.is_finite() {
        // No visible character: fall back to the axis aligned box.
        let b = run.bbox;
        a = (b.x, b.x + b.width);
        c = (b.y, b.y + b.height);
        return OrientedBox {
            center: Point {
                x: f64::midpoint(a.0, a.1),
                y: f64::midpoint(c.0, c.1),
            },
            size: Size {
                width: b.width,
                height: b.height,
            },
            angle: 0.0,
        };
    }
    let (u, v) = (f64::midpoint(a.0, a.1), f64::midpoint(c.0, c.1));
    // along = p . dir, across = p . n with n = (-dir.y, dir.x); invert the rotation.
    let center = Point {
        x: u * dir.0 - v * dir.1,
        y: u * dir.1 + v * dir.0,
    };
    OrientedBox {
        center,
        size: Size {
            width: a.1 - a.0,
            height: c.1 - c.0,
        },
        angle: signed_angle(run.rotation),
    }
}

/// An angle in degrees mapped to `(-180, 180]`.
fn signed_angle(degrees: f64) -> f64 {
    let a = degrees.rem_euclid(360.0);
    if a > 180.0 { a - 360.0 } else { a + 0.0 }
}

/// Coordinates of `p` in the frame of `b`: x along the box's width, y along its height, both
/// relative to the center.
fn local(b: &OrientedBox, p: (f64, f64)) -> (f64, f64) {
    let (sin, cos) = b.angle.to_radians().sin_cos();
    let (dx, dy) = (p.0 - b.center.x, p.1 - b.center.y);
    // Box axes in sheet space (y down): u = (cos, -sin), v = (sin, cos). See OrientedBox.
    (dx * cos - dy * sin, dx * sin + dy * cos)
}

/// Whether `p` lies inside the oriented box (edges included, with a tiny tolerance).
pub(crate) fn contains(b: &OrientedBox, p: (f64, f64)) -> bool {
    const EPS: f64 = 1e-9;
    let (x, y) = local(b, p);
    x.abs() <= b.size.width / 2.0 + EPS && y.abs() <= b.size.height / 2.0 + EPS
}

fn rect_center(r: SheetRect) -> (f64, f64) {
    (r.x + r.width / 2.0, r.y + r.height / 2.0)
}

/// Axis aligned bounds of an oriented box.
fn aabb(b: &OrientedBox) -> SheetRect {
    let pts = b.corners();
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in pts {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    SheetRect::new(x0, y0, x1 - x0, y1 - y0)
}

/// Whether the rectangle with corners `rect` is a frame tightly around `frame` (see the module
/// docs). `em` is the run's font size.
pub(crate) fn is_tight_frame(frame: &OrientedBox, em: f64, rect: &[(f64, f64); 4]) -> bool {
    let em = em.max(f64::EPSILON);
    // Aligned: every edge runs along or across the reading direction.
    let tolerance = FRAME_ANGLE_DEG.to_radians().sin();
    for i in 0..4 {
        let (p, q) = (rect[i], rect[(i + 1) % 4]);
        let (ex, ey) = (q.0 - p.0, q.1 - p.1);
        let len = ex.hypot(ey);
        if len <= f64::EPSILON {
            return false;
        }
        let rotated = OrientedBox {
            center: Point { x: 0.0, y: 0.0 },
            ..*frame
        };
        let (lx, ly) = local(&rotated, (ex, ey));
        if (lx.abs() / len).min(ly.abs() / len) > tolerance {
            return false;
        }
    }
    let local_pts = rect.map(|p| local(frame, p));
    let x0 = local_pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let x1 = local_pts
        .iter()
        .map(|p| p.0)
        .fold(f64::NEG_INFINITY, f64::max);
    let y0 = local_pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let y1 = local_pts
        .iter()
        .map(|p| p.1)
        .fold(f64::NEG_INFINITY, f64::max);
    let (hw, hh) = (frame.size.width / 2.0, frame.size.height / 2.0);
    let margins = [-hw - x0, x1 - hw, -hh - y0, y1 - hh];
    margins
        .iter()
        .all(|&m| m >= -FRAME_OVERLAP_EM * em && m <= FRAME_MARGIN_EM * em)
}

/// Closed four sided paths with right angles (rectangles, possibly rotated) on the page whose
/// bounds touch `near`, as corners in sheet space. Only top level path objects are read; frames
/// inside form `XObject`s are not found yet.
fn closed_rectangles(
    page: &PdfPage<'_>,
    to_sheet: &PageToSheet,
    near: &SheetRect,
) -> Vec<[(f64, f64); 4]> {
    let mut out = Vec::new();
    for object in page.objects().iter() {
        let PdfPageObject::Path(path) = &object else {
            continue;
        };
        let Ok(quad) = object.bounds() else {
            continue;
        };
        let bounds = to_sheet.rect(
            f64::from(quad.left().value),
            f64::from(quad.bottom().value),
            f64::from(quad.right().value),
            f64::from(quad.top().value),
        );
        if !overlaps(&bounds, near) {
            continue;
        }
        let Ok(matrix) = path.matrix() else {
            continue;
        };
        let segments = path.segments().transform(matrix);
        let mut subpath: Vec<(f64, f64)> = Vec::new();
        let finish = |points: &mut Vec<(f64, f64)>, out: &mut Vec<[(f64, f64); 4]>| {
            if let Some(rect) = rectangle(points) {
                out.push(rect.map(|(x, y)| to_sheet.point(x, y)));
            }
            points.clear();
        };
        let mut curved = false;
        for segment in segments.iter() {
            let (x, y) = segment.point();
            let p = (f64::from(x.value), f64::from(y.value));
            match segment.segment_type() {
                PdfPathSegmentType::MoveTo => {
                    if !curved {
                        finish(&mut subpath, &mut out);
                    }
                    subpath.clear();
                    curved = false;
                    subpath.push(p);
                }
                PdfPathSegmentType::LineTo => subpath.push(p),
                PdfPathSegmentType::BezierTo | PdfPathSegmentType::Unknown => curved = true,
            }
            if segment.is_close() && !curved {
                finish(&mut subpath, &mut out);
            }
        }
        if !curved {
            finish(&mut subpath, &mut out);
        }
    }
    out
}

fn overlaps(a: &SheetRect, b: &SheetRect) -> bool {
    a.x <= b.x + b.width && b.x <= a.x + a.width && a.y <= b.y + b.height && b.y <= a.y + a.height
}

/// The four corners if `points` (one sub path) is a rectangle: four corners with right angles,
/// optionally closed by a fifth point equal to the first.
pub(crate) fn rectangle(points: &[(f64, f64)]) -> Option<[(f64, f64); 4]> {
    const SAME: f64 = 1e-3;
    let mut pts: Vec<(f64, f64)> = points.to_vec();
    if pts.len() == 5 {
        let (first, last) = (pts[0], pts[4]);
        if (first.0 - last.0).abs() > SAME || (first.1 - last.1).abs() > SAME {
            return None;
        }
        pts.pop();
    }
    if pts.len() != 4 {
        return None;
    }
    let corner = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| {
        let (u, v) = ((b.0 - a.0, b.1 - a.1), (c.0 - b.0, c.1 - b.1));
        let (lu, lv) = (u.0.hypot(u.1), v.0.hypot(v.1));
        lu > SAME && lv > SAME && ((u.0 * v.0 + u.1 * v.1) / (lu * lv)).abs() < 0.035
    };
    (0..4)
        .all(|i| corner(pts[i], pts[(i + 1) % 4], pts[(i + 2) % 4]))
        .then(|| [pts[0], pts[1], pts[2], pts[3]])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: f64 = 10.0;
    const ADVANCE: f64 = 6.0;

    /// Characters of `text` on a horizontal baseline at `y`, starting at `x`.
    fn line(text: &str, x: f64, y: f64, size: f64) -> Vec<Glyph> {
        let advance = ADVANCE * size / SIZE;
        text.chars()
            .enumerate()
            .map(|(i, ch)| {
                #[allow(clippy::cast_precision_loss)]
                let gx = x + advance * i as f64;
                Glyph {
                    ch,
                    bbox: SheetRect::new(gx, y - 0.8 * size, advance, size),
                    origin: (gx, y),
                    rotation: 0.0,
                    font_name: "Sans".to_owned(),
                    font_size: size,
                }
            })
            .collect()
    }

    /// Characters of `text` reading bottom to top (rotation 90), baseline at `x`, starting at
    /// `y` and going up.
    fn vertical(text: &str, x: f64, y: f64) -> Vec<Glyph> {
        text.chars()
            .enumerate()
            .map(|(i, ch)| {
                #[allow(clippy::cast_precision_loss)]
                let gy = y - ADVANCE * i as f64;
                Glyph {
                    ch,
                    bbox: SheetRect::new(x - 8.0, gy - ADVANCE, SIZE, ADVANCE),
                    origin: (x, gy),
                    rotation: 90.0,
                    font_name: "Sans".to_owned(),
                    font_size: SIZE,
                }
            })
            .collect()
    }

    fn boxed(x: f64, y: f64, w: f64, h: f64, angle: f64) -> OrientedBox {
        OrientedBox {
            center: Point { x, y },
            size: Size {
                width: w,
                height: h,
            },
            angle,
        }
    }

    fn inside(glyphs: &[Glyph], region: &OrientedBox) -> Vec<Glyph> {
        glyphs
            .iter()
            .filter(|g| contains(region, rect_center(g.bbox)))
            .cloned()
            .collect()
    }

    fn texts(r: &RegionText) -> Vec<&str> {
        r.runs.iter().map(|r| r.run.text.as_str()).collect()
    }

    fn rect(x: f64, y: f64, w: f64, h: f64) -> [(f64, f64); 4] {
        [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
    }

    // FR-REC-01: the box keeps only the characters it covers.
    #[test]
    fn region_keeps_only_covered_characters() {
        let glyphs = line("50 Ø8 f7", 100.0, 50.0, SIZE);
        // Covers "Ø8 f7" (x 118 to 148), not "50".
        let region = boxed(134.0, 46.0, 34.0, 14.0, 0.0);
        let r = select(&inside(&glyphs, &region), &[]);
        assert_eq!(texts(&r), ["Ø8 f7"]);
        assert_eq!(r.chars.len(), 5);
        let f = r.runs[0].frame;
        assert_eq!((f.center.x, f.center.y), (133.0, 47.0));
        assert_eq!((f.size.width, f.size.height, f.angle), (30.0, 10.0, 0.0));
    }

    #[test]
    fn stacked_lines_stay_separate_runs() {
        let mut glyphs = line("Ø30 H7", 100.0, 50.0, SIZE);
        glyphs.extend(line("+0.0203", 140.0, 45.0, 7.0));
        glyphs.extend(line("-0", 140.0, 54.0, 7.0));
        let region = boxed(130.0, 48.0, 80.0, 30.0, 0.0);
        let r = select(&inside(&glyphs, &region), &[]);
        assert_eq!(texts(&r), ["Ø30 H7", "+0.0203", "-0"]);
    }

    // Rotated text (spec 08 stage 2): the frame turns with the run.
    #[test]
    fn rotated_run_gets_a_rotated_frame() {
        let glyphs = vertical("R15", 100.0, 200.0);
        // An axis aligned box drawn around the vertical text.
        let region = boxed(97.0, 191.0, 20.0, 30.0, 0.0);
        let r = select(&inside(&glyphs, &region), &[]);
        assert_eq!(texts(&r), ["R15"]);
        let f = r.runs[0].frame;
        assert_eq!(f.angle, 90.0);
        assert!((f.size.width - 18.0).abs() < 1e-9, "{f:?}");
        assert!((f.size.height - 10.0).abs() < 1e-9, "{f:?}");
        assert!((f.center.x - 97.0).abs() < 1e-9 && (f.center.y - 191.0).abs() < 1e-9);
        // A rotated query box selects the same characters.
        let turned = boxed(97.0, 191.0, 30.0, 20.0, 90.0);
        assert_eq!(texts(&select(&inside(&glyphs, &turned), &[])), ["R15"]);
    }

    #[test]
    fn contains_respects_the_box_rotation() {
        let b = boxed(0.0, 0.0, 40.0, 10.0, 90.0);
        assert!(contains(&b, (0.0, 19.0)));
        assert!(contains(&b, (4.0, -19.0)));
        assert!(!contains(&b, (19.0, 0.0)));
        let b = boxed(0.0, 0.0, 40.0, 10.0, 45.0);
        assert!(contains(&b, (10.0, -10.0)));
        assert!(!contains(&b, (10.0, 10.0)));
    }

    // M2 decision 6: a closed rectangle tightly around the run marks a basic dimension.
    #[test]
    fn tight_rectangle_frames_a_run() {
        let glyphs = line("25", 100.0, 50.0, SIZE); // box x 100..112, y 42..52
        let tight = rect(97.0, 40.0, 18.0, 14.0);
        let loose = rect(60.0, 20.0, 100.0, 60.0);
        let cut = rect(104.0, 40.0, 12.0, 14.0);
        let r = select(&glyphs, &[loose, cut]);
        assert!(!r.runs[0].framed);
        let r = select(&glyphs, &[loose, tight]);
        assert!(r.runs[0].framed);
    }

    #[test]
    fn rotated_frame_around_rotated_text() {
        let glyphs = vertical("25", 100.0, 200.0); // box x 92..102, y 188..200
        let frame = rect(90.0, 186.0, 14.0, 16.0);
        assert!(select(&glyphs, &[frame]).runs[0].framed);
        // A rectangle turned by 30 degrees is not a frame of this run.
        let c = (97.0, 194.0);
        let (s, k) = 30f64.to_radians().sin_cos();
        let turned = [(-9.0, -9.0), (9.0, -9.0), (9.0, 9.0), (-9.0, 9.0)]
            .map(|(x, y)| (c.0 + x * k - y * s, c.1 + x * s + y * k));
        assert!(!select(&glyphs, &[turned]).runs[0].framed);
    }

    #[test]
    fn rectangles_from_path_points() {
        let r = rectangle(&[(0.0, 0.0), (10.0, 0.0), (10.0, 5.0), (0.0, 5.0)]);
        assert!(r.is_some());
        let closed = rectangle(&[(0.0, 0.0), (10.0, 0.0), (10.0, 5.0), (0.0, 5.0), (0.0, 0.0)]);
        assert!(closed.is_some());
        assert!(rectangle(&[(0.0, 0.0), (10.0, 0.0), (12.0, 5.0), (0.0, 5.0)]).is_none());
        assert!(rectangle(&[(0.0, 0.0), (10.0, 0.0), (10.0, 5.0)]).is_none());
        let open = [(0.0, 0.0), (10.0, 0.0), (10.0, 5.0), (0.0, 5.0), (1.0, 1.0)];
        assert!(rectangle(&open).is_none());
    }

    #[test]
    fn empty_region_is_empty() {
        let glyphs = line("Ø8", 100.0, 50.0, SIZE);
        let region = boxed(500.0, 500.0, 10.0, 10.0, 0.0);
        assert_eq!(
            select(&inside(&glyphs, &region), &[]),
            RegionText::default()
        );
    }

    #[test]
    fn angles_map_to_signed_range() {
        assert_eq!(signed_angle(0.0), 0.0);
        assert_eq!(signed_angle(90.0), 90.0);
        assert_eq!(signed_angle(180.0), 180.0);
        assert_eq!(signed_angle(270.0), -90.0);
        assert_eq!(signed_angle(-0.0), 0.0);
    }
}
