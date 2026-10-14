//! Rendering tests of dimo-report (T3.2): determinism (FR-EXP-11), sandbox (NFR-SEC-05),
//! diagnostics with file and line.

#![allow(clippy::expect_used)] // Test crate; rust.md allows expect in tests.

use dimo_core::Language;
use dimo_report::{RenderRequest, ReportDate, ReportError, TemplateSet, bundled_ids, render_pdf};
use serde_json::{Value, json};

/// A table document like a report: title, columns and rows from the data, page x of y.
const TABLE_DOC: &str = r#"
#let data = json("/data.json")
#set page(paper: "a4", flipped: true, margin: 12mm,
  header: [#data.title #h(1fr) #context counter(page).display() / #context counter(page).final().first()])
#set text(font: "Open Sans", size: 8pt, lang: sys.inputs.lang)
#show "⌀": "Ø"
#table(columns: data.columns.len(), table.header(..data.columns.map(strong)), ..data.rows.flatten())
"#;

fn table_data(rows: usize) -> Value {
    json!({
        "title": "Table Ø ± ° ′ ″ × ⌀ − äöüÄÖÜß",
        "columns": ["No.", "Requirement", "Lower", "Upper", "1", "2", "3"],
        "rows": (1..=rows).map(|i| vec![
            i.to_string(),
            format!("Ø{}.5 ±0.05", 10 + i % 50),
            format!("{}.45", 10 + i % 50),
            format!("{}.55", 10 + i % 50),
            "12.10".to_string(),
            "12.08".to_string(),
            "12.11".to_string(),
        ]).collect::<Vec<_>>(),
    })
}

fn request(data: &Value, language: Language) -> RenderRequest<'_> {
    RenderRequest {
        document: "doc",
        data,
        language,
        date: Some(ReportDate {
            year: 2026,
            month: 10,
            day: 12,
        }),
    }
}

/// A template set with one document `doc` whose source is `main`.
fn custom(main: &str) -> TemplateSet {
    TemplateSet::from_files([
        (
            "template.toml".to_string(),
            b"id = \"test\"\nversion = 1\n[[documents]]\nname = \"doc\"\nmain = \"main.typ\"\n"
                .to_vec(),
        ),
        ("main.typ".to_string(), main.as_bytes().to_vec()),
    ])
    .expect("valid test template")
}

fn table_set() -> TemplateSet {
    custom(TABLE_DOC)
}

fn render_custom(main: &str) -> Result<Vec<u8>, ReportError> {
    let data = json!({ "title": "ok" });
    render_pdf(
        &custom(main),
        &RenderRequest {
            document: "doc",
            data: &data,
            language: Language::En,
            date: None,
        },
    )
}

/// The error text of a render that must fail.
fn refusal(main: &str) -> String {
    match render_custom(main) {
        Ok(_) => panic!("template {main:?} rendered but must be refused"),
        Err(err) => err.to_string(),
    }
}

#[test]
fn bundled_ids_list_the_default_set() {
    assert_eq!(bundled_ids().collect::<Vec<_>>(), ["dimo-default"]);
    assert!(matches!(
        TemplateSet::bundled("nope"),
        Err(ReportError::UnknownTemplate(_))
    ));
}

#[test]
fn table_renders_in_both_languages() {
    let data = table_data(120);
    for language in [Language::En, Language::De] {
        let pdf = render_pdf(&table_set(), &request(&data, language)).expect("render");
        assert!(pdf.starts_with(b"%PDF-"), "not a PDF");
        // 120 rows do not fit on one A4 landscape page: the page tree counts several kids.
        let text = String::from_utf8_lossy(&pdf);
        assert!(
            text.contains("/Count 4") || text.contains("/Count 3"),
            "page count"
        );
    }
}

#[test]
fn two_renders_are_byte_identical() {
    let data = table_data(60);
    let first = render_pdf(&table_set(), &request(&data, Language::De)).expect("first render");
    // Start the second render with an empty cache so nothing is reused.
    typst::comemo::evict(0);
    let second = render_pdf(&table_set(), &request(&data, Language::De)).expect("second render");
    assert!(first == second, "two renders differ");
}

#[test]
fn output_depends_on_data_language_and_date() {
    let data = table_data(5);
    let base = render_pdf(&table_set(), &request(&data, Language::En)).expect("render");
    let german = render_pdf(&table_set(), &request(&data, Language::De)).expect("render");
    assert_ne!(base, german);
    let other_data = table_data(6);
    let more = render_pdf(&table_set(), &request(&other_data, Language::En)).expect("render");
    assert_ne!(base, more);
    let mut undated = request(&data, Language::En);
    undated.date = None;
    let undated = render_pdf(&table_set(), &undated).expect("render");
    assert_ne!(base, undated);
}

#[test]
fn creation_date_comes_from_the_request() {
    let data = table_data(1);
    let pdf = render_pdf(&table_set(), &request(&data, Language::En)).expect("render");
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("D:20261012"), "creation date missing");
    let mut bad = request(&data, Language::En);
    bad.date = Some(ReportDate {
        year: 2026,
        month: 2,
        day: 30,
    });
    assert!(matches!(
        render_pdf(&table_set(), &bad),
        Err(ReportError::InvalidDate(_))
    ));
}

#[test]
fn unknown_document_is_refused() {
    let data = json!({});
    let err = render_pdf(
        &table_set(),
        &RenderRequest {
            document: "missing",
            data: &data,
            language: Language::En,
            date: None,
        },
    )
    .expect_err("unknown document");
    assert!(matches!(err, ReportError::UnknownDocument { .. }));
}

#[test]
fn sandbox_refuses_files_outside_the_template() {
    for main in [
        "#read(\"/etc/passwd\")",
        "#read(\"/Users/someone/secret.txt\")",
        "#image(\"/tmp/a.png\")",
        "#include \"other.typ\"",
        "#import \"/lib.typ\": *",
    ] {
        let err = refusal(main);
        assert!(
            err.contains("is not a file of the report template"),
            "{main}: {err}"
        );
    }
    let err = refusal("#read(\"../../Cargo.toml\")");
    assert!(err.contains("escape"), "{err}");
}

#[test]
fn sandbox_refuses_packages_plugins_and_the_clock() {
    for main in [
        "#import \"@preview/example:0.1.0\": *",
        "#import \"@local/example:0.1.0\": *",
    ] {
        let err = refusal(main);
        assert!(err.contains("packages are not available"), "{main}: {err}");
    }
    let err = refusal("#plugin(\"/plugin.wasm\")");
    assert!(
        err.contains("is not a file of the report template"),
        "{err}"
    );
    let err = refusal("#datetime.today().display()");
    assert!(err.contains("current date"), "{err}");
}

#[test]
fn template_reads_data_and_language() {
    render_custom("#let d = json(\"/data.json\")\n#d.title #sys.inputs.lang").expect("render");
}

#[test]
fn compile_errors_carry_file_and_line() {
    let err = render_custom("Line one\n\n#let x = (\n#undefined_name\n").expect_err("error");
    let ReportError::Compile(diagnostics) = &err else {
        panic!("expected a compile error, got {err}");
    };
    let first = &diagnostics[0];
    assert_eq!(first.file.as_deref(), Some("main.typ"));
    assert!(first.line.is_some_and(|l| l >= 3), "{first}");
    assert!(
        err.to_string()
            .starts_with("report template error: main.typ:"),
        "{err}"
    );

    let err = render_custom("#undefined_name").expect_err("error");
    let text = err.to_string();
    assert!(
        text.contains("main.typ:1:2: unknown variable: undefined_name"),
        "{text}"
    );
}

#[test]
fn invalid_template_sets_are_refused() {
    let manifest = |main: &str| {
        format!("id = \"t\"\nversion = 1\n[[documents]]\nname = \"d\"\nmain = \"{main}\"\n")
            .into_bytes()
    };
    let cases: Vec<Vec<(String, Vec<u8>)>> = vec![
        vec![],
        vec![("template.toml".into(), b"id = 1".to_vec())],
        vec![("template.toml".into(), manifest("missing.typ"))],
        vec![
            ("template.toml".into(), manifest("a.typ")),
            ("../a.typ".into(), Vec::new()),
        ],
        vec![
            ("template.toml".into(), manifest("a.typ")),
            ("/a.typ".into(), Vec::new()),
        ],
        vec![
            ("template.toml".into(), manifest("data.json")),
            ("data.json".into(), Vec::new()),
        ],
        vec![(
            "template.toml".into(),
            b"id = \"t\"\nversion = 1\nlabels = \"labels.toml\"\n".to_vec(),
        )],
    ];
    for files in cases {
        let names: Vec<String> = files.iter().map(|(p, _)| p.clone()).collect();
        assert!(
            matches!(
                TemplateSet::from_files(files),
                Err(ReportError::InvalidTemplate(_))
            ),
            "{names:?}"
        );
    }
}
