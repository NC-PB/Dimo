//! What commands need from the outside world: new IDs, the current time and the user name.
//!
//! `dimo-core` has no IO and no global randomness or clock (rule 11). The caller passes an
//! [`Environment`]: the desktop shell uses random UUIDs, the system clock and the configured
//! audit user (D-27); tests and deterministic tools use [`FixedEnvironment`].

use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A point in time as an RFC 3339 UTC string, for example `2026-10-09T14:03:21.250Z`.
///
/// Kept as text so `dimo-core` needs no date library. Timestamps written by the same provider
/// have the same format, so they sort chronologically as strings.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "specta", specta(transparent))]
pub struct Timestamp(String);

/// Error for text that is not an RFC 3339 UTC timestamp.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid timestamp {0:?}, expected RFC 3339 UTC such as 2026-10-09T14:03:21Z")]
pub struct TimestampError(String);

impl Timestamp {
    /// Checks the format `YYYY-MM-DDTHH:MM:SS[.fraction]Z`.
    pub fn parse(text: &str) -> Result<Self, TimestampError> {
        if is_rfc3339_utc(text) {
            Ok(Self(text.to_owned()))
        } else {
            Err(TimestampError(text.to_owned()))
        }
    }

    /// Formats milliseconds since 1970-01-01T00:00:00Z with millisecond precision.
    pub fn from_unix_millis(millis: u64) -> Self {
        let (days, rest) = (millis / 86_400_000, millis % 86_400_000);
        let (year, month, day) = civil_from_days(days);
        let (h, m, s, ms) = (
            rest / 3_600_000,
            rest / 60_000 % 60,
            rest / 1000 % 60,
            rest % 1000,
        );
        Self(format!(
            "{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}.{ms:03}Z"
        ))
    }

    /// The timestamp text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Timestamp {
    type Error = TimestampError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}

impl From<Timestamp> for String {
    fn from(timestamp: Timestamp) -> Self {
        timestamp.0
    }
}

impl JsonSchema for Timestamp {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Timestamp".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "pattern": r"^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]{1,9})?Z$",
            "description": "RFC 3339 UTC timestamp, e.g. \"2026-10-09T14:03:21.250Z\"."
        })
    }
}

fn is_rfc3339_utc(text: &str) -> bool {
    let b = text.as_bytes();
    let digits = |range: std::ops::Range<usize>| b[range].iter().all(u8::is_ascii_digit);
    if b.len() < 20 || b[b.len() - 1] != b'Z' {
        return false;
    }
    let fixed = digits(0..4)
        && b[4] == b'-'
        && digits(5..7)
        && b[7] == b'-'
        && digits(8..10)
        && b[10] == b'T'
        && digits(11..13)
        && b[13] == b':'
        && digits(14..16)
        && b[16] == b':'
        && digits(17..19);
    let fraction = &b[19..b.len() - 1];
    let fraction_ok = fraction.is_empty()
        || (fraction[0] == b'.'
            && (2..=10).contains(&fraction.len())
            && fraction[1..].iter().all(u8::is_ascii_digit));
    fixed && fraction_ok
}

/// Gregorian date from days since 1970-01-01 (algorithm by Howard Hinnant, `civil_from_days`).
fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

/// Provider of IDs, time and user name for commands (D-27, rule 11).
pub trait Environment {
    /// A new UUID that was never used in the project. Production uses random version 4 UUIDs.
    fn new_uuid(&mut self) -> Uuid;
    /// The current time.
    fn now(&mut self) -> Timestamp;
    /// Name recorded in audit entries: the OS user name or the name set in the settings (D-27).
    fn user_name(&self) -> String;
}

/// Deterministic [`Environment`]: sequential UUIDs, a fixed time and a fixed user.
///
/// For tests and for tools that must produce identical output on every run.
#[derive(Debug, Clone)]
pub struct FixedEnvironment {
    next: u128,
    time: Timestamp,
    user: String,
}

impl FixedEnvironment {
    /// UUIDs start at `00000000-0000-0000-0000-000000000001`, the time is
    /// `2026-01-01T00:00:00Z`, the user is `test`.
    pub fn new() -> Self {
        Self {
            next: 1,
            time: Timestamp("2026-01-01T00:00:00Z".to_owned()),
            user: "test".to_owned(),
        }
    }

    /// Sets the time returned by [`Environment::now`].
    pub fn set_time(&mut self, time: Timestamp) {
        self.time = time;
    }

    /// Sets the user name.
    pub fn set_user(&mut self, user: impl Into<String>) {
        self.user = user.into();
    }
}

impl Default for FixedEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment for FixedEnvironment {
    fn new_uuid(&mut self) -> Uuid {
        let uuid = Uuid::from_u128(self.next);
        self.next += 1;
        uuid
    }

    fn now(&mut self) -> Timestamp {
        self.time.clone()
    }

    fn user_name(&self) -> String {
        self.user.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_unix_millis() {
        let cases = [
            (0, "1970-01-01T00:00:00.000Z"),
            (951_782_400_000, "2000-02-29T00:00:00.000Z"),
            (1_791_554_601_250, "2026-10-09T14:03:21.250Z"),
            (4_107_542_399_999, "2100-02-28T23:59:59.999Z"),
        ];
        for (millis, text) in cases {
            assert_eq!(Timestamp::from_unix_millis(millis).as_str(), text);
            assert!(Timestamp::parse(text).is_ok());
        }
    }

    #[test]
    fn rejects_other_formats() {
        for text in [
            "",
            "2026-10-09",
            "2026-10-09T14:03:21",
            "2026-10-09T14:03:21+02:00",
            "2026-10-09 14:03:21Z",
            "2026-10-09T14:03:21.Z",
            "2026-1O-09T14:03:21Z",
        ] {
            assert!(Timestamp::parse(text).is_err(), "{text:?}");
        }
        assert!(serde_json::from_str::<Timestamp>("\"yesterday\"").is_err());
    }

    #[test]
    fn fixed_environment_is_sequential() {
        let mut env = FixedEnvironment::new();
        assert_eq!(env.new_uuid(), Uuid::from_u128(1));
        assert_eq!(env.new_uuid(), Uuid::from_u128(2));
        assert_eq!(env.now().as_str(), "2026-01-01T00:00:00Z");
    }
}
