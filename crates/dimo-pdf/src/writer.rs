//! Writing the ballooned PDF with PDFium (T1.2, FR-EXP-01, FR-EXP-11, D-33).
//!
//! Runs on the render thread. The original bytes are loaded as a separate PDFium document,
//! so documents open for the viewer are never touched. Balloons become PDFium page objects
//! (paths and text) in page space; PDFium appends them as a new content stream behind the
//! original ones (page content mode) or as appearance streams of stamp annotations
//! (annotation mode). The original content streams are copied unchanged.
//!
//! # Determinism (FR-EXP-11)
//!
//! PDFium's writer is deterministic except for the file identifier in the trailer: it fills
//! the generated parts of `/ID` from the clock and a random number. [`fix_trailer_id`] replaces
//! them with hashes of the same length after saving, so byte offsets and the cross reference
//! table stay valid. Annotation dates come from the caller ([`crate::PdfDate`]); pdfium-render
//! sets the creation date of a new annotation to the current time, and PDFium sets the
//! modification date when an object is appended, so both are overwritten afterwards.
//!
//! # Font
//!
//! Text uses a subset of the bundled font with exactly the characters of all balloon texts,
//! loaded once per document as a simple TrueType font with `WinAnsiEncoding` (see
//! [`crate::font`]), shared by all pages and annotations.

use std::collections::BTreeSet;

use pdfium_render::prelude::{
    PdfColor, PdfDocument, PdfFontToken, PdfMatrix, PdfPage, PdfPageAnnotationCommon,
    PdfPageContentRegenerationStrategy, PdfPageIndex, PdfPageObjectCommon, PdfPageObjectsCommon,
    PdfPagePathObject, PdfPageTextObject, PdfPoints, PdfRect, Pdfium, PdfiumError,
};

use crate::PdfError;
use crate::font::{BALLOON_FONT, TrueTypeFont};
use crate::hash::ContentHash;
use crate::overlay::{
    Balloon, BalloonOutput, BalloonOverlay, PathSegment, PdfDate, Rgb, SheetPoint, Stroke,
};
use crate::text::page_to_sheet;

#[allow(clippy::needless_pass_by_value)] // used as a map_err callback
fn write_error(e: PdfiumError) -> PdfError {
    PdfError::Write(e.to_string())
}

/// Writes `overlay` into a copy of `original` and returns the new PDF.
pub(crate) fn write_ballooned(
    pdfium: &Pdfium,
    original: &[u8],
    overlay: &BalloonOverlay,
) -> Result<Vec<u8>, PdfError> {
    overlay.validate()?;
    let mut doc = pdfium
        .load_pdf_from_byte_slice(original, None)
        .map_err(|e| PdfError::Open(e.to_string()))?;
    let count = usize::try_from(doc.pages().len()).unwrap_or(0);
    if let Some(bad) = overlay.sheets.iter().find(|s| s.sheet >= count) {
        return Err(PdfError::SheetOutOfRange {
            index: bad.sheet,
            count,
        });
    }

    let font = TrueTypeFont::parse(BALLOON_FONT)?;
    let chars: BTreeSet<char> = overlay
        .sheets
        .iter()
        .flat_map(|s| &s.balloons)
        .flat_map(|b| b.text.chars())
        .collect();
    let token = if chars.iter().any(|c| *c != ' ') {
        let subset = font.subset(&chars)?;
        Some(
            doc.fonts_mut()
                .load_true_type_from_bytes(&subset, false)
                .map_err(write_error)?,
        )
    } else {
        None
    };
    let writer = Writer {
        doc: &doc,
        font: &font,
        token,
    };
    for sheet in &overlay.sheets {
        if sheet.balloons.is_empty() {
            continue;
        }
        let index = PdfPageIndex::try_from(sheet.sheet).map_err(|_| PdfError::SheetOutOfRange {
            index: sheet.sheet,
            count,
        })?;
        let mut page = doc.pages().get(index).map_err(write_error)?;
        page.set_content_regeneration_strategy(PdfPageContentRegenerationStrategy::Manual);
        let to_page = page_to_sheet(&page)?.sheet_to_page();
        match overlay.output {
            BalloonOutput::PageContent => {
                writer.add_to_content(&mut page, &sheet.balloons, to_page)?;
            }
            BalloonOutput::Annotations { date } => {
                for balloon in &sheet.balloons {
                    writer.add_annotation(&mut page, balloon, to_page, date)?;
                }
            }
        }
    }
    let mut bytes = doc.save_to_bytes().map_err(write_error)?;
    fix_trailer_id(&mut bytes, original)?;
    Ok(bytes)
}

/// Sheet to page matrix `[a b c d e f]`, see `PageToSheet::sheet_to_page`.
type Matrix = [f64; 6];

/// Applies `matrix` to a sheet point. PDFium stores coordinates as `f32`; screen geometry only.
#[allow(clippy::cast_possible_truncation)]
fn to_page(matrix: Matrix, point: SheetPoint) -> (PdfPoints, PdfPoints) {
    let [xx, xy, yx, yy, tx, ty] = matrix;
    (
        PdfPoints::new((xx * point.x + yx * point.y + tx) as f32),
        PdfPoints::new((xy * point.x + yy * point.y + ty) as f32),
    )
}

/// `first` then `then`, for PDF row vector matrices.
fn concat(first: Matrix, then: Matrix) -> Matrix {
    let [a1, b1, c1, d1, e1, f1] = first;
    let [a2, b2, c2, d2, e2, f2] = then;
    [
        a1 * a2 + b1 * c2,
        a1 * b2 + b1 * d2,
        c1 * a2 + d1 * c2,
        c1 * b2 + d1 * d2,
        e1 * a2 + f1 * c2 + e2,
        e1 * b2 + f1 * d2 + f2,
    ]
}

const fn color(c: Rgb) -> PdfColor {
    PdfColor::new(c.r, c.g, c.b, 255)
}

#[allow(clippy::cast_possible_truncation)]
fn points(v: f64) -> PdfPoints {
    PdfPoints::new(v as f32)
}

struct Writer<'d, 'a> {
    doc: &'d PdfDocument<'a>,
    font: &'d TrueTypeFont<'static>,
    token: Option<PdfFontToken>,
}

/// The page objects of one balloon in drawing order.
struct BalloonObjects<'a> {
    leader: Option<PdfPagePathObject<'a>>,
    shape: Option<PdfPagePathObject<'a>>,
    text: Option<PdfPageTextObject<'a>>,
}

impl<'a> Writer<'_, 'a> {
    fn add_to_content(
        &self,
        page: &mut PdfPage<'a>,
        balloons: &[Balloon],
        m: Matrix,
    ) -> Result<(), PdfError> {
        let objects = balloons
            .iter()
            .map(|b| self.balloon_objects(b, m))
            .collect::<Result<Vec<_>, _>>()?;
        let mut rest = Vec::with_capacity(objects.len());
        // All leaders first, so no leader crosses a balloon.
        for o in objects {
            if let Some(leader) = o.leader {
                page.objects_mut()
                    .add_path_object(leader)
                    .map_err(write_error)?;
            }
            rest.push((o.shape, o.text));
        }
        for (shape, text) in rest {
            if let Some(shape) = shape {
                page.objects_mut()
                    .add_path_object(shape)
                    .map_err(write_error)?;
            }
            if let Some(text) = text {
                page.objects_mut()
                    .add_text_object(text)
                    .map_err(write_error)?;
            }
        }
        page.regenerate_content().map_err(write_error)
    }

    fn add_annotation(
        &self,
        page: &mut PdfPage<'a>,
        balloon: &Balloon,
        m: Matrix,
        date: PdfDate,
    ) -> Result<(), PdfError> {
        let objects = self.balloon_objects(balloon, m)?;
        let (left, top, right, bottom) = balloon.bounds();
        let (x0, y0) = to_page(m, SheetPoint::new(left, top));
        let (x1, y1) = to_page(m, SheetPoint::new(right, bottom));
        // PdfRect::new orders the edges itself.
        let rect = PdfRect::new(y0, x0, y1, x1);
        let mut annotation = page
            .annotations_mut()
            .create_stamp_annotation()
            .map_err(write_error)?;
        // The appearance stream's box is taken from the rectangle, so set it first.
        annotation.set_bounds(rect).map_err(write_error)?;
        annotation
            .set_contents(&balloon.text)
            .map_err(write_error)?;
        annotation.set_is_printed(true).map_err(write_error)?;
        let parts = [
            objects.leader.map(Into::into),
            objects.shape.map(Into::into),
            objects.text.map(Into::into),
        ];
        for object in parts.into_iter().flatten() {
            annotation
                .objects_mut()
                .add_object(object)
                .map_err(write_error)?;
        }
        let date = date_time(date)?;
        annotation.set_creation_date(date).map_err(write_error)?;
        annotation.set_modification_date(date).map_err(write_error)
    }

    fn balloon_objects(&self, b: &Balloon, m: Matrix) -> Result<BalloonObjects<'a>, PdfError> {
        let leader = match b.leader {
            Some(leader) => match b.leader_start(leader.anchor) {
                Some(start) => Some(self.line(start, leader.anchor, leader.stroke, m)?),
                None => None,
            },
            None => None,
        };
        let shape = if b.fill.is_some() || b.outline.is_some() {
            Some(self.path(&b.outline_path(), b.fill, b.outline, m)?)
        } else {
            None
        };
        let text = match self.token {
            Some(token) if !b.text.trim().is_empty() => Some(self.text(b, token, m)?),
            _ => None,
        };
        Ok(BalloonObjects {
            leader,
            shape,
            text,
        })
    }

    fn line(
        &self,
        from: SheetPoint,
        to: SheetPoint,
        stroke: Stroke,
        m: Matrix,
    ) -> Result<PdfPagePathObject<'a>, PdfError> {
        let (x0, y0) = to_page(m, from);
        let (x1, y1) = to_page(m, to);
        let mut path = PdfPagePathObject::new(
            self.doc,
            x0,
            y0,
            Some(color(stroke.color)),
            Some(points(stroke.width)),
            None,
        )
        .map_err(write_error)?;
        path.line_to(x1, y1).map_err(write_error)?;
        Ok(path)
    }

    fn path(
        &self,
        segments: &[PathSegment],
        fill: Option<Rgb>,
        outline: Option<Stroke>,
        m: Matrix,
    ) -> Result<PdfPagePathObject<'a>, PdfError> {
        let Some(PathSegment::MoveTo(start)) = segments.first() else {
            return Err(PdfError::Write("path must start with a move".to_owned()));
        };
        let (x, y) = to_page(m, *start);
        let mut path = PdfPagePathObject::new(
            self.doc,
            x,
            y,
            outline.map(|s| color(s.color)),
            outline.map(|s| points(s.width)),
            fill.map(color),
        )
        .map_err(write_error)?;
        for segment in &segments[1..] {
            match *segment {
                PathSegment::MoveTo(p) => {
                    let (x, y) = to_page(m, p);
                    path.move_to(x, y)
                }
                PathSegment::LineTo(p) => {
                    let (x, y) = to_page(m, p);
                    path.line_to(x, y)
                }
                PathSegment::CurveTo(c1, c2, p) => {
                    let (x1, y1) = to_page(m, c1);
                    let (x2, y2) = to_page(m, c2);
                    let (x, y) = to_page(m, p);
                    path.bezier_to(x, y, x1, y1, x2, y2)
                }
                PathSegment::Close => path.close_path(),
            }
            .map_err(write_error)?;
        }
        Ok(path)
    }

    fn text(
        &self,
        b: &Balloon,
        token: PdfFontToken,
        m: Matrix,
    ) -> Result<PdfPageTextObject<'a>, PdfError> {
        let width = self.font.text_width(&b.text, b.text_size);
        let cap =
            f64::from(self.font.cap_height()) * b.text_size / f64::from(self.font.units_per_em());
        // Baseline origin on the sheet; glyph space has y upward, sheet space downward.
        let origin = [
            1.0,
            0.0,
            0.0,
            -1.0,
            b.center.x - width / 2.0,
            b.center.y + cap / 2.0,
        ];
        let page = concat(origin, m);
        let mut text = PdfPageTextObject::new(self.doc, &b.text, token, points(b.text_size))
            .map_err(write_error)?;
        text.set_fill_color(color(b.text_color))
            .map_err(write_error)?;
        #[allow(clippy::cast_possible_truncation)]
        let matrix = PdfMatrix::new(
            page[0] as f32,
            page[1] as f32,
            page[2] as f32,
            page[3] as f32,
            page[4] as f32,
            page[5] as f32,
        );
        text.apply_matrix(matrix).map_err(write_error)?;
        Ok(text)
    }
}

fn date_time(date: PdfDate) -> Result<chrono::DateTime<chrono::Utc>, PdfError> {
    chrono::NaiveDate::from_ymd_opt(
        i32::from(date.year),
        u32::from(date.month),
        u32::from(date.day),
    )
    .and_then(|d| {
        d.and_hms_opt(
            u32::from(date.hour),
            u32::from(date.minute),
            u32::from(date.second),
        )
    })
    .map(|dt| dt.and_utc())
    .ok_or_else(|| PdfError::InvalidOverlay(format!("invalid date {date:?}")))
}

/// Replaces the parts of the trailer `/ID` that PDFium generated from the clock and a random
/// number with deterministic values of the same length (FR-EXP-11).
///
/// PDFium writes `/ID[<first><second>]`. If the original file had an `/ID`, `first` is copied
/// from it and stays; otherwise PDFium generates it and writes the same value twice. Generated
/// values are replaced: `first` by the leading bytes of the SHA-256 of the original file,
/// `second` by the leading bytes of the SHA-256 of the output with `second` zeroed. A missing
/// `/ID` is left alone; any other shape is an error, because the output would not be
/// reproducible.
pub(crate) fn fix_trailer_id(out: &mut [u8], original: &[u8]) -> Result<(), PdfError> {
    let unexpected = || PdfError::Write("unexpected trailer /ID written by PDFium".to_owned());
    let Some(trailer) = rfind(out, b"trailer") else {
        return Err(unexpected());
    };
    let Some(id) = find(&out[trailer..], b"/ID").map(|i| trailer + i + 3) else {
        return Ok(());
    };
    let open = skip_ws(out, id);
    if out.get(open) != Some(&b'[') {
        return Err(unexpected());
    }
    let first = pdf_string(out, skip_ws(out, open + 1)).ok_or_else(unexpected)?;
    let second = pdf_string(out, skip_ws(out, first.end)).ok_or_else(unexpected)?;
    if out.get(skip_ws(out, second.end)) != Some(&b']') || !second.hex {
        return Err(unexpected());
    }
    let generated_first = first.hex && out[first.content()] == out[second.content()];
    if generated_first {
        write_hex(out, first.content(), ContentHash::of(original).as_bytes())?;
    }
    out[second.content()].fill(b'0');
    let hash = ContentHash::of(out);
    write_hex(out, second.content(), hash.as_bytes())
}

/// Writes the leading bytes of `value` as upper case hex into `range`.
fn write_hex(out: &mut [u8], range: std::ops::Range<usize>, value: &[u8]) -> Result<(), PdfError> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let len = range.len();
    if !len.is_multiple_of(2) || len / 2 > value.len() {
        return Err(PdfError::Write(format!("trailer /ID of {len} hex digits")));
    }
    for (i, byte) in value.iter().take(len / 2).enumerate() {
        out[range.start + 2 * i] = HEX[usize::from(byte >> 4)];
        out[range.start + 2 * i + 1] = HEX[usize::from(byte & 0x0F)];
    }
    Ok(())
}

/// A string object in `out`: `<hex>` or `(literal)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PdfString {
    /// Position of the opening delimiter.
    start: usize,
    /// Position after the closing delimiter.
    end: usize,
    hex: bool,
}

impl PdfString {
    const fn content(&self) -> std::ops::Range<usize> {
        self.start + 1..self.end - 1
    }
}

fn pdf_string(data: &[u8], start: usize) -> Option<PdfString> {
    match data.get(start)? {
        b'<' => {
            let len = data[start + 1..].iter().position(|&c| c == b'>')?;
            let content = &data[start + 1..start + 1 + len];
            content
                .iter()
                .all(u8::is_ascii_hexdigit)
                .then_some(PdfString {
                    start,
                    end: start + len + 2,
                    hex: true,
                })
        }
        b'(' => {
            let mut depth = 0usize;
            let mut i = start;
            while i < data.len() {
                match data[i] {
                    b'\\' => i += 1,
                    b'(' => depth += 1,
                    b')' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(PdfString {
                                start,
                                end: i + 1,
                                hex: false,
                            });
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            None
        }
        _ => None,
    }
}

fn skip_ws(data: &[u8], mut i: usize) -> usize {
    while data
        .get(i)
        .is_some_and(|c| matches!(c, b' ' | b'\t' | b'\r' | b'\n' | b'\x0C' | b'\0'))
    {
        i += 1;
    }
    i
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn rfind(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).rposition(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAIL: &str = "xref\n0 1\n0000000000 65535 f \nstartxref\n9\n%%EOF\n";

    fn trailer(id: &str) -> Vec<u8> {
        format!("%PDF-1.7\nstream (trailer) endstream\ntrailer\r\n<</Root 1 0 R /Size 2/ID{id}>>\n{TAIL}")
            .into_bytes()
    }

    fn id_part(out: &[u8]) -> String {
        let s = String::from_utf8_lossy(out);
        let start = s.rfind("/ID").unwrap();
        s[start..s.rfind(">>").unwrap()].to_owned()
    }

    #[test]
    fn generated_ids_become_hashes() {
        let random = "A1B2C3D4E5F60718293A4B5C6D7E8F90";
        let mut out = trailer(&format!("[<{random}><{random}>]"));
        let len = out.len();
        fix_trailer_id(&mut out, b"original").unwrap();
        assert_eq!(out.len(), len);
        let first = &ContentHash::of(b"original").to_hex().to_uppercase()[..32];
        let id = id_part(&out);
        assert!(id.starts_with(&format!("/ID[<{first}><")), "{id}");
        assert!(!id.contains(random));
        // Only the random value matters: another random value gives the same output.
        let mut other =
            trailer("[<00000000000000000000000000000001><00000000000000000000000000000001>]");
        fix_trailer_id(&mut other, b"original").unwrap();
        assert_eq!(out, other);
        // The second part depends on the rest of the file.
        let mut changed = trailer(&format!("[<{random}><{random}>]"));
        changed[1] = b'Q';
        fix_trailer_id(&mut changed, b"original").unwrap();
        assert_ne!(id_part(&out), id_part(&changed));
    }

    #[test]
    fn original_first_id_is_kept() {
        let mut out = trailer("[(orig\\)inal ID)<FFEEDDCCBBAA99887766554433221100>]");
        fix_trailer_id(&mut out, b"x").unwrap();
        let id = id_part(&out);
        assert!(id.starts_with("/ID[(orig\\)inal ID)<"), "{id}");
        assert!(!id.contains("FFEEDDCC"));
        let mut out =
            trailer("[<0123456789ABCDEF0123456789ABCDEF> <FFEEDDCCBBAA99887766554433221100>]");
        fix_trailer_id(&mut out, b"x").unwrap();
        assert!(id_part(&out).starts_with("/ID[<0123456789ABCDEF0123456789ABCDEF> <"));
    }

    #[test]
    fn unexpected_trailers() {
        let mut no_id = trailer("[]").into_iter().collect::<Vec<_>>();
        assert!(fix_trailer_id(&mut no_id, b"").is_err());
        let mut missing = b"%PDF-1.7\ntrailer\n<</Root 1 0 R>>\n".to_vec();
        assert!(fix_trailer_id(&mut missing, b"").is_ok());
        let mut none = b"%PDF-1.7\n".to_vec();
        assert!(fix_trailer_id(&mut none, b"").is_err());
        let mut odd = trailer("[<ABC><ABC>]");
        assert!(fix_trailer_id(&mut odd, b"").is_err());
    }

    #[test]
    fn matrices_compose() {
        // Flip y, then translate: a point at glyph (1, 1) lands at (11, 19).
        let m = concat(
            [1.0, 0.0, 0.0, -1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 1.0, 10.0, 20.0],
        );
        let (x, y) = to_page(m, SheetPoint::new(1.0, 1.0));
        assert_eq!((x.value, y.value), (11.0, 19.0));
        // Quarter turn: (x, y) -> (y, x) after a y flip gives a rotation.
        let r = concat(
            [1.0, 0.0, 0.0, -1.0, 0.0, 0.0],
            [0.0, 1.0, 1.0, 0.0, 0.0, 0.0],
        );
        assert_eq!(r, [0.0, 1.0, -1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn dates_convert() {
        let d = PdfDate {
            year: 2026,
            month: 10,
            day: 9,
            hour: 8,
            minute: 30,
            second: 0,
        };
        assert_eq!(
            date_time(d).unwrap().to_rfc3339(),
            "2026-10-09T08:30:00+00:00"
        );
        assert!(
            date_time(PdfDate {
                day: 31,
                month: 2,
                ..d
            })
            .is_err()
        );
    }
}
