//! Content hash of an imported drawing (FR-DOC-07).

use std::fmt;

use sha2::{Digest as _, Sha256};

/// SHA-256 of the exact input bytes of a drawing.
///
/// FR-DOC-07: stored with every imported drawing so a report can be tied to the exact file.
/// Displays as 64 lowercase hex digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Hashes the given bytes.
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// The raw 32 digest bytes.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Lowercase hex form, 64 characters.
    pub fn to_hex(&self) -> String {
        self.to_string()
    }

    /// Parses the 64 digit hex form written by [`ContentHash::to_hex`]. Upper case digits are
    /// accepted. Returns `None` for any other input.
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
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::ContentHash;

    #[test]
    fn known_vectors() {
        // FIPS 180-2 test vectors.
        assert_eq!(
            ContentHash::of(b"").to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            ContentHash::of(b"abc").to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hex_round_trip() {
        let hash = ContentHash::of(b"abc");
        assert_eq!(ContentHash::from_hex(&hash.to_hex()), Some(hash));
        assert_eq!(
            ContentHash::from_hex(&hash.to_hex().to_uppercase()),
            Some(hash)
        );
        assert_eq!(ContentHash::from_hex(""), None);
        assert_eq!(ContentHash::from_hex(&hash.to_hex()[1..]), None);
        assert_eq!(ContentHash::from_hex(&"g".repeat(64)), None);
        assert_eq!(ContentHash::from_hex(&"é".repeat(32)), None);
    }
}
