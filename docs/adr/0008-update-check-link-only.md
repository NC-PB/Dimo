# ADR 0008: Update check as a button with a link to the release page

Status: proposed

## Context
D-05 planned the Tauri updater against GitHub Releases, off by default and offered once in the
first run dialog. That needs an updater key pair, signed update bundles, an installer flow inside
the app and a network capability that can download and run code. Dimo is local first (rule 9,
NFR-SEC-01) and 0.1 ships unsigned builds (D-08). The owner decided on 2026-10-14 to keep updates
manual for now.

## Decision
- Help (and File > About) get a "Check for update" button. Only when the user clicks it, Dimo asks
  GitHub Releases for the latest published version (one HTTPS request, no data sent beyond what the
  request itself needs) and compares it with its own version.
- If a newer version exists, Dimo shows its version number and a link that opens the release page
  in the user's browser. Dimo never downloads or installs anything itself.
- No automatic check, no first run question, no background network calls. An organization policy
  file can hide the button (NFR-SEC-03). The default build makes no network call until the button
  is clicked.

## Consequences
- No updater keys, no signed update bundles, no install code inside the app; less to secure.
- Users update by downloading and installing the new release themselves, as they did the first
  one; the getting started guide explains it.
- One narrow network permission (a single GET to the GitHub API host) is added to the app.
- Changes D-05 in `docs/spec/12-implementation-defaults.md` (updated in the same change to point
  here). A full updater can be reconsidered with code signing before 1.0 (D-08).
