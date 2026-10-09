//! Tile addressing: zoom levels, the tile grid of a sheet and tile routes.

use std::ops::Range;

use crate::geometry::{SheetRect, SheetSize};
use crate::hash::ContentHash;

use super::TileError;

/// Width and height of a full tile in pixels. Tiles in the last column or row are cut at the
/// sheet edge and can be smaller.
pub const TILE_SIZE: u32 = 512;

/// Smallest zoom level: 1/16 pixel per sheet unit (4.5 dpi). An A0 sheet fits in one tile.
pub const MIN_ZOOM: i32 = -4;

/// Largest zoom level: 32 pixels per sheet unit (2304 dpi). Deeper zoom is CSS scaling in the
/// viewport; at this level a 0.1 mm line is already about 9 pixels wide.
pub const MAX_ZOOM: i32 = 5;

/// Pixels per sheet unit at a zoom level: `2^level`. Level 0 is 72 dpi.
pub fn zoom_scale(level: i32) -> f64 {
    2f64.powi(level)
}

/// Address of one tile: document, sheet, zoom level and column and row in the tile grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileKey {
    /// SHA-256 of the document bytes (FR-DOC-07). Also the disk cache key.
    pub doc: ContentHash,
    /// Zero based sheet index.
    pub sheet: u32,
    /// Zoom level, see [`zoom_scale`].
    pub zoom: i32,
    /// Column, zero at the left sheet edge.
    pub x: u32,
    /// Row, zero at the top sheet edge.
    pub y: u32,
}

impl TileKey {
    /// Parses the route of a tile URL: `tile/{doc}/{sheet}/{zoom}/{x}/{y}`, where `doc` is the
    /// 64 digit hex content hash. A leading `/` is ignored.
    pub fn from_route(route: &str) -> Result<Self, TileError> {
        let invalid = || TileError::InvalidAddress(route.to_owned());
        let mut parts = route.trim_start_matches('/').split('/');
        if parts.next() != Some("tile") {
            return Err(invalid());
        }
        let doc = parts
            .next()
            .and_then(ContentHash::from_hex)
            .ok_or_else(invalid)?;
        let mut number = || parts.next().ok_or_else(invalid);
        let sheet = number()?.parse().map_err(|_| invalid())?;
        let zoom = number()?.parse().map_err(|_| invalid())?;
        let x = number()?.parse().map_err(|_| invalid())?;
        let y = number()?.parse().map_err(|_| invalid())?;
        if parts.next().is_some() {
            return Err(invalid());
        }
        Ok(Self {
            doc,
            sheet,
            zoom,
            x,
            y,
        })
    }

    /// Parses a tile URL from its host and path, in every form a webview produces for the
    /// `dimo` scheme:
    ///
    /// - macOS and Linux: `dimo://localhost/tile/...` (host `localhost`, path `/tile/...`)
    /// - Windows and Android: `http://dimo.localhost/tile/...` (host `dimo.localhost`)
    /// - the short form of the architecture spec, `dimo://tile/...` (host `tile`)
    pub fn from_url_parts(host: Option<&str>, path: &str) -> Result<Self, TileError> {
        if host == Some("tile") {
            Self::from_route(&format!("tile{path}"))
        } else {
            Self::from_route(path)
        }
    }

    /// The route of this tile, the inverse of [`TileKey::from_route`].
    pub fn route(&self) -> String {
        format!(
            "tile/{}/{}/{}/{}/{}",
            self.doc, self.sheet, self.zoom, self.x, self.y
        )
    }
}

/// A rectangle of tiles on one sheet at one zoom level. Columns and rows are half open ranges.
///
/// Used to declare which tiles the viewport still wants (see [`super::TileService::set_interest`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileRange {
    /// Zero based sheet index.
    pub sheet: u32,
    /// Zoom level.
    pub zoom: i32,
    /// Columns, start inclusive, end exclusive.
    pub columns: Range<u32>,
    /// Rows, start inclusive, end exclusive.
    pub rows: Range<u32>,
}

impl TileRange {
    /// Whether the range covers the tile (the document is not compared).
    pub fn contains(&self, key: &TileKey) -> bool {
        key.sheet == self.sheet
            && key.zoom == self.zoom
            && self.columns.contains(&key.x)
            && self.rows.contains(&key.y)
    }
}

/// The tile grid of one sheet at one zoom level.
///
/// The sheet covers `width_px` by `height_px` pixels (sheet size times scale, rounded up).
/// Tile `(x, y)` covers pixels `x * 512 .. min((x + 1) * 512, width_px)` and the same for rows,
/// so pixel `(0, 0)` of every tile sits on a whole pixel of the sheet raster and tiles join
/// without seams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileGrid {
    zoom: i32,
    width_px: u32,
    height_px: u32,
}

impl TileGrid {
    /// The grid of a sheet of `size` at `zoom`. Fails for zoom levels outside
    /// [`MIN_ZOOM`]`..=`[`MAX_ZOOM`] and for sheets without a positive finite size.
    pub fn new(size: SheetSize, zoom: i32) -> Result<Self, TileError> {
        if !(MIN_ZOOM..=MAX_ZOOM).contains(&zoom) {
            return Err(TileError::OutOfRange(format!(
                "zoom level {zoom} outside {MIN_ZOOM}..={MAX_ZOOM}"
            )));
        }
        let scale = zoom_scale(zoom);
        let pixels = |units: f64| -> Result<u32, TileError> {
            let px = (units * scale).ceil();
            if !(px.is_finite() && px >= 1.0 && px <= f64::from(u32::MAX)) {
                return Err(TileError::OutOfRange(format!(
                    "sheet size {units} cannot be tiled at zoom {zoom}"
                )));
            }
            // Whole number in 1..=u32::MAX, checked above.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            Ok(px as u32)
        };
        Ok(Self {
            zoom,
            width_px: pixels(size.width)?,
            height_px: pixels(size.height)?,
        })
    }

    /// Zoom level of the grid.
    pub const fn zoom(&self) -> i32 {
        self.zoom
    }

    /// Width of the sheet raster in pixels.
    pub const fn width_px(&self) -> u32 {
        self.width_px
    }

    /// Height of the sheet raster in pixels.
    pub const fn height_px(&self) -> u32 {
        self.height_px
    }

    /// Number of tile columns.
    pub const fn columns(&self) -> u32 {
        self.width_px.div_ceil(TILE_SIZE)
    }

    /// Number of tile rows.
    pub const fn rows(&self) -> u32 {
        self.height_px.div_ceil(TILE_SIZE)
    }

    /// Pixel size of tile `(x, y)`, or `None` outside the grid.
    pub fn tile_pixels(&self, x: u32, y: u32) -> Option<(u32, u32)> {
        if x >= self.columns() || y >= self.rows() {
            return None;
        }
        let width = (self.width_px - x * TILE_SIZE).min(TILE_SIZE);
        let height = (self.height_px - y * TILE_SIZE).min(TILE_SIZE);
        Some((width, height))
    }

    /// The sheet region tile `(x, y)` shows, or `None` outside the grid. Rendered at
    /// [`zoom_scale`]`(zoom)` it yields exactly [`TileGrid::tile_pixels`]: all values are
    /// multiples of a power of two, so the float arithmetic is exact.
    pub fn tile_region(&self, x: u32, y: u32) -> Option<SheetRect> {
        let (width, height) = self.tile_pixels(x, y)?;
        let scale = zoom_scale(self.zoom);
        Some(SheetRect::new(
            f64::from(x * TILE_SIZE) / scale,
            f64::from(y * TILE_SIZE) / scale,
            f64::from(width) / scale,
            f64::from(height) / scale,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A4: SheetSize = SheetSize {
        width: 841.89,
        height: 595.276,
    };

    fn hash() -> ContentHash {
        ContentHash::of(b"tile")
    }

    #[test]
    fn grid_of_a4_landscape() {
        let g = TileGrid::new(A4, 0).unwrap();
        assert_eq!((g.width_px(), g.height_px()), (842, 596));
        assert_eq!((g.columns(), g.rows()), (2, 2));
        assert_eq!(g.tile_pixels(0, 0), Some((512, 512)));
        assert_eq!(g.tile_pixels(1, 1), Some((330, 84)));
        assert_eq!(g.tile_pixels(2, 0), None);

        let g = TileGrid::new(A4, 2).unwrap();
        assert_eq!((g.width_px(), g.height_px()), (3368, 2382));
        assert_eq!((g.columns(), g.rows()), (7, 5));

        let g = TileGrid::new(A4, MIN_ZOOM).unwrap();
        assert_eq!((g.columns(), g.rows()), (1, 1));
        assert_eq!(g.tile_pixels(0, 0), Some((53, 38)));
    }

    #[test]
    fn zoom_out_of_range() {
        assert!(TileGrid::new(A4, MIN_ZOOM - 1).is_err());
        assert!(TileGrid::new(A4, MAX_ZOOM + 1).is_err());
        assert!(TileGrid::new(A4, MAX_ZOOM).is_ok());
    }

    #[test]
    fn regions_tile_the_sheet_exactly() {
        for zoom in MIN_ZOOM..=MAX_ZOOM {
            let g = TileGrid::new(A4, zoom).unwrap();
            let scale = zoom_scale(zoom);
            for x in 0..g.columns() {
                for y in [0, g.rows() - 1] {
                    let r = g.tile_region(x, y).unwrap();
                    let (w, h) = g.tile_pixels(x, y).unwrap();
                    // Exact: offsets and sizes are whole pixels at this scale.
                    assert_eq!(r.x * scale, f64::from(x * TILE_SIZE));
                    assert_eq!(r.y * scale, f64::from(y * TILE_SIZE));
                    assert_eq!(r.width * scale, f64::from(w));
                    assert_eq!(r.height * scale, f64::from(h));
                }
            }
            // The last tile ends at the rounded up sheet edge.
            let last = g.tile_region(g.columns() - 1, 0).unwrap();
            assert!(last.x + last.width >= A4.width);
            assert!(last.x + last.width - A4.width < 1.0 / scale);
        }
    }

    #[test]
    fn route_round_trip() {
        let key = TileKey {
            doc: hash(),
            sheet: 3,
            zoom: -2,
            x: 10,
            y: 7,
        };
        let route = key.route();
        assert_eq!(TileKey::from_route(&route).unwrap(), key);
        assert_eq!(TileKey::from_route(&format!("/{route}")).unwrap(), key);
    }

    #[test]
    fn url_forms_of_all_platforms() {
        let key = TileKey {
            doc: hash(),
            sheet: 0,
            zoom: 1,
            x: 2,
            y: 3,
        };
        let path = format!("/tile/{}/0/1/2/3", hash());
        assert_eq!(
            TileKey::from_url_parts(Some("localhost"), &path).unwrap(),
            key
        );
        assert_eq!(
            TileKey::from_url_parts(Some("dimo.localhost"), &path).unwrap(),
            key
        );
        let short = format!("/{}/0/1/2/3", hash());
        assert_eq!(TileKey::from_url_parts(Some("tile"), &short).unwrap(), key);
    }

    #[test]
    fn bad_routes() {
        let h = hash();
        for route in [
            String::new(),
            "tile".to_owned(),
            format!("tiles/{h}/0/0/0/0"),
            format!("tile/{h}/0/0/0"),
            format!("tile/{h}/0/0/0/0/0"),
            format!("tile/{h}/-1/0/0/0"),
            format!("tile/{h}/0/0/x/0"),
            "tile/abc/0/0/0/0".to_owned(),
        ] {
            assert!(
                matches!(
                    TileKey::from_route(&route),
                    Err(TileError::InvalidAddress(_))
                ),
                "{route}"
            );
        }
    }

    #[test]
    fn range_contains() {
        let range = TileRange {
            sheet: 0,
            zoom: 1,
            columns: 2..4,
            rows: 0..1,
        };
        let key = |sheet, zoom, x, y| TileKey {
            doc: hash(),
            sheet,
            zoom,
            x,
            y,
        };
        assert!(range.contains(&key(0, 1, 2, 0)));
        assert!(range.contains(&key(0, 1, 3, 0)));
        assert!(!range.contains(&key(0, 1, 4, 0)));
        assert!(!range.contains(&key(0, 1, 2, 1)));
        assert!(!range.contains(&key(1, 1, 2, 0)));
        assert!(!range.contains(&key(0, 0, 2, 0)));
    }
}
