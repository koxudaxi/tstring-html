use tstring_syntax::{ErrorKind, TemplateInput, TemplateInterpolation, TemplateSegment};
use tstring_tdom as backend_tdom;

fn interpolation(index: usize, expression: &str, raw_source: &str) -> TemplateSegment {
    TemplateSegment::Interpolation(TemplateInterpolation {
        expression: expression.to_owned(),
        conversion: None,
        format_spec: String::new(),
        interpolation_index: index,
        raw_source: Some(raw_source.to_owned()),
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

#[test]
fn tdom_backend_public_api_smoke_test() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<".to_owned()),
        interpolation(0, "Button", "{Button}"),
        TemplateSegment::StaticText(" kind=".to_owned()),
        interpolation(1, "kind", "{kind}"),
        TemplateSegment::StaticText(">".to_owned()),
        interpolation(2, "label", "{label}"),
        TemplateSegment::StaticText("</".to_owned()),
        interpolation(3, "Button", "{Button}"),
        TemplateSegment::StaticText(">".to_owned()),
    ]);

    backend_tdom::check_template(&template).expect("expected tdom check success");
    assert_eq!(
        backend_tdom::format_template(&template).expect("expected tdom format success"),
        "<{Button} kind={kind}>{label}</{Button}>"
    );

    let document = backend_tdom::prepare_template(&template).expect("prepare tdom");
    let backend_tdom::Node::ComponentTag(component) = &document.children[0] else {
        panic!("expected component tag");
    };
    assert_eq!(component.start_tag.raw_source.as_deref(), Some("{Button}"));
    assert_eq!(
        component
            .end_tag
            .as_ref()
            .and_then(|tag| tag.raw_source.as_deref()),
        Some("{Button}")
    );
}

#[test]
fn tdom_backend_comment_interpolation_spans_are_preserved() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<!--before ".to_owned()),
        interpolation(0, "value", "{value}"),
        TemplateSegment::StaticText(" after-->".to_owned()),
    ]);

    let document = backend_tdom::prepare_template(&template).expect("prepare comment");
    let backend_tdom::Node::Comment(comment) = &document.children[0] else {
        panic!("expected comment");
    };
    assert_eq!(comment.parts.len(), 3);
    let backend_tdom::ValuePart::Interpolation(interpolation) = &comment.parts[1] else {
        panic!("expected interpolation part");
    };
    assert_eq!(interpolation.interpolation_index, 0);
    assert!(interpolation.span.is_some());
    assert_eq!(
        backend_tdom::format_template(&template).expect("format comment"),
        "<!--before {value} after-->"
    );
}

#[test]
fn tdom_backend_raw_text_and_rcdata_cases_round_trip() {
    let script = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<script>if (a < b) ".to_owned()),
        interpolation(0, "body", "{body}"),
        TemplateSegment::StaticText("</script>".to_owned()),
    ]);
    backend_tdom::check_template(&script).expect("script should parse");
    let prepared_script = backend_tdom::prepare_template(&script).expect("prepare script");
    assert!(matches!(
        prepared_script.children.as_slice(),
        [backend_tdom::Node::RawTextElement(_)]
    ));
    assert_eq!(
        backend_tdom::format_template(&script).expect("format script"),
        "<script>if (a < b) {body}</script>"
    );

    let title = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<title>Hello ".to_owned()),
        interpolation(0, "title", "{title}"),
        TemplateSegment::StaticText("</title>".to_owned()),
    ]);
    backend_tdom::check_template(&title).expect("title should parse");
    assert_eq!(
        backend_tdom::format_template(&title).expect("format title"),
        "<title>Hello {title}</title>"
    );

    let textarea = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<textarea>prefix ".to_owned()),
        interpolation(0, "value", "{value}"),
        TemplateSegment::StaticText("</textarea>".to_owned()),
    ]);
    backend_tdom::check_template(&textarea).expect("textarea should parse");
    assert_eq!(
        backend_tdom::format_template(&textarea).expect("format textarea"),
        "<textarea>prefix {value}</textarea>"
    );

    let close_tag_prefix = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
        "<script>const closing = '</scripting>';</script>".to_owned(),
    )]);
    backend_tdom::check_template(&close_tag_prefix).expect("close-tag prefix should not terminate");
    assert_eq!(
        backend_tdom::format_template(&close_tag_prefix).expect("format close-tag prefix"),
        "<script>const closing = '</scripting>';</script>"
    );
}

#[test]
fn tdom_backend_rejects_invalid_declarations_and_tag_names() {
    let invalid_doctype = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
        "<!doctype-alt html>".to_owned(),
    )]);
    let doctype_error =
        backend_tdom::check_template(&invalid_doctype).expect_err("doctype should fail");
    assert_eq!(doctype_error.kind, ErrorKind::Parse);
    assert!(doctype_error.message.contains("DOCTYPE"));

    let invalid_component = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<".to_owned()),
        interpolation(0, "Button", "{Button}"),
        TemplateSegment::StaticText("Suffix></".to_owned()),
        interpolation(1, "Button", "{Button}"),
        TemplateSegment::StaticText(">".to_owned()),
    ]);
    let component_error =
        backend_tdom::check_template(&invalid_component).expect_err("component should fail");
    assert_eq!(component_error.kind, ErrorKind::Parse);
    assert!(
        component_error
            .message
            .contains("exactly one interpolation")
    );
}

#[test]
fn tdom_backend_normalizes_void_tags_and_preserves_raw_source() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<BR><div data-safe=".to_owned()),
        interpolation_with_format(0, "value", "{value:safe}", "safe"),
        TemplateSegment::StaticText(">".to_owned()),
        interpolation_with_format(1, "other", "{other:unsafe}", "unsafe"),
        TemplateSegment::StaticText("</div>".to_owned()),
    ]);

    assert_eq!(
        backend_tdom::format_template(&template).expect("format tdom"),
        "<br /><div data-safe={value:safe}>{other:unsafe}</div>"
    );
}

#[test]
fn tdom_backend_rejects_mismatched_component_end_tags_when_statically_evident() {
    let template = TemplateInput::from_segments(vec![
        TemplateSegment::StaticText("<".to_owned()),
        interpolation(0, "Button", "{Button}"),
        TemplateSegment::StaticText("></".to_owned()),
        interpolation(1, "Other", "{Other}"),
        TemplateSegment::StaticText(">".to_owned()),
    ]);

    let error = backend_tdom::check_template(&template).expect_err("must reject mismatch");
    assert_eq!(error.kind, ErrorKind::Parse);
    assert!(
        error
            .message
            .contains("Mismatched component start and end callables")
    );
}
