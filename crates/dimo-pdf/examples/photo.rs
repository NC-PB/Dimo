//! Writes the first sheet of a PDF as a JPEG turned a quarter to the left, as a phone photo
//! held the wrong way would be, for the image step of the smoke test (R.22). The picture is
//! made from the PDF each time, so no binary test file is needed.
//!
//! ```text
//! cargo run -p dimo-pdf --example photo -- <file.pdf> <out.jpg>
//! ```

#![allow(clippy::print_stderr, reason = "command line tool for developers")]

use std::process::ExitCode;

use dimo_core::Rect;
use dimo_pdf::PdfEngine;
use image::codecs::jpeg::JpegEncoder;

/// Pixels per sheet unit: 144 dpi, about what a phone photo of an A3 sheet gives.
const ZOOM: f64 = 2.0;
const QUALITY: u8 = 85;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [pdf, out] = args.as_slice() else {
        eprintln!("usage: photo <file.pdf> <out.jpg>");
        return ExitCode::FAILURE;
    };
    let result = (|| {
        let bytes = std::fs::read(pdf).map_err(|e| e.to_string())?;
        let engine = PdfEngine::start().map_err(|e| e.to_string())?;
        let doc = engine.open(bytes).map_err(|e| e.to_string())?;
        let size = doc.sheet_size(0).map_err(|e| e.to_string())?;
        let region = Rect::new(0.0, 0.0, size.width, size.height);
        let sheet = doc
            .render_region(0, region, ZOOM)
            .map_err(|e| e.to_string())?;
        let (width, height) = (sheet.width(), sheet.height());
        let rgba = image::RgbaImage::from_raw(width, height, sheet.into_bytes())
            .ok_or("render size does not match its pixels")?;
        let turned = image::imageops::rotate270(&image::DynamicImage::ImageRgba8(rgba).to_rgb8());
        let file = std::fs::File::create(out).map_err(|e| e.to_string())?;
        JpegEncoder::new_with_quality(std::io::BufWriter::new(file), QUALITY)
            .encode_image(&turned)
            .map_err(|e| e.to_string())
    })();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
