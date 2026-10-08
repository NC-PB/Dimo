//! Tile encoding: PNG through the pure Rust `png` crate.
//!
//! Why PNG with `Compression::Fast`:
//!
//! - Lossless. Thin lines and small text of a drawing must not get compression artefacts.
//! - Drawings are mostly white with thin dark lines, which deflate compresses well. `Fast`
//!   uses `fdeflate`, a deflate tuned for PNG filters.
//! - Every webview decodes PNG natively; no extra decoder or format negotiation, and `png` is
//!   pure Rust (MIT or Apache 2.0), already in the dependency tree through Tauri.
//!
//! Measured on `test_drawing_1.pdf` (Apple M series, encoder built with optimizations, per
//! 512 px tile at zoom 0 and 2): greyscale `Fast` 0.7 to 0.9 ms and 7.2 / 5.0 KB, greyscale
//! `Balanced` 1.5 to 1.8 ms and 5.4 / 3.4 KB, RGB `Fast` 1.7 to 2.1 ms and 17 / 11 KB. PDFium
//! needs 0.6 to 0.7 ms for the same tile, so `Fast` keeps encoding about as fast as rendering
//! while tiles stay small enough for a large memory cache. Lossless WebP was not measured: it
//! would add a dependency, and size is not the bottleneck for a local protocol.
//!
//! Tiles are rendered on a white background without transparency, so the alpha channel is
//! dropped. Tiles whose pixels are all grey (`r == g == b`, the usual case for drawings) are
//! written as 8 bit greyscale, which is two to three times smaller and faster than RGB.

use crate::raster::RgbaImage;

use super::TileError;

/// MIME type of encoded tiles.
pub const TILE_CONTENT_TYPE: &str = "image/png";

/// The eight byte PNG signature, used to check disk cache files.
pub(crate) const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Encodes the part of an opaque rendered image that starts at pixel `(left, top)` and is
/// `width` x `height` pixels as PNG.
pub(crate) fn encode_png(
    image: &RgbaImage,
    left: u32,
    top: u32,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, TileError> {
    if left + width > image.width() || top + height > image.height() || width == 0 || height == 0 {
        return Err(TileError::Render(format!(
            "crop {width} x {height} at ({left}, {top}) outside {image:?}"
        )));
    }
    let stride = image.width() as usize * 4;
    let rows = image
        .as_bytes()
        .chunks_exact(stride)
        .skip(top as usize)
        .take(height as usize)
        .map(|row| &row[left as usize * 4..(left + width) as usize * 4]);
    let grey = rows
        .clone()
        .flat_map(|row| row.as_chunks::<4>().0)
        .all(|[r, g, b, _]| r == g && g == b);
    let pixels = rows.flat_map(|row| row.as_chunks::<4>().0);
    let (color, data): (png::ColorType, Vec<u8>) = if grey {
        (
            png::ColorType::Grayscale,
            pixels.map(|[v, ..]| *v).collect(),
        )
    } else {
        (
            png::ColorType::Rgb,
            pixels.flat_map(|&[r, g, b, _]| [r, g, b]).collect(),
        )
    };
    let mut out = Vec::with_capacity(data.len() / 8);
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(color);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder.write_header().map_err(encode_error)?;
    writer.write_image_data(&data).map_err(encode_error)?;
    writer.finish().map_err(encode_error)?;
    Ok(out)
}

#[allow(clippy::needless_pass_by_value)] // used as a map_err callback
fn encode_error(e: png::EncodingError) -> TileError {
    TileError::Render(format!("PNG encoding failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(bytes: &[u8]) -> (png::OutputInfo, Vec<u8>) {
        let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        buf.truncate(info.buffer_size());
        (info, buf)
    }

    #[test]
    fn grey_image_becomes_greyscale_png() {
        let mut px = vec![255u8; 3 * 2 * 4];
        px[4..8].copy_from_slice(&[10, 10, 10, 255]);
        let img = RgbaImage::from_raw(3, 2, px).unwrap();
        let png = encode_png(&img, 0, 0, 3, 2).unwrap();
        assert_eq!(png[..8], PNG_SIGNATURE);
        let (info, data) = decode(&png);
        assert_eq!(info.color_type, png::ColorType::Grayscale);
        assert_eq!((info.width, info.height), (3, 2));
        assert_eq!(data, vec![255, 10, 255, 255, 255, 255]);
    }

    #[test]
    fn colour_image_keeps_rgb() {
        let px = vec![255, 0, 0, 255, 0, 0, 255, 255];
        let img = RgbaImage::from_raw(2, 1, px).unwrap();
        let (info, data) = decode(&encode_png(&img, 0, 0, 2, 1).unwrap());
        assert_eq!(info.color_type, png::ColorType::Rgb);
        assert_eq!(data, vec![255, 0, 0, 0, 0, 255]);
    }

    #[test]
    fn crops_before_encoding() {
        // 3 x 3 grey ramp, values 0..9; a red pixel outside the crop does not make it colour.
        let mut px: Vec<u8> = (0..9u8).flat_map(|v| [v, v, v, 255]).collect();
        px[0..4].copy_from_slice(&[255, 0, 0, 255]);
        let img = RgbaImage::from_raw(3, 3, px).unwrap();
        let (info, data) = decode(&encode_png(&img, 1, 1, 2, 2).unwrap());
        assert_eq!(info.color_type, png::ColorType::Grayscale);
        assert_eq!((info.width, info.height), (2, 2));
        assert_eq!(data, vec![4, 5, 7, 8]);
        assert!(encode_png(&img, 2, 0, 2, 1).is_err());
        assert!(encode_png(&img, 0, 0, 0, 1).is_err());
    }
}
