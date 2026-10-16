# Workflow

## Branches and commits

- `main` is always green. Work on short lived branches `feat/<topic>`, `fix/<topic>` when using PRs;
  direct commits to `main` are acceptable while the owner is the only maintainer.
- Conventional Commits, signed off: `git commit -s`. Types: feat, fix, perf, refactor, test,
  docs, build, ci, chore. Scope is the crate or area.
- One logical change per commit. Requirement IDs in the body when relevant.

## CI

`.github/workflows/ci.yml` runs on every push and pull request:

- `check`: `./scripts/check.sh` on Ubuntu, macOS and Windows (fmt, clippy, tests, `cargo deny`,
  frontend check, lint, npm licenses, tests).
- `npm-licenses`: `node scripts/check-npm-licenses.mjs`. The allow list mirrors `deny.toml`; the
  accepted build time exceptions are listed in the script and in the STATUS decision log.
- `dco`: on pull requests, `./scripts/check-dco.sh <base> <head>` fails if a commit lacks
  `Signed-off-by`. Run it locally with `./scripts/check-dco.sh origin/main`.

Actions are pinned to full commit SHAs of releases at least 14 days old. Release workflows come
with 0.1.

## Task lifecycle

1. Task is listed in `docs/plan/M<n>.md` with acceptance criteria.
2. Mark it `in progress` in `docs/plan/STATUS.md`.
3. Plan, implement, test, verify (see AGENTS.md "How to work").
4. Tick the acceptance criteria, set the task `done` in STATUS.md, add notes and decisions.
5. Commit.

## Milestones

A milestone is done when every task is done and its exit criteria from
[09 Roadmap](../spec/09-roadmap.md) are measured and recorded. The agent then drafts
`docs/plan/M<n+1>.md` from the roadmap and requirements, and the owner reviews it before work starts.

## Making a release

Releases are built by `.github/workflows/release.yml` (R.13). It never publishes anything itself.

1. Set the version in `Cargo.toml` (`[workspace.package]`), `apps/desktop/src-tauri/tauri.conf.json`
   and `apps/desktop/package.json`. `./scripts/check-version.sh` checks that the three agree.
2. Commit, push, and wait until CI is green on all three platforms.
3. Tag and push the tag: `git tag -s v0.1.0 -m "Dimo 0.1.0"`, `git push origin v0.1.0`. A tag with
   a suffix such as `v0.1.0-rc.1` is a pre-release of that version. Its files carry the same names as
   the final ones (`Dimo_0.1.0_...`; the MSI version must be numeric), only the release tag tells
   them apart.
4. The workflow checks the release gates (CI green on the tagged commit, tag equals the version,
   tolerance tables verified with
   `DIMO_RELEASE=1`, dependencies at least 14 days old, cargo deny, npm licenses), then builds:
   macOS universal DMG (ad-hoc signed, D-08), Windows MSI with the WebView2 bootstrapper and a
   portable zip (D-11), Linux AppImage and deb built on Ubuntu 22.04 (D-10). Every build bundles
   PDFium and its licenses (`scripts/stage-pdfium.sh`, `tauri.release.<os>.json`).
5. It creates a **draft** GitHub Release with all files and `SHA256SUMS.txt`. Review the files,
   replace the notes with the release notes, then publish the draft by hand.

To try the builds without a release, run the workflow by hand (Actions, Release, "Run workflow");
the files are then workflow artifacts. A local installer build works the same way:

```sh
./scripts/stage-pdfium.sh mac-universal        # or win-x64, linux-x64, mac-arm64
pnpm -C apps/desktop tauri build --config src-tauri/tauri.release.macos.json   # .windows / .linux
```

On Windows run `stage-pdfium.sh` in Git Bash (there is no PowerShell version).

The installed app finds PDFium in `Contents/Frameworks/` (macOS) or in `pdfium/` in its resource
folder (Windows, Linux). `DIMO_PDFIUM_PATH` still overrides it.

## Spikes

Throwaway experiments go to `spikes/<name>/` with a README stating the question and the answer.
Spikes are never imported by crates. Findings move into docs or code, then the spike can be deleted.
