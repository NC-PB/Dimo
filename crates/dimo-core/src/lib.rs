//! Domain model of a Dimo project: characteristics, balloons, commands with undo and redo, numbering and validation. Pure logic without IO.
//!
//! - [`project::Project`] is the state stored in `project.json`.
//! - [`document::Document`] wraps a project, executes [`command::Command`]s, keeps unlimited
//!   undo and redo, and records [`document::AuditEntry`]s (NFR-REL-02).
//! - Every command returns a [`patch::Patch`] of primitive [`patch::Change`]s for the frontend.
//! - IDs, time and the audit user come from an [`env::Environment`], so the same commands
//!   give the same project (rule 11).
//!
//! The `specta` feature derives `specta::Type` on every IPC facing type.

pub mod balloon;
pub mod characteristic;
pub mod command;
pub mod decimal;
pub mod document;
pub mod env;
pub mod geometry;
pub mod id;
pub mod patch;
pub mod project;
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
pub use document::{AuditAction, AuditEntry, Document};
pub use env::{Environment, FixedEnvironment, Timestamp};
pub use geometry::{OrientedBox, Point, Size};
pub use id::{BalloonId, CharId, RevisionId, SheetId};
pub use patch::{Change, Patch};
pub use project::{
    InsertPolicy, LockReason, Numbering, NumberingLock, Project, ProjectInfo, ProjectSettings,
};
pub use sheet::{DrawingRevision, Rotation, Scale, Sha256Hex, Sheet, SheetKind};

/// The JSON schema of `project.json` content ([`Project`]), with object keys sorted.
pub fn project_schema() -> serde_json::Value {
    truth::sort_keys(schemars::schema_for!(Project).to_value())
}
