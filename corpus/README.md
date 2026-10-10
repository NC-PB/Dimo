# Test corpus

Drawings used for recognition development and regression tests (see [08 Recognition pipeline](../docs/spec/08-recognition-pipeline.md)).

```
corpus/
  drawings/      source files (PDF, TIFF, PNG)
  truth/         ground truth per drawing: <name>.truth.json (schema: docs/schema/truth.schema.json)
  notes/         observations per drawing: sheet kind, baseline results, pitfalls
  PROVENANCE.md  origin and license of every file
```

Rules: every file needs a provenance entry before it is committed. Only self-made drawings or drawings with an explicit permissive license.

A truth file may name the project `tolerance_settings` its expected limits assume (for example ISO 2768-1 class m). Without it the limits assume a new project: no general tolerance, so untoleranced dimensions have no limits.

`cargo run -p dimo-cli -- eval box-select` runs box select with the tolerance engine on every truth region of the corpus and of a fixed set of synthetic drawings and reports per callout category (M2 exit criterion). CI runs it through `scripts/check.sh`.
