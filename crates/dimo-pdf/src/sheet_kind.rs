//! Sheet classification (FR-DOC-03, recognition stage 1).
//!
//! The kind decides the recognition path: text runs from the PDF, or render and OCR. It is
//! decided from two independent kinds of evidence (spec 08, stage 1: never trust a single
//! library's text extraction):
//!
//! 1. **The page content.** PDFium's page objects are the parsed content stream: one text object
//!    per text showing operator inside `BT`/`ET`, with its render mode (`Tr`) and font (`Tf`),
//!    plus path and image objects. Form `XObjects` are walked recursively. This says whether text
//!    exists, whether it is visible, which fonts it uses, and how much of the sheet is image.
//! 2. **The decoded characters.** The Unicode values PDFium extracts for the visible text. A
//!    character is unmapped when PDFium has no usable Unicode value for it: zero, U+FFFD,
//!    control characters, private use code points. This catches fonts whose codes PDFium can
//!    only pass through as raw character codes (see the `/ToUnicode` notes below).
//!
//! # Rule
//!
//! - `vector_text`: visible text objects exist, at least [`MIN_TEXT_CHARS`] of their
//!   characters are readable, and at most half of them are unmapped.
//! - `raster`: otherwise, if images cover at least [`RASTER_COVERAGE`] of the sheet.
//! - `vector_outlined`: everything else, including text that exists but cannot be decoded
//!   (it must be read by OCR like outlined text) and empty sheets.
//!
//! Mixed sheets (some text, some outlined callouts) are `vector_text`; the spec handles them per
//! region later. Thresholds are first guesses, to be calibrated on the corpus.
//!
//! # `/ToUnicode`
//!
//! PDFium does not expose font dictionaries, so the presence of a `/ToUnicode` `CMap` cannot be
//! read directly. What matters for the recognition path is whether the decoded values are usable,
//! which the unmapped share measures. Findings on how PDFium decodes Type0 fonts with
//! `Identity-H` and a missing or invalid `/ToUnicode` are in `corpus/notes/test_drawing_1.md`.

use dimo_core::sheet::SheetKind;
use pdfium_render::prelude::{
    PdfPage, PdfPageObject, PdfPageObjectCommon, PdfPageObjectsCommon, PdfPageTextChar,
};

use crate::PdfError;
use crate::geometry::SheetRect;
use crate::page_space::PageToSheet;
use crate::text::{page_to_sheet, strip_subset_tag, text_error};

/// Fewer readable characters than this are not enough for `vector_text` (page numbers, stamps).
pub const MIN_TEXT_CHARS: usize = 10;
/// Share of the sheet area covered by images from which a sheet without text is `raster`.
pub const RASTER_COVERAGE: f64 = 0.5;
/// Form `XObjects` nested deeper than this are not inspected (guards against cycles).
const MAX_FORM_DEPTH: usize = 32;

/// What a sheet contains and the kind derived from it.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetAnalysis {
    /// The derived kind (see the module docs for the rule).
    pub kind: SheetKind,
    /// Text objects in the content stream, including form `XObjects`.
    pub text_objects: usize,
    /// Text objects with an invisible render mode (OCR layers of scans).
    pub invisible_text_objects: usize,
    /// Fonts used by text objects, sorted by name.
    pub fonts: Vec<FontInfo>,
    /// Non whitespace characters PDFium decoded from visible text.
    pub chars: usize,
    /// Of [`Self::chars`], those without a usable Unicode value.
    pub unmapped_chars: usize,
    /// Path objects (lines, curves, outlined glyphs).
    pub path_objects: usize,
    /// Image objects.
    pub image_objects: usize,
    /// Share of the sheet area covered by images, 0 to 1 (overlaps counted once per image,
    /// capped at 1).
    pub image_coverage: f64,
}

/// A font used by text on the sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontInfo {
    /// Base font name without the subset tag.
    pub name: String,
    /// Whether the font program is embedded in the PDF.
    pub embedded: bool,
    /// Number of text objects using it.
    pub text_objects: usize,
}

/// Counts gathered from one sheet, before the decision.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct Evidence {
    pub visible_text_objects: usize,
    pub chars: usize,
    pub unmapped_chars: usize,
    pub image_coverage: f64,
}

/// The classification rule (see the module docs).
pub(crate) fn decide(e: &Evidence) -> SheetKind {
    let readable = e.chars.saturating_sub(e.unmapped_chars);
    if e.visible_text_objects > 0 && readable >= MIN_TEXT_CHARS && e.unmapped_chars * 2 <= e.chars {
        SheetKind::VectorText
    } else if e.image_coverage >= RASTER_COVERAGE {
        SheetKind::Raster
    } else {
        SheetKind::VectorOutlined
    }
}

/// Whether PDFium found no usable Unicode value for a decoded character.
pub(crate) fn is_unmapped(value: u32) -> bool {
    match char::from_u32(value) {
        None => true,
        Some(c) => {
            c == '\0'
                || c == char::REPLACEMENT_CHARACTER
                || (c.is_control() && !c.is_whitespace())
                || matches!(c, '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{10FFFF}')
        }
    }
}

/// Analyzes the loaded page of `sheet`. Runs on the render thread.
pub(crate) fn analyze_sheet(page: &PdfPage<'_>, sheet: usize) -> Result<SheetAnalysis, PdfError> {
    let sheet_rect = SheetRect::new(
        0.0,
        0.0,
        f64::from(page.width().value),
        f64::from(page.height().value),
    );
    let sheet_area = sheet_rect.width * sheet_rect.height;
    let mut walk = Walk::new(page_to_sheet(page)?, sheet_rect);
    for object in page.objects().iter() {
        walk.visit(&object, false, 0);
    }

    let text = page.text().map_err(text_error)?;
    let (mut chars, mut unmapped_chars) = (0, 0);
    for ch in text.chars().iter() {
        if counts_as_char(&ch) {
            chars += 1;
            if is_unmapped(ch.unicode_value()) {
                unmapped_chars += 1;
            }
        }
    }

    let image_coverage = if sheet_area > 0.0 {
        (walk.image_area / sheet_area).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let evidence = Evidence {
        visible_text_objects: walk.text_objects - walk.invisible_text_objects,
        chars,
        unmapped_chars,
        image_coverage,
    };
    let kind = decide(&evidence);
    if walk.text_objects > walk.invisible_text_objects && kind != SheetKind::VectorText {
        tracing::warn!(
            sheet,
            text_objects = walk.text_objects,
            chars,
            unmapped_chars,
            "sheet has visible text objects whose characters cannot be decoded"
        );
    }
    let mut fonts: Vec<FontInfo> = walk.fonts.into_values().collect();
    fonts.sort_by(|a, b| a.name.cmp(&b.name).then(a.embedded.cmp(&b.embedded)));
    Ok(SheetAnalysis {
        kind,
        text_objects: walk.text_objects,
        invisible_text_objects: walk.invisible_text_objects,
        fonts,
        chars,
        unmapped_chars,
        path_objects: walk.path_objects,
        image_objects: walk.image_objects,
        image_coverage,
    })
}

/// Visible, non generated, non whitespace characters count for classification.
fn counts_as_char(ch: &PdfPageTextChar<'_>) -> bool {
    !ch.is_generated().unwrap_or(false)
        && ch.text_object().is_ok_and(|object| object.is_visible())
        && !ch.unicode_char().is_some_and(char::is_whitespace)
}

struct Walk {
    to_sheet: PageToSheet,
    sheet: SheetRect,
    text_objects: usize,
    invisible_text_objects: usize,
    path_objects: usize,
    image_objects: usize,
    image_area: f64,
    fonts: std::collections::BTreeMap<(String, bool), FontInfo>,
}

impl Walk {
    fn new(to_sheet: PageToSheet, sheet: SheetRect) -> Self {
        Self {
            to_sheet,
            sheet,
            text_objects: 0,
            invisible_text_objects: 0,
            path_objects: 0,
            image_objects: 0,
            image_area: 0.0,
            fonts: std::collections::BTreeMap::new(),
        }
    }

    /// Counts one object. Bounds of objects inside a form `XObject` are in form space, so
    /// images inside a form count with the area of the outermost form, once per form.
    fn visit(&mut self, object: &PdfPageObject<'_>, in_form: bool, depth: usize) {
        match object {
            PdfPageObject::Text(text) => {
                self.text_objects += 1;
                if !text.is_visible() {
                    self.invisible_text_objects += 1;
                }
                let font = text.font();
                let name = strip_subset_tag(&font.name()).to_owned();
                let embedded = font.is_embedded().unwrap_or(false);
                self.fonts
                    .entry((name.clone(), embedded))
                    .or_insert(FontInfo {
                        name,
                        embedded,
                        text_objects: 0,
                    })
                    .text_objects += 1;
            }
            PdfPageObject::Path(_) => self.path_objects += 1,
            PdfPageObject::Image(_) => {
                self.image_objects += 1;
                if !in_form {
                    self.image_area += self.area_on_sheet(object);
                }
            }
            PdfPageObject::XObjectForm(form) if depth < MAX_FORM_DEPTH => {
                let images_before = self.image_objects;
                for child in form.iter() {
                    self.visit(&child, true, depth + 1);
                }
                if !in_form && self.image_objects > images_before {
                    self.image_area += self.area_on_sheet(object);
                }
            }
            _ => {}
        }
    }

    /// Area of an object's bounds on the sheet, clipped to the sheet.
    fn area_on_sheet(&self, object: &PdfPageObject<'_>) -> f64 {
        object.bounds().map_or(0.0, |quad| {
            let r = self.to_sheet.rect(
                f64::from(quad.left().value),
                f64::from(quad.bottom().value),
                f64::from(quad.right().value),
                f64::from(quad.top().value),
            );
            let s = self.sheet;
            let w = (r.x + r.width).min(s.x + s.width) - r.x.max(s.x);
            let h = (r.y + r.height).min(s.y + s.height) - r.y.max(s.y);
            w.max(0.0) * h.max(0.0)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(text_objects: usize, chars: usize, unmapped: usize, images: f64) -> Evidence {
        Evidence {
            visible_text_objects: text_objects,
            chars,
            unmapped_chars: unmapped,
            image_coverage: images,
        }
    }

    #[test]
    fn readable_text_is_vector_text() {
        assert_eq!(decide(&evidence(74, 400, 0, 0.0)), SheetKind::VectorText);
        // Text over a large image still has a usable text layer.
        assert_eq!(decide(&evidence(5, 40, 0, 0.9)), SheetKind::VectorText);
        // A few symbol font characters do not matter.
        assert_eq!(decide(&evidence(5, 40, 10, 0.0)), SheetKind::VectorText);
    }

    #[test]
    fn undecodable_text_is_not_vector_text() {
        assert_eq!(
            decide(&evidence(20, 100, 80, 0.0)),
            SheetKind::VectorOutlined
        );
        assert_eq!(decide(&evidence(20, 100, 80, 0.8)), SheetKind::Raster);
        // Text objects whose characters PDFium cannot extract at all.
        assert_eq!(decide(&evidence(20, 0, 0, 0.0)), SheetKind::VectorOutlined);
    }

    #[test]
    fn little_or_no_text_depends_on_images() {
        assert_eq!(decide(&evidence(0, 0, 0, 0.0)), SheetKind::VectorOutlined);
        assert_eq!(decide(&evidence(1, 6, 0, 0.0)), SheetKind::VectorOutlined);
        assert_eq!(decide(&evidence(1, 6, 0, 0.95)), SheetKind::Raster);
        assert_eq!(decide(&evidence(0, 0, 0, 0.5)), SheetKind::Raster);
        assert_eq!(decide(&evidence(0, 0, 0, 0.49)), SheetKind::VectorOutlined);
    }

    #[test]
    fn unmapped_values() {
        for v in [
            0, 0x1, 0x1F, 0x7F, 0xFFFD, 0xE000, 0xF8FF, 0xF0000, 0xD800, 0x11_0000,
        ] {
            assert!(is_unmapped(v), "{v:#x}");
        }
        for v in ['A', 'Ø', '\u{2212}', '°', '±', '⌀', ' ', '\n', '一'] {
            assert!(!is_unmapped(u32::from(v)), "{v:?}");
        }
    }
}
