# dimo-notation

Parser for dimension and tolerance callouts and feature control frames. Pure, parse errors are values with a position.

`parse_callout(&str) -> Result<Callout, ParseError>` reads one callout (winnow grammar, spec 08
stage 6). `Callout::to_canonical()` prints it back; the property tests check that generated
callouts round trip.

Stacked text is joined by the caller with one space in reading order: main line, upper line,
lower line. Examples: `Ø30 H7 +0.0203 -0`, `90.0° +0.0° −0.1°`, `12.02 11.98`.

The callout records what is written (kind, quantity, nominal, fit, tolerance, suffixes,
reference and basic markers, unit). Limits come from `dimo-tolerance`.
