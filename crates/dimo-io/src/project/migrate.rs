//! Forward migrations of `project.json` and `audit.jsonl` (NFR-REL-03, ADR 0004).
//!
//! Every increase of [`SCHEMA_VERSION`] adds one [`Migration`] to [`MIGRATIONS`] and one
//! fixture folder `tests/fixtures/v<n>/` (testing.md). Migrations work on JSON values, because
//! the Rust types only describe the current version. Audit entries are migrated too: they hold
//! whole characteristics and balloons as before and after states, and the autosave journal
//! replays them onto the project.

use serde_json::Value;

use super::SCHEMA_VERSION;
use super::error::ProjectError;

/// One step from schema version `from` to `from + 1`.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    /// The version this step reads.
    pub from: u32,
    /// Rewrites the `project.json` value.
    pub project: fn(&mut Value) -> Result<(), String>,
    /// Rewrites one `audit.jsonl` entry.
    pub audit_entry: fn(&mut Value) -> Result<(), String>,
}

/// All migrations, oldest first. `MIGRATIONS[i].from == i + 1`.
pub const MIGRATIONS: &[Migration] = &[];

/// Brings a project and its audit entries from schema version `from` to [`SCHEMA_VERSION`].
pub fn migrate(from: u32, project: &mut Value, audit: &mut [Value]) -> Result<(), ProjectError> {
    run(MIGRATIONS, from, SCHEMA_VERSION, project, audit)
}

pub(crate) fn run(
    migrations: &[Migration],
    from: u32,
    to: u32,
    project: &mut Value,
    audit: &mut [Value],
) -> Result<(), ProjectError> {
    if from == 0 || from > to {
        return Err(ProjectError::UnknownVersion(from));
    }
    for version in from..to {
        let step = migrations
            .iter()
            .find(|m| m.from == version)
            .ok_or_else(|| ProjectError::Migration {
                from: version,
                message: "no migration registered".into(),
            })?;
        let fail = |message| ProjectError::Migration {
            from: version,
            message,
        };
        (step.project)(project).map_err(fail)?;
        for (line, entry) in audit.iter_mut().enumerate() {
            (step.audit_entry)(entry).map_err(|m| fail(format!("audit line {}: {m}", line + 1)))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn add_flag(value: &mut Value) -> Result<(), String> {
        value
            .as_object_mut()
            .ok_or("not an object")?
            .insert("flag".into(), json!(true));
        Ok(())
    }

    fn rename_note(value: &mut Value) -> Result<(), String> {
        let object = value.as_object_mut().ok_or("not an object")?;
        let note = object.remove("note").ok_or("no note")?;
        object.insert("comment".into(), note);
        Ok(())
    }

    const STEPS: &[Migration] = &[
        Migration {
            from: 1,
            project: add_flag,
            audit_entry: add_flag,
        },
        Migration {
            from: 2,
            project: rename_note,
            audit_entry: |_| Ok(()),
        },
    ];

    #[test]
    fn steps_run_in_order_from_the_file_version() {
        let mut project = json!({ "note": "x" });
        let mut audit = vec![json!({}), json!({})];
        run(STEPS, 1, 3, &mut project, &mut audit).unwrap();
        assert_eq!(project, json!({ "comment": "x", "flag": true }));
        assert_eq!(audit, vec![json!({ "flag": true }); 2]);

        let mut project = json!({ "note": "y" });
        run(STEPS, 2, 3, &mut project, &mut []).unwrap();
        assert_eq!(project, json!({ "comment": "y" }));
    }

    #[test]
    fn failures_name_the_step() {
        let mut project = json!({});
        let error = run(STEPS, 2, 3, &mut project, &mut []).unwrap_err();
        assert!(matches!(error, ProjectError::Migration { from: 2, .. }));
        let error = run(STEPS, 1, 4, &mut json!({ "note": 1 }), &mut []).unwrap_err();
        assert!(matches!(error, ProjectError::Migration { from: 3, .. }));
        assert!(matches!(
            run(STEPS, 0, 3, &mut project, &mut []),
            Err(ProjectError::UnknownVersion(0))
        ));
    }

    #[test]
    fn registry_covers_every_version() {
        for (i, step) in MIGRATIONS.iter().enumerate() {
            assert_eq!(step.from, u32::try_from(i).unwrap() + 1);
        }
        assert_eq!(
            u32::try_from(MIGRATIONS.len()).unwrap() + 1,
            SCHEMA_VERSION,
            "every schema version increase needs a migration"
        );
    }
}
