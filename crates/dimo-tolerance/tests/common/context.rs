//! Tolerance contexts for the engine tests.
#![allow(dead_code, reason = "each test crate uses a different subset")]
#![allow(clippy::unwrap_used)] // Helpers of a test crate; rust.md allows unwrap in tests.

use dimo_core::characteristic::Unit;
use dimo_core::derivation::TableRef;
use dimo_core::project::{
    CustomTable, DecimalPlaceRule, TableClass, ToleranceSettings, UnitRounding,
};
use dimo_core::sheet::Sha256Hex;
use dimo_tolerance::{TableSet, ToleranceContext};
use rust_decimal::Decimal;

/// A custom table of a shop: linear values up to 100 mm only, class `a`. Draft, like every
/// table written by the agent (D-43).
pub const SHOP_TABLE: &str = r#"
[table]
id = "shop"
title = "Shop table"
kind = "custom"
unit = "mm"
version = 2
status = "draft"
source = "test fixture"

[[part]]
id = "linear"
source = "fixture"
kind = "symmetric"
applies_to = "linear"
value_unit = "mm"
columns = ["a"]
rows = [
  { min_inclusive = "0.5", max_inclusive = "50", values = ["0.05"] },
  { min_exclusive = "50", max_inclusive = "100", values = ["0.1"] },
]
"#;

/// Settings variants of the precedence tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cfg {
    /// New project: no general tolerance, no rules.
    None,
    /// ISO 2768-1 class m.
    General,
    /// Decimal rules only: 1 place ±0.1, 2 places ±0.05.
    Decimal,
    /// ISO 2768-1 class m and the decimal rules.
    GeneralDecimal,
    /// Drawing rule `shop` class a, ISO 2768-1 m and the decimal rules.
    DrawingRule,
    /// The custom table `shop` class a as general tolerance.
    CustomGeneral,
}

pub fn shop_entry() -> CustomTable {
    CustomTable {
        table: TableRef {
            id: "shop".into(),
            version: 2,
        },
        sha256: Sha256Hex::parse(&"a".repeat(64)).unwrap(),
    }
}

pub fn iso_2768_m() -> TableClass {
    TableClass {
        table: TableRef {
            id: "iso-2768-1".into(),
            version: 1,
        },
        class: "m".into(),
    }
}

pub fn shop_a() -> TableClass {
    TableClass {
        table: shop_entry().table,
        class: "a".into(),
    }
}

pub fn decimal_rules() -> Vec<DecimalPlaceRule> {
    vec![
        DecimalPlaceRule {
            places: 1,
            tolerance: Decimal::new(1, 1),
        },
        DecimalPlaceRule {
            places: 2,
            tolerance: Decimal::new(5, 2),
        },
    ]
}

pub fn settings(cfg: Cfg) -> ToleranceSettings {
    let mut s = ToleranceSettings {
        unit_rounding: UnitRounding::default(),
        ..ToleranceSettings::default()
    };
    match cfg {
        Cfg::None => {}
        Cfg::General => s.general = Some(iso_2768_m()),
        Cfg::Decimal => s.decimal_rules = decimal_rules(),
        Cfg::GeneralDecimal => {
            s.general = Some(iso_2768_m());
            s.decimal_rules = decimal_rules();
        }
        Cfg::DrawingRule => {
            s.general = Some(iso_2768_m());
            s.decimal_rules = decimal_rules();
            s.custom_tables = vec![shop_entry()];
            s.drawing_rule = Some(shop_a());
        }
        Cfg::CustomGeneral => {
            s.custom_tables = vec![shop_entry()];
            s.general = Some(shop_a());
        }
    }
    s
}

/// A context with the shipped tables and, where the settings need it, the shop table.
pub fn context(cfg: Cfg, drawing_unit: Unit, output_unit: Option<Unit>) -> ToleranceContext {
    ToleranceContext::for_project(
        &settings(cfg),
        TableSet::shipped().unwrap(),
        [("shop", SHOP_TABLE)],
    )
    .unwrap()
    .with_drawing_unit(drawing_unit)
    .with_output_unit(output_unit)
}
