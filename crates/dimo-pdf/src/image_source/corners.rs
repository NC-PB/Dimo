//! Finding the sheet in a photo for the corner correction (R01 decision 8, R.20).
//!
//! A drawing photographed on a table is a large bright area on a darker background. On a small
//! reduction of the page the grey values are split by Otsu's threshold, the largest bright
//! 4-connected area is taken as the sheet, and its corners are the area's extreme points along the
//! two diagonals. If the area is too small, fills almost the whole photo, reaches three or four
//! sides of it, or its corners do not make a usable quadrilateral, nothing is proposed and the
//! editor starts at the image corners.

// Pixel counts and coordinates of a reduction of at most 600 pixels a side are far below the
// 52 bit mantissa and the i64 range.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::many_single_char_names
)]

use dimo_core::Point;

use super::Level;
use super::clean::luminance;
use super::geometry::PageInfo;

/// Longest side of the reduction searched.
const SEARCH_SIDE: u32 = 600;

/// Widest dark line, in pixels of the searched reduction, that does not split the sheet: a
/// 0.7 mm frame line of an A4 sheet filling the photo is about 2 pixels there.
const LINE_GAP: usize = 4;

/// Smallest and largest part of the photo the sheet may cover.
const MIN_COVER: f64 = 0.2;
const MAX_COVER: f64 = 0.98;

/// Corners of the sheet in upright page pixels, clockwise from the top left of the page.
pub(super) fn propose(levels: &[Level], block: f64, info: &PageInfo) -> Option<[Point; 4]> {
    let (k, level) = levels
        .iter()
        .enumerate()
        .find(|(_, l)| l.width.max(l.height) <= SEARCH_SIDE)
        .or_else(|| levels.iter().enumerate().next_back())?;
    let (w, h) = (level.width as usize, level.height as usize);
    if w < 8 || h < 8 {
        return None;
    }
    let grey = luminance(level);
    let threshold = otsu(&grey);
    let bright: Vec<bool> = grey.iter().map(|&v| v > threshold).collect();
    // The lines of the drawing (frame, title block) are dark and would cut the sheet into
    // parts; closing the bright area bridges them.
    let bright = close(&bright, w, h, LINE_GAP);
    let sheet = largest_area(&bright, w, h)?;
    let cover = sheet.len() as f64 / (w * h) as f64;
    if !(MIN_COVER..=MAX_COVER).contains(&cover) || edges_touched(&sheet, w, h) >= 3 {
        return None;
    }
    let key = |f: &dyn Fn(i64, i64) -> i64| {
        sheet
            .iter()
            .map(|&i| (i % w, i / w))
            .max_by_key(|&(x, y)| f(x as i64, y as i64))
    };
    let tl = key(&|x, y| -(x + y))?;
    let tr = key(&|x, y| x - y)?;
    let br = key(&|x, y| x + y)?;
    let bl = key(&|x, y| y - x)?;
    let scale = block * f64::from(1_u32 << k.min(31));
    let (full_w, full_h) = (f64::from(info.width), f64::from(info.height));
    let to_page = |(x, y): (usize, usize)| Point {
        x: ((x as f64 + 0.5) * scale).clamp(0.0, full_w),
        y: ((y as f64 + 0.5) * scale).clamp(0.0, full_h),
    };
    let quad = [to_page(tl), to_page(tr), to_page(br), to_page(bl)];
    usable(&quad, full_w, full_h).then_some(quad)
}

/// How many sides of the photo the area reaches. A sheet whose corners are all in view
/// reaches at most two; more means the bright area is not a whole sheet.
fn edges_touched(area: &[usize], w: usize, h: usize) -> usize {
    let (mut left, mut right, mut top, mut bottom) = (false, false, false, false);
    for &i in area {
        let (x, y) = (i % w, i / w);
        left |= x == 0;
        right |= x + 1 == w;
        top |= y == 0;
        bottom |= y + 1 == h;
    }
    [left, right, top, bottom]
        .into_iter()
        .filter(|&t| t)
        .count()
}

/// Convex, clockwise, and every side at least a tenth of the photo.
fn usable(quad: &[Point; 4], w: f64, h: f64) -> bool {
    let min_side = 0.1 * w.min(h);
    (0..4).all(|i| {
        let (a, b, c) = (quad[i], quad[(i + 1) % 4], quad[(i + 2) % 4]);
        let side = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        // With y downward a clockwise turn has a positive cross product.
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        side >= min_side && cross > 0.0
    })
}

/// Otsu's threshold: the grey value that best splits the histogram into two classes.
fn otsu(grey: &[u8]) -> u8 {
    let mut histogram = [0u64; 256];
    for &v in grey {
        histogram[usize::from(v)] += 1;
    }
    let total = grey.len() as f64;
    let sum: f64 = histogram
        .iter()
        .enumerate()
        .map(|(v, &n)| v as f64 * n as f64)
        .sum();
    let (mut below, mut below_sum) = (0.0, 0.0);
    let (mut best, mut best_value) = (0_u8, -1.0);
    for (v, &n) in histogram.iter().enumerate() {
        below += n as f64;
        below_sum += v as f64 * n as f64;
        let above = total - below;
        if below == 0.0 || above == 0.0 {
            continue;
        }
        let diff = below_sum / below - (sum - below_sum) / above;
        let between = below * above * diff * diff;
        if between > best_value {
            best_value = between;
            best = u8::try_from(v).unwrap_or(u8::MAX);
        }
    }
    best
}

/// Morphological closing with a square of side `2 r + 1`: dilation, then erosion, each done
/// as a row pass and a column pass.
fn close(mask: &[bool], w: usize, h: usize, r: usize) -> Vec<bool> {
    let dilated = pass(&pass(mask, w, h, r, true, true), w, h, r, false, true);
    pass(&pass(&dilated, w, h, r, true, false), w, h, r, false, false)
}

/// One pass of a square filter along rows (`rows`) or columns: any `true` within `r` gives
/// `true` (`grow`), or all within `r` must be `true` (shrink; outside the photo counts as
/// `true`, so the photo edge does not eat the sheet).
fn pass(mask: &[bool], w: usize, h: usize, r: usize, rows: bool, grow: bool) -> Vec<bool> {
    let (len, lines) = if rows { (w, h) } else { (h, w) };
    let at = |line: usize, i: usize| if rows { line * w + i } else { i * w + line };
    let mut out = vec![false; mask.len()];
    for line in 0..lines {
        // Running count of `true` cells in the window.
        let mut count = 0usize;
        for i in 0..r.min(len) {
            count += usize::from(mask[at(line, i)]);
        }
        for i in 0..len {
            if i + r < len {
                count += usize::from(mask[at(line, i + r)]);
            }
            if i > r {
                count -= usize::from(mask[at(line, i - r - 1)]);
            }
            let start = i.saturating_sub(r);
            let end = (i + r).min(len - 1);
            let inside = end - start + 1;
            let missing = 2 * r + 1 - inside;
            out[at(line, i)] = if grow {
                count > 0
            } else {
                count + missing == 2 * r + 1
            };
        }
    }
    out
}

/// Pixel indexes of the largest 4-connected area of `true` pixels.
fn largest_area(mask: &[bool], w: usize, h: usize) -> Option<Vec<usize>> {
    let mut seen = vec![false; mask.len()];
    let mut best: Vec<usize> = Vec::new();
    let mut stack = Vec::new();
    for start in 0..mask.len() {
        if !mask[start] || seen[start] {
            continue;
        }
        let mut area = Vec::new();
        seen[start] = true;
        stack.push(start);
        while let Some(i) = stack.pop() {
            area.push(i);
            let (x, y) = (i % w, i / w);
            let mut visit = |j: usize| {
                if mask[j] && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            };
            if x > 0 {
                visit(i - 1);
            }
            if x + 1 < w {
                visit(i + 1);
            }
            if y > 0 {
                visit(i - w);
            }
            if y + 1 < h {
                visit(i + w);
            }
        }
        if area.len() > best.len() {
            best = area;
        }
    }
    (!best.is_empty()).then_some(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bright quadrilateral on a dark table, tested with the point in polygon rule.
    fn photo(w: u32, h: u32, quad: [Point; 4]) -> Level {
        let inside = |x: f64, y: f64| {
            (0..4).all(|i| {
                let (a, b) = (quad[i], quad[(i + 1) % 4]);
                (b.x - a.x) * (y - a.y) - (b.y - a.y) * (x - a.x) >= 0.0
            })
        };
        let mut pixels = Vec::new();
        for y in 0..h {
            for x in 0..w {
                let v = if inside(f64::from(x) + 0.5, f64::from(y) + 0.5) {
                    235
                } else {
                    70
                };
                pixels.push(v);
            }
        }
        Level {
            width: w,
            height: h,
            channels: 1,
            pixels,
        }
    }

    #[test]
    fn finds_the_corners_of_a_skewed_sheet() {
        let quad = [
            Point { x: 60.0, y: 40.0 },
            Point { x: 330.0, y: 60.0 },
            Point { x: 310.0, y: 250.0 },
            Point { x: 40.0, y: 230.0 },
        ];
        let level = photo(400, 300, quad);
        let info = PageInfo {
            width: 800,
            height: 600,
            dpi: None,
        };
        let found = propose(&[level], 2.0, &info).unwrap();
        for (f, q) in found.iter().zip(quad) {
            assert!(
                (f.x - q.x * 2.0).abs() < 8.0 && (f.y - q.y * 2.0).abs() < 8.0,
                "{f:?} {q:?}"
            );
        }
    }

    #[test]
    fn frame_lines_do_not_split_the_sheet() {
        let quad = [
            Point { x: 40.0, y: 30.0 },
            Point { x: 360.0, y: 40.0 },
            Point { x: 350.0, y: 270.0 },
            Point { x: 30.0, y: 260.0 },
        ];
        let mut level = photo(400, 300, quad);
        // A dark frame 15 px inside the sheet edge and a title block line, 2 px wide.
        for y in 45..255 {
            for x in 55..345 {
                let on_frame = (55..57).contains(&x) || (343..345).contains(&x);
                let on_rows = (45..47).contains(&y) || (253..255).contains(&y);
                let on_block = (200..202).contains(&y) && x > 200;
                if on_frame || on_rows || on_block {
                    level.pixels[y * 400 + x] = 10;
                }
            }
        }
        let info = PageInfo {
            width: 400,
            height: 300,
            dpi: None,
        };
        let found = propose(&[level], 1.0, &info).unwrap();
        for (f, q) in found.iter().zip(quad) {
            assert!(
                (f.x - q.x).abs() < 4.0 && (f.y - q.y).abs() < 4.0,
                "{f:?} {q:?}"
            );
        }
    }

    #[test]
    fn proposes_nothing_without_a_clear_sheet() {
        let info = PageInfo {
            width: 100,
            height: 100,
            dpi: None,
        };
        let flat = Level {
            width: 100,
            height: 100,
            channels: 1,
            pixels: vec![200; 10_000],
        };
        assert!(propose(&[flat], 1.0, &info).is_none());
    }
}
