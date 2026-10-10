//! Zone grid defaults and label schemes (T2.7, T2.7a, M2 decision 1, D-21).
//!
//! The zone grid editor of the desktop app asks Rust for a first grid of a sheet and for the
//! labels of an axis, so the rules live here and not in the frontend (rule 2). Every result is a
//! plain [`ZoneGrid`]; it becomes part of the project only through the undoable `set_zone_grid`
//! command, which validates it.

use serde::{Deserialize, Serialize};

use crate::balloon::UNITS_PER_MM;
use crate::geometry::{Point, Rect, Size};
use crate::sheet::ZoneGrid;

/// How the labels of one axis are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum LabelKind {
    /// `A, B, ..., Z, AA, AB, ...`
    Letters,
    /// `1, 2, 3, ...`
    Numbers,
}

/// Label scheme of one axis: the kind and whether it counts from the far end (numbers right to
/// left, letters bottom to top). Labels are always stored as printed, left to right and top to
/// bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct AxisScheme {
    /// Letters or numbers.
    pub kind: LabelKind,
    /// Counted from the far end.
    pub reversed: bool,
}

impl AxisScheme {
    /// Default for rows: lettered top to bottom.
    pub const ROWS: Self = Self {
        kind: LabelKind::Letters,
        reversed: false,
    };
    /// Default for columns: numbered left to right.
    pub const COLUMNS: Self = Self {
        kind: LabelKind::Numbers,
        reversed: false,
    };

    /// Every scheme, in the order [`AxisScheme::detect`] tries them.
    pub const ALL: [Self; 4] = [
        Self {
            kind: LabelKind::Letters,
            reversed: false,
        },
        Self {
            kind: LabelKind::Letters,
            reversed: true,
        },
        Self {
            kind: LabelKind::Numbers,
            reversed: false,
        },
        Self {
            kind: LabelKind::Numbers,
            reversed: true,
        },
    ];

    /// The labels of an axis with `count` divisions as printed (left to right, top to bottom).
    /// The count is clamped to 1 to [`ZoneGrid::MAX_DIVISIONS`].
    pub fn labels(self, count: u32) -> Vec<String> {
        let max = u32::try_from(ZoneGrid::MAX_DIVISIONS).unwrap_or(u32::MAX);
        let n = count.clamp(1, max);
        let mut labels: Vec<String> = (1..=n)
            .map(|i| match self.kind {
                LabelKind::Letters => letters(i),
                LabelKind::Numbers => i.to_string(),
            })
            .collect();
        if self.reversed {
            labels.reverse();
        }
        labels
    }

    /// The scheme that produces `labels`, or `None` for labels typed by hand.
    pub fn detect(labels: &[String]) -> Option<Self> {
        let count = u32::try_from(labels.len()).ok()?;
        if count == 0 || labels.len() > ZoneGrid::MAX_DIVISIONS {
            return None;
        }
        Self::ALL
            .into_iter()
            .find(|scheme| scheme.labels(count) == labels)
    }
}

/// Letters of a count: 1 is `A`, 26 is `Z`, 27 is `AA`.
pub fn letters(mut value: u32) -> String {
    let mut out = Vec::new();
    while value > 0 {
        let rest = (value - 1) % 26;
        // `rest` is below 26, so the byte is an upper case ASCII letter.
        out.push(b'A' + u8::try_from(rest).unwrap_or(0));
        value = (value - 1) / 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

/// One axis of a zone grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub enum ZoneAxis {
    /// The column labels, left to right.
    Columns,
    /// The row labels, top to bottom.
    Rows,
}

/// Margin of the default frame inside the sheet edge, in mm on the printed sheet.
pub const DEFAULT_FRAME_MARGIN_MM: f64 = 10.0;

impl ZoneGrid {
    /// A first grid for a sheet of `size`: a frame 10 mm inside the sheet edge (at most a quarter
    /// of the sheet side), 8 by 6 zones on landscape sheets and 6 by 8 on portrait ones, rows
    /// lettered and columns numbered.
    pub fn default_for(size: Size) -> Self {
        let landscape = size.width >= size.height;
        let margin = (DEFAULT_FRAME_MARGIN_MM * UNITS_PER_MM)
            .min(size.width / 4.0)
            .min(size.height / 4.0);
        let (columns, rows) = if landscape { (8, 6) } else { (6, 8) };
        Self {
            frame: Rect {
                origin: Point {
                    x: margin,
                    y: margin,
                },
                size: Size {
                    width: size.width - 2.0 * margin,
                    height: size.height - 2.0 * margin,
                },
            },
            column_labels: AxisScheme::COLUMNS.labels(columns),
            row_labels: AxisScheme::ROWS.labels(rows),
        }
    }

    /// The labels of `axis`.
    pub fn labels(&self, axis: ZoneAxis) -> &[String] {
        match axis {
            ZoneAxis::Columns => &self.column_labels,
            ZoneAxis::Rows => &self.row_labels,
        }
    }

    /// The grid with `count` divisions on `axis`, labelled by `scheme`. Without a scheme the
    /// axis keeps its current one, or gets the default of the axis if its labels were typed by
    /// hand.
    #[must_use]
    pub fn with_axis(&self, axis: ZoneAxis, count: u32, scheme: Option<AxisScheme>) -> Self {
        let scheme = scheme
            .or_else(|| AxisScheme::detect(self.labels(axis)))
            .unwrap_or(match axis {
                ZoneAxis::Columns => AxisScheme::COLUMNS,
                ZoneAxis::Rows => AxisScheme::ROWS,
            });
        let labels = scheme.labels(count);
        let mut grid = self.clone();
        match axis {
            ZoneAxis::Columns => grid.column_labels = labels,
            ZoneAxis::Rows => grid.row_labels = labels,
        }
        grid
    }
}

/// A request of the zone grid editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ZoneGridEdit {
    /// The first grid for a sheet of this size, see [`ZoneGrid::default_for`].
    Default {
        /// Sheet size in sheet units.
        size: Size,
    },
    /// The grid unchanged, to learn the label schemes of its axes.
    Describe {
        /// The grid.
        grid: ZoneGrid,
    },
    /// The grid with another count or scheme on one axis, see [`ZoneGrid::with_axis`].
    Axis {
        /// The grid.
        grid: ZoneGrid,
        /// The axis to change.
        axis: ZoneAxis,
        /// Number of divisions, clamped to 1 to 100.
        count: u32,
        /// Label scheme; `null` keeps the current one.
        scheme: Option<AxisScheme>,
    },
}

/// A zone grid as the editor shows it: the grid and the label scheme of each axis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct ZoneGridForm {
    /// The resulting grid. Not stored until it is sent with `set_zone_grid`.
    pub grid: ZoneGrid,
    /// Scheme of the column labels, `null` for labels typed by hand.
    pub column_scheme: Option<AxisScheme>,
    /// Scheme of the row labels, `null` for labels typed by hand.
    pub row_scheme: Option<AxisScheme>,
}

impl ZoneGridForm {
    /// The form of `grid`.
    pub fn of(grid: ZoneGrid) -> Self {
        Self {
            column_scheme: AxisScheme::detect(&grid.column_labels),
            row_scheme: AxisScheme::detect(&grid.row_labels),
            grid,
        }
    }
}

/// Answers a request of the zone grid editor.
pub fn zone_grid_form(edit: ZoneGridEdit) -> ZoneGridForm {
    match edit {
        ZoneGridEdit::Default { size } => ZoneGridForm::of(ZoneGrid::default_for(size)),
        ZoneGridEdit::Describe { grid } => ZoneGridForm::of(grid),
        ZoneGridEdit::Axis {
            grid,
            axis,
            count,
            scheme,
        } => ZoneGridForm::of(grid.with_axis(axis, count, scheme)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    const LETTERS_REVERSED: AxisScheme = AxisScheme {
        kind: LabelKind::Letters,
        reversed: true,
    };
    const NUMBERS_REVERSED: AxisScheme = AxisScheme {
        kind: LabelKind::Numbers,
        reversed: true,
    };

    #[test]
    fn letters_count_like_spreadsheet_columns() {
        let got: Vec<String> = [1, 2, 26, 27, 28, 52, 53].map(letters).to_vec();
        assert_eq!(got, strings(&["A", "B", "Z", "AA", "AB", "AZ", "BA"]));
    }

    #[test]
    fn labels_an_axis_as_printed_also_from_the_far_end() {
        assert_eq!(
            AxisScheme::COLUMNS.labels(4),
            strings(&["1", "2", "3", "4"])
        );
        assert_eq!(NUMBERS_REVERSED.labels(4), strings(&["4", "3", "2", "1"]));
        assert_eq!(AxisScheme::ROWS.labels(3), strings(&["A", "B", "C"]));
        assert_eq!(LETTERS_REVERSED.labels(3), strings(&["C", "B", "A"]));
        assert_eq!(AxisScheme::COLUMNS.labels(0), strings(&["1"]));
        assert_eq!(AxisScheme::COLUMNS.labels(500).len(), 100);
    }

    #[test]
    fn detects_the_scheme_or_none_for_labels_typed_by_hand() {
        assert_eq!(
            AxisScheme::detect(&strings(&["C", "B", "A"])),
            Some(LETTERS_REVERSED)
        );
        assert_eq!(
            AxisScheme::detect(&strings(&["1", "2"])),
            Some(AxisScheme::COLUMNS)
        );
        assert_eq!(AxisScheme::detect(&strings(&["X", "Y"])), None);
        assert_eq!(AxisScheme::detect(&strings(&["F", "E", "D"])), None);
        assert_eq!(AxisScheme::detect(&[]), None);
        // One label is both `A` and `A` reversed: the first scheme tried wins.
        assert_eq!(AxisScheme::detect(&strings(&["A"])), Some(AxisScheme::ROWS));
    }

    #[test]
    fn default_grid_is_8_by_6_on_landscape_and_6_by_8_on_portrait() {
        let landscape = ZoneGrid::default_for(Size {
            width: 1190.0,
            height: 842.0,
        });
        assert_eq!(
            landscape.column_labels,
            strings(&["1", "2", "3", "4", "5", "6", "7", "8"])
        );
        assert_eq!(
            landscape.row_labels,
            strings(&["A", "B", "C", "D", "E", "F"])
        );
        let margin = 10.0 * 72.0 / 25.4;
        assert!((landscape.frame.origin.x - margin).abs() < 1e-9);
        assert!((landscape.frame.origin.y - margin).abs() < 1e-9);
        assert!((landscape.frame.size.width - (1190.0 - 2.0 * margin)).abs() < 1e-9);
        assert!(landscape.validate().is_ok());

        let portrait = ZoneGrid::default_for(Size {
            width: 595.0,
            height: 842.0,
        });
        assert_eq!(portrait.columns(), 6);
        assert_eq!(portrait.rows(), 8);
        assert!(portrait.validate().is_ok());
    }

    #[test]
    fn default_frame_stays_inside_a_tiny_sheet() {
        let grid = ZoneGrid::default_for(Size {
            width: 40.0,
            height: 20.0,
        });
        assert!((grid.frame.origin.y - 5.0).abs() < 1e-9);
        assert!((grid.frame.size.height - 10.0).abs() < 1e-9);
        assert!(grid.validate().is_ok());
    }

    #[test]
    fn changes_one_axis_and_keeps_its_scheme_unless_told_otherwise() {
        let grid = ZoneGrid {
            frame: Rect {
                origin: Point { x: 10.0, y: 20.0 },
                size: Size {
                    width: 300.0,
                    height: 100.0,
                },
            },
            column_labels: strings(&["3", "2", "1"]),
            row_labels: strings(&["X", "Y"]),
        };
        let more = grid.with_axis(ZoneAxis::Columns, 4, None);
        assert_eq!(more.column_labels, strings(&["4", "3", "2", "1"]));
        assert_eq!(more.row_labels, grid.row_labels);
        assert_eq!(more.frame, grid.frame);
        // Labels typed by hand get the default scheme of the axis.
        let rows = grid.with_axis(ZoneAxis::Rows, 3, None);
        assert_eq!(rows.row_labels, strings(&["A", "B", "C"]));
        let lettered = grid.with_axis(ZoneAxis::Rows, 3, Some(LETTERS_REVERSED));
        assert_eq!(lettered.row_labels, strings(&["C", "B", "A"]));
    }

    #[test]
    fn form_reports_the_scheme_of_each_axis() {
        let form = zone_grid_form(ZoneGridEdit::Default {
            size: Size {
                width: 842.0,
                height: 595.0,
            },
        });
        assert_eq!(form.column_scheme, Some(AxisScheme::COLUMNS));
        assert_eq!(form.row_scheme, Some(AxisScheme::ROWS));
        let mut typed = form.grid.clone();
        typed.row_labels = strings(&["P", "Q"]);
        let described = zone_grid_form(ZoneGridEdit::Describe { grid: typed });
        assert_eq!(described.row_scheme, None);
        let changed = zone_grid_form(ZoneGridEdit::Axis {
            grid: form.grid,
            axis: ZoneAxis::Columns,
            count: 3,
            scheme: Some(NUMBERS_REVERSED),
        });
        assert_eq!(changed.grid.column_labels, strings(&["3", "2", "1"]));
        assert_eq!(changed.column_scheme, Some(NUMBERS_REVERSED));
    }

    #[test]
    fn edits_have_a_tagged_json_shape() {
        let edit: ZoneGridEdit = serde_json::from_str(
            r#"{"type":"axis","grid":{"frame":{"origin":{"x":0,"y":0},"size":{"width":10,"height":10}},"column_labels":["1"],"row_labels":["A"]},"axis":"rows","count":2,"scheme":null}"#,
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let form = zone_grid_form(edit);
        assert_eq!(form.grid.row_labels, strings(&["A", "B"]));
    }
}
