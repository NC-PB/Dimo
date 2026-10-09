//! Property tests of the callout parser (spec 08 stage 6, testing.md, FR-REC-08):
//! generated callouts print and parse back equal, and no input makes the parser panic.

#![allow(clippy::unwrap_used)] // Generators of a test crate; rust.md allows unwrap in tests.

use dimo_core::characteristic::Unit;
use dimo_notation::{
    Callout, Fit, Kind, Number, NumberForm, Suffix, Tolerance, ToleranceClass, parse_callout,
};
use proptest::prelude::*;
use rust_decimal::Decimal;

const CASES: u32 = 10_000;

fn plain() -> impl Strategy<Value = Number> {
    (0i64..10_000_000, 0u32..=4).prop_map(|(m, s)| Number::decimal(Decimal::new(m, s)))
}

fn no_leading_zero() -> impl Strategy<Value = Number> {
    (1u32..=4)
        .prop_flat_map(|s| (0i64..10i64.pow(s), Just(s)))
        .prop_map(|(m, s)| Number {
            value: Decimal::new(m, s),
            form: NumberForm::NoLeadingZero,
        })
}

fn fraction() -> impl Strategy<Value = Number> {
    (0u32..50, 1u32..=7)
        .prop_flat_map(|(whole, exp)| {
            let denominator = 1u32 << exp;
            (Just(whole), 1..denominator, Just(denominator))
        })
        .prop_map(|(w, n, d)| Number::fraction(w, n, d).unwrap())
}

fn dms() -> impl Strategy<Value = Number> {
    (0u32..360, 0u32..60, proptest::option::of(0u32..60))
        .prop_map(|(d, m, s)| Number::deg_min_sec(d, m, s).unwrap())
}

fn length() -> BoxedStrategy<Number> {
    prop_oneof![4 => plain(), 1 => no_leading_zero(), 1 => fraction()].boxed()
}

fn angle() -> BoxedStrategy<Number> {
    prop_oneof![3 => plain(), 1 => no_leading_zero(), 2 => dms()].boxed()
}

fn signed(number: impl Strategy<Value = Number>) -> impl Strategy<Value = Number> {
    (number, any::<bool>()).prop_map(|(mut n, negative)| {
        n.value.set_sign_negative(negative);
        n
    })
}

fn tolerance(number: BoxedStrategy<Number>) -> impl Strategy<Value = Option<Tolerance>> {
    proptest::option::of(prop_oneof![
        number.clone().prop_map(Tolerance::Symmetric),
        (signed(number.clone()), signed(number.clone()))
            .prop_map(|(upper, lower)| Tolerance::Deviations { upper, lower }),
        number.prop_map(|other| Tolerance::Limits { other }),
        Just(Tolerance::Min),
        Just(Tolerance::Max),
    ])
}

fn tolerance_class() -> impl Strategy<Value = ToleranceClass> {
    const LETTERS: &[&str] = &[
        "A", "B", "C", "CD", "D", "E", "EF", "F", "FG", "G", "H", "JS", "J", "K", "M", "N", "P",
        "R", "S", "T", "U", "V", "X", "Y", "Z", "ZA", "ZB", "ZC",
    ];
    (prop::sample::select(LETTERS), any::<bool>(), 1u8..=18).prop_map(|(letters, hole, grade)| {
        ToleranceClass {
            deviation: if hole {
                letters.to_owned()
            } else {
                letters.to_ascii_lowercase()
            },
            grade,
        }
    })
}

fn fit() -> impl Strategy<Value = Option<Fit>> {
    proptest::option::of(
        (tolerance_class(), proptest::option::of(tolerance_class()))
            .prop_map(|(first, second)| Fit { first, second }),
    )
}

fn length_unit() -> impl Strategy<Value = Option<Unit>> {
    prop_oneof![Just(None), Just(Some(Unit::Mm)), Just(Some(Unit::In))]
}

/// Kind, nominal, fit, tolerance and unit, consistent with each other.
type Body = (Kind, Number, Option<Fit>, Option<Tolerance>, Option<Unit>);

fn body() -> impl Strategy<Value = Body> {
    let fitting_kind = prop_oneof![
        Just(Kind::Linear),
        Just(Kind::Diameter),
        Just(Kind::Radius),
        Just(Kind::SphericalRadius),
        Just(Kind::SphericalDiameter),
        Just(Kind::Depth),
    ];
    let thread_class = prop::sample::select(&["6H", "6g", "4h", "5H6H", "6e"][..]);
    prop_oneof![
        4 => (fitting_kind, length(), fit(), tolerance(length()), length_unit()),
        2 => (angle(), tolerance(angle()))
            .prop_map(|(n, t)| (Kind::Angle, n, None, t, Some(Unit::Deg))),
        1 => (proptest::option::of(angle()), length(), tolerance(length()), length_unit())
            .prop_map(|(a, n, t, u)| (Kind::Chamfer { angle: a }, n, None, t, u)),
        1 => (plain(), proptest::option::of(plain()), proptest::option::of(thread_class))
            .prop_map(|(n, pitch, class)| {
                let kind = Kind::Thread { pitch, class: class.map(str::to_owned) };
                (kind, n, None, None, None)
            }),
    ]
}

fn suffix() -> impl Strategy<Value = Suffix> {
    prop_oneof![Just(Suffix::Thru), length().prop_map(Suffix::Depth)]
}

fn callout() -> impl Strategy<Value = Callout> {
    (
        body(),
        proptest::option::of(1u32..1000),
        prop::collection::vec(suffix(), 0..3),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |((kind, nominal, fit, tolerance, unit), quantity, suffixes, reference, basic)| {
                Callout {
                    quantity,
                    kind,
                    nominal,
                    fit,
                    tolerance,
                    suffixes,
                    reference,
                    basic,
                    unit,
                }
            },
        )
}

/// Characters that occur in callouts, so random strings reach deep into the grammar.
const ALPHABET: &[char] = &[
    '0', '1', '2', '4', '5', '9', '.', ',', '/', '+', '-', '\u{2212}', '±', 'Ø', '⌀', 'ø', 'S',
    'R', 'M', 'C', 'D', 'I', 'A', 'x', 'X', '×', '°', '\'', '"', '′', '″', '(', ')', '[', ']', 'H',
    'h', 'g', 'j', 's', 'T', 'U', 'P', 'L', 'N', 'E', 'F', 'm', 'i', 'n', '↧', ' ', '\t',
];

proptest! {
    #![proptest_config(ProptestConfig::with_cases(CASES))]

    /// Generated callouts print and parse back equal, and the print is stable.
    #[test]
    fn canonical_print_round_trips(c in callout()) {
        let text = c.to_canonical();
        let parsed = parse_callout(&text);
        prop_assert!(parsed.is_ok(), "{text:?}: {parsed:?}");
        let parsed = parsed.unwrap();
        prop_assert_eq!(&parsed, &c, "text {:?}", text);
        prop_assert_eq!(parsed.to_canonical(), text);
    }

    /// Arbitrary strings never panic; errors point to a char boundary inside the text.
    #[test]
    fn arbitrary_strings_never_panic(text in any::<String>()) {
        if let Err(e) = parse_callout(&text) {
            prop_assert!(e.position <= text.len());
            prop_assert!(text.is_char_boundary(e.position));
        }
    }

    /// Strings from the callout alphabet never panic.
    #[test]
    fn callout_like_strings_never_panic(
        chars in prop::collection::vec(prop::sample::select(ALPHABET), 0..24)
    ) {
        let text: String = chars.into_iter().collect();
        if let Err(e) = parse_callout(&text) {
            prop_assert!(text.is_char_boundary(e.position));
        }
    }

    /// Valid callouts with one character removed or inserted never panic.
    #[test]
    fn damaged_callouts_never_panic(
        c in callout(),
        cut in any::<prop::sample::Index>(),
        insert in prop::sample::select(ALPHABET),
        remove in any::<bool>(),
    ) {
        let mut chars: Vec<char> = c.to_canonical().chars().collect();
        let at = cut.index(chars.len() + 1);
        if remove && at < chars.len() {
            chars.remove(at);
        } else {
            chars.insert(at, insert);
        }
        let text: String = chars.into_iter().collect();
        if let Err(e) = parse_callout(&text) {
            prop_assert!(text.is_char_boundary(e.position));
        }
    }
}
