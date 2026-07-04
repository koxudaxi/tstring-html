use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

use serde::Deserialize;
use tstring_html as backend_html;
use tstring_html::{RuntimeContext, RuntimeValue};
use tstring_syntax::{TemplateInput, TemplateInterpolation, TemplateSegment};
use tstring_tdom as backend_tdom;
use tstring_thtml as backend_thtml;

#[derive(Debug, Deserialize)]
struct ProfilesIndex {
    default_profile: String,
    profiles: BTreeMap<String, ProfileEntry>,
}

#[derive(Debug, Deserialize)]
struct ProfileEntry {
    manifest_path: String,
}

#[derive(Debug, Deserialize)]
struct Provenance {
    source: String,
    snapshot: String,
}

#[derive(Debug, Deserialize)]
struct Manifest {
    spec_title: String,
    claim_status: String,
    provenance: Provenance,
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    case_id: String,
    execution_layer: String,
    expected: Option<String>,
    expected_error: Option<String>,
}

fn interpolation(index: usize, expression: &str, raw_source: &str) -> TemplateSegment {
    TemplateSegment::Interpolation(TemplateInterpolation {
        expression: expression.to_owned(),
        conversion: None,
        format_spec: String::new(),
        interpolation_index: index,
        raw_source: Some(raw_source.to_owned()),
    })
}

fn interpolation_without_raw(index: usize, expression: &str) -> TemplateSegment {
    TemplateSegment::Interpolation(TemplateInterpolation {
        expression: expression.to_owned(),
        conversion: None,
        format_spec: String::new(),
        interpolation_index: index,
        raw_source: None,
    })
}

fn interpolation_with_format(
    index: usize,
    expression: &str,
    raw_source: &str,
    format_spec: &str,
) -> TemplateSegment {
    TemplateSegment::Interpolation(TemplateInterpolation {
        expression: expression.to_owned(),
        conversion: None,
        format_spec: format_spec.to_owned(),
        interpolation_index: index,
        raw_source: Some(raw_source.to_owned()),
    })
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust/")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn load_manifest(format_name: &str) -> Manifest {
    let format_root = repo_root().join("conformance").join(format_name);
    let profiles: ProfilesIndex =
        toml::from_str(&fs::read_to_string(format_root.join("profiles.toml")).unwrap()).unwrap();
    let profile = profiles
        .profiles
        .get(&profiles.default_profile)
        .expect("default profile entry");
    toml::from_str(&fs::read_to_string(format_root.join(&profile.manifest_path)).unwrap()).unwrap()
}

fn runtime(values: Vec<RuntimeValue>) -> RuntimeContext {
    RuntimeContext { values }
}

#[test]
fn html_manifest_metadata_is_100_percent_and_unique() {
    let manifest = load_manifest("html");
    assert_eq!(manifest.claim_status, "100%");
    assert!(!manifest.spec_title.is_empty());
    assert!(!manifest.provenance.source.is_empty());
    assert!(!manifest.provenance.snapshot.is_empty());
    let mut ids = BTreeSet::new();
    for case in &manifest.cases {
        assert!(
            ids.insert(case.case_id.clone()),
            "duplicate case id {}",
            case.case_id
        );
        assert!(matches!(
            case.execution_layer.as_str(),
            "python" | "rust" | "both"
        ));
    }
}

#[test]
fn thtml_manifest_metadata_is_100_percent_and_unique() {
    let manifest = load_manifest("thtml");
    assert_eq!(manifest.claim_status, "100%");
    assert!(!manifest.spec_title.is_empty());
    assert!(!manifest.provenance.source.is_empty());
    assert!(!manifest.provenance.snapshot.is_empty());
    let mut ids = BTreeSet::new();
    for case in &manifest.cases {
        assert!(
            ids.insert(case.case_id.clone()),
            "duplicate case id {}",
            case.case_id
        );
        assert!(matches!(
            case.execution_layer.as_str(),
            "python" | "rust" | "both"
        ));
    }
}

#[test]
fn tdom_manifest_metadata_is_100_percent_and_unique() {
    let manifest = load_manifest("tdom");
    assert_eq!(manifest.claim_status, "100%");
    assert!(!manifest.spec_title.is_empty());
    assert!(!manifest.provenance.source.is_empty());
    assert!(!manifest.provenance.snapshot.is_empty());
    let mut ids = BTreeSet::new();
    for case in &manifest.cases {
        assert!(
            ids.insert(case.case_id.clone()),
            "duplicate case id {}",
            case.case_id
        );
        assert!(matches!(
            case.execution_layer.as_str(),
            "python" | "rust" | "both"
        ));
    }
}

#[test]
fn html_manifest_cases_match_rust_backend() {
    let manifest = load_manifest("html");
    for case in manifest
        .cases
        .iter()
        .filter(|case| matches!(case.execution_layer.as_str(), "rust" | "both"))
    {
        match case.case_id.as_str() {
            "escaped-child" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div>".into()),
                    interpolation(0, "name", "{name}"),
                    TemplateSegment::StaticText("</div>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let rendered = backend_html::render_html(
                    &compiled,
                    &runtime(vec![RuntimeValue::String("<world>".into())]),
                )
                .unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "raw-html-child" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div>".into()),
                    interpolation(0, "value", "{value}"),
                    TemplateSegment::StaticText("</div>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let rendered = backend_html::render_html(
                    &compiled,
                    &runtime(vec![RuntimeValue::RawHtml("<strong>safe</strong>".into())]),
                )
                .unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "raw-html-attribute-escaped" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div title=\"".into()),
                    interpolation(0, "value", "{value}"),
                    TemplateSegment::StaticText("\"></div>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let rendered = backend_html::render_html(
                    &compiled,
                    &runtime(vec![RuntimeValue::RawHtml("<b>x</b>".into())]),
                )
                .unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "raw-html-spread-escaped" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div ".into()),
                    interpolation(0, "attrs", "{attrs}"),
                    TemplateSegment::StaticText("></div>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let rendered = backend_html::render_html(
                    &compiled,
                    &runtime(vec![RuntimeValue::Attributes(vec![(
                        "title".into(),
                        RuntimeValue::RawHtml("<b>x</b>".into()),
                    )])]),
                )
                .unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "quoted-dynamic-attribute" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<a href=\"".into()),
                    interpolation(0, "href", "{href}"),
                    TemplateSegment::StaticText("\">Dashboard</a>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let rendered = backend_html::render_html(
                    &compiled,
                    &runtime(vec![RuntimeValue::String("/dashboard".into())]),
                )
                .unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "attribute-ampersand-always-escaped" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div title=\"".into()),
                    interpolation(0, "title", "{title}"),
                    TemplateSegment::StaticText("\"></div>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let rendered = backend_html::render_html(
                    &compiled,
                    &runtime(vec![RuntimeValue::String("Tom &amp; Jerry".into())]),
                )
                .unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "dangerous-url-scheme-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<a href=\"".into()),
                    interpolation(0, "href", "{href}"),
                    TemplateSegment::StaticText("\">x</a>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let err = backend_html::render_html(
                    &compiled,
                    &runtime(vec![RuntimeValue::String("javascript:alert(1)".into())]),
                )
                .unwrap_err();
                assert_eq!(
                    case.expected_error.as_deref(),
                    Some("TemplateSemanticError")
                );
                assert!(err.message.contains("unsafe javascript:"));
            }
            "dangerous-url-scheme-normalized-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<a href=\"".into()),
                    interpolation(0, "href", "{href}"),
                    TemplateSegment::StaticText("\">x</a>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let err = backend_html::render_html(
                    &compiled,
                    &runtime(vec![RuntimeValue::String("java\t script:alert(1)".into())]),
                )
                .unwrap_err();
                assert_eq!(
                    case.expected_error.as_deref(),
                    Some("TemplateSemanticError")
                );
                assert!(err.message.contains("unsafe javascript:"));
            }
            "dangerous-url-spread-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<a ".into()),
                    interpolation(0, "attrs", "{attrs}"),
                    TemplateSegment::StaticText(">x</a>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let err = backend_html::render_html(
                    &compiled,
                    &runtime(vec![RuntimeValue::Attributes(vec![(
                        "href".into(),
                        RuntimeValue::String("data:text/html,<svg></svg>".into()),
                    )])]),
                )
                .unwrap_err();
                assert_eq!(
                    case.expected_error.as_deref(),
                    Some("TemplateSemanticError")
                );
                assert!(err.message.contains("unsafe data:"));
            }
            "boolean-attribute-bare-and-omitted" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<button disabled=\"".into()),
                    interpolation(0, "visible", "{visible}"),
                    TemplateSegment::StaticText("\" hidden=\"".into()),
                    interpolation(1, "hidden", "{hidden}"),
                    TemplateSegment::StaticText("\" data-x=\"".into()),
                    interpolation(2, "missing", "{missing}"),
                    TemplateSegment::StaticText("\">OK</button>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let rendered = backend_html::render_html(
                    &compiled,
                    &runtime(vec![
                        RuntimeValue::Bool(true),
                        RuntimeValue::Bool(false),
                        RuntimeValue::Null,
                    ]),
                )
                .unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "class-normalization" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<button class=\"".into()),
                    interpolation(0, "classes", "{classes}"),
                    TemplateSegment::StaticText("\"></button>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let classes = RuntimeValue::Sequence(vec![
                    RuntimeValue::String("btn".into()),
                    RuntimeValue::Attributes(vec![(
                        "btn-primary".into(),
                        RuntimeValue::Bool(true),
                    )]),
                    RuntimeValue::String("extra".into()),
                    RuntimeValue::Attributes(vec![("active".into(), RuntimeValue::Bool(true))]),
                ]);
                let rendered =
                    backend_html::render_html(&compiled, &runtime(vec![classes])).unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "spread-merge-order" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div class=\"base ".into()),
                    interpolation(0, "spread_classes", "{spread_classes}"),
                    TemplateSegment::StaticText(" ".into()),
                    interpolation(1, "tail", "{tail}"),
                    TemplateSegment::StaticText("\" ".into()),
                    interpolation(2, "attrs", "{attrs}"),
                    TemplateSegment::StaticText("></div>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let rendered = backend_html::render_html(
                    &compiled,
                    &runtime(vec![
                        RuntimeValue::Sequence(vec![
                            RuntimeValue::String("extra".into()),
                            RuntimeValue::Attributes(vec![(
                                "active".into(),
                                RuntimeValue::Bool(true),
                            )]),
                        ]),
                        RuntimeValue::String("tail".into()),
                        RuntimeValue::Attributes(vec![(
                            "data-id".into(),
                            RuntimeValue::String("2".into()),
                        )]),
                    ]),
                )
                .unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "spread-non-mapping-error" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div ".into()),
                    interpolation(0, "attrs", "{attrs}"),
                    TemplateSegment::StaticText("></div>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let err =
                    backend_html::render_html(&compiled, &runtime(vec![RuntimeValue::Int(1)]))
                        .unwrap_err();
                assert_eq!(
                    case.expected_error.as_deref(),
                    Some("TemplateSemanticError")
                );
                assert!(err.message.contains("mapping-like"));
            }
            "spread-invalid-attribute-name-error" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div ".into()),
                    interpolation(0, "attrs", "{attrs}"),
                    TemplateSegment::StaticText("></div>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let err = backend_html::render_html(
                    &compiled,
                    &runtime(vec![RuntimeValue::Attributes(vec![(
                        "x onmouseover=alert(1)".into(),
                        RuntimeValue::String("y".into()),
                    )])]),
                )
                .unwrap_err();
                assert_eq!(
                    case.expected_error.as_deref(),
                    Some("TemplateSemanticError")
                );
                assert!(err.message.contains("attribute name"));
            }
            "fragment-children" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div>".into()),
                    interpolation(0, "children", "{children}"),
                    TemplateSegment::StaticText("</div>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let rendered = backend_html::render_html(
                    &compiled,
                    &runtime(vec![RuntimeValue::Fragment(vec![
                        RuntimeValue::RawHtml("<em>first</em>".into()),
                        RuntimeValue::String("second".into()),
                    ])]),
                )
                .unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "render-fragment" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<p>".into()),
                    interpolation(0, "name", "{name}"),
                    TemplateSegment::StaticText("</p>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let rendered = backend_html::render_fragment(
                    &compiled,
                    &runtime(vec![RuntimeValue::String("world".into())]),
                )
                .unwrap();
                assert_eq!(rendered.html, case.expected.as_deref().unwrap());
            }
            "comment-and-doctype" => {
                let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
                    "<!DOCTYPE html><!--x--><div>ok</div>".into(),
                )]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let rendered = backend_html::render_html(&compiled, &runtime(vec![])).unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "component-rejected" => {
                let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
                    "<Button />".into(),
                )]);
                let err = backend_html::check_template(&input).unwrap_err();
                assert_eq!(
                    case.expected_error.as_deref(),
                    Some("TemplateSemanticError")
                );
                assert!(err.message.contains("Component tag"));
            }
            "raw-text-script-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<script>".into()),
                    interpolation(0, "script", "{script}"),
                    TemplateSegment::StaticText("</script>".into()),
                ]);
                let err = backend_html::check_template(&input).unwrap_err();
                assert_eq!(
                    case.expected_error.as_deref(),
                    Some("TemplateSemanticError")
                );
                assert!(err.message.contains("<script>"));
            }
            "raw-text-style-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<style>".into()),
                    interpolation(0, "style", "{style}"),
                    TemplateSegment::StaticText("</style>".into()),
                ]);
                let err = backend_html::check_template(&input).unwrap_err();
                assert!(err.message.contains("<style>"));
            }
            "raw-text-title-escaped" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<title>".into()),
                    interpolation(0, "title", "{title}"),
                    TemplateSegment::StaticText("</title>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let rendered = backend_html::render_html(
                    &compiled,
                    &runtime(vec![backend_html::RuntimeValue::RawHtml("<safe>".into())]),
                )
                .unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "raw-text-textarea-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<textarea>".into()),
                    interpolation(0, "value", "{value}"),
                    TemplateSegment::StaticText("</textarea>".into()),
                ]);
                let err = backend_html::check_template(&input).unwrap_err();
                assert!(err.message.contains("<textarea>"));
            }
            "parse-mismatched-tag" => {
                let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
                    "<div></span>".into(),
                )]);
                let err = backend_html::check_template(&input).unwrap_err();
                assert_eq!(case.expected_error.as_deref(), Some("TemplateParseError"));
                assert_eq!(err.diagnostics[0].code, "html.parse.mismatched_tag");
            }
            "parse-unclosed-tag" => {
                let input =
                    TemplateInput::from_segments(vec![TemplateSegment::StaticText("<div".into())]);
                let err = backend_html::check_template(&input).unwrap_err();
                assert_eq!(case.expected_error.as_deref(), Some("TemplateParseError"));
                assert_eq!(err.kind, tstring_syntax::ErrorKind::Parse);
                assert!(err.diagnostics[0].code.starts_with("html.parse.unclosed"));
            }
            "format-roundtrip" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div>".into()),
                    interpolation(0, "name", "{name}"),
                    TemplateSegment::StaticText("</div>".into()),
                ]);
                let formatted = backend_html::format_template(&input).unwrap();
                assert_eq!(formatted, case.expected.as_deref().unwrap());
            }
            "format-rich-roundtrip" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div data-repr=\"".into()),
                    TemplateSegment::Interpolation(TemplateInterpolation {
                        expression: "value".into(),
                        conversion: Some("r".into()),
                        format_spec: String::new(),
                        interpolation_index: 0,
                        raw_source: Some("{value!r}".into()),
                    }),
                    TemplateSegment::StaticText("\">".into()),
                    TemplateSegment::Interpolation(TemplateInterpolation {
                        expression: "amount".into(),
                        conversion: None,
                        format_spec: ".2f".into(),
                        interpolation_index: 1,
                        raw_source: Some("{amount:.2f}".into()),
                    }),
                    TemplateSegment::StaticText("</div>".into()),
                ]);
                let formatted = backend_html::format_template(&input).unwrap();
                assert_eq!(formatted, case.expected.as_deref().unwrap());
            }
            "unquoted-dynamic-attr-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div title=".into()),
                    interpolation(0, "title", "{title}"),
                    TemplateSegment::StaticText("></div>".into()),
                ]);
                let err = backend_html::check_template(&input).unwrap_err();
                assert!(err.message.contains("must be quoted"));
            }
            "semantic-attr-primary-span" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div title=".into()),
                    interpolation(0, "title", "{title}"),
                    TemplateSegment::StaticText("></div>".into()),
                ]);
                let err = backend_html::check_template(&input).unwrap_err();
                assert!(
                    err.diagnostics
                        .first()
                        .and_then(|d| d.span.as_ref())
                        .is_some()
                );
            }
            "semantic-component-primary-span" => {
                let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
                    "<Button />".into(),
                )]);
                let err = backend_html::check_template(&input).unwrap_err();
                assert!(
                    err.diagnostics
                        .first()
                        .and_then(|d| d.span.as_ref())
                        .is_some()
                );
            }
            "class-bool-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<button class=\"".into()),
                    interpolation(0, "value", "{value}"),
                    TemplateSegment::StaticText("\"></button>".into()),
                ]);
                let compiled = backend_html::compile_template(&input).unwrap();
                let err =
                    backend_html::render_html(&compiled, &runtime(vec![RuntimeValue::Bool(true)]))
                        .unwrap_err();
                assert!(err.message.to_lowercase().contains("class"));
            }
            "format-missing-raw-source" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div>".into()),
                    interpolation_without_raw(0, "name"),
                    TemplateSegment::StaticText("</div>".into()),
                ]);
                let err = backend_html::format_template(&input).unwrap_err();
                assert!(err.message.contains("raw_source"));
            }
            "static-key-empty-boundaries" => {
                let input = TemplateInput::from_segments(vec![
                    interpolation(0, "first", "{first}"),
                    interpolation(1, "second", "{second}"),
                    TemplateSegment::StaticText("<div>".into()),
                    interpolation(2, "third", "{third}"),
                ]);
                assert_eq!(
                    backend_html::static_key_parts(&input),
                    vec![
                        String::new(),
                        String::new(),
                        "<div>".to_owned(),
                        String::new()
                    ]
                );
            }
            "static-key-concatenate-adjacent" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<".into()),
                    TemplateSegment::StaticText("div>".into()),
                    interpolation(0, "name", "{name}"),
                    TemplateSegment::StaticText("</".into()),
                    TemplateSegment::StaticText("div>".into()),
                ]);
                assert_eq!(
                    backend_html::static_key_parts(&input),
                    vec!["<div>".to_owned(), "</div>".to_owned()]
                );
            }
            "invalid-template-primary-span" => {
                let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
                    "<div></span>".into(),
                )]);
                let err = backend_html::check_template(&input).unwrap_err();
                assert!(
                    err.diagnostics
                        .first()
                        .and_then(|d| d.span.as_ref())
                        .is_some()
                );
            }
            other => panic!("unhandled HTML rust conformance case {other}"),
        }
    }
}

#[test]
fn thtml_manifest_cases_match_rust_seam() {
    let manifest = load_manifest("thtml");
    for case in manifest
        .cases
        .iter()
        .filter(|case| matches!(case.execution_layer.as_str(), "rust" | "both"))
    {
        match case.case_id.as_str() {
            "format-roundtrip" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<Button>".into()),
                    interpolation(0, "label", "{label}"),
                    TemplateSegment::StaticText("</Button>".into()),
                ]);
                let formatted = backend_thtml::format_template(&input).unwrap();
                assert_eq!(formatted, case.expected.as_deref().unwrap());
            }
            "raw-text-script-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<script>".into()),
                    interpolation(0, "script", "{script}"),
                    TemplateSegment::StaticText("</script>".into()),
                ]);
                let err = backend_thtml::check_template(&input).unwrap_err();
                assert!(err.message.contains("<script>"));
            }
            "raw-text-style-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<style>".into()),
                    interpolation(0, "style", "{style}"),
                    TemplateSegment::StaticText("</style>".into()),
                ]);
                let err = backend_thtml::check_template(&input).unwrap_err();
                assert!(err.message.contains("<style>"));
            }
            "raw-text-title-escaped" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<title>".into()),
                    interpolation(0, "title", "{title}"),
                    TemplateSegment::StaticText("</title>".into()),
                ]);
                let compiled = backend_thtml::compile_template(&input).unwrap();
                let rendered = backend_thtml::render_html(
                    &compiled,
                    &runtime(vec![backend_html::RuntimeValue::RawHtml("<safe>".into())]),
                )
                .unwrap();
                assert_eq!(rendered, case.expected.as_deref().unwrap());
            }
            "raw-text-textarea-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<textarea>".into()),
                    interpolation(0, "value", "{value}"),
                    TemplateSegment::StaticText("</textarea>".into()),
                ]);
                let err = backend_thtml::check_template(&input).unwrap_err();
                assert!(err.message.contains("<textarea>"));
            }
            "html-vs-thtml-split" => {
                let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
                    "<Button />".into(),
                )]);
                let err = backend_html::check_template(&input).unwrap_err();
                assert!(err.message.contains("Component tag"));
            }
            "unquoted-dynamic-attr-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<Button kind=".into()),
                    interpolation(0, "kind", "{kind}"),
                    TemplateSegment::StaticText(" />".into()),
                ]);
                let err = backend_thtml::check_template(&input).unwrap_err();
                assert!(err.message.contains("must be quoted"));
            }
            "semantic-attr-primary-span" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<Button kind=".into()),
                    interpolation(0, "kind", "{kind}"),
                    TemplateSegment::StaticText(" />".into()),
                ]);
                let err = backend_thtml::check_template(&input).unwrap_err();
                assert!(
                    err.diagnostics
                        .first()
                        .and_then(|d| d.span.as_ref())
                        .is_some()
                );
            }
            "semantic-raw-text-primary-span" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<script>".into()),
                    interpolation(0, "script", "{script}"),
                    TemplateSegment::StaticText("</script>".into()),
                ]);
                let err = backend_thtml::check_template(&input).unwrap_err();
                assert!(
                    err.diagnostics
                        .first()
                        .and_then(|d| d.span.as_ref())
                        .is_some()
                );
            }
            "parse-mismatched-tag" => {
                let input =
                    TemplateInput::from_segments(vec![TemplateSegment::StaticText("<div".into())]);
                let err = backend_thtml::check_template(&input).unwrap_err();
                assert_eq!(case.expected_error.as_deref(), Some("TemplateParseError"));
                assert_eq!(err.kind, tstring_syntax::ErrorKind::Parse);
                assert!(err.diagnostics[0].code.starts_with("html.parse.unclosed"));
            }
            "parse-unclosed-tag" => {
                let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
                    "<Button".into(),
                )]);
                let err = backend_thtml::check_template(&input).unwrap_err();
                assert_eq!(case.expected_error.as_deref(), Some("TemplateParseError"));
                assert_eq!(err.kind, tstring_syntax::ErrorKind::Parse);
                assert!(err.diagnostics[0].code.starts_with("html.parse.unclosed"));
            }
            "runtime-without-bindings" => {
                let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
                    "<Button />".into(),
                )]);
                let compiled = backend_thtml::compile_template(&input).unwrap();
                let err =
                    backend_thtml::render_html(&compiled, &RuntimeContext::default()).unwrap_err();
                assert!(err.message.contains("bindings layer runtime"));
            }
            "format-missing-raw-source" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<Button>".into()),
                    interpolation_without_raw(0, "label"),
                    TemplateSegment::StaticText("</Button>".into()),
                ]);
                let err = backend_thtml::format_template(&input).unwrap_err();
                assert!(err.message.contains("raw_source"));
            }
            other => panic!("unhandled T-HTML rust conformance case {other}"),
        }
    }
}

#[test]
fn tdom_manifest_cases_match_rust_seam() {
    let manifest = load_manifest("tdom");
    for case in manifest
        .cases
        .iter()
        .filter(|case| matches!(case.execution_layer.as_str(), "rust" | "both"))
    {
        match case.case_id.as_str() {
            "component-format-roundtrip" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<".into()),
                    interpolation(0, "Button", "{Button}"),
                    TemplateSegment::StaticText(" kind=".into()),
                    interpolation(1, "kind", "{kind}"),
                    TemplateSegment::StaticText(">".into()),
                    interpolation(2, "label", "{label}"),
                    TemplateSegment::StaticText("</".into()),
                    interpolation(3, "Button", "{Button}"),
                    TemplateSegment::StaticText(">".into()),
                ]);
                let formatted = backend_tdom::format_template(&input).unwrap();
                assert_eq!(formatted, case.expected.as_deref().unwrap());
            }
            "comment-interpolation-roundtrip" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<!--before ".into()),
                    interpolation(0, "value", "{value}"),
                    TemplateSegment::StaticText(" after-->".into()),
                ]);
                let formatted = backend_tdom::format_template(&input).unwrap();
                assert_eq!(formatted, case.expected.as_deref().unwrap());
            }
            "raw-text-script-roundtrip" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<script>if (a < b) ".into()),
                    interpolation(0, "body", "{body}"),
                    TemplateSegment::StaticText("</script>".into()),
                ]);
                let formatted = backend_tdom::format_template(&input).unwrap();
                assert_eq!(formatted, case.expected.as_deref().unwrap());
            }
            "rcdata-title-roundtrip" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<title>Hello ".into()),
                    interpolation(0, "title", "{title}"),
                    TemplateSegment::StaticText("</title>".into()),
                ]);
                let formatted = backend_tdom::format_template(&input).unwrap();
                assert_eq!(formatted, case.expected.as_deref().unwrap());
            }
            "doctype-html-roundtrip" => {
                let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
                    "<!doctype html>".into(),
                )]);
                let formatted = backend_tdom::format_template(&input).unwrap();
                assert_eq!(formatted, case.expected.as_deref().unwrap());
            }
            "doctype-unknown-rejected" => {
                let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
                    "<!doctype-alt html>".into(),
                )]);
                let err = backend_tdom::check_template(&input).unwrap_err();
                assert_eq!(case.expected_error.as_deref(), Some("TemplateParseError"));
                assert!(err.message.contains("DOCTYPE"));
            }
            "component-name-singleton-rejected" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<".into()),
                    interpolation(0, "Button", "{Button}"),
                    TemplateSegment::StaticText("Suffix></".into()),
                    interpolation(1, "Button", "{Button}"),
                    TemplateSegment::StaticText(">".into()),
                ]);
                let err = backend_tdom::check_template(&input).unwrap_err();
                assert_eq!(case.expected_error.as_deref(), Some("TemplateParseError"));
                assert!(err.message.contains("exactly one interpolation"));
            }
            "void-normalization" => {
                let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
                    "<BR><Img>".into(),
                )]);
                let formatted = backend_tdom::format_template(&input).unwrap();
                assert_eq!(formatted, case.expected.as_deref().unwrap());
            }
            "raw-source-preserved" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<div data-safe=".into()),
                    interpolation_with_format(0, "value", "{value:safe}", "safe"),
                    TemplateSegment::StaticText(">".into()),
                    interpolation_with_format(1, "other", "{other:unsafe}", "unsafe"),
                    TemplateSegment::StaticText("</div>".into()),
                ]);
                let formatted = backend_tdom::format_template(&input).unwrap();
                assert_eq!(formatted, case.expected.as_deref().unwrap());
            }
            "component-close-expression-runtime-validated" => {
                let input = TemplateInput::from_segments(vec![
                    TemplateSegment::StaticText("<".into()),
                    interpolation(0, "Button", "{Button}"),
                    TemplateSegment::StaticText("></".into()),
                    interpolation(1, "Other", "{Other}"),
                    TemplateSegment::StaticText(">".into()),
                ]);
                let formatted = backend_tdom::format_template(&input).unwrap();
                assert_eq!(formatted, case.expected.as_deref().unwrap());
            }
            other => panic!("unhandled TDOM rust conformance case {other}"),
        }
    }
}
