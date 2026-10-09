//! Stress document for the performance harness (T0.9, NFR-PERF-01 to 03).
//!
//! Writes one PDF with many A0 sheets of dense line work and text, by hand like the dimensioned
//! drawing in the parent module. The content is meaningless for recognition: it only has to cost
//! PDFium what a busy real drawing costs (tens of thousands of path segments, some thousand
//! short text objects per sheet, curves, filled arrow heads, rotated text) and to be big enough
//! that reading, hashing and parsing the file shows up in the timings.
//!
//! Determinism (AGENTS.md rule 11): all choices come from the seeded generator of the parent
//! module, the content streams are compressed with a fixed `miniz_oxide` version and level, and
//! the PDF has no Info dictionary, no document ID and no dates. Same seed, sheet count and tool
//! version give byte identical files.
//!
//! Per sheet (see the `*_PER_SHEET` constants): frame and title block, free line segments in
//! chains of eight, circles drawn as four Bezier curves, dimension lines with filled arrow heads
//! and a value, and free text in several sizes, a quarter of it rotated.

// Coordinates are positive and far below 2^23, the word table has 28 entries.
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::fmt::Write as _;

use miniz_oxide::deflate::compress_to_vec_zlib;

use super::{Rng, TO_UNICODE, r2};

/// Sheet width in PDF user units (A0 landscape, 1189 mm).
pub const STRESS_SHEET_WIDTH: f64 = 3370.0;
/// Sheet height in PDF user units (A0 landscape, 841 mm).
pub const STRESS_SHEET_HEIGHT: f64 = 2384.0;

/// Free line segments per sheet, drawn in chains of [`CHAIN`].
const LINES_PER_SHEET: u32 = 40_000;
/// Segments per chain.
const CHAIN: u32 = 8;
/// Circles per sheet, four curves each.
const CIRCLES_PER_SHEET: u32 = 3_600;
/// Dimension lines with arrow heads and a value per sheet.
const DIMENSIONS_PER_SHEET: u32 = 1_500;
/// Free text objects per sheet.
const TEXTS_PER_SHEET: u32 = 5_400;
/// Compression level of the content streams (zlib, 1 to 10).
const COMPRESSION_LEVEL: u8 = 6;

const MARGIN: f64 = 60.0;
const TEXT_WORDS: &[&str] = &[
    "SECTION", "A-A", "DETAIL", "SCALE", "1:2", "NOTE", "DEBURR", "ALL", "EDGES", "MATERIAL",
    "S235JR", "RA", "3.2", "THREAD", "M12", "DEPTH", "15", "TYP", "BOTH", "SIDES", "REF", "SEE",
    "ITEM", "42", "H7", "g6", "45", "CHAMFER",
];

/// Writes a PDF with `sheets` A0 sheets of dense content from `seed`.
pub fn generate_stress(seed: u64, sheets: usize) -> Vec<u8> {
    let mut rng = Rng(seed ^ 0x5EED_57E5_5000_0001);
    let streams: Vec<Vec<u8>> = (0..sheets)
        .map(|_| compress_to_vec_zlib(sheet_content(&mut rng).as_bytes(), COMPRESSION_LEVEL))
        .collect();
    write_pdf(&streams)
}

/// PDF y coordinate (origin bottom left) of a sheet space y.
fn pdf_y(y: f64) -> f64 {
    STRESS_SHEET_HEIGHT - y
}

/// Uniform value in `lo..=hi` with one decimal.
fn coord(rng: &mut Rng, lo: f64, hi: f64) -> f64 {
    let steps = ((hi - lo) * 10.0).max(1.0) as u32;
    r2(lo + f64::from(rng.range(0, steps)) / 10.0)
}

fn word(rng: &mut Rng) -> &'static str {
    TEXT_WORDS[rng.range(0, TEXT_WORDS.len() as u32 - 1) as usize]
}

fn sheet_content(rng: &mut Rng) -> String {
    let mut c = String::with_capacity(2 << 20);
    frame_and_title_block(&mut c, rng);
    for _ in 0..LINES_PER_SHEET / CHAIN {
        line_chain(&mut c, rng);
    }
    for _ in 0..CIRCLES_PER_SHEET {
        circle(&mut c, rng);
    }
    for _ in 0..DIMENSIONS_PER_SHEET {
        dimension(&mut c, rng);
    }
    for _ in 0..TEXTS_PER_SHEET {
        free_text(&mut c, rng);
    }
    c
}

fn frame_and_title_block(c: &mut String, rng: &mut Rng) {
    let (w, h) = (STRESS_SHEET_WIDTH, STRESS_SHEET_HEIGHT);
    let _ = writeln!(
        c,
        "2 w {MARGIN} {MARGIN} {:.2} {:.2} re S",
        w - 2.0 * MARGIN,
        h - 2.0 * MARGIN
    );
    // Title block, bottom right: 600 x 240 with a grid and text cells.
    let (x0, y0, bw, bh) = (w - MARGIN - 600.0, MARGIN, 600.0, 240.0);
    let _ = writeln!(c, "1 w {x0:.2} {y0:.2} {bw:.2} {bh:.2} re S");
    for i in 1..8 {
        let y = y0 + f64::from(i) * bh / 8.0;
        let _ = writeln!(c, "0.5 w {x0:.2} {y:.2} m {:.2} {y:.2} l S", x0 + bw);
    }
    for i in 1..5 {
        let x = x0 + f64::from(i) * bw / 5.0;
        let _ = writeln!(c, "0.5 w {x:.2} {y0:.2} m {x:.2} {:.2} l S", y0 + bh);
    }
    for row in 0..8 {
        for col in 0..5 {
            let _ = writeln!(
                c,
                "BT /F1 10 Tf {:.2} {:.2} Td <{}> Tj ET",
                x0 + f64::from(col) * bw / 5.0 + 4.0,
                y0 + f64::from(row) * bh / 8.0 + 8.0,
                hex(word(rng))
            );
        }
    }
}

/// A chain of short segments with a random width, like hatching or an outline.
fn line_chain(c: &mut String, rng: &mut Rng) {
    let mut x = coord(rng, MARGIN + 10.0, STRESS_SHEET_WIDTH - MARGIN - 130.0);
    let mut y = pdf_y(coord(
        rng,
        MARGIN + 10.0,
        STRESS_SHEET_HEIGHT - MARGIN - 130.0,
    ));
    let width = [0.18, 0.25, 0.35, 0.5, 0.7][rng.range(0, 4) as usize];
    let _ = write!(c, "{width} w {x:.2} {y:.2} m ");
    for _ in 0..CHAIN {
        x = r2(x + f64::from(rng.range(0, 120)) - 20.0);
        y = r2(y + f64::from(rng.range(0, 120)) - 60.0);
        let _ = write!(c, "{x:.2} {y:.2} l ");
    }
    c.push_str("S\n");
}

fn circle(c: &mut String, rng: &mut Rng) {
    // Four Bezier curves; K is the usual control point factor of a quarter circle.
    const K: f64 = 0.552_284_75;
    let r = f64::from(rng.range(2, 45));
    let cx = coord(rng, MARGIN + r + 5.0, STRESS_SHEET_WIDTH - MARGIN - r - 5.0);
    let cy = pdf_y(coord(
        rng,
        MARGIN + r + 5.0,
        STRESS_SHEET_HEIGHT - MARGIN - r - 5.0,
    ));
    let k = r2(r * K);
    let curves = [
        [(cx + r, cy + k), (cx + k, cy + r), (cx, cy + r)],
        [(cx - k, cy + r), (cx - r, cy + k), (cx - r, cy)],
        [(cx - r, cy - k), (cx - k, cy - r), (cx, cy - r)],
        [(cx + k, cy - r), (cx + r, cy - k), (cx + r, cy)],
    ];
    let _ = write!(c, "0.35 w {:.2} {cy:.2} m ", cx + r);
    for curve in curves {
        for (x, y) in curve {
            let _ = write!(c, "{x:.2} {y:.2} ");
        }
        c.push_str("c ");
    }
    c.push_str("S\n");
}

fn dimension(c: &mut String, rng: &mut Rng) {
    let len = f64::from(rng.range(40, 400));
    let x0 = coord(rng, MARGIN + 20.0, STRESS_SHEET_WIDTH - MARGIN - len - 20.0);
    let y = pdf_y(coord(
        rng,
        MARGIN + 20.0,
        STRESS_SHEET_HEIGHT - MARGIN - 40.0,
    ));
    let x1 = r2(x0 + len);
    let _ = writeln!(
        c,
        "0.25 w {x0:.2} {y:.2} m {x1:.2} {y:.2} l S {x0:.2} {:.2} m {x0:.2} {:.2} l S \
         {x1:.2} {:.2} m {x1:.2} {:.2} l S",
        y - 10.0,
        y + 10.0,
        y - 10.0,
        y + 10.0
    );
    // Filled arrow heads.
    let _ = writeln!(
        c,
        "{x0:.2} {y:.2} m {:.2} {:.2} l {:.2} {:.2} l f {x1:.2} {y:.2} m {:.2} {:.2} l {:.2} {:.2} l f",
        x0 + 8.0,
        y + 1.5,
        x0 + 8.0,
        y - 1.5,
        x1 - 8.0,
        y + 1.5,
        x1 - 8.0,
        y - 1.5
    );
    let value = format!(
        "{}.{:02}",
        rng.range(1, 999),
        [0, 5, 10, 25, 50][rng.range(0, 4) as usize]
    );
    let _ = writeln!(
        c,
        "BT /F1 12 Tf {:.2} {:.2} Td <{}> Tj ET",
        r2(f64::midpoint(x0, x1) - 14.0),
        y + 3.0,
        hex(&value)
    );
}

fn free_text(c: &mut String, rng: &mut Rng) {
    let size = [6.0, 8.0, 10.0, 12.0, 14.0][rng.range(0, 4) as usize];
    let words = rng.range(1, 4);
    let mut text = String::new();
    for i in 0..words {
        if i > 0 {
            text.push(' ');
        }
        text.push_str(word(rng));
    }
    let x = coord(rng, MARGIN + 20.0, STRESS_SHEET_WIDTH - MARGIN - 150.0);
    let y = pdf_y(coord(
        rng,
        MARGIN + 20.0,
        STRESS_SHEET_HEIGHT - MARGIN - 150.0,
    ));
    if rng.range(0, 3) == 0 {
        // Rotated by 90 degrees counter clockwise.
        let _ = writeln!(
            c,
            "BT /F1 {size} Tf 0 1 -1 0 {x:.2} {y:.2} Tm <{}> Tj ET",
            hex(&text)
        );
    } else {
        let _ = writeln!(
            c,
            "BT /F1 {size} Tf {x:.2} {y:.2} Td <{}> Tj ET",
            hex(&text)
        );
    }
}

fn hex(text: &str) -> String {
    text.bytes().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02X}");
        s
    })
}

/// Objects: 1 catalog, 2 page tree, 3 font, 4 `ToUnicode`, then a page and a content stream per
/// sheet. No Info, no ID, no dates.
fn write_pdf(streams: &[Vec<u8>]) -> Vec<u8> {
    let kids = (0..streams.len())
        .map(|i| format!("{} 0 R", 5 + 2 * i))
        .collect::<Vec<_>>()
        .join(" ");
    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", streams.len()).into_bytes(),
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding /ToUnicode 4 0 R >>".to_vec(),
        plain_stream(TO_UNICODE.as_bytes()),
    ];
    for (i, data) in streams.iter().enumerate() {
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {STRESS_SHEET_WIDTH} {STRESS_SHEET_HEIGHT}] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
                6 + 2 * i
            )
            .into_bytes(),
        );
        let mut obj = format!(
            "<< /Filter /FlateDecode /Length {} >>\nstream\n",
            data.len()
        )
        .into_bytes();
        obj.extend_from_slice(data);
        obj.extend_from_slice(b"\nendstream");
        objects.push(obj);
    }
    let mut pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::with_capacity(objects.len());
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        pdf.extend_from_slice(body);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref = pdf.len();
    let mut table = format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(table, "{offset:010} 00000 n ");
    }
    let _ = write!(
        table,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    pdf.extend_from_slice(table.as_bytes());
    pdf
}

fn plain_stream(data: &[u8]) -> Vec<u8> {
    let mut obj = format!("<< /Length {} >>\nstream\n", data.len()).into_bytes();
    obj.extend_from_slice(data);
    obj.extend_from_slice(b"endstream");
    obj
}
