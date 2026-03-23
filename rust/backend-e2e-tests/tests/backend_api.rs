use tstring_html as backend_html;
use tstring_html::FormatOptions;
use tstring_syntax::{TemplateInput, TemplateInterpolation, TemplateSegment};
use tstring_thtml as backend_thtml;

fn interpolation(index: usize, expression: &str, raw_source: &str) -> TemplateSegment {
    TemplateSegment::Interpolation(TemplateInterpolation {
        expression: expression.to_owned(),
        conversion: None,
        format_spec: String::new(),
        interpolation_index: index,
        raw_source: Some(raw_source.to_owned()),
    })
}

#[test]
fn html_backend_public_api_smoke_test() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<div class=\"".to_owned()),
        interpolation(0, "classes", "{classes}"),
        TemplateSegment::StaticText("\">Hello ".to_owned()),
        interpolation(1, "name", "{name}"),
        TemplateSegment::StaticText("</div>".to_owned()),
    ]);

    backend_html::check_template(&template).expect("expected html check success");
    assert_eq!(
        backend_html::format_template(&template).expect("expected html format success"),
        "<div class=\"{classes}\">Hello {name}</div>"
    );
}

#[test]
fn html_backend_title_interpolation_renders_escaped_text() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<title>".to_owned()),
        interpolation(0, "title", "{title}"),
        TemplateSegment::StaticText("</title>".to_owned()),
    ]);

    backend_html::check_template(&template).expect("title should validate");
    assert_eq!(
        backend_html::format_template(&template).expect("expected html format success"),
        "<title>{title}</title>"
    );

    let compiled = backend_html::compile_template(&template).expect("compile title");
    let rendered = backend_html::render_html(
        &compiled,
        &backend_html::RuntimeContext {
            values: vec![backend_html::RuntimeValue::RawHtml("<safe>".to_owned())],
        },
    )
    .expect("render title");
    assert_eq!(rendered, "<title>&lt;safe&gt;</title>");
}

#[test]
fn html_backend_format_preserves_conversion_and_format_spec() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<div data-repr=\"".to_owned()),
        TemplateSegment::Interpolation(TemplateInterpolation {
            expression: "value".to_owned(),
            conversion: Some("r".to_owned()),
            format_spec: String::new(),
            interpolation_index: 0,
            raw_source: Some("{value!r}".to_owned()),
        }),
        TemplateSegment::StaticText("\">".to_owned()),
        TemplateSegment::Interpolation(TemplateInterpolation {
            expression: "amount".to_owned(),
            conversion: None,
            format_spec: ".2f".to_owned(),
            interpolation_index: 1,
            raw_source: Some("{amount:.2f}".to_owned()),
        }),
        TemplateSegment::StaticText("</div>".to_owned()),
    ]);

    assert_eq!(
        backend_html::format_template(&template).expect("expected html format success"),
        "<div data-repr=\"{value!r}\">{amount:.2f}</div>"
    );
}

#[test]
fn thtml_backend_public_api_smoke_test() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<Button kind=\"primary\">".to_owned()),
        interpolation(0, "label", "{label}"),
        TemplateSegment::StaticText("</Button>".to_owned()),
    ]);

    backend_thtml::check_template(&template).expect("expected thtml check success");
    assert_eq!(
        backend_thtml::format_template(&template).expect("expected thtml format success"),
        "<Button kind=\"primary\">{label}</Button>"
    );
}

#[test]
fn thtml_backend_title_interpolation_renders_escaped_text() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<title>".to_owned()),
        interpolation(0, "title", "{title}"),
        TemplateSegment::StaticText("</title>".to_owned()),
    ]);

    backend_thtml::check_template(&template).expect("title should validate");
    assert_eq!(
        backend_thtml::format_template(&template).expect("expected thtml format success"),
        "<title>{title}</title>"
    );

    let compiled = backend_thtml::compile_template(&template).expect("compile title");
    let rendered = backend_thtml::render_html(
        &compiled,
        &backend_html::RuntimeContext {
            values: vec![backend_html::RuntimeValue::RawHtml("<safe>".to_owned())],
        },
    )
    .expect("render title");
    assert_eq!(rendered, "<title>&lt;safe&gt;</title>");
}

#[test]
fn html_check_reports_spans_for_invalid_templates_end_to_end() {
    let template =
        TemplateInput::from_segments(vec![TemplateSegment::StaticText("<div></span>".to_owned())]);

    let error = backend_html::check_template(&template).expect_err("expected html parse failure");
    let first = error
        .diagnostics
        .first()
        .expect("expected at least one diagnostic");
    assert_eq!(first.code, "html.parse.mismatched_tag");
    assert!(first.span.is_some());
}

#[test]
fn html_semantic_errors_report_spans_for_unquoted_dynamic_attrs_and_components() {
    let attr_template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<div title=".to_owned()),
        interpolation(0, "title", "{title}"),
        TemplateSegment::StaticText("></div>".to_owned()),
    ]);
    let attr_error =
        backend_html::check_template(&attr_template).expect_err("expected attr semantic failure");
    let first = attr_error
        .diagnostics
        .first()
        .expect("expected attr diagnostic");
    assert!(first.span.is_some());
    assert!(attr_error.message.contains("quoted"));

    let component_template =
        TemplateInput::from_segments(vec![TemplateSegment::StaticText("<Button />".to_owned())]);
    let component_error = backend_html::check_template(&component_template)
        .expect_err("expected html component semantic failure");
    let first = component_error
        .diagnostics
        .first()
        .expect("expected component diagnostic");
    assert!(first.span.is_some());
    assert!(
        component_error
            .message
            .to_ascii_lowercase()
            .contains("component")
    );
}

#[test]
fn html_static_key_parts_match_template_strings_shape() {
    let template = TemplateInput::from_segments(vec![
        interpolation(0, "first", "{first}"),
        interpolation(1, "second", "{second}"),
        TemplateSegment::StaticText("<div>".to_owned()),
        interpolation(2, "third", "{third}"),
    ]);

    assert_eq!(
        backend_html::static_key_parts(&template),
        vec![
            String::new(),
            String::new(),
            "<div>".to_owned(),
            String::new(),
        ]
    );
}

#[test]
fn thtml_runtime_without_bindings_is_runtime_error() {
    let template =
        TemplateInput::from_segments(vec![TemplateSegment::StaticText("<Button />".to_owned())]);
    let compiled = backend_thtml::compile_template(&template).expect("compile should succeed");
    let err = backend_thtml::render_html(&compiled, &backend_html::RuntimeContext::default())
        .expect_err("render should fail without bindings runtime");
    assert_eq!(
        err.message,
        "Component rendering requires the bindings layer runtime context."
    );
}

#[test]
fn html_format_requires_raw_source_on_template_input_seam() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<div>".to_owned()),
        TemplateSegment::Interpolation(TemplateInterpolation {
            expression: "name".to_owned(),
            conversion: None,
            format_spec: String::new(),
            interpolation_index: 0,
            raw_source: None,
        }),
        TemplateSegment::StaticText("</div>".to_owned()),
    ]);

    let err = backend_html::format_template(&template).expect_err("format should fail");
    assert_eq!(
        err.message,
        "Formatting requires raw_source for interpolation 'name'."
    );
}

#[test]
fn html_static_key_parts_concatenate_adjacent_static_segments() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<".to_owned()),
        TemplateSegment::StaticText("div>".to_owned()),
        interpolation(0, "name", "{name}"),
        TemplateSegment::StaticText("</".to_owned()),
        TemplateSegment::StaticText("div>".to_owned()),
    ]);

    assert_eq!(
        backend_html::static_key_parts(&template),
        vec!["<div>".to_owned(), "</div>".to_owned()]
    );
}

#[test]
fn thtml_backend_rejects_unquoted_dynamic_attrs_and_raw_text_interpolation() {
    let attr_template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<Button kind=".to_owned()),
        interpolation(0, "kind", "{kind}"),
        TemplateSegment::StaticText(" />".to_owned()),
    ]);
    let attr_err =
        backend_thtml::check_template(&attr_template).expect_err("attr check should fail");
    assert_eq!(
        attr_err.message,
        "Dynamic attribute value for 'kind' must be quoted."
    );
    assert!(
        attr_err
            .diagnostics
            .first()
            .and_then(|d| d.span.as_ref())
            .is_some()
    );

    let raw_text_template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<script>".to_owned()),
        interpolation(0, "script", "{script}"),
        TemplateSegment::StaticText("</script>".to_owned()),
    ]);
    let raw_text_err =
        backend_thtml::check_template(&raw_text_template).expect_err("raw-text check should fail");
    assert_eq!(
        raw_text_err.message,
        "Interpolations are not allowed inside <script>."
    );
    assert!(
        raw_text_err
            .diagnostics
            .first()
            .and_then(|d| d.span.as_ref())
            .is_some()
    );
}

#[test]
fn thtml_format_requires_raw_source_on_template_input_seam() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<Button>".to_owned()),
        TemplateSegment::Interpolation(TemplateInterpolation {
            expression: "label".to_owned(),
            conversion: None,
            format_spec: String::new(),
            interpolation_index: 0,
            raw_source: None,
        }),
        TemplateSegment::StaticText("</Button>".to_owned()),
    ]);

    let err = backend_thtml::format_template(&template).expect_err("format should fail");
    assert_eq!(
        err.message,
        "Formatting requires raw_source for interpolation 'label'."
    );
}

#[test]
fn html_formatter_normalizes_self_closing_and_attribute_quotes() {
    let template = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
        "<div hidden title='say \"hi\"'></div><img src=test /><section />".to_owned(),
    )]);

    assert_eq!(
        backend_html::format_template(&template).expect("expected html format success"),
        "<div hidden title=\"say &quot;hi&quot;\"></div><img src=\"test\" /><section></section>"
    );
}

#[test]
fn thtml_formatter_preserves_component_self_closing_and_source_order() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<Button kind='primary' ".to_owned()),
        interpolation(0, "attrs", "{attrs}"),
        TemplateSegment::StaticText(" disabled />".to_owned()),
    ]);

    assert_eq!(
        backend_thtml::format_template(&template).expect("expected thtml format success"),
        "<Button kind=\"primary\" {attrs} disabled />"
    );
}

#[test]
fn formatter_breaks_pure_element_children_and_keeps_mixed_content_inline() {
    let pure = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
        "<div><span></span><span></span></div>".to_owned(),
    )]);
    assert_eq!(
        backend_html::format_template_with_options(&pure, &FormatOptions { line_length: 20 })
            .expect("expected html format success"),
        "<div>\n  <span></span>\n  <span></span>\n</div>"
    );

    let mixed = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
        "<div>Hello <span>world</span>!</div>".to_owned(),
    )]);
    assert_eq!(
        backend_html::format_template_with_options(&mixed, &FormatOptions { line_length: 10 })
            .expect("expected html format success"),
        "<div>Hello <span>world</span>!</div>"
    );
}

#[test]
fn formatter_preserves_raw_text_and_multiline_interpolations() {
    let raw_text = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
        "<script>if (a < b) {\n  console.log(a)\n}</script>".to_owned(),
    )]);
    assert_eq!(
        backend_html::format_template(&raw_text).expect("expected html format success"),
        "<script>if (a < b) {\n  console.log(a)\n}</script>"
    );

    let interpolation = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<div title=\"".to_owned()),
        TemplateSegment::Interpolation(TemplateInterpolation {
            expression: "value".to_owned(),
            conversion: None,
            format_spec: String::new(),
            interpolation_index: 0,
            raw_source: Some("{\n  value\n}".to_owned()),
        }),
        TemplateSegment::StaticText("\"></div>".to_owned()),
    ]);
    assert_eq!(
        backend_html::format_template_with_options(
            &interpolation,
            &FormatOptions { line_length: 8 }
        )
        .expect("expected html format success"),
        "<div\n  title=\"{\n  value\n}\"\n></div>"
    );
}

#[test]
fn formatter_is_idempotent() {
    let template = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
        "<div data-a='1' data-b='2'><span></span><span></span></div>".to_owned(),
    )]);
    let once =
        backend_html::format_template_with_options(&template, &FormatOptions { line_length: 24 })
            .expect("expected html format success");
    let twice = backend_html::format_template_with_options(
        &TemplateInput::from_segments(vec![TemplateSegment::StaticText(once.clone())]),
        &FormatOptions { line_length: 24 },
    )
    .expect("expected html format success");
    assert_eq!(once, twice);
}

#[test]
fn formatted_attribute_quote_escaping_preserves_render_output() {
    let template = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
        "<div title='say \"hi\"'></div>".to_owned(),
    )]);
    let formatted = backend_html::format_template(&template).expect("expected html format success");

    let original = backend_html::compile_template(&template).expect("compile original");
    let reparsed = backend_html::compile_template(&TemplateInput::from_segments(vec![
        TemplateSegment::StaticText(formatted),
    ]))
    .expect("compile formatted");

    let original_rendered =
        backend_html::render_html(&original, &backend_html::RuntimeContext::default())
            .expect("render original");
    let reparsed_rendered =
        backend_html::render_html(&reparsed, &backend_html::RuntimeContext::default())
            .expect("render formatted");

    assert_eq!(original_rendered, reparsed_rendered);
}
