# Truth regions from the content stream (T0.10)

Question: how to get exact regions for `corpus/truth/test_drawing_1.truth.json` before Dimo can
extract text itself (T0.6)?

Answer: `extract_text_boxes.py` reads the page content stream with pypdf and computes a box for
each of the 74 text objects from the page CTM, `Tm`, `Tf`, the glyph widths (`/W`) and the font
ascent and descent. The truth regions are the union of the boxes of each callout's text objects,
without leading or trailing spaces, rounded to 0.01 pt. Checked by drawing the boxes over a
render of the sheet: every box covers its callout.

The script handles only what this drawing uses (one font, no rotation, `Tj` only). It is not part
of the build and not a dependency of any crate.
