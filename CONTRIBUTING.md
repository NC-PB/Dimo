# Contributing to Dimo

Thank you for helping. Dimo is maintained by a single maintainer, so small, focused changes are easiest to review.

## Before you start

1. Read the [clean room policy](docs/spec/00-clean-room-policy.md). It is mandatory.
   In short: describe features by user problem and public standard, never by another product,
   and never name other products anywhere in the repository.
2. Check [docs/plan/STATUS.md](docs/plan/STATUS.md) and open issues to avoid duplicate work.
3. For larger changes, open an issue first.

## Developer Certificate of Origin

Every commit must be signed off, affirming the [DCO](https://developercertificate.org/):

```sh
git commit -s -m "feat(dimo-notation): parse stacked deviations"
```

## Commits

[Conventional Commits](https://www.conventionalcommits.org/). Scope is the crate or area:
`dimo-core`, `dimo-pdf`, `desktop`, `ci`, `docs`, `corpus`, `data`.

## Checks

Run `./scripts/check.sh` before opening a pull request. See [docs/dev/](docs/dev/) for conventions.

## Test drawings

Only drawings you created yourself or that carry an explicit permissive license. Every file needs an
entry in [corpus/PROVENANCE.md](corpus/PROVENANCE.md) and a metadata check for personal information.

## AI assisted contributions

Welcome, under the same rules. You are responsible for every line you submit and sign off.
Agents follow [AGENTS.md](AGENTS.md).
