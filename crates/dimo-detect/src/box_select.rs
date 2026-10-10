//! Box select (FR-REC-01, FR-REC-02, ADR 0006): the text inside a box drawn by the user
//! becomes proposals.
//!
//! Steps (spec 08 stages 5 to 7, for the runs of one box):
//!
//! 1. Runs are grouped by reading direction (rotation). The direction with the most characters
//!    comes first, so a box around rotated text reads it in its own direction.
//! 2. Within one direction, runs whose extents across the reading direction overlap form a
//!    **line**: the main text and the smaller stacked tolerances beside it are one line,
//!    separate text lines are not.
//! 3. Within a line, runs that overlap along the reading direction are a **stack** (upper
//!    above lower). The line text is the stacks in reading order, each stack top to bottom,
//!    joined with one space: `Ø30 H7 +0.0203 -0` (the `dimo-notation` convention).
//! 4. A limit dimension written as two lines without a main text (`12.02` above `11.98`) is
//!    one line again: see [`join_limit_pairs`].
//! 5. If every line parses as a callout on its own, each line is a proposal (a box around
//!    several dimensions). Otherwise all lines are joined into one text (a note over several
//!    lines) and read as one proposal.
//! 6. Each text is parsed and interpreted by [`read_callout`]. A line made only of runs with a
//!    basic dimension frame is read as basic (M2 decision 6).
//!
//! Nothing here changes the project: the result are proposals that only an accept command
//! turns into characteristics (ADR 0006).

use dimo_core::characteristic::{Origin, SourceRegion, TextSource, Unit};
use dimo_core::geometry::{OrientedBox, Point, Size};
use dimo_core::id::SheetId;
use dimo_core::proposal::{BalloonPlacement, ParseHint, Proposal};
use dimo_notation::{Callout, Kind, Tolerance, parse_callout};
use dimo_pdf::{RegionRun, RegionText};
use rust_decimal::Decimal;

use crate::interpret::Interpreter;
use crate::read::read_callout;

/// Two rotations closer than this, in degrees, are one reading direction.
const SAME_DIRECTION_DEG: f64 = 5.0;
/// Runs are on one line if their extents across the reading direction overlap by at least this
/// share of the smaller height.
const LINE_OVERLAP: f64 = 0.25;
/// Runs are stacked if their extents along the reading direction overlap by at least this share
/// of the narrower one.
const STACK_OVERLAP: f64 = 0.2;
/// Two lines are a stacked limit pair only if the gap between them across the reading
/// direction is at most this share of the smaller height.
const LIMIT_GAP: f64 = 0.5;
/// ... and the smaller height (font size) is at least this share of the larger one, so a
/// 7 pt deviation under a 10 pt main text never pairs.
const LIMIT_HEIGHT_RATIO: f64 = 0.8;
/// ... and the lower limit is at least this share of the upper one (`12.02` above `11.98`, not
/// `52.61` above `31.05`).
const LIMIT_MIN_RATIO: Decimal = Decimal::from_parts(8, 0, 0, false, 1);

/// What box select needs besides the text of the region.
pub struct BoxSelectContext<'a> {
    /// Turns parsed callouts into limits (the tolerance engine in the app).
    pub interpreter: &'a dyn Interpreter,
    /// Unit of the sheet, for callouts that write none (D-20).
    pub drawing_unit: Unit,
    /// Where the balloon of the box goes, as the box tool places it. With several proposals
    /// each balloon keeps the same offset from its own text.
    pub placement: BalloonPlacement,
    /// Job ID stored on the proposals.
    pub job_id: Option<u32>,
}

/// Proposals for the text inside `region` of `sheet` (FR-REC-01, FR-REC-02). Empty when the
/// region holds no text. See the module docs for the rules.
pub fn box_select(
    sheet: SheetId,
    region: &OrientedBox,
    text: &RegionText,
    context: &BoxSelectContext<'_>,
) -> Vec<Proposal> {
    box_select_with_notes(sheet, region, text, context)
        .into_iter()
        .map(|p| p.proposal)
        .collect()
}

/// A proposal with what the interpreter could not decide. The notes are shown with the
/// proposal and never stored in the project.
#[derive(Debug, Clone, PartialEq)]
pub struct Proposed {
    /// The proposal.
    pub proposal: Proposal,
    /// Notes of the interpreter, such as an angle whose shorter leg is unknown.
    pub notes: Vec<dimo_tolerance::Note>,
}

/// Like [`box_select`], with the interpreter's notes of each proposal.
pub fn box_select_with_notes(
    sheet: SheetId,
    region: &OrientedBox,
    text: &RegionText,
    context: &BoxSelectContext<'_>,
) -> Vec<Proposed> {
    let lines = join_limit_pairs(lines(&text.runs));
    if lines.is_empty() {
        return Vec::new();
    }
    let separate = lines.len() > 1 && lines.iter().all(|l| parse_callout(&l.text).is_ok());
    let groups: Vec<Line> = if separate || lines.len() == 1 {
        lines
    } else {
        vec![Line::join(&lines)]
    };
    let single = groups.len() == 1;
    groups
        .into_iter()
        .map(|line| {
            let reading = read_callout(
                &line.text,
                line.framed,
                context.drawing_unit,
                context.interpreter,
            );
            let source_box = if single { *region } else { line.bounds };
            let placement = if single {
                context.placement
            } else {
                shifted(context.placement, region.center, line.bounds.center)
            };
            let mut parse_hints = reading.parse_hints;
            if line.stacked && reading.parse_error.is_none() {
                parse_hints.push(ParseHint::StackedLinesJoined);
            }
            let proposal = Proposal {
                kind: reading.kind,
                requirement_text: reading.requirement_text,
                nominal: reading.nominal,
                unit: reading.unit,
                upper_dev: reading.upper_dev,
                lower_dev: reading.lower_dev,
                upper_limit: reading.upper_limit,
                lower_limit: reading.lower_limit,
                fit: reading.fit,
                derivation: reading.derivation,
                quantity: reading.quantity,
                inspect: reading.inspect,
                source: SourceRegion {
                    sheet,
                    region: source_box,
                    text_source: TextSource::PdfText,
                    raw_text: Some(line.text),
                },
                origin: Origin::BoxSelect,
                placement,
                parse_error: reading.parse_error,
                parse_hints,
                job_id: context.job_id,
                engines: reading.engines,
            };
            Proposed {
                proposal,
                notes: reading.notes,
            }
        })
        .collect()
}

/// The placement moved by the offset from `from` to `to`.
fn shifted(p: BalloonPlacement, from: Point, to: Point) -> BalloonPlacement {
    let (dx, dy) = (to.x - from.x, to.y - from.y);
    BalloonPlacement {
        position: Point {
            x: p.position.x + dx,
            y: p.position.y + dy,
        },
        anchor: Point {
            x: p.anchor.x + dx,
            y: p.anchor.y + dy,
        },
    }
}

/// One run in the frame of its reading direction.
#[derive(Debug, Clone)]
struct Piece<'a> {
    text: &'a str,
    /// Extent along the reading direction.
    along: (f64, f64),
    /// Extent across it, toward the descenders positive.
    across: (f64, f64),
    framed: bool,
}

/// One text line of the box: its joined text and bounds.
#[derive(Debug, Clone)]
struct Line {
    text: String,
    bounds: OrientedBox,
    /// Extent along the reading direction (frame of [`direction`] of `bounds.angle`).
    along: (f64, f64),
    /// Extent across the reading direction, toward the descenders positive.
    across: (f64, f64),
    /// The line has stacked runs (tolerances above each other).
    stacked: bool,
    /// Every run of the line has a basic dimension frame.
    framed: bool,
}

impl Line {
    /// All lines as one text, for a note over several lines.
    fn join(lines: &[Line]) -> Line {
        let text = lines
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let first = &lines[0];
        Line {
            text,
            bounds: first.bounds,
            along: first.along,
            across: first.across,
            stacked: lines.iter().any(|l| l.stacked),
            framed: false,
        }
    }
}

/// Joins a stacked limit dimension written without a main text (`12.02` above `11.98`, T2.9)
/// into one line `12.02 11.98`, which the parser reads as a limit dimension.
///
/// Two neighbouring lines of one reading direction are joined when all of these hold: the upper
/// line is a plain size (no tolerance, fit, quantity or reference; linear, diameter or radius),
/// the lower line a bare number; they overlap along the reading direction like a stack; the gap
/// between them is at most [`LIMIT_GAP`] of the smaller height and the smaller height is at
/// least [`LIMIT_HEIGHT_RATIO`] of the larger; both values have the same number of decimals,
/// at least one (limits are written to the same precision, `12.02` and `11.98`); the upper
/// value is larger and the lower one at least [`LIMIT_MIN_RATIO`] of it. A column of separate
/// dimensions fails one of the value rules (or the gap), so it still gives one proposal per
/// line.
fn join_limit_pairs(lines: Vec<Line>) -> Vec<Line> {
    let mut out: Vec<Line> = Vec::with_capacity(lines.len());
    for line in lines {
        match out.pop() {
            Some(upper) if is_limit_pair(&upper, &line) => out.push(join_pair(&upper, &line)),
            Some(upper) => {
                out.push(upper);
                out.push(line);
            }
            None => out.push(line),
        }
    }
    out
}

fn is_limit_pair(upper: &Line, lower: &Line) -> bool {
    if angle_diff(upper.bounds.angle, lower.bounds.angle) > SAME_DIRECTION_DEG {
        return false;
    }
    let (hu, hl) = (
        upper.across.1 - upper.across.0,
        lower.across.1 - lower.across.0,
    );
    let (wu, wl) = (upper.along.1 - upper.along.0, lower.along.1 - lower.along.0);
    let gap = lower.across.0 - upper.across.1;
    let stacked = overlap(upper.along, lower.along) >= STACK_OVERLAP * wu.min(wl).max(f64::EPSILON);
    if !stacked || gap > LIMIT_GAP * hu.min(hl) || hu.min(hl) < LIMIT_HEIGHT_RATIO * hu.max(hl) {
        return false;
    }
    let (Ok(a), Ok(b)) = (parse_callout(&upper.text), parse_callout(&lower.text)) else {
        return false;
    };
    let plain = |c: &Callout| {
        c.tolerance.is_none()
            && c.fit.is_none()
            && c.quantity.is_none()
            && c.suffixes.is_empty()
            && !c.reference
            && !c.basic
    };
    let (hi, lo) = (a.nominal.value, b.nominal.value);
    plain(&a)
        && plain(&b)
        && matches!(a.kind, Kind::Linear | Kind::Diameter | Kind::Radius)
        && b.kind == Kind::Linear
        && hi.scale() == lo.scale()
        && hi.scale() >= 1
        && hi > lo
        && lo > Decimal::ZERO
        && lo >= hi * LIMIT_MIN_RATIO
        && parse_callout(&format!("{} {}", upper.text, lower.text))
            .is_ok_and(|c| matches!(c.tolerance, Some(Tolerance::Limits { .. })))
}

/// One line from a limit pair: text upper then lower, bounds around both.
fn join_pair(upper: &Line, lower: &Line) -> Line {
    let along = (
        upper.along.0.min(lower.along.0),
        upper.along.1.max(lower.along.1),
    );
    let across = (upper.across.0, lower.across.1.max(upper.across.1));
    let angle = upper.bounds.angle;
    Line {
        text: format!("{} {}", upper.text, lower.text),
        bounds: frame_box(angle, along, across),
        along,
        across,
        stacked: true,
        framed: upper.framed && lower.framed,
    }
}

/// The oriented box with these extents in the reading frame of `angle`.
fn frame_box(angle: f64, along: (f64, f64), across: (f64, f64)) -> OrientedBox {
    let dir = direction(angle);
    let (u, v) = (
        f64::midpoint(along.0, along.1),
        f64::midpoint(across.0, across.1),
    );
    OrientedBox {
        center: Point {
            x: u * dir.0 - v * dir.1,
            y: u * dir.1 + v * dir.0,
        },
        size: Size {
            width: along.1 - along.0,
            height: across.1 - across.0,
        },
        angle,
    }
}

/// Unit vector of the reading direction (sheet space, y down).
fn direction(angle: f64) -> (f64, f64) {
    let rad = angle.to_radians();
    (rad.cos(), -rad.sin())
}

fn angle_diff(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

fn overlap(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.1.min(b.1) - a.0.max(b.0)).max(0.0)
}

/// Lines of all reading directions, the direction with the most characters first.
fn lines(runs: &[RegionRun]) -> Vec<Line> {
    // Reading directions with their runs, in first seen order.
    let mut directions: Vec<(f64, Vec<&RegionRun>)> = Vec::new();
    for run in runs {
        let angle = run.frame.angle;
        match directions
            .iter_mut()
            .find(|(a, _)| angle_diff(*a, angle) <= SAME_DIRECTION_DEG)
        {
            Some((_, members)) => members.push(run),
            None => directions.push((angle, vec![run])),
        }
    }
    let weight = |members: &Vec<&RegionRun>| -> usize {
        members.iter().map(|r| r.run.text.chars().count()).sum()
    };
    // Stable: equal weights keep content order.
    directions.sort_by_key(|(_, members)| std::cmp::Reverse(weight(members)));
    directions
        .into_iter()
        .flat_map(|(angle, members)| lines_in_direction(angle, &members))
        .collect()
}

/// Lines of the runs of one reading direction, top to bottom in that direction.
fn lines_in_direction(angle: f64, runs: &[&RegionRun]) -> Vec<Line> {
    let dir = direction(angle);
    let mut pieces: Vec<Piece<'_>> = runs
        .iter()
        .map(|r| {
            let f = &r.frame;
            let u = f.center.x * dir.0 + f.center.y * dir.1;
            let v = -f.center.x * dir.1 + f.center.y * dir.0;
            let (hw, hh) = (f.size.width / 2.0, f.size.height / 2.0);
            Piece {
                text: r.run.text.as_str(),
                along: (u - hw, u + hw),
                across: (v - hh, v + hh),
                framed: r.framed,
            }
        })
        .collect();
    pieces.sort_by(|a, b| a.across.0.total_cmp(&b.across.0));

    // Lines: chains of pieces overlapping across the reading direction.
    let mut groups: Vec<((f64, f64), Vec<Piece<'_>>)> = Vec::new();
    for piece in pieces {
        let height = piece.across.1 - piece.across.0;
        if let Some((range, members)) = groups.last_mut() {
            let min_height = height.min(range.1 - range.0).max(f64::EPSILON);
            if overlap(*range, piece.across) >= LINE_OVERLAP * min_height {
                *range = (range.0.min(piece.across.0), range.1.max(piece.across.1));
                members.push(piece);
                continue;
            }
        }
        groups.push((piece.across, vec![piece]));
    }
    groups
        .into_iter()
        .map(|(_, members)| line(angle, members))
        .collect()
}

/// One line from its pieces: stacks in reading order, each top to bottom.
fn line(angle: f64, mut pieces: Vec<Piece<'_>>) -> Line {
    pieces.sort_by(|a, b| a.along.0.total_cmp(&b.along.0));
    let mut stacks: Vec<((f64, f64), Vec<&Piece<'_>>)> = Vec::new();
    for piece in &pieces {
        let width = piece.along.1 - piece.along.0;
        if let Some((range, members)) = stacks.last_mut() {
            let min_width = width.min(range.1 - range.0).max(f64::EPSILON);
            if overlap(*range, piece.along) >= STACK_OVERLAP * min_width {
                *range = (range.0.min(piece.along.0), range.1.max(piece.along.1));
                members.push(piece);
                continue;
            }
        }
        stacks.push((piece.along, vec![piece]));
    }
    let stacked = stacks.iter().any(|(_, members)| members.len() > 1);
    let mut texts = Vec::new();
    for (_, mut members) in stacks {
        members.sort_by(|a, b| {
            let ca = a.across.0 + a.across.1;
            let cb = b.across.0 + b.across.1;
            ca.total_cmp(&cb)
        });
        texts.extend(members.iter().map(|p| p.text));
    }
    let along = pieces
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |acc, p| {
            (acc.0.min(p.along.0), acc.1.max(p.along.1))
        });
    let across = pieces
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |acc, p| {
            (acc.0.min(p.across.0), acc.1.max(p.across.1))
        });
    Line {
        text: texts.join(" "),
        bounds: frame_box(angle, along, across),
        along,
        across,
        stacked,
        framed: pieces.iter().all(|p| p.framed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpret::CalloutOnly;
    use dimo_core::Environment as _;
    use dimo_core::characteristic::CharacteristicKind;
    use dimo_core::derivation::DerivationRule;
    use dimo_pdf::{SheetRect, TextRun};
    use rust_decimal::Decimal;

    /// A run of `text` with a frame centered at `(x, y)`, `size` high, 0.6 em per character.
    fn run(text: &str, x: f64, y: f64, size: f64, angle: f64) -> RegionRun {
        #[allow(clippy::cast_precision_loss)]
        let width = 0.6 * size * text.chars().count() as f64;
        RegionRun {
            run: TextRun {
                text: text.to_owned(),
                bbox: SheetRect::new(x - width / 2.0, y - size / 2.0, width, size),
                rotation: angle.rem_euclid(360.0),
                font_name: "Sans".to_owned(),
                font_size: size,
            },
            frame: OrientedBox {
                center: Point { x, y },
                size: Size {
                    width,
                    height: size,
                },
                angle,
            },
            framed: false,
        }
    }

    fn region() -> OrientedBox {
        OrientedBox {
            center: Point { x: 100.0, y: 100.0 },
            size: Size {
                width: 100.0,
                height: 50.0,
            },
            angle: 0.0,
        }
    }

    fn select(runs: Vec<RegionRun>) -> Vec<Proposal> {
        let text = RegionText {
            chars: Vec::new(),
            runs,
        };
        let context = BoxSelectContext {
            interpreter: &CalloutOnly,
            drawing_unit: Unit::Mm,
            placement: BalloonPlacement {
                position: Point { x: 160.0, y: 60.0 },
                anchor: Point { x: 150.0, y: 75.0 },
            },
            job_id: Some(3),
        };
        let sheet = SheetId::from_uuid(dimo_core::FixedEnvironment::new().new_uuid());
        box_select(sheet, &region(), &text, &context)
    }

    fn texts(proposals: &[Proposal]) -> Vec<&str> {
        proposals
            .iter()
            .map(|p| p.requirement_text.as_str())
            .collect()
    }

    // FR-REC-01: stacked tolerances join main, upper, lower, whatever the content order.
    #[test]
    fn stacked_tolerances_join_upper_above_lower() {
        let runs = vec![
            run("-0", 140.0, 104.0, 7.0, 0.0),
            run("Ø30 H7", 100.0, 100.0, 10.0, 0.0),
            run("+0.0203", 145.0, 96.0, 7.0, 0.0),
        ];
        let p = select(runs);
        assert_eq!(texts(&p), ["Ø30 H7 +0.0203 -0"]);
        let p = &p[0];
        assert_eq!(p.kind, CharacteristicKind::Diameter);
        assert_eq!(p.upper_limit, Some(Decimal::new(300_203, 4)));
        assert_eq!(p.lower_limit, Some(Decimal::new(30, 0)));
        assert_eq!(
            p.derivation.as_ref().map(|d| &d.rule),
            Some(&DerivationRule::Explicit)
        );
        assert_eq!(p.parse_hints, [ParseHint::StackedLinesJoined]);
        assert_eq!(p.origin, Origin::BoxSelect);
        assert_eq!(p.source.region, region());
        assert_eq!(p.job_id, Some(3));
        assert_eq!(p.engines.len(), 2);
    }

    #[test]
    fn several_dimensions_give_several_proposals() {
        let runs = vec![
            run("31.05", 100.0, 80.0, 10.0, 0.0),
            run("52.61", 100.0, 100.0, 10.0, 0.0),
            run("71.04", 100.0, 120.0, 10.0, 0.0),
        ];
        let p = select(runs);
        assert_eq!(texts(&p), ["31.05", "52.61", "71.04"]);
        // Each balloon keeps the box's offset from its own text.
        assert_eq!(p[0].placement.position, Point { x: 160.0, y: 40.0 });
        assert_eq!(p[2].placement.anchor, Point { x: 150.0, y: 95.0 });
        assert_eq!(p[1].source.region.center, Point { x: 100.0, y: 100.0 });
    }

    #[test]
    fn a_note_over_two_lines_is_one_proposal() {
        let runs = vec![
            run("BREAK ALL SHARP", 100.0, 90.0, 10.0, 0.0),
            run("EDGES", 80.0, 104.0, 10.0, 0.0),
        ];
        let p = select(runs);
        assert_eq!(texts(&p), ["BREAK ALL SHARP EDGES"]);
        assert_eq!(p[0].kind, CharacteristicKind::Note);
        assert!(p[0].parse_error.is_some());
        assert_eq!(p[0].upper_limit, None);
    }

    // Spec 08 stage 2: rotated text reads in its own direction.
    #[test]
    fn rotated_callout_reads_bottom_to_top() {
        // Text read bottom to top: "upper" stack line lies left on the sheet (smaller x).
        let runs = vec![
            run("+0.1", 95.0, 80.0, 7.0, 90.0),
            run("25", 100.0, 100.0, 10.0, 90.0),
            run("-0.2", 104.0, 85.0, 7.0, 90.0),
        ];
        let p = select(runs);
        assert_eq!(texts(&p), ["25 +0.1 -0.2"]);
        assert_eq!(p[0].upper_limit, Some(Decimal::new(251, 1)));
        assert_eq!(p[0].lower_limit, Some(Decimal::new(248, 1)));
    }

    #[test]
    fn mixed_directions_take_the_dominant_one_first() {
        let runs = vec![
            run("R5", 60.0, 100.0, 10.0, 90.0),
            run("Ø12.5", 100.0, 100.0, 10.0, 0.0),
        ];
        assert_eq!(texts(&select(runs)), ["Ø12.5", "R5"]);
    }

    // M2 decision 6: a framed run is a basic dimension, not inspected.
    #[test]
    fn framed_run_is_basic() {
        let mut framed = run("40", 100.0, 100.0, 10.0, 0.0);
        framed.framed = true;
        let p = select(vec![framed]);
        assert!(!p[0].inspect);
        let p = select(vec![run("40", 100.0, 100.0, 10.0, 0.0)]);
        assert!(p[0].inspect);
    }

    // FR-REC-01, T2.9 regression: a limit dimension written as two lines without a main text
    // is one proposal (T2.6 gave two).
    #[test]
    fn stacked_limit_dimension_is_one_proposal() {
        let runs = vec![
            run("11.98", 100.0, 106.0, 10.0, 0.0),
            run("12.02", 100.0, 94.0, 10.0, 0.0),
        ];
        let p = select(runs);
        assert_eq!(texts(&p), ["12.02 11.98"]);
        assert_eq!(p[0].nominal, Some(Decimal::new(1202, 2)));
        assert_eq!(p[0].upper_limit, Some(Decimal::new(1202, 2)));
        assert_eq!(p[0].lower_limit, Some(Decimal::new(1198, 2)));
        assert_eq!(p[0].parse_hints, [ParseHint::StackedLinesJoined]);
        assert_eq!(p[0].source.region, region());

        // With a diameter sign and decimal commas, rotated by 90 degrees (upper line left).
        let runs = vec![
            run("Ø8,02", 94.0, 100.0, 10.0, 90.0),
            run("7,98", 106.0, 100.0, 10.0, 90.0),
        ];
        let p = select(runs);
        assert_eq!(texts(&p), ["Ø8,02 7,98"]);
        assert_eq!(p[0].kind, CharacteristicKind::Diameter);
        assert_eq!(p[0].lower_limit, Some(Decimal::new(798, 2)));
    }

    #[test]
    fn close_lines_that_are_no_limit_pair_stay_separate() {
        let pair = |upper: &str, lower: &str| {
            texts(&select(vec![
                run(upper, 100.0, 94.0, 10.0, 0.0),
                run(lower, 100.0, 106.0, 10.0, 0.0),
            ]))
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
        };
        // Lower value above, values too far apart, a prefix on the lower line, a tolerance,
        // whole numbers, different decimals.
        assert_eq!(pair("11.98", "12.02"), ["11.98", "12.02"]);
        assert_eq!(pair("25", "22"), ["25", "22"]);
        assert_eq!(pair("12.5", "11.98"), ["12.5", "11.98"]);
        assert_eq!(pair("52.61", "31.05"), ["52.61", "31.05"]);
        assert_eq!(pair("12.02", "Ø11.98"), ["12.02", "Ø11.98"]);
        assert_eq!(pair("12.02±0.1", "11.98"), ["12.02±0.1", "11.98"]);
        // A smaller font under a larger one (a deviation, not a limit).
        let p = select(vec![
            run("12.02", 100.0, 94.0, 10.0, 0.0),
            run("11.98", 100.0, 103.0, 7.0, 0.0),
        ]);
        assert_eq!(texts(&p), ["12.02", "11.98"]);
        // Far apart across the reading direction.
        let p = select(vec![
            run("12.02", 100.0, 80.0, 10.0, 0.0),
            run("11.98", 100.0, 106.0, 10.0, 0.0),
        ]);
        assert_eq!(texts(&p), ["12.02", "11.98"]);
    }

    #[test]
    fn empty_region_gives_nothing() {
        assert_eq!(select(Vec::new()), Vec::<Proposal>::new());
    }
}
