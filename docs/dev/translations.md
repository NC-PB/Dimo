# Adding a UI language

Dimo ships English and German and is built so that more UI languages can be added without code
changes beyond one table entry (FR-SET-04). Every visible string of the app goes through
[Paraglide](https://inlang.com/m/gerre34r/library-inlang-paraglideJs) messages.

## Where the texts live

| What | Where |
|---|---|
| UI messages, one file per language | `apps/desktop/messages/<tag>.json` |
| List of languages | `apps/desktop/project.inlang/settings.json` (`locales`) |
| Language names in the language menu | `LOCALE_NAMES` in `apps/desktop/src/lib/i18n.ts` |
| User guide | `docs/user/<tag>/` |

`<tag>` is a language tag such as `fr` or `pt-BR`. The generated code in
`apps/desktop/src/lib/paraglide/` is rebuilt from these files and is not committed.

## Steps

1. Add the tag to `locales` in `apps/desktop/project.inlang/settings.json`.
2. Copy `apps/desktop/messages/en.json` to `apps/desktop/messages/<tag>.json` and translate every
   value. Keep the keys and the `{placeholders}` unchanged. English is the base language: a
   missing key falls back to it, but the test below fails, so translate all of them.
3. Add the language's own name to `LOCALE_NAMES` in `apps/desktop/src/lib/i18n.ts`, written in
   that language (for example `fr: "Français"`), so users find it whatever language is shown.
   TypeScript refuses to compile until the entry exists.
4. Update the expected list in `apps/desktop/src/lib/i18n.test.ts`.
5. Run the checks:

   ```sh
   pnpm -C apps/desktop check    # compiles the messages, type checks
   pnpm -C apps/desktop test     # every message file has the same keys as en.json
   pnpm -C apps/desktop tauri dev
   ```

   In the app open Settings, choose the language and look through every view. The window
   reloads when the language changes.

6. Optional: translate the user guide into `docs/user/<tag>/`. Copy `docs/user/en/shortcuts.md`
   and add the table header words of the language to `WORDS` in
   `apps/desktop/src/lib/shortcuts-docs.test.ts`; the test then checks the shortcut tables and
   prints the correct ones when they differ.

## Wording

- Short, plain words of the shop floor. Use the terms of the standards for the language
  (ISO 1101, ISO 286, ISO 2768 in their national editions) for tolerances and characteristics.
- Address the user the same way throughout (the German texts use "du").
- Never copy texts or terms from other inspection or ballooning software (clean room policy,
  `docs/spec/00-clean-room-policy.md`).

## What is not a UI language

- Export and report languages are chosen per export, independent of the UI language (D-32).
  The headers of the characteristic list are defined in Rust
  (`crates/dimo-io/src/export/columns.rs`, `Language`); a new export language needs a change
  there and in `ReportLanguage` of `apps/desktop/src-tauri/src/settings.rs`.
- Enumeration values in exports (`linear`, `diameter`, ...) are stable identifiers and are never
  translated.
