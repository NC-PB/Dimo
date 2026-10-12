# Bundled fonts

| File | Font | SHA-256 | License |
|---|---|---|---|
| `OpenSans-Bold.ttf` | Open Sans Bold, version 1.10, by Steve Matteson, copyright 2010-2011 Google Corporation | `a7a41b04969454dfbe620bfbc7699647b2819d768374b3f0f90a714a0d80b199` | Apache License 2.0 |
| `OpenSans-Regular.ttf` | Open Sans Regular, version 1.10, by Steve Matteson, copyright 2010-2011 Google Corporation | `34ad67cfc362403e3baabe4ad0f4ef0b4b6b68e2f252dd703bbb1e10198188e2` | Apache License 2.0 |

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

## OpenSans-Regular.ttf

Body text of the PDF reports (D-30, M3 decision 11), next to the Bold for headings. Added in T3.2.

- Origin: the same CTAN package and TeX Live 2023 installation as the Bold
  (`fonts/truetype/ascender/opensans/OpenSans-Regular.ttf`), copied unchanged. Same version
  (1.10 in the `name` table), same license statement (name ID 14: Apache License 2.0).
- Glyph coverage checked for reports: `Ø ± ° ′ ″ ×`, U+2212 minus and `ä ö ü Ä Ö Ü ß` are
  present in both files. `⌀` (U+2300) and check mark, cross and circle symbols are missing:
  templates map `⌀` to `Ø` and draw verdict marks as shapes.
