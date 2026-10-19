//! An image drawing as a plain PDF (R.21): one page per sheet, the edited sheet as one image.
//!
//! The ballooned PDF of a photo or scan is this PDF with the balloons added by the same writer
//! as for PDF drawings ([`crate::PdfEngine::write_ballooned`]), so balloons, annotations and
//! the file ID follow one path. Each page is the sheet in sheet space (rule 4): its media box
//! is the sheet size, origin at the bottom left as PDF requires, no `/Rotate`.
//!
//! The pixels are the ones the viewport shows (corrections, trim, paper and clean up), drawn
//! at the kept resolution of the page ([`RasterSource::raster_dpi`], at most 300 dpi). Grey
//! pages are written as 8 bit grey with Flate compression, colour pages as JPEG. The file has
//! no dates and no producer, and its `/ID` is the SHA-256 of its body, so the same project
//! gives the same bytes (FR-EXP-11).

use std::fmt::Write as _;

use dimo_core::{Rect, Sha256Hex};
use image::ImageEncoder;
use image::codecs::jpeg::JpegEncoder;

use super::ImageSource;
use super::geometry::to_u32;
use crate::error::PdfError;
use crate::source::RasterSource;

/// JPEG quality of colour pages: no visible loss on drawing lines and text.
const JPEG_QUALITY: u8 = 90;

/// Flate level of grey pages; fixed so the output never changes with a library default.
const FLATE_LEVEL: u8 = 6;

/// Resolution used if a page reports none.
const FALLBACK_DPI: u32 = 150;

/// Rows drawn per render call, so a large sheet is never drawn as one RGBA image.
const STRIP_ROWS: u32 = 512;

/// Columns drawn per render call, well below [`crate::MAX_RENDER_SIDE`].
const BLOCK_COLUMNS: u32 = 4096;

/// One page image ready for the PDF.
struct PageImage {
    width: u32,
    height: u32,
    grey: bool,
    /// `DCTDecode` (JPEG) or `FlateDecode` data.
    data: Vec<u8>,
    jpeg: bool,
}

impl ImageSource {
    /// The edited sheets as a PDF, one page per sheet. See the module docs.
    pub fn to_pdf(&self) -> Result<Vec<u8>, PdfError> {
        let mut pages = Vec::with_capacity(self.sheet_count());
        for sheet in 0..self.sheet_count() {
            let size = self.sheet_size(sheet)?;
            let image = self.page_image(sheet)?;
            pages.push((size, image));
        }
        Ok(write_pdf(&pages))
    }

    /// The pixels of one sheet at its kept resolution, compressed.
    fn page_image(&self, sheet: usize) -> Result<PageImage, PdfError> {
        let size = self.sheet_size(sheet)?;
        let dpi = self
            .raster_dpi(sheet)
            .filter(|&d| d > 0)
            .unwrap_or(FALLBACK_DPI);
        let zoom = f64::from(dpi) / 72.0;
        let width = to_u32((size.width * zoom).round()).max(1);
        let height = to_u32((size.height * zoom).round()).max(1);
        // Grey kept pixels (a grey scan or clean up) render with r = g = b.
        let grey = self.reduced(sheet)?.levels[0].channels == 1;
        let channels = if grey { 1 } else { 3 };
        let row_bytes = width as usize * channels;
        let mut pixels = vec![0u8; row_bytes * height as usize];
        // Blocks of at most STRIP_ROWS x BLOCK_COLUMNS pixels: no render call ever reaches the
        // render size limit, also for sheets wider than an A0 at 300 dpi.
        let mut row = 0;
        while row < height {
            let rows = STRIP_ROWS.min(height - row);
            let mut column = 0;
            while column < width {
                let columns = BLOCK_COLUMNS.min(width - column);
                let region = Rect::new(
                    f64::from(column) / zoom,
                    f64::from(row) / zoom,
                    f64::from(columns) / zoom,
                    f64::from(rows) / zoom,
                );
                let block = self.render_region(sheet, region, zoom)?;
                if block.width() != columns || block.height() != rows {
                    return Err(PdfError::Render(format!(
                        "image block is {} x {} pixels, expected {columns} x {rows}",
                        block.width(),
                        block.height()
                    )));
                }
                for (y, line) in block.as_bytes().chunks(columns as usize * 4).enumerate() {
                    let start = (row as usize + y) * row_bytes + column as usize * channels;
                    let out = &mut pixels[start..start + columns as usize * channels];
                    for (target, pixel) in out.chunks_mut(channels).zip(line.as_chunks::<4>().0) {
                        target.copy_from_slice(&pixel[..channels]);
                    }
                }
                column += columns;
            }
            row += rows;
        }
        let (data, jpeg) = if grey {
            let data = miniz_oxide::deflate::compress_to_vec_zlib(&pixels, FLATE_LEVEL);
            (data, false)
        } else {
            let mut data = Vec::new();
            JpegEncoder::new_with_quality(&mut data, JPEG_QUALITY)
                .write_image(&pixels, width, height, image::ExtendedColorType::Rgb8)
                .map_err(|e| PdfError::Write(format!("cannot encode the sheet image: {e}")))?;
            (data, true)
        };
        Ok(PageImage {
            width,
            height,
            grey,
            data,
            jpeg,
        })
    }
}

/// A number for a PDF dictionary or content stream: fixed decimals, no exponent.
fn num(value: f64) -> String {
    let text = format!("{value:.4}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

/// Writes catalog, page tree, and per page the page, its content stream and its image.
fn write_pdf(pages: &[(dimo_core::Size, PageImage)]) -> Vec<u8> {
    // Object numbers: 1 catalog, 2 page tree, then page, content, image per sheet.
    let page_obj = |i: usize| 3 + i * 3;
    let mut objects: Vec<Vec<u8>> = Vec::new();
    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    let kids = (0..pages.len())
        .map(|i| format!("{} 0 R", page_obj(i)))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", pages.len()).into_bytes());
    for (i, (size, image)) in pages.iter().enumerate() {
        let (w, h) = (num(size.width), num(size.height));
        let page = page_obj(i);
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {w} {h}] \
                 /Resources << /XObject << /Im0 {} 0 R >> >> /Contents {} 0 R >>",
                page + 2,
                page + 1
            )
            .into_bytes(),
        );
        let content = format!("q {w} 0 0 {h} 0 0 cm /Im0 Do Q");
        objects.push(stream(
            &format!("<< /Length {} >>", content.len()),
            content.as_bytes(),
        ));
        let dict = format!(
            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {} \
             /BitsPerComponent 8 /Filter {} /Length {} >>",
            image.width,
            image.height,
            if image.grey {
                "/DeviceGray"
            } else {
                "/DeviceRGB"
            },
            if image.jpeg {
                "/DCTDecode"
            } else {
                "/FlateDecode"
            },
            image.data.len()
        );
        objects.push(stream(&dict, &image.data));
    }

    let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let id = Sha256Hex::of(&pdf).to_hex();
    let xref = pdf.len();
    let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {} /Root 1 0 R /ID [<{id}> <{id}>] >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    pdf.extend_from_slice(table.as_bytes());
    pdf
}

fn stream(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = dict.as_bytes().to_vec();
    out.extend_from_slice(b"\nstream\n");
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

#[cfg(test)]
mod tests {
    use super::num;

    #[test]
    fn numbers_have_no_exponent_and_no_trailing_zeros() {
        assert_eq!(num(841.889_763_779_527_7), "841.8898");
        assert_eq!(num(595.0), "595");
        assert_eq!(num(-0.000_01), "0");
        assert_eq!(num(1.5), "1.5");
    }
}
