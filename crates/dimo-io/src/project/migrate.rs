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
pub const MIGRATIONS: &[Migration] = &[Migration {
    from: 1,
    project: v1::project,
    audit_entry: v1::audit_entry,
}];

/// Version 1 to 2 (T2.4): structured display numbers, tolerance derivations, zone grids and
/// views, numbering and tolerance settings, insert anchors.
mod v1 {
    use serde_json::{Map, Value, json};

    type Result = std::result::Result<(), String>;

    fn object<'a>(
        value: &'a mut Value,
        what: &str,
    ) -> std::result::Result<&'a mut Map<String, Value>, String> {
        value
            .as_object_mut()
            .ok_or_else(|| format!("{what} is not an object"))
    }

    fn array<'a>(
        value: &'a mut Value,
        what: &str,
    ) -> std::result::Result<&'a mut Vec<Value>, String> {
        value
            .as_array_mut()
            .ok_or_else(|| format!("{what} is not an array"))
    }

    fn field<'a>(
        map: &'a mut Map<String, Value>,
        key: &str,
    ) -> std::result::Result<&'a mut Value, String> {
        map.get_mut(key).ok_or_else(|| format!("missing {key}"))
    }

    /// `number` becomes the display number text; limits that exist were typed by hand, so their
    /// derivation is `manual` (FR-TOL-08).
    fn characteristic(value: &mut Value) -> Result {
        let c = object(value, "characteristic")?;
        let number = field(c, "number")?
            .as_u64()
            .filter(|n| *n >= 1)
            .ok_or("number is not a positive integer")?;
        c.insert("number".into(), Value::String(number.to_string()));
        let has_limits = ["upper_limit", "lower_limit"]
            .iter()
            .any(|key| c.get(*key).is_some_and(|v| !v.is_null()));
        let derivation = if has_limits {
            json!({ "rule": { "rule": "manual" }, "draft": false, "hints": [], "conversion": null })
        } else {
            Value::Null
        };
        // Key order does not matter: the result is read into the Rust types.
        c.insert("derivation".into(), derivation);
        Ok(())
    }

    /// Sheets get no zone grid and no views.
    fn sheet(value: &mut Value) -> Result {
        let s = object(value, "sheet")?;
        s.insert("zone_grid".into(), Value::Null);
        s.insert("views".into(), json!([]));
        Ok(())
    }

    /// The insert policy moves from the lock to the numbering settings; the lock gets the list
    /// of numbers given while locked (none in version 1).
    fn numbering(value: &mut Value) -> Result {
        let n = object(value, "numbering")?;
        if let Some(lock) = n.get_mut("lock").filter(|l| !l.is_null()) {
            let lock = object(lock, "lock")?;
            match lock.remove("insert_policy") {
                Some(Value::String(policy)) if policy == "next_free" => {}
                other => return Err(format!("unexpected insert policy {other:?}")),
            }
            lock.insert("given".into(), json!([]));
        }
        Ok(())
    }

    /// Version 1 projects keep their placement order: strategy `manual`. Version 1 had only
    /// the next free insert policy and no tolerance settings.
    fn settings(value: &mut Value) -> Result {
        let s = object(value, "settings")?;
        s.insert(
            "numbering".into(),
            json!({
                "strategy": "manual",
                "multi_instance": "quantity",
                "insert_when_locked": "next_free"
            }),
        );
        s.insert(
            "tolerance".into(),
            json!({
                "general": null,
                "drawing_rule": null,
                "decimal_rules": [],
                "unit_rounding": { "mm_places": 3, "inch_places": 4 },
                "custom_tables": []
            }),
        );
        Ok(())
    }

    pub(super) fn project(value: &mut Value) -> Result {
        let p = object(value, "project")?;
        for c in array(field(p, "characteristics")?, "characteristics")? {
            characteristic(c)?;
        }
        for revision in array(field(p, "revisions")?, "revisions")? {
            let revision = object(revision, "revision")?;
            for s in array(field(revision, "sheets")?, "sheets")? {
                sheet(s)?;
            }
        }
        numbering(field(p, "numbering")?)?;
        settings(field(p, "settings")?)
    }

    /// `add_characteristic` commands get `insert_after: null`, also inside batches.
    fn command(value: &mut Value) -> Result {
        let c = object(value, "command")?;
        match c.get("type").and_then(Value::as_str) {
            Some("add_characteristic") => {
                c.insert("insert_after".into(), Value::Null);
            }
            Some("batch") => {
                for inner in array(field(c, "commands")?, "commands")? {
                    command(inner)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn change(value: &mut Value) -> Result {
        let c = object(value, "change")?;
        let kind = c
            .get("type")
            .and_then(Value::as_str)
            .ok_or("change without type")?
            .to_owned();
        let keys: &[&str] = match kind.as_str() {
            "characteristic_inserted" | "characteristic_removed" => &["characteristic"],
            "characteristic_changed"
            | "sheet_changed"
            | "numbering_changed"
            | "settings_changed" => &["before", "after"],
            _ => &[],
        };
        for key in keys {
            let state = field(c, key)?;
            match kind.as_str() {
                "sheet_changed" => sheet(state)?,
                "numbering_changed" => numbering(state)?,
                "settings_changed" => settings(state)?,
                _ => characteristic(state)?,
            }
        }
        Ok(())
    }

    pub(super) fn audit_entry(value: &mut Value) -> Result {
        let entry = object(value, "audit entry")?;
        let action = object(field(entry, "action")?, "action")?;
        command(field(action, "command")?)?;
        for c in array(field(entry, "changes")?, "changes")? {
            change(c)?;
        }
        Ok(())
    }
}

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

    fn v1_characteristic(number: u64, limits: bool) -> Value {
        let limit = if limits { json!("8.1") } else { Value::Null };
        json!({
            "id": "00000000-0000-0000-0000-000000000004", "number": number, "kind": "linear",
            "requirement_text": "", "nominal": "8", "unit": "mm", "upper_dev": null,
            "lower_dev": null, "upper_limit": limit, "lower_limit": null, "fit": null,
            "quantity": 1, "classification": "none",
            "inspection": { "method": "", "gauge": "", "sampling": "", "frequency": "" },
            "inspect": true, "status": "accepted", "origin": "manual", "sources": [],
            "comment": ""
        })
    }

    fn v1_sheet() -> Value {
        json!({
            "id": "00000000-0000-0000-0000-000000000002", "index": 0,
            "size": { "width": 10.0, "height": 10.0 }, "rotation": "deg0",
            "kind": "vector_text", "raster_dpi": null, "unit": "mm",
            "scale": { "drawing": 1, "actual": 1 }
        })
    }

    fn v1_settings() -> Value {
        json!({ "balloon_style": {
            "shape": "circle", "size_mm": 7.0, "outline_mm": 0.35, "outline_color": "#0057B8",
            "fill_color": "#FFFFFF", "text_color": "#000000", "leader": true
        }})
    }

    /// Every branch of the version 1 audit migration: nested batches, undo and redo
    /// actions, sheet, settings, numbering and characteristic changes (ADR 0004).
    #[test]
    fn version_1_audit_entries_of_every_kind_migrate() {
        let lock = json!({
            "reason": "manual", "locked_at": "2026-03-01T08:00:00Z", "locked_by": "t",
            "insert_policy": "next_free", "highest_number": 1
        });
        let add = json!({
            "type": "add_characteristic", "sheet": "00000000-0000-0000-0000-000000000002",
            "position": { "x": 1.0, "y": 1.0 }, "anchor": { "x": 2.0, "y": 2.0 },
            "region": null, "values": []
        });
        let changes = json!([
            { "type": "characteristic_inserted", "index": 0,
              "characteristic": v1_characteristic(1, true) },
            { "type": "characteristic_changed", "before": v1_characteristic(1, true),
              "after": v1_characteristic(2, false) },
            { "type": "characteristic_removed", "index": 0,
              "characteristic": v1_characteristic(2, false) },
            { "type": "sheet_changed", "before": v1_sheet(), "after": v1_sheet() },
            { "type": "settings_changed", "before": v1_settings(), "after": v1_settings() },
            { "type": "numbering_changed", "before": { "lock": null }, "after": { "lock": lock } },
            { "type": "order_changed", "before": [], "after": [] }
        ]);
        for action in ["command", "undo", "redo"] {
            let mut entry = json!({
                "timestamp": "2026-03-01T08:00:00Z", "user": "t",
                "action": { "type": action, "command": {
                    "type": "batch",
                    "commands": [add.clone(), { "type": "batch", "commands": [add.clone()] }]
                }},
                "changes": changes.clone()
            });
            (MIGRATIONS[0].audit_entry)(&mut entry).unwrap();
            let typed: dimo_core::AuditEntry = serde_json::from_value(entry.clone())
                .unwrap_or_else(|e| panic!("{action}: {e}\n{entry:#}"));
            assert_eq!(typed.changes.len(), 7);
            let inner = &entry["action"]["command"]["commands"];
            assert!(
                inner[0]["insert_after"].is_null()
                    && inner[1]["commands"][0]["insert_after"].is_null()
            );
            assert_eq!(entry["changes"][0]["characteristic"]["number"], "1");
            assert_eq!(
                entry["changes"][0]["characteristic"]["derivation"]["rule"]["rule"],
                "manual"
            );
            assert!(entry["changes"][1]["after"]["derivation"].is_null());
            assert_eq!(entry["changes"][5]["after"]["lock"]["given"], json!([]));
        }

        // An insert policy version 1 never wrote is refused.
        let mut entry = json!({
            "timestamp": "2026-03-01T08:00:00Z", "user": "t",
            "action": { "type": "command", "command": { "type": "unlock_numbering" } },
            "changes": [{ "type": "numbering_changed", "before": { "lock": null },
                "after": { "lock": { "insert_policy": "other" } } }]
        });
        assert!((MIGRATIONS[0].audit_entry)(&mut entry).is_err());
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
