//! "Check for update" in File > About (R.9, ADR 0008, D-05, NFR-SEC-03).
//!
//! Only when the user clicks the button, DimoMark sends one HTTPS GET to the GitHub Releases API
//! ([`LATEST_RELEASE_API`]) and compares the version of the latest published release with its
//! own. The request carries a User-Agent with the app version and an Accept header, nothing
//! else; no cookies, no body. Nothing is downloaded or installed: a newer version is shown with
//! a link to its release page, which the browser opens ([`crate::opener::open_url`]).
//!
//! This module is the only place with network code (rule 9). `tests/network_guard.rs` checks
//! that `ureq` is used nowhere else and that the real source [`GitHub`] is only built inside
//! [`check_for_update`], so the app makes no network call until the button is clicked.
//!
//! An organization can hide the button with a policy file ([`policy_path`]):
//! `{"update_check": false}`.

use std::path::PathBuf;
use std::time::Duration;

use semver::Version;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ipc::{CommandError, ExportReason, blocking};
use crate::opener::{RELEASES_URL, is_release_page, open_url};

/// The one address the update check asks. GitHub answers with the latest published release;
/// drafts and pre-releases are never returned, 404 means there is none yet.
pub const LATEST_RELEASE_API: &str = "https://api.github.com/repos/NC-PB/Dimo/releases/latest";

/// Time limit for the whole request, connect to last byte.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Largest response body that is read. A release description is a few kilobytes.
const BODY_LIMIT: u64 = 1024 * 1024;

/// Result of a click on "Check for update". Every case has its own message in the frontend, so
/// the command never fails with an error dump.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum UpdateCheck {
    /// A newer release exists.
    Available {
        /// Its version, without a leading `v`, for example `0.2.0`.
        version: String,
        /// Its release page, always below [`RELEASES_URL`].
        url: String,
    },
    /// The running version is the latest published one, or newer.
    Latest {
        /// The running version.
        version: String,
    },
    /// No release has been published yet.
    NoRelease,
    /// GitHub could not be reached: no network, name lookup, connection, TLS or timeout.
    Offline,
    /// GitHub answered with something unexpected. `detail` is short English text, also logged.
    Failed {
        /// What went wrong, for a bug report.
        detail: String,
    },
    /// An organization policy file hides the update check.
    Disabled,
}

/// What a [`ReleaseSource`] returns: the HTTP status and the body text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// HTTP status code.
    pub status: u16,
    /// Response body, at most [`BODY_LIMIT`] bytes.
    pub body: String,
}

/// Why a [`ReleaseSource`] got no answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchError {
    /// The server could not be reached (see [`UpdateCheck::Offline`]).
    Offline(String),
    /// Any other failure, for example a body above the limit.
    Other(String),
}

/// Where the latest release comes from. The app uses [`GitHub`]; tests pass fakes, so they never
/// touch the network.
pub trait ReleaseSource {
    /// Asks for the latest published release.
    fn latest(&self) -> Result<Answer, FetchError>;
}

/// The GitHub Releases API, one GET to [`LATEST_RELEASE_API`].
pub struct GitHub {
    /// Sent as User-Agent, which the GitHub API requires: `DimoMark/<version>`.
    user_agent: String,
}

impl GitHub {
    /// The source for the running app version.
    fn new() -> Self {
        Self {
            user_agent: format!("DimoMark/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

impl ReleaseSource for GitHub {
    fn latest(&self) -> Result<Answer, FetchError> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .https_only(true)
            // The one host: a redirect elsewhere is not followed but fails (3xx below).
            .max_redirects(0)
            .http_status_as_error(false)
            .user_agent(self.user_agent.as_str())
            .accept("application/vnd.github+json")
            .build()
            .into();
        let mut response = agent
            .get(LATEST_RELEASE_API)
            .call()
            .map_err(|e| fetch_error(&e))?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(BODY_LIMIT)
            .read_to_string()
            .map_err(|e| fetch_error(&e))?;
        Ok(Answer { status, body })
    }
}

/// Sorts a ureq error into offline and other failures. TLS errors such as an unknown
/// certificate are other failures, their text names the cause.
fn fetch_error(error: &ureq::Error) -> FetchError {
    let text = error.to_string();
    match error {
        ureq::Error::Timeout(_)
        | ureq::Error::HostNotFound
        | ureq::Error::ConnectionFailed
        | ureq::Error::Io(_)
        | ureq::Error::ConnectProxyFailed(_) => FetchError::Offline(text),
        // TLS errors are not "offline": a network that inspects HTTPS with its own certificate
        // reaches GitHub, but DimoMark trusts only the built in roots. The detail tells why.
        _ => FetchError::Other(text),
    }
}

/// The fields of the API answer that are used.
#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: Option<String>,
}

/// Asks `source` for the latest release and compares it with `current`.
pub fn check(current: &Version, source: &dyn ReleaseSource) -> UpdateCheck {
    let answer = match source.latest() {
        Ok(answer) => answer,
        Err(FetchError::Offline(detail)) => {
            tracing::info!("update check: GitHub not reachable: {detail}");
            return UpdateCheck::Offline;
        }
        Err(FetchError::Other(detail)) => return failed(detail),
    };
    match answer.status {
        200 => {}
        404 => return UpdateCheck::NoRelease,
        300..=399 => {
            return failed(format!(
                "GitHub answered with a redirect (HTTP status {}), not followed",
                answer.status
            ));
        }
        status => return failed(format!("GitHub answered with HTTP status {status}")),
    }
    let release: Release = match serde_json::from_str(&answer.body) {
        Ok(release) => release,
        Err(e) => return failed(format!("unexpected answer from GitHub: {e}")),
    };
    let Some(latest) = parse_tag(&release.tag_name) else {
        return failed(format!(
            "the release tag {:?} is not a version",
            release.tag_name
        ));
    };
    if latest <= *current {
        return UpdateCheck::Latest {
            version: current.to_string(),
        };
    }
    let url = release
        .html_url
        .filter(|url| is_release_page(url))
        .unwrap_or_else(|| format!("{RELEASES_URL}/tag/v{latest}"));
    UpdateCheck::Available {
        version: latest.to_string(),
        url,
    }
}

/// Logs and returns [`UpdateCheck::Failed`].
fn failed(detail: String) -> UpdateCheck {
    tracing::warn!("update check failed: {detail}");
    UpdateCheck::Failed { detail }
}

/// The version of a release tag, `v0.2.0` or `0.2.0`. Pre-release tags such as `v0.2.0-rc.1`
/// are versions too and order before the release (semver rules).
pub fn parse_tag(tag: &str) -> Option<Version> {
    let tag = tag.trim();
    Version::parse(tag.strip_prefix(['v', 'V']).unwrap_or(tag)).ok()
}

/// Whether the policy file text `text` allows the update check (NFR-SEC-03). Unknown keys are
/// ignored and a missing `update_check` means allowed. A file that is not a JSON object, or whose
/// `update_check` is not a boolean, hides the button: an administrator who wrote a policy wanted
/// something restricted.
pub fn parse_policy(text: &str) -> bool {
    let value = match serde_json::from_str::<serde_json::Value>(text) {
        Ok(value) => value,
        Err(e) => {
            tracing::warn!("policy file is not valid JSON, update check hidden: {e}");
            return false;
        }
    };
    let allowed = value.as_object().and_then(|policy| {
        policy
            .get("update_check")
            .map_or(Some(true), serde_json::Value::as_bool)
    });
    if allowed.is_none() {
        tracing::warn!(
            "policy file is not an object with a boolean update_check, update check hidden"
        );
    }
    allowed.unwrap_or(false)
}

/// Whether the policy file at `path` allows the update check. No file means allowed; a file that
/// cannot be read hides the button.
pub fn policy_allows(path: Option<PathBuf>) -> bool {
    let Some(path) = path else {
        return true;
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => parse_policy(&text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => true,
        Err(e) => {
            tracing::warn!(
                "cannot read the policy file {}, update check hidden: {e}",
                path.display()
            );
            false
        }
    }
}

/// The system wide policy file of this operating system, writable only by an administrator:
/// - macOS: `/Library/Application Support/DimoMark/policy.json`
/// - Windows: `%ProgramData%\DimoMark\policy.json`
/// - Linux and others: `/etc/dimomark/policy.json`
pub fn policy_path() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        Some(PathBuf::from(
            "/Library/Application Support/DimoMark/policy.json",
        ))
    } else if cfg!(windows) {
        let base = std::env::var_os("ProgramData")
            .map_or_else(|| PathBuf::from(r"C:\ProgramData"), PathBuf::from);
        Some(base.join("DimoMark").join("policy.json"))
    } else {
        Some(PathBuf::from("/etc/dimomark/policy.json"))
    }
}

/// Whether "Check for update" is shown. Reads the policy file only, no network; on the
/// blocking pool like every file access.
#[tauri::command]
#[specta::specta]
pub async fn update_check_allowed() -> Result<bool, CommandError> {
    blocking(|| Ok(policy_allows(policy_path()))).await
}

/// Asks GitHub for the latest published release, once, and compares it with the running
/// version. Runs on the blocking pool. The only caller of the network code (rule 9).
#[tauri::command]
#[specta::specta]
pub async fn check_for_update() -> Result<UpdateCheck, CommandError> {
    blocking(|| {
        if !policy_allows(policy_path()) {
            return Ok(UpdateCheck::Disabled);
        }
        let Some(current) = parse_tag(env!("CARGO_PKG_VERSION")) else {
            return Ok(failed(format!(
                "the app version {} is not a version",
                env!("CARGO_PKG_VERSION")
            )));
        };
        Ok(check(&current, &GitHub::new()))
    })
    .await
}

/// Opens the release page `url` from [`UpdateCheck::Available`] in the browser. Any address
/// that is not below [`RELEASES_URL`] is refused.
#[tauri::command]
#[specta::specta]
#[allow(
    clippy::needless_pass_by_value,
    reason = "Tauri commands take arguments by value"
)]
pub async fn open_release_page(url: String) -> Result<(), CommandError> {
    blocking(move || {
        open_url(&url).map_err(|e| {
            CommandError::export(
                ExportReason::OpenFailed,
                format_args!("cannot open {url}: {e}"),
            )
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source with a fixed answer.
    struct Fake(Result<Answer, FetchError>);

    impl ReleaseSource for Fake {
        fn latest(&self) -> Result<Answer, FetchError> {
            self.0.clone()
        }
    }

    fn ok(body: &str) -> Fake {
        Fake(Ok(Answer {
            status: 200,
            body: body.to_owned(),
        }))
    }

    fn release(tag: &str) -> Fake {
        ok(&format!(
            r#"{{"tag_name":"{tag}","html_url":"https://github.com/NC-PB/Dimo/releases/tag/{tag}","name":"x","assets":[]}}"#
        ))
    }

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn a_newer_release_is_available_with_its_page() {
        assert_eq!(
            check(&v("0.1.0"), &release("v0.2.0")),
            UpdateCheck::Available {
                version: "0.2.0".to_owned(),
                url: "https://github.com/NC-PB/Dimo/releases/tag/v0.2.0".to_owned(),
            }
        );
    }

    #[test]
    fn the_same_or_an_older_release_is_latest() {
        for tag in ["v0.1.0", "0.1.0", "v0.0.9", "v0.1.0-rc.2"] {
            assert_eq!(
                check(&v("0.1.0"), &release(tag)),
                UpdateCheck::Latest {
                    version: "0.1.0".to_owned()
                },
                "{tag}"
            );
        }
    }

    #[test]
    fn a_release_is_newer_than_its_pre_releases() {
        assert!(matches!(
            check(&v("0.2.0-rc.1"), &release("v0.2.0")),
            UpdateCheck::Available { version, .. } if version == "0.2.0"
        ));
        assert!(parse_tag("v0.2.0-rc.1") < parse_tag("v0.2.0-rc.2"));
        assert!(parse_tag("v0.2.0-rc.2") < parse_tag("v0.2.0"));
        assert!(parse_tag("v0.9.0") < parse_tag("v0.10.0"));
        assert_eq!(parse_tag(" V1.2.3 "), Some(v("1.2.3")));
        for bad in ["", "v", "latest", "v1.2", "release-1"] {
            assert_eq!(parse_tag(bad), None, "{bad}");
        }
    }

    #[test]
    fn offline_and_missing_releases_have_their_own_answers() {
        let offline = Fake(Err(FetchError::Offline("dns error".to_owned())));
        assert_eq!(check(&v("0.1.0"), &offline), UpdateCheck::Offline);
        let none = Fake(Ok(Answer {
            status: 404,
            body: r#"{"message":"Not Found"}"#.to_owned(),
        }));
        assert_eq!(check(&v("0.1.0"), &none), UpdateCheck::NoRelease);
    }

    #[test]
    fn unexpected_answers_fail_with_a_short_detail() {
        let limited = Fake(Ok(Answer {
            status: 403,
            body: "rate limit".to_owned(),
        }));
        let cases = [
            (limited, "HTTP status 403"),
            (
                Fake(Ok(Answer {
                    status: 301,
                    body: String::new(),
                })),
                "redirect (HTTP status 301), not followed",
            ),
            (ok("<html>"), "unexpected answer"),
            (ok(r#"{"html_url":"x"}"#), "unexpected answer"),
            (release("nightly"), "not a version"),
            (
                Fake(Err(FetchError::Other("body too large".to_owned()))),
                "body too large",
            ),
        ];
        for (source, expected) in cases {
            let result = check(&v("0.1.0"), &source);
            assert!(
                matches!(&result, UpdateCheck::Failed { detail } if detail.contains(expected)),
                "{result:?}"
            );
        }
    }

    #[test]
    fn a_foreign_release_page_is_replaced_by_the_own_one() {
        let source = ok(r#"{"tag_name":"v0.3.0","html_url":"https://evil.example/x"}"#);
        assert_eq!(
            check(&v("0.1.0"), &source),
            UpdateCheck::Available {
                version: "0.3.0".to_owned(),
                url: format!("{RELEASES_URL}/tag/v0.3.0"),
            }
        );
        let no_url = ok(r#"{"tag_name":"v0.3.0"}"#);
        assert!(matches!(
            check(&v("0.1.0"), &no_url),
            UpdateCheck::Available { url, .. } if url == format!("{RELEASES_URL}/tag/v0.3.0")
        ));
    }

    #[test]
    fn the_policy_file_can_hide_the_update_check() {
        assert!(parse_policy("{}"));
        assert!(parse_policy(r#"{"update_check": true}"#));
        assert!(parse_policy(r#"{"update_check": true, "other": 1}"#));
        assert!(!parse_policy(r#"{"update_check": false}"#));
        for invalid in [
            "",
            "false",
            "[]",
            r#"{"update_check": "no"}"#,
            "{update_check:0}",
        ] {
            assert!(!parse_policy(invalid), "{invalid}");
        }
    }

    #[test]
    fn a_missing_policy_file_allows_and_an_existing_one_decides() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("policy.json");
        assert!(policy_allows(None));
        assert!(policy_allows(Some(path.clone())));
        std::fs::write(&path, r#"{"update_check": false}"#).unwrap();
        assert!(!policy_allows(Some(path.clone())));
        std::fs::write(&path, r#"{"update_check": true}"#).unwrap();
        assert!(policy_allows(Some(path)));
        // A directory where the file should be cannot be read: hidden.
        assert!(!policy_allows(Some(dir.path().to_path_buf())));
    }

    #[test]
    fn the_app_version_is_a_version() {
        assert!(parse_tag(env!("CARGO_PKG_VERSION")).is_some());
    }

    #[test]
    fn the_policy_lives_in_a_system_wide_folder() {
        let path = policy_path().unwrap();
        assert!(path.ends_with("policy.json"), "{}", path.display());
        assert!(path.is_absolute(), "{}", path.display());
    }

    #[test]
    fn the_api_address_belongs_to_the_release_page_repository() {
        let repo = RELEASES_URL
            .strip_prefix("https://github.com/")
            .and_then(|r| r.strip_suffix("/releases"))
            .unwrap();
        assert_eq!(
            LATEST_RELEASE_API,
            format!("https://api.github.com/repos/{repo}/releases/latest")
        );
    }

    #[test]
    fn only_the_release_page_opens() {
        let refused =
            tauri::async_runtime::block_on(open_release_page("https://evil.example".to_owned()))
                .unwrap_err();
        assert!(
            matches!(
                refused,
                CommandError::Export {
                    reason: ExportReason::OpenFailed,
                    ..
                }
            ),
            "{refused:?}"
        );
    }
}
