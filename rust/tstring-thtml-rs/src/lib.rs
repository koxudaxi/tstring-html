use tstring_html::{
    AttributeLike, CompiledHtmlTemplate, Document, FormatOptions, Node, RenderedFragment,
    RuntimeContext, ValuePart, format_document_as_thtml_with_options, format_template_syntax,
    parse_template, runtime_error,
};
use tstring_syntax::{BackendError, BackendResult, SourceSpan, TemplateInput};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledThtmlTemplate {
    document: Document,
}

pub fn check_template(template: &TemplateInput) -> BackendResult<()> {
    prepare_template(template).map(|_| ())
}

pub fn format_template(template: &TemplateInput) -> BackendResult<String> {
    format_template_with_options(template, &FormatOptions::default())
}

pub fn format_template_with_options(
    template: &TemplateInput,
    options: &FormatOptions,
) -> BackendResult<String> {
    let document = format_template_syntax(template)?;
    validate_thtml_document(&document)?;
    Ok(format_document_as_thtml_with_options(&document, options))
}

pub fn compile_template(template: &TemplateInput) -> BackendResult<CompiledThtmlTemplate> {
    let document = prepare_template(template)?;
    Ok(CompiledThtmlTemplate { document })
}

pub fn render_html(
    compiled: &CompiledThtmlTemplate,
    context: &RuntimeContext,
) -> BackendResult<String> {
    if contains_components(&compiled.document) {
        return Err(runtime_error(
            "thtml.runtime.component_resolution",
            "Component rendering requires the bindings layer runtime context.",
            None,
        ));
    }
    let html_compiled = CompiledHtmlTemplate::from_document(compiled.document.clone());
    tstring_html::render_html(&html_compiled, context)
}

pub fn render_fragment(
    compiled: &CompiledThtmlTemplate,
    context: &RuntimeContext,
) -> BackendResult<RenderedFragment> {
    Ok(RenderedFragment {
        html: render_html(compiled, context)?,
    })
}

impl CompiledThtmlTemplate {
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    #[must_use]
    pub fn from_document(document: Document) -> Self {
        Self { document }
    }
}

pub fn prepare_template(template: &TemplateInput) -> BackendResult<Document> {
    let document = parse_template(template)?;
    validate_thtml_document(&document)?;
    Ok(document)
}

fn validate_thtml_document(document: &Document) -> BackendResult<()> {
    for child in &document.children {
        validate_thtml_node(child)?;
    }
    Ok(())
}

fn validate_thtml_node(node: &Node) -> BackendResult<()> {
    match node {
        Node::Element(element) => {
            validate_attributes(&element.attributes)?;
            for child in &element.children {
                validate_thtml_node(child)?;
            }
            Ok(())
        }
        Node::RawTextElement(element) => {
            validate_attributes(&element.attributes)?;
            validate_raw_text_children(element)
        }
        Node::ComponentTag(component) => {
            validate_attributes(&component.attributes)?;
            for child in &component.children {
                validate_thtml_node(child)?;
            }
            Ok(())
        }
        Node::Fragment(fragment) => {
            for child in &fragment.children {
                validate_thtml_node(child)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_raw_text_children(element: &tstring_html::RawTextElementNode) -> BackendResult<()> {
    for child in &element.children {
        match child {
            Node::Text(_) => {}
            Node::Interpolation(_) if element.name == "title" => {}
            Node::Interpolation(interpolation) => {
                return Err(semantic_error(
                    "html.semantic.raw_text_interpolation",
                    format!("Interpolations are not allowed inside <{}>.", element.name),
                    interpolation.span.clone(),
                ));
            }
            _ => {
                let message = if element.name == "title" {
                    format!(
                        "Only text and interpolations are allowed inside <{}>.",
                        element.name
                    )
                } else {
                    format!("Only text is allowed inside <{}>.", element.name)
                };
                return Err(semantic_error(
                    "html.semantic.raw_text_content",
                    message,
                    element.span.clone(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_attributes(attributes: &[AttributeLike]) -> BackendResult<()> {
    for attribute in attributes {
        match attribute {
            AttributeLike::Attribute(attribute) => {
                if let Some(value) = &attribute.value {
                    if !value.quoted
                        && value
                            .parts
                            .iter()
                            .any(|part| matches!(part, ValuePart::Interpolation(_)))
                    {
                        return Err(semantic_error(
                            "html.semantic.unquoted_dynamic_attr",
                            format!(
                                "Dynamic attribute value for '{}' must be quoted.",
                                attribute.name
                            ),
                            attribute.span.clone(),
                        ));
                    }
                }
            }
            AttributeLike::SpreadAttribute(_) => {}
        }
    }
    Ok(())
}

fn semantic_error(
    code: impl Into<String>,
    message: impl Into<String>,
    span: Option<SourceSpan>,
) -> BackendError {
    BackendError::semantic_at(code, message, span)
}

fn contains_components(document: &Document) -> bool {
    document.children.iter().any(node_contains_component)
}

fn node_contains_component(node: &Node) -> bool {
    match node {
        Node::ComponentTag(_) => true,
        Node::Element(element) => element.children.iter().any(node_contains_component),
        Node::RawTextElement(element) => element.children.iter().any(node_contains_component),
        Node::Fragment(fragment) => fragment.children.iter().any(node_contains_component),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tstring_syntax::TemplateSegment;

    #[test]
    fn thtml_accepts_component_tags() {
        let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
            "<Button disabled />".to_string(),
        )]);
        check_template(&input).expect("thtml should allow component tags");
    }

    #[test]
    fn thtml_runtime_without_bindings_rejects_components() {
        let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
            "<Button />".to_string(),
        )]);
        let compiled = compile_template(&input).expect("compile thtml");
        let err = render_html(&compiled, &RuntimeContext::default()).expect_err("must fail");
        assert_eq!(
            err.message,
            "Component rendering requires the bindings layer runtime context."
        );
    }

    #[test]
    fn thtml_accepts_title_interpolation() {
        let input = TemplateInput::from_segments(vec![
            TemplateSegment::StaticText("<title>".to_string()),
            TemplateSegment::Interpolation(tstring_syntax::TemplateInterpolation {
                expression: "title".to_string(),
                conversion: None,
                format_spec: String::new(),
                interpolation_index: 0,
                raw_source: Some("{title}".to_string()),
            }),
            TemplateSegment::StaticText("</title>".to_string()),
        ]);
        check_template(&input).expect("title interpolation should be allowed");
        assert_eq!(
            format_template(&input).expect("format title"),
            "<title>{title}</title>"
        );
    }
}
