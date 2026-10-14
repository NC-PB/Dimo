//! The one content hash type of the workspace (FR-DOC-07, MF F1).
//!
//! [`Sha256Hex`] identifies drawings, custom tolerance tables and written reports in the
//! project file, and drawings in the tile service and its caches. It holds the 32 digest bytes
//! (cheap to copy and to use as a map key) and is written as 64 lowercase hex digits in JSON,
//! JSON Schema and the TypeScript bindings.

use std::fmt;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

/// SHA-256 of a file, written as 64 lowercase hex digits (FR-DOC-07).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(try_from = "String", into = "String")]
#[cfg_attr(feature = "specta", specta(transparent))]
pub struct Sha256Hex(#[cfg_attr(feature = "specta", specta(type = String))] [u8; 32]);

/// Error for text that is not 64 lowercase hex digits.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid SHA-256 {0:?}, expected 64 lowercase hex digits")]
pub struct Sha256Error(String);

impl Sha256Hex {
    /// Hashes the given bytes.
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// The hash with these 32 digest bytes.
    pub const fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(*bytes)
    }

    /// The 32 digest bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Parses exactly 64 lowercase hex digits, the stored form.
    pub fn parse(text: &str) -> Result<Self, Sha256Error> {
        let lower = text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        Self::from_hex(text)
            .filter(|_| lower)
            .ok_or_else(|| Sha256Error(text.to_owned()))
    }

    /// Parses 64 hex digits, upper or lower case, for addresses such as tile URLs. Returns
    /// `None` for any other input.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let digits = hex.as_bytes();
        if digits.len() != 64 {
            return None;
        }
        let mut bytes = [0u8; 32];
        for (byte, &[high, low]) in bytes.iter_mut().zip(digits.as_chunks::<2>().0) {
            let high = char::from(high).to_digit(16)?;
            let low = char::from(low).to_digit(16)?;
            // Two hex digits are at most 255.
            #[allow(clippy::cast_possible_truncation)]
            {
                *byte = (high * 16 + low) as u8;
            }
        }
        Some(Self(bytes))
    }

    /// The 64 lowercase hex digits.
    pub fn to_hex(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for Sha256Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for Sha256Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha256Hex({self})")
    }
}

impl TryFrom<String> for Sha256Hex {
    type Error = Sha256Error;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}

impl From<Sha256Hex> for String {
    fn from(hash: Sha256Hex) -> Self {
        hash.to_hex()
    }
}

impl JsonSchema for Sha256Hex {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "Sha256Hex".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({ "type": "string", "pattern": "^[0-9a-f]{64}$" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vectors() {
        // FIPS 180-2 test vectors.
        assert_eq!(
            Sha256Hex::of(b"").to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            Sha256Hex::of(b"abc").to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn stored_form_is_only_lowercase_hex() {
        let zeros = "0".repeat(64);
        assert!(Sha256Hex::parse(&zeros).is_ok());
        assert!(Sha256Hex::parse(&"A".repeat(64)).is_err());
        assert!(Sha256Hex::parse("abc").is_err());
        assert_eq!(Sha256Hex::from_bytes(&[0xab; 32]).to_hex(), "ab".repeat(32));
        assert!(serde_json::from_str::<Sha256Hex>("\"xyz\"").is_err());
        let hash = Sha256Hex::of(b"abc");
        let json = serde_json::to_string(&hash).unwrap();
        assert_eq!(json, format!("\"{hash}\""));
        assert_eq!(serde_json::from_str::<Sha256Hex>(&json).unwrap(), hash);
    }

    #[test]
    fn addresses_accept_both_cases() {
        let hash = Sha256Hex::of(b"abc");
        assert_eq!(Sha256Hex::from_hex(&hash.to_hex()), Some(hash));
        assert_eq!(
            Sha256Hex::from_hex(&hash.to_hex().to_uppercase()),
            Some(hash)
        );
        assert_eq!(Sha256Hex::from_hex(""), None);
        assert_eq!(Sha256Hex::from_hex(&hash.to_hex()[1..]), None);
        assert_eq!(Sha256Hex::from_hex(&"g".repeat(64)), None);
        assert_eq!(Sha256Hex::from_hex(&"é".repeat(32)), None);
    }
}
