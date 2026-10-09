//! Validated tolerance tables and range lookup (T2.2, D-43, FR-TOL-02, FR-TOL-07).
//!
//! A [`Table`] can only be built by [`Table::parse`], which validates the whole file, so every
//! loaded table satisfies the rules listed there.

use std::collections::HashSet;
use std::fmt;

use rust_decimal::Decimal;

use crate::designation::{Fit, Grade};
use crate::error::TableError;
use crate::format::{
    AppliesTo, DATE_PATTERN, Deviation, Feature, Part, PartKind, RangeOf, Row, Status, TableFile,
    TableHeader, TableKind, ValueUnit,
};

/// One end of a size range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bound {
    /// The bound value in the table's size unit.
    pub value: Decimal,
    /// Whether the bound itself belongs to the range.
    pub inclusive: bool,
}

/// A size range of a table row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Range {
    /// Lower bound; `None` means the range starts above zero.
    pub min: Option<Bound>,
    /// Upper bound; `None` means the range is open ended.
    pub max: Option<Bound>,
}

impl Range {
    /// Whether `size` lies in this range. Sizes of zero or below lie in no range.
    pub fn contains(&self, size: Decimal) -> bool {
        if size <= Decimal::ZERO {
            return false;
        }
        let above_min = match self.min {
            None => true,
            Some(Bound {
                value,
                inclusive: true,
            }) => size >= value,
            Some(Bound { value, .. }) => size > value,
        };
        let below_max = match self.max {
            None => true,
            Some(Bound {
                value,
                inclusive: true,
            }) => size <= value,
            Some(Bound { value, .. }) => size < value,
        };
        above_min && below_max
    }

    /// Whether this range lies completely inside `outer`.
    pub fn is_within(&self, outer: &Range) -> bool {
        let min_ok = match (outer.min, self.min) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(o), Some(s)) => {
                s.value > o.value || (s.value == o.value && (o.inclusive || !s.inclusive))
            }
        };
        let max_ok = match (outer.max, self.max) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(o), Some(s)) => {
                s.value < o.value || (s.value == o.value && (o.inclusive || !s.inclusive))
            }
        };
        min_ok && max_ok
    }

    fn of_row(row: &Row) -> Self {
        let min = row
            .min_inclusive
            .map(|value| Bound {
                value,
                inclusive: true,
            })
            .or(row.min_exclusive.map(|value| Bound {
                value,
                inclusive: false,
            }));
        let max = row
            .max_inclusive
            .map(|value| Bound {
                value,
                inclusive: true,
            })
            .or(row.max_exclusive.map(|value| Bound {
                value,
                inclusive: false,
            }));
        Self { min, max }
    }
}

impl fmt::Display for Range {
    /// Stable English form, e.g. `over 30 up to and including 120`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        match self.min {
            Some(Bound {
                value,
                inclusive: true,
            }) => parts.push(format!("from {value}")),
            Some(Bound { value, .. }) => parts.push(format!("over {value}")),
            None => {}
        }
        match self.max {
            Some(Bound {
                value,
                inclusive: true,
            }) => parts.push(format!("up to and including {value}")),
            Some(Bound { value, .. }) => parts.push(format!("below {value}")),
            None => {}
        }
        if parts.is_empty() {
            parts.push("any size".to_owned());
        }
        f.write_str(&parts.join(" "))
    }
}

/// Why a cell lookup found no value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LookupError {
    /// The table has no part for this dimension type.
    #[error("table {table} has no part for {applies_to:?} dimensions")]
    NoPart {
        /// Table id.
        table: String,
        /// Requested dimension type.
        applies_to: AppliesTo,
    },
    /// The part has no such column (class, grade or letter).
    #[error("part {part} of table {table} has no column {column:?}")]
    UnknownColumn {
        /// Table id.
        table: String,
        /// Part id.
        part: String,
        /// Requested column.
        column: String,
    },
    /// The size is zero or negative.
    #[error("size {size} is not positive")]
    SizeNotPositive {
        /// Requested size.
        size: Decimal,
    },
    /// No row covers the size.
    #[error("size {size} is outside the ranges of part {part} of table {table}")]
    OutOfRange {
        /// Table id.
        table: String,
        /// Part id.
        part: String,
        /// Requested size.
        size: Decimal,
    },
    /// The row covers the size, but the source defines no value for this column.
    #[error("table {table} defines no value for {column} in part {part}, range {range}")]
    NotDefined {
        /// Table id.
        table: String,
        /// Part id.
        part: String,
        /// Requested column.
        column: String,
        /// Range of the row that covers the size.
        range: Range,
    },
}

/// A value found in a part, with the row it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellValue {
    /// The value in the part's value unit.
    pub value: Decimal,
    /// Range of the row the value came from.
    pub range: Range,
}

/// A general tolerance found by [`Table::general`] (FR-TOL-02, FR-TOL-07).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneralTolerance {
    /// Table id.
    pub table: String,
    /// Table version.
    pub version: u32,
    /// Whether the table is still a draft (D-43 badge).
    pub draft: bool,
    /// Part id, e.g. `linear`.
    pub part: String,
    /// Class (column), e.g. `m`.
    pub class: String,
    /// Which size selected the row.
    pub range_of: RangeOf,
    /// Range of the row that covers the size.
    pub range: Range,
    /// The tolerance: deviations are `+value` and `-value`.
    pub value: Decimal,
    /// Unit of `value`.
    pub unit: ValueUnit,
}

/// A validated part, see [`Table::parse`].
#[derive(Debug, Clone, PartialEq)]
pub struct TablePart {
    part: Part,
    ranges: Vec<Range>,
}

impl TablePart {
    /// The part as written in the file.
    pub fn data(&self) -> &Part {
        &self.part
    }

    /// Ranges of the rows, in row order.
    pub fn ranges(&self) -> &[Range] {
        &self.ranges
    }

    /// Index of a column.
    pub fn column(&self, name: &str) -> Option<usize> {
        self.part.columns.iter().position(|c| c == name)
    }

    /// Index of the row that covers `size`.
    pub fn row_for(&self, size: Decimal) -> Option<usize> {
        self.ranges.iter().position(|r| r.contains(size))
    }

    /// The value of `column` for `size`.
    pub fn lookup(
        &self,
        table: &str,
        column: &str,
        size: Decimal,
    ) -> Result<CellValue, LookupError> {
        let col = self
            .column(column)
            .ok_or_else(|| LookupError::UnknownColumn {
                table: table.to_owned(),
                part: self.part.id.clone(),
                column: column.to_owned(),
            })?;
        if size <= Decimal::ZERO {
            return Err(LookupError::SizeNotPositive { size });
        }
        let row = self.row_for(size).ok_or_else(|| LookupError::OutOfRange {
            table: table.to_owned(),
            part: self.part.id.clone(),
            size,
        })?;
        let range = self.ranges[row];
        match self.part.rows[row].values[col].0 {
            Some(value) => Ok(CellValue { value, range }),
            None => Err(LookupError::NotDefined {
                table: table.to_owned(),
                part: self.part.id.clone(),
                column: column.to_owned(),
                range,
            }),
        }
    }
}

/// A validated tolerance table.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    origin: String,
    header: TableHeader,
    parts: Vec<TablePart>,
}

impl Table {
    /// Parse and validate a table file. `origin` names the file in error messages, e.g.
    /// `shipped:iso-2768-1.toml` or a path.
    ///
    /// Rules checked here (D-43, FR-TOL-07):
    /// - the TOML matches the format (unknown keys and TOML numbers are refused);
    /// - ids match their patterns, `version` is at least 1, `title` and `source` are not empty;
    /// - a draft has no verification fields, a verified table has both;
    /// - part ids and column names are unique, each row has one value per column;
    /// - rows are ascending and contiguous: a row starts where the previous one ends, with the
    ///   bound belonging to exactly one of them; only the first row may lack a lower bound and
    ///   only the last row may lack an upper bound;
    /// - general and custom tables hold only `symmetric` parts with non negative values and
    ///   one part per dimension type; fit tables hold the parts that the fit expansion needs.
    pub fn parse(origin: &str, text: &str) -> Result<Self, TableError> {
        let file: TableFile = toml::from_str(text).map_err(|e| TableError::Parse {
            origin: origin.to_owned(),
            message: e.to_string().trim_end().to_owned(),
        })?;
        Self::validate(origin, file)
    }

    fn validate(origin: &str, file: TableFile) -> Result<Self, TableError> {
        let invalid = |location: &str, message: String| TableError::Invalid {
            origin: origin.to_owned(),
            location: location.to_owned(),
            message,
        };
        let header = file.table;
        check_header(&header).map_err(|m| invalid("table", m))?;

        let mut ids = HashSet::new();
        let mut roles = HashSet::new();
        let mut parts = Vec::with_capacity(file.parts.len());
        for part in file.parts {
            let location = format!("part {}", part.id);
            if !ids.insert(part.id.clone()) {
                return Err(invalid(&location, "part id is used twice".to_owned()));
            }
            check_part(header.kind, &part).map_err(|m| invalid(&location, m))?;
            let role = (part.kind, part.applies_to, part.feature, part.deviation);
            if !roles.insert(role) {
                return Err(invalid(
                    &location,
                    "another part has the same kind, applies_to, feature and deviation".to_owned(),
                ));
            }
            let ranges = check_rows(&part).map_err(|m| invalid(&location, m))?;
            parts.push(TablePart { part, ranges });
        }
        let table = Self {
            origin: origin.to_owned(),
            header,
            parts,
        };
        if table.header.kind == TableKind::Fit {
            for (kind, feature, deviation) in FIT_PARTS {
                if table.part_by_role(kind, feature, deviation).is_none() {
                    return Err(invalid(
                        "table",
                        format!(
                            "a fit table needs a {kind:?} part{}",
                            match (feature, deviation) {
                                (Some(f), Some(d)) => format!(" for {f:?} {d:?}"),
                                _ => String::new(),
                            }
                        ),
                    ));
                }
            }
        }
        Ok(table)
    }

    /// Where the table was loaded from.
    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// The `[table]` header.
    pub fn header(&self) -> &TableHeader {
        &self.header
    }

    /// Table id.
    pub fn id(&self) -> &str {
        &self.header.id
    }

    /// Whether the table is still a draft (D-43).
    pub fn is_draft(&self) -> bool {
        self.header.status == Status::Draft
    }

    /// All parts in file order.
    pub fn parts(&self) -> &[TablePart] {
        &self.parts
    }

    /// A part by id.
    pub fn part(&self, id: &str) -> Option<&TablePart> {
        self.parts.iter().find(|p| p.part.id == id)
    }

    pub(crate) fn part_by_role(
        &self,
        kind: PartKind,
        feature: Option<Feature>,
        deviation: Option<Deviation>,
    ) -> Option<&TablePart> {
        self.parts.iter().find(|p| {
            p.part.kind == kind && p.part.feature == feature && p.part.deviation == deviation
        })
    }

    /// The general tolerance for a dimension type, class and size (FR-TOL-02, FR-TOL-07).
    ///
    /// `size` is the nominal size, or the shorter leg length for parts with
    /// `range_of = "shorter_leg"` (angles in ISO 2768-1).
    pub fn general(
        &self,
        applies_to: AppliesTo,
        class: &str,
        size: Decimal,
    ) -> Result<GeneralTolerance, LookupError> {
        let part = self
            .parts
            .iter()
            .find(|p| p.part.kind == PartKind::Symmetric && p.part.applies_to == Some(applies_to))
            .ok_or_else(|| LookupError::NoPart {
                table: self.header.id.clone(),
                applies_to,
            })?;
        let found = part.lookup(&self.header.id, class, size)?;
        Ok(GeneralTolerance {
            table: self.header.id.clone(),
            version: self.header.version,
            draft: self.is_draft(),
            part: part.part.id.clone(),
            class: class.to_owned(),
            range_of: part.part.range_of,
            range: found.range,
            value: found.value,
            unit: part.part.value_unit,
        })
    }
}

/// Parts every fit table needs (FR-TOL-04).
const FIT_PARTS: [(PartKind, Option<Feature>, Option<Deviation>); 6] = [
    (PartKind::StandardTolerance, None, None),
    (PartKind::Delta, None, None),
    (
        PartKind::FundamentalDeviation,
        Some(Feature::Shaft),
        Some(Deviation::Upper),
    ),
    (
        PartKind::FundamentalDeviation,
        Some(Feature::Shaft),
        Some(Deviation::Lower),
    ),
    (
        PartKind::FundamentalDeviation,
        Some(Feature::Hole),
        Some(Deviation::Upper),
    ),
    (
        PartKind::FundamentalDeviation,
        Some(Feature::Hole),
        Some(Deviation::Lower),
    ),
];

fn matches_pattern(text: &str, allowed: impl Fn(char) -> bool, sep: char) -> bool {
    !text.is_empty()
        && text
            .split(sep)
            .all(|g| !g.is_empty() && g.chars().all(&allowed))
}

fn is_lower_alnum(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit()
}

fn is_date(text: &str) -> bool {
    // DATE_PATTERN: four digits, '-', two digits, '-', two digits.
    debug_assert_eq!(DATE_PATTERN, r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$");
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes.iter().enumerate().all(|(i, b)| match i {
            4 | 7 => *b == b'-',
            _ => b.is_ascii_digit(),
        })
}

fn check_header(header: &TableHeader) -> Result<(), String> {
    if !matches_pattern(&header.id, is_lower_alnum, '-') {
        return Err(format!(
            "id {:?} must be lowercase letters and digits joined by '-'",
            header.id
        ));
    }
    if header.version == 0 {
        return Err("version starts at 1".to_owned());
    }
    if header.title.trim().is_empty() {
        return Err("title is empty".to_owned());
    }
    if header.source.trim().is_empty() {
        return Err("source is empty, name the standard, edition and table".to_owned());
    }
    // D-43: the verification fields go with the status, nothing else.
    match header.status {
        Status::Draft => {
            if header.verified_by.is_some() || header.verified_date.is_some() {
                return Err("a draft table has no verification name or date".to_owned());
            }
        }
        Status::Verified => {
            let by = header.verified_by.as_deref().unwrap_or("").trim();
            let date = header.verified_date.as_deref().unwrap_or("");
            if by.is_empty() || !is_date(date) {
                return Err(
                    "a verified table needs the name of the verifier and a date YYYY-MM-DD"
                        .to_owned(),
                );
            }
        }
    }
    Ok(())
}

fn is_grade_column(name: &str) -> bool {
    name.strip_prefix("IT").and_then(Grade::parse).is_some()
}

fn check_part(kind: TableKind, part: &Part) -> Result<(), String> {
    if !matches_pattern(&part.id, is_lower_alnum, '_') {
        return Err("id must be lowercase letters and digits joined by '_'".to_owned());
    }
    if part.source.trim().is_empty() {
        return Err("source is empty".to_owned());
    }
    let mut columns = HashSet::new();
    for column in &part.columns {
        if column.is_empty() || !columns.insert(column.as_str()) {
            return Err(format!("column {column:?} is empty or used twice"));
        }
    }
    let symmetric = part.kind == PartKind::Symmetric;
    match kind {
        TableKind::General | TableKind::Custom if !symmetric => {
            return Err("general and custom tables hold only symmetric parts".to_owned());
        }
        TableKind::Fit if symmetric => {
            return Err("fit tables hold no symmetric parts".to_owned());
        }
        _ => {}
    }
    if symmetric != part.applies_to.is_some() {
        return Err("applies_to is required for symmetric parts and absent otherwise".to_owned());
    }
    let fundamental = part.kind == PartKind::FundamentalDeviation;
    if fundamental != (part.feature.is_some() && part.deviation.is_some())
        || (!fundamental && (part.feature.is_some() || part.deviation.is_some()))
    {
        return Err(
            "feature and deviation are required for fundamental_deviation parts and absent otherwise"
                .to_owned(),
        );
    }
    let arcmin = part.value_unit == ValueUnit::Arcmin;
    if arcmin != (part.applies_to == Some(AppliesTo::Angular)) {
        return Err("angular parts use value_unit arcmin, all others mm".to_owned());
    }
    match part.kind {
        PartKind::StandardTolerance | PartKind::Delta => {
            if let Some(c) = part.columns.iter().find(|c| !is_grade_column(c)) {
                return Err(format!(
                    "column {c:?} is not a grade IT01, IT0, IT1 to IT18"
                ));
            }
        }
        PartKind::Override => {
            if let Some(c) = part.columns.iter().find(|c| Fit::parse(c).is_none()) {
                return Err(format!("column {c:?} is not a tolerance class such as M6"));
            }
        }
        PartKind::Symmetric | PartKind::FundamentalDeviation => {}
    }
    if part.range_of == RangeOf::ShorterLeg && part.applies_to != Some(AppliesTo::Angular) {
        return Err("range_of shorter_leg is only for angular parts".to_owned());
    }
    Ok(())
}

fn check_rows(part: &Part) -> Result<Vec<Range>, String> {
    let mut ranges: Vec<Range> = Vec::with_capacity(part.rows.len());
    let last = part.rows.len().saturating_sub(1);
    for (i, row) in part.rows.iter().enumerate() {
        let at = |m: &str| format!("row {}: {m}", i + 1);
        if row.min_inclusive.is_some() && row.min_exclusive.is_some() {
            return Err(at("min_inclusive and min_exclusive together"));
        }
        if row.max_inclusive.is_some() && row.max_exclusive.is_some() {
            return Err(at("max_inclusive and max_exclusive together"));
        }
        if row.values.len() != part.columns.len() {
            return Err(at(&format!(
                "{} values for {} columns",
                row.values.len(),
                part.columns.len()
            )));
        }
        let range = Range::of_row(row);
        if let Some(min) = range.min
            && min.value < Decimal::ZERO
        {
            return Err(at("negative lower bound"));
        }
        if let (Some(min), Some(max)) = (range.min, range.max)
            && min.value >= max.value
        {
            return Err(at("lower bound is not below the upper bound"));
        }
        if range.max.is_none() && i != last {
            return Err(at("only the last row may omit the upper bound"));
        }
        if let Some(prev) = ranges.last() {
            let (Some(end), Some(start)) = (prev.max, range.min) else {
                return Err(at("only the first row may omit the lower bound"));
            };
            if end.value != start.value || end.inclusive == start.inclusive {
                return Err(at(
                    "the row must start where the previous one ends, with the bound in exactly one of them",
                ));
            }
        }
        let non_negative = matches!(
            part.kind,
            PartKind::Symmetric | PartKind::StandardTolerance | PartKind::Delta
        );
        if non_negative
            && row
                .values
                .iter()
                .any(|c| c.0.is_some_and(|v| v < Decimal::ZERO))
        {
            return Err(at("negative value in a part that holds magnitudes"));
        }
        if part.kind == PartKind::StandardTolerance
            && row.values.iter().any(|c| c.0 == Some(Decimal::ZERO))
        {
            return Err(at("a standard tolerance is never zero"));
        }
        ranges.push(range);
    }
    Ok(ranges)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(text: &str) -> Decimal {
        Decimal::from_str(text).unwrap()
    }

    #[test]
    fn range_bounds_follow_inclusivity() {
        let r = Range {
            min: Some(Bound {
                value: d("30"),
                inclusive: false,
            }),
            max: Some(Bound {
                value: d("120"),
                inclusive: true,
            }),
        };
        assert!(!r.contains(d("30")));
        assert!(r.contains(d("30.0001")));
        assert!(r.contains(d("120")));
        assert!(!r.contains(d("120.0001")));
        assert_eq!(r.to_string(), "over 30 up to and including 120");
        let open = Range {
            min: None,
            max: None,
        };
        assert!(!open.contains(d("0")));
        assert!(open.contains(d("0.001")));
    }

    #[test]
    fn within_compares_bounds() {
        let outer = Range {
            min: Some(Bound {
                value: d("10"),
                inclusive: false,
            }),
            max: Some(Bound {
                value: d("18"),
                inclusive: true,
            }),
        };
        let inner = Range {
            min: Some(Bound {
                value: d("10"),
                inclusive: false,
            }),
            max: Some(Bound {
                value: d("14"),
                inclusive: true,
            }),
        };
        assert!(inner.is_within(&outer));
        assert!(!outer.is_within(&inner));
    }

    #[test]
    fn date_check() {
        assert!(is_date("2026-10-09"));
        assert!(!is_date("2026-1-09"));
        assert!(!is_date("2026/10/09"));
    }
}
