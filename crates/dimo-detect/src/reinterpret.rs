//! Re-interpretation of characteristics after the tolerance settings changed (T2.8, FR-TOL-01,
//! FR-TOL-08).
//!
//! Changing the project tolerance settings never changes stored limits by itself. The user
//! selects characteristics and asks for a re-interpretation: each requirement text is read again
//! with [`read_callout`] and the current settings, and the new limits and derivations are set
//! with one [`Command`], so the whole action is one undo step.
//!
//! Characteristics whose limits were edited by hand (rule `manual`) are never changed. Only the
//! tolerance fields are set (nominal, unit, deviations, limits, fit, derivation); kind, quantity
//! and the `inspect` flag stay as the user left them (FR-CHR-08: `inspect` is overridable),
//! except for a characteristic that was never interpreted (no derivation), which gets them from
//! the reading like typed text.

use dimo_core::characteristic::{Characteristic, FieldValue};
use dimo_core::derivation::{DerivationHint, DerivationRule};
use dimo_core::{CharId, Command, Project, Unit};

use crate::interpret::Interpreter;
use crate::read::read_callout;

/// What a re-interpretation would do (T2.8).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Reinterpretation {
    /// One `update_fields` per characteristic in a batch, `None` if nothing is read again.
    pub command: Option<Command>,
    /// Characteristics read again (their values may come out unchanged).
    pub reinterpreted: Vec<CharId>,
    /// Characteristics with limits edited by hand, left unchanged.
    pub skipped_manual: Vec<CharId>,
    /// Characteristics whose requirement text is empty or does not parse, left unchanged.
    pub skipped_unreadable: Vec<CharId>,
}

/// The re-interpretation of the characteristics `ids` of `project` with `interpreter` (the
/// tolerance engine with the current project settings). IDs that are not in the project are
/// ignored. Changes nothing: execute the returned command.
pub fn reinterpret(
    project: &Project,
    ids: &[CharId],
    interpreter: &dyn Interpreter,
) -> Reinterpretation {
    let mut out = Reinterpretation::default();
    let mut commands = Vec::new();
    let mut seen = Vec::new();
    for &id in ids {
        if seen.contains(&id) {
            continue;
        }
        seen.push(id);
        let Some(c) = project.characteristic(id) else {
            continue;
        };
        // Limits without any derivation come from a project older than derivations; they were
        // typed, so they count as manual too.
        let manual = match &c.derivation {
            Some(d) => d.rule == DerivationRule::Manual,
            None => c.upper_limit.is_some() || c.lower_limit.is_some(),
        };
        if manual {
            out.skipped_manual.push(id);
            continue;
        }
        let Some(values) = tolerance_values(project, c, interpreter) else {
            out.skipped_unreadable.push(id);
            continue;
        };
        out.reinterpreted.push(id);
        commands.push(Command::UpdateFields {
            ids: vec![id],
            values,
        });
    }
    if !commands.is_empty() {
        out.command = Some(Command::Batch { commands });
    }
    out
}

/// The tolerance fields for the requirement text of `c`, `None` if it does not parse.
fn tolerance_values(
    project: &Project,
    c: &Characteristic,
    interpreter: &dyn Interpreter,
) -> Option<Vec<FieldValue>> {
    if c.requirement_text.trim().is_empty() {
        return None;
    }
    // A basic dimension frame found in the PDF is not in the text; the derivation remembers it.
    let basic = c
        .derivation
        .as_ref()
        .is_some_and(|d| d.hints.contains(&DerivationHint::BasicDimension));
    let reading = read_callout(
        &c.requirement_text,
        basic,
        drawing_unit(project, c),
        interpreter,
    );
    if reading.parse_error.is_some() {
        return None;
    }
    // A characteristic never interpreted (no derivation, e.g. text typed before T2.6) gets
    // kind, quantity and inspect from the reading too, as typed text would; once read, they are
    // the user's.
    let mut values = Vec::new();
    if c.derivation.is_none() {
        values.extend([
            FieldValue::Kind(reading.kind),
            FieldValue::Quantity(reading.quantity),
            FieldValue::Inspect(reading.inspect),
        ]);
    }
    values.extend([
        FieldValue::Nominal(reading.nominal.into()),
        FieldValue::Unit(reading.unit),
        FieldValue::UpperDev(reading.upper_dev.into()),
        FieldValue::LowerDev(reading.lower_dev.into()),
        FieldValue::UpperLimit(reading.upper_limit.into()),
        FieldValue::LowerLimit(reading.lower_limit.into()),
        FieldValue::Fit(reading.fit),
        FieldValue::Derivation(reading.derivation),
    ]);
    Some(values)
}

/// Unit of the sheet the characteristic sits on (D-20): its first balloon, else its first
/// source region; mm if neither is known.
fn drawing_unit(project: &Project, c: &Characteristic) -> Unit {
    project
        .balloons_of(c.id)
        .map(|b| b.sheet)
        .next()
        .or_else(|| c.sources.first().map(|s| s.sheet))
        .and_then(|id| project.sheet(id))
        .map_or(Unit::Mm, |s| s.unit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpret::ToleranceEngine;
    use dimo_core::command::execute;
    use dimo_core::derivation::ToleranceDerivation;
    use dimo_core::project::{TableClass, ToleranceSettings};
    use dimo_core::{
        Characteristic, CharacteristicKind, DisplayNumber, DrawingRevision, Environment,
        FixedEnvironment, RevisionId, Sha256Hex, Sheet, SheetId, SheetKind, Size, TableRef,
        Timestamp,
    };
    use dimo_tolerance::{Language, TableSet, ToleranceContext, explain};
    use rust_decimal::Decimal;

    fn d(text: &str) -> Option<Decimal> {
        text.parse().ok()
    }

    fn engine(class: Option<&str>) -> ToleranceEngine {
        let settings = ToleranceSettings {
            general: class.map(|class| TableClass {
                table: TableRef {
                    id: "iso-2768-1".into(),
                    version: 1,
                },
                class: class.into(),
            }),
            ..ToleranceSettings::default()
        };
        let tables = TableSet::shipped().unwrap();
        ToleranceEngine::new(ToleranceContext::new(&settings, tables).unwrap())
    }

    /// A project with characteristics read from `texts` with ISO 2768-m.
    fn project(texts: &[&str]) -> (Project, Vec<CharId>, FixedEnvironment) {
        let mut env = FixedEnvironment::new();
        let sheet = SheetId::from_uuid(env.new_uuid());
        let revision = DrawingRevision {
            id: RevisionId::from_uuid(env.new_uuid()),
            label: "A".into(),
            file_name: "part.pdf".into(),
            sha256: Sha256Hex::parse(&"0".repeat(64)).unwrap(),
            imported_at: Timestamp::parse("2026-01-01T00:00:00Z").unwrap(),
            sheets: vec![Sheet::new(
                sheet,
                0,
                Size {
                    width: 842.0,
                    height: 595.0,
                },
                SheetKind::VectorText,
            )],
        };
        let mut project = Project::new(dimo_core::ProjectInfo::default(), revision);
        let m = engine(Some("m"));
        let mut ids = Vec::new();
        for (i, text) in texts.iter().enumerate() {
            let id = CharId::from_uuid(env.new_uuid());
            let mut c =
                Characteristic::manual(id, DisplayNumber::plain(u32::try_from(i + 1).unwrap()));
            let reading = read_callout(text, false, Unit::Mm, &m);
            c.set_values(&reading.field_values()).unwrap();
            project.characteristics.push(c);
            ids.push(id);
        }
        (project, ids, env)
    }

    fn limits(project: &Project, id: CharId) -> (Option<Decimal>, Option<Decimal>) {
        let c = project.characteristic(id).unwrap();
        (c.upper_limit, c.lower_limit)
    }

    // T2.8 acceptance: changing the general tolerance and re-interpreting updates limits and
    // explanations, as one undo step.
    #[test]
    fn a_new_general_class_changes_limits_and_explanation() {
        let (mut project, ids, mut env) = project(&["50", "Ø20 +0.1 -0", "BREAK EDGES"]);
        assert_eq!(limits(&project, ids[0]), (d("50.3"), d("49.7")));
        let before = project.clone();
        let explained = |p: &Project| {
            let c = p.characteristic(ids[0]).unwrap();
            explain(c.derivation.as_ref().unwrap(), &c.into(), Language::English)
        };
        assert!(
            explained(&project).contains("class m"),
            "{}",
            explained(&project)
        );

        let result = reinterpret(&project, &ids, &engine(Some("f")));
        assert_eq!(result.reinterpreted, [ids[0], ids[1]]);
        assert_eq!(result.skipped_unreadable, [ids[2]]);
        let command = result.command.unwrap();
        let changes = execute(&mut project, &command, &mut env).unwrap();
        assert_eq!(limits(&project, ids[0]), (d("50.15"), d("49.85")));
        assert!(
            explained(&project).contains("class f"),
            "{}",
            explained(&project)
        );
        // The explicit tolerance does not depend on the general class.
        assert_eq!(limits(&project, ids[1]), (d("20.1"), d("20")));
        // One command: undoing its changes restores the project.
        for change in dimo_core::patch::invert(&changes) {
            project.apply_change(&change).unwrap();
        }
        assert_eq!(project, before);
    }

    // Never automatic for characteristics edited by hand.
    #[test]
    fn manual_limits_are_kept() {
        let (mut project, ids, _) = project(&["50", "60"]);
        let c = &mut project.characteristics[0];
        c.upper_limit = d("50.5");
        c.derivation = Some(ToleranceDerivation::manual());
        let result = reinterpret(&project, &[ids[0], ids[1], ids[0]], &engine(None));
        assert_eq!(result.skipped_manual, [ids[0]]);
        assert_eq!(result.reinterpreted, [ids[1]]);
        let Some(Command::Batch { commands }) = result.command else {
            panic!("expected a batch");
        };
        assert_eq!(commands.len(), 1);
    }

    // Without a general tolerance a plain size gets no limits; a later class gives them.
    #[test]
    fn no_tolerance_defined_becomes_general() {
        let (mut project, ids, mut env) = project(&["50"]);
        let none = reinterpret(&project, &ids, &engine(None));
        execute(&mut project, &none.command.unwrap(), &mut env).unwrap();
        let c = project.characteristic(ids[0]).unwrap();
        assert_eq!(limits(&project, ids[0]), (None, None));
        assert_eq!(
            c.derivation.as_ref().map(|d| d.rule.clone()),
            Some(DerivationRule::NoToleranceDefined)
        );
        assert_eq!(c.kind, CharacteristicKind::Linear);
        let again = reinterpret(&project, &ids, &engine(Some("c")));
        execute(&mut project, &again.command.unwrap(), &mut env).unwrap();
        assert_eq!(limits(&project, ids[0]), (d("50.8"), d("49.2")));
    }

    // A characteristic with text but no derivation was never read: it gets kind and inspect.
    #[test]
    fn never_read_characteristics_get_kind_and_inspect() {
        let (mut project, ids, mut env) = project(&["(42)"]);
        let c = &mut project.characteristics[0];
        c.derivation = None;
        c.kind = CharacteristicKind::Other;
        c.inspect = true;
        let result = reinterpret(&project, &ids, &engine(Some("m")));
        execute(&mut project, &result.command.unwrap(), &mut env).unwrap();
        let c = &project.characteristics[0];
        assert_eq!(c.kind, CharacteristicKind::Linear);
        assert!(!c.inspect);
    }

    // Limits stored without a derivation (older projects) were typed: kept.
    #[test]
    fn limits_without_derivation_are_kept() {
        let (mut project, ids, _) = project(&["50"]);
        project.characteristics[0].derivation = None;
        let result = reinterpret(&project, &ids, &engine(Some("f")));
        assert_eq!(result.skipped_manual, [ids[0]]);
        assert_eq!(result.command, None);
    }

    // FR-CHR-08: the inspect flag the user set on a reference dimension stays.
    #[test]
    fn inspect_stays_as_the_user_left_it() {
        let (mut project, ids, mut env) = project(&["(42)"]);
        project.characteristics[0].inspect = true;
        let result = reinterpret(&project, &ids, &engine(Some("m")));
        execute(&mut project, &result.command.unwrap(), &mut env).unwrap();
        assert!(project.characteristics[0].inspect);
    }
}
