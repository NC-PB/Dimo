//! Numbering strategies: the order and display numbers a strategy gives the characteristics of a
//! project (FR-BAL-04, FR-BAL-05, FR-BAL-07, D-21, D-22).
//!
//! Everything here is a pure function of the project. [`preview`] returns what
//! [`Command::ApplyNumbering`](crate::command::Command::ApplyNumbering) then does, so the user sees
//! the numbers before they change (FR-BAL-05).
//!
//! # Reference point
//!
//! A characteristic is ordered by one point on one sheet: the leader anchor of its first balloon
//! (the feature on the drawing), or without a balloon the center of its first source region.
//! Characteristics without either come last, in their current order.
//!
//! All ordering happens as the sheet is shown on screen: points, zones and views are turned by
//! the view rotation of their sheet first, so "top" is the top the user sees (FR-DOC-05).
//!
//! # Reading order
//!
//! Points are read in rows, top to bottom, and left to right inside a row:
//!
//! 1. Sort the points by their distance from the top (`y`), ties by the current order.
//! 2. The topmost point not yet in a row starts a new row. Every following point whose `y` is at
//!    most one row tolerance below that first point joins the row. The tolerance is the diameter
//!    of the project's default balloon (7 mm by default, D-24), so callouts on one visual line
//!    form one row even when their text baselines differ a little.
//! 3. Inside a row, points go left to right (`x`), ties by `y`, then by the current order.
//!
//! # Strategies
//!
//! Every strategy numbers sheet by sheet in page order (current revision first).
//!
//! - [`NumberingStrategy::SheetZone`] (default, D-21): zone by zone, then reading order inside
//!   the zone. Zones are read in the same way as points: rows of zones top to bottom, left to
//!   right, by their position on screen, not by their labels. A point outside the frame belongs
//!   to the nearest zone. A sheet without a zone grid is read in reading order as a whole.
//! - [`NumberingStrategy::View`]: view by view in the order the views were drawn, reading order
//!   inside each view. A point inside several views belongs to the smallest one. Points outside
//!   every view come last on their sheet, in reading order.
//! - [`NumberingStrategy::ViewClockwise`]: as `View`, but inside a view clockwise around the view
//!   center, starting at 12 o'clock; equal angles go from the center outwards.
//! - [`NumberingStrategy::Kind`]: grouped by characteristic kind in the order of
//!   [`CharacteristicKind`], each kind in the `SheetZone` order.
//! - [`NumberingStrategy::Manual`]: the current placement order.
//!
//! # Multi-instance sub-numbers (D-22, FR-BAL-07)
//!
//! A callout for several features (`4X`, at most 100) accepted while numbering is unlocked and
//! the project setting is [`MultiInstance::SubNumber`] becomes one characteristic per feature,
//! all with the same source region, kind and requirement text. Characteristics that agree in
//! these form a group. Every strategy except `Manual` keeps a group together at the place of its
//! first member, whatever the setting. With the sub-number setting, a group of two or more that
//! stands together in the order shares one base number with sub-numbers `5.1`, `5.2`;
//! everything else gets plain numbers `1..`.

use std::collections::{BTreeMap, HashMap};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::balloon::UNITS_PER_MM;
use crate::characteristic::{Characteristic, CharacteristicKind};
use crate::command::CommandError;
use crate::geometry::{Point, Rect, Size};
use crate::id::{CharId, SheetId};
use crate::number::DisplayNumber;
use crate::project::{MultiInstance, NumberingStrategy, Project};
use crate::sheet::{Rotation, Sheet};

/// One characteristic with the number a strategy gives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct NumberedCharacteristic {
    /// The characteristic.
    pub id: CharId,
    /// Its number under the strategy.
    #[cfg_attr(feature = "specta", specta(type = String))]
    pub number: DisplayNumber,
}

/// What applying a strategy would do, without changing anything (FR-BAL-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "snake_case")]
pub struct NumberingPreview {
    /// The strategy.
    pub strategy: NumberingStrategy,
    /// All characteristics in the new placement order with their new numbers.
    pub entries: Vec<NumberedCharacteristic>,
    /// Number of characteristics whose number would change.
    pub changed: u32,
}

/// The new order and numbers of `strategy` (FR-BAL-05). Refused while numbering is locked,
/// because applying would be refused too (D-23).
pub fn preview(
    project: &Project,
    strategy: NumberingStrategy,
) -> Result<NumberingPreview, CommandError> {
    if project.is_numbering_locked() {
        return Err(CommandError::NumberingLocked);
    }
    let order = strategy_order(project, strategy);
    let numbers = numbers_for_order(project, &order);
    let changed = order
        .iter()
        .zip(&numbers)
        .filter(|(id, number)| project.characteristic(**id).map(|c| c.number) != Some(**number))
        .count();
    Ok(NumberingPreview {
        strategy,
        entries: order
            .into_iter()
            .zip(numbers)
            .map(|(id, number)| NumberedCharacteristic { id, number })
            .collect(),
        changed: u32::try_from(changed).unwrap_or(u32::MAX),
    })
}

/// Display numbers for the characteristics in `order` while numbering is unlocked (D-21, D-22):
/// plain `1..`, except that with [`MultiInstance::SubNumber`] each run of two or more adjacent
/// characteristics of one multi-instance group shares a base number with sub-numbers.
pub fn numbers_for_order(project: &Project, order: &[CharId]) -> Vec<DisplayNumber> {
    let sub_numbers = project.settings.numbering.multi_instance == MultiInstance::SubNumber;
    let keys: Vec<Option<GroupKey>> = order
        .iter()
        .map(|id| {
            project
                .characteristic(*id)
                .and_then(group_key)
                .filter(|_| sub_numbers)
        })
        .collect();
    let mut numbers = Vec::with_capacity(order.len());
    let mut base: u32 = 0;
    let mut start = 0;
    while start < keys.len() {
        let mut end = start + 1;
        if let Some(key) = &keys[start] {
            while end < keys.len() && keys[end].as_ref() == Some(key) {
                end += 1;
            }
        }
        base = base.saturating_add(1);
        if end - start == 1 {
            numbers.push(DisplayNumber::plain(base));
        } else {
            for sub in 1..=(end - start) {
                let sub = u32::try_from(sub).unwrap_or(u32::MAX);
                numbers.push(DisplayNumber::plain(base).with_sub(sub));
            }
        }
        start = end;
    }
    numbers
}

/// The placement order `strategy` gives the characteristics of `project` (FR-BAL-04, D-21).
/// See the module documentation for the rules.
pub fn strategy_order(project: &Project, strategy: NumberingStrategy) -> Vec<CharId> {
    let current: Vec<CharId> = project.characteristics.iter().map(|c| c.id).collect();
    if strategy == NumberingStrategy::Manual {
        return current;
    }
    let tolerance = row_tolerance(project);
    let sheets = sheet_order(project);
    // Characteristics by sheet ordinal; the ones without a point go last.
    let mut by_sheet: BTreeMap<usize, (&Sheet, Vec<Item>)> = BTreeMap::new();
    let mut unplaced = Vec::new();
    for (index, c) in project.characteristics.iter().enumerate() {
        let placed = reference_point(project, c).and_then(|(sheet, at)| {
            sheets
                .get(&sheet)
                .map(|(ordinal, sheet)| (*ordinal, *sheet, at))
        });
        match placed {
            Some((ordinal, sheet, at)) => by_sheet
                .entry(ordinal)
                .or_insert_with(|| (sheet, Vec::new()))
                .1
                .push(Item {
                    index,
                    at,
                    shown: shown_point(sheet, at),
                }),
            None => unplaced.push(index),
        }
    }
    let mut ordered: Vec<usize> = Vec::with_capacity(current.len());
    for (sheet, items) in by_sheet.into_values() {
        match strategy {
            NumberingStrategy::View | NumberingStrategy::ViewClockwise => {
                let clockwise = strategy == NumberingStrategy::ViewClockwise;
                order_by_view(sheet, items, clockwise, tolerance, &mut ordered);
            }
            _ => order_by_zone(sheet, items, tolerance, &mut ordered),
        }
    }
    ordered.extend(unplaced);
    if strategy == NumberingStrategy::Kind {
        let characteristics = &project.characteristics;
        ordered.sort_by_key(|i| kind_rank(characteristics[*i].kind));
    }
    let ordered = gather_groups(project, &ordered);
    ordered.into_iter().map(|i| current[i]).collect()
}

/// The point a characteristic is ordered by: the leader anchor of its first balloon, else the
/// center of its first source region.
pub fn reference_point(project: &Project, c: &Characteristic) -> Option<(SheetId, Point)> {
    project
        .balloons_of(c.id)
        .next()
        .map(|b| (b.sheet, b.anchor))
        .or_else(|| c.sources.first().map(|s| (s.sheet, s.region.center)))
        .filter(|(_, p)| p.x.is_finite() && p.y.is_finite())
}

/// A characteristic to order: its index in the current order, its point in sheet space and the
/// same point as shown on screen.
#[derive(Debug, Clone, Copy)]
struct Item {
    index: usize,
    at: Point,
    shown: Point,
}

/// Row tolerance of the reading order: one default balloon diameter in sheet units.
fn row_tolerance(project: &Project) -> f64 {
    let size = project.settings.balloon_style.size_mm;
    if size.is_finite() && size > 0.0 {
        size * UNITS_PER_MM
    } else {
        7.0 * UNITS_PER_MM
    }
}

/// Sheets by ID with their ordinal: the current revision in page order first, then the sheets
/// of older revisions.
fn sheet_order(project: &Project) -> HashMap<SheetId, (usize, &Sheet)> {
    let current = project.current_revision();
    let others = project
        .revisions
        .iter()
        .filter(|r| r.id != project.current_revision);
    let mut map = HashMap::new();
    for (ordinal, sheet) in current
        .into_iter()
        .chain(others)
        .flat_map(|r| &r.sheets)
        .enumerate()
    {
        map.entry(sheet.id).or_insert((ordinal, sheet));
    }
    map
}

/// `p` in the frame the user sees: the sheet turned clockwise by its view rotation, origin at
/// the top left of the turned sheet.
fn shown_point(sheet: &Sheet, p: Point) -> Point {
    let Size { width, height } = sheet.size;
    match sheet.rotation {
        Rotation::Deg0 => p,
        Rotation::Deg90 => Point {
            x: height - p.y,
            y: p.x,
        },
        Rotation::Deg180 => Point {
            x: width - p.x,
            y: height - p.y,
        },
        Rotation::Deg270 => Point {
            x: p.y,
            y: width - p.x,
        },
    }
}

/// Indexes of `items` in reading order (see the module documentation).
fn reading_order(mut items: Vec<Item>, tolerance: f64) -> Vec<usize> {
    items.sort_by(|a, b| a.shown.y.total_cmp(&b.shown.y).then(a.index.cmp(&b.index)));
    let mut out = Vec::with_capacity(items.len());
    let mut start = 0;
    while start < items.len() {
        let top = items[start].shown.y;
        let mut end = start + 1;
        while end < items.len() && items[end].shown.y - top <= tolerance {
            end += 1;
        }
        let row = &mut items[start..end];
        row.sort_by(|a, b| {
            a.shown
                .x
                .total_cmp(&b.shown.x)
                .then(a.shown.y.total_cmp(&b.shown.y))
                .then(a.index.cmp(&b.index))
        });
        out.extend(row.iter().map(|i| i.index));
        start = end;
    }
    out
}

/// Sheet, then zone, then reading order (D-21). Without a grid, reading order on the sheet.
fn order_by_zone(sheet: &Sheet, items: Vec<Item>, tolerance: f64, out: &mut Vec<usize>) {
    let Some(grid) = sheet.zone_grid.as_ref().filter(|g| g.validate().is_ok()) else {
        out.extend(reading_order(items, tolerance));
        return;
    };
    let (columns, rows) = (grid.columns(), grid.rows());
    let cell = |value: f64, start: f64, length: f64, count: usize| -> usize {
        let fraction = (value - start) / length;
        let index = (fraction * small(count)).floor().max(0.0);
        // The value is clamped to 0 below and to the cell count right after; truncation and
        // sign loss cannot happen for the at most 100 cells of a grid.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to 0..count"
        )]
        let index = index.min(small(count - 1)) as usize;
        index
    };
    let frame = grid.frame;
    let mut zones: BTreeMap<(usize, usize), Vec<Item>> = BTreeMap::new();
    for item in items {
        let column = cell(item.at.x, frame.origin.x, frame.size.width, columns);
        let row = cell(item.at.y, frame.origin.y, frame.size.height, rows);
        zones.entry((column, row)).or_default().push(item);
    }
    // Zones in reading order of their centers as shown.
    let center = |(column, row): (usize, usize)| -> Point {
        shown_point(
            sheet,
            Point {
                x: frame.origin.x + (small(column) + 0.5) * frame.size.width / small(columns),
                y: frame.origin.y + (small(row) + 0.5) * frame.size.height / small(rows),
            },
        )
    };
    let mut keys: Vec<(usize, usize)> = zones.keys().copied().collect();
    keys.sort_by(|a, b| {
        let (pa, pb) = (center(*a), center(*b));
        pa.y.total_cmp(&pb.y).then(pa.x.total_cmp(&pb.x))
    });
    for key in keys {
        if let Some(items) = zones.remove(&key) {
            out.extend(reading_order(items, tolerance));
        }
    }
}

/// A grid index or count as a float; grids have at most 100 cells per axis.
fn small(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

fn contains(rect: &Rect, p: Point) -> bool {
    p.x >= rect.origin.x
        && p.y >= rect.origin.y
        && p.x <= rect.origin.x + rect.size.width
        && p.y <= rect.origin.y + rect.size.height
}

/// View by view in drawing order; outside every view last in reading order.
fn order_by_view(
    sheet: &Sheet,
    items: Vec<Item>,
    clockwise: bool,
    tolerance: f64,
    out: &mut Vec<usize>,
) {
    let views: Vec<(usize, &Rect)> = sheet
        .views
        .iter()
        .enumerate()
        .filter(|(_, v)| v.validate().is_ok())
        .map(|(i, v)| (i, &v.rect))
        .collect();
    let mut inside: BTreeMap<usize, Vec<Item>> = BTreeMap::new();
    let mut outside = Vec::new();
    for item in items {
        let smallest =
            views
                .iter()
                .filter(|(_, r)| contains(r, item.at))
                .min_by(|(ia, a), (ib, b)| {
                    let area = |r: &Rect| r.size.width * r.size.height;
                    area(a).total_cmp(&area(b)).then(ia.cmp(ib))
                });
        match smallest {
            Some((view, _)) => inside.entry(*view).or_default().push(item),
            None => outside.push(item),
        }
    }
    for (view, items) in inside {
        if clockwise {
            let center = shown_point(sheet, sheet.views[view].rect.center());
            out.extend(clockwise_order(items, center));
        } else {
            out.extend(reading_order(items, tolerance));
        }
    }
    out.extend(reading_order(outside, tolerance));
}

/// Clockwise as shown, starting at 12 o'clock; equal angles from the center outwards.
fn clockwise_order(mut items: Vec<Item>, center: Point) -> Vec<usize> {
    let key = |item: &Item| -> (f64, f64) {
        let (dx, dy) = (item.shown.x - center.x, item.shown.y - center.y);
        // y grows downwards, so "up" is -dy; atan2(dx, -dy) is 0 at 12 o'clock and grows
        // clockwise on screen.
        let mut angle = dx.atan2(-dy);
        if angle < 0.0 {
            angle += std::f64::consts::TAU;
        }
        (angle, dx.hypot(dy))
    };
    items.sort_by(|a, b| {
        let (ka, kb) = (key(a), key(b));
        ka.0.total_cmp(&kb.0)
            .then(ka.1.total_cmp(&kb.1))
            .then(a.index.cmp(&b.index))
    });
    items.into_iter().map(|i| i.index).collect()
}

/// Position of a kind in the `Kind` strategy: the declaration order of [`CharacteristicKind`].
fn kind_rank(kind: CharacteristicKind) -> u8 {
    use CharacteristicKind as K;
    match kind {
        K::Linear => 0,
        K::Diameter => 1,
        K::Radius => 2,
        K::SphericalRadius => 3,
        K::Angle => 4,
        K::Chamfer => 5,
        K::Thread => 6,
        K::Counterbore => 7,
        K::Countersink => 8,
        K::Depth => 9,
        K::SurfaceTexture => 10,
        K::Geometric => 11,
        K::Note => 12,
        K::FlagNote => 13,
        K::MaterialProcess => 14,
        K::Other => 15,
    }
}

/// Identity of a multi-instance group: the sheet and exact region of the first source, the kind
/// and the requirement text. The copies of one split callout agree in all of them; different
/// callouts read from one box (a diameter and its depth) differ in kind or text.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GroupKey {
    sheet: SheetId,
    region: [u64; 5],
    kind: CharacteristicKind,
    text: String,
}

fn group_key(c: &Characteristic) -> Option<GroupKey> {
    let source = c.sources.first()?;
    let r = &source.region;
    Some(GroupKey {
        sheet: source.sheet,
        region: [
            r.center.x.to_bits(),
            r.center.y.to_bits(),
            r.size.width.to_bits(),
            r.size.height.to_bits(),
            r.angle.to_bits(),
        ],
        kind: c.kind,
        text: c.requirement_text.clone(),
    })
}

/// Moves every member of a multi-instance group to the place of its first member, keeping the
/// relative order of the members (D-22).
fn gather_groups(project: &Project, ordered: &[usize]) -> Vec<usize> {
    let characteristics = &project.characteristics;
    let mut members: HashMap<GroupKey, Vec<usize>> = HashMap::new();
    for &i in ordered {
        if let Some(key) = group_key(&characteristics[i]) {
            members.entry(key).or_default().push(i);
        }
    }
    let mut out = Vec::with_capacity(ordered.len());
    for &i in ordered {
        match group_key(&characteristics[i]) {
            Some(key) => {
                if let Some(group) = members.remove(&key) {
                    out.extend(group);
                }
            }
            None => out.push(i),
        }
    }
    out
}
