use tstring_format_doc::{Doc, RenderOptions, flat_width, has_forced_break, render};
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
    pub start_tag: InterpolationNode,
    pub end_tag: Option<InterpolationNode>,
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
    pub self_closing: bool,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttributeLike {
    LiteralAttribute(LiteralAttribute),
    InterpolatedAttribute(InterpolatedAttribute),
    TemplatedAttribute(TemplatedAttribute),
    SpreadAttribute(SpreadAttribute),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiteralAttribute {
    pub name: String,
    pub value: Option<String>,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterpolatedAttribute {
    pub name: String,
    pub interpolation: InterpolationNode,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TemplatedAttribute {
    pub name: String,
    pub parts: Vec<ValuePart>,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpreadAttribute {
    pub interpolation: InterpolationNode,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValuePart {
    Text(String),
    Interpolation(InterpolationNode),
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
    pub parts: Vec<ValuePart>,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DoctypeNode {
    pub text: String,
    pub span: Option<SourceSpan>,
}

#[derive(Clone, Debug)]
enum Token {
    Char(char, Option<SourceSpan>),
    Interpolation(TemplateInterpolation, Option<SourceSpan>),
    Eof,
}

#[derive(Clone, Debug)]
enum TagName {
    Literal(String),
    Component(InterpolationNode),
}

#[derive(Clone, Debug)]
enum OpenTag {
    Literal(String),
    Component(InterpolationNode),
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
        let (children, _) = self.parse_nodes(None, false)?;
        Ok(Document {
            span: merge_children_span(&children),
            children,
        })
    }

    fn parse_nodes(
        &mut self,
        closing_tag: Option<&OpenTag>,
        raw_text_mode: bool,
    ) -> BackendResult<(Vec<Node>, Option<InterpolationNode>)> {
        let mut children = Vec::new();
        loop {
            if self.is_eof() {
                if let Some(closing_tag) = closing_tag {
                    return Err(parse_error(
                        "tdom.parse.unclosed_tag",
                        match closing_tag {
                            OpenTag::Literal(name) => format!("Unclosed tag <{name}>."),
                            OpenTag::Component(tag) => format!(
                                "Unclosed component tag <{}>.",
                                tag.raw_source.clone().unwrap_or_else(|| "{}".to_string())
                            ),
                        },
                        self.current_span(),
                    ));
                }
                break;
            }

            if let Some(closing_tag) = closing_tag
                && self.starts_with_literal("</")
            {
                let end_tag = self.parse_close_tag(closing_tag)?;
                return Ok((children, end_tag));
            }

            if raw_text_mode
                && let Some(OpenTag::Literal(name)) = closing_tag
                && let Some(node) = self.parse_raw_text_chunk(name)?
            {
                children.push(node);
                continue;
            }

            if self.starts_with_literal("<!--") {
                children.push(Node::Comment(self.parse_comment()?));
                continue;
            }
            if self.starts_with_literal("<!") {
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
        Ok((children, None))
    }

    fn parse_raw_text_chunk(&mut self, closing_tag: &str) -> BackendResult<Option<Node>> {
        let mut text = String::new();
        let mut span = None;
        while !self.is_eof() {
            if self.starts_with_close_tag_ignore_ascii_case(closing_tag) {
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
        let mut parts = Vec::new();
        let mut text = String::new();
        while !self.is_eof() && !self.starts_with_literal("-->") {
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
        if !self.starts_with_literal("-->") {
            return Err(parse_error(
                "tdom.parse.comment_unclosed",
                "Unclosed HTML comment.",
                start,
            ));
        }
        self.consume_literal("-->");
        Ok(CommentNode {
            parts,
            span: merge_span_opt(start, self.previous_span()),
        })
    }

    fn parse_doctype(&mut self) -> BackendResult<DoctypeNode> {
        let start = self.current_span();
        self.expect_char('<')?;
        self.expect_char('!')?;
        let mut text = String::new();
        while !self.is_eof() {
            if self.current_is_char('>') {
                self.index += 1;
                break;
            }
            match self.current() {
                Token::Char(ch, _) => {
                    text.push(*ch);
                    self.index += 1;
                }
                Token::Interpolation(_, span) => {
                    return Err(parse_error(
                        "tdom.parse.doctype_interpolation",
                        "Interpolations are not allowed in declarations.",
                        span.clone(),
                    ));
                }
                Token::Eof => break,
            }
        }

        let trimmed = text.trim();
        let after_doctype = if let Some(value) = trimmed.strip_prefix("DOCTYPE ") {
            value
        } else if let Some(value) = trimmed.strip_prefix("doctype ") {
            value
        } else {
            return Err(parse_error(
                "tdom.parse.unknown_declaration",
                "Only well formed DOCTYPE declarations are supported.",
                start,
            ));
        };

        let payload = after_doctype.trim();
        if payload.is_empty() {
            return Err(parse_error(
                "tdom.parse.missing_declaration",
                "DOCTYPE declarations must include a target.",
                start,
            ));
        }

        Ok(DoctypeNode {
            text: payload.to_string(),
            span: merge_span_opt(start, self.previous_span()),
        })
    }

    fn parse_tag(&mut self) -> BackendResult<Node> {
        let start = self.current_span();
        self.expect_char('<')?;
        let tag = self.parse_tag_name()?;
        let mut attributes = Vec::new();

        loop {
            self.skip_whitespace();
            if self.starts_with_literal("/>") {
                self.consume_literal("/>");
                return Ok(self.build_tag_node(tag, attributes, Vec::new(), true, start));
            }
            if self.current_is_char('>') {
                self.index += 1;
                break;
            }
            if self.is_eof() {
                return Err(parse_error(
                    "tdom.parse.unclosed_start_tag",
                    "Unclosed start tag.",
                    start,
                ));
            }
            attributes.push(self.parse_attribute_like()?);
        }

        if let TagName::Literal(name) = &tag
            && is_void_html_tag(name)
        {
            return Ok(self.build_tag_node(tag, attributes, Vec::new(), true, start));
        }

        let open_tag = match &tag {
            TagName::Literal(name) => OpenTag::Literal(name.clone()),
            TagName::Component(interpolation) => OpenTag::Component(interpolation.clone()),
        };
        let raw_text_mode = matches!(&tag, TagName::Literal(name) if is_raw_text_tag(name));
        let (children, end_tag) = self.parse_nodes(Some(&open_tag), raw_text_mode)?;
        Ok(self.build_tag_node_with_end(tag, attributes, children, false, start, end_tag))
    }

    fn parse_close_tag(&mut self, open_tag: &OpenTag) -> BackendResult<Option<InterpolationNode>> {
        let start = self.current_span();
        self.consume_literal("</");
        self.skip_whitespace();
        let close_tag = self.parse_tag_name()?;
        self.skip_whitespace();
        self.expect_char('>')?;

        match (open_tag, close_tag) {
            (OpenTag::Literal(expected), TagName::Literal(actual))
                if expected.eq_ignore_ascii_case(&actual) =>
            {
                Ok(None)
            }
            (OpenTag::Literal(expected), TagName::Literal(actual)) => Err(parse_error(
                "tdom.parse.mismatched_tag",
                format!("Mismatched closing tag </{actual}>. Expected </{expected}>."),
                start,
            )),
            (OpenTag::Literal(expected), TagName::Component(_)) => Err(parse_error(
                "tdom.parse.component_end_tag_for_element",
                format!("Component closing tag found for element <{expected}>."),
                start,
            )),
            (OpenTag::Component(open), TagName::Literal(actual)) => Err(parse_error(
                "tdom.parse.literal_end_tag_for_component",
                format!(
                    "Mismatched closing tag </{actual}> for component starting at {}.",
                    open.raw_source.clone().unwrap_or_else(|| "{}".to_string())
                ),
                start,
            )),
            (OpenTag::Component(open), TagName::Component(close)) => {
                if interpolation_matches(open, &close) {
                    Ok(Some(close))
                } else {
                    Err(parse_error(
                        "tdom.parse.mismatched_component_tag",
                        "Mismatched component start and end callables.",
                        start,
                    ))
                }
            }
        }
    }

    fn parse_tag_name(&mut self) -> BackendResult<TagName> {
        if let Some(interpolation) = self.take_interpolation() {
            self.ensure_boundary_after_singleton_interpolation(
                "Component element tags must have exactly one interpolation.",
            )?;
            return Ok(TagName::Component(interpolation));
        }

        Ok(TagName::Literal(self.parse_name()?.to_ascii_lowercase()))
    }

    fn parse_attribute_like(&mut self) -> BackendResult<AttributeLike> {
        if let Some(interpolation) = self.take_interpolation() {
            self.ensure_boundary_after_singleton_interpolation(
                "Spread attributes must have exactly one interpolation in the name.",
            )?;
            let span = interpolation.span.clone();
            return Ok(AttributeLike::SpreadAttribute(SpreadAttribute {
                interpolation,
                span,
            }));
        }

        let span = self.current_span();
        let name = self.parse_name()?.to_ascii_lowercase();
        self.skip_whitespace();
        if !self.current_is_char('=') {
            return Ok(AttributeLike::LiteralAttribute(LiteralAttribute {
                name,
                value: None,
                span,
            }));
        }
        self.index += 1;
        self.skip_whitespace();
        self.parse_attribute_value(name, span)
    }

    fn parse_attribute_value(
        &mut self,
        name: String,
        span: Option<SourceSpan>,
    ) -> BackendResult<AttributeLike> {
        let parts = if self.current_is_char('"') || self.current_is_char('\'') {
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
            parts
        } else {
            let mut parts = Vec::new();
            let mut text = String::new();
            while !self.is_eof() {
                if self.current_is_whitespace()
                    || self.current_is_char('>')
                    || self.starts_with_literal("/>")
                {
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
            parts
        };

        Ok(match parts.as_slice() {
            [ValuePart::Text(text)] => AttributeLike::LiteralAttribute(LiteralAttribute {
                name,
                value: Some(text.clone()),
                span,
            }),
            [ValuePart::Interpolation(interpolation)] => {
                AttributeLike::InterpolatedAttribute(InterpolatedAttribute {
                    name,
                    interpolation: interpolation.clone(),
                    span,
                })
            }
            _ => AttributeLike::TemplatedAttribute(TemplatedAttribute { name, parts, span }),
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
                "tdom.parse.expected_name",
                "Expected a tag or attribute name.",
                self.current_span(),
            ))
        } else {
            Ok(name)
        }
    }

    fn build_tag_node(
        &self,
        tag: TagName,
        attributes: Vec<AttributeLike>,
        children: Vec<Node>,
        self_closing: bool,
        start: Option<SourceSpan>,
    ) -> Node {
        self.build_tag_node_with_end(tag, attributes, children, self_closing, start, None)
    }

    fn build_tag_node_with_end(
        &self,
        tag: TagName,
        attributes: Vec<AttributeLike>,
        children: Vec<Node>,
        self_closing: bool,
        start: Option<SourceSpan>,
        end_tag: Option<InterpolationNode>,
    ) -> Node {
        let span = merge_span_opt(start, merge_children_span(&children));
        match tag {
            TagName::Literal(name) if is_raw_text_tag(&name) => {
                Node::RawTextElement(RawTextElementNode {
                    name,
                    attributes,
                    children,
                    self_closing,
                    span,
                })
            }
            TagName::Literal(name) => Node::Element(ElementNode {
                name,
                attributes,
                children,
                self_closing,
                span,
            }),
            TagName::Component(start_tag) => Node::ComponentTag(ComponentTagNode {
                start_tag,
                end_tag,
                attributes,
                children,
                self_closing,
                span,
            }),
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

    fn ensure_boundary_after_singleton_interpolation(&self, message: &str) -> BackendResult<()> {
        if matches!(self.current(), Token::Interpolation(_, _))
            || self
                .current_char()
                .is_some_and(|ch| is_name_char(ch, false))
        {
            return Err(parse_error(
                "tdom.parse.singleton_interpolation_required",
                message,
                self.current_span(),
            ));
        }
        Ok(())
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

    fn starts_with_close_tag_ignore_ascii_case(&self, name: &str) -> bool {
        let literal = format!("</{name}");
        for (offset, expected) in literal.chars().enumerate() {
            match self.tokens.get(self.index + offset) {
                Some(Token::Char(ch, _)) if ch.eq_ignore_ascii_case(&expected) => {}
                _ => return false,
            }
        }
        match self.tokens.get(self.index + literal.len()) {
            Some(Token::Char('>', _)) => true,
            Some(Token::Char(ch, _)) => ch.is_whitespace(),
            _ => false,
        }
    }

    fn consume_literal(&mut self, literal: &str) {
        for _ in literal.chars() {
            self.index += 1;
        }
    }

    fn expect_char(&mut self, expected: char) -> BackendResult<()> {
        match self.current() {
            Token::Char(ch, _) if *ch == expected => {
                self.index += 1;
                Ok(())
            }
            _ => Err(parse_error(
                "tdom.parse.expected_char",
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

    fn previous_span(&self) -> Option<SourceSpan> {
        match self
            .index
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
        {
            Some(Token::Char(_, span) | Token::Interpolation(_, span)) => span.clone(),
            _ => None,
        }
    }

    fn is_eof(&self) -> bool {
        matches!(self.current(), Token::Eof)
    }
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
    require_raw_source(template)?;
    let document = prepare_template(template)?;
    Ok(format_document(&document, options))
}

pub fn prepare_template(template: &TemplateInput) -> BackendResult<Document> {
    let document = parse_template(template)?;
    validate_document(&document)?;
    Ok(document)
}

fn validate_document(document: &Document) -> BackendResult<()> {
    for child in &document.children {
        validate_node(child)?;
    }
    Ok(())
}

fn validate_node(node: &Node) -> BackendResult<()> {
    match node {
        Node::Fragment(fragment) => {
            for child in &fragment.children {
                validate_node(child)?;
            }
        }
        Node::Element(element) => {
            for child in &element.children {
                validate_node(child)?;
            }
        }
        Node::ComponentTag(component) => {
            for child in &component.children {
                validate_node(child)?;
            }
        }
        Node::RawTextElement(element) => {
            for child in &element.children {
                match child {
                    Node::Text(_) | Node::Interpolation(_) => {}
                    _ => {
                        return Err(semantic_error(
                            "tdom.semantic.raw_text_content",
                            format!(
                                "Only text-like content is allowed inside <{}>.",
                                element.name
                            ),
                            element.span.clone(),
                        ));
                    }
                }
            }
        }
        Node::Text(_) | Node::Interpolation(_) | Node::Comment(_) | Node::Doctype(_) => {}
    }
    Ok(())
}

fn require_raw_source(template: &TemplateInput) -> BackendResult<()> {
    for segment in &template.segments {
        if let TemplateSegment::Interpolation(interpolation) = segment
            && interpolation.raw_source.is_none()
        {
            return Err(semantic_error(
                "tdom.format.raw_source_required",
                format!(
                    "Formatting requires raw_source for interpolation '{}'.",
                    interpolation.expression_label()
                ),
                None,
            ));
        }
    }
    Ok(())
}

fn format_document(document: &Document, options: &FormatOptions) -> String {
    let doc = build_nodes(&document.children, options);
    render(
        &doc,
        RenderOptions {
            line_length: options.line_length.max(1),
            indent_width: 2,
        },
    )
}

fn build_nodes(nodes: &[Node], options: &FormatOptions) -> Doc {
    Doc::concat(nodes.iter().map(|node| build_node(node, options)).collect())
}

fn build_node(node: &Node, options: &FormatOptions) -> Doc {
    match node {
        Node::Fragment(fragment) => build_nodes(&fragment.children, options),
        Node::Element(element) => build_element(element, options),
        Node::ComponentTag(component) => build_component(component, options),
        Node::Text(text) => Doc::text(text.value.clone()),
        Node::Interpolation(interpolation) => build_interpolation(interpolation),
        Node::Comment(comment) => build_comment(comment),
        Node::Doctype(doctype) => build_doctype(doctype),
        Node::RawTextElement(element) => build_raw_text_element(element, options),
    }
}

fn build_element(element: &ElementNode, options: &FormatOptions) -> Doc {
    let is_void = is_void_html_tag(&element.name) && element.children.is_empty();
    let closing = if is_void || element.self_closing {
        " />"
    } else {
        ">"
    };
    let start = build_start_tag(
        Doc::text(element.name.clone()),
        &element.attributes,
        closing,
    );
    if is_void || element.self_closing {
        return start;
    }
    build_standard_element(
        start,
        close_literal_tag(&element.name),
        &element.children,
        options,
    )
}

fn build_component(component: &ComponentTagNode, options: &FormatOptions) -> Doc {
    let closing = if component.self_closing && component.children.is_empty() {
        " />"
    } else {
        ">"
    };
    let start = build_start_tag(
        build_interpolation(&component.start_tag),
        &component.attributes,
        closing,
    );
    if component.self_closing && component.children.is_empty() {
        return start;
    }
    let close = close_component_tag(component.end_tag.as_ref().unwrap_or(&component.start_tag));
    build_standard_element(start, close, &component.children, options)
}

fn build_raw_text_element(element: &RawTextElementNode, options: &FormatOptions) -> Doc {
    let closing = if element.self_closing { " />" } else { ">" };
    let start = build_start_tag(
        Doc::text(element.name.clone()),
        &element.attributes,
        closing,
    );
    if element.self_closing {
        return start;
    }
    let children = build_nodes(&element.children, options);
    Doc::concat(vec![start, children, close_literal_tag(&element.name)])
}

fn build_standard_element(
    start: Doc,
    close: Doc,
    children: &[Node],
    options: &FormatOptions,
) -> Doc {
    if children.is_empty() {
        return Doc::concat(vec![start, close]);
    }

    if is_mixed_content(children) {
        return Doc::concat(vec![start, build_nodes(children, options), close]);
    }

    let significant_children = strip_padding_whitespace(children);
    if significant_children.is_empty() {
        return Doc::concat(vec![start, close]);
    }

    let child_doc = build_nodes(&significant_children, options);
    let inline_doc = Doc::concat(vec![start.clone(), child_doc.clone(), close.clone()]);
    if flat_width(&inline_doc).is_some_and(|width| width <= options.line_length.max(1))
        && !has_forced_break(&child_doc)
    {
        return inline_doc;
    }

    let broken_children = join_with_hard_lines(
        significant_children
            .iter()
            .map(|child| build_node(child, options))
            .collect(),
    );

    Doc::concat(vec![
        start,
        Doc::concat(vec![Doc::hard_line(), broken_children]).indent(),
        Doc::hard_line(),
        close,
    ])
}

fn build_start_tag(name: Doc, attributes: &[AttributeLike], closing: &str) -> Doc {
    let mut parts = vec![Doc::text("<".to_string()), name];
    if !attributes.is_empty() {
        let attr_lines = attributes
            .iter()
            .flat_map(|attribute| [Doc::line(), build_attribute_like(attribute)])
            .collect();
        parts.push(Doc::concat(attr_lines).indent());
        parts.push(Doc::soft_line());
    }
    parts.push(Doc::text(closing.to_string()));
    Doc::concat(parts).group()
}

fn build_attribute_like(attribute: &AttributeLike) -> Doc {
    match attribute {
        AttributeLike::LiteralAttribute(attribute) => {
            if let Some(value) = &attribute.value {
                Doc::text(format!(
                    "{}=\"{}\"",
                    attribute.name,
                    escape_attribute_text(value)
                ))
            } else {
                Doc::text(attribute.name.clone())
            }
        }
        AttributeLike::InterpolatedAttribute(attribute) => Doc::concat(vec![
            Doc::text(format!("{}=", attribute.name)),
            build_interpolation(&attribute.interpolation),
        ]),
        AttributeLike::TemplatedAttribute(attribute) => {
            let mut parts = vec![Doc::text(format!("{}=\"", attribute.name))];
            for part in &attribute.parts {
                parts.push(match part {
                    ValuePart::Text(text) => Doc::text(escape_attribute_text(text)),
                    ValuePart::Interpolation(interpolation) => build_interpolation(interpolation),
                });
            }
            parts.push(Doc::text("\"".to_string()));
            Doc::concat(parts)
        }
        AttributeLike::SpreadAttribute(attribute) => build_interpolation(&attribute.interpolation),
    }
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
    let mut parts = vec![Doc::text("<!--".to_string())];
    for part in &comment.parts {
        parts.push(match part {
            ValuePart::Text(text) => Doc::text(text.clone()),
            ValuePart::Interpolation(interpolation) => build_interpolation(interpolation),
        });
    }
    parts.push(Doc::text("-->".to_string()));
    Doc::concat(parts)
}

fn build_doctype(doctype: &DoctypeNode) -> Doc {
    Doc::text(format!("<!DOCTYPE {}>", doctype.text))
}

fn close_literal_tag(name: &str) -> Doc {
    Doc::text(format!("</{name}>"))
}

fn close_component_tag(interpolation: &InterpolationNode) -> Doc {
    Doc::concat(vec![
        Doc::text("</".to_string()),
        build_interpolation(interpolation),
        Doc::text(">".to_string()),
    ])
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

fn interpolation_matches(left: &InterpolationNode, right: &InterpolationNode) -> bool {
    left.raw_source == right.raw_source && left.expression == right.expression
}

fn is_raw_text_tag(name: &str) -> bool {
    matches!(name, "script" | "style" | "title" | "textarea")
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

fn escape_attribute_text(text: &str) -> String {
    text.replace('"', "&quot;")
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
    let message = message.into();
    BackendError {
        kind: ErrorKind::Semantic,
        message: message.clone(),
        diagnostics: vec![Diagnostic::error(code, message, span)],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn tdom_accepts_component_tags_and_preserves_end_tag_interpolation() {
        let template = TemplateInput::from_segments(vec![
            TemplateSegment::StaticText("<".to_owned()),
            interpolation(0, "Card", "{Card}"),
            TemplateSegment::StaticText(" title=".to_owned()),
            interpolation(1, "title", "{title}"),
            TemplateSegment::StaticText("></".to_owned()),
            interpolation(2, "Card", "{Card}"),
            TemplateSegment::StaticText(">".to_owned()),
        ]);

        let document = prepare_template(&template).expect("tdom should parse");
        let Node::ComponentTag(component) = &document.children[0] else {
            panic!("expected component tag");
        };
        assert_eq!(component.start_tag.interpolation_index, 0);
        assert_eq!(component.end_tag.as_ref().unwrap().interpolation_index, 2);
    }

    #[test]
    fn tdom_rejects_non_singleton_component_tag_names() {
        let template = TemplateInput::from_segments(vec![
            TemplateSegment::StaticText("<".to_owned()),
            interpolation(0, "Card", "{Card}"),
            interpolation(1, "Extra", "{Extra}"),
            TemplateSegment::StaticText(" />".to_owned()),
        ]);

        assert!(check_template(&template).is_err());
    }

    #[test]
    fn tdom_rejects_mismatched_component_tag_pairs() {
        let template = TemplateInput::from_segments(vec![
            TemplateSegment::StaticText("<".to_owned()),
            interpolation(0, "Card", "{Card}"),
            TemplateSegment::StaticText("></".to_owned()),
            interpolation(1, "Other", "{Other}"),
            TemplateSegment::StaticText(">".to_owned()),
        ]);

        let error = check_template(&template).expect_err("must fail");
        assert!(
            error
                .message
                .contains("Mismatched component start and end callables")
        );
    }

    #[test]
    fn tdom_formats_comments_and_preserves_raw_source() {
        let template = TemplateInput::from_segments(vec![
            TemplateSegment::StaticText("<!-- hello ".to_owned()),
            TemplateSegment::Interpolation(TemplateInterpolation {
                expression: "value".to_owned(),
                conversion: Some("r".to_owned()),
                format_spec: "safe".to_owned(),
                interpolation_index: 0,
                raw_source: Some("{value!r:safe}".to_owned()),
            }),
            TemplateSegment::StaticText(" -->".to_owned()),
        ]);

        assert_eq!(
            format_template(&template).expect("format comment"),
            "<!-- hello {value!r:safe} -->"
        );
    }

    #[test]
    fn tdom_rejects_interpolated_and_unknown_doctypes() {
        let interpolated = TemplateInput::from_segments(vec![
            TemplateSegment::StaticText("<!DOCTYPE ".to_owned()),
            interpolation(0, "kind", "{kind}"),
            TemplateSegment::StaticText(">".to_owned()),
        ]);
        assert!(check_template(&interpolated).is_err());

        let unknown = TemplateInput::from_segments(vec![TemplateSegment::StaticText(
            "<!doctype-alt html>".to_owned(),
        )]);
        assert!(check_template(&unknown).is_err());
    }

    #[test]
    fn tdom_formats_void_tags_with_canonical_lowercase_output() {
        let template =
            TemplateInput::from_segments(vec![TemplateSegment::StaticText("<BR>".to_owned())]);

        assert_eq!(format_template(&template).expect("format br"), "<br />");
    }

    #[test]
    fn tdom_formats_raw_text_title_and_textarea_content() {
        let template = TemplateInput::from_segments(vec![
            TemplateSegment::StaticText("<title>".to_owned()),
            interpolation(0, "title", "{title}"),
            TemplateSegment::StaticText("</title><textarea>".to_owned()),
            interpolation(1, "value", "{value}"),
            TemplateSegment::StaticText("</textarea>".to_owned()),
        ]);

        assert_eq!(
            format_template(&template).expect("format raw text"),
            "<title>{title}</title><textarea>{value}</textarea>"
        );
    }
}
