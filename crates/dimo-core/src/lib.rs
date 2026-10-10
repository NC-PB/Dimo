//! Domain model of a Dimo project: characteristics, balloons, commands with undo and redo, numbering and validation. Pure logic without IO.
//!
//! - [`project::Project`] is the state stored in `project.json`.
//! - [`document::Document`] wraps a project, executes [`command::Command`]s, keeps unlimited
//!   undo and redo, and records [`document::AuditEntry`]s (NFR-REL-02).
//! - Every command returns a [`patch::Patch`] of primitive [`patch::Change`]s for the frontend.
//! - IDs, time and the audit user come from an [`env::Environment`], so the same commands
//!   give the same project (rule 11).
//! - Automation returns [`proposal::Proposal`]s; only an accept command turns them into
//!   characteristics (ADR 0006).
//! - [`history::characteristic_history`] reads the change history of one characteristic from
//!   the audit log (FR-CHR-10).
//!
//! The `specta` feature derives `specta::Type` on every IPC facing type.

pub mod balloon;
pub mod characteristic;
pub mod command;
pub mod decimal;
pub mod derivation;
pub mod document;
pub mod env;
pub mod geometry;
pub mod history;
pub mod id;
pub mod number;
pub mod patch;
pub mod project;
pub mod proposal;
pub mod sheet;
pub mod truth;

pub use balloon::{
    BALLOON_METRICS, Balloon, BalloonLayout, BalloonMetrics, BalloonShape, BalloonStyle,
    BalloonStyleOverride, Color, UNITS_PER_MM,
};
pub use characteristic::{
    Characteristic, CharacteristicKind, CharacteristicStatus, Classification, FieldError,
    FieldValue, Inspection, Origin, SourceRegion, TextSource, ToleranceRule, Unit,
};
pub use command::{BalloonMove, Command, CommandError};
pub use derivation::{
    DerivationHint, DerivationRule, RangeBound, SizeRange, TableLookup, TableRef,
    ToleranceDerivation, UnitConversion,
};
pub use document::{AuditAction, AuditEntry, Document};
pub use env::{Environment, FixedEnvironment, Timestamp};
pub use geometry::{OrientedBox, Point, Rect, Size};
pub use history::{
    ChangeSource, CharacteristicField, HistoryAction, HistoryEntry, characteristic_history,
};
pub use id::{BalloonId, CharId, RevisionId, SheetId};
pub use number::DisplayNumber;
pub use patch::{Change, Patch};
pub use project::{
    CustomTable, DecimalPlaceRule, InsertPolicy, LockReason, MultiInstance, Numbering,
    NumberingLock, NumberingSettings, NumberingStrategy, Project, ProjectInfo, ProjectSettings,
    TableClass, ToleranceSettings, UnitRounding,
};
pub use proposal::{BalloonPlacement, EngineVersion, ParseHint, ParseIssue, Proposal};
pub use sheet::{
    DrawingRevision, Rotation, Scale, Sha256Hex, Sheet, SheetKind, SheetView, ZoneGrid,
};

/// The JSON schema of `project.json` content ([`Project`]), with object keys sorted.
pub fn project_schema() -> serde_json::Value {
    truth::sort_keys(schemars::schema_for!(Project).to_value())
}
