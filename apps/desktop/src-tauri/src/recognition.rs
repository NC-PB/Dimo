//! Box select and typed callouts (T2.6, FR-REC-01, FR-REC-02, M2 decision 5).
//!
//! Both commands only read: they return proposals or field values and change nothing (ADR 0006).
//! The frontend accepts proposals with the document command `accept_proposals` through
//! `execute`, which is one undo step, and applies typed values with `update_fields`.
//!
//! Callouts are interpreted by the tolerance engine of `dimo-tolerance` with the project's
//! tolerance settings and custom tables (FR-TOL-01). Each result comes with its explanation in
//! the UI language (FR-TOL-08) and the engine's notes; both are shown on the card and never
//! stored.
//!
//! Box select answers directly instead of as a job: on `test_drawing_1` it takes about a
//! millisecond (a region query on the cached page plus parsing), far below the 50 ms where a
//! job with progress events would pay off. The proposals still carry a job ID from the job
//! counter, so a later job based box select fits the same data.

use std::sync::OnceLock;

use dimo_core::characteristic::FieldValue;
use dimo_core::derivation::ToleranceDerivation;
use dimo_core::geometry::OrientedBox;
use dimo_core::proposal::{BalloonPlacement, ParseIssue, Proposal};
use dimo_core::{Language, Length, SheetId, Unit};
use dimo_detect::{
    BoxSelectContext, CalloutOnly, Interpreter, ToleranceEngine, box_select_with_notes,
    read_callout,
};
use dimo_pdf::tiles::TileService;
use dimo_tolerance::{ExplainValues, Measures, Note, TableSet, ToleranceContext};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Manager};

use crate::export::next_job_id;
use crate::ipc::{ArgumentReason, CommandError, InternalReason, tile_service};
use crate::project::with_session;
use crate::session::RecognitionTarget;
use crate::tiles::TileState;

/// Something the tolerance engine could not decide, shown with a proposal, never stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum InterpreterNote {
    /// An angle whose shorter leg is unknown, so angular general tolerances cannot apply.
    ShorterLegUnknown,
    /// A fit pair such as `H7/g6` names two features; no limits.
    FitPair,
}

impl From<Note> for InterpreterNote {
    fn from(note: Note) -> Self {
        match note {
            Note::ShorterLegUnknown => Self::ShorterLegUnknown,
            Note::FitPair => Self::FitPair,
        }
    }
}

/// A proposal of box select with what the card shows besides it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ProposalView {
    /// The proposal, to edit and accept.
    pub proposal: Proposal,
    /// Explanation of the limits in the requested language, `null` without a derivation.
    pub explanation: Option<String>,
    /// What the engine could not decide.
    pub notes: Vec<InterpreterNote>,
}

/// Values for text typed into the value field or the card (M2 decision 5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct TypedCallout {
    /// Fields to set in one `update_fields` command: the requirement text, and when the text
    /// parses, kind, nominal, unit, deviations, limits, fit, quantity, inspect, derivation and
    /// the shorter leg.
    pub values: Vec<FieldValue>,
    /// Why the text could not be parsed; then `values` holds only the requirement text.
    pub parse_error: Option<ParseIssue>,
    /// Explanation of the limits in the requested language, `null` without a derivation.
    pub explanation: Option<String>,
    /// What the engine could not decide.
    pub notes: Vec<InterpreterNote>,
}

/// The shipped tolerance tables, parsed once per process.
pub(crate) fn shipped() -> Option<&'static TableSet> {
    static SHIPPED: OnceLock<Option<TableSet>> = OnceLock::new();
    SHIPPED
        .get_or_init(|| match TableSet::shipped() {
            Ok(tables) => Some(tables),
            Err(error) => {
                tracing::warn!("shipped tolerance tables do not load: {error}");
                None
            }
        })
        .as_ref()
}

/// The interpreter for the project of `target` (spec 08 stage 7): the tolerance engine with the
/// project's tolerance settings, the shipped tables and the project's custom tables. If they do
/// not load (which saving and opening prevent), the callout alone is read and a warning logged.
pub fn interpreter(target: &RecognitionTarget) -> Box<dyn Interpreter + Send + Sync> {
    let context = shipped().map(|tables| {
        ToleranceContext::for_project(
            &target.settings.tolerance,
            tables.clone(),
            target
                .custom_tables
                .iter()
                .map(|(id, text)| (id.as_str(), text.as_str())),
        )
    });
    match context {
        Some(Ok(context)) => Box::new(ToleranceEngine::new(context.with_drawing_unit(target.unit))),
        Some(Err(error)) => {
            tracing::warn!("tolerance engine unavailable, reading callouts only: {error}");
            Box::new(CalloutOnly)
        }
        None => Box::new(CalloutOnly),
    }
}

/// Values an explanation talks about.
struct Values {
    nominal: Option<Decimal>,
    unit: Option<Unit>,
    upper_dev: Option<Decimal>,
    lower_dev: Option<Decimal>,
    upper_limit: Option<Decimal>,
    lower_limit: Option<Decimal>,
    shorter_leg: Option<Length>,
}

fn explanation(
    derivation: Option<&ToleranceDerivation>,
    v: &Values,
    language: Language,
) -> Option<String> {
    let values = ExplainValues {
        nominal: v.nominal,
        unit: v.unit,
        upper_dev: v.upper_dev,
        lower_dev: v.lower_dev,
        upper_limit: v.upper_limit,
        lower_limit: v.lower_limit,
        shorter_leg: v.shorter_leg,
    };
    derivation.map(|d| dimo_tolerance::explain(d, &values, language))
}

fn check_region(region: &OrientedBox) -> Result<(), CommandError> {
    if region.is_valid() {
        Ok(())
    } else {
        Err(CommandError::argument(
            ArgumentReason::Region,
            "the region needs a finite position and a size above zero",
        ))
    }
}

/// Proposals for the text inside `region` of the open drawing (FR-REC-01, FR-REC-02), with
/// explanations in `language`. `placement` is where the balloon of the box goes. Reads only.
pub fn propose(
    tiles: &TileService,
    target: &RecognitionTarget,
    sheet: SheetId,
    region: OrientedBox,
    placement: BalloonPlacement,
    language: Language,
) -> Result<Vec<ProposalView>, CommandError> {
    check_region(&region)?;
    let doc = tiles.document(&target.drawing).ok_or_else(|| {
        CommandError::internal(
            InternalReason::Bug,
            "the drawing of the project is not open",
        )
    })?;
    let started = std::time::Instant::now();
    let text = doc.region_text(target.page, region)?;
    let interpreter = interpreter(target);
    let proposals = box_select_with_notes(
        sheet,
        &region,
        &text,
        &BoxSelectContext {
            interpreter: interpreter.as_ref(),
            drawing_unit: target.unit,
            placement,
            job_id: Some(next_job_id().0),
        },
    );
    tracing::debug!(
        proposals = proposals.len(),
        micros = started.elapsed().as_micros(),
        "box select"
    );
    Ok(proposals
        .into_iter()
        .map(|p| {
            let q = &p.proposal;
            let values = Values {
                nominal: q.nominal,
                unit: q.unit,
                upper_dev: q.upper_dev,
                lower_dev: q.lower_dev,
                upper_limit: q.upper_limit,
                lower_limit: q.lower_limit,
                shorter_leg: q.shorter_leg,
            };
            ProposalView {
                explanation: explanation(q.derivation.as_ref(), &values, language),
                notes: p.notes.into_iter().map(Into::into).collect(),
                proposal: p.proposal,
            }
        })
        .collect())
}

/// The shorter leg typed on the proposal card or in the detail panel (T2.9a), in the unit of the
/// sheet: empty text means no leg; anything but a positive number is refused.
pub fn parse_shorter_leg(text: Option<&str>, unit: Unit) -> Result<Option<Length>, CommandError> {
    Length::parse_optional(text.unwrap_or_default(), unit).map_err(|e| {
        CommandError::argument(
            ArgumentReason::ShorterLeg,
            format_args!("the shorter leg {e}"),
        )
    })
}

/// The shorter leg sent with a callout text (T2.9a).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum LegInput {
    /// Text just typed in the leg field, in the unit of the sheet; empty means no leg.
    Typed(String),
    /// A leg already stored on the characteristic or proposal, used with its own unit, so a
    /// later change of the sheet unit does not change it.
    Stored(Length),
}

/// The leg of `input` (T2.9a): typed text is parsed in `sheet_unit`, a stored leg is taken as
/// it is. `None` input or empty text is no leg.
pub fn resolve_leg(
    input: Option<&LegInput>,
    sheet_unit: Unit,
) -> Result<Option<Length>, CommandError> {
    match input {
        None => Ok(None),
        Some(LegInput::Typed(text)) => parse_shorter_leg(Some(text), sheet_unit),
        Some(LegInput::Stored(leg)) => Ok(Some(*leg)),
    }
}

/// Reads typed text like a box selection (M2 decision 5), explained in `language`, with the
/// shorter leg of an angle if one was entered (T2.9a).
pub fn read_typed(
    target: &RecognitionTarget,
    text: &str,
    shorter_leg: Option<Length>,
    language: Language,
) -> TypedCallout {
    let interpreter = interpreter(target);
    let r = read_callout(
        text,
        false,
        target.unit,
        interpreter.as_ref(),
        &Measures::with_shorter_leg(shorter_leg),
    );
    let values = Values {
        nominal: r.nominal,
        unit: r.unit,
        upper_dev: r.upper_dev,
        lower_dev: r.lower_dev,
        upper_limit: r.upper_limit,
        lower_limit: r.lower_limit,
        shorter_leg: r.shorter_leg,
    };
    TypedCallout {
        explanation: explanation(r.derivation.as_ref(), &values, language),
        notes: r.notes.iter().copied().map(Into::into).collect(),
        values: r.field_values(),
        parse_error: r.parse_error,
    }
}

/// Box select: proposals for the text inside `region` of `sheet` (FR-REC-01, FR-REC-02), with
/// explanations in `language`. Changes nothing; accept them with the `accept_proposals`
/// document command (ADR 0006). An empty list means the region holds no PDF text.
#[tauri::command]
#[specta::specta]
pub async fn propose_from_region(
    app: AppHandle,
    sheet: SheetId,
    region: OrientedBox,
    placement: BalloonPlacement,
    language: Language,
) -> Result<Vec<ProposalView>, CommandError> {
    let tiles = tile_service(&app.state::<TileState>())?;
    let target = with_session(app, move |_, session| session.recognition_target(sheet)).await?;
    crate::ipc::blocking(move || propose(&tiles, &target, sheet, region, placement, language)).await
}

/// Parses and interprets text typed for a characteristic on `sheet` (M2 decision 5), explained
/// in `language`. `shorter_leg` is the shorter leg of an angle (T2.9a): text typed in the unit
/// of the sheet (empty means none, other text that is not a positive number is refused), or a
/// leg already stored, which keeps its unit; `null` means none. Changes nothing; the frontend
/// sets the returned values with one `update_fields` command, or applies them to a proposal.
#[tauri::command]
#[specta::specta]
pub async fn read_callout_text(
    app: AppHandle,
    sheet: SheetId,
    text: String,
    shorter_leg: Option<LegInput>,
    language: Language,
) -> Result<TypedCallout, CommandError> {
    let target = with_session(app, move |_, session| session.recognition_target(sheet)).await?;
    let leg = resolve_leg(shorter_leg.as_ref(), target.unit)?;
    Ok(read_typed(&target, &text, leg, language))
}
