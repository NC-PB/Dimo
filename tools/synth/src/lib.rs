//! Synthetic drawing generator, first cut (T0.11, roadmap "First steps" 4).
//!
//! Writes one sheet with dimension lines and callouts, plus the matching truth file in the
//! format of T0.10 (`dimo_core::truth`). Callout styles: plain linear size (no tolerance),
//! symmetric `±`, stacked deviations (upper and lower as separate smaller text objects) and
//! fits such as `Ø20 H7`. Every fourth callout (T2.6) sits at a vertical dimension line and is
//! rotated by 90 degrees, read bottom to top; its truth region has angle 90.
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
//! and `plusminus`. Every text object is written as a hex string. A `ToUnicode` `CMap` maps the
//! used codes back to U+2212, U+00D8 and U+00B1, so text extraction returns the printed
//! characters (`Ø`, `±`, `−`), also for readers that ignore glyph names.

// Layout indices are tiny (at most 15), so the casts to f64 are exact.
#![allow(clippy::cast_precision_loss)]

use std::fmt::Write as _;

use dimo_core::characteristic::{CharacteristicKind, ToleranceRule, Unit};
use dimo_core::geometry::{OrientedBox, Point, Size};
use dimo_core::sheet::SheetKind;
use dimo_core::truth::{
    TRUTH_FORMAT_VERSION, TruthCharacteristic, TruthDrawing, TruthFile, TruthSheet,
};
use rust_decimal::Decimal;
use sha2::{Digest, Sha256};

/// Sheet width in PDF user units (A4 landscape).
pub const SHEET_WIDTH: f64 = 842.0;
/// Sheet height in PDF user units (A4 landscape).
pub const SHEET_HEIGHT: f64 = 595.0;

const MAIN_SIZE: f64 = 10.0;
const DEV_SIZE: f64 = 7.0;
mod stress;

pub use stress::{STRESS_SHEET_HEIGHT, STRESS_SHEET_WIDTH, generate_stress};

const MAX_COUNT: usize = 15;
const COLUMNS: usize = 3;
const MARGIN: f64 = 40.0;

const MINUS: char = '\u{2212}';
const OSLASH: char = 'Ø';
const PLUS_MINUS: char = '±';

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
        characteristics.push(callout.into_truth(index, region));
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
        notes: notes(seed, count),
    };
    let mut truth_json = serde_json::to_string_pretty(&truth).map_err(SynthError::Json)?;
    truth_json.push('\n');
    Ok(Drawing {
        name,
        pdf,
        truth_json,
        truth,
    })
}

fn notes(seed: u64, count: usize) -> Vec<String> {
    vec![
        format!(
            "Synthetic drawing from dimo-synth {} (seed {seed}, {count} callouts). Not a corpus drawing; drawing.file is the file name next to this truth file.",
            env!("CARGO_PKG_VERSION")
        ),
        "Regions are computed from the placed text with the Helvetica width table: x from the start to the end of the text, y from 0.8 times the font size above the baseline to 0.2 times below it. Stacked deviations are 7 pt text right of the main text.".to_string(),
        "Every fourth callout is rotated by 90 degrees (read bottom to top) at a vertical dimension line; its region is measured the same way in the reading direction and has angle 90.".to_string(),
        "Fit limits come from a small table in the generator and are drafts: the ISO 286 data tables are not verified yet (D-43). Entries with a fit carry a review_note.".to_string(),
    ]
}

// ---------------------------------------------------------------------------------------------
// Callouts

/// Fit table, micrometres: (nominal mm, designation, upper deviation, lower deviation).
/// Values for shafts and holes as recalled from ISO 286 for nominal sizes 10, 20 and 40 mm.
/// DRAFT, not verified by the owner (AGENTS.md rule 7, D-43). The tolerance engine (M2) replaces
/// this table; it exists only so that fit callouts have an expected limit pair.
const FITS: &[(i64, &str, i64, i64)] = &[
    (10, "H7", 15, 0),
    (10, "h6", 0, -9),
    (10, "f7", -13, -28),
    (10, "g6", -5, -14),
    (10, "k6", 10, 1),
    (20, "H7", 21, 0),
    (20, "h6", 0, -13),
    (20, "f7", -20, -41),
    (20, "g6", -7, -20),
    (20, "k6", 15, 2),
    (40, "H7", 25, 0),
    (40, "h6", 0, -16),
    (40, "f7", -25, -50),
    (40, "g6", -9, -25),
    (40, "k6", 18, 2),
];

#[derive(Debug, Clone)]
enum Style {
    Plain,
    Symmetric(Decimal),
    /// Upper and lower deviation as magnitudes; the lower one is printed with a minus.
    Stacked(Decimal, Decimal),
    Fit(&'static str, Decimal, Decimal),
}

#[derive(Debug, Clone)]
struct Callout {
    diameter: bool,
    nominal: Decimal,
    style: Style,
}

fn make_callout(rng: &mut Rng) -> Callout {
    let pick = rng.range(0, 3);
    if pick == 3 {
        let (nominal, fit, upper, lower) =
            FITS[rng.range(0, u32::try_from(FITS.len() - 1).unwrap_or(0)) as usize];
        return Callout {
            diameter: true,
            nominal: Decimal::from(nominal),
            style: Style::Fit(fit, Decimal::new(upper, 3), Decimal::new(lower, 3)),
        };
    }
    let scale = rng.range(0, 2);
    let int = rng.range(5, 250);
    let mut mantissa = i64::from(int);
    for digit in 0..scale {
        let last = digit + 1 == scale;
        // The last decimal is never 0, so the printed text equals the decimal.
        let d = if last {
            rng.range(1, 9)
        } else {
            rng.range(0, 9)
        };
        mantissa = mantissa * 10 + i64::from(d);
    }
    let nominal = Decimal::new(mantissa, scale);
    let style = match pick {
        0 => Style::Plain,
        1 => Style::Symmetric(Decimal::new([5, 10, 20, 50][rng.range(0, 3) as usize], 2)),
        _ => Style::Stacked(
            Decimal::new([5, 10, 20][rng.range(0, 2) as usize], 2),
            Decimal::new([2, 5, 10][rng.range(0, 2) as usize], 2),
        ),
    };
    Callout {
        diameter: rng.range(0, 3) == 0,
        nominal,
        style,
    }
}

impl Callout {
    fn prefix(&self) -> &'static str {
        if self.diameter { "Ø" } else { "" }
    }

    /// Main text, then optional upper and lower deviation text.
    fn texts(&self) -> (String, Option<String>, Option<String>) {
        let p = self.prefix();
        let n = &self.nominal;
        match &self.style {
            Style::Plain => (format!("{p}{n}"), None, None),
            Style::Symmetric(t) => (format!("{p}{n}{PLUS_MINUS}{t}"), None, None),
            Style::Stacked(up, low) => (
                format!("{p}{n}"),
                Some(format!("+{up}")),
                Some(format!("{MINUS}{low}")),
            ),
            Style::Fit(fit, _, _) => (format!("{p}{n} {fit}"), None, None),
        }
    }

    fn into_truth(self, index: usize, region: OrientedBox) -> TruthCharacteristic {
        let (main, upper, lower) = self.texts();
        let requirement_text = [Some(main), upper, lower]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        let n = self.nominal;
        let (fit, rule, limits, review_note) = match self.style {
            Style::Plain => (None, ToleranceRule::NoToleranceDefined, None, None),
            Style::Symmetric(t) => (None, ToleranceRule::Explicit, Some((n + t, n - t)), None),
            Style::Stacked(up, low) => {
                (None, ToleranceRule::Explicit, Some((n + up, n - low)), None)
            }
            Style::Fit(fit, up, low) => (
                Some(fit.to_string()),
                ToleranceRule::Fit,
                Some((n + up, n + low)),
                Some(format!(
                    "Draft limits from the generator's small fit table (ISO 286 as recalled by the agent, not verified, D-43). Owner: check {fit} at {n} mm."
                )),
            ),
        };
        TruthCharacteristic {
            id: format!("c{:02}", index + 1),
            sheet: 0,
            kind: if self.diameter {
                CharacteristicKind::Diameter
            } else {
                CharacteristicKind::Linear
            },
            requirement_text,
            region,
            nominal: Some(n),
            unit: Some(Unit::Mm),
            fit,
            tolerance_rule: Some(rule),
            upper_limit: limits.map(|(u, _)| u),
            lower_limit: limits.map(|(_, l)| l),
            inspect: true,
            review_note,
        }
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

impl Placed {
    fn width(&self) -> f64 {
        self.text
            .chars()
            .map(|c| glyph_width(c) * self.size / 1000.0)
            .sum()
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
    let (main, upper, lower) = callout.texts();
    let main = Placed {
        text: main,
        x: 0.0,
        baseline: 0.0,
        size: MAIN_SIZE,
        rotated,
    };
    let mut parts = vec![main];
    let dev_x = r2(parts[0].width() + 1.5);
    if let Some(text) = upper {
        parts.push(Placed {
            text,
            x: dev_x,
            baseline: -4.0,
            size: DEV_SIZE,
            rotated,
        });
    }
    if let Some(text) = lower {
        parts.push(Placed {
            text,
            x: dev_x,
            baseline: 4.0,
            size: DEV_SIZE,
            rotated,
        });
    }
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
        '0'..='9' => 556.0,
        ' ' | '.' | 'f' => 278.0,
        '+' | PLUS_MINUS | MINUS => 584.0,
        OSLASH => 778.0,
        'H' => 722.0,
        'h' | 'g' | 'p' => 556.0,
        'k' | 's' => 500.0,
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
3 beginbfchar
<80> <2212>
<81> <00D8>
<82> <00B1>
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
