# Bundled fonts

| File | Font | SHA-256 | License |
|---|---|---|---|
| `OpenSans-Bold.ttf` | Open Sans Bold, version 1.10, by Steve Matteson, copyright 2010-2011 Google Corporation | `a7a41b04969454dfbe620bfbc7699647b2819d768374b3f0f90a714a0d80b199` | Apache License 2.0 |
| `OpenSans-Regular.ttf` | Open Sans Regular, version 1.10, by Steve Matteson, copyright 2010-2011 Google Corporation | `34ad67cfc362403e3baabe4ad0f4ef0b4b6b68e2f252dd703bbb1e10198188e2` | Apache License 2.0 |
| `SpaceGrotesk-Medium.woff2` | Space Grotesk Medium (500), version 2.000, copyright 2020 The Space Grotesk Project Authors | `91d49699a2bfc8e33e7ec5c290243eca1b3f31d4836d0330558ed104ef8bd4fa` | SIL Open Font License 1.1, `SpaceGrotesk-OFL.txt` |
| `SpaceGrotesk-Bold.woff2` | Space Grotesk Bold (700), version 2.000, copyright 2020 The Space Grotesk Project Authors | `c3eb1afbc31643c6b8eaec6169baa36510bfb9999605ddebcd81af7d0c1bdea5` | SIL Open Font License 1.1, `SpaceGrotesk-OFL.txt` |
| `JetBrainsMono-latin.woff2` | JetBrains Mono, variable weight 100 to 800, upright, Latin subset, version 2.211, copyright 2020 The JetBrains Mono Project Authors | `18be452724bfdc236c074ca94a249a7f41a86752c7d04ab258ce9ed5651f6a7e` | SIL Open Font License 1.1, `JetBrainsMono-OFL.txt` |
| `JetBrainsMono-latin-ext.woff2` | JetBrains Mono, as above, Latin Extended subset | `79bfdab9ba467e26eea4122e6f2567e188dd8a09a8c730d501fc487c4ab99c6e` | SIL Open Font License 1.1, `JetBrainsMono-OFL.txt` |

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

## Interface fonts (Space Grotesk, JetBrains Mono)

Fonts of the desktop interface (M3 decisions 24 and 25, T3.14): Space Grotesk for headings and
the brand name, JetBrains Mono for values and other monospace text. Body text stays the system
font. `apps/desktop/src/lib/theme/fonts.css` declares them with `@font-face`; Vite copies the
files into the app bundle, so they load from the app itself and never from the network
(rule 9). They live here, not in the frontend folder, so that every bundled font, its license
text and its provenance sit in one place.

- License: SIL Open Font License 1.1, accepted by the owner on 2026-10-13 as "similar" under
  rule 10 on the condition the license sets: every copy carries the copyright notice and the
  full license text. Each family has its license as a plain text file next to it
  (`SpaceGrotesk-OFL.txt`, `JetBrainsMono-OFL.txt`), starting with the copyright notice. The
  About page (M3 decision 29) embeds these files. The fonts are not sold on their own. DimoMark does
  not modify them: they are byte identical to the source files below, which are the
  distributors' Latin and Latin Extended subsets of the upstream releases. Neither font's
  `name` table names a Reserved Font Name.
- Check after copying: the copyright line and version were read from the `name` table of
  each file (macOS Core Text, name IDs 0 and 5); the license URL in name ID 14 is
  `https://scripts.sil.org/OFL` in all four files.

### Space Grotesk

- Origin: copied unchanged from the owner's local cnc-master site repository,
  `cnc-master-landing/assets/fonts/space-grotesk-v22-latin_latin-ext-500.woff2` and
  `-700.woff2` (Latin and Latin Extended subset, "v22" is the Google Fonts release of the
  family). Nothing was downloaded for DimoMark.
- Copyright (name ID 0): "Copyright 2020 The Space Grotesk Project Authors
  (https://github.com/floriankarsten/space-grotesk)". Version (name ID 5): 2.000.
- License text: the source folder carries no license file. `SpaceGrotesk-OFL.txt` is the
  copyright line from the font file followed by the unchanged SIL OFL 1.1 text, taken from the
  JetBrains Mono license file below (the OFL text is the same for every font). Whether the
  upstream project names a Reserved Font Name could not be checked offline; the font's
  `name` table carries none, and DimoMark uses the font unmodified.

### JetBrains Mono

- Origin: copied unchanged from the npm package `@fontsource-variable/jetbrains-mono` 5.2.8
  (font release "v24" of the Google Fonts collection, metadata dated 2025-09-11), installed in
  the owner's local `peterbuilds_v2/node_modules`: `files/jetbrains-mono-latin-wght-normal.woff2`
  and `files/jetbrains-mono-latin-ext-wght-normal.woff2` (the upright variable font, Latin and
  Latin Extended subsets; italic, Cyrillic, Greek and Vietnamese subsets are not bundled).
- Copyright (name ID 0): "Copyright 2020 The JetBrains Mono Project Authors
  (https://github.com/JetBrains/JetBrainsMono)". Version (name ID 5): 2.211.
- License text: `JetBrainsMono-OFL.txt` is the package's `LICENSE` file, unchanged (SHA-256
  `403581b69dac5cff4079205e01c6b467e56af449ecbd7247693ddb1baafa005b`).
- Glyph coverage: the Latin subset covers ASCII, Latin-1 (`Ø ± ° × ä ö ü ß`), the general
  punctuation block (`′ ″`) and U+2212 minus; `⌀` (U+2300) is not in either subset and falls
  back to the next font in the stack.
