//! Columns of the characteristic list (FR-CHR-02) and their cell values.

use dimo_core::{
    Characteristic, CharacteristicKind, CharacteristicStatus, Classification, DerivationRule,
    Project, Unit,
};
use rust_decimal::Decimal;

use super::Language;

/// One column of the characteristic list. The order of [`COLUMNS`] is the column order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    /// Display number (balloon number).
    Number,
    /// Kind of characteristic.
    Kind,
    /// Requirement text as on the drawing.
    Requirement,
    /// Nominal value.
    Nominal,
    /// Upper deviation.
    UpperDeviation,
    /// Lower deviation.
    LowerDeviation,
    /// Upper limit.
    UpperLimit,
    /// Lower limit.
    LowerLimit,
    /// Unit.
    Unit,
    /// Fit designation.
    Fit,
    /// Tolerance rule that produced the limits (FR-TOL-08), as a stable English identifier.
    Rule,
    /// Quantity of features.
    Quantity,
    /// Classification.
    Classification,
    /// Inspection method.
    Method,
    /// Gauge.
    Gauge,
    /// Sampling.
    Sampling,
    /// Inspection frequency.
    Frequency,
    /// Whether the characteristic is inspected.
    Inspect,
    /// Review status.
    Status,
    /// Sheet (page) number, starting at 1.
    Sheet,
    /// Comment.
    Comment,
}

/// All columns in output order.
pub const COLUMNS: [Column; 21] = [
    Column::Number,
    Column::Kind,
    Column::Requirement,
    Column::Nominal,
    Column::UpperDeviation,
    Column::LowerDeviation,
    Column::UpperLimit,
    Column::LowerLimit,
    Column::Unit,
    Column::Fit,
    Column::Rule,
    Column::Quantity,
    Column::Classification,
    Column::Method,
    Column::Gauge,
    Column::Sampling,
    Column::Frequency,
    Column::Inspect,
    Column::Status,
    Column::Sheet,
    Column::Comment,
];

/// A cell value before formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Cell {
    /// Empty cell.
    Empty,
    /// Text.
    Text(String),
    /// Whole number.
    Integer(u32),
    /// Exact decimal.
    Decimal(Decimal),
}

impl Column {
    /// Header text in the given language. Texts are UTF-8, German headers use umlauts.
    pub fn header(self, language: Language) -> &'static str {
        let (en, de) = match self {
            Self::Number => ("No", "Nr"),
            Self::Kind => ("Kind", "Art"),
            Self::Requirement => ("Requirement", "Anforderung"),
            Self::Nominal => ("Nominal", "Nennmaß"),
            Self::UpperDeviation => ("Upper deviation", "Oberes Abmaß"),
            Self::LowerDeviation => ("Lower deviation", "Unteres Abmaß"),
            Self::UpperLimit => ("Upper limit", "Obere Grenze"),
            Self::LowerLimit => ("Lower limit", "Untere Grenze"),
            Self::Unit => ("Unit", "Einheit"),
            Self::Fit => ("Fit", "Passung"),
            Self::Rule => ("Rule", "Regel"),
            Self::Quantity => ("Quantity", "Anzahl"),
            Self::Classification => ("Classification", "Klassifizierung"),
            Self::Method => ("Inspection method", "Prüfmethode"),
            Self::Gauge => ("Gauge", "Prüfmittel"),
            Self::Sampling => ("Sampling", "Stichprobe"),
            Self::Frequency => ("Frequency", "Häufigkeit"),
            Self::Inspect => ("Inspect", "Prüfen"),
            Self::Status => ("Status", "Status"),
            Self::Sheet => ("Sheet", "Blatt"),
            Self::Comment => ("Comment", "Kommentar"),
        };
        match language {
            Language::English => en,
            Language::German => de,
        }
    }

    /// Suggested column width in characters for spreadsheets.
    pub(super) fn width(self) -> f64 {
        match self {
            Self::Requirement | Self::Comment => 36.0,
            Self::Method | Self::Gauge | Self::Sampling | Self::Classification => 18.0,
            Self::UpperDeviation | Self::LowerDeviation | Self::UpperLimit | Self::LowerLimit => {
                15.0
            }
            Self::Number | Self::Unit | Self::Quantity | Self::Inspect | Self::Sheet => 8.0,
            Self::Rule => 20.0,
            _ => 12.0,
        }
    }

    /// The value of this column for one characteristic.
    pub(super) fn cell(self, project: &Project, c: &Characteristic) -> Cell {
        let text = |s: &str| {
            if s.is_empty() {
                Cell::Empty
            } else {
                Cell::Text(s.to_owned())
            }
        };
        let decimal = |d: Option<Decimal>| d.map_or(Cell::Empty, Cell::Decimal);
        match self {
            // Plain numbers stay number cells, so exports of plain numbering do not change.
            Self::Number => c
                .number
                .as_plain()
                .map_or_else(|| Cell::Text(c.number.to_string()), Cell::Integer),
            Self::Kind => Cell::Text(kind_id(c.kind).to_owned()),
            Self::Requirement => text(&c.requirement_text),
            Self::Nominal => decimal(c.nominal),
            Self::UpperDeviation => decimal(c.upper_dev),
            Self::LowerDeviation => decimal(c.lower_dev),
            Self::UpperLimit => decimal(c.upper_limit),
            Self::LowerLimit => decimal(c.lower_limit),
            Self::Unit => c
                .unit
                .map_or(Cell::Empty, |u| Cell::Text(unit_id(u).to_owned())),
            Self::Fit => c.fit.as_deref().map_or(Cell::Empty, text),
            Self::Rule => c
                .derivation
                .as_ref()
                .map_or(Cell::Empty, |d| Cell::Text(rule_id(&d.rule).to_owned())),
            Self::Quantity => Cell::Integer(c.quantity),
            Self::Classification => match c.classification {
                Classification::None => Cell::Empty,
                other => Cell::Text(classification_id(other).to_owned()),
            },
            Self::Method => text(&c.inspection.method),
            Self::Gauge => text(&c.inspection.gauge),
            Self::Sampling => text(&c.inspection.sampling),
            Self::Frequency => text(&c.inspection.frequency),
            Self::Inspect => Cell::Text(if c.inspect { "yes" } else { "no" }.to_owned()),
            Self::Status => Cell::Text(status_id(c.status).to_owned()),
            Self::Sheet => sheet_number(project, c).map_or(Cell::Empty, Cell::Integer),
            Self::Comment => text(&c.comment),
        }
    }
}

/// Characteristics to export in display number order. Rejected ones are left out (they are kept
/// in the project for traceability only). Equal numbers keep placement order.
pub(super) fn rows(project: &Project) -> Vec<&Characteristic> {
    let mut rows: Vec<&Characteristic> = project
        .characteristics
        .iter()
        .filter(|c| c.status != CharacteristicStatus::Rejected)
        .collect();
    rows.sort_by_key(|c| c.number);
    rows
}

/// Page number (from 1) of the sheet the characteristic was read from, else of its first balloon.
fn sheet_number(project: &Project, c: &Characteristic) -> Option<u32> {
    let id = c
        .sources
        .first()
        .map(|s| s.sheet)
        .or_else(|| project.balloons_of(c.id).next().map(|b| b.sheet))?;
    project.sheet(id).map(|s| s.index + 1)
}

fn kind_id(kind: CharacteristicKind) -> &'static str {
    use CharacteristicKind as K;
    match kind {
        K::Linear => "linear",
        K::Diameter => "diameter",
        K::Radius => "radius",
        K::SphericalRadius => "spherical_radius",
        K::Angle => "angle",
        K::Chamfer => "chamfer",
        K::Thread => "thread",
        K::Counterbore => "counterbore",
        K::Countersink => "countersink",
        K::Depth => "depth",
        K::SurfaceTexture => "surface_texture",
        K::Geometric => "geometric",
        K::Note => "note",
        K::FlagNote => "flag_note",
        K::MaterialProcess => "material_process",
        K::Other => "other",
    }
}

/// The rule as in `project.json` and the corpus truth format, never translated.
fn rule_id(rule: &DerivationRule) -> &'static str {
    match rule {
        DerivationRule::Explicit => "explicit",
        DerivationRule::Fit { .. } => "fit",
        DerivationRule::DrawingRule { .. } => "drawing_rule",
        DerivationRule::General { .. } => "general",
        DerivationRule::DecimalRule { .. } => "decimal_rule",
        DerivationRule::CustomTable { .. } => "custom_table",
        DerivationRule::NoToleranceDefined => "no_tolerance_defined",
        DerivationRule::Manual => "manual",
    }
}

fn unit_id(unit: Unit) -> &'static str {
    match unit {
        Unit::Mm => "mm",
        Unit::In => "in",
        Unit::Deg => "deg",
    }
}

fn classification_id(c: Classification) -> &'static str {
    match c {
        Classification::Critical => "critical",
        Classification::Major => "major",
        Classification::Minor => "minor",
        Classification::Key => "key",
        Classification::None => "none",
    }
}

fn status_id(s: CharacteristicStatus) -> &'static str {
    match s {
        CharacteristicStatus::Proposed => "proposed",
        CharacteristicStatus::Accepted => "accepted",
        CharacteristicStatus::Verified => "verified",
        CharacteristicStatus::Rejected => "rejected",
    }
}
