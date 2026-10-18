//! Clean up of photographed drawings (R01 decision 11, R.20): grey scale, background made
//! white, contrast raised. Only the kept pixels change; the stored image stays as it is.
//!
//! The paper of a photo is rarely evenly lit. The background is estimated per cell of
//! [`CELL`] pixels as a bright percentile of the cell (lines and text cover far less than the
//! rest), interpolated between cell centres, and every pixel is divided by it. What is then
//! close to the paper becomes white, the rest is stretched to full contrast. Integer math
//! only, so the same pixels give the same result everywhere.

use super::Level;

/// Side of a background cell in pixels of the kept page.
const CELL: usize = 32;

/// Percentile of a cell taken as its background brightness.
const BACKGROUND_PERCENT: usize = 90;

/// Brightness relative to the background (0 to 255) from which a pixel is white.
const WHITE_FROM: u32 = 220;

/// Brightness relative to the background up to which a pixel is black.
const BLACK_UP_TO: u32 = 60;

/// The cleaned page, one grey channel.
pub(super) fn clean_up(level: &Level) -> Level {
    let (w, h) = (level.width as usize, level.height as usize);
    let grey = luminance(level);
    let (cols, rows) = (w.div_ceil(CELL), h.div_ceil(CELL));
    let mut background = Vec::with_capacity(cols * rows);
    let mut histogram = [0u32; 256];
    for cy in 0..rows {
        for cx in 0..cols {
            histogram.fill(0);
            let mut count = 0;
            for y in cy * CELL..((cy + 1) * CELL).min(h) {
                for x in cx * CELL..((cx + 1) * CELL).min(w) {
                    histogram[usize::from(grey[y * w + x])] += 1;
                    count += 1;
                }
            }
            background.push(percentile(&histogram, count, BACKGROUND_PERCENT).max(1));
        }
    }
    let mut pixels = Vec::with_capacity(w * h);
    for y in 0..h {
        let (r0, r1, ty) = between(y, rows);
        for x in 0..w {
            let (c0, c1, tx) = between(x, cols);
            // Bilinear in 1/256 steps.
            let at = |r: usize, c: usize| u32::from(background[r * cols + c]);
            let top = at(r0, c0) * (256 - tx) + at(r0, c1) * tx;
            let bottom = at(r1, c0) * (256 - tx) + at(r1, c1) * tx;
            let bg = ((top * (256 - ty) + bottom * ty) >> 16).max(1);
            let relative = (u32::from(grey[y * w + x]) * 255 / bg).min(255);
            pixels.push(stretch(relative));
        }
    }
    Level {
        width: level.width,
        height: level.height,
        channels: 1,
        pixels,
    }
}

/// White from [`WHITE_FROM`], black up to [`BLACK_UP_TO`], linear between.
fn stretch(relative: u32) -> u8 {
    if relative >= WHITE_FROM {
        255
    } else if relative <= BLACK_UP_TO {
        0
    } else {
        let v = (relative - BLACK_UP_TO) * 255 / (WHITE_FROM - BLACK_UP_TO);
        u8::try_from(v).unwrap_or(u8::MAX)
    }
}

/// The two cell centres around pixel `i` and the weight of the second in 1/256 steps.
fn between(i: usize, cells: usize) -> (usize, usize, u32) {
    // Position in cells, measured from the first cell centre, in 1/256 steps.
    let pos = (i * 256 + 128).saturating_sub(CELL * 128) / CELL;
    let first = (pos / 256).min(cells - 1);
    let second = (first + 1).min(cells - 1);
    let t = if first == second { 0 } else { pos % 256 };
    (first, second, u32::try_from(t).unwrap_or(0))
}

fn percentile(histogram: &[u32; 256], count: u32, percent: usize) -> u8 {
    let wanted = (count as usize * percent).div_ceil(100).max(1);
    let mut seen = 0;
    for (value, &n) in histogram.iter().enumerate() {
        seen += n as usize;
        if seen >= wanted {
            return u8::try_from(value).unwrap_or(u8::MAX);
        }
    }
    u8::MAX
}

/// Grey values of the level, `(299 R + 587 G + 114 B) / 1000` for colour.
pub(super) fn luminance(level: &Level) -> Vec<u8> {
    if level.channels == 1 {
        return level.pixels.clone();
    }
    level
        .pixels
        .chunks_exact(level.channels)
        .map(|p| {
            let v = 299 * u32::from(p[0]) + 587 * u32::from(p[1]) + 114 * u32::from(p[2]);
            u8::try_from((v + 500) / 1000).unwrap_or(u8::MAX)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uneven_paper_becomes_white_and_lines_black() {
        // Paper that darkens from 220 to 120 left to right, with a dark line at x = 40.
        let (w, h) = (128_u32, 64_u32);
        let mut pixels = Vec::new();
        for _ in 0..h {
            for x in 0..w {
                let paper = 220 - x * 100 / w;
                let v = if x == 40 { paper / 4 } else { paper };
                let v = u8::try_from(v).unwrap();
                pixels.extend([v, v, v]);
            }
        }
        let level = Level {
            width: w,
            height: h,
            channels: 3,
            pixels,
        };
        let out = clean_up(&level);
        assert_eq!(out.channels, 1);
        let at = |x: usize, y: usize| out.pixels[y * w as usize + x];
        assert_eq!(at(5, 10), 255);
        assert_eq!(at(120, 50), 255, "dark paper is white too");
        assert_eq!(at(40, 30), 0);
        assert_eq!(
            clean_up(&level).pixels,
            out.pixels,
            "same pixels every time"
        );
    }
}
