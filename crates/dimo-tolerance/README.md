# dimo-tolerance

Tolerance engine: fit tables, general tolerance tables and limit derivation with exact decimal numbers. Data driven and pure.

The table values live in `data/tolerances/` (format and verification rules in its README).
`interpret` turns a parsed callout (`dimo-notation`) into nominal, deviations, limits and a
structured `ToleranceDerivation` with the precedence of FR-TOL-01; `explain` renders a derivation
in English or German (FR-TOL-08).

Tests:

```sh
cargo test -p dimo-tolerance                                    # all, including the test vectors
cargo test -p dimo-tolerance --test interpret                   # precedence and conflict cases
cargo test -p dimo-tolerance --test corpus_truth                # limits of corpus/truth/*.truth.json
cargo insta review                                              # after explanation text changes
DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-tolerance --test schema # regenerate the table schema
```
