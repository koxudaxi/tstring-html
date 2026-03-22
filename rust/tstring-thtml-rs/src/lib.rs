use tstring_html::{
    format_template_syntax, parse_template, runtime_error, AttributeLike, CompiledHtmlTemplate,
    Document, Node, RuntimeContext, ValuePart, RenderedFragment,
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
    let document = format_template_syntax(template)?;
    validate_thtml_document(&document)?;
    Ok(format_document(&document))
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
            for child in &element.children {
                match child {
                    Node::Interpolation(interpolation) => {
                        return Err(semantic_error(
                            "html.semantic.raw_text_interpolation",
                            format!("Interpolations are not allowed inside <{}>.", element.name),
                            interpolation.span.clone(),
                        ));
                    }
                    Node::Text(_) => {}
                    _ => {
                        return Err(semantic_error(
                            "html.semantic.raw_text_content",
                            format!("Only text is allowed inside <{}>.", element.name),
                            element.span.clone(),
                        ));
                    }
                }
            }
            Ok(())
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

fn format_document(document: &Document) -> String {
    let mut out = String::new();
    for node in &document.children {
        format_node(node, &mut out);
    }
    out
}

fn format_node(node: &Node, out: &mut String) {
    match node {
        Node::Fragment(fragment) => {
            for child in &fragment.children {
                format_node(child, out);
            }
        }
        Node::Element(element) => {
            format_tag_like(&element.name, &element.attributes, &element.children, element.self_closing, out);
        }
        Node::ComponentTag(component) => {
            format_tag_like(
                &component.name,
                &component.attributes,
                &component.children,
                component.self_closing,
                out,
            );
        }
        Node::RawTextElement(element) => {
            format_tag_like(&element.name, &element.attributes, &element.children, false, out);
        }
        Node::Text(text) => out.push_str(&text.value),
        Node::Interpolation(interpolation) => {
            if let Some(raw_source) = &interpolation.raw_source {
                out.push_str(raw_source);
            }
        }
        Node::Comment(comment) => {
            out.push_str("<!--");
            out.push_str(&comment.value);
            out.push_str("-->");
        }
        Node::Doctype(doctype) => {
            out.push_str("<!DOCTYPE ");
            out.push_str(&doctype.value);
            out.push('>');
        }
    }
}

fn format_tag_like(
    name: &str,
    attributes: &[tstring_html::AttributeLike],
    children: &[Node],
    self_closing: bool,
    out: &mut String,
) {
    out.push('<');
    out.push_str(name);
    for attribute in attributes {
        out.push(' ');
        match attribute {
            tstring_html::AttributeLike::Attribute(attribute) => {
                out.push_str(&attribute.name);
                if let Some(value) = &attribute.value {
                    out.push('=');
                    if value.quoted {
                        out.push('"');
                    }
                    for part in &value.parts {
                        match part {
                            tstring_html::ValuePart::Text(text) => out.push_str(text),
                            tstring_html::ValuePart::Interpolation(interpolation) => {
                                if let Some(raw_source) = &interpolation.raw_source {
                                    out.push_str(raw_source);
                                }
                            }
                        }
                    }
                    if value.quoted {
                        out.push('"');
                    }
                }
            }
            tstring_html::AttributeLike::SpreadAttribute(attribute) => {
                if let Some(raw_source) = &attribute.interpolation.raw_source {
                    out.push_str(raw_source);
                }
            }
        }
    }
    if self_closing {
        out.push_str(" />");
        return;
    }
    out.push('>');
    for child in children {
        format_node(child, out);
    }
    out.push_str("</");
    out.push_str(name);
    out.push('>');
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
        let input =
            TemplateInput::from_segments(vec![TemplateSegment::StaticText("<Button />".to_string())]);
        let compiled = compile_template(&input).expect("compile thtml");
        let err = render_html(&compiled, &RuntimeContext::default()).expect_err("must fail");
        assert_eq!(err.message, "Component rendering requires the bindings layer runtime context.");
    }
}
