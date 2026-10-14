//! The command table `commands.txt` must equal the commands registered in `collect_commands!`
//! (F6, NFR-SEC-01). A command missing from the table gets no permission and fails at run time;
//! a table entry without a command is a dead permission. Both fail here instead.

use std::collections::BTreeSet;

/// Names listed in `commands.txt`, without comments and blank lines.
fn table_commands() -> BTreeSet<String> {
    include_str!("../commands.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// Names of the registered commands, read from the generated bindings, where each command calls
/// `__TAURI_INVOKE[<Ok>]("<name>"`. The collector offers no public list of names.
fn registered_commands() -> BTreeSet<String> {
    let path = std::env::temp_dir().join(format!("dimo-command-table-{}.ts", std::process::id()));
    dimo_desktop::export_bindings(&dimo_desktop::specta_builder(), &path).expect("export bindings");
    let text = std::fs::read_to_string(&path).expect("read generated bindings");
    let _ = std::fs::remove_file(&path);

    text.split("__TAURI_INVOKE")
        .skip(1)
        .filter_map(|rest| {
            // The call is `("name"` or `<Ok>("name"`; the import line has no quote and is skipped.
            let after = rest.split_once("(\"")?.1;
            after.split('"').next()
        })
        .map(str::to_owned)
        .collect()
}

#[test]
fn table_equals_registered_commands() {
    let table = table_commands();
    let registered = registered_commands();
    assert!(!registered.is_empty(), "no registered commands found");

    let missing_in_table: Vec<_> = registered.difference(&table).collect();
    let missing_in_registry: Vec<_> = table.difference(&registered).collect();
    assert!(
        missing_in_table.is_empty() && missing_in_registry.is_empty(),
        "commands.txt and collect_commands! differ.\n\
         registered but not in commands.txt (add them): {missing_in_table:?}\n\
         in commands.txt but not registered (remove or register): {missing_in_registry:?}"
    );
}

#[test]
fn table_has_no_duplicates() {
    let lines = include_str!("../commands.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .count();
    assert_eq!(lines, table_commands().len(), "duplicate command in table");
}
