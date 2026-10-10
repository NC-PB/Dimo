//! Tolerance settings, explanations, re-interpretation and change history of the main window
//! (T2.8, FR-TOL-07, FR-TOL-08, FR-CHR-10, M2 decisions 2 to 4).
//!
//! The rules live in `dimo-tolerance` (tables, import check, explanations), `dimo-detect`
//! (re-interpretation) and `dimo-core` (history); this module reads the open project, runs them
//! and sends the resulting document commands through the session, so every change is one undo
//! step and journaled. The settings themselves change with the document command
//! `set_tolerance_settings` through [`crate::project::execute`].

use std::path::{Path, PathBuf};

use dimo_core::{CharId, Command, HistoryEntry, TableRef};
use dimo_detect::ToleranceEngine;
use dimo_tolerance::{
    ContextError, ExplainValues, TableKind, ToleranceContext, import_custom_table,
};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, WebviewWindow};

use crate::ipc::{CommandError, RejectReason};
use crate::project::{emit_patched, with_session};
use crate::recognition::{ExplainLanguage, shipped};
use crate::session::{AppSession, ProjectPatched};

/// Largest custom table file read, as in the project container.
const MAX_TABLE_BYTES: u64 = 16 * 1024 * 1024;

/// What a tolerance table is used for (data/tolerances README).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum TableUse {
    /// General tolerances by size range and class.
    General,
    /// ISO system of limits and fits.
    Fit,
    /// A table of the user, stored in the project.
    Custom,
}

/// A tolerance table the project settings can name (M2 decision 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ToleranceTableInfo {
    /// Table id and version, as the settings name it.
    pub table: TableRef,
    /// Short name from the file.
    pub title: String,
    /// What it is used for.
    pub usage: TableUse,
    /// True until the owner verified the values (D-43): limits from it get a draft badge.
    pub draft: bool,
    /// Classes to choose from (columns of its general parts); empty for fit tables.
    pub classes: Vec<String>,
}

/// Result of importing a custom table (FR-TOL-07).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum TableImport {
    /// The file dialog was cancelled.
    Cancelled,
    /// The table is stored in the project and listed in its settings (one undo step).
    Imported {
        /// The table.
        table: TableRef,
        /// The change, also sent as `project-patched`.
        patched: ProjectPatched,
    },
    /// The file was refused; the project is unchanged.
    Invalid {
        /// What is wrong, in English, naming the file.
        message: String,
        /// Line of the file, counted from 1, if the error has one.
        line: Option<u32>,
    },
}

/// Result of re-interpreting characteristics (T2.8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct Reinterpreted {
    /// The change as one undo step, also sent as `project-patched`; empty if nothing changed.
    pub patched: ProjectPatched,
    /// Characteristics read again with the current settings.
    pub reinterpreted: u32,
    /// Characteristics with limits edited by hand, left unchanged.
    pub skipped_manual: u32,
    /// Characteristics whose text is empty or not a callout, left unchanged.
    pub skipped_unreadable: u32,
}

fn count(items: &[CharId]) -> u32 {
    u32::try_from(items.len()).unwrap_or(u32::MAX)
}

fn tables_unavailable() -> CommandError {
    CommandError::Project {
        message: "the shipped tolerance tables cannot be loaded".to_owned(),
    }
}

fn settings_refused(error: &ContextError) -> CommandError {
    CommandError::Rejected {
        reason: RejectReason::InvalidProjectSetting,
        message: error.to_string(),
    }
}

/// The tolerance context of the open project: settings, shipped and custom tables.
fn context(session: &AppSession) -> Result<ToleranceContext, CommandError> {
    let project = session.project().ok_or(CommandError::NoProject)?;
    let shipped = shipped().ok_or_else(tables_unavailable)?;
    let custom = session.custom_tables()?;
    ToleranceContext::for_project(
        &project.settings.tolerance,
        shipped.clone(),
        custom.iter().map(|(id, text)| (id.as_str(), text.as_str())),
    )
    .map_err(|e| settings_refused(&e))
}

/// The shipped tables and the custom tables of the open project (M2 decision 2).
pub fn tables(session: &AppSession) -> Result<Vec<ToleranceTableInfo>, CommandError> {
    // Settings that cannot be used (never saved that way) still show the shipped tables.
    let set = match context(session) {
        Ok(context) => context.tables().clone(),
        Err(CommandError::Rejected { .. }) => shipped().ok_or_else(tables_unavailable)?.clone(),
        Err(error) => return Err(error),
    };
    Ok(set
        .tables()
        .iter()
        .map(|table| {
            let header = table.header();
            ToleranceTableInfo {
                table: TableRef {
                    id: header.id.clone(),
                    version: header.version,
                },
                title: header.title.clone(),
                usage: match header.kind {
                    TableKind::General => TableUse::General,
                    TableKind::Fit => TableUse::Fit,
                    TableKind::Custom => TableUse::Custom,
                },
                draft: table.is_draft(),
                classes: table.classes(),
            }
        })
        .collect())
}

/// The explanation of the limits of characteristic `id` in `language` (FR-TOL-08), `null`
/// without a derivation.
pub fn explanation(
    session: &AppSession,
    id: CharId,
    language: ExplainLanguage,
) -> Result<Option<String>, CommandError> {
    let project = session.project().ok_or(CommandError::NoProject)?;
    let c = project
        .characteristic(id)
        .ok_or_else(|| CommandError::InvalidArgument {
            message: format!("unknown characteristic {id}"),
        })?;
    Ok(c.derivation
        .as_ref()
        .map(|d| dimo_tolerance::explain(d, &ExplainValues::from(c), language.into())))
}

/// Re-interprets the characteristics `ids` with the current settings as one undo step; limits
/// edited by hand are kept (T2.8).
pub fn reinterpret(
    session: &mut AppSession,
    ids: &[CharId],
) -> Result<Reinterpreted, CommandError> {
    let engine = ToleranceEngine::new(context(session)?);
    let project = session.project().ok_or(CommandError::NoProject)?;
    let plan = dimo_detect::reinterpret(project, ids, &engine);
    let command = plan.command.unwrap_or(Command::Batch {
        commands: Vec::new(),
    });
    Ok(Reinterpreted {
        patched: session.execute(command)?,
        reinterpreted: count(&plan.reinterpreted),
        skipped_manual: count(&plan.skipped_manual),
        skipped_unreadable: count(&plan.skipped_unreadable),
    })
}

/// Imports the custom table file at `path` (FR-TOL-07): reads and checks it, stores it in the
/// project and lists it in the settings as one undo step. The file is journaled with that step,
/// so a crash before saving keeps it (M2 decision 4).
pub fn import(session: &mut AppSession, path: &Path) -> Result<TableImport, CommandError> {
    let project = session.project().ok_or(CommandError::NoProject)?;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let invalid = |message: String| TableImport::Invalid {
        message,
        line: None,
    };
    let too_big = std::fs::metadata(path).is_ok_and(|m| m.len() > MAX_TABLE_BYTES);
    if too_big {
        return Ok(invalid(format!("{name}: the file is larger than 16 MiB")));
    }
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => return Ok(invalid(format!("{name}: cannot read the file: {error}"))),
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Ok(invalid(format!("{name}: the file is not UTF-8 text")));
    };
    let shipped = shipped().ok_or_else(tables_unavailable)?;
    let custom = session.custom_tables()?;
    let checked = import_custom_table(
        &project.settings.tolerance,
        shipped.clone(),
        custom.iter().map(|(id, text)| (id.as_str(), text.as_str())),
        &name,
        text,
        dimo_io::project::sha256(&bytes),
    );
    let (settings, table) = match checked {
        Ok(checked) => checked,
        Err(error) => {
            return Ok(TableImport::Invalid {
                message: error.to_string(),
                line: error.line(),
            });
        }
    };
    session.insert_table(bytes)?;
    let patched = session.execute(Command::SetToleranceSettings { settings })?;
    Ok(TableImport::Imported { table, patched })
}

/// The tolerance tables the project settings can name: shipped tables and the custom tables of
/// the open project, with their classes and draft state (M2 decision 2, D-43).
#[tauri::command]
#[specta::specta]
pub async fn tolerance_tables(app: AppHandle) -> Result<Vec<ToleranceTableInfo>, CommandError> {
    with_session(app, |_, session| tables(session)).await
}

/// The explanation of the limits of characteristic `id` in `language` (FR-TOL-08); `null` if
/// it has no derivation. Built by Rust from the stored derivation, never stored itself.
#[tauri::command]
#[specta::specta]
pub async fn explain_characteristic(
    app: AppHandle,
    id: CharId,
    language: ExplainLanguage,
) -> Result<Option<String>, CommandError> {
    with_session(app, move |_, session| explanation(session, id, language)).await
}

/// Change history of characteristic `id` from the audit log, oldest first (FR-CHR-10).
#[tauri::command]
#[specta::specta]
pub async fn characteristic_history(
    app: AppHandle,
    id: CharId,
) -> Result<Vec<HistoryEntry>, CommandError> {
    with_session(app, move |_, session| session.characteristic_history(id)).await
}

/// Reads the characteristics `ids` again with the current tolerance settings, as one undo
/// step. Characteristics with limits edited by hand are left unchanged (T2.8).
#[tauri::command]
#[specta::specta]
pub async fn reinterpret_characteristics(
    app: AppHandle,
    ids: Vec<CharId>,
) -> Result<Reinterpreted, CommandError> {
    with_session(app, move |app, session| {
        let mut result = reinterpret(session, &ids)?;
        result.patched = emit_patched(app, result.patched);
        Ok(result)
    })
    .await
}

/// Lets the user pick a custom tolerance table file and imports it (FR-TOL-07). Rust reads and
/// checks the file; a refused file is reported with its line where known.
#[tauri::command]
#[specta::specta]
pub async fn import_tolerance_table(
    app: AppHandle,
    window: WebviewWindow,
) -> Result<TableImport, CommandError> {
    with_session(app.clone(), |_, session| {
        session.project().map(|_| ()).ok_or(CommandError::NoProject)
    })
    .await?;
    let path: PathBuf = if let Some(path) = crate::dev::dev_import_table() {
        path
    } else {
        let picked = rfd::AsyncFileDialog::new()
            .add_filter("TOML", &["toml"])
            .set_parent(&window)
            .pick_file()
            .await;
        let Some(file) = picked else {
            return Ok(TableImport::Cancelled);
        };
        file.path().to_path_buf()
    };
    with_session(app, move |app, session| {
        let mut result = import(session, &path)?;
        if let TableImport::Imported { patched, .. } = &mut result {
            *patched = emit_patched(app, patched.clone());
        }
        Ok(result)
    })
    .await
}
