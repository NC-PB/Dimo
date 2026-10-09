//! The balloons of a project as a [`BalloonOverlay`] for the ballooned PDF (FR-EXP-01, D-24).
//!
//! This is the one place that maps the domain model (`dimo_core` balloons, characteristics and
//! styles) to drawing primitives. Sizes come from [`BalloonStyle::layout`], the rule the
//! viewport uses as well (`BALLOON_METRICS`), so the PDF shows the balloons as the app does.
//!
//! - Only sheets of the current revision are drawn; the overlay sheet is the sheet index.
//! - Balloons of rejected characteristics are left out, as in the characteristic list: they
//!   are kept for traceability, not inspected.
//! - The text is the characteristic number; the leader uses the outline color and width.
//! - Balloons are drawn in placement order, so later balloons lie on top, as in the viewport.
//!
//! [`BalloonStyle::layout`]: dimo_core::BalloonStyle::layout

use dimo_core::{CharacteristicStatus, Color, Project, Timestamp};

use crate::overlay::{
    Balloon, BalloonOutput, BalloonOverlay, BalloonShape, Leader, PdfDate, Rgb, SheetBalloons,
    SheetPoint, Stroke,
};

fn rgb(color: &Color) -> Rgb {
    let [r, g, b] = color.rgb();
    Rgb::new(r, g, b)
}

fn shape(shape: dimo_core::BalloonShape) -> BalloonShape {
    match shape {
        dimo_core::BalloonShape::Circle => BalloonShape::Circle,
        dimo_core::BalloonShape::Flag => BalloonShape::Flag,
        dimo_core::BalloonShape::Rectangle => BalloonShape::Rectangle,
    }
}

/// The balloons of the current revision of `project`, written as `output`. Sheets without
/// balloons are left out.
pub fn project_overlay(project: &Project, output: BalloonOutput) -> BalloonOverlay {
    let mut sheets = Vec::new();
    let Some(revision) = project.current_revision() else {
        return BalloonOverlay { sheets, output };
    };
    let base = &project.settings.balloon_style;
    for sheet in &revision.sheets {
        let balloons: Vec<Balloon> = project
            .balloons
            .iter()
            .filter(|b| b.sheet == sheet.id)
            .filter_map(|b| {
                let characteristic = project.characteristic(b.characteristic)?;
                if characteristic.status == CharacteristicStatus::Rejected {
                    return None;
                }
                let style = base.with_override(&b.style);
                let text = characteristic.number.to_string();
                let layout = style.layout(&text);
                let stroke = Stroke {
                    color: rgb(&style.outline_color),
                    width: layout.stroke,
                };
                Some(Balloon {
                    shape: shape(style.shape),
                    center: SheetPoint::new(b.position.x, b.position.y),
                    width: layout.width,
                    height: layout.height,
                    fill: Some(rgb(&style.fill_color)),
                    outline: Some(stroke),
                    text,
                    text_color: rgb(&style.text_color),
                    text_size: layout.font_size,
                    leader: style.leader.then_some(Leader {
                        anchor: SheetPoint::new(b.anchor.x, b.anchor.y),
                        stroke,
                    }),
                })
            })
            .collect();
        if !balloons.is_empty() {
            sheets.push(SheetBalloons {
                sheet: sheet.index as usize,
                balloons,
            });
        }
    }
    BalloonOverlay { sheets, output }
}

impl PdfDate {
    /// The date and time of an RFC 3339 UTC timestamp, without fractions of a second. For
    /// annotation dates that come from the project (FR-EXP-11).
    pub fn from_timestamp(timestamp: &Timestamp) -> Option<Self> {
        // `YYYY-MM-DDTHH:MM:SS[.fff]Z`, checked by `Timestamp::parse`.
        let text = timestamp.as_str();
        let part = |range: std::ops::Range<usize>| text.get(range)?.parse::<u16>().ok();
        let small = |range| part(range).and_then(|v| u8::try_from(v).ok());
        let date = Self {
            year: part(0..4)?,
            month: small(5..7)?,
            day: small(8..10)?,
            hour: small(11..13)?,
            minute: small(14..16)?,
            second: small(17..19)?,
        };
        date.is_valid().then_some(date)
    }
}

#[cfg(test)]
mod tests {
    use dimo_core::{
        BalloonStyle, BalloonStyleOverride, Command, Document, DrawingRevision, Environment,
        FixedEnvironment, Point, ProjectInfo, RevisionId, Sha256Hex, Sheet, SheetId, SheetKind,
        Size,
    };

    use super::*;

    fn project(env: &mut FixedEnvironment) -> Project {
        let sheets = (0..2u32)
            .map(|i| {
                Sheet::new(
                    SheetId::from_uuid(env.new_uuid()),
                    i,
                    Size {
                        width: 842.0,
                        height: 595.0,
                    },
                    SheetKind::VectorText,
                )
            })
            .collect();
        Project::new(
            ProjectInfo::default(),
            DrawingRevision {
                id: RevisionId::from_uuid(env.new_uuid()),
                label: String::new(),
                file_name: "a.pdf".into(),
                sha256: Sha256Hex::parse(&"0".repeat(64)).unwrap(),
                imported_at: Timestamp::parse("2026-01-01T00:00:00Z").unwrap(),
                sheets,
            },
        )
    }

    fn add(doc: &mut Document, env: &mut FixedEnvironment, sheet: usize, x: f64) {
        let sheet = doc.project().current_revision().unwrap().sheets[sheet].id;
        doc.execute(
            Command::AddCharacteristic {
                sheet,
                position: Point { x, y: 100.0 },
                anchor: Point {
                    x: x + 30.0,
                    y: 130.0,
                },
                region: None,
                values: Vec::new(),
            },
            env,
        )
        .unwrap();
    }

    #[test]
    fn balloons_map_to_primitives_with_the_shared_layout() {
        let mut env = FixedEnvironment::new();
        let mut doc = Document::new(project(&mut env));
        for x in [100.0, 200.0, 300.0] {
            add(&mut doc, &mut env, 0, x);
        }
        add(&mut doc, &mut env, 1, 400.0);
        let ids: Vec<_> = doc.project().balloons.iter().map(|b| b.id).collect();
        doc.execute(
            Command::RestyleBalloons {
                ids: vec![ids[1]],
                style: BalloonStyleOverride {
                    shape: Some(dimo_core::BalloonShape::Flag),
                    leader: Some(false),
                    size_mm: Some(10.0),
                    ..Default::default()
                },
            },
            &mut env,
        )
        .unwrap();
        let rejected = doc.project().characteristics[2].id;
        doc.execute(
            Command::UpdateFields {
                ids: vec![rejected],
                values: vec![dimo_core::FieldValue::Status(
                    CharacteristicStatus::Rejected,
                )],
            },
            &mut env,
        )
        .unwrap();

        let overlay = project_overlay(doc.project(), BalloonOutput::PageContent);
        assert_eq!(overlay.sheets.len(), 2);
        assert_eq!(overlay.sheets[0].sheet, 0);
        assert_eq!(overlay.sheets[1].sheet, 1);
        let first = &overlay.sheets[0].balloons;
        assert_eq!(first.len(), 2, "the rejected balloon is left out");
        assert_eq!(first[0].text, "1");
        assert_eq!(first[1].text, "2");

        let base = BalloonStyle::default();
        let layout = base.layout("1");
        assert_eq!(first[0].shape, BalloonShape::Circle);
        assert_eq!(
            (first[0].width, first[0].height),
            (layout.width, layout.height)
        );
        assert_eq!(first[0].text_size, layout.font_size);
        assert_eq!(first[0].outline.unwrap().width, layout.stroke);
        assert_eq!(first[0].outline.unwrap().color, Rgb::new(0, 0x57, 0xB8));
        assert_eq!(first[0].fill, Some(Rgb::WHITE));
        assert_eq!(
            first[0].leader.unwrap().anchor,
            SheetPoint::new(130.0, 130.0)
        );

        let flag = BalloonStyle {
            shape: dimo_core::BalloonShape::Flag,
            size_mm: 10.0,
            leader: false,
            ..base
        }
        .layout("2");
        assert_eq!(first[1].shape, BalloonShape::Flag);
        assert_eq!((first[1].width, first[1].height), (flag.width, flag.height));
        assert_eq!(first[1].leader, None);
        assert_eq!(overlay.sheets[1].balloons[0].text, "4");
        assert!(overlay.validate().is_ok());
    }

    #[test]
    fn dates_come_from_timestamps() {
        let t = Timestamp::parse("2026-10-09T14:03:21.250Z").unwrap();
        assert_eq!(
            PdfDate::from_timestamp(&t),
            Some(PdfDate {
                year: 2026,
                month: 10,
                day: 9,
                hour: 14,
                minute: 3,
                second: 21,
            })
        );
        let t = Timestamp::parse("2026-01-01T00:00:00Z").unwrap();
        assert_eq!(PdfDate::from_timestamp(&t).map(|d| d.second), Some(0));
    }
}
