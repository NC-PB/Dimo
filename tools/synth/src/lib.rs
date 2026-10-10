//! Synthetic drawing generator (T0.11, extended for the M2 evaluation in T2.9).
//!
//! Writes one sheet with dimension lines and callouts, plus the matching truth file in the
//! format of T0.10 (`dimo_core::truth`). The callouts are the "common callouts" of the M2 exit
//! criterion (see [`Form`]): linear, diameter, radius, angle and chamfer sizes without
//! tolerance (ISO 2768-1 class m, written into the truth as `tolerance_settings`), symmetric,
//! asymmetric, one sided and limit tolerances, fits with and without printed deviations,
//! metric threads and reference dimensions. One callout in four writes decimal commas.
//! Deviations are stacked (upper above lower, smaller text right of the main text), a limit
//! dimension is two full size lines without a main text. Every fourth callout (T2.6) sits at
//! a vertical dimension line and is rotated by 90 degrees, read bottom to top; its truth region
//! has angle 90.
//!
//! Determinism (AGENTS.md rule 11): all choices come from a seeded generator implemented here,
//! the PDF has no Info dictionary, no document ID and no dates, and all numbers are formatted
//! with fixed precision. Same seed, count and tool version give byte identical files.
//!
//! The truth regions are computed from the layout this module places, using the width table
//! of the standard Helvetica font. The truth never comes from a recognizer.
//!
//! # Symbol encoding
//!
//! The PDF uses the standard font Helvetica (not embedded) with an own `Encoding` dictionary
//! based on `WinAnsiEncoding`. `WinAnsi` has `Ø` and `±` but no U+2212, so the codes `0x80`,
//! `0x81` and `0x82` are remapped by `/Differences` to the Helvetica glyphs `minus`, `Oslash`
//! and `plusminus`; the degree sign keeps its `WinAnsi` code `0xB0`. Every text object is
//! written as a hex string. A `ToUnicode` `CMap` maps the used codes back to U+2212, U+00D8,
//! U+00B1 and U+00B0, so text extraction returns the printed characters (`Ø`, `±`, `−`, `°`),
//! also for readers that ignore glyph names.

// Layout indices are tiny (at most 15), so the casts to f64 are exact.
#![allow(clippy::cast_precision_loss)]

use std::fmt::Write as _;

use dimo_core::derivation::TableRef;
use dimo_core::geometry::{OrientedBox, Point, Size};
use dimo_core::project::{TableClass, ToleranceSettings};
use dimo_core::sheet::SheetKind;
use dimo_core::truth::{
    TRUTH_FORMAT_VERSION, TruthCharacteristic, TruthDrawing, TruthFile, TruthSheet,
};
use sha2::{Digest, Sha256};

/// Sheet width in PDF user units (A4 landscape).
pub const SHEET_WIDTH: f64 = 842.0;
/// Sheet height in PDF user units (A4 landscape).
pub const SHEET_HEIGHT: f64 = 595.0;

const MAIN_SIZE: f64 = 10.0;
const DEV_SIZE: f64 = 7.0;
mod callout;
mod stress;

use callout::{Callout, DEGREE, MINUS, OSLASH, PLUS_MINUS, Slot, make_callout};
pub use callout::{FITS, Form, GENERAL_CLASS, GENERAL_TABLE, GENERAL_TABLE_VERSION};
pub use stress::{STRESS_SHEET_HEIGHT, STRESS_SHEET_WIDTH, generate_stress};

const MAX_COUNT: usize = 15;
const COLUMNS: usize = 3;
const MARGIN: f64 = 40.0;

/// A generated drawing: PDF bytes, truth file text and a file name stem.
#[derive(Debug, Clone)]
pub struct Drawing {
    /// File name stem, `synth_<seed>`; the files are `<name>.pdf` and `<name>.truth.json`.
    pub name: String,
    /// The PDF file.
    pub pdf: Vec<u8>,
    /// The truth file as pretty printed JSON with a trailing newline.
    pub truth_json: String,
    /// The truth file as a value.
    pub truth: TruthFile,
    /// The form of each characteristic, in truth order.
    pub forms: Vec<Form>,
}

/// Errors from [`generate`].
#[derive(Debug)]
pub enum SynthError {
    /// `count` is outside 1..=15.
    BadCount(usize),
    /// The truth file could not be serialized.
    Json(serde_json::Error),
}

impl std::fmt::Display for SynthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadCount(n) => write!(f, "count {n} is outside 1..=15"),
            Self::Json(e) => write!(f, "cannot serialize truth: {e}"),
        }
    }
}

impl std::error::Error for SynthError {}

/// Generate a drawing with `count` callouts (1 to 15) from `seed`.
pub fn generate(seed: u64, count: usize) -> Result<Drawing, SynthError> {
    if !(1..=MAX_COUNT).contains(&count) {
        return Err(SynthError::BadCount(count));
    }
    let mut rng = Rng(seed);
    let rows = count.div_ceil(COLUMNS);
    let cell_w = (SHEET_WIDTH - 2.0 * MARGIN) / COLUMNS as f64;
    let cell_h = (SHEET_HEIGHT - 2.0 * MARGIN) / rows as f64;

    let mut content = String::new();
    let mut characteristics = Vec::new();
    let mut forms = Vec::new();
    for index in 0..count {
        let cell_x = MARGIN + (index % COLUMNS) as f64 * cell_w;
        let cell_y = MARGIN + (index / COLUMNS) as f64 * cell_h;
        let callout = make_callout(&mut rng);

        // Dimension line with two extension ticks, the callout sits above it (left of it when
        // rotated). The length is drawn in both cases, so the generator sequence is the same.
        let length = f64::from(rng.range(120, 220));
        let rotated = index % 4 == 3;
        let (parts, region) = if rotated {
            let x = r2(cell_x + 60.0);
            let y1 = r2(cell_y + cell_h - 8.0);
            let y0 = r2(y1 - length.min(cell_h - 16.0));
            line(&mut content, x, y0, x, y1);
            line(&mut content, x - 8.0, y0, x + 8.0, y0);
            line(&mut content, x - 8.0, y1, x + 8.0, y1);
            layout(&callout, r2(x - 6.0), r2(y1 - 6.0), true)
        } else {
            let x0 = r2(cell_x + 16.0);
            let x1 = r2(x0 + length);
            let line_y = r2(cell_y + cell_h * 0.65);
            let baseline = r2(line_y - 6.0);
            line(&mut content, x0, line_y, x1, line_y);
            line(&mut content, x0, line_y - 8.0, x0, line_y + 8.0);
            line(&mut content, x1, line_y - 8.0, x1, line_y + 8.0);
            layout(&callout, r2(x0 + 6.0), baseline, false)
        };
        for part in &parts {
            text(&mut content, part);
        }
        forms.push(callout.form);
        characteristics.push(into_truth(callout, index, region));
    }
    let pdf = write_pdf(&content);

    let name = format!("synth_{seed}");
    let truth = TruthFile {
        format_version: TRUTH_FORMAT_VERSION,
        drawing: TruthDrawing {
            file: format!("{name}.pdf"),
            sha256: format!("{:x}", Sha256::digest(&pdf)),
        },
        sheets: vec![TruthSheet {
            index: 0,
            kind: SheetKind::VectorText,
            size: Size {
                width: SHEET_WIDTH,
                height: SHEET_HEIGHT,
            },
        }],
        characteristics,
        tolerance_settings: Some(tolerance_settings()),
        notes: notes(seed, count),
    };
    let mut truth_json = serde_json::to_string_pretty(&truth).map_err(SynthError::Json)?;
    truth_json.push('\n');
    Ok(Drawing {
        name,
        pdf,
        truth_json,
        truth,
        forms,
    })
}

fn notes(seed: u64, count: usize) -> Vec<String> {
    vec![
        format!(
            "Synthetic drawing from dimo-synth {} (seed {seed}, {count} callouts). Not a corpus drawing; drawing.file is the file name next to this truth file.",
            env!("CARGO_PKG_VERSION")
        ),
        "Regions are computed from the placed text with the Helvetica width table: x from the start to the end of the text, y from 0.8 times the font size above the baseline to 0.2 times below it. Stacked deviations are 7 pt text right of the main text; a limit dimension is two 10 pt lines, upper limit above lower limit, the upper one written first and taken as nominal.".to_string(),
        "Every fourth callout is rotated by 90 degrees (read bottom to top) at a vertical dimension line; its region is measured the same way in the reading direction and has angle 90.".to_string(),
        "tolerance_settings: untoleranced sizes follow ISO 2768-1 class m. The expected limits come from tables in the generator (ISO 2768-1 tables 1 to 3, ISO 286-2 fits in FITS), written by hand and independent of dimo-tolerance. They are drafts like every agent written table (D-43); entries that use them carry a review_note.".to_string(),
        "Angles without tolerance have limits from ISO 2768-1 table 3 by the shorter leg of the part, given in the review_note. The leg is not on the sheet.".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// Truth

/// The project tolerance settings the expected limits assume: ISO 2768-1 class m, no drawing
/// rule, no decimal place rules, default unit rounding.
pub fn tolerance_settings() -> ToleranceSettings {
    ToleranceSettings {
        general: Some(TableClass {
            table: TableRef {
                id: GENERAL_TABLE.to_owned(),
                version: GENERAL_TABLE_VERSION,
            },
            class: GENERAL_CLASS.to_owned(),
        }),
        ..ToleranceSettings::default()
    }
}

fn into_truth(callout: Callout, index: usize, region: OrientedBox) -> TruthCharacteristic {
    TruthCharacteristic {
        id: format!("c{:02}", index + 1),
        sheet: 0,
        kind: callout.kind,
        requirement_text: callout.requirement_text(),
        region,
        nominal: Some(callout.nominal),
        unit: Some(callout.unit),
        fit: callout.fit,
        tolerance_rule: Some(callout.rule),
        upper_limit: callout.limits.map(|(u, _)| u),
        lower_limit: callout.limits.map(|(_, l)| l),
        inspect: callout.inspect,
        review_note: callout.review_note,
    }
}

// ---------------------------------------------------------------------------------------------
// Layout and PDF

/// One placed text object, positions in sheet space (origin top left, y down). `x` and
/// `baseline` are the start of the text on its baseline; rotated text runs up the sheet from
/// there.
#[derive(Debug, Clone)]
struct Placed {
    text: String,
    x: f64,
    baseline: f64,
    size: f64,
    rotated: bool,
}

fn text_width(text: &str, size: f64) -> f64 {
    text.chars().map(|c| glyph_width(c) * size / 1000.0).sum()
}

impl Placed {
    fn width(&self) -> f64 {
        text_width(&self.text, self.size)
    }
    fn top(&self) -> f64 {
        self.baseline - 0.8 * self.size
    }
    fn bottom(&self) -> f64 {
        self.baseline + 0.2 * self.size
    }
}

/// Place the texts of a callout and compute the box around all of them. The texts start at
/// `x`, `baseline`; `rotated` turns the whole callout by 90 degrees counterclockwise around that
/// point (read bottom to top).
fn layout(callout: &Callout, x: f64, baseline: f64, rotated: bool) -> (Vec<Placed>, OrientedBox) {
    // Laid out in the reading frame with the start at the origin, then moved to the sheet.
    let main_width = callout
        .texts
        .iter()
        .filter(|(_, slot)| *slot == Slot::Main)
        .map(|(text, _)| text_width(text, MAIN_SIZE))
        .sum::<f64>();
    let dev_x = r2(main_width + 1.5);
    let mut parts: Vec<Placed> = callout
        .texts
        .iter()
        .map(|(text, slot)| {
            let (x, baseline, size) = match slot {
                Slot::Main => (0.0, 0.0, MAIN_SIZE),
                Slot::Upper => (dev_x, -4.0, DEV_SIZE),
                Slot::Lower => (dev_x, 4.0, DEV_SIZE),
                Slot::LimitUpper => (0.0, -5.5, MAIN_SIZE),
                Slot::LimitLower => (0.0, 5.5, MAIN_SIZE),
            };
            Placed {
                text: text.clone(),
                x,
                baseline,
                size,
                rotated,
            }
        })
        .collect();
    let left = parts.iter().map(|p| p.x).fold(f64::MAX, f64::min);
    let right = parts
        .iter()
        .map(|p| p.x + p.width())
        .fold(f64::MIN, f64::max);
    let top = parts.iter().map(Placed::top).fold(f64::MAX, f64::min);
    let bottom = parts.iter().map(Placed::bottom).fold(f64::MIN, f64::max);
    // Reading frame (u along, v down) to the sheet: unrotated (x + u, baseline + v), rotated
    // by 90 degrees (x + v, baseline - u).
    let to_sheet = |u: f64, v: f64| {
        if rotated {
            (x + v, baseline - u)
        } else {
            (x + u, baseline + v)
        }
    };
    for part in &mut parts {
        let (px, py) = to_sheet(part.x, part.baseline);
        part.x = r2(px);
        part.baseline = r2(py);
    }
    let (cx, cy) = to_sheet(f64::midpoint(left, right), f64::midpoint(top, bottom));
    let region = OrientedBox {
        center: Point {
            x: r2(cx),
            y: r2(cy),
        },
        size: Size {
            width: r2(right - left),
            height: r2(bottom - top),
        },
        angle: if rotated { 90.0 } else { 0.0 },
    };
    (parts, region)
}

/// Advance width in 1/1000 em from the Helvetica AFM, for the characters the generator uses.
fn glyph_width(c: char) -> f64 {
    match c {
        '0'..='9' | 'a' | 'b' | 'd' | 'e' | 'g' | 'h' | 'n' | 'p' | 'u' => 556.0,
        ' ' | '.' | ',' | 'f' | 't' | '/' => 278.0,
        '+' | PLUS_MINUS | MINUS => 584.0,
        '-' | '(' | ')' | 'r' => 333.0,
        DEGREE => 400.0,
        OSLASH | 'G' => 778.0,
        'C' | 'D' | 'H' | 'R' => 722.0,
        'E' | 'X' => 667.0,
        'F' => 611.0,
        'M' | 'm' => 833.0,
        'c' | 'k' | 's' | 'x' => 500.0,
        'j' => 222.0,
        other => unreachable!("character {other:?} has no width entry in the generator"),
    }
}

/// Single byte code of a character in the PDF encoding (see the module docs).
fn code(c: char) -> u8 {
    match c {
        MINUS => 0x80,
        OSLASH => 0x81,
        PLUS_MINUS => 0x82,
        DEGREE => 0xB0,
        c if c.is_ascii() => c as u8,
        other => unreachable!("character {other:?} has no code in the generator"),
    }
}

/// Round to 0.01, the precision of positions in both the PDF and the truth.
fn r2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// PDF y coordinate (origin bottom left) of a sheet space y.
fn pdf_y(y: f64) -> f64 {
    SHEET_HEIGHT - y
}

fn line(out: &mut String, x0: f64, y0: f64, x1: f64, y1: f64) {
    let _ = writeln!(
        out,
        "0.5 w {x0:.2} {:.2} m {x1:.2} {:.2} l S",
        pdf_y(y0),
        pdf_y(y1)
    );
}

fn text(out: &mut String, p: &Placed) {
    let hex: String = p.text.chars().fold(String::new(), |mut s, c| {
        let _ = write!(s, "{:02X}", code(c));
        s
    });
    if p.rotated {
        // Text matrix turned by 90 degrees: the baseline runs up the page.
        let _ = writeln!(
            out,
            "BT /F1 {:.1} Tf 0 1 -1 0 {:.2} {:.2} Tm <{hex}> Tj ET",
            p.size,
            p.x,
            pdf_y(p.baseline)
        );
    } else {
        let _ = writeln!(
            out,
            "BT /F1 {:.1} Tf {:.2} {:.2} Td <{hex}> Tj ET",
            p.size,
            p.x,
            pdf_y(p.baseline)
        );
    }
}

const TO_UNICODE: &str = "/CIDInit /ProcSet findresource begin
12 dict begin
begincmap
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def
/CMapName /Adobe-Identity-UCS def
/CMapType 2 def
1 begincodespacerange
<00> <FF>
endcodespacerange
1 beginbfrange
<20> <7E> <0020>
endbfrange
4 beginbfchar
<80> <2212>
<81> <00D8>
<82> <00B1>
<B0> <00B0>
endbfchar
endcmap
CMapName currentdict /CMap defineresource pop
end
end
";

/// Assemble the PDF by hand: six objects, no compression, no Info, no ID, no dates.
fn write_pdf(content: &str) -> Vec<u8> {
    let objects: [String; 6] = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {SHEET_WIDTH} {SHEET_HEIGHT}] /Resources << /Font << /F1 5 0 R >> >> /Contents 4 0 R >>"
        ),
        stream(content),
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding << /Type /Encoding /BaseEncoding /WinAnsiEncoding /Differences [128 /minus /Oslash /plusminus] >> /ToUnicode 6 0 R >>".to_string(),
        stream(TO_UNICODE),
    ];
    let mut pdf = b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
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

fn stream(data: &str) -> String {
    format!("<< /Length {} >>\nstream\n{data}endstream", data.len())
}

// ---------------------------------------------------------------------------------------------
// Seeded generator

/// `SplitMix64`: tiny, fixed algorithm, so output never depends on a library version.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform value in `lo..=hi`.
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        let span = u64::from(hi - lo) + 1;
        // Modulo bias is irrelevant for layout choices.
        lo + u32::try_from(self.next() % span).unwrap_or(0)
    }
}
