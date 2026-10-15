//! Platform independent ZIP entry metadata (FR-EXP-11, NFR-PORT-03).
//!
//! The `zip` crate records the host system of an entry in the "version made by" field of the
//! central directory. Unless the writer sets it, it uses the system the program runs on: DOS
//! (0) on Windows, Unix (3) everywhere else, and on DOS it may add DOS attribute bits to the
//! external attributes. `rust_xlsxwriter` does not set the system, so the same workbook had
//! different bytes on Windows. [`set_unix_host`] rewrites these fields in a finished archive,
//! so every platform writes the bytes a Unix build writes. It changes no other byte.

/// Host system value for Unix in the "version made by" field (APPNOTE 4.4.2).
const UNIX_HOST: u8 = 3;

const EOCD_SIGNATURE: u32 = 0x0605_4b50;
const EOCD_LEN: usize = 22;
const ZIP64_LOCATOR_SIGNATURE: u32 = 0x0706_4b50;
const ZIP64_LOCATOR_LEN: usize = 20;
const ZIP64_EOCD_SIGNATURE: u32 = 0x0606_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const CENTRAL_LEN: usize = 46;

/// Error for bytes that are not a ZIP archive this module can read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("malformed ZIP archive: {0}")]
pub struct ZipHostError(&'static str);

/// Metadata of one central directory entry, as stored. Tests use it to check that ZIP outputs
/// do not depend on the platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryMetadata {
    /// Entry name as stored.
    pub name: String,
    /// "Version made by": high byte host system (3 Unix, 0 DOS), low byte spec version.
    pub version_made_by: u16,
    /// General purpose bit flags.
    pub flags: u16,
    /// Compression method (0 stored, 8 deflate).
    pub method: u16,
    /// DOS time and date, in this order.
    pub dos_time_date: (u16, u16),
    /// External attributes: Unix mode in the high 16 bits, DOS attributes in the low 16 bits.
    pub external_attributes: u32,
}

impl EntryMetadata {
    /// Host system byte of `version_made_by`.
    pub fn host(&self) -> u8 {
        self.version_made_by.to_be_bytes()[0]
    }
}

/// Sets the host system of every central directory entry to Unix and clears the DOS attribute
/// bits (low 16 bits) of its external attributes. The Unix mode (high 16 bits), names, times,
/// compression and data stay as written. On an archive written on Unix this changes nothing.
pub fn set_unix_host(bytes: &mut [u8]) -> Result<(), ZipHostError> {
    for pos in central_entries(bytes)? {
        bytes[pos + 5] = UNIX_HOST;
        bytes[pos + 38] = 0;
        bytes[pos + 39] = 0;
    }
    Ok(())
}

/// Metadata of every central directory entry, in archive order.
pub fn entry_metadata(bytes: &[u8]) -> Result<Vec<EntryMetadata>, ZipHostError> {
    central_entries(bytes)?
        .into_iter()
        .map(|pos| {
            let name_len = usize::from(u16_at(bytes, pos + 28)?);
            let name = bytes
                .get(pos + CENTRAL_LEN..pos + CENTRAL_LEN + name_len)
                .ok_or(ZipHostError("truncated"))?;
            Ok(EntryMetadata {
                name: String::from_utf8_lossy(name).into_owned(),
                version_made_by: u16_at(bytes, pos + 4)?,
                flags: u16_at(bytes, pos + 8)?,
                method: u16_at(bytes, pos + 10)?,
                dos_time_date: (u16_at(bytes, pos + 12)?, u16_at(bytes, pos + 14)?),
                external_attributes: u32_at(bytes, pos + 38)?,
            })
        })
        .collect()
}

/// Start positions of all central directory entries, each checked to lie inside `bytes`.
fn central_entries(bytes: &[u8]) -> Result<Vec<usize>, ZipHostError> {
    let eocd = find_eocd(bytes)?;
    let mut entries = u64::from(u16_at(bytes, eocd + 10)?);
    let mut offset = u64::from(u32_at(bytes, eocd + 16)?);
    if entries == u64::from(u16::MAX) || offset == u64::from(u32::MAX) {
        // ZIP64: the real values are in the ZIP64 end of central directory record.
        let locator = eocd
            .checked_sub(ZIP64_LOCATOR_LEN)
            .ok_or(ZipHostError("missing ZIP64 locator"))?;
        if u32_at(bytes, locator)? != ZIP64_LOCATOR_SIGNATURE {
            return Err(ZipHostError("missing ZIP64 locator"));
        }
        let record = index(u64_at(bytes, locator + 8)?)?;
        if u32_at(bytes, record)? != ZIP64_EOCD_SIGNATURE {
            return Err(ZipHostError("missing ZIP64 end of central directory"));
        }
        entries = u64_at(bytes, record + 32)?;
        offset = u64_at(bytes, record + 48)?;
    }
    let mut positions = Vec::new();
    let mut pos = index(offset)?;
    for _ in 0..entries {
        if u32_at(bytes, pos)? != CENTRAL_SIGNATURE {
            return Err(ZipHostError("bad central directory entry"));
        }
        let name = usize::from(u16_at(bytes, pos + 28)?);
        let extra = usize::from(u16_at(bytes, pos + 30)?);
        let comment = usize::from(u16_at(bytes, pos + 32)?);
        let next = pos + CENTRAL_LEN + name + extra + comment;
        if next > bytes.len() {
            return Err(ZipHostError("truncated central directory"));
        }
        positions.push(pos);
        pos = next;
    }
    Ok(positions)
}

/// Position of the end of central directory record: the last signature whose comment length
/// reaches exactly to the end of the archive.
fn find_eocd(bytes: &[u8]) -> Result<usize, ZipHostError> {
    let last = bytes
        .len()
        .checked_sub(EOCD_LEN)
        .ok_or(ZipHostError("too short"))?;
    let first = last.saturating_sub(usize::from(u16::MAX));
    (first..=last)
        .rev()
        .find(|&pos| {
            u32_at(bytes, pos) == Ok(EOCD_SIGNATURE)
                && u16_at(bytes, pos + 20)
                    .is_ok_and(|comment| pos + EOCD_LEN + usize::from(comment) == bytes.len())
        })
        .ok_or(ZipHostError("no end of central directory"))
}

fn index(value: u64) -> Result<usize, ZipHostError> {
    usize::try_from(value).map_err(|_| ZipHostError("offset out of range"))
}

fn field<const N: usize>(bytes: &[u8], pos: usize) -> Result<[u8; N], ZipHostError> {
    bytes
        .get(pos..pos.saturating_add(N))
        .and_then(|s| s.try_into().ok())
        .ok_or(ZipHostError("truncated"))
}

fn u16_at(bytes: &[u8], pos: usize) -> Result<u16, ZipHostError> {
    field(bytes, pos).map(u16::from_le_bytes)
}

fn u32_at(bytes: &[u8], pos: usize) -> Result<u32, ZipHostError> {
    field(bytes, pos).map(u32::from_le_bytes)
}

fn u64_at(bytes: &[u8], pos: usize) -> Result<u64, ZipHostError> {
    field(bytes, pos).map(u64::from_le_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal stored archive with one entry `a` holding `x`, written with host `host` and
    /// external attributes `attrs`, as a DOS build of a ZIP writer would.
    fn archive(host: u8, attrs: u32) -> Vec<u8> {
        let mut b = Vec::new();
        // Local file header.
        b.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        b.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0x21, 0]); // version, flags, method, time, date
        b.extend_from_slice(&0x8cdc_1683u32.to_le_bytes()); // crc32 of "x"
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(b"ax");
        let central = b.len();
        b.extend_from_slice(&CENTRAL_SIGNATURE.to_le_bytes());
        b.extend_from_slice(&[20, host, 20, 0, 0, 0, 0, 0, 0, 0, 0x21, 0]);
        b.extend_from_slice(&0x8cdc_1683u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes()); // name
        b.extend_from_slice(&0u16.to_le_bytes()); // extra
        b.extend_from_slice(&0u16.to_le_bytes()); // comment
        b.extend_from_slice(&0u16.to_le_bytes()); // disk
        b.extend_from_slice(&0u16.to_le_bytes()); // internal attributes
        b.extend_from_slice(&attrs.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes()); // local header offset
        b.push(b'a');
        let size = b.len() - central;
        b.extend_from_slice(&EOCD_SIGNATURE.to_le_bytes());
        b.extend_from_slice(&[0, 0, 0, 0, 1, 0, 1, 0]);
        b.extend_from_slice(&u32::try_from(size).unwrap().to_le_bytes());
        b.extend_from_slice(&u32::try_from(central).unwrap().to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b
    }

    #[test]
    fn dos_host_and_attribute_bits_become_unix() {
        let mut bytes = archive(0, 0o100_600 << 16 | 0x20);
        set_unix_host(&mut bytes).unwrap();
        assert_eq!(bytes, archive(UNIX_HOST, 0o100_600 << 16));
    }

    #[test]
    fn metadata_reads_the_central_directory() {
        let meta = entry_metadata(&archive(0, 0o100_600 << 16 | 0x20)).unwrap();
        assert_eq!(
            meta,
            [EntryMetadata {
                name: "a".into(),
                version_made_by: 20,
                flags: 0,
                method: 0,
                dos_time_date: (0, 0x21),
                external_attributes: 0o100_600 << 16 | 0x20,
            }]
        );
        assert_eq!(meta[0].host(), 0);
    }

    #[test]
    fn unix_archive_is_unchanged() {
        let unix = archive(UNIX_HOST, 0o100_644 << 16);
        let mut bytes = unix.clone();
        set_unix_host(&mut bytes).unwrap();
        assert_eq!(bytes, unix);
    }

    #[test]
    fn malformed_input_is_an_error_not_a_panic() {
        let good = archive(0, 0);
        for len in 0..good.len() {
            let mut cut = good[..len].to_vec();
            assert!(set_unix_host(&mut cut).is_err(), "length {len}");
        }
        let mut wrong = good.clone();
        let central = wrong.len() - EOCD_LEN - CENTRAL_LEN - 1;
        wrong[central] = 0;
        assert!(set_unix_host(&mut wrong).is_err());
    }
}
