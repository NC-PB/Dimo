# Test corpus

Drawings used for recognition development and regression tests (see [08 Recognition pipeline](../docs/spec/08-recognition-pipeline.md)).

```
corpus/
  drawings/      source files (PDF, TIFF, PNG)
  truth/         ground truth per drawing: <name>.truth.json (format defined in M0)
  notes/         observations per drawing: sheet kind, baseline results, pitfalls
  PROVENANCE.md  origin and license of every file
```

Rules: every file needs a provenance entry before it is committed. Only self-made drawings or drawings with an explicit permissive license.
