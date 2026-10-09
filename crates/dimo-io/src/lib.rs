//! Project container, migrations, and imports and exports of inspection data.
//!
//! - [`export`]: characteristic list as CSV and XLSX (T1.4).

pub mod export;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_builds() {
        assert_eq!(env!("CARGO_PKG_NAME"), "dimo-io");
    }
}
