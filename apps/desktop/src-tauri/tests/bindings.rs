//! Fails when the committed TypeScript bindings differ from the Rust types (T0.3, NFR-MNT-03).
//!
//! On a mismatch the test writes the fresh bindings, so a second run passes. Review the diff of
//! `apps/desktop/src/lib/ipc/bindings.ts` and commit it together with the Rust change.

use std::fs;

#[test]
fn committed_bindings_match_rust_types() {
    let committed_path = dimo_desktop::bindings_path();
    let generated_path = std::env::temp_dir().join(format!(
        "dimo-bindings-{}-{:?}.ts",
        std::process::id(),
        std::thread::current().id()
    ));

    dimo_desktop::export_bindings(&dimo_desktop::specta_builder(), &generated_path)
        .expect("export bindings");
    let generated = fs::read_to_string(&generated_path).expect("read generated bindings");
    let _ = fs::remove_file(&generated_path);

    let committed = fs::read_to_string(&committed_path).unwrap_or_default();
    if committed != generated {
        fs::write(&committed_path, &generated).expect("write regenerated bindings");
        panic!(
            "{} was out of date and has been regenerated. Review and commit it.",
            committed_path.display()
        );
    }
}
