//! Parse errors carry the byte position where parsing stopped and what was expected there
//! (spec 08 stage 6: a parse error lowers confidence instead of failing silently, FR-REC-08).

use dimo_notation::{ParseError, parse_callout};

fn error(text: &str) -> ParseError {
    match parse_callout(text) {
        Ok(callout) => panic!("{text:?} parsed as {callout:?}"),
        Err(error) => error,
    }
}

/// Byte offset of the first occurrence of `marker` in `text`.
fn at(text: &str, marker: &str) -> usize {
    text.find(marker)
        .unwrap_or_else(|| panic!("{marker:?} not in {text:?}"))
}

#[test]
fn error_positions_are_byte_offsets_with_expectation() {
    // (text, expected position, part of the expectation)
    let table: Vec<(&str, usize, &str)> = vec![
        ("", 0, "dimension value"),
        ("BREAK ALL SHARP EDGES", 0, "dimension value"),
        ("   note", 3, "dimension value"),
        // `Ø` is two bytes: the number is expected at byte 2.
        ("Ø", 2, "number"),
        ("Ø H7", 3, "number"),
        ("⌀", "⌀".len(), "number"),
        ("SX5", 1, "R or Ø after S"),
        ("50±", "50±".len(), "tolerance value after ±"),
        ("Ø30 H7 +0.0203", "Ø30 H7 +0.0203".len(), "lower deviation"),
        ("25 -0.05", "25 -0.05".len(), "lower deviation"),
        ("25 +0.1 5", at("25 +0.1 5", " 5"), "lower deviation"),
        ("25 +", "25 +".len(), "deviation value"),
        ("12.02/", "12.02/".len(), "second limit"),
        ("(42", 3, "`)`"),
        ("[42)", 3, "`]`"),
        ("(42 ±0.1", "(42 ±0.1".len(), "`)`"),
        ("12.5 abc", 5, "end of callout"),
        ("Ø8 H7 abc", at("Ø8 H7 abc", "abc"), "end of callout"),
        (
            "90.0° +0.0° −0.1° x",
            at("90.0° +0.0° −0.1° x", "x"),
            "end of callout",
        ),
        ("30°75'", "30°".len(), "below 60"),
        ("30.5°15'", 0, "whole degrees"),
        ("M8x", 3, "thread pitch"),
        ("M8x1.25-", "M8x1.25-".len(), "thread tolerance class"),
        ("M", 1, "thread size"),
        ("4X R5 2 PL", at("4X R5 2 PL", "2 PL"), "only one quantity"),
        ("0X R5", 0, "quantity"),
        ("1.5\" ±0.1mm", at("1.5\" ±0.1mm", "0.1mm"), "same unit"),
        ("99999999999999999999999999999999", 0, "at most 28 digits"),
    ];
    for (text, position, expected) in table {
        let e = error(text);
        assert_eq!(e.position, position, "position for {text:?}: {e}");
        assert!(
            e.expected.contains(expected),
            "expectation for {text:?}: got {:?}, want {expected:?}",
            e.expected
        );
        assert!(
            text.is_char_boundary(e.position),
            "{text:?}: not a char boundary"
        );
    }
}

#[test]
fn error_message_names_position_and_expectation() {
    let e = error("Ø30 H7 +0.0203");
    assert_eq!(
        e.to_string(),
        "cannot read callout at byte 15: expected a lower deviation"
    );
}
