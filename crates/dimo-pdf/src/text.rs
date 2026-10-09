//! Text runs with geometry (recognition stage 2, "PDF text").
//!
//! PDFium reports every character of a sheet with its Unicode value, a box, its baseline
//! origin, rotation, font and size. This module converts them to sheet space and merges them
//! into [`TextRun`]s: neighbouring characters with the same font, size and rotation that sit on
//! one baseline with at most a word gap between them.
//!
//! # What a run is
//!
//! - Characters are visited in content order (the order PDFium extracts them). A run never
//!   reorders characters, so text drawn in two separate pieces of the content stream stays two
//!   runs even when the pieces touch. Token grouping (stage 5) joins runs by geometry.
//! - Characters PDFium generates itself (spaces and line breaks it guesses between text
//!   objects) are ignored. Invisible text (render mode 3, typical for OCR layers of scans) is
//!   ignored: it is not what the sheet shows.
//! - Text keeps every code point as extracted, including `Ø` (U+00D8), `°`, `±` and the
//!   Unicode minus U+2212. Whitespace is normalized: any whitespace becomes one space, repeated
//!   spaces collapse, and runs are trimmed. A gap of at least [`SPACE_GAP_EM`] between two
//!   characters without a space glyph inserts one space.
//! - A character without a Unicode value is kept as U+FFFD, so the run shows that something is
//!   there.
//!
//! # Geometry
//!
//! [`TextRun::bbox`] is the axis aligned union of the characters' loose boxes in sheet space:
//! advance width along the baseline, font ascent to descent across it. Leading and trailing
//! spaces do not count. For rotated text the box is axis aligned on the sheet, use
//! [`TextRun::rotation`] to recover the reading direction.

use pdfium_render::prelude::{PdfDocument, PdfPage, PdfPageIndex, PdfPageTextChar, PdfiumError};

use crate::PdfError;
use crate::geometry::SheetRect;
use crate::page_space::{PageToSheet, normalize_degrees};

/// Characters belong to the same run only if their baseline offset is at most this many em.
pub const BASELINE_TOLERANCE_EM: f64 = 0.2;
/// Largest gap along the baseline, in em, that still joins two characters into one run.
pub const MAX_GAP_EM: f64 = 0.8;
/// Largest backward step along the baseline, in em, that still joins two characters (kerning,
/// overprinted accents).
pub const MAX_OVERLAP_EM: f64 = 0.3;
/// A gap of at least this many em between two characters inserts a space.
pub const SPACE_GAP_EM: f64 = 0.2;
/// Font sizes that differ by more than this fraction split runs.
pub const SIZE_TOLERANCE: f64 = 0.05;
/// Rotations that differ by more than this many degrees split runs.
pub const ROTATION_TOLERANCE_DEG: f64 = 1.0;

/// A piece of text on one baseline, in one font, size and rotation (FR-DOC-03, stage 2).
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    /// The characters as extracted, whitespace normalized (see the module docs).
    pub text: String,
    /// Axis aligned box around the characters in sheet space.
    pub bbox: SheetRect,
    /// Reading direction in degrees, counterclockwise as seen on the sheet, in `[0, 360)`.
    /// 0 reads left to right, 90 reads bottom to top.
    pub rotation: f64,
    /// Base font name without the subset tag (`AAAAAA+NotoSans` becomes `NotoSans`). Empty if
    /// PDFium reports none.
    pub font_name: String,
    /// Font size in sheet units (em size after all scaling).
    pub font_size: f64,
}

/// One extracted character in sheet space, the input of [`merge_runs`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Glyph {
    pub ch: char,
    /// Loose box in sheet space.
    pub bbox: SheetRect,
    /// Baseline origin in sheet space.
    pub origin: (f64, f64),
    /// Degrees counterclockwise on the sheet.
    pub rotation: f64,
    pub font_name: String,
    pub font_size: f64,
}

/// Extracts the text runs of one loaded page. Runs on the render thread.
pub(crate) fn text_runs(page: &PdfPage<'_>) -> Result<Vec<TextRun>, PdfError> {
    let to_sheet = page_to_sheet(page)?;
    let text = page.text().map_err(text_error)?;
    let glyphs: Vec<Glyph> = text
        .chars()
        .iter()
        .filter_map(|ch| glyph(&ch, &to_sheet))
        .collect();
    Ok(merge_runs(&glyphs))
}

/// Loads the page of `sheet`; a PDFium failure is mapped with `error`. The page does not borrow
/// the document, but must be dropped before it (see `page_cache`).
pub(crate) fn load_page<'a>(
    doc: &PdfDocument<'a>,
    sheet: usize,
    error: fn(PdfiumError) -> PdfError,
) -> Result<PdfPage<'a>, PdfError> {
    let count = usize::try_from(doc.pages().len()).unwrap_or(0);
    let out_of_range = PdfError::SheetOutOfRange {
        index: sheet,
        count,
    };
    if sheet >= count {
        return Err(out_of_range);
    }
    let index = PdfPageIndex::try_from(sheet).map_err(|_| out_of_range)?;
    doc.pages().get(index).map_err(error)
}

pub(crate) fn page_to_sheet(page: &PdfPage<'_>) -> Result<PageToSheet, PdfError> {
    let bbox = page.boundaries().bounding().map_err(text_error)?.bounds;
    let quarter_turns = match page.rotation().map_err(text_error)? {
        pdfium_render::prelude::PdfPageRenderRotation::None => 0,
        pdfium_render::prelude::PdfPageRenderRotation::Degrees90 => 1,
        pdfium_render::prelude::PdfPageRenderRotation::Degrees180 => 2,
        pdfium_render::prelude::PdfPageRenderRotation::Degrees270 => 3,
    };
    Ok(PageToSheet::new(
        (
            f64::from(bbox.left().value),
            f64::from(bbox.bottom().value),
            f64::from(bbox.right().value),
            f64::from(bbox.top().value),
        ),
        quarter_turns,
    ))
}

/// Converts one PDFium character, or `None` for generated, invisible or boxless characters.
fn glyph(ch: &PdfPageTextChar<'_>, to_sheet: &PageToSheet) -> Option<Glyph> {
    if ch.is_generated().unwrap_or(false) {
        return None;
    }
    if !ch.text_object().is_ok_and(|object| object.is_visible()) {
        return None;
    }
    let bounds = ch.loose_bounds().ok()?;
    let (ox, oy) = ch.origin().ok()?;
    // Rotation and size come from the character matrix. PDFium's own char angle is clockwise,
    // and pdfium-render's scaled font size multiplies by the matrix `d` only, which is 0 for
    // text rotated by 90 degrees. The matrix maps the glyph's x axis to (a, b) and its y axis
    // to (c, d) in page space.
    let (rotation, scale) = ch.matrix().map_or((0.0, 1.0), |m| {
        let (a, b, c, d) = (
            f64::from(m.a()),
            f64::from(m.b()),
            f64::from(m.c()),
            f64::from(m.d()),
        );
        (b.atan2(a).to_degrees(), c.hypot(d))
    });
    Some(Glyph {
        ch: ch.unicode_char().unwrap_or(char::REPLACEMENT_CHARACTER),
        bbox: to_sheet.rect(
            f64::from(bounds.left().value),
            f64::from(bounds.bottom().value),
            f64::from(bounds.right().value),
            f64::from(bounds.top().value),
        ),
        origin: to_sheet.point(f64::from(ox.value), f64::from(oy.value)),
        rotation: to_sheet.angle(rotation),
        font_name: strip_subset_tag(&ch.font_name()).to_owned(),
        font_size: f64::from(ch.unscaled_font_size().value) * scale,
    })
}

/// Removes the subset tag of an embedded subset font: six uppercase letters and `+`
/// (ISO 32000-1, 9.6.4).
pub(crate) fn strip_subset_tag(name: &str) -> &str {
    match name.split_once('+') {
        Some((tag, rest)) if tag.len() == 6 && tag.bytes().all(|b| b.is_ascii_uppercase()) => rest,
        _ => name,
    }
}

#[allow(clippy::needless_pass_by_value)] // used as a map_err callback
pub(crate) fn text_error(e: PdfiumError) -> PdfError {
    PdfError::Text(e.to_string())
}

/// Merges characters in content order into runs (see the module docs).
pub(crate) fn merge_runs(glyphs: &[Glyph]) -> Vec<TextRun> {
    let mut runs = Vec::new();
    let mut current: Option<RunBuilder> = None;
    for g in glyphs {
        if let Some(builder) = current.as_mut()
            && builder.accepts(g)
        {
            builder.push(g);
            continue;
        }
        if let Some(run) = current.take().and_then(RunBuilder::finish) {
            runs.push(run);
        }
        current = Some(RunBuilder::start(g));
    }
    if let Some(run) = current.and_then(RunBuilder::finish) {
        runs.push(run);
    }
    runs
}

struct RunBuilder {
    text: String,
    bbox: Option<SheetRect>,
    rotation: f64,
    font_name: String,
    font_size: f64,
    /// Unit vector along the baseline in sheet space.
    dir: (f64, f64),
    /// Position of the baseline across the reading direction.
    baseline: f64,
    /// Furthest extent of the characters so far along the reading direction.
    end: f64,
}

impl RunBuilder {
    fn start(g: &Glyph) -> Self {
        let rad = g.rotation.to_radians();
        // Sheet space has y downward, so counterclockwise on the sheet means negative y.
        let dir = (rad.cos(), -rad.sin());
        let mut builder = Self {
            text: String::new(),
            bbox: None,
            rotation: g.rotation,
            font_name: g.font_name.clone(),
            font_size: g.font_size,
            dir,
            baseline: across(dir, g.origin),
            end: f64::NEG_INFINITY,
        };
        builder.push(g);
        builder
    }

    fn accepts(&self, g: &Glyph) -> bool {
        let em = self.font_size.max(f64::EPSILON);
        let rotation_diff = normalize_degrees(g.rotation - self.rotation);
        let rotation_diff = rotation_diff.min(360.0 - rotation_diff);
        let gap = along(self.dir, g.origin) - self.end;
        g.font_name == self.font_name
            && (g.font_size - self.font_size).abs() <= SIZE_TOLERANCE * em
            && rotation_diff <= ROTATION_TOLERANCE_DEG
            && (across(self.dir, g.origin) - self.baseline).abs() <= BASELINE_TOLERANCE_EM * em
            && gap >= -MAX_OVERLAP_EM * em
            && gap <= MAX_GAP_EM * em
    }

    fn push(&mut self, g: &Glyph) {
        let em = self.font_size.max(f64::EPSILON);
        let last_is_space = self.text.is_empty() || self.text.ends_with(' ');
        if g.ch.is_whitespace() {
            if !last_is_space {
                self.text.push(' ');
            }
        } else {
            let gap = along(self.dir, g.origin) - self.end;
            if !last_is_space && gap >= SPACE_GAP_EM * em {
                self.text.push(' ');
            }
            self.text.push(g.ch);
            self.bbox = Some(match self.bbox {
                Some(b) => union(b, g.bbox),
                None => g.bbox,
            });
        }
        let extent = corners(g.bbox)
            .into_iter()
            .map(|p| along(self.dir, p))
            .fold(f64::NEG_INFINITY, f64::max);
        self.end = self.end.max(extent);
    }

    fn finish(self) -> Option<TextRun> {
        let text = self.text.trim();
        if text.is_empty() {
            return None;
        }
        Some(TextRun {
            text: text.to_owned(),
            bbox: self.bbox?,
            rotation: self.rotation,
            font_name: self.font_name,
            font_size: self.font_size,
        })
    }
}

fn along(dir: (f64, f64), p: (f64, f64)) -> f64 {
    p.0 * dir.0 + p.1 * dir.1
}

/// Distance across the reading direction (toward the descenders is positive).
fn across(dir: (f64, f64), p: (f64, f64)) -> f64 {
    -p.0 * dir.1 + p.1 * dir.0
}

fn corners(r: SheetRect) -> [(f64, f64); 4] {
    [
        (r.x, r.y),
        (r.x + r.width, r.y),
        (r.x, r.y + r.height),
        (r.x + r.width, r.y + r.height),
    ]
}

pub(crate) fn union(a: SheetRect, b: SheetRect) -> SheetRect {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = (a.x + a.width).max(b.x + b.width);
    let y1 = (a.y + a.height).max(b.y + b.height);
    SheetRect::new(x0, y0, x1 - x0, y1 - y0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: f64 = 10.0;
    const ADVANCE: f64 = 6.0;

    /// Glyphs of `text` set horizontally from baseline origin (x, y), 6 units per character,
    /// ascent 8 and descent 2.
    fn line(text: &str, x: f64, y: f64) -> Vec<Glyph> {
        text.chars()
            .enumerate()
            .map(|(i, ch)| {
                #[allow(clippy::cast_precision_loss)]
                let gx = x + ADVANCE * i as f64;
                Glyph {
                    ch,
                    bbox: SheetRect::new(gx, y - 8.0, ADVANCE, 10.0),
                    origin: (gx, y),
                    rotation: 0.0,
                    font_name: "Sans".to_owned(),
                    font_size: SIZE,
                }
            })
            .collect()
    }

    fn texts(runs: &[TextRun]) -> Vec<&str> {
        runs.iter().map(|r| r.text.as_str()).collect()
    }

    #[test]
    fn one_line_is_one_run() {
        let runs = merge_runs(&line("Ø30 H7", 100.0, 50.0));
        assert_eq!(texts(&runs), ["Ø30 H7"]);
        assert_eq!(runs[0].bbox, SheetRect::new(100.0, 42.0, 36.0, 10.0));
        assert_eq!(runs[0].rotation, 0.0);
        assert_eq!(runs[0].font_name, "Sans");
    }

    #[test]
    fn symbols_are_kept_exactly() {
        let runs = merge_runs(&line("−0.1°", 0.0, 20.0));
        assert_eq!(runs[0].text, "\u{2212}0.1\u{B0}");
    }

    #[test]
    fn stacked_deviations_split_by_baseline() {
        // Main text, then upper and lower deviation right of it, half an em up and down.
        let mut glyphs = line("Ø30 H7 ", 100.0, 50.0);
        glyphs.extend(line("+0.0203", 145.0, 45.0));
        glyphs.extend(line("-0", 145.0, 55.0));
        let runs = merge_runs(&glyphs);
        assert_eq!(texts(&runs), ["Ø30 H7", "+0.0203", "-0"]);
        // The trailing space does not widen the box.
        assert_eq!(runs[0].bbox.width, 36.0);
    }

    #[test]
    fn small_baseline_jitter_is_one_run() {
        let mut glyphs = line("12", 0.0, 50.0);
        glyphs.extend(line("34", 12.0, 51.5));
        assert_eq!(texts(&merge_runs(&glyphs)), ["1234"]);
    }

    #[test]
    fn pieces_on_one_baseline_join_with_gap_rules() {
        // A word gap of 3 units (0.3 em) inserts a space; repeated spaces collapse.
        let mut glyphs = line("1 ", 0.0, 50.0);
        glyphs.extend(line(" of ", 15.0, 50.0));
        glyphs.extend(line("1", 42.0, 50.0));
        assert_eq!(texts(&merge_runs(&glyphs)), ["1 of 1"]);

        let mut glyphs = line("AB", 0.0, 50.0);
        glyphs.extend(line("CD", 15.0, 50.0)); // gap 3 = 0.3 em
        assert_eq!(texts(&merge_runs(&glyphs)), ["AB CD"]);

        let mut glyphs = line("AB", 0.0, 50.0);
        glyphs.extend(line("CD", 13.0, 50.0)); // gap 1 = 0.1 em
        assert_eq!(texts(&merge_runs(&glyphs)), ["ABCD"]);

        let mut glyphs = line("AB", 0.0, 50.0);
        glyphs.extend(line("CD", 25.0, 50.0)); // gap 13 = 1.3 em
        assert_eq!(texts(&merge_runs(&glyphs)), ["AB", "CD"]);
    }

    #[test]
    fn next_line_starts_a_new_run() {
        let mut glyphs = line("BREAK ALL", 0.0, 50.0);
        glyphs.extend(line("REMOVE", 0.0, 62.0));
        assert_eq!(texts(&merge_runs(&glyphs)), ["BREAK ALL", "REMOVE"]);
        // Same baseline but going backwards (content order jumps left).
        let mut glyphs = line("AB", 100.0, 50.0);
        glyphs.extend(line("CD", 0.0, 50.0));
        assert_eq!(texts(&merge_runs(&glyphs)), ["AB", "CD"]);
    }

    #[test]
    fn font_size_and_rotation_split_runs() {
        let mut glyphs = line("AB", 0.0, 50.0);
        let mut small = line("C", 12.0, 50.0);
        small[0].font_size = 7.0;
        glyphs.extend(small);
        let mut other_font = line("D", 18.0, 50.0);
        other_font[0].font_name = "Symbols".to_owned();
        glyphs.extend(other_font);
        assert_eq!(texts(&merge_runs(&glyphs)), ["AB", "C", "D"]);
    }

    #[test]
    fn rotated_text_merges_along_its_baseline() {
        // Reading bottom to top (rotation 90): the pen moves toward smaller y.
        let glyphs: Vec<Glyph> = "R15"
            .chars()
            .enumerate()
            .map(|(i, ch)| {
                #[allow(clippy::cast_precision_loss)]
                let gy = 200.0 - ADVANCE * i as f64;
                Glyph {
                    ch,
                    bbox: SheetRect::new(92.0, gy - ADVANCE, 10.0, ADVANCE),
                    origin: (100.0, gy),
                    rotation: 90.0,
                    font_name: "Sans".to_owned(),
                    font_size: SIZE,
                }
            })
            .collect();
        let runs = merge_runs(&glyphs);
        assert_eq!(texts(&runs), ["R15"]);
        assert_eq!(runs[0].bbox, SheetRect::new(92.0, 182.0, 10.0, 18.0));
        assert_eq!(runs[0].rotation, 90.0);
    }

    #[test]
    fn whitespace_only_runs_are_dropped() {
        assert_eq!(merge_runs(&line("   ", 0.0, 0.0)), Vec::new());
        assert_eq!(merge_runs(&[]), Vec::new());
    }

    #[test]
    fn subset_tags_are_stripped() {
        assert_eq!(strip_subset_tag("AAAAAA+NotoSans"), "NotoSans");
        assert_eq!(strip_subset_tag("NotoSans"), "NotoSans");
        assert_eq!(strip_subset_tag("Abc+Def"), "Abc+Def");
        assert_eq!(strip_subset_tag(""), "");
    }
}
