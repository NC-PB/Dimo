//! Sheet space geometry (AGENTS.md rule 4).
//!
//! Sheet space uses PDF user units (1/72 inch), origin at the top left corner of the sheet,
//! x to the right, y downward. The sheet is the page as PDFium displays it: crop box applied,
//! page rotation applied. Geometry is screen geometry, so `f64` is used, never for tolerances.

/// Width and height of a sheet in sheet units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetSize {
    /// Width in PDF user units.
    pub width: f64,
    /// Height in PDF user units.
    pub height: f64,
}

/// An axis aligned rectangle in sheet space. `x`, `y` is the top left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SheetRect {
    /// Left edge in sheet units.
    pub x: f64,
    /// Top edge in sheet units.
    pub y: f64,
    /// Width in sheet units.
    pub width: f64,
    /// Height in sheet units.
    pub height: f64,
}

impl SheetRect {
    /// Creates a rectangle from its top left corner and size.
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The rectangle covering a whole sheet.
    pub const fn full(size: SheetSize) -> Self {
        Self::new(0.0, 0.0, size.width, size.height)
    }
}
