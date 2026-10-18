//! Decoding of stored image drawings into 8 bit grey or RGB pages (R.19).
//!
//! JPEG and PNG are read with the `image` crate, TIFF with the `tiff` crate (every directory is
//! one page). Every page is turned the way its EXIF or TIFF orientation says, so a page is what
//! any image viewer shows; image edit corners refer to that page (R01 decision 10). Transparent
//! parts are put on white. 16 bit samples are reduced to 8 bit.

use std::io::Cursor;

use dimo_core::ImageFormat;
use image::codecs::jpeg::JpegDecoder;
use image::codecs::png::PngDecoder;
use image::metadata::Orientation;
use image::{DynamicImage, GrayImage, ImageDecoder, RgbImage};
use tiff::ColorType;
use tiff::decoder::{Decoder as TiffDecoder, DecodingResult, Limits as TiffLimits};
use tiff::tags::Tag;

use crate::error::PdfError;

/// Largest decode buffer of one page. A page above it is refused with a clear message instead
/// of running out of memory (an A0 scan at 600 dpi in colour is about 1.6 GiB).
pub const MAX_DECODE_BYTES: u64 = 1024 * 1024 * 1024;

/// Most pages read from one TIFF.
pub const MAX_PAGES: usize = 200;

/// One decoded page, turned upright.
#[derive(Debug)]
pub(crate) struct Page {
    /// The pixels, [`DynamicImage::ImageLuma8`] or [`DynamicImage::ImageRgb8`].
    pub image: DynamicImage,
    /// Resolution the file states, pixels per inch horizontally and vertically as the page
    /// reads (after the orientation). `None` if the file states none.
    pub dpi: Option<(f64, f64)>,
}

/// Decodes the pages of `bytes` one after the other and hands each to `page` with the index
/// of its TIFF directory (0 for JPEG and PNG). Each page is dropped before the next one is
/// decoded. TIFF thumbnails and other reduced resolution images (`NewSubfileType` bit 0) are
/// not pages and are skipped.
pub(crate) fn for_each_page(
    bytes: &[u8],
    format: ImageFormat,
    mut page: impl FnMut(usize, Page) -> Result<(), PdfError>,
) -> Result<(), PdfError> {
    match format {
        ImageFormat::Jpeg => page(0, decode_jpeg(bytes)?),
        ImageFormat::Png => page(0, decode_png(bytes)?),
        ImageFormat::Tiff => {
            let mut decoder = tiff_decoder(bytes)?;
            let mut pages = 0;
            for directory in 0.. {
                if !is_thumbnail(&mut decoder) {
                    if pages == MAX_PAGES {
                        return Err(PdfError::Image(format!(
                            "the TIFF has more than {MAX_PAGES} pages"
                        )));
                    }
                    page(directory, tiff_page(&mut decoder)?)?;
                    pages += 1;
                }
                if !decoder.more_images() {
                    break;
                }
                decoder.next_image().map_err(tiff_error)?;
            }
            if pages == 0 {
                return Err(PdfError::Image("the TIFF has no full size page".to_owned()));
            }
            Ok(())
        }
    }
}

/// Decodes the page in TIFF directory `directory` of `bytes` (0 for JPEG and PNG).
pub(crate) fn decode_page(
    bytes: &[u8],
    format: ImageFormat,
    directory: usize,
) -> Result<Page, PdfError> {
    match format {
        ImageFormat::Jpeg if directory == 0 => decode_jpeg(bytes),
        ImageFormat::Png if directory == 0 => decode_png(bytes),
        ImageFormat::Tiff => {
            let mut decoder = tiff_decoder(bytes)?;
            decoder.seek_to_image(directory).map_err(tiff_error)?;
            tiff_page(&mut decoder)
        }
        _ => Err(PdfError::Image(format!(
            "the image has no page {directory}"
        ))),
    }
}

/// Whether the current TIFF directory is a reduced resolution copy of another page.
fn is_thumbnail(decoder: &mut TiffDecoder<Cursor<&[u8]>>) -> bool {
    decoder
        .find_tag_unsigned::<u32>(Tag::NewSubfileType)
        .ok()
        .flatten()
        .is_some_and(|kind| kind & 1 == 1)
}

fn limits() -> image::Limits {
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    limits
}

fn decode_jpeg(bytes: &[u8]) -> Result<Page, PdfError> {
    let mut decoder = JpegDecoder::new(Cursor::new(bytes)).map_err(image_error)?;
    decoder.set_limits(limits()).map_err(image_error)?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let image = DynamicImage::from_decoder(decoder).map_err(image_error)?;
    Ok(upright(image, jfif_dpi(bytes), orientation))
}

fn decode_png(bytes: &[u8]) -> Result<Page, PdfError> {
    let mut decoder = PngDecoder::new(Cursor::new(bytes)).map_err(image_error)?;
    decoder.set_limits(limits()).map_err(image_error)?;
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let image = DynamicImage::from_decoder(decoder).map_err(image_error)?;
    Ok(upright(image, png_dpi(bytes), orientation))
}

/// Flattens to grey or RGB on white and applies the orientation (also to the resolution).
fn upright(image: DynamicImage, dpi: Option<(f64, f64)>, orientation: Orientation) -> Page {
    let mut image = flatten(image);
    image.apply_orientation(orientation);
    let swaps = matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    );
    let dpi = dpi.map(|(x, y)| if swaps { (y, x) } else { (x, y) });
    Page { image, dpi }
}

/// Grey stays grey, colour becomes RGB; alpha is composed onto white.
fn flatten(image: DynamicImage) -> DynamicImage {
    let grey = !image.color().has_color();
    if !image.color().has_alpha() {
        return if grey {
            DynamicImage::ImageLuma8(image.into_luma8())
        } else {
            DynamicImage::ImageRgb8(image.into_rgb8())
        };
    }
    let rgba = image.into_rgba8();
    let (width, height) = rgba.dimensions();
    let on_white = |c: u8, a: u8| {
        let (c, a) = (u16::from(c), u16::from(a));
        // Rounded c * a / 255 + (255 - a), exact in integers.
        u8::try_from((c * a + 255 * (255 - a) + 127) / 255).unwrap_or(u8::MAX)
    };
    if grey {
        let pixels = rgba.pixels().map(|p| on_white(p[0], p[3])).collect();
        DynamicImage::ImageLuma8(GrayImage::from_raw(width, height, pixels).unwrap_or_default())
    } else {
        let pixels = rgba
            .pixels()
            .flat_map(|p| [0, 1, 2].map(|i| on_white(p[i], p[3])))
            .collect();
        DynamicImage::ImageRgb8(RgbImage::from_raw(width, height, pixels).unwrap_or_default())
    }
}

fn tiff_decoder(bytes: &[u8]) -> Result<TiffDecoder<Cursor<&[u8]>>, PdfError> {
    let mut limits = TiffLimits::default();
    let max = usize::try_from(MAX_DECODE_BYTES).unwrap_or(usize::MAX);
    limits.decoding_buffer_size = max;
    limits.intermediate_buffer_size = max;
    Ok(TiffDecoder::new(Cursor::new(bytes))
        .map_err(tiff_error)?
        .with_limits(limits))
}

/// Reads the current TIFF directory as one page.
fn tiff_page(decoder: &mut TiffDecoder<Cursor<&[u8]>>) -> Result<Page, PdfError> {
    let (width, height) = decoder.dimensions().map_err(tiff_error)?;
    let color = decoder.colortype().map_err(tiff_error)?;
    let orientation = decoder
        .find_tag_unsigned::<u8>(Tag::Orientation)
        .ok()
        .flatten()
        .and_then(Orientation::from_exif)
        .unwrap_or(Orientation::NoTransforms);
    let dpi = tiff_dpi(decoder);
    let data = decoder.read_image().map_err(tiff_error)?;
    let image = tiff_pixels(width, height, color, data)?;
    Ok(upright(image, dpi, orientation))
}

/// Converts decoded TIFF samples to grey or RGB (alpha still present, removed by `flatten`).
fn tiff_pixels(
    width: u32,
    height: u32,
    color: ColorType,
    data: DecodingResult,
) -> Result<DynamicImage, PdfError> {
    let unsupported = || {
        PdfError::Image(format!(
            "TIFF colour type {color:?} is not supported; save the scan as grey or RGB"
        ))
    };
    let samples: Vec<u8> = match data {
        DecodingResult::U8(v) => v,
        // Upper byte: 16 bit samples reduced to 8 bit.
        DecodingResult::U16(v) => v.into_iter().map(|s| s.to_be_bytes()[0]).collect(),
        _ => return Err(unsupported()),
    };
    let build = |image: Option<DynamicImage>| image.ok_or_else(short_data);
    match color {
        ColorType::Gray(8 | 16) => {
            build(GrayImage::from_raw(width, height, samples).map(DynamicImage::ImageLuma8))
        }
        ColorType::Gray(bits @ (1 | 2 | 4)) => build(
            GrayImage::from_raw(width, height, unpack(&samples, width, height, bits)?)
                .map(DynamicImage::ImageLuma8),
        ),
        ColorType::RGB(8 | 16) => {
            build(RgbImage::from_raw(width, height, samples).map(DynamicImage::ImageRgb8))
        }
        ColorType::RGBA(8 | 16) => {
            build(image::RgbaImage::from_raw(width, height, samples).map(DynamicImage::ImageRgba8))
        }
        ColorType::GrayA(8 | 16)
        | ColorType::Multiband {
            bit_depth: 8 | 16,
            num_samples: 2,
        } => build(
            image::GrayAlphaImage::from_raw(width, height, samples).map(DynamicImage::ImageLumaA8),
        ),
        _ => Err(unsupported()),
    }
    .map(flatten)
}

/// Unpacks rows of 1, 2 or 4 bit grey samples (each row starts on a byte) to 8 bit, 0 black.
fn unpack(packed: &[u8], width: u32, height: u32, bits: u8) -> Result<Vec<u8>, PdfError> {
    let width = width as usize;
    let per_byte = 8 / usize::from(bits);
    let row_bytes = width.div_ceil(per_byte);
    let max = (1u16 << bits) - 1;
    let mut out = Vec::with_capacity(width * height as usize);
    for row in packed.chunks(row_bytes).take(height as usize) {
        if row.len() < row_bytes {
            return Err(short_data());
        }
        for x in 0..width {
            let byte = row[x / per_byte];
            let shift = 8 - usize::from(bits) * (x % per_byte + 1);
            let value = (u16::from(byte) >> shift) & max;
            out.push(u8::try_from(value * 255 / max).unwrap_or(u8::MAX));
        }
    }
    if out.len() != width * height as usize {
        return Err(short_data());
    }
    Ok(out)
}

fn short_data() -> PdfError {
    PdfError::Image("the image data is shorter than its size says".to_owned())
}

/// Resolution from the TIFF tags (unit inch by default, or centimetre).
fn tiff_dpi(decoder: &mut TiffDecoder<Cursor<&[u8]>>) -> Option<(f64, f64)> {
    let mut rational = |tag| match decoder.find_tag(tag).ok().flatten()? {
        tiff::decoder::ifd::Value::Rational(n, d) if d > 0 => Some(f64::from(n) / f64::from(d)),
        _ => None,
    };
    let (x, y) = (rational(Tag::XResolution)?, rational(Tag::YResolution)?);
    let unit = decoder
        .find_tag_unsigned::<u16>(Tag::ResolutionUnit)
        .ok()
        .flatten()
        .unwrap_or(2);
    match unit {
        2 => Some((x, y)),
        3 => Some((x * 2.54, y * 2.54)),
        _ => None,
    }
}

/// Resolution from the JFIF header of a JPEG (density unit 1 inch, 2 centimetre).
fn jfif_dpi(bytes: &[u8]) -> Option<(f64, f64)> {
    // SOI, then APP0: FF E0, length, "JFIF\0", version (2), unit (1), x (2), y (2).
    let app0 = bytes.get(2..20)?;
    if app0[..2] != [0xFF, 0xE0] || &app0[4..9] != b"JFIF\0" {
        return None;
    }
    let x = f64::from(u16::from_be_bytes([app0[12], app0[13]]));
    let y = f64::from(u16::from_be_bytes([app0[14], app0[15]]));
    match app0[11] {
        1 => Some((x, y)),
        2 => Some((x * 2.54, y * 2.54)),
        _ => None,
    }
}

/// Resolution from the `pHYs` chunk of a PNG (unit metre).
fn png_dpi(bytes: &[u8]) -> Option<(f64, f64)> {
    let mut at = 8;
    while let Some(header) = bytes.get(at..at + 8) {
        let len = u32::from_be_bytes([header[0], header[1], header[2], header[3]]) as usize;
        let kind = &header[4..8];
        if kind == b"pHYs" {
            let data = bytes.get(at + 8..at + 17)?;
            let x = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
            let y = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
            return (data[8] == 1).then(|| (f64::from(x) * 0.0254, f64::from(y) * 0.0254));
        }
        if kind == b"IDAT" || kind == b"IEND" {
            return None;
        }
        at = at.checked_add(12)?.checked_add(len)?;
    }
    None
}

fn image_error(error: image::ImageError) -> PdfError {
    match error {
        image::ImageError::Limits(_) => too_large(),
        other => PdfError::Image(other.to_string()),
    }
}

fn tiff_error(error: tiff::TiffError) -> PdfError {
    match error {
        tiff::TiffError::LimitsExceeded => too_large(),
        other => PdfError::Image(other.to_string()),
    }
}

fn too_large() -> PdfError {
    PdfError::Image(format!(
        "a page needs more than {} MiB to decode; scan it at a lower resolution",
        MAX_DECODE_BYTES / (1024 * 1024)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpacks_one_bit_rows_that_start_on_a_byte() {
        // Width 10: two bytes per row. Row 0: 1000 0000 01..; row 1 all white.
        let packed = [0b1000_0000, 0b0100_0000, 0xFF, 0xFF];
        let out = unpack(&packed, 10, 2, 1).unwrap();
        assert_eq!(&out[..10], &[255, 0, 0, 0, 0, 0, 0, 0, 0, 255]);
        assert!(out[10..].iter().all(|&v| v == 255));
        assert!(unpack(&packed[..3], 10, 2, 1).is_err());
    }

    #[test]
    fn four_bit_grey_scales_to_full_range() {
        assert_eq!(unpack(&[0x0F], 2, 1, 4).unwrap(), vec![0, 255]);
    }

    #[test]
    fn alpha_is_composed_onto_white() {
        let rgba = image::RgbaImage::from_raw(2, 1, vec![0, 0, 0, 0, 0, 0, 0, 255]).unwrap();
        let flat = flatten(DynamicImage::ImageRgba8(rgba));
        assert_eq!(flat.as_bytes(), &[255, 255, 255, 0, 0, 0]);
    }

    #[test]
    fn jfif_density_in_inch_and_centimetre() {
        let mut jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 0, 16];
        jpeg.extend_from_slice(b"JFIF\0");
        jpeg.extend_from_slice(&[1, 2, 1, 1, 44, 1, 44, 0, 0]);
        assert_eq!(jfif_dpi(&jpeg), Some((300.0, 300.0)));
        jpeg[13] = 2;
        let (x, _) = jfif_dpi(&jpeg).unwrap();
        assert!((x - 762.0).abs() < 1e-9);
        jpeg[13] = 0;
        assert_eq!(jfif_dpi(&jpeg), None);
    }
}
