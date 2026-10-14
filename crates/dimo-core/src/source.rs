//! Where the sheets of a drawing revision come from (MF F1, ADR 0010).
//!
//! A revision is either a PDF, whose pages are the sheets, or a photo or scan (JPEG, PNG or
//! TIFF, one sheet per page, R01 decision 7). An image is stored unchanged; how it is turned
//! into a sheet is kept as non-destructive [`ImageEdit`] settings that can be changed later
//! (R01 decisions 8 to 11). Image revisions are part of the data model from schema 4 on, but
//! they cannot be created yet (release 0.1, R.19 to R.21).
//!
//! # Image edit coordinates
//!
//! Edit settings are not sheet geometry (rule 4 applies to the sheet the edit produces). They
//! are measured in pixels of the stored image, origin at its top left corner, x to the right,
//! y downward, as exact decimals (rule 5), so a saved edit gives the same sheet on every
//! machine. The edit is applied in this order: quarter turns, fine angle, four-corner
//! correction, trim, paper format.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::decimal::decimal_schema;
use crate::sheet::Rotation;

/// The kind of file a drawing revision was imported from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DrawingSource {
    /// A PDF; sheets are its pages.
    Pdf,
    /// A photo or scan; sheets are its pages (one for JPEG and PNG).
    Image {
        /// File format of the stored image.
        format: ImageFormat,
        /// How the image becomes a sheet.
        edit: Box<ImageEdit>,
    },
}

impl DrawingSource {
    /// File name extension of the stored file, without the dot: the file is stored as
    /// `drawings/<sha256>.<extension>`.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Image { format, .. } => format.extension(),
        }
    }

    /// The image edit settings, `None` for a PDF.
    pub fn image_edit(&self) -> Option<&ImageEdit> {
        match self {
            Self::Pdf => None,
            Self::Image { edit, .. } => Some(edit),
        }
    }
}

/// Image file formats a drawing can be imported from (R01 decision 7). HEIC is not supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ImageFormat {
    /// JPEG, one sheet.
    Jpeg,
    /// PNG, one sheet.
    Png,
    /// TIFF, one sheet per page.
    Tiff,
}

impl ImageFormat {
    /// Every format, in a fixed order.
    pub const ALL: [Self; 3] = [Self::Jpeg, Self::Png, Self::Tiff];

    /// File name extension of the stored file, without the dot.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Tiff => "tiff",
        }
    }
}

/// Non-destructive settings that turn a stored image into a sheet (R01 decisions 8 to 11).
/// See the module docs for coordinates and order. [`ImageEdit::default`] leaves the image as
/// it is.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ImageEdit {
    /// Clockwise quarter turns.
    pub rotation: Rotation,
    /// Further clockwise rotation in degrees, from -45 to 45, applied after the quarter turns.
    #[serde(with = "crate::decimal::serde_str")]
    #[schemars(schema_with = "decimal_schema")]
    #[cfg_attr(feature = "specta", specta(type = String))]
    pub fine_angle: Decimal,
    /// Sheet corners in the stored image for perspective correction (R01 decision 8); `null`
    /// for none.
    pub corners: Option<SheetCorners>,
    /// Part of the image to keep, in pixels of the image after rotation and corner correction;
    /// `null` keeps all of it.
    pub trim: Option<ImageRect>,
    /// Paper the trimmed image is mapped onto (R01 decision 9); `null` until the user picks it.
    pub paper: Option<Paper>,
    /// Grey scale, white background and raised contrast for display and the ballooned PDF
    /// (R01 decision 11). The stored image stays as it is.
    pub clean_up: bool,
}

/// Largest fine angle in degrees, either way.
const MAX_FINE_ANGLE: Decimal = Decimal::from_parts(45, 0, 0, false, 0);

/// Largest side of a custom paper size in mm.
const MAX_PAPER_MM: Decimal = Decimal::from_parts(10_000, 0, 0, false, 0);

impl ImageEdit {
    /// Checks the ranges: fine angle from -45 to 45 degrees, no negative pixel coordinates,
    /// a trim and a custom paper size with a positive size, custom paper at most 10 m a side.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.fine_angle.abs() > MAX_FINE_ANGLE {
            return Err("the fine angle is between -45 and 45 degrees");
        }
        if let Some(corners) = &self.corners
            && corners.points().iter().any(|p| !p.is_valid())
        {
            return Err("corner points need coordinates of 0 or more");
        }
        if let Some(trim) = &self.trim
            && !trim.is_valid()
        {
            return Err("the trim needs a position of 0 or more and a positive size");
        }
        if let Some(Paper::Custom {
            width_mm,
            height_mm,
        }) = &self.paper
        {
            let ok = |v: &Decimal| *v > Decimal::ZERO && *v <= MAX_PAPER_MM;
            if !ok(width_mm) || !ok(height_mm) {
                return Err("a custom paper size is more than 0 and at most 10000 mm a side");
            }
        }
        Ok(())
    }
}

/// A point in pixels of the stored image (see the module docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ImagePoint {
    /// Pixels from the left edge.
    #[serde(with = "crate::decimal::serde_str")]
    #[schemars(schema_with = "decimal_schema")]
    #[cfg_attr(feature = "specta", specta(type = String))]
    pub x: Decimal,
    /// Pixels from the top edge.
    #[serde(with = "crate::decimal::serde_str")]
    #[schemars(schema_with = "decimal_schema")]
    #[cfg_attr(feature = "specta", specta(type = String))]
    pub y: Decimal,
}

impl ImagePoint {
    fn is_valid(self) -> bool {
        self.x >= Decimal::ZERO && self.y >= Decimal::ZERO
    }
}

/// The four sheet corners in the stored image, named as the sheet reads after the quarter
/// turns (R01 decision 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct SheetCorners {
    /// Top left corner of the sheet.
    pub top_left: ImagePoint,
    /// Top right corner of the sheet.
    pub top_right: ImagePoint,
    /// Bottom right corner of the sheet.
    pub bottom_right: ImagePoint,
    /// Bottom left corner of the sheet.
    pub bottom_left: ImagePoint,
}

impl SheetCorners {
    /// The corners clockwise from the top left.
    pub const fn points(&self) -> [ImagePoint; 4] {
        [
            self.top_left,
            self.top_right,
            self.bottom_right,
            self.bottom_left,
        ]
    }
}

/// An axis parallel rectangle in image pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ImageRect {
    /// Left edge.
    #[serde(with = "crate::decimal::serde_str")]
    #[schemars(schema_with = "decimal_schema")]
    #[cfg_attr(feature = "specta", specta(type = String))]
    pub x: Decimal,
    /// Top edge.
    #[serde(with = "crate::decimal::serde_str")]
    #[schemars(schema_with = "decimal_schema")]
    #[cfg_attr(feature = "specta", specta(type = String))]
    pub y: Decimal,
    /// Width, more than 0.
    #[serde(with = "crate::decimal::serde_str")]
    #[schemars(schema_with = "decimal_schema")]
    #[cfg_attr(feature = "specta", specta(type = String))]
    pub width: Decimal,
    /// Height, more than 0.
    #[serde(with = "crate::decimal::serde_str")]
    #[schemars(schema_with = "decimal_schema")]
    #[cfg_attr(feature = "specta", specta(type = String))]
    pub height: Decimal,
}

impl ImageRect {
    fn is_valid(&self) -> bool {
        self.x >= Decimal::ZERO
            && self.y >= Decimal::ZERO
            && self.width > Decimal::ZERO
            && self.height > Decimal::ZERO
    }
}

/// The paper a photographed sheet is mapped onto (R01 decision 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum Paper {
    /// A standard paper size.
    Standard {
        /// The size.
        size: PaperSize,
        /// Portrait or landscape.
        orientation: Orientation,
    },
    /// Width and height in mm as the sheet reads.
    Custom {
        /// Width in mm.
        #[serde(with = "crate::decimal::serde_str")]
        #[schemars(schema_with = "decimal_schema")]
        #[cfg_attr(feature = "specta", specta(type = String))]
        width_mm: Decimal,
        /// Height in mm.
        #[serde(with = "crate::decimal::serde_str")]
        #[schemars(schema_with = "decimal_schema")]
        #[cfg_attr(feature = "specta", specta(type = String))]
        height_mm: Decimal,
    },
}

/// Standard paper sizes (R01 decision 9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum PaperSize {
    /// ISO 216 A4, 210 x 297 mm.
    A4,
    /// ISO 216 A3, 297 x 420 mm.
    A3,
    /// ISO 216 A2, 420 x 594 mm.
    A2,
    /// ISO 216 A1, 594 x 841 mm.
    A1,
    /// ISO 216 A0, 841 x 1189 mm.
    A0,
    /// US Letter, 8.5 x 11 in.
    Letter,
    /// US Tabloid, 11 x 17 in.
    Tabloid,
}

/// Paper orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    /// Taller than wide.
    Portrait,
    /// Wider than tall.
    Landscape,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn dec(text: &str) -> Decimal {
        text.parse().unwrap()
    }

    #[test]
    fn pdf_and_image_sources_serialize_readably() {
        assert_eq!(
            serde_json::to_value(DrawingSource::Pdf).unwrap(),
            json!({ "type": "pdf" })
        );
        let image = DrawingSource::Image {
            format: ImageFormat::Jpeg,
            edit: Box::new(ImageEdit {
                rotation: Rotation::Deg90,
                fine_angle: dec("-1.25"),
                corners: None,
                trim: Some(ImageRect {
                    x: dec("10"),
                    y: dec("20"),
                    width: dec("3000"),
                    height: dec("2000.5"),
                }),
                paper: Some(Paper::Standard {
                    size: PaperSize::A3,
                    orientation: Orientation::Landscape,
                }),
                clean_up: true,
            }),
        };
        let value = serde_json::to_value(&image).unwrap();
        assert_eq!(
            value,
            json!({
                "type": "image", "format": "jpeg",
                "edit": {
                    "rotation": "deg90", "fine_angle": "-1.25", "corners": null,
                    "trim": { "x": "10", "y": "20", "width": "3000", "height": "2000.5" },
                    "paper": { "format": "standard", "size": "a3", "orientation": "landscape" },
                    "clean_up": true
                }
            })
        );
        let back: DrawingSource = serde_json::from_value(value).unwrap();
        assert_eq!(back, image);
        assert_eq!(image.extension(), "jpg");
        assert_eq!(DrawingSource::Pdf.extension(), "pdf");
        let custom = Paper::Custom {
            width_mm: dec("100"),
            height_mm: dec("150.5"),
        };
        assert_eq!(
            serde_json::to_value(custom).unwrap(),
            json!({ "format": "custom", "width_mm": "100", "height_mm": "150.5" })
        );
    }

    #[test]
    fn extensions_are_distinct() {
        let mut all: Vec<&str> = ImageFormat::ALL.iter().map(|f| f.extension()).collect();
        all.push(DrawingSource::Pdf.extension());
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), 4);
    }

    #[test]
    fn edit_ranges_are_checked() {
        assert_eq!(ImageEdit::default().validate(), Ok(()));
        let angle = |a: &str| ImageEdit {
            fine_angle: dec(a),
            ..ImageEdit::default()
        };
        assert!(angle("45").validate().is_ok());
        assert!(angle("-45.0").validate().is_ok());
        assert!(angle("45.01").validate().is_err());
        let point = |x: &str| ImagePoint {
            x: dec(x),
            y: dec("0"),
        };
        let corners = |x: &str| ImageEdit {
            corners: Some(SheetCorners {
                top_left: point("0"),
                top_right: point(x),
                bottom_right: point("5"),
                bottom_left: point("0"),
            }),
            ..ImageEdit::default()
        };
        assert!(corners("10").validate().is_ok());
        assert!(corners("-1").validate().is_err());
        let trim = |w: &str| ImageEdit {
            trim: Some(ImageRect {
                x: dec("0"),
                y: dec("0"),
                width: dec(w),
                height: dec("1"),
            }),
            ..ImageEdit::default()
        };
        assert!(trim("1").validate().is_ok());
        assert!(trim("0").validate().is_err());
        let paper = |w: &str| ImageEdit {
            paper: Some(Paper::Custom {
                width_mm: dec(w),
                height_mm: dec("297"),
            }),
            ..ImageEdit::default()
        };
        assert!(paper("210").validate().is_ok());
        assert!(paper("0").validate().is_err());
        assert!(paper("10000.1").validate().is_err());
    }
}
