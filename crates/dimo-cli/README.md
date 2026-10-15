# dimo-cli

Headless command line tool for Dimo.

## Export

Exports a saved project through the same code as the desktop app, so the files are byte
identical to the app's exports of the same project. The project file is never changed.

```sh
cargo run -p dimo-cli -- export part.dimo --format inspection-report-pdf --language de -o report.pdf
```

Formats: `ballooned-pdf`, `csv`, `xlsx`, `check-sheet-pdf`, `check-sheet-xlsx`,
`inspection-report-pdf`, `inspection-report-xlsx`. A report of a project without a report date
needs `--date YYYY-MM-DD`; the app stores today's date at the first report export, the tool reads
no clock.
