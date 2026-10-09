# dimo-tolerance

Tolerance engine: fit tables, general tolerance tables and limit derivation with exact decimal numbers. Data driven and pure.

The table values live in `data/tolerances/` (format and verification rules in its README).
Tests:

```sh
cargo test -p dimo-tolerance                                    # all, including the test vectors
DIMO_UPDATE_SCHEMA=1 cargo test -p dimo-tolerance --test schema # regenerate the table schema
```
