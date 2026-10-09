//! The [`Environment`] of the desktop app: random IDs, the system clock and the audit user.

use std::time::{SystemTime, UNIX_EPOCH};

use dimo_core::{Environment, Timestamp};
use uuid::Uuid;

/// User name recorded when the operating system does not tell one.
const UNKNOWN_USER: &str = "unknown";

/// Random version 4 UUIDs, the system clock in UTC and the audit user name (D-27).
#[derive(Debug, Clone)]
pub struct DesktopEnvironment {
    user: String,
}

impl DesktopEnvironment {
    /// Starts with the operating system user name (D-27).
    pub fn new() -> Self {
        Self {
            user: os_user_name(),
        }
    }

    /// Sets the name recorded in audit entries, for the user name setting (D-27, T1.9).
    /// Empty or blank names fall back to the operating system user.
    pub fn set_user_name(&mut self, name: &str) {
        let name = name.trim();
        self.user = if name.is_empty() {
            os_user_name()
        } else {
            name.to_owned()
        };
    }
}

impl Default for DesktopEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment for DesktopEnvironment {
    fn new_uuid(&mut self) -> Uuid {
        Uuid::new_v4()
    }

    fn now(&mut self) -> Timestamp {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
        Timestamp::from_unix_millis(millis)
    }

    fn user_name(&self) -> String {
        self.user.clone()
    }
}

/// The login name from the environment: `USER` on macOS and Linux, `USERNAME` on Windows.
fn os_user_name() -> String {
    ["USER", "USERNAME", "LOGNAME"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .map(|value| value.trim().to_owned())
        .find(|value| !value.is_empty())
        .unwrap_or_else(|| UNKNOWN_USER.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_random_and_time_is_utc() {
        let mut env = DesktopEnvironment::new();
        assert_ne!(env.new_uuid(), env.new_uuid());
        assert_eq!(env.new_uuid().get_version_num(), 4);
        let now = env.now();
        assert!(now.as_str().ends_with('Z'), "{now}");
        assert!(now.as_str() > "2026-01-01", "{now}");
    }

    #[test]
    fn user_name_can_be_set_and_reset() {
        let mut env = DesktopEnvironment::new();
        let os = env.user_name();
        assert_ne!(os, "");
        env.set_user_name("  Anna Muster ");
        assert_eq!(env.user_name(), "Anna Muster");
        env.set_user_name(" ");
        assert_eq!(env.user_name(), os);
    }
}
