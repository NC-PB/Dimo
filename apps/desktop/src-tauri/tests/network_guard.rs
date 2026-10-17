//! The default build makes no network call until "Check for update" is clicked (R.9, ADR 0008,
//! rule 9). The webview cannot reach the internet: its CSP allows only the app itself and the
//! IPC (`tauri.conf.json`, checked below). On the Rust side the only network code is
//! `src/update.rs`, and its real source `GitHub` is built only inside the command that the
//! button calls. This test fails if network code appears anywhere else.

use std::fs;
use std::path::{Path, PathBuf};

/// The one file allowed to make network requests, relative to the repository root.
const NETWORK_MODULE: &str = "apps/desktop/src-tauri/src/update.rs";

/// Words that mean network code in Rust sources.
const NETWORK_WORDS: [&str; 6] = [
    "ureq",
    "reqwest",
    "hyper::",
    "TcpStream",
    "UdpSocket",
    "ToSocketAddrs",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn files(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path
                .file_name()
                .is_some_and(|n| n != "target" && n != "node_modules")
            {
                files(&path, extension, out);
            }
        } else if path.extension().is_some_and(|e| e == extension) {
            out.push(path);
        }
    }
}

fn relative(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn network_code_only_in_the_update_module() {
    let root = root();
    let mut sources = Vec::new();
    for dir in ["crates", "tools", "apps/desktop/src-tauri/src"] {
        files(&root.join(dir), "rs", &mut sources);
    }
    assert!(
        sources.len() > 20,
        "scan found only {} files",
        sources.len()
    );
    let mut offenders = Vec::new();
    for file in sources {
        let name = relative(&file);
        if name == NETWORK_MODULE {
            continue;
        }
        let text = fs::read_to_string(&file).unwrap();
        for word in NETWORK_WORDS {
            if text.contains(word) {
                offenders.push(format!("{name}: {word}"));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "network code outside {NETWORK_MODULE}: {offenders:?}"
    );
}

#[test]
fn only_the_desktop_shell_depends_on_the_http_client() {
    let root = root();
    let mut manifests = Vec::new();
    for dir in ["crates", "tools", "apps/desktop/src-tauri"] {
        files(&root.join(dir), "toml", &mut manifests);
    }
    let users: Vec<String> = manifests
        .iter()
        .filter(|m| m.file_name().is_some_and(|n| n == "Cargo.toml"))
        .filter(|m| {
            fs::read_to_string(m)
                .unwrap()
                .lines()
                .any(|l| l.trim_start().starts_with("ureq"))
        })
        .map(|m| relative(m))
        .collect();
    assert_eq!(users, ["apps/desktop/src-tauri/Cargo.toml"]);
}

#[test]
fn the_real_source_is_built_only_by_the_button_command() {
    let text = fs::read_to_string(root().join(NETWORK_MODULE)).unwrap();
    let calls: Vec<usize> = text
        .match_indices("GitHub::new()")
        .map(|(i, _)| i)
        .collect();
    assert_eq!(calls.len(), 1, "GitHub::new() must be called exactly once");
    let command = text
        .find("pub async fn check_for_update")
        .expect("the check_for_update command");
    let end = text[command..]
        .find("\n}\n")
        .map(|i| command + i)
        .expect("end of check_for_update");
    assert!(
        (command..end).contains(&calls[0]),
        "GitHub::new() is called outside check_for_update"
    );
    // Nothing starts the command on its own: no Rust code calls it, the frontend only on click.
    let mut sources = Vec::new();
    files(
        &root().join("apps/desktop/src-tauri/src"),
        "rs",
        &mut sources,
    );
    for file in sources {
        let text = fs::read_to_string(&file).unwrap();
        let uses = text.matches("check_for_update").count();
        let allowed = match relative(&file).as_str() {
            NETWORK_MODULE => continue, // the command itself and its docs
            "apps/desktop/src-tauri/src/lib.rs" => 1, // collect_commands!
            _ => 0,
        };
        assert_eq!(uses, allowed, "{}", relative(&file));
    }
}

#[test]
fn the_webview_cannot_reach_the_internet() {
    let conf = fs::read_to_string(root().join("apps/desktop/src-tauri/tauri.conf.json")).unwrap();
    let conf: serde_json::Value = serde_json::from_str(&conf).unwrap();
    let csp = conf["app"]["security"]["csp"].as_str().expect("a CSP");
    let connect = csp
        .split(';')
        .map(str::trim)
        .find_map(|d| d.strip_prefix("connect-src "))
        .expect("a connect-src directive");
    for source in connect.split_whitespace() {
        assert!(
            ["'self'", "ipc:", "http://ipc.localhost"].contains(&source),
            "connect-src allows {source}"
        );
    }
}
