use tstring_format_doc::{Doc, RenderOptions, flat_width, has_forced_break, render};

use crate::{
    Attribute, AttributeLike, CommentNode, ComponentTagNode, DoctypeNode, Document, ElementNode,
    FormatFlavor, FormatOptions, InterpolationNode, Node, RawTextElementNode, ValuePart,
    is_raw_text_tag,
};

pub fn format_document(
    document: &Document,
    options: &FormatOptions,
    flavor: FormatFlavor,
) -> String {
    let doc = build_nodes(&document.children, flavor, options);
    render(
        &doc,
        RenderOptions {
            line_length: options.line_length.max(1),
            indent_width: 2,
        },
    )
}

fn build_nodes(nodes: &[Node], flavor: FormatFlavor, options: &FormatOptions) -> Doc {
    Doc::concat(
        nodes
            .iter()
            .map(|node| build_node(node, flavor, options))
            .collect(),
    )
}

fn build_node(node: &Node, flavor: FormatFlavor, options: &FormatOptions) -> Doc {
    match node {
        Node::Fragment(fragment) => build_nodes(&fragment.children, flavor, options),
        Node::Element(element) => build_element(element, flavor, options),
        Node::ComponentTag(component) => build_component(component, flavor, options),
        Node::Text(text) => Doc::text(text.value.clone()),
        Node::Interpolation(interpolation) => build_interpolation(interpolation),
        Node::Comment(comment) => build_comment(comment),
        Node::Doctype(doctype) => build_doctype(doctype),
        Node::RawTextElement(element) => build_raw_text_element(element, flavor, options),
    }
}

fn build_element(element: &ElementNode, flavor: FormatFlavor, options: &FormatOptions) -> Doc {
    let is_void = matches!(flavor, FormatFlavor::Html | FormatFlavor::Thtml)
        && is_void_html_tag(&element.name)
        && element.children.is_empty();
    let start = build_start_tag(
        &element.name,
        &element.attributes,
        if is_void { " />" } else { ">" },
    );
    if is_void {
        return start;
    }
    build_standard_element(&element.name, start, &element.children, flavor, options)
}

fn build_component(
    component: &ComponentTagNode,
    flavor: FormatFlavor,
    options: &FormatOptions,
) -> Doc {
    if matches!(flavor, FormatFlavor::Thtml)
        && component.self_closing
        && component.children.is_empty()
    {
        return build_start_tag(&component.name, &component.attributes, " />");
    }

    let start = build_start_tag(&component.name, &component.attributes, ">");
    build_standard_element(&component.name, start, &component.children, flavor, options)
}

fn build_raw_text_element(
    element: &RawTextElementNode,
    flavor: FormatFlavor,
    options: &FormatOptions,
) -> Doc {
    debug_assert!(is_raw_text_tag(&element.name));
    let start = build_start_tag(&element.name, &element.attributes, ">");
    let close = close_tag(&element.name);
    let children = build_nodes(&element.children, flavor, options);
    Doc::concat(vec![start, children, close])
}

fn build_standard_element(
    name: &str,
    start: Doc,
    children: &[Node],
    flavor: FormatFlavor,
    options: &FormatOptions,
) -> Doc {
    let close = close_tag(name);
    if children.is_empty() {
        return Doc::concat(vec![start, close]);
    }

    let mixed = is_mixed_content(children);
    if mixed {
        return Doc::concat(vec![start, build_nodes(children, flavor, options), close]);
    }

    let significant_children = strip_padding_whitespace(children);
    if significant_children.is_empty() {
        return Doc::concat(vec![start, close]);
    }

    let child_doc = build_nodes(&significant_children, flavor, options);
    let inline_doc = Doc::concat(vec![start.clone(), child_doc.clone(), close.clone()]);
    if flat_width(&inline_doc).is_some_and(|width| width <= options.line_length.max(1))
        && !has_forced_break(&child_doc)
    {
        return inline_doc;
    }

    let broken_children = join_with_hard_lines(
        significant_children
            .iter()
            .map(|child| build_node(child, flavor, options))
            .collect(),
    );

    Doc::concat(vec![
        start,
        Doc::concat(vec![Doc::hard_line(), broken_children]).indent(),
        Doc::hard_line(),
        close,
    ])
}

fn build_start_tag(name: &str, attributes: &[AttributeLike], closing: &str) -> Doc {
    if attributes.is_empty() {
        return Doc::text(format!("<{name}{closing}"));
    }

    let mut parts = vec![Doc::text(format!("<{name}"))];
    let attr_lines = attributes
        .iter()
        .flat_map(|attribute| [Doc::line(), build_attribute_like(attribute)])
        .collect();
    parts.push(Doc::concat(attr_lines).indent());
    parts.push(Doc::soft_line());
    parts.push(Doc::text(closing.to_string()));
    Doc::concat(parts).group()
}

fn build_attribute_like(attribute: &AttributeLike) -> Doc {
    match attribute {
        AttributeLike::Attribute(attribute) => build_attribute(attribute),
        AttributeLike::SpreadAttribute(spread) => build_interpolation(&spread.interpolation),
    }
}

fn build_attribute(attribute: &Attribute) -> Doc {
    let Some(value) = &attribute.value else {
        return Doc::text(attribute.name.clone());
    };

    let mut parts = vec![Doc::text(format!("{}=\"", attribute.name))];
    for part in &value.parts {
        parts.push(match part {
            ValuePart::Text(text) => Doc::text(escape_attribute_text(text)),
            ValuePart::Interpolation(interpolation) => build_interpolation(interpolation),
        });
    }
    parts.push(Doc::text("\""));
    Doc::concat(parts)
}

fn build_interpolation(interpolation: &InterpolationNode) -> Doc {
    Doc::text(
        interpolation
            .raw_source
            .clone()
            .unwrap_or_else(|| "{}".to_string()),
    )
}

fn build_comment(comment: &CommentNode) -> Doc {
    Doc::text(format!("<!--{}-->", comment.value))
}

fn build_doctype(doctype: &DoctypeNode) -> Doc {
    Doc::text(format!("<!{}>", doctype.value))
}

fn close_tag(name: &str) -> Doc {
    Doc::text(format!("</{name}>"))
}

fn strip_padding_whitespace(children: &[Node]) -> Vec<Node> {
    children
        .iter()
        .filter(|child| !matches!(child, Node::Text(text) if text.value.trim().is_empty()))
        .cloned()
        .collect()
}

fn join_with_hard_lines(parts: Vec<Doc>) -> Doc {
    let mut docs = Vec::new();
    for (index, part) in parts.into_iter().enumerate() {
        if index > 0 {
            docs.push(Doc::hard_line());
        }
        docs.push(part);
    }
    Doc::concat(docs)
}

fn is_mixed_content(children: &[Node]) -> bool {
    children.iter().any(|child| match child {
        Node::Text(text) => !text.value.trim().is_empty(),
        Node::Fragment(fragment) => is_mixed_content(&fragment.children),
        _ => false,
    })
}

fn escape_attribute_text(text: &str) -> String {
    text.replace('"', "&quot;")
}

fn is_void_html_tag(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}
