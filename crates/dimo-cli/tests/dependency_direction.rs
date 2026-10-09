//! Enforces the crate dependency direction from `docs/dev/rust.md` (T0.1).
//!
//! Reads every `crates/*/Cargo.toml` and checks each `dimo-*` dependency against an allow list.
//! Extend the allow list only when rust.md permits the new edge.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Allowed internal dependencies per crate. `dimo-cli` may depend on everything.
const ALLOWED: &[(&str, &[&str])] = &[
    ("dimo-core", &[]),
    ("dimo-notation", &["dimo-core"]),
    ("dimo-tolerance", &["dimo-core", "dimo-notation"]),
    ("dimo-pdf", &["dimo-core"]),
    ("dimo-vision", &["dimo-core"]),
    (
        "dimo-detect",
        &[
            "dimo-core",
            "dimo-pdf",
            "dimo-vision",
            "dimo-notation",
            "dimo-tolerance",
        ],
    ),
    ("dimo-io", &["dimo-core", "dimo-notation", "dimo-tolerance"]),
    (
        "dimo-report",
        &["dimo-core", "dimo-notation", "dimo-tolerance"],
    ),
];

/// Crate names declared in any dependency table of a manifest.
///
/// Handles inline entries (`x.workspace = true`, `x = { .. }`), table headers
/// (`[dependencies.x]`) and renames (`alias = { package = "x", .. }`).
fn dependency_names(manifest: &str) -> Vec<String> {
    let mut in_deps = false;
    let mut names = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if let Some(header) = line.strip_prefix('[') {
            let header = header.trim_end_matches(']');
            in_deps = false;
            if let Some((table, name)) = header.rsplit_once('.')
                && table.ends_with("dependencies")
            {
                names.push(name.to_string());
            } else {
                in_deps = header.ends_with("dependencies");
            }
            continue;
        }
        if let Some(package) = package_key(line) {
            if in_deps {
                names.push(package);
            } else if let Some(last) = names.last_mut() {
                // `package = ".."` inside a `[dependencies.alias]` table.
                *last = package;
            }
            continue;
        }
        if in_deps
            && let Some(name) = line.split(['=', '.', ' ']).next()
            && !name.is_empty()
            && !name.starts_with('#')
        {
            names.push(name.to_string());
        }
    }
    names
}

/// Value of a `package = ".."` key on this line, if any.
fn package_key(line: &str) -> Option<String> {
    let rest = line
        .split("package")
        .nth(1)?
        .trim_start()
        .strip_prefix('=')?;
    let value = rest.trim_start().strip_prefix('"')?;
    Some(value.split('"').next()?.to_string())
}

#[test]
fn crate_dependencies_follow_rust_md() {
    let crates_dir =
        Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").unwrap_or_default()).join("..");
    let allowed: BTreeMap<_, _> = ALLOWED.iter().copied().collect();
    let mut violations = Vec::new();
    let mut checked = 0;

    for entry in fs::read_dir(&crates_dir).unwrap() {
        let dir = entry.unwrap().path();
        let manifest_path = dir.join("Cargo.toml");
        let Ok(manifest) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let deps = dependency_names(&manifest);

        if deps.iter().any(|d| d == "tauri" || d.starts_with("tauri-")) {
            violations.push(format!("{name} depends on tauri"));
        }
        if name == "dimo-cli" {
            continue;
        }
        let Some(ok) = allowed.get(name.as_str()) else {
            violations.push(format!("{name} is missing from the allow list"));
            continue;
        };
        checked += 1;
        for dep in deps.iter().filter(|d| d.starts_with("dimo-")) {
            if !ok.contains(&dep.as_str()) {
                violations.push(format!("{name} -> {dep} is not allowed"));
            }
        }
    }

    assert_eq!(
        checked,
        ALLOWED.len(),
        "not every library crate was checked"
    );
    assert!(violations.is_empty(), "forbidden edges: {violations:#?}");
}

#[test]
fn parser_finds_dependencies() {
    let manifest = "[package]\nname = \"x\"\n\n[dependencies]\ndimo-core.workspace = true\n\
                    clap = { workspace = true }\n\n[dev-dependencies]\ndimo-io = \"1\"\n";
    assert_eq!(dependency_names(manifest), ["dimo-core", "clap", "dimo-io"]);
}

#[test]
fn parser_handles_tables_and_renames() {
    let manifest = "[dependencies.dimo-pdf]\nworkspace = true\n\n[dependencies]\n\
                    core = { package = \"dimo-core\", workspace = true }\n\n\
                    [target.'cfg(unix)'.dependencies.alias]\npackage = \"tauri\"\n";
    assert_eq!(
        dependency_names(manifest),
        ["dimo-pdf", "dimo-core", "tauri"]
    );
}
