use std::collections::BTreeMap;

mod formatter;

use tstring_syntax::{
    BackendError, BackendResult, Diagnostic, ErrorKind, SourceSpan, StreamItem, TemplateInput,
    TemplateInterpolation, TemplateSegment,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatOptions {
    pub line_length: usize,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self { line_length: 80 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatFlavor {
    Html,
    Thtml,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub children: Vec<Node>,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node {
    Fragment(FragmentNode),
    Element(ElementNode),
    ComponentTag(ComponentTagNode),
    Text(TextNode),
    Interpolation(InterpolationNode),
    Comment(CommentNode),
    Doctype(DoctypeNode),
    RawTextElement(RawTextElementNode),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FragmentNode {
    pub children: Vec<Node>,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElementNode {
    pub name: String,
    pub attributes: Vec<AttributeLike>,
    pub children: Vec<Node>,
    pub self_closing: bool,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentTagNode {
    pub name: String,
    pub attributes: Vec<AttributeLike>,
    pub children: Vec<Node>,
    pub self_closing: bool,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawTextElementNode {
    pub name: String,
    pub attributes: Vec<AttributeLike>,
    pub children: Vec<Node>,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttributeLike {
    Attribute(Attribute),
    SpreadAttribute(SpreadAttribute),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub value: Option<AttributeValue>,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributeValue {
    pub quoted: bool,
    pub parts: Vec<ValuePart>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValuePart {
    Text(String),
    Interpolation(InterpolationNode),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpreadAttribute {
    pub interpolation: InterpolationNode,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextNode {
    pub value: String,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterpolationNode {
    pub interpolation_index: usize,
    pub expression: String,
    pub raw_source: Option<String>,
    pub conversion: Option<String>,
    pub format_spec: String,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommentNode {
    pub value: String,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctypeNode {
    pub value: String,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledHtmlTemplate {
    document: Document,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RuntimeValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Fragment(Vec<RuntimeValue>),
    RawHtml(String),
    Sequence(Vec<RuntimeValue>),
    Attributes(Vec<(String, RuntimeValue)>),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RuntimeContext {
    pub values: Vec<RuntimeValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedFragment {
    pub html: String,
}

#[derive(Clone, Debug)]
enum Token {
    Char(char, Option<SourceSpan>),
    Interpolation(TemplateInterpolation, Option<SourceSpan>),
    Eof,
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn new(input: &TemplateInput) -> Self {
        let mut tokens = Vec::new();
        for item in flatten_input(input) {
            match item {
                StreamItem::Char { ch, span } => tokens.push(Token::Char(ch, Some(span))),
                StreamItem::Interpolation {
                    interpolation,
                    span,
                    ..
                } => tokens.push(Token::Interpolation(interpolation, Some(span))),
                StreamItem::Eof { .. } => tokens.push(Token::Eof),
            }
        }
        if tokens.is_empty() || !matches!(tokens.last(), Some(Token::Eof)) {
            tokens.push(Token::Eof);
        }
        Self { tokens, index: 0 }
    }

    fn parse_document(&mut self) -> BackendResult<Document> {
        let children = self.parse_nodes(None, false)?;
        Ok(Document {
            span: merge_children_span(&children),
            children,
        })
    }

    fn parse_nodes(
        &mut self,
        closing_tag: Option<&str>,
        raw_text_mode: bool,
    ) -> BackendResult<Vec<Node>> {
        let mut children = Vec::new();
        loop {
            if self.is_eof() {
                if let Some(name) = closing_tag {
                    return Err(parse_error(
                        "html.parse.unclosed_tag",
                        format!("Unclosed tag <{name}>."),
                        self.current_span(),
                    ));
                }
                break;
            }

            if let Some(name) = closing_tag {
                if self.starts_with_literal("</") {
                    let close_span = self.current_span();
                    self.consume_literal("</");
                    self.skip_whitespace();
                    let close_name = self.parse_name()?;
                    self.skip_whitespace();
                    self.expect_char('>')?;
                    if close_name != name {
                        return Err(parse_error(
                            "html.parse.mismatched_tag",
                            format!("Mismatched closing tag </{close_name}>. Expected </{name}>."),
                            close_span,
                        ));
                    }
                    break;
                }
                if raw_text_mode {
                    if let Some(text) = self.parse_raw_text_chunk(name)? {
                        children.push(text);
                        continue;
                    }
                }
            }

            if self.is_eof() {
                break;
            }

            if self.starts_with_literal("<!--") {
                children.push(Node::Comment(self.parse_comment()?));
                continue;
            }
            if self.starts_with_doctype() {
                children.push(Node::Doctype(self.parse_doctype()?));
                continue;
            }
            if self.current_is_char('<') {
                children.push(self.parse_tag()?);
                continue;
            }
            if let Some(interpolation) = self.take_interpolation() {
                children.push(Node::Interpolation(interpolation));
                continue;
            }
            children.push(Node::Text(self.parse_text()?));
        }
        Ok(children)
    }

    fn parse_raw_text_chunk(&mut self, closing_tag: &str) -> BackendResult<Option<Node>> {
        let mut text = String::new();
        let mut span = None;
        while !self.is_eof() {
            if self.starts_with_close_tag(closing_tag) {
                break;
            }
            match self.current() {
                Token::Interpolation(_, _) => {
                    if !text.is_empty() {
                        return Ok(Some(Node::Text(TextNode { value: text, span })));
                    }
                    if let Some(interpolation) = self.take_interpolation() {
                        return Ok(Some(Node::Interpolation(interpolation)));
                    }
                }
                Token::Char(ch, node_span) => {
                    span = merge_span_opt(span, node_span.clone());
                    text.push(*ch);
                    self.index += 1;
                }
                Token::Eof => break,
            }
        }
        if text.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Node::Text(TextNode { value: text, span })))
        }
    }

    fn parse_comment(&mut self) -> BackendResult<CommentNode> {
        let start = self.current_span();
        self.consume_literal("<!--");
        let mut value = String::new();
        while !self.is_eof() && !self.starts_with_literal("-->") {
            match self.current() {
                Token::Char(ch, _) => {
                    value.push(*ch);
                    self.index += 1;
                }
                Token::Interpolation(_, span) => {
                    return Err(parse_error(
                        "html.parse.comment_interpolation",
                        "Interpolations are not allowed inside HTML comments.",
                        span.clone(),
                    ));
                }
                Token::Eof => break,
            }
        }
        if !self.starts_with_literal("-->") {
            return Err(parse_error(
                "html.parse.comment_unclosed",
                "Unclosed HTML comment.",
                start,
            ));
        }
        self.consume_literal("-->");
        Ok(CommentNode { value, span: start })
    }

    fn parse_doctype(&mut self) -> BackendResult<DoctypeNode> {
        let start = self.current_span();
        self.consume_char('<')?;
        self.consume_char('!')?;
        let mut value = String::new();
        while !self.is_eof() {
            if self.current_is_char('>') {
                self.index += 1;
                break;
            }
            match self.current() {
                Token::Char(ch, _) => {
                    value.push(*ch);
                    self.index += 1;
                }
                Token::Interpolation(_, span) => {
                    return Err(parse_error(
                        "html.parse.doctype_interpolation",
                        "Interpolations are not allowed inside a doctype.",
                        span.clone(),
                    ));
                }
                Token::Eof => break,
            }
        }
        Ok(DoctypeNode {
            value: value.trim().to_string(),
            span: start,
        })
    }

    fn parse_tag(&mut self) -> BackendResult<Node> {
        let start = self.current_span();
        self.expect_char('<')?;
        let name = self.parse_name()?;
        let mut attributes = Vec::new();
        loop {
            self.skip_whitespace();
            if self.starts_with_literal("/>") {
                self.consume_literal("/>");
                let kind = classify_tag_name(&name);
                let span = start;
                return Ok(match kind {
                    TagKind::Html => {
                        if is_raw_text_tag(&name) {
                            Node::RawTextElement(RawTextElementNode {
                                name,
                                attributes,
                                children: Vec::new(),
                                span,
                            })
                        } else {
                            Node::Element(ElementNode {
                                name,
                                attributes,
                                children: Vec::new(),
                                self_closing: true,
                                span,
                            })
                        }
                    }
                    TagKind::Component => Node::ComponentTag(ComponentTagNode {
                        name,
                        attributes,
                        children: Vec::new(),
                        self_closing: true,
                        span,
                    }),
                });
            }
            if self.current_is_char('>') {
                self.index += 1;
                break;
            }
            if self.is_eof() {
                return Err(parse_error(
                    "html.parse.unclosed_start_tag",
                    format!("Unclosed start tag <{name}>."),
                    start,
                ));
            }
            if let Some(interpolation) = self.take_interpolation() {
                attributes.push(AttributeLike::SpreadAttribute(SpreadAttribute {
                    span: interpolation.span.clone(),
                    interpolation,
                }));
                continue;
            }
            attributes.push(AttributeLike::Attribute(self.parse_attribute()?));
        }

        let kind = classify_tag_name(&name);
        let children = if is_raw_text_tag(&name) {
            self.parse_nodes(Some(&name), true)?
        } else {
            self.parse_nodes(Some(&name), false)?
        };
        let span = merge_span_opt(start, merge_children_span(&children));
        Ok(match kind {
            TagKind::Html => {
                if is_raw_text_tag(&name) {
                    Node::RawTextElement(RawTextElementNode {
                        name,
                        attributes,
                        children,
                        span,
                    })
                } else {
                    Node::Element(ElementNode {
                        name,
                        attributes,
                        children,
                        self_closing: false,
                        span,
                    })
                }
            }
            TagKind::Component => Node::ComponentTag(ComponentTagNode {
                name,
                attributes,
                children,
                self_closing: false,
                span,
            }),
        })
    }

    fn parse_attribute(&mut self) -> BackendResult<Attribute> {
        let span = self.current_span();
        let name = self.parse_name()?;
        self.skip_whitespace();
        if !self.current_is_char('=') {
            return Ok(Attribute {
                name,
                value: None,
                span,
            });
        }
        self.index += 1;
        self.skip_whitespace();
        let value = self.parse_attribute_value()?;
        Ok(Attribute {
            name,
            value: Some(value),
            span,
        })
    }

    fn parse_attribute_value(&mut self) -> BackendResult<AttributeValue> {
        if self.current_is_char('"') || self.current_is_char('\'') {
            let quote = self.current_char().unwrap_or('"');
            self.index += 1;
            let mut parts = Vec::new();
            let mut text = String::new();
            while !self.is_eof() {
                if self.current_is_char(quote) {
                    self.index += 1;
                    break;
                }
                if let Some(interpolation) = self.take_interpolation() {
                    if !text.is_empty() {
                        parts.push(ValuePart::Text(std::mem::take(&mut text)));
                    }
                    parts.push(ValuePart::Interpolation(interpolation));
                    continue;
                }
                match self.current() {
                    Token::Char(ch, _) => {
                        text.push(*ch);
                        self.index += 1;
                    }
                    Token::Eof => break,
                    Token::Interpolation(_, _) => {}
                }
            }
            if !text.is_empty() {
                parts.push(ValuePart::Text(text));
            }
            return Ok(AttributeValue {
                quoted: true,
                parts,
            });
        }

        if let Some(interpolation) = self.take_interpolation() {
            return Ok(AttributeValue {
                quoted: false,
                parts: vec![ValuePart::Interpolation(interpolation)],
            });
        }

        let mut text = String::new();
        while !self.is_eof() {
            if self.current_is_whitespace()
                || self.current_is_char('>')
                || self.starts_with_literal("/>")
            {
                break;
            }
            match self.current() {
                Token::Char(ch, _) => {
                    text.push(*ch);
                    self.index += 1;
                }
                Token::Interpolation(_, _) | Token::Eof => break,
            }
        }
        Ok(AttributeValue {
            quoted: false,
            parts: vec![ValuePart::Text(text)],
        })
    }

    fn parse_text(&mut self) -> BackendResult<TextNode> {
        let mut value = String::new();
        let mut span = self.current_span();
        while !self.is_eof() && !self.current_is_char('<') {
            if matches!(self.current(), Token::Interpolation(_, _)) {
                break;
            }
            match self.current() {
                Token::Char(ch, node_span) => {
                    span = merge_span_opt(span, node_span.clone());
                    value.push(*ch);
                    self.index += 1;
                }
                Token::Interpolation(_, _) | Token::Eof => break,
            }
        }
        Ok(TextNode { value, span })
    }

    fn parse_name(&mut self) -> BackendResult<String> {
        let mut name = String::new();
        while !self.is_eof() {
            match self.current() {
                Token::Char(ch, _) if is_name_char(*ch, name.is_empty()) => {
                    name.push(*ch);
                    self.index += 1;
                }
                _ => break,
            }
        }
        if name.is_empty() {
            Err(parse_error(
                "html.parse.expected_name",
                "Expected a tag or attribute name.",
                self.current_span(),
            ))
        } else {
            Ok(name)
        }
    }

    fn take_interpolation(&mut self) -> Option<InterpolationNode> {
        match self.current().clone() {
            Token::Interpolation(interpolation, span) => {
                self.index += 1;
                Some(InterpolationNode {
                    interpolation_index: interpolation.interpolation_index,
                    expression: interpolation.expression,
                    raw_source: interpolation.raw_source,
                    conversion: interpolation.conversion,
                    format_spec: interpolation.format_spec,
                    span,
                })
            }
            _ => None,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.current_is_whitespace() {
            self.index += 1;
        }
    }

    fn starts_with_literal(&self, value: &str) -> bool {
        for (offset, expected) in value.chars().enumerate() {
            match self.tokens.get(self.index + offset) {
                Some(Token::Char(ch, _)) if *ch == expected => {}
                _ => return false,
            }
        }
        true
    }

    fn starts_with_close_tag(&self, name: &str) -> bool {
        let literal = format!("</{name}");
        self.starts_with_literal(&literal)
    }

    fn starts_with_doctype(&self) -> bool {
        let literal = "<!DOCTYPE";
        for (offset, expected) in literal.chars().enumerate() {
            match self.tokens.get(self.index + offset) {
                Some(Token::Char(ch, _)) if ch.eq_ignore_ascii_case(&expected) => {}
                _ => return false,
            }
        }
        true
    }

    fn consume_literal(&mut self, literal: &str) {
        for _ in literal.chars() {
            self.index += 1;
        }
    }

    fn consume_char(&mut self, expected: char) -> BackendResult<()> {
        self.expect_char(expected)
    }

    fn expect_char(&mut self, expected: char) -> BackendResult<()> {
        match self.current() {
            Token::Char(ch, _) if *ch == expected => {
                self.index += 1;
                Ok(())
            }
            _ => Err(parse_error(
                "html.parse.expected_char",
                format!("Expected '{expected}'."),
                self.current_span(),
            )),
        }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.index).unwrap_or(&Token::Eof)
    }

    fn current_char(&self) -> Option<char> {
        match self.current() {
            Token::Char(ch, _) => Some(*ch),
            _ => None,
        }
    }

    fn current_is_char(&self, expected: char) -> bool {
        self.current_char() == Some(expected)
    }

    fn current_is_whitespace(&self) -> bool {
        self.current_char().is_some_and(char::is_whitespace)
    }

    fn current_span(&self) -> Option<SourceSpan> {
        match self.current() {
            Token::Char(_, span) | Token::Interpolation(_, span) => span.clone(),
            Token::Eof => None,
        }
    }

    fn is_eof(&self) -> bool {
        matches!(self.current(), Token::Eof)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TagKind {
    Html,
    Component,
}

pub fn parse_template(template: &TemplateInput) -> BackendResult<Document> {
    Parser::new(template).parse_document()
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
    validate_html_document(&document)?;
    Ok(format_document_with_options(&document, options))
}

pub fn compile_template(template: &TemplateInput) -> BackendResult<CompiledHtmlTemplate> {
    let document = prepare_template(template)?;
    Ok(CompiledHtmlTemplate { document })
}

pub fn render_html(
    compiled: &CompiledHtmlTemplate,
    context: &RuntimeContext,
) -> BackendResult<String> {
    render_document(&compiled.document, context)
}

pub fn render_fragment(
    compiled: &CompiledHtmlTemplate,
    context: &RuntimeContext,
) -> BackendResult<RenderedFragment> {
    Ok(RenderedFragment {
        html: render_document(&compiled.document, context)?,
    })
}

pub fn format_template_syntax(template: &TemplateInput) -> BackendResult<Document> {
    require_raw_source(template)?;
    parse_template(template)
}

#[must_use]
pub fn format_document_with_options(document: &Document, options: &FormatOptions) -> String {
    formatter::format_document(document, options, FormatFlavor::Html)
}

#[must_use]
pub fn format_document_as_thtml_with_options(
    document: &Document,
    options: &FormatOptions,
) -> String {
    formatter::format_document(document, options, FormatFlavor::Thtml)
}

pub fn prepare_template(template: &TemplateInput) -> BackendResult<Document> {
    let document = parse_template(template)?;
    validate_html_document(&document)?;
    Ok(document)
}

pub fn rebind_document_interpolations(document: &mut Document, template: &TemplateInput) {
    for child in &mut document.children {
        rebind_node_interpolations(child, template);
    }
}

pub fn render_attributes_fragment(
    attributes: &[AttributeLike],
    context: &RuntimeContext,
) -> BackendResult<String> {
    let normalized = normalize_attributes(attributes, context)?;
    let mut out = String::new();
    write_attributes(&normalized, &mut out);
    Ok(out)
}

impl CompiledHtmlTemplate {
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }
}

pub fn static_key_parts(template: &TemplateInput) -> Vec<String> {
    let interpolation_count = template
        .segments
        .iter()
        .filter(|segment| matches!(segment, TemplateSegment::Interpolation(_)))
        .count();
    let mut parts = Vec::with_capacity(interpolation_count + 1);
    let mut current = String::new();
    let mut seen_any = false;

    for segment in &template.segments {
        match segment {
            TemplateSegment::StaticText(text) => {
                current.push_str(text);
                seen_any = true;
            }
            TemplateSegment::Interpolation(_) => {
                parts.push(std::mem::take(&mut current));
                seen_any = true;
            }
        }
    }
    if !seen_any {
        parts.push(String::new());
    } else {
        parts.push(current);
    }
    while parts.len() < interpolation_count + 1 {
        parts.push(String::new());
    }
    parts
}

fn classify_tag_name(name: &str) -> TagKind {
    if name.chars().next().is_some_and(char::is_uppercase) {
        TagKind::Component
    } else {
        TagKind::Html
    }
}

pub fn is_raw_text_tag(name: &str) -> bool {
    matches!(name, "script" | "style" | "title" | "textarea")
}

fn raw_text_allows_interpolation(name: &str) -> bool {
    name == "title"
}

fn validate_html_document(document: &Document) -> BackendResult<()> {
    for child in &document.children {
        validate_html_node(child)?;
    }
    Ok(())
}

fn rebind_node_interpolations(node: &mut Node, template: &TemplateInput) {
    match node {
        Node::Fragment(fragment) => {
            for child in &mut fragment.children {
                rebind_node_interpolations(child, template);
            }
        }
        Node::Element(element) => {
            rebind_attributes(&mut element.attributes, template);
            for child in &mut element.children {
                rebind_node_interpolations(child, template);
            }
        }
        Node::ComponentTag(component) => {
            rebind_attributes(&mut component.attributes, template);
            for child in &mut component.children {
                rebind_node_interpolations(child, template);
            }
        }
        Node::RawTextElement(element) => {
            rebind_attributes(&mut element.attributes, template);
            for child in &mut element.children {
                rebind_node_interpolations(child, template);
            }
        }
        Node::Interpolation(interpolation) => {
            rebind_interpolation(interpolation, template);
        }
        Node::Text(_) | Node::Comment(_) | Node::Doctype(_) => {}
    }
}

fn rebind_attributes(attributes: &mut [AttributeLike], template: &TemplateInput) {
    for attribute in attributes {
        match attribute {
            AttributeLike::Attribute(attribute) => {
                if let Some(value) = &mut attribute.value {
                    for part in &mut value.parts {
                        if let ValuePart::Interpolation(interpolation) = part {
                            rebind_interpolation(interpolation, template);
                        }
                    }
                }
            }
            AttributeLike::SpreadAttribute(attribute) => {
                rebind_interpolation(&mut attribute.interpolation, template);
            }
        }
    }
}

fn rebind_interpolation(interpolation: &mut InterpolationNode, template: &TemplateInput) {
    if let Some(source) = template.interpolation(interpolation.interpolation_index) {
        interpolation.expression = source.expression.clone();
        interpolation.raw_source = source.raw_source.clone();
        interpolation.conversion = source.conversion.clone();
        interpolation.format_spec = source.format_spec.clone();
    }
}

fn validate_html_node(node: &Node) -> BackendResult<()> {
    match node {
        Node::ComponentTag(component) => Err(semantic_error(
            "html.semantic.component_tag",
            format!(
                "Component tag <{}> is only valid in the T-HTML backend.",
                component.name
            ),
            component.span.clone(),
        )),
        Node::Element(element) => {
            validate_children(&element.children)?;
            validate_attributes(&element.attributes)?;
            Ok(())
        }
        Node::RawTextElement(element) => {
            validate_attributes(&element.attributes)?;
            validate_raw_text_children(element)
        }
        Node::Fragment(fragment) => validate_children(&fragment.children),
        _ => Ok(()),
    }
}

fn validate_raw_text_children(element: &RawTextElementNode) -> BackendResult<()> {
    for child in &element.children {
        match child {
            Node::Text(_) => {}
            Node::Interpolation(_) if raw_text_allows_interpolation(&element.name) => {}
            Node::Interpolation(interpolation) => {
                return Err(semantic_error(
                    "html.semantic.raw_text_interpolation",
                    format!("Interpolations are not allowed inside <{}>.", element.name),
                    interpolation.span.clone(),
                ));
            }
            _ => {
                let message = if raw_text_allows_interpolation(&element.name) {
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

fn validate_children(children: &[Node]) -> BackendResult<()> {
    for child in children {
        validate_html_node(child)?;
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

fn require_raw_source(template: &TemplateInput) -> BackendResult<()> {
    for segment in &template.segments {
        if let TemplateSegment::Interpolation(interpolation) = segment {
            if interpolation.raw_source.is_none() {
                return Err(semantic_error(
                    "html.format.raw_source_required",
                    format!(
                        "Formatting requires raw_source for interpolation '{}'.",
                        interpolation.expression_label()
                    ),
                    None,
                ));
            }
        }
    }
    Ok(())
}

fn render_document(document: &Document, context: &RuntimeContext) -> BackendResult<String> {
    let mut out = String::new();
    for child in &document.children {
        render_node(child, context, &mut out)?;
    }
    Ok(out)
}

fn render_node(node: &Node, context: &RuntimeContext, out: &mut String) -> BackendResult<()> {
    match node {
        Node::Text(text) => out.push_str(&escape_html_text(&text.value)),
        Node::Interpolation(interpolation) => {
            render_child_value(value_for_interpolation(context, interpolation)?, out)?
        }
        Node::Comment(comment) => {
            out.push_str("<!--");
            out.push_str(&comment.value);
            out.push_str("-->");
        }
        Node::Doctype(doctype) => {
            out.push('<');
            out.push('!');
            out.push_str(&doctype.value);
            out.push('>');
        }
        Node::Fragment(fragment) => {
            for child in &fragment.children {
                render_node(child, context, out)?;
            }
        }
        Node::Element(element) => render_html_element(
            &element.name,
            &element.attributes,
            &element.children,
            element.self_closing,
            context,
            out,
        )?,
        Node::RawTextElement(element) => render_raw_text_element(element, context, out)?,
        Node::ComponentTag(component) => {
            return Err(semantic_error(
                "html.semantic.component_render",
                format!(
                    "Component tag <{}> is only valid in the T-HTML backend.",
                    component.name
                ),
                component.span.clone(),
            ));
        }
    }
    Ok(())
}

fn render_raw_text_element(
    element: &RawTextElementNode,
    context: &RuntimeContext,
    out: &mut String,
) -> BackendResult<()> {
    out.push('<');
    out.push_str(&element.name);
    let normalized = normalize_attributes(&element.attributes, context)?;
    write_attributes(&normalized, out);
    out.push('>');
    for child in &element.children {
        match child {
            Node::Text(text) => out.push_str(&text.value),
            Node::Interpolation(interpolation) if raw_text_allows_interpolation(&element.name) => {
                render_escaped_text_value(value_for_interpolation(context, interpolation)?, out)?;
            }
            _ => {
                let message = if raw_text_allows_interpolation(&element.name) {
                    format!(
                        "Only text and interpolations can be rendered inside <{}>.",
                        element.name
                    )
                } else {
                    format!("Only text can be rendered inside <{}>.", element.name)
                };
                return Err(semantic_error(
                    "html.semantic.raw_text_render",
                    message,
                    element.span.clone(),
                ));
            }
        }
    }
    out.push_str("</");
    out.push_str(&element.name);
    out.push('>');
    Ok(())
}

fn render_html_element(
    name: &str,
    attributes: &[AttributeLike],
    children: &[Node],
    self_closing: bool,
    context: &RuntimeContext,
    out: &mut String,
) -> BackendResult<()> {
    out.push('<');
    out.push_str(name);
    let normalized = normalize_attributes(attributes, context)?;
    write_attributes(&normalized, out);
    if self_closing {
        out.push_str("/>");
        return Ok(());
    }
    out.push('>');
    for child in children {
        render_node(child, context, out)?;
    }
    out.push_str("</");
    out.push_str(name);
    out.push('>');
    Ok(())
}

#[derive(Default)]
struct NormalizedAttributes {
    order: Vec<String>,
    attrs: BTreeMap<String, Option<String>>,
    class_values: Vec<String>,
    saw_class: bool,
}

fn normalize_attributes(
    attributes: &[AttributeLike],
    context: &RuntimeContext,
) -> BackendResult<NormalizedAttributes> {
    let mut normalized = NormalizedAttributes::default();
    for attribute in attributes {
        match attribute {
            AttributeLike::Attribute(attribute) => {
                if attribute.name == "class" {
                    normalized.saw_class = true;
                    if !normalized.order.iter().any(|value| value == "class") {
                        normalized.order.push("class".to_string());
                    }
                    if let Some(value) = &attribute.value {
                        let rendered = render_attribute_value_parts(value, context, "class")?;
                        normalized.class_values.extend(rendered);
                    }
                    continue;
                }

                let rendered = render_attribute(attribute, context)?;
                if let Some(value) = rendered {
                    if !normalized
                        .order
                        .iter()
                        .any(|entry| entry == &attribute.name)
                    {
                        normalized.order.push(attribute.name.clone());
                    }
                    normalized.attrs.insert(attribute.name.clone(), value);
                }
            }
            AttributeLike::SpreadAttribute(attribute) => {
                apply_spread_attribute(&mut normalized, attribute, context)?
            }
        }
    }
    Ok(normalized)
}

fn render_attribute(
    attribute: &Attribute,
    context: &RuntimeContext,
) -> BackendResult<Option<Option<String>>> {
    match &attribute.value {
        None => Ok(Some(None)),
        Some(value) => {
            if value.parts.len() == 1
                && matches!(value.parts.first(), Some(ValuePart::Interpolation(_)))
            {
                let interpolation = match value.parts.first() {
                    Some(ValuePart::Interpolation(interpolation)) => interpolation,
                    _ => unreachable!(),
                };
                return match value_for_interpolation(context, interpolation)? {
                    RuntimeValue::Null => Ok(None),
                    RuntimeValue::Bool(false) => Ok(None),
                    RuntimeValue::Bool(true) => Ok(Some(None)),
                    other => Ok(Some(Some(escape_html_attribute(&stringify_runtime_value(
                        &other,
                    )?)))),
                };
            }
            let rendered = render_attribute_value_string(value, context, &attribute.name)?;
            Ok(Some(Some(escape_html_attribute(&rendered))))
        }
    }
}

fn apply_spread_attribute(
    normalized: &mut NormalizedAttributes,
    attribute: &SpreadAttribute,
    context: &RuntimeContext,
) -> BackendResult<()> {
    match value_for_interpolation(context, &attribute.interpolation)? {
        RuntimeValue::Attributes(entries) => {
            for (name, value) in entries {
                if name == "class" {
                    normalized.saw_class = true;
                    if !normalized.order.iter().any(|entry| entry == "class") {
                        normalized.order.push("class".to_string());
                    }
                    normalized
                        .class_values
                        .extend(normalize_class_value(&value)?);
                    continue;
                }
                match value {
                    RuntimeValue::Null | RuntimeValue::Bool(false) => {
                        normalized.attrs.remove(name.as_str());
                    }
                    RuntimeValue::Bool(true) => {
                        if !normalized.order.iter().any(|entry| entry == name) {
                            normalized.order.push(name.clone());
                        }
                        normalized.attrs.insert(name.clone(), None);
                    }
                    other => {
                        if !normalized.order.iter().any(|entry| entry == name) {
                            normalized.order.push(name.clone());
                        }
                        normalized.attrs.insert(
                            name.clone(),
                            Some(escape_html_attribute(&stringify_runtime_value_impl(
                                &other,
                            )?)),
                        );
                    }
                }
            }
            Ok(())
        }
        _ => Err(runtime_error(
            "html.runtime.spread_type",
            "Spread attributes require a mapping-like value.",
            attribute.span.clone(),
        )),
    }
}

fn write_attributes(normalized: &NormalizedAttributes, out: &mut String) {
    for name in &normalized.order {
        if name == "class" {
            if !normalized.class_values.is_empty() {
                out.push(' ');
                out.push_str("class=\"");
                out.push_str(&escape_html_attribute(&normalized.class_values.join(" ")));
                out.push('"');
            }
            continue;
        }
        if let Some(value) = normalized.attrs.get(name) {
            out.push(' ');
            out.push_str(name);
            if let Some(value) = value {
                out.push_str("=\"");
                out.push_str(value);
                out.push('"');
            }
        }
    }
}

pub fn render_child_value(value: &RuntimeValue, out: &mut String) -> BackendResult<()> {
    match value {
        RuntimeValue::Null => {}
        RuntimeValue::Bool(value) => out.push_str(&escape_html_text(&value.to_string())),
        RuntimeValue::Int(value) => out.push_str(&escape_html_text(&value.to_string())),
        RuntimeValue::Float(value) => out.push_str(&escape_html_text(&value.to_string())),
        RuntimeValue::String(value) => out.push_str(&escape_html_text(value)),
        RuntimeValue::RawHtml(value) => out.push_str(value),
        RuntimeValue::Fragment(values) | RuntimeValue::Sequence(values) => {
            for value in values {
                render_child_value(value, out)?;
            }
        }
        RuntimeValue::Attributes(_) => {
            return Err(runtime_error(
                "html.runtime.child_type",
                "Mapping-like values cannot be rendered as children.",
                None,
            ));
        }
    }
    Ok(())
}

fn render_escaped_text_value(value: &RuntimeValue, out: &mut String) -> BackendResult<()> {
    match value {
        RuntimeValue::Null => {}
        RuntimeValue::Bool(value) => out.push_str(&escape_html_text(&value.to_string())),
        RuntimeValue::Int(value) => out.push_str(&escape_html_text(&value.to_string())),
        RuntimeValue::Float(value) => out.push_str(&escape_html_text(&value.to_string())),
        RuntimeValue::String(value) => out.push_str(&escape_html_text(value)),
        RuntimeValue::RawHtml(value) => out.push_str(&escape_html_text(value)),
        RuntimeValue::Fragment(values) | RuntimeValue::Sequence(values) => {
            for value in values {
                render_escaped_text_value(value, out)?;
            }
        }
        RuntimeValue::Attributes(_) => {
            return Err(runtime_error(
                "html.runtime.child_type",
                "Mapping-like values cannot be rendered as children.",
                None,
            ));
        }
    }
    Ok(())
}

fn render_attribute_value_string(
    value: &AttributeValue,
    context: &RuntimeContext,
    name: &str,
) -> BackendResult<String> {
    let mut rendered = String::new();
    for part in &value.parts {
        match part {
            ValuePart::Text(text) => rendered.push_str(text),
            ValuePart::Interpolation(interpolation) => {
                if name == "class" {
                    let normalized =
                        normalize_class_value(value_for_interpolation(context, interpolation)?)?;
                    if !normalized.is_empty() {
                        if !rendered.is_empty() {
                            rendered.push(' ');
                        }
                        rendered.push_str(&normalized.join(" "));
                    }
                } else {
                    rendered.push_str(&stringify_runtime_value_impl(value_for_interpolation(
                        context,
                        interpolation,
                    )?)?);
                }
            }
        }
    }
    Ok(rendered)
}

fn render_attribute_value_parts(
    value: &AttributeValue,
    context: &RuntimeContext,
    name: &str,
) -> BackendResult<Vec<String>> {
    if name != "class" {
        return Ok(vec![render_attribute_value_string(value, context, name)?]);
    }

    let mut class_values = Vec::new();
    for part in &value.parts {
        match part {
            ValuePart::Text(text) => {
                class_values.extend(
                    text.split_ascii_whitespace()
                        .filter(|part| !part.is_empty())
                        .map(str::to_string),
                );
            }
            ValuePart::Interpolation(interpolation) => {
                class_values.extend(normalize_class_value(value_for_interpolation(
                    context,
                    interpolation,
                )?)?);
            }
        }
    }
    Ok(class_values)
}

fn normalize_class_value(value: &RuntimeValue) -> BackendResult<Vec<String>> {
    match value {
        RuntimeValue::Null => Ok(Vec::new()),
        RuntimeValue::Bool(false) => Ok(Vec::new()),
        RuntimeValue::Bool(true) => Err(runtime_error(
            "html.runtime.class_bool",
            "True is not a supported scalar class value.",
            None,
        )),
        RuntimeValue::String(value) => Ok(value
            .split_ascii_whitespace()
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect()),
        RuntimeValue::Sequence(values) | RuntimeValue::Fragment(values) => {
            let mut normalized = Vec::new();
            for value in values {
                normalized.extend(normalize_class_value(value)?);
            }
            Ok(normalized)
        }
        RuntimeValue::Attributes(entries) => Ok(entries
            .iter()
            .filter_map(|(name, value)| truthy_runtime_value(value).then_some(name.clone()))
            .collect()),
        RuntimeValue::Int(_) | RuntimeValue::Float(_) | RuntimeValue::RawHtml(_) => {
            Err(runtime_error(
                "html.runtime.class_type",
                "Unsupported class value type.",
                None,
            ))
        }
    }
}

fn truthy_runtime_value(value: &RuntimeValue) -> bool {
    match value {
        RuntimeValue::Null => false,
        RuntimeValue::Bool(value) => *value,
        RuntimeValue::Int(value) => *value != 0,
        RuntimeValue::Float(value) => *value != 0.0,
        RuntimeValue::String(value) => !value.is_empty(),
        RuntimeValue::Fragment(value) | RuntimeValue::Sequence(value) => !value.is_empty(),
        RuntimeValue::RawHtml(value) => !value.is_empty(),
        RuntimeValue::Attributes(value) => !value.is_empty(),
    }
}

fn value_for_interpolation<'a>(
    context: &'a RuntimeContext,
    interpolation: &InterpolationNode,
) -> BackendResult<&'a RuntimeValue> {
    context
        .values
        .get(interpolation.interpolation_index)
        .ok_or_else(|| {
            runtime_error(
                "html.runtime.missing_value",
                format!(
                    "Missing runtime value for interpolation '{}'.",
                    interpolation.expression
                ),
                interpolation.span.clone(),
            )
        })
}

fn stringify_runtime_value_impl(value: &RuntimeValue) -> BackendResult<String> {
    match value {
        RuntimeValue::Null => Ok(String::new()),
        RuntimeValue::Bool(value) => Ok(value.to_string()),
        RuntimeValue::Int(value) => Ok(value.to_string()),
        RuntimeValue::Float(value) => Ok(value.to_string()),
        RuntimeValue::String(value) => Ok(value.clone()),
        RuntimeValue::RawHtml(value) => Ok(value.clone()),
        RuntimeValue::Fragment(_) | RuntimeValue::Sequence(_) | RuntimeValue::Attributes(_) => {
            Err(runtime_error(
                "html.runtime.scalar_type",
                "Value cannot be stringified in this position.",
                None,
            ))
        }
    }
}

fn escape_html_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_html_attribute(value: &str) -> String {
    let mut out = String::new();
    let mut index = 0usize;

    while index < value.len() {
        let ch = value[index..]
            .chars()
            .next()
            .expect("valid character boundary");
        match ch {
            '&' => {
                if let Some(entity_len) = html_entity_len(&value[index..]) {
                    out.push_str(&value[index..index + entity_len]);
                    index += entity_len;
                } else {
                    out.push_str("&amp;");
                    index += 1;
                }
            }
            '<' => {
                out.push_str("&lt;");
                index += 1;
            }
            '>' => {
                out.push_str("&gt;");
                index += 1;
            }
            '"' => {
                out.push_str("&quot;");
                index += 1;
            }
            _ => {
                out.push(ch);
                index += ch.len_utf8();
            }
        }
    }

    out
}

fn html_entity_len(input: &str) -> Option<usize> {
    let bytes = input.as_bytes();
    if !bytes.starts_with(b"&") {
        return None;
    }

    let mut index = 1usize;
    if bytes.get(index) == Some(&b'#') {
        index += 1;
        if matches!(bytes.get(index), Some(b'x' | b'X')) {
            index += 1;
            let start = index;
            while bytes.get(index).is_some_and(u8::is_ascii_hexdigit) {
                index += 1;
            }
            if index == start || bytes.get(index) != Some(&b';') {
                return None;
            }
            return Some(index + 1);
        }

        let start = index;
        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
            index += 1;
        }
        if index == start || bytes.get(index) != Some(&b';') {
            return None;
        }
        return Some(index + 1);
    }

    let start = index;
    while bytes.get(index).is_some_and(u8::is_ascii_alphanumeric) {
        index += 1;
    }
    if index == start || bytes.get(index) != Some(&b';') {
        return None;
    }
    Some(index + 1)
}

fn flatten_input(template: &TemplateInput) -> Vec<StreamItem> {
    template.flatten()
}

fn merge_children_span(children: &[Node]) -> Option<SourceSpan> {
    let mut iter = children.iter().filter_map(node_span);
    let first = iter.next()?;
    Some(iter.fold(first, merge_span))
}

fn node_span(node: &Node) -> Option<SourceSpan> {
    match node {
        Node::Fragment(node) => node.span.clone(),
        Node::Element(node) => node.span.clone(),
        Node::ComponentTag(node) => node.span.clone(),
        Node::Text(node) => node.span.clone(),
        Node::Interpolation(node) => node.span.clone(),
        Node::Comment(node) => node.span.clone(),
        Node::Doctype(node) => node.span.clone(),
        Node::RawTextElement(node) => node.span.clone(),
    }
}

fn merge_span(left: SourceSpan, right: SourceSpan) -> SourceSpan {
    left.merge(&right)
}

fn merge_span_opt(left: Option<SourceSpan>, right: Option<SourceSpan>) -> Option<SourceSpan> {
    match (left, right) {
        (Some(left), Some(right)) => Some(merge_span(left, right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn is_name_char(value: char, is_start: bool) -> bool {
    if is_start {
        value.is_ascii_alphabetic() || value == '_'
    } else {
        value.is_ascii_alphanumeric() || matches!(value, '_' | '-' | ':' | '.')
    }
}

fn parse_error(
    code: impl Into<String>,
    message: impl Into<String>,
    span: Option<SourceSpan>,
) -> BackendError {
    BackendError::parse_at(code, message, span)
}

fn semantic_error(
    code: impl Into<String>,
    message: impl Into<String>,
    span: Option<SourceSpan>,
) -> BackendError {
    BackendError::semantic_at(code, message, span)
}

pub fn runtime_error(
    code: impl Into<String>,
    message: impl Into<String>,
    span: Option<SourceSpan>,
) -> BackendError {
    let message = message.into();
    BackendError {
        kind: ErrorKind::Semantic,
        message: message.clone(),
        diagnostics: vec![Diagnostic::error(code, message, span)],
    }
}

impl CompiledHtmlTemplate {
    #[must_use]
    pub fn from_document(document: Document) -> Self {
        Self { document }
    }
}

pub fn stringify_runtime_value(value: &RuntimeValue) -> BackendResult<String> {
    stringify_runtime_value_impl(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interpolation(index: usize, expression: &str, raw_source: Option<&str>) -> TemplateSegment {
        TemplateSegment::Interpolation(TemplateInterpolation {
            expression: expression.to_string(),
            conversion: None,
            format_spec: String::new(),
            interpolation_index: index,
            raw_source: raw_source.map(str::to_string),
        })
    }

    #[test]
    fn static_key_parts_preserve_empty_boundaries() {
        let input = TemplateInput::from_segments(vec![
            interpolation(0, "a", Some("{a}")),
            interpolation(1, "b", Some("{b}")),
        ]);
        assert_eq!(static_key_parts(&input), vec!["", "", ""]);
    }

    #[test]
    fn parse_and_render_html() {
        let input = TemplateInput::from_segments(vec![
            TemplateSegment::StaticText("<div class=\"hello ".to_string()),
            interpolation(0, "name", Some("{name}")),
            TemplateSegment::StaticText("\">".to_string()),
            interpolation(1, "content", Some("{content}")),
            TemplateSegment::StaticText("</div>".to_string()),
        ]);
        let compiled = compile_template(&input).expect("compile html template");
        let rendered = render_html(
            &compiled,
            &RuntimeContext {
                values: vec![
                    RuntimeValue::String("world".to_string()),
                    RuntimeValue::String("<safe>".to_string()),
                ],
            },
        )
        .expect("render html");
        assert_eq!(rendered, "<div class=\"hello world\">&lt;safe&gt;</div>");
    }

    #[test]
    fn html_backend_rejects_component_tags() {
        let input = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
            "<Button />".to_string(),
        )]);
        let err = check_template(&input).expect_err("component tags must fail");
        assert_eq!(err.kind, ErrorKind::Semantic);
    }

    #[test]
    fn format_requires_raw_source() {
        let input = TemplateInput::from_segments(vec![
            TemplateSegment::StaticText("<div>".to_string()),
            interpolation(0, "value", None),
            TemplateSegment::StaticText("</div>".to_string()),
        ]);
        let err = format_template(&input).expect_err("format requires raw source");
        assert_eq!(err.kind, ErrorKind::Semantic);
    }

    #[test]
    fn class_normalization_supports_mappings_and_sequences() {
        let values = normalize_class_value(&RuntimeValue::Sequence(vec![
            RuntimeValue::String("foo bar".to_string()),
            RuntimeValue::Attributes(vec![
                ("baz".to_string(), RuntimeValue::Bool(true)),
                ("skip".to_string(), RuntimeValue::Bool(false)),
            ]),
        ]))
        .expect("normalize class");
        assert_eq!(values, vec!["foo", "bar", "baz"]);
    }

    #[test]
    fn title_interpolation_is_allowed_and_escaped_on_render() {
        let input = TemplateInput::from_segments(vec![
            TemplateSegment::StaticText("<title>".to_string()),
            interpolation(0, "title", Some("{title}")),
            TemplateSegment::StaticText("</title>".to_string()),
        ]);
        let compiled = compile_template(&input).expect("compile title template");
        let rendered = render_html(
            &compiled,
            &RuntimeContext {
                values: vec![RuntimeValue::RawHtml("<safe>".to_string())],
            },
        )
        .expect("render title");
        assert_eq!(rendered, "<title>&lt;safe&gt;</title>");
    }

    #[test]
    fn script_interpolation_is_still_rejected() {
        let input = TemplateInput::from_segments(vec![
            TemplateSegment::StaticText("<script>".to_string()),
            interpolation(0, "script", Some("{script}")),
            TemplateSegment::StaticText("</script>".to_string()),
        ]);
        let err = check_template(&input).expect_err("script must still fail");
        assert_eq!(err.kind, ErrorKind::Semantic);
        assert!(err.message.contains("<script>"));
    }
}
