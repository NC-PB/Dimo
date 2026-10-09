//! Callout grammar with winnow (spec 08 stage 6, FR-REC-08).
//!
//! The parser works on the original text, so error positions are byte offsets into it.
//! Small token parsers backtrack; once a token commits to a reading (a kind prefix, `±`, a
//! signed deviation after a space, an opening parenthesis), a missing part is a cut error at
//! the exact position, with what was expected there.
//!
//! Grammar, informally (whitespace is allowed between all parts):
//!
//! ```text
//! callout    = [quantity] [ "(" | "[" ] [quantity] core [fit] [tolerance] [ ")" | "]" ]
//!              { suffix } end
//! quantity   = count ("X" | "x" | "×" | "PL" | "PLS" | "PLCS" | "PLACES")
//! core       = ("Ø" | "⌀" | "ø" | "DIA") length      diameter
//!            | "SR" length | ("SØ" | "S⌀") length    spherical radius and diameter
//!            | "R" length | "↧" length | "C" length  radius, depth, chamfer at 45 degrees
//!            | "M" number ["x" pitch] ["-" class]    metric thread
//!            | length "x" angle                      chamfer
//!            | angle                                 needs ° or minutes
//!            | length                                linear
//! tolerance  = "±" value | "MIN" | "MAX" | signed [ "/" ] (signed | zero)
//!            | "/" value | "-" value (no space before) | value (space before)
//! suffix     = "THRU" | "↧" length | "REF" | count "PL"
//! ```
//!
//! Length values accept decimal point or comma, an optional leading zero, inch fractions
//! (`1/4`, `1 1/4`, denominators 2 to 128) and a unit marker (`"`, `in`, `mm`). Angle values
//! accept decimal degrees (`90.0°`) and degrees, minutes, seconds (`30°15'20"`). Signs are
//! `+`, the hyphen minus and the Unicode minus U+2212.

use dimo_core::characteristic::Unit;
use rust_decimal::Decimal;
use winnow::ascii::{Caseless, digit1};
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::token::{literal, take_while};

use crate::callout::{
    Callout, Fit, Kind, Number, NumberForm, Suffix, Tolerance, ToleranceClass, is_inch_fraction,
};
use crate::error::ParseError;

/// Parse one callout.
///
/// Stacked text (deviations or limits on two lines) is joined by the caller with one space in
/// reading order: main line, then the upper line, then the lower line, e.g.
/// `Ø30 H7 +0.0203 -0` or `12.02 11.98`. This is the `requirement_text` convention of the
/// corpus truth files.
///
/// Never panics. Free text such as a note returns an error at position 0.
pub fn parse_callout(text: &str) -> Result<Callout, ParseError> {
    callout.parse(text).map_err(|error| ParseError {
        position: error.offset(),
        expected: expected_of(error.inner()),
    })
}

fn expected_of(error: &ContextError) -> String {
    error
        .context()
        .find_map(|context| match context {
            StrContext::Expected(value) => Some(value.to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "a callout".to_owned())
}

// ---------------------------------------------------------------------------------------------
// Error and token helpers

/// A cut error at `at`, which becomes the reported position.
fn cut_at<'a>(input: &mut &'a str, at: &'a str, expected: &'static str) -> ErrMode<ContextError> {
    *input = at;
    let mut error = ContextError::new();
    error.push(StrContext::Expected(StrContextValue::Description(expected)));
    ErrMode::Cut(error)
}

fn backtrack() -> ErrMode<ContextError> {
    ErrMode::Backtrack(ContextError::new())
}

/// Run `parse`; a backtrack becomes a cut error at the start position.
fn required<'a, T>(
    input: &mut &'a str,
    expected: &'static str,
    parse: impl FnOnce(&mut &'a str) -> ModalResult<T>,
) -> ModalResult<T> {
    let at = *input;
    match parse(input) {
        Err(ErrMode::Backtrack(_)) => Err(cut_at(input, at, expected)),
        other => other,
    }
}

/// Skip whitespace (spaces, tabs, line breaks, no-break spaces). Returns the bytes skipped.
fn ws0(input: &mut &str) -> ModalResult<usize> {
    take_while(0.., char::is_whitespace)
        .map(str::len)
        .parse_next(input)
}

/// Consume `token` if the input starts with it.
fn eat(input: &mut &str, token: &'static str) -> bool {
    let result: ModalResult<&str> = literal(token).parse_next(input);
    result.is_ok()
}

/// Consume the first of `tokens` the input starts with.
fn eat_any(input: &mut &str, tokens: &[&'static str]) -> Option<&'static str> {
    tokens.iter().copied().find(|token| eat(input, token))
}

/// Consume a whole word, ignoring ASCII case. It must not be followed by a letter or digit.
fn word(input: &mut &str, w: &'static str) -> bool {
    let start = *input;
    let result: ModalResult<&str> = literal(Caseless(w)).parse_next(input);
    if result.is_err() {
        return false;
    }
    if input.chars().next().is_some_and(char::is_alphanumeric) {
        *input = start;
        return false;
    }
    true
}

fn digits<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    digit1.parse_next(input)
}

const DIAMETER_SIGNS: &[&str] = &["Ø", "⌀", "ø"];
const TIMES_SIGNS: &[&str] = &["x", "X", "×"];
const MINUS_SIGNS: &[&str] = &["-", "\u{2212}"];
const PRIME_SIGNS: &[&str] = &["'", "\u{2032}"];
const DOUBLE_PRIME_SIGNS: &[&str] = &["\"", "\u{2033}"];
const PLACES_WORDS: &[&str] = &["PLACES", "PLCS", "PLS", "PL"];

fn places_word(input: &mut &str) -> bool {
    PLACES_WORDS.iter().any(|w| word(input, w))
}

/// A sign: `Some(false)` for `+`, `Some(true)` for a minus.
fn sign(input: &mut &str) -> Option<bool> {
    if eat(input, "+") {
        Some(false)
    } else if eat_any(input, MINUS_SIGNS).is_some() {
        Some(true)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------------------------
// Numbers

/// Digits of a decimal number: `12`, `12.5`, `12,5`, `.5`.
struct RawNumber<'a> {
    int: &'a str,
    frac: Option<&'a str>,
}

fn raw_number<'a>(input: &mut &'a str) -> ModalResult<RawNumber<'a>> {
    let start = *input;
    let int: &str = take_while(0.., |c: char| c.is_ascii_digit()).parse_next(input)?;
    let mut frac = None;
    let before_separator = *input;
    if eat_any(input, &[".", ","]).is_some() {
        match digits(input) {
            Ok(f) => frac = Some(f),
            Err(_) => *input = before_separator,
        }
    }
    if int.is_empty() && frac.is_none() {
        *input = start;
        return Err(backtrack());
    }
    Ok(RawNumber { int, frac })
}

/// The decimal value of the digits; a cut error at `at` when it does not fit a `Decimal`.
fn raw_decimal<'a>(input: &mut &'a str, at: &'a str, raw: &RawNumber<'_>) -> ModalResult<Number> {
    let int = if raw.int.is_empty() { "0" } else { raw.int };
    let text = match raw.frac {
        Some(frac) => format!("{int}.{frac}"),
        None => int.to_owned(),
    };
    let Ok(value) = Decimal::from_str_exact(&text) else {
        return Err(cut_at(input, at, "a number with at most 28 digits"));
    };
    let form = if raw.int.is_empty() {
        NumberForm::NoLeadingZero
    } else {
        NumberForm::Decimal
    };
    Ok(Number { value, form })
}

/// A decimal number without fraction or unit marker (thread sizes and pitches).
fn decimal_number(input: &mut &str) -> ModalResult<Number> {
    let start = *input;
    let raw = raw_number(input)?;
    raw_decimal(input, start, &raw)
}

/// `n/d` with a power of two denominator (inch fraction). Resets and returns `None` otherwise.
fn fraction_part(input: &mut &str) -> Option<(u32, u32)> {
    let start = *input;
    let parsed: ModalResult<(&str, &str, &str)> = (digit1, "/", digit1).parse_next(input);
    if let Ok((n, _, d)) = parsed
        && let (Ok(n), Ok(d)) = (n.parse::<u32>(), d.parse::<u32>())
        && is_inch_fraction(n, d)
    {
        return Some((n, d));
    }
    *input = start;
    None
}

/// A unit marker after a length value: `"`, `in`, `inch`, `mm`.
fn unit_marker(input: &mut &str) -> Option<Unit> {
    let start = *input;
    if eat(input, "\"") {
        return Some(Unit::In);
    }
    let _ = ws0(input);
    if word(input, "inch") || word(input, "in") {
        return Some(Unit::In);
    }
    if word(input, "mm") {
        return Some(Unit::Mm);
    }
    *input = start;
    None
}

/// A length value: decimal, inch fraction or mixed number, with an optional unit marker.
fn length_value(input: &mut &str) -> ModalResult<(Number, Option<Unit>)> {
    let start = *input;
    let raw = raw_number(input)?;
    let mut number = None;
    if raw.frac.is_none() {
        // `1/4`: the digits just read are the numerator.
        let after_int = *input;
        *input = start;
        if let Some((n, d)) = fraction_part(input) {
            number = Number::fraction(0, n, d);
        } else {
            *input = after_int;
            // `1 1/4`: a whole number, a space and a fraction.
            if let Ok(whole) = raw.int.parse::<u32>() {
                let before_space = *input;
                if ws0(input)? > 0
                    && let Some((n, d)) = fraction_part(input)
                {
                    number = Number::fraction(whole, n, d);
                } else {
                    *input = before_space;
                }
            }
        }
    }
    let number = match number {
        Some(number) => number,
        None => raw_decimal(input, start, &raw)?,
    };
    Ok((number, unit_marker(input)))
}

/// How an angle value was marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AngleMark {
    /// No mark: a plain number (allowed for angular tolerances).
    None,
    /// Starts with degrees: `30°`, `30°15'`.
    Degrees,
    /// Minutes first: `30'`, `30'15"`.
    Minutes,
    /// Seconds only: `30"`.
    Seconds,
}

/// Whole minutes or seconds followed by one of `marks`. Resets and returns `None` otherwise.
fn sub_unit(input: &mut &str, marks: &[&'static str]) -> ModalResult<Option<u32>> {
    let start = *input;
    let _ = ws0(input)?;
    let Ok(text) = digits(input) else {
        *input = start;
        return Ok(None);
    };
    if eat_any(input, marks).is_none() {
        *input = start;
        return Ok(None);
    }
    match text.parse::<u32>() {
        Ok(value) if value < 60 => Ok(Some(value)),
        _ => Err(cut_at(input, start, "minutes and seconds below 60")),
    }
}

fn whole_degrees<'a>(input: &mut &'a str, at: &'a str, raw: &RawNumber<'_>) -> ModalResult<u32> {
    if raw.frac.is_some() || raw.int.is_empty() {
        return Err(cut_at(input, at, "whole degrees before minutes"));
    }
    raw.int
        .parse::<u32>()
        .map_err(|_| cut_at(input, at, "a number with at most 9 digits"))
}

fn dms<'a>(
    input: &mut &'a str,
    at: &'a str,
    degrees: u32,
    minutes: u32,
    seconds: Option<u32>,
) -> ModalResult<Number> {
    Number::deg_min_sec(degrees, minutes, seconds)
        .ok_or_else(|| cut_at(input, at, "an angle within range"))
}

/// An angle value: `90.0°`, `30°15'`, `30°15'20"`, `30'`, `20"` or a plain number.
fn angle_value(input: &mut &str) -> ModalResult<(Number, AngleMark)> {
    let start = *input;
    let raw = raw_number(input)?;
    if eat(input, "°") {
        let after_degrees = *input;
        if let Some(minutes) = sub_unit(input, PRIME_SIGNS)? {
            let degrees = whole_degrees(input, start, &raw)?;
            let seconds = sub_unit(input, DOUBLE_PRIME_SIGNS)?;
            return Ok((
                dms(input, start, degrees, minutes, seconds)?,
                AngleMark::Degrees,
            ));
        }
        *input = after_degrees;
        return Ok((raw_decimal(input, start, &raw)?, AngleMark::Degrees));
    }
    let after_number = *input;
    if eat_any(input, PRIME_SIGNS).is_some() {
        let minutes = whole_degrees(input, start, &raw)?;
        if minutes >= 60 {
            return Err(cut_at(input, start, "minutes and seconds below 60"));
        }
        let seconds = sub_unit(input, DOUBLE_PRIME_SIGNS)?;
        return Ok((dms(input, start, 0, minutes, seconds)?, AngleMark::Minutes));
    }
    // Seconds only. Anything else with `"` (such as `12.5"`) is left for the inch reading.
    if raw.frac.is_none()
        && let Ok(seconds) = raw.int.parse::<u32>()
        && seconds < 60
        && eat_any(input, DOUBLE_PRIME_SIGNS).is_some()
    {
        return Ok((dms(input, start, 0, 0, Some(seconds))?, AngleMark::Seconds));
    }
    *input = after_number;
    Ok((raw_decimal(input, start, &raw)?, AngleMark::None))
}

/// Whether values are lengths or angles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Length,
    Angle,
}

/// Record a unit marker. A second, different marker is an error at `at`.
fn merge_unit<'a>(
    input: &mut &'a str,
    at: &'a str,
    unit: &mut Option<Unit>,
    marker: Option<Unit>,
) -> ModalResult<()> {
    match (marker, *unit) {
        (None, _) => Ok(()),
        (Some(m), None) => {
            *unit = Some(m);
            Ok(())
        }
        (Some(m), Some(u)) if m == u => Ok(()),
        (Some(_), Some(_)) => Err(cut_at(input, at, "the same unit for all values")),
    }
}

/// An unsigned value in the given mode.
fn value(input: &mut &str, mode: Mode, unit: &mut Option<Unit>) -> ModalResult<Number> {
    let start = *input;
    match mode {
        Mode::Length => {
            let (number, marker) = length_value(input)?;
            merge_unit(input, start, unit, marker)?;
            Ok(number)
        }
        Mode::Angle => angle_value(input).map(|(number, _)| number),
    }
}

fn with_sign(mut number: Number, negative: bool) -> Number {
    number.value.set_sign_negative(negative);
    number
}

// ---------------------------------------------------------------------------------------------
// Fits and tolerances

/// ISO 286 fundamental deviations of holes. Shafts use the same letters in lower case.
const DEVIATION_LETTERS: &[&str] = &[
    "A", "B", "C", "CD", "D", "E", "EF", "F", "FG", "G", "H", "JS", "J", "K", "M", "N", "P", "R",
    "S", "T", "U", "V", "X", "Y", "Z", "ZA", "ZB", "ZC",
];

fn is_deviation(letters: &str) -> bool {
    let upper = letters.chars().all(|c| c.is_ascii_uppercase());
    let lower = letters.chars().all(|c| c.is_ascii_lowercase());
    (upper || lower)
        && DEVIATION_LETTERS
            .iter()
            .any(|known| known.eq_ignore_ascii_case(letters))
}

/// One tolerance class such as `H7`, `js6`, `c10`.
fn tolerance_class(input: &mut &str) -> ModalResult<ToleranceClass> {
    let start = *input;
    let parsed: ModalResult<(&str, &str)> = (
        take_while(1..=2, |c: char| c.is_ascii_alphabetic()),
        take_while(1..=2, |c: char| c.is_ascii_digit()),
    )
        .parse_next(input);
    if let Ok((letters, grade)) = parsed
        && is_deviation(letters)
        && let Ok(grade) = grade.parse::<u8>()
        && (1..=18).contains(&grade)
        && !input.chars().next().is_some_and(char::is_alphanumeric)
    {
        return Ok(ToleranceClass {
            deviation: letters.to_owned(),
            grade,
        });
    }
    *input = start;
    Err(backtrack())
}

/// An optional fit: `H7`, `g6`, `H7/g6`.
fn fit(input: &mut &str) -> ModalResult<Option<Fit>> {
    let start = *input;
    let _ = ws0(input)?;
    let Ok(first) = tolerance_class(input) else {
        *input = start;
        return Ok(None);
    };
    let before_slash = *input;
    let mut second = None;
    if eat(input, "/") {
        match tolerance_class(input) {
            Ok(class) => second = Some(class),
            Err(_) => *input = before_slash,
        }
    }
    Ok(Some(Fit { first, second }))
}

/// True if a trailing quantity (`2 PL`) follows. Consumes nothing.
fn places_follow(input: &mut &str) -> bool {
    let start = *input;
    let found =
        ws0(input).is_ok() && digits(input).is_ok() && ws0(input).is_ok() && places_word(input);
    *input = start;
    found
}

/// The second deviation: signed, or an unsigned zero (`+0.1 0`), optionally after `/`.
fn lower_deviation(input: &mut &str, mode: Mode, unit: &mut Option<Unit>) -> ModalResult<Number> {
    let start = *input;
    let saved_unit = *unit;
    let _ = ws0(input)?;
    if eat(input, "/") {
        let _ = ws0(input)?;
    }
    if let Some(negative) = sign(input) {
        let number = required(input, "a deviation value", |i| value(i, mode, unit))?;
        return Ok(with_sign(number, negative));
    }
    match value(input, mode, unit) {
        Ok(number) if number.value.is_zero() => Ok(number),
        Ok(_) | Err(ErrMode::Backtrack(_)) => {
            *input = start;
            *unit = saved_unit;
            Err(backtrack())
        }
        Err(error) => Err(error),
    }
}

/// An optional tolerance after the nominal (and fit).
fn tolerance(
    input: &mut &str,
    mode: Mode,
    unit: &mut Option<Unit>,
) -> ModalResult<Option<Tolerance>> {
    let start = *input;
    let had_space = ws0(input)? > 0;

    if eat_any(input, &["±", "+/-", "+-"]).is_some() {
        let _ = ws0(input)?;
        let number = required(input, "a tolerance value after ±", |i| {
            value(i, mode, unit)
        })?;
        return Ok(Some(Tolerance::Symmetric(number)));
    }
    if word(input, "MIN") {
        return Ok(Some(Tolerance::Min));
    }
    if word(input, "MAX") {
        return Ok(Some(Tolerance::Max));
    }

    if let Some(negative) = sign(input) {
        if !negative || had_space {
            // `+0.1 -0.05`, `+0 -0.6`, ` -0.0127 -0.0279`: two deviations.
            let upper = required(input, "a deviation value", |i| value(i, mode, unit))?;
            let lower = required(input, "a lower deviation", |i| {
                lower_deviation(i, mode, unit)
            })?;
            return Ok(Some(Tolerance::Deviations {
                upper: with_sign(upper, negative),
                lower,
            }));
        }
        // A minus right after the nominal: two deviations (`f7-0.0127-0.0279`) or a limit
        // dimension written on one line (`12.02-11.98`).
        let number = required(input, "a number after the minus", |i| value(i, mode, unit))?;
        return match lower_deviation(input, mode, unit) {
            Ok(lower) => Ok(Some(Tolerance::Deviations {
                upper: with_sign(number, true),
                lower,
            })),
            Err(ErrMode::Backtrack(_)) => Ok(Some(Tolerance::Limits { other: number })),
            Err(error) => Err(error),
        };
    }

    if eat(input, "/") {
        let _ = ws0(input)?;
        let other = required(input, "a second limit", |i| value(i, mode, unit))?;
        return Ok(Some(Tolerance::Limits { other }));
    }

    if had_space && !places_follow(input) {
        let before = *input;
        let saved_unit = *unit;
        match value(input, mode, unit) {
            Ok(number) => {
                // `+0 -0.6` written with an unsigned upper zero: `0 -0.6`.
                if number.value.is_zero() {
                    let before_lower = *input;
                    let _ = ws0(input)?;
                    if let Some(negative) = sign(input) {
                        let lower = required(input, "a deviation value", |i| value(i, mode, unit))?;
                        return Ok(Some(Tolerance::Deviations {
                            upper: number,
                            lower: with_sign(lower, negative),
                        }));
                    }
                    *input = before_lower;
                }
                // Two stacked limits joined with a space: `12.02 11.98`.
                return Ok(Some(Tolerance::Limits { other: number }));
            }
            Err(ErrMode::Backtrack(_)) => {
                *input = before;
                *unit = saved_unit;
            }
            Err(error) => return Err(error),
        }
    }

    *input = start;
    Ok(None)
}

// ---------------------------------------------------------------------------------------------
// Callout structure

/// A leading quantity: `4X`, `4x`, `4 ×`, `2 PL`. Resets and returns `None` if there is none.
fn quantity_prefix(input: &mut &str) -> ModalResult<Option<u32>> {
    let start = *input;
    let Ok(count) = digits(input) else {
        return Ok(None);
    };
    let _ = ws0(input)?;
    let is_quantity = if eat_any(input, TIMES_SIGNS).is_some() {
        // `4X Ø6` or `4XØ6`, but not `4x10` (no space before a digit) and not the chamfer
        // `1x45°` or `1 x 45°`.
        let next = input.chars().next();
        let boundary = next
            .is_some_and(|c| c.is_whitespace() || !(c.is_ascii_digit() || c == '.' || c == ','));
        boundary && !angle_follows(input)
    } else {
        places_word(input)
    };
    if !is_quantity {
        *input = start;
        return Ok(None);
    }
    match count.parse::<u32>() {
        Ok(n) if n > 0 => Ok(Some(n)),
        _ => Err(cut_at(input, start, "a quantity from 1 to 4294967295")),
    }
}

/// True if a number with a degree sign follows. Consumes nothing.
fn angle_follows(input: &mut &str) -> bool {
    let start = *input;
    let found = ws0(input).is_ok() && raw_number(input).is_ok() && eat(input, "°");
    *input = start;
    found
}

/// A required length value, recording its unit marker.
fn length(input: &mut &str, unit: &mut Option<Unit>) -> ModalResult<Number> {
    let _ = ws0(input)?;
    required(input, "a number", |i| value(i, Mode::Length, unit))
}

/// Thread tolerance class: `6H`, `6g`, `5H6H`.
fn thread_class(input: &mut &str) -> ModalResult<String> {
    let start = *input;
    let mut class = String::new();
    for _ in 0..2 {
        let parsed: ModalResult<(char, char)> = (
            winnow::token::one_of(|c: char| c.is_ascii_digit()),
            winnow::token::one_of(|c: char| matches!(c, 'a'..='h' | 'A'..='H')),
        )
            .parse_next(input);
        match parsed {
            Ok((grade, position)) => {
                class.push(grade);
                class.push(position);
            }
            Err(_) => break,
        }
    }
    if class.is_empty() || input.chars().next().is_some_and(char::is_alphanumeric) {
        *input = start;
        return Err(backtrack());
    }
    Ok(class)
}

/// `M8`, `M8x1.25`, `M8x1.25-6H` after the `M`.
fn thread(input: &mut &str) -> ModalResult<(Kind, Number)> {
    let _ = ws0(input)?;
    let nominal = required(input, "a thread size", decimal_number)?;
    let before_pitch = *input;
    let _ = ws0(input)?;
    let pitch = if eat_any(input, TIMES_SIGNS).is_some() {
        let _ = ws0(input)?;
        Some(required(input, "a thread pitch", decimal_number)?)
    } else {
        *input = before_pitch;
        None
    };
    let before_class = *input;
    let _ = ws0(input)?;
    let class = if eat_any(input, MINUS_SIGNS).is_some() {
        let _ = ws0(input)?;
        Some(required(
            input,
            "a thread tolerance class such as 6H",
            thread_class,
        )?)
    } else {
        *input = before_class;
        None
    };
    Ok((Kind::Thread { pitch, class }, nominal))
}

/// A callout that starts with a number: chamfer `1x45°`, angle or linear.
fn numeric_core(input: &mut &str, unit: &mut Option<Unit>) -> ModalResult<(Kind, Number)> {
    let start = *input;
    let saved_unit = *unit;

    // Chamfer: length, `x`, angle with a degree sign.
    if let Ok((leg, marker)) = length_value(input) {
        let _ = ws0(input)?;
        if eat_any(input, TIMES_SIGNS).is_some() {
            let _ = ws0(input)?;
            let before_angle = *input;
            if let Ok((angle, AngleMark::Degrees)) = angle_value(input) {
                merge_unit(input, start, unit, marker)?;
                return Ok((Kind::Chamfer { angle: Some(angle) }, leg));
            }
            *input = before_angle;
        }
    }
    *input = start;
    *unit = saved_unit;

    // Angle: needs a degree sign or minutes. A bare `30"` is a length in inch.
    match angle_value(input) {
        Ok((angle, AngleMark::Degrees | AngleMark::Minutes)) => {
            *unit = Some(Unit::Deg);
            return Ok((Kind::Angle, angle));
        }
        Ok(_) | Err(ErrMode::Backtrack(_)) => {}
        Err(error) => return Err(error),
    }
    *input = start;

    let nominal = value(input, Mode::Length, unit)?;
    Ok((Kind::Linear, nominal))
}

/// The kind prefix and the nominal.
fn core(input: &mut &str, unit: &mut Option<Unit>) -> ModalResult<(Kind, Number)> {
    if eat(input, "S") {
        if eat(input, "R") {
            return Ok((Kind::SphericalRadius, length(input, unit)?));
        }
        if eat_any(input, DIAMETER_SIGNS).is_some() {
            return Ok((Kind::SphericalDiameter, length(input, unit)?));
        }
        let at = *input;
        return Err(cut_at(input, at, "R or Ø after S"));
    }
    if eat_any(input, DIAMETER_SIGNS).is_some() {
        return Ok((Kind::Diameter, length(input, unit)?));
    }
    let before_dia = *input;
    let dia: ModalResult<&str> = literal(Caseless("DIA")).parse_next(input);
    if dia.is_ok() {
        let _ = ws0(input)?;
        if input.starts_with(|c: char| c.is_ascii_digit() || c == '.' || c == ',') {
            return Ok((Kind::Diameter, length(input, unit)?));
        }
        *input = before_dia;
    }
    if eat(input, "R") {
        return Ok((Kind::Radius, length(input, unit)?));
    }
    if eat(input, "M") {
        return thread(input);
    }
    if eat(input, "C") {
        return Ok((Kind::Chamfer { angle: None }, length(input, unit)?));
    }
    if eat(input, "↧") {
        return Ok((Kind::Depth, length(input, unit)?));
    }
    numeric_core(input, unit)
}

/// The whole callout.
fn callout(input: &mut &str) -> ModalResult<Callout> {
    let _ = ws0(input)?;
    let mut quantity = quantity_prefix(input)?;
    let _ = ws0(input)?;
    let marker = eat_any(input, &["(", "["]);
    if marker.is_some() {
        let _ = ws0(input)?;
        if quantity.is_none() {
            quantity = quantity_prefix(input)?;
            let _ = ws0(input)?;
        }
    }

    let mut unit = None;
    let (kind, nominal) = required(input, "a dimension value", |i| core(i, &mut unit))?;
    let fit = if kind.takes_fit() { fit(input)? } else { None };
    let tolerance = match kind {
        Kind::Thread { .. } => None,
        Kind::Angle => tolerance(input, Mode::Angle, &mut unit)?,
        _ => tolerance(input, Mode::Length, &mut unit)?,
    };

    let mut reference = marker == Some("(");
    let basic = marker == Some("[");
    if let Some(open) = marker {
        let close = if open == "(" { ")" } else { "]" };
        let _ = ws0(input)?;
        let at = *input;
        if !eat(input, close) {
            return Err(cut_at(input, at, if open == "(" { "`)`" } else { "`]`" }));
        }
    }

    let mut suffixes = Vec::new();
    loop {
        let before = *input;
        let _ = ws0(input)?;
        if word(input, "THRU") {
            suffixes.push(Suffix::Thru);
        } else if eat(input, "↧") {
            let depth = length(input, &mut unit)?;
            suffixes.push(Suffix::Depth(depth));
        } else if word(input, "REF") {
            reference = true;
        } else if let Ok(count) = digits(input)
            && ws0(input).is_ok()
            && places_word(input)
        {
            if quantity.is_some() {
                let at = before.trim_start();
                return Err(cut_at(input, at, "only one quantity"));
            }
            match count.parse::<u32>() {
                Ok(n) if n > 0 => quantity = Some(n),
                _ => {
                    let at = before.trim_start();
                    return Err(cut_at(input, at, "a quantity from 1 to 4294967295"));
                }
            }
        } else {
            *input = before;
            break;
        }
    }

    let _ = ws0(input)?;
    if !input.is_empty() {
        let at = *input;
        return Err(cut_at(input, at, "end of callout"));
    }

    Ok(Callout {
        quantity,
        kind,
        nominal,
        fit,
        tolerance,
        suffixes,
        reference,
        basic,
        unit,
    })
}
