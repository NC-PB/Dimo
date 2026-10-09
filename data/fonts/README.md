# Bundled fonts

| File | Font | SHA-256 | License |
|---|---|---|---|
| `OpenSans-Bold.ttf` | Open Sans Bold, version 1.10, by Steve Matteson, copyright 2010-2011 Google Corporation | `a7a41b04969454dfbe620bfbc7699647b2819d768374b3f0f90a714a0d80b199` | Apache License 2.0 |

## OpenSans-Bold.ttf

Used for balloon numbers in exported PDFs (D-24 "bold sans number", T1.2). `dimo-pdf` compiles
the file in and embeds a subset with only the characters of the balloon texts into each
ballooned PDF, so numbers look the same in every PDF viewer.

- Origin: the TrueType file of the CTAN package `opensans` (version 2.2), copied unchanged
  from a local TeX Live 2023 installation (`fonts/truetype/ascender/opensans/`). The package
  states that its `.ttf` files are released under the Apache License 2.0; the font's own
  `name` table carries the same license URL (name ID 14).
- License: Apache License 2.0, the license of this repository (see `LICENSE`). No reserved
  font name applies. The attribution is listed in `NOTICE`.
- Why this font: permissive license identical to the project license, TrueType outlines
  (`glyf`, which the subsetter in `crates/dimo-pdf/src/font.rs` supports), tabular digits,
  small file (104 KB), all printable ASCII characters.

Replacing the font changes every ballooned PDF and the rendered snapshots in
`crates/dimo-pdf/tests/snapshots/`. Keep the file byte identical to its upstream release and
update the table above.
