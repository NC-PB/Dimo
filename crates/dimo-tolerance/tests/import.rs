//! Import of a custom table into the project settings (T2.8, FR-TOL-07, M2 decision 4).

#![allow(clippy::unwrap_used)] // Test crate; rust.md allows unwrap in tests.

mod common;

use common::context::{Cfg, SHOP_TABLE, settings};
use dimo_core::derivation::TableRef;
use dimo_core::sheet::Sha256Hex;
use dimo_tolerance::{ContextError, TableError, TableSet, import_custom_table};

fn hash(c: char) -> Sha256Hex {
    Sha256Hex::parse(&c.to_string().repeat(64)).unwrap()
}

#[test]
fn a_new_table_is_added_sorted_by_id() {
    let current = settings(Cfg::DrawingRule);
    let other = SHOP_TABLE.replace(r#"id = "shop""#, r#"id = "alpha""#);
    let (next, table) = import_custom_table(
        &current,
        TableSet::shipped().unwrap(),
        [("shop", SHOP_TABLE)],
        "alpha.toml",
        &other,
        hash('b'),
    )
    .unwrap();
    assert_eq!(
        table,
        TableRef {
            id: "alpha".into(),
            version: 2
        }
    );
    let ids: Vec<&str> = next
        .custom_tables
        .iter()
        .map(|t| t.table.id.as_str())
        .collect();
    assert_eq!(ids, ["alpha", "shop"]);
    assert_eq!(next.drawing_rule, current.drawing_rule);
    assert_eq!(next.general, current.general);
}

#[test]
fn a_new_version_replaces_the_table_and_the_settings_follow() {
    let current = settings(Cfg::DrawingRule);
    let newer = SHOP_TABLE.replace("version = 2", "version = 3");
    let (next, _) = import_custom_table(
        &current,
        TableSet::shipped().unwrap(),
        [("shop", SHOP_TABLE)],
        "shop.toml",
        &newer,
        hash('c'),
    )
    .unwrap();
    assert_eq!(next.custom_tables.len(), 1);
    assert_eq!(next.custom_tables[0].table.version, 3);
    assert_eq!(next.custom_tables[0].sha256, hash('c'));
    assert_eq!(next.drawing_rule.unwrap().table.version, 3);
}

#[test]
fn refusals() {
    let current = settings(Cfg::DrawingRule);
    let import = |text: &str| {
        import_custom_table(
            &current,
            TableSet::shipped().unwrap(),
            [("shop", SHOP_TABLE)],
            "x.toml",
            text,
            hash('d'),
        )
        .unwrap_err()
    };
    // Syntax error: with the line.
    let broken = SHOP_TABLE.replace(r#"kind = "custom""#, "kind = custom");
    let error = import(&broken);
    assert!(matches!(
        error,
        ContextError::Table(TableError::Parse { .. })
    ));
    assert_eq!(error.line(), Some(5));
    // A general table cannot be a custom table.
    let general = SHOP_TABLE.replace(r#"kind = "custom""#, r#"kind = "general""#);
    assert!(matches!(import(&general), ContextError::WrongKind { .. }));
    // The id of a shipped table.
    let shipped = SHOP_TABLE.replace(r#"id = "shop""#, r#"id = "iso-2768-1""#);
    assert!(matches!(
        import(&shipped),
        ContextError::Table(TableError::Duplicate { .. })
    ));
    // The drawing rule names class a; the new version lacks it.
    let no_class = SHOP_TABLE
        .replace("version = 2", "version = 3")
        .replace(r#"columns = ["a"]"#, r#"columns = ["b"]"#);
    assert!(matches!(
        import(&no_class),
        ContextError::UnknownClass { .. }
    ));
}
