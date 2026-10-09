//! The bundled balloon font and a small TrueType subsetter (T1.2, D-24).
//!
//! Balloon numbers use Open Sans Bold 1.10 (Apache License 2.0), bundled from
//! `data/fonts/OpenSans-Bold.ttf`, see `data/fonts/README.md`. Numbers therefore look the same
//! in every viewer, independent of the fonts installed on a machine.
//!
//! The ballooned PDF embeds a subset with only the glyphs the balloon texts use. PDFium embeds
//! whatever font program it is given (`FPDFText_LoadFont`) and needs a `cmap` to map text to
//! glyphs, so the subsetter here keeps a working `cmap`:
//!
//! - glyph set: `.notdef`, the glyphs of the requested characters and all components of
//!   composite glyphs, renumbered in ascending order of the original glyph IDs;
//! - rebuilt tables: `glyf`, `loca` (long format), `hmtx`, `cmap` (format 4, Windows Unicode
//!   BMP), `post` (format 3, no glyph names), `name` (Windows records, with the PostScript name
//!   prefixed by a subset tag), `head`, `hhea` and `maxp` with the new counts;
//! - copied unchanged: `OS/2` and the hinting tables `cvt `, `fpgm`, `prep`, `gasp`;
//! - dropped: everything else (layout tables such as `GSUB` and `GPOS`, which a single line of
//!   digits does not need).
//!
//! The output depends only on the font and the character set, so it is deterministic
//! (FR-EXP-11). Only TrueType outlines (`glyf`) are supported, which is all the bundled font
//! needs.

use std::collections::{BTreeMap, BTreeSet};

use crate::PdfError;
use crate::hash::ContentHash;

/// The bundled balloon font program (Open Sans Bold 1.10, Apache License 2.0).
pub(crate) static BALLOON_FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../data/fonts/OpenSans-Bold.ttf"
));

/// Tables copied unchanged into a subset.
const COPIED_TABLES: [[u8; 4]; 5] = [*b"OS/2", *b"cvt ", *b"fpgm", *b"gasp", *b"prep"];

/// Composite glyph flags (OpenType `glyf` specification).
const ARG_1_AND_2_ARE_WORDS: u16 = 0x0001;
const WE_HAVE_A_SCALE: u16 = 0x0008;
const MORE_COMPONENTS: u16 = 0x0020;
const WE_HAVE_AN_X_AND_Y_SCALE: u16 = 0x0040;
const WE_HAVE_A_TWO_BY_TWO: u16 = 0x0080;

fn font_error(message: impl Into<String>) -> PdfError {
    PdfError::Font(message.into())
}

/// A parsed TrueType font: just enough to measure text and to subset it.
#[derive(Debug, Clone)]
pub(crate) struct TrueTypeFont<'a> {
    data: &'a [u8],
    /// Table tag to (offset, length).
    tables: BTreeMap<[u8; 4], (usize, usize)>,
    units_per_em: u16,
    cap_height: i16,
    num_glyphs: u16,
    /// Byte range of each glyph in `glyf`, from `loca`.
    glyph_ranges: Vec<(usize, usize)>,
    /// Advance width and left side bearing per glyph.
    metrics: Vec<(u16, i16)>,
    /// Unicode code point to glyph ID, from the Windows Unicode BMP `cmap`.
    cmap: BTreeMap<u32, u16>,
}

impl<'a> TrueTypeFont<'a> {
    /// Parses the tables a subset needs.
    pub(crate) fn parse(data: &'a [u8]) -> Result<Self, PdfError> {
        let r = Reader(data);
        let version = r.u32(0)?;
        if version != 0x0001_0000 && version != u32::from_be_bytes(*b"true") {
            return Err(font_error("not a TrueType font (glyf outlines required)"));
        }
        let count = usize::from(r.u16(4)?);
        let mut tables = BTreeMap::new();
        for i in 0..count {
            let record = 12 + 16 * i;
            let tag: [u8; 4] = r.array(record)?;
            let offset = r.u32(record + 8)? as usize;
            let length = r.u32(record + 12)? as usize;
            r.slice(offset, length)?;
            tables.insert(tag, (offset, length));
        }
        let mut font = Self {
            data,
            tables,
            units_per_em: 0,
            cap_height: 0,
            num_glyphs: 0,
            glyph_ranges: Vec::new(),
            metrics: Vec::new(),
            cmap: BTreeMap::new(),
        };
        let head = font.table(*b"head")?;
        font.units_per_em = head.u16(18)?;
        if font.units_per_em == 0 {
            return Err(font_error("unitsPerEm is 0"));
        }
        let long_loca = head.u16(50)? == 1;
        font.num_glyphs = font.table(*b"maxp")?.u16(4)?;
        font.glyph_ranges = font.parse_loca(long_loca)?;
        font.metrics = font.parse_hmtx()?;
        font.cmap = font.parse_cmap()?;
        // sCapHeight exists from OS/2 version 2; 0.7 em is a typical value otherwise.
        font.cap_height = match font.table(*b"OS/2") {
            Ok(os2) if os2.u16(0)? >= 2 => os2.i16(88)?,
            _ => i16::try_from(u32::from(font.units_per_em) * 7 / 10).unwrap_or(i16::MAX),
        };
        Ok(font)
    }

    fn table(&self, tag: [u8; 4]) -> Result<Reader<'a>, PdfError> {
        let (offset, length) = self.tables.get(&tag).copied().ok_or_else(|| {
            font_error(format!(
                "table {} missing",
                String::from_utf8_lossy(&tag).trim_end()
            ))
        })?;
        Ok(Reader(&self.data[offset..offset + length]))
    }

    fn parse_loca(&self, long: bool) -> Result<Vec<(usize, usize)>, PdfError> {
        let loca = self.table(*b"loca")?;
        let glyf_len = self.table(*b"glyf")?.0.len();
        let offset = |i: usize| -> Result<usize, PdfError> {
            if long {
                Ok(loca.u32(4 * i)? as usize)
            } else {
                Ok(usize::from(loca.u16(2 * i)?) * 2)
            }
        };
        let n = usize::from(self.num_glyphs);
        let mut ranges = Vec::with_capacity(n);
        for i in 0..n {
            let (start, end) = (offset(i)?, offset(i + 1)?);
            if start > end || end > glyf_len {
                return Err(font_error(format!("bad loca entry for glyph {i}")));
            }
            ranges.push((start, end));
        }
        Ok(ranges)
    }

    fn parse_hmtx(&self) -> Result<Vec<(u16, i16)>, PdfError> {
        let long_metrics = usize::from(self.table(*b"hhea")?.u16(34)?);
        let hmtx = self.table(*b"hmtx")?;
        if long_metrics == 0 {
            return Err(font_error("numberOfHMetrics is 0"));
        }
        let mut metrics = Vec::with_capacity(usize::from(self.num_glyphs));
        for i in 0..usize::from(self.num_glyphs) {
            if i < long_metrics {
                metrics.push((hmtx.u16(4 * i)?, hmtx.i16(4 * i + 2)?));
            } else {
                let advance = metrics.last().map_or(0, |m: &(u16, i16)| m.0);
                let lsb = hmtx.i16(4 * long_metrics + 2 * (i - long_metrics))?;
                metrics.push((advance, lsb));
            }
        }
        Ok(metrics)
    }

    /// Reads the Windows Unicode BMP subtable (platform 3, encoding 1, format 4).
    fn parse_cmap(&self) -> Result<BTreeMap<u32, u16>, PdfError> {
        let cmap = self.table(*b"cmap")?;
        let count = usize::from(cmap.u16(2)?);
        let mut subtable = None;
        for i in 0..count {
            let record = 4 + 8 * i;
            if cmap.u16(record)? == 3 && cmap.u16(record + 2)? == 1 {
                subtable = Some(cmap.u32(record + 4)? as usize);
            }
        }
        let start = subtable.ok_or_else(|| font_error("no Windows Unicode cmap"))?;
        if cmap.u16(start)? != 4 {
            return Err(font_error("Windows Unicode cmap is not format 4"));
        }
        let seg_count = usize::from(cmap.u16(start + 6)? / 2);
        let ends = start + 14;
        let starts = ends + 2 * seg_count + 2;
        let deltas = starts + 2 * seg_count;
        let range_offsets = deltas + 2 * seg_count;
        let mut map = BTreeMap::new();
        for s in 0..seg_count {
            let end = cmap.u16(ends + 2 * s)?;
            let first = cmap.u16(starts + 2 * s)?;
            let delta = cmap.u16(deltas + 2 * s)?;
            let range_offset = usize::from(cmap.u16(range_offsets + 2 * s)?);
            if first > end {
                continue;
            }
            for code in first..=end {
                if code == 0xFFFF {
                    break;
                }
                let glyph = if range_offset == 0 {
                    code.wrapping_add(delta)
                } else {
                    let at = range_offsets + 2 * s + range_offset + 2 * usize::from(code - first);
                    match cmap.u16(at)? {
                        0 => 0,
                        g => g.wrapping_add(delta),
                    }
                };
                if glyph != 0 && glyph < self.num_glyphs {
                    map.insert(u32::from(code), glyph);
                }
            }
        }
        Ok(map)
    }

    /// Font design units per em.
    pub(crate) const fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    /// Height of capital letters (and lining digits) in font units.
    pub(crate) const fn cap_height(&self) -> i16 {
        self.cap_height
    }

    /// The glyph of a character, if the font has one.
    pub(crate) fn glyph(&self, ch: char) -> Option<u16> {
        self.cmap.get(&u32::from(ch)).copied()
    }

    /// Advance width of a glyph in font units.
    fn advance(&self, glyph: u16) -> u16 {
        self.metrics.get(usize::from(glyph)).map_or(0, |m| m.0)
    }

    /// Width of `text` set at `size` (units per em scaled to `size`), without kerning.
    /// Characters the font lacks count as `.notdef`.
    pub(crate) fn text_width(&self, text: &str, size: f64) -> f64 {
        let units: u32 = text
            .chars()
            .map(|c| u32::from(self.advance(self.glyph(c).unwrap_or(0))))
            .sum();
        f64::from(units) * size / f64::from(self.units_per_em)
    }

    /// The outline data of a glyph. `glyph` must be below `num_glyphs`; the ranges were
    /// checked against the `glyf` table when parsing.
    fn glyph_data(&self, glyph: u16) -> &'a [u8] {
        let (start, end) = self.glyph_ranges[usize::from(glyph)];
        let (offset, _) = self.tables[b"glyf"];
        &self.data[offset + start..offset + end]
    }

    /// Positions of the component glyph IDs inside a composite glyph, empty for simple glyphs.
    fn component_positions(data: &[u8]) -> Result<Vec<usize>, PdfError> {
        let r = Reader(data);
        if data.is_empty() || r.i16(0)? >= 0 {
            return Ok(Vec::new());
        }
        let mut positions = Vec::new();
        let mut at = 10;
        loop {
            let flags = r.u16(at)?;
            positions.push(at + 2);
            at += 4;
            at += if flags & ARG_1_AND_2_ARE_WORDS != 0 {
                4
            } else {
                2
            };
            if flags & WE_HAVE_A_SCALE != 0 {
                at += 2;
            } else if flags & WE_HAVE_AN_X_AND_Y_SCALE != 0 {
                at += 4;
            } else if flags & WE_HAVE_A_TWO_BY_TWO != 0 {
                at += 8;
            }
            if flags & MORE_COMPONENTS == 0 {
                return Ok(positions);
            }
        }
    }

    /// Builds a subset with the glyphs of `chars`. Characters the font lacks are an error.
    pub(crate) fn subset(&self, chars: &BTreeSet<char>) -> Result<Vec<u8>, PdfError> {
        // Glyph closure: .notdef, the mapped glyphs and all composite components.
        let mut glyphs = BTreeSet::from([0u16]);
        let mut cmap = BTreeMap::new();
        for &ch in chars {
            let glyph = self
                .glyph(ch)
                .ok_or_else(|| font_error(format!("the balloon font has no glyph for {ch:?}")))?;
            cmap.insert(ch, glyph);
            glyphs.insert(glyph);
        }
        let mut pending: Vec<u16> = glyphs.iter().copied().collect();
        while let Some(glyph) = pending.pop() {
            let data = self.glyph_data(glyph);
            for at in Self::component_positions(data)? {
                let component = Reader(data).u16(at)?;
                if component >= self.num_glyphs {
                    return Err(font_error(format!("glyph {glyph} has a bad component")));
                }
                if glyphs.insert(component) {
                    pending.push(component);
                }
            }
        }
        let new_id: BTreeMap<u16, u16> = glyphs
            .iter()
            .enumerate()
            .map(|(new, &old)| {
                Ok((
                    old,
                    u16::try_from(new).map_err(|_| font_error("too many glyphs"))?,
                ))
            })
            .collect::<Result<_, PdfError>>()?;
        let count = u16::try_from(glyphs.len()).map_err(|_| font_error("too many glyphs"))?;

        // glyf and loca (long offsets, glyphs padded to 4 bytes), hmtx.
        let mut glyf = Vec::new();
        let mut loca = Vec::new();
        let mut hmtx = Vec::new();
        for &old in &glyphs {
            loca.extend_from_slice(&u32_len(glyf.len())?.to_be_bytes());
            let data = self.glyph_data(old);
            let start = glyf.len();
            glyf.extend_from_slice(data);
            for at in Self::component_positions(data)? {
                let component = Reader(data).u16(at)?;
                let mapped = new_id[&component].to_be_bytes();
                glyf[start + at..start + at + 2].copy_from_slice(&mapped);
            }
            pad4(&mut glyf);
            let (advance, lsb) = self.metrics[usize::from(old)];
            hmtx.extend_from_slice(&advance.to_be_bytes());
            hmtx.extend_from_slice(&lsb.to_be_bytes());
        }
        loca.extend_from_slice(&u32_len(glyf.len())?.to_be_bytes());

        let mut head = self.table(*b"head")?.0.to_vec();
        head[8..12].copy_from_slice(&[0; 4]); // checkSumAdjustment, set at the end
        head[50..52].copy_from_slice(&1u16.to_be_bytes()); // indexToLocFormat: long
        let mut hhea = self.table(*b"hhea")?.0.to_vec();
        hhea[34..36].copy_from_slice(&count.to_be_bytes());
        let mut maxp = self.table(*b"maxp")?.0.to_vec();
        maxp[4..6].copy_from_slice(&count.to_be_bytes());
        let mut post = self
            .table(*b"post")?
            .0
            .get(..32)
            .ok_or_else(|| font_error("post too short"))?
            .to_vec();
        post[0..4].copy_from_slice(&0x0003_0000u32.to_be_bytes());

        let cmap_new: BTreeMap<u16, u16> = cmap
            .iter()
            .map(|(&ch, old)| {
                let code = u16::try_from(u32::from(ch))
                    .map_err(|_| font_error(format!("{ch:?} is outside the BMP")))?;
                Ok((code, new_id[old]))
            })
            .collect::<Result<_, PdfError>>()?;

        let mut tables: BTreeMap<[u8; 4], Vec<u8>> = BTreeMap::new();
        tables.insert(*b"cmap", build_cmap(&cmap_new)?);
        tables.insert(*b"glyf", glyf);
        tables.insert(*b"head", head);
        tables.insert(*b"hhea", hhea);
        tables.insert(*b"hmtx", hmtx);
        tables.insert(*b"loca", loca);
        tables.insert(*b"maxp", maxp);
        tables.insert(*b"name", self.build_name(&subset_tag(&cmap_new))?);
        tables.insert(*b"post", post);
        for tag in COPIED_TABLES {
            if let Ok(table) = self.table(tag) {
                tables.insert(tag, table.0.to_vec());
            }
        }
        assemble(&tables)
    }

    /// The Windows name records, with the PostScript name (ID 6) prefixed by `tag` and `+`.
    fn build_name(&self, tag: &str) -> Result<Vec<u8>, PdfError> {
        let name = self.table(*b"name")?;
        let count = usize::from(name.u16(2)?);
        let strings = usize::from(name.u16(4)?);
        let mut records = Vec::new();
        for i in 0..count {
            let at = 6 + 12 * i;
            let ids = [
                name.u16(at)?,
                name.u16(at + 2)?,
                name.u16(at + 4)?,
                name.u16(at + 6)?,
            ];
            if ids[0] != 3 {
                continue;
            }
            let length = usize::from(name.u16(at + 8)?);
            let offset = usize::from(name.u16(at + 10)?);
            let mut value = name.slice(strings + offset, length)?.to_vec();
            if ids[3] == 6 {
                let mut tagged: Vec<u8> = format!("{tag}+")
                    .encode_utf16()
                    .flat_map(u16::to_be_bytes)
                    .collect();
                tagged.extend_from_slice(&value);
                value = tagged;
            }
            records.push((ids, value));
        }
        records.sort();
        let mut out = Vec::new();
        let header = 6 + 12 * records.len();
        out.extend_from_slice(&0u16.to_be_bytes());
        out.extend_from_slice(&u16_len(records.len())?.to_be_bytes());
        out.extend_from_slice(&u16_len(header)?.to_be_bytes());
        let mut storage = Vec::new();
        for (ids, value) in &records {
            for id in ids {
                out.extend_from_slice(&id.to_be_bytes());
            }
            out.extend_from_slice(&u16_len(value.len())?.to_be_bytes());
            out.extend_from_slice(&u16_len(storage.len())?.to_be_bytes());
            storage.extend_from_slice(value);
        }
        out.extend_from_slice(&storage);
        Ok(out)
    }
}

/// Six capital letters derived from the character map, as PDF expects for subset fonts
/// (ISO 32000-1, 9.6.4). Same characters, same tag.
fn subset_tag(cmap: &BTreeMap<u16, u16>) -> String {
    let mut bytes = Vec::new();
    for (code, glyph) in cmap {
        bytes.extend_from_slice(&code.to_be_bytes());
        bytes.extend_from_slice(&glyph.to_be_bytes());
    }
    ContentHash::of(&bytes)
        .as_bytes()
        .iter()
        .take(6)
        .map(|b| char::from(b'A' + b % 26))
        .collect()
}

/// A `cmap` with one format 4 subtable (platform 3, encoding 1): one segment per character
/// plus the required final segment.
fn build_cmap(map: &BTreeMap<u16, u16>) -> Result<Vec<u8>, PdfError> {
    let mut segments: Vec<(u16, u16)> = map
        .iter()
        .filter(|(code, _)| **code != 0xFFFF)
        .map(|(&code, &glyph)| (code, glyph))
        .collect();
    segments.push((0xFFFF, 0));
    let seg_count = u16_len(segments.len())?;
    let mut entry_selector = 0u16;
    while (2u32 << entry_selector) <= u32::from(seg_count) {
        entry_selector += 1;
    }
    let search_range = 2 * (1u16 << entry_selector);
    let length = u16_len(16 + 8 * segments.len())?;
    let mut sub = Vec::new();
    for v in [
        4,
        length,
        0,
        2 * seg_count,
        search_range,
        entry_selector,
        2 * seg_count - search_range,
    ] {
        sub.extend_from_slice(&v.to_be_bytes());
    }
    for (code, _) in &segments {
        sub.extend_from_slice(&code.to_be_bytes());
    }
    sub.extend_from_slice(&0u16.to_be_bytes()); // reservedPad
    for (code, _) in &segments {
        sub.extend_from_slice(&code.to_be_bytes());
    }
    for (code, glyph) in &segments {
        // The final segment maps 0xFFFF to glyph 0: delta 1 wraps it to 0.
        let delta = if *code == 0xFFFF {
            1
        } else {
            glyph.wrapping_sub(*code)
        };
        sub.extend_from_slice(&delta.to_be_bytes());
    }
    for _ in &segments {
        sub.extend_from_slice(&0u16.to_be_bytes());
    }
    let mut out = Vec::new();
    for v in [0u16, 1, 3, 1] {
        out.extend_from_slice(&v.to_be_bytes());
    }
    out.extend_from_slice(&12u32.to_be_bytes());
    out.extend_from_slice(&sub);
    Ok(out)
}

/// Writes the table directory and the tables, and sets `head.checkSumAdjustment`.
fn assemble(tables: &BTreeMap<[u8; 4], Vec<u8>>) -> Result<Vec<u8>, PdfError> {
    let count = u16_len(tables.len())?;
    let mut entry_selector = 0u16;
    while (2u32 << entry_selector) <= u32::from(count) {
        entry_selector += 1;
    }
    let search_range = 16 * (1u16 << entry_selector);
    let mut out = Vec::new();
    out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    for v in [
        count,
        search_range,
        entry_selector,
        count * 16 - search_range,
    ] {
        out.extend_from_slice(&v.to_be_bytes());
    }
    let mut offset = 12 + 16 * tables.len();
    let mut body = Vec::new();
    let mut head_at = None;
    for (tag, data) in tables {
        out.extend_from_slice(tag);
        out.extend_from_slice(&checksum(data).to_be_bytes());
        out.extend_from_slice(&u32_len(offset)?.to_be_bytes());
        out.extend_from_slice(&u32_len(data.len())?.to_be_bytes());
        if tag == b"head" {
            head_at = Some(offset);
        }
        body.extend_from_slice(data);
        pad4(&mut body);
        offset = 12 + 16 * tables.len() + body.len();
    }
    out.extend_from_slice(&body);
    let head_at = head_at.ok_or_else(|| font_error("head missing"))?;
    let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
    out[head_at + 8..head_at + 12].copy_from_slice(&adjustment.to_be_bytes());
    Ok(out)
}

/// TrueType table checksum: sum of big endian u32 words, zero padded.
fn checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

fn pad4(data: &mut Vec<u8>) {
    while !data.len().is_multiple_of(4) {
        data.push(0);
    }
}

fn u16_len(n: usize) -> Result<u16, PdfError> {
    u16::try_from(n).map_err(|_| font_error("table too large"))
}

fn u32_len(n: usize) -> Result<u32, PdfError> {
    u32::try_from(n).map_err(|_| font_error("table too large"))
}

/// Bounds checked big endian reads.
#[derive(Debug, Clone, Copy)]
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn slice(&self, at: usize, len: usize) -> Result<&'a [u8], PdfError> {
        at.checked_add(len)
            .and_then(|end| self.0.get(at..end))
            .ok_or_else(|| font_error("font data truncated"))
    }

    fn array<const N: usize>(&self, at: usize) -> Result<[u8; N], PdfError> {
        let mut out = [0; N];
        out.copy_from_slice(self.slice(at, N)?);
        Ok(out)
    }

    fn u16(&self, at: usize) -> Result<u16, PdfError> {
        self.array(at).map(u16::from_be_bytes)
    }

    fn i16(&self, at: usize) -> Result<i16, PdfError> {
        self.array(at).map(i16::from_be_bytes)
    }

    fn u32(&self, at: usize) -> Result<u32, PdfError> {
        self.array(at).map(u32::from_be_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled() -> TrueTypeFont<'static> {
        TrueTypeFont::parse(BALLOON_FONT).unwrap()
    }

    fn chars(s: &str) -> BTreeSet<char> {
        s.chars().collect()
    }

    #[test]
    fn bundled_font_metrics() {
        let font = bundled();
        assert_eq!(font.units_per_em(), 2048);
        assert_eq!(font.cap_height(), 1462);
        // Digits are tabular: all ten have the same advance.
        let widths: BTreeSet<u16> = ('0'..='9')
            .map(|c| font.advance(font.glyph(c).unwrap()))
            .collect();
        assert_eq!(widths.len(), 1);
        for c in ' '..='~' {
            assert!(font.glyph(c).is_some(), "{c:?}");
        }
        let w = font.text_width("10", 2048.0);
        assert_eq!(w, 2.0 * f64::from(*widths.first().unwrap()));
    }

    #[test]
    fn layout_metrics_never_underestimate_the_font() {
        // The shared balloon layout (D-24) estimates text widths; the real glyphs must fit.
        let font = bundled();
        let em = |c: char| {
            f64::from(font.advance(font.glyph(c).unwrap())) / f64::from(font.units_per_em())
        };
        let metrics = dimo_core::BALLOON_METRICS;
        // Balloon texts are characteristic numbers: digits, later with `.` sub-numbers.
        for c in '0'..='9' {
            assert!(em(c) <= metrics.digit_em, "{c:?}: {}", em(c));
        }
        for c in ['.', ','] {
            assert!(em(c) <= metrics.narrow_em, "{c:?}: {}", em(c));
        }
    }

    #[test]
    fn subset_keeps_metrics_outlines_and_cmap() {
        let font = bundled();
        let set = chars("0123456789.");
        let bytes = font.subset(&set).unwrap();
        assert!(bytes.len() < 12_000, "{}", bytes.len());
        let sub = TrueTypeFont::parse(&bytes).unwrap();
        assert_eq!(sub.num_glyphs, 12);
        assert_eq!(sub.units_per_em(), font.units_per_em());
        assert_eq!(sub.cap_height(), font.cap_height());
        for c in &set {
            let (old, new) = (font.glyph(*c).unwrap(), sub.glyph(*c).unwrap());
            assert_eq!(
                font.metrics[usize::from(old)],
                sub.metrics[usize::from(new)]
            );
            let (a, b) = (font.glyph_data(old), sub.glyph_data(new));
            assert_eq!(a, &b[..a.len()], "{c:?}");
        }
        assert_eq!(sub.glyph('A'), None);
        // The whole file sums to the magic number (head.checkSumAdjustment).
        assert_eq!(checksum(&bytes), 0xB1B0_AFBA);
    }

    #[test]
    fn subset_follows_composite_glyphs() {
        let font = bundled();
        let ch = ('\u{C0}'..='\u{17F}')
            .find(|&c| {
                font.glyph(c).is_some_and(|g| {
                    !TrueTypeFont::component_positions(font.glyph_data(g))
                        .unwrap()
                        .is_empty()
                })
            })
            .unwrap();
        let bytes = font.subset(&BTreeSet::from([ch])).unwrap();
        let sub = TrueTypeFont::parse(&bytes).unwrap();
        let glyph = sub.glyph(ch).unwrap();
        let data = sub.glyph_data(glyph);
        let components = TrueTypeFont::component_positions(data).unwrap();
        assert_ne!(components.len(), 0);
        for at in components {
            let component = Reader(data).u16(at).unwrap();
            assert!(component < sub.num_glyphs);
            assert_ne!(sub.glyph_data(component).len(), 0);
        }
    }

    #[test]
    fn subset_is_deterministic_and_tagged() {
        let font = bundled();
        let a = font.subset(&chars("12")).unwrap();
        assert_eq!(a, font.subset(&chars("21")).unwrap());
        assert_ne!(a, font.subset(&chars("13")).unwrap());
        let name = TrueTypeFont::parse(&a)
            .unwrap()
            .table(*b"name")
            .unwrap()
            .0
            .to_vec();
        let text: String = char::decode_utf16(
            name.chunks(2)
                .filter(|c| c.len() == 2)
                .map(|c| u16::from_be_bytes([c[0], c[1]])),
        )
        .map(|c| c.unwrap_or('?'))
        .collect();
        let tag = subset_tag(&BTreeMap::from([
            (u16::from(b'1'), 1),
            (u16::from(b'2'), 2),
        ]));
        assert!(text.contains(&format!("{tag}+OpenSans-Bold")), "{text}");
    }

    #[test]
    fn missing_glyph_is_an_error() {
        let font = bundled();
        assert!(matches!(
            font.subset(&chars("\u{4E2D}")),
            Err(PdfError::Font(_))
        ));
    }

    #[test]
    fn rejects_non_truetype_data() {
        assert!(TrueTypeFont::parse(b"OTTO\0\0\0\0").is_err());
        assert!(TrueTypeFont::parse(b"").is_err());
    }
}
