//! Comark extension types, parsers, and HTML renderers.
//!
//! This module provides the custom Markdown extensions used by OxydeMark:
//! - **Block components**: `::name{attrs}\ncontent\n::`
//! - **Inline components**: `:name[content]{attrs}`
//! - **Span attributes**: `[text]{.class #id key="val"}`
//!
//! Each extension includes an AST node type, a rushdown parser extension,
//! and a rushdown HTML renderer extension.

use std::fmt;

use rushdown::ast::{self, Arena, KindData, NodeKind, NodeRef, NodeType, PrettyPrint, pp_indent};
use rushdown::parser::{
    self, BlockParser, Context, InlineParser, NoParserOptions, Parser, ParserExtension, State,
};
use rushdown::renderer::html;
use rushdown::renderer::{
    self as renderer, BoxRenderNode, NoRendererOptions, NodeRenderer, NodeRendererRegistry,
    RenderNode, TextWrite,
};
use rushdown::text::{self, Reader};

use crate::html_render::html_escape;

// ---------------------------------------------------------------------------
// Extension node types
// ---------------------------------------------------------------------------

/// A block component node: `::name{attrs}\ncontent\n::`.
///
/// Represents a container block introduced by `::component_name{attributes}`.
/// The component body runs until a closing `::` line. All component semantics
/// (name, attributes) are carried in the AST for Python plugins to consume.
#[derive(Debug)]
pub(crate) struct BlockComponent {
    pub(crate) name: String,
}

impl NodeKind for BlockComponent {
    fn typ(&self) -> NodeType {
        NodeType::ContainerBlock
    }

    fn kind_name(&self) -> &'static str {
        "BlockComponent"
    }
}

impl PrettyPrint for BlockComponent {
    fn pretty_print(&self, w: &mut dyn fmt::Write, _source: &str, level: usize) -> fmt::Result {
        writeln!(w, "{}name: {}", pp_indent(level), self.name)
    }
}

impl From<BlockComponent> for KindData {
    fn from(e: BlockComponent) -> Self {
        KindData::Extension(Box::new(e))
    }
}

/// An inline component node: `:name[content]{attrs}`.
///
/// Represents an inline element introduced by `:component_name[label]{attrs}`.
/// The label content is parsed as inline Markdown children.
#[derive(Debug)]
pub(crate) struct InlineComponent {
    pub(crate) name: String,
}

impl NodeKind for InlineComponent {
    fn typ(&self) -> NodeType {
        NodeType::Inline
    }

    fn kind_name(&self) -> &'static str {
        "InlineComponent"
    }
}

impl PrettyPrint for InlineComponent {
    fn pretty_print(&self, w: &mut dyn fmt::Write, _source: &str, level: usize) -> fmt::Result {
        writeln!(w, "{}name: {}", pp_indent(level), self.name)
    }
}

impl From<InlineComponent> for KindData {
    fn from(e: InlineComponent) -> Self {
        KindData::Extension(Box::new(e))
    }
}

/// A span attribute node: `[text]{.class #id key="val"}`.
///
/// Wraps inline content with HTML attributes. This is a pure wrapper node
/// whose rendering semantics depend on its attributes.
#[derive(Debug)]
pub(crate) struct SpanAttributes;

impl NodeKind for SpanAttributes {
    fn typ(&self) -> NodeType {
        NodeType::Inline
    }

    fn kind_name(&self) -> &'static str {
        "SpanAttributes"
    }
}

impl PrettyPrint for SpanAttributes {
    fn pretty_print(&self, _w: &mut dyn fmt::Write, _source: &str, _level: usize) -> fmt::Result {
        Ok(())
    }
}

impl From<SpanAttributes> for KindData {
    fn from(e: SpanAttributes) -> Self {
        KindData::Extension(Box::new(e))
    }
}

// ---------------------------------------------------------------------------
// Block component parser
// ---------------------------------------------------------------------------

/// Parses block components: `::name{attrs}\ncontent\n::`.
#[derive(Debug)]
struct BlockComponentParser;

impl BlockParser for BlockComponentParser {
    fn trigger(&self) -> &[u8] {
        b":"
    }

    fn open(
        &self,
        arena: &mut Arena,
        _parent_ref: NodeRef,
        reader: &mut text::BasicReader,
        _ctx: &mut Context,
    ) -> Option<(NodeRef, State)> {
        let (line_bytes, _seg) = reader.peek_line_bytes()?;
        let line = std::str::from_utf8(&line_bytes).ok()?;
        let trimmed = line.trim();

        // Must start with :: but NOT ::: (Phase 3 nesting).
        if !trimmed.starts_with("::") || trimmed.starts_with(":::") {
            return None;
        }

        let rest = trimmed[2..].trim();
        if rest.is_empty() {
            // This is a bare `::` which is a closing marker, not an opening.
            return None;
        }

        // Parse name and optional attributes: `name{attrs}` or just `name`.
        let (name, attr_str) = if let Some(brace_start) = rest.find('{') {
            if !rest.ends_with('}') {
                return None;
            }
            let name = rest[..brace_start].trim();
            let attrs = &rest[brace_start + 1..rest.len() - 1];
            (name, Some(attrs))
        } else {
            (rest, None)
        };

        // Name must be non-empty and alphanumeric (with hyphens/underscores).
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return None;
        }

        let node_ref = arena.new_node(BlockComponent {
            name: name.to_string(),
        });

        // Parse and attach attributes.
        if let Some(attrs) = attr_str {
            parse_component_attributes(attrs, &mut arena[node_ref]);
        }

        reader.advance_line();
        Some((node_ref, State::HAS_CHILDREN))
    }

    fn cont(
        &self,
        _arena: &mut Arena,
        _node_ref: NodeRef,
        reader: &mut text::BasicReader,
        _ctx: &mut Context,
    ) -> Option<State> {
        let (line_bytes, _seg) = reader.peek_line_bytes()?;
        let line = std::str::from_utf8(&line_bytes).ok()?;
        let trimmed = line.trim();

        // Closing marker: a line that is exactly `::`.
        if trimmed == "::" {
            reader.advance_line();
            return None; // Close this block.
        }

        // Continue accepting children.
        Some(State::HAS_CHILDREN)
    }

    fn can_interrupt_paragraph(&self) -> bool {
        true
    }
}

/// Parser extension function for block components.
pub(crate) fn block_component_parser_extension() -> impl ParserExtension {
    parser::ParserExtensionFn::new(|parser: &mut Parser| {
        parser.add_block_parser(
            || Box::new(BlockComponentParser) as Box<dyn BlockParser>,
            NoParserOptions,
            650,
        );
    })
}

// ---------------------------------------------------------------------------
// Inline component parser
// ---------------------------------------------------------------------------

/// Parses inline components: `:name[content]{attrs}`.
#[derive(Debug)]
struct InlineComponentParser;

impl InlineParser for InlineComponentParser {
    fn trigger(&self) -> &[u8] {
        b":"
    }

    fn parse(
        &self,
        arena: &mut Arena,
        _parent_ref: NodeRef,
        reader: &mut text::BlockReader,
        _ctx: &mut Context,
    ) -> Option<NodeRef> {
        let start_pos = reader.position();

        // We're triggered on ':'. Check it's a single colon (not :: for block).
        if reader.peek_byte() != b':' {
            return None;
        }
        reader.advance(1);

        // Next char must NOT be ':' (that would be a block component `::`)
        // and must be alphanumeric (start of component name).
        let next = reader.peek_byte();
        if next == b':' || next == text::EOS || !is_name_start(next) {
            reader.set_position(start_pos.0, start_pos.1);
            return None;
        }

        // Read the component name.
        let name_start = reader.position();
        while {
            let b = reader.peek_byte();
            b != text::EOS && is_name_char(b)
        } {
            reader.advance(1);
        }

        let name_end = reader.position();
        let name = &reader.source()[name_start.1.start()..name_end.1.start()];
        if name.is_empty() {
            reader.set_position(start_pos.0, start_pos.1);
            return None;
        }

        // Expect '[' for content (optional: if no '[', we still accept as an
        // empty inline component like `:icon{name="star"}`).
        let has_bracket = reader.peek_byte() == b'[';

        let node_ref = arena.new_node(InlineComponent {
            name: name.to_string(),
        });

        if has_bracket {
            reader.advance(1); // skip '['

            // Read content until matching ']', handling nested brackets.
            let mut depth: usize = 1;
            let content_start = reader.position();
            loop {
                let b = reader.peek_byte();
                if b == text::EOS {
                    // Unclosed bracket -- abort.
                    reader.set_position(start_pos.0, start_pos.1);
                    return None;
                }
                if b == b'[' {
                    depth += 1;
                } else if b == b']' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                reader.advance(1);
            }

            // Extract content text and create a text child node.
            let content_end = reader.position();
            let content = &reader.source()[content_start.1.start()..content_end.1.start()];
            if !content.is_empty() {
                let text_ref = arena.new_node(ast::Text::new(text::Segment::new(
                    content_start.1.start(),
                    content_end.1.start(),
                )));
                node_ref.append_child(arena, text_ref);
            }

            reader.advance(1); // skip ']'
        }

        // Optional attributes: `{attrs}`.
        if reader.peek_byte() == b'{' {
            reader.advance(1);
            let attr_start = reader.position();
            let mut brace_depth: usize = 1;
            loop {
                let b = reader.peek_byte();
                if b == text::EOS {
                    reader.set_position(start_pos.0, start_pos.1);
                    return None;
                }
                if b == b'{' {
                    brace_depth += 1;
                } else if b == b'}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        break;
                    }
                }
                reader.advance(1);
            }
            let attr_end = reader.position();
            let attr_str = &reader.source()[attr_start.1.start()..attr_end.1.start()];
            parse_component_attributes(attr_str, &mut arena[node_ref]);
            reader.advance(1); // skip '}'
        } else if !has_bracket {
            // Must have at least bracket or attributes.
            reader.set_position(start_pos.0, start_pos.1);
            return None;
        }

        Some(node_ref)
    }
}

/// Parser extension function for inline components.
pub(crate) fn inline_component_parser_extension() -> impl ParserExtension {
    parser::ParserExtensionFn::new(|parser: &mut Parser| {
        parser.add_inline_parser(
            || Box::new(InlineComponentParser) as Box<dyn InlineParser>,
            NoParserOptions,
            450,
        );
    })
}

// ---------------------------------------------------------------------------
// Span attribute parser
// ---------------------------------------------------------------------------

/// Parses span attributes: `[text]{.class #id key="val"}`.
#[derive(Debug)]
struct SpanAttributeParser;

impl InlineParser for SpanAttributeParser {
    fn trigger(&self) -> &[u8] {
        b"["
    }

    fn parse(
        &self,
        arena: &mut Arena,
        _parent_ref: NodeRef,
        reader: &mut text::BlockReader,
        _ctx: &mut Context,
    ) -> Option<NodeRef> {
        let start_pos = reader.position();

        if reader.peek_byte() != b'[' {
            return None;
        }
        reader.advance(1);

        // Read content until matching ']'.
        let mut depth: usize = 1;
        let content_start = reader.position();
        loop {
            let b = reader.peek_byte();
            if b == text::EOS {
                reader.set_position(start_pos.0, start_pos.1);
                return None;
            }
            if b == b'[' {
                depth += 1;
            } else if b == b']' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            reader.advance(1);
        }

        let content_end = reader.position();
        let content = &reader.source()[content_start.1.start()..content_end.1.start()];
        reader.advance(1); // skip ']'

        // Must be immediately followed by `{`.
        if reader.peek_byte() != b'{' {
            reader.set_position(start_pos.0, start_pos.1);
            return None;
        }
        reader.advance(1);

        // Read attributes until matching '}'.
        let attr_start = reader.position();
        let mut brace_depth: usize = 1;
        loop {
            let b = reader.peek_byte();
            if b == text::EOS {
                reader.set_position(start_pos.0, start_pos.1);
                return None;
            }
            if b == b'{' {
                brace_depth += 1;
            } else if b == b'}' {
                brace_depth -= 1;
                if brace_depth == 0 {
                    break;
                }
            }
            reader.advance(1);
        }

        let attr_end = reader.position();
        let attr_str = &reader.source()[attr_start.1.start()..attr_end.1.start()];

        // Attributes must be non-empty for span syntax.
        if attr_str.trim().is_empty() {
            reader.set_position(start_pos.0, start_pos.1);
            return None;
        }

        let node_ref = arena.new_node(SpanAttributes);

        // Create a text child node for the content.
        if !content.is_empty() {
            let text_ref = arena.new_node(ast::Text::new(text::Segment::new(
                content_start.1.start(),
                content_end.1.start(),
            )));
            node_ref.append_child(arena, text_ref);
        }

        // Parse and attach attributes.
        parse_component_attributes(attr_str, &mut arena[node_ref]);

        reader.advance(1); // skip '}'

        Some(node_ref)
    }
}

/// Parser extension function for span attributes.
pub(crate) fn span_attribute_parser_extension() -> impl ParserExtension {
    parser::ParserExtensionFn::new(|parser: &mut Parser| {
        parser.add_inline_parser(
            || Box::new(SpanAttributeParser) as Box<dyn InlineParser>,
            NoParserOptions,
            150,
        );
    })
}

// ---------------------------------------------------------------------------
// Shared attribute parsing helper
// ---------------------------------------------------------------------------

/// Parse a comark-style attribute string and attach key/value pairs to a node.
///
/// Supports: `.class`, `#id`, `key="value"`, `key='value'`, `key=value`.
/// Multiple `.class` entries are merged into a single `class` attribute,
/// space-separated.
pub(crate) fn parse_component_attributes(attr_str: &str, node: &mut ast::Node) {
    let mut classes = Vec::new();
    let input = attr_str.trim();
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Skip whitespace.
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

        if bytes[i] == b'.' {
            // Class shorthand: .classname
            i += 1;
            let start = i;
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && bytes[i] != b'.'
                && bytes[i] != b'#'
                && bytes[i] != b'}'
            {
                i += 1;
            }
            if i > start {
                classes.push(input[start..i].to_string());
            }
        } else if bytes[i] == b'#' {
            // ID shorthand: #identifier
            i += 1;
            let start = i;
            while i < bytes.len()
                && !bytes[i].is_ascii_whitespace()
                && bytes[i] != b'.'
                && bytes[i] != b'#'
                && bytes[i] != b'}'
            {
                i += 1;
            }
            if i > start {
                node.attributes_mut()
                    .insert("id", text::Value::from(input[start..i].to_string()));
            }
        } else {
            // Key=value pair.
            let key_start = i;
            while i < bytes.len()
                && bytes[i] != b'='
                && !bytes[i].is_ascii_whitespace()
                && bytes[i] != b'}'
            {
                i += 1;
            }
            let key = &input[key_start..i];
            if key.is_empty() {
                i += 1;
                continue;
            }

            if i < bytes.len() && bytes[i] == b'=' {
                i += 1; // skip '='
                let value = if i < bytes.len() && (bytes[i] == b'"' || bytes[i] == b'\'') {
                    let quote = bytes[i];
                    i += 1;
                    let val_start = i;
                    while i < bytes.len() && bytes[i] != quote {
                        i += 1;
                    }
                    let val = &input[val_start..i];
                    if i < bytes.len() {
                        i += 1; // skip closing quote
                    }
                    val
                } else {
                    let val_start = i;
                    while i < bytes.len() && !bytes[i].is_ascii_whitespace() && bytes[i] != b'}' {
                        i += 1;
                    }
                    &input[val_start..i]
                };
                node.attributes_mut()
                    .insert(key, text::Value::from(value.to_string()));
            } else {
                // Boolean attribute (key with no value).
                node.attributes_mut()
                    .insert(key, text::Value::from(String::new()));
            }
        }
    }

    if !classes.is_empty() {
        node.attributes_mut()
            .insert("class", text::Value::from(classes.join(" ")));
    }
}

/// Check if a byte is valid as the start of a component name.
fn is_name_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

/// Check if a byte is valid as part of a component name.
fn is_name_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

// ---------------------------------------------------------------------------
// HTML renderers for Comark extensions (rushdown fast path)
// ---------------------------------------------------------------------------

/// Renders block components as bare `<div>` elements.
#[derive(Debug)]
struct BlockComponentHtmlRenderer;

impl<W: TextWrite> RenderNode<W> for BlockComponentHtmlRenderer {
    fn render_node<'a>(
        &self,
        writer: &mut W,
        source: &'a str,
        arena: &'a Arena,
        node_ref: NodeRef,
        entering: bool,
        _context: &mut renderer::Context,
    ) -> rushdown::Result<ast::WalkStatus> {
        let node = &arena[node_ref];
        if entering {
            writer.write_str("<div")?;
            for (key, value) in node.attributes().iter() {
                writer.write_str(" ")?;
                writer.write_str(key)?;
                writer.write_str("=\"")?;
                let val = value.str(source);
                writer.write_str(&html_escape(val))?;
                writer.write_str("\"")?;
            }
            writer.write_str(">\n")?;
        } else {
            writer.write_str("</div>\n")?;
        }
        Ok(ast::WalkStatus::Continue)
    }
}

impl<'r, W: TextWrite> NodeRenderer<'r, W> for BlockComponentHtmlRenderer {
    fn register_node_renderer_fn(self, nrr: &mut impl NodeRendererRegistry<'r, W>) {
        nrr.register_node_renderer_fn(
            std::any::TypeId::of::<BlockComponent>(),
            BoxRenderNode::new(self),
        );
    }
}

/// Extension function to register the block component HTML renderer.
pub(crate) fn block_component_html_renderer_extension()
-> impl html::RendererExtension<'static, String> {
    html::RendererExtensionFn::new(|renderer: &mut html::Renderer<'_, String>| {
        renderer.add_node_renderer(|| BlockComponentHtmlRenderer, NoRendererOptions);
    })
}

/// Renders inline components as bare `<span>` elements.
#[derive(Debug)]
struct InlineComponentHtmlRenderer;

impl<W: TextWrite> RenderNode<W> for InlineComponentHtmlRenderer {
    fn render_node<'a>(
        &self,
        writer: &mut W,
        source: &'a str,
        arena: &'a Arena,
        node_ref: NodeRef,
        entering: bool,
        _context: &mut renderer::Context,
    ) -> rushdown::Result<ast::WalkStatus> {
        let node = &arena[node_ref];
        if entering {
            writer.write_str("<span")?;
            for (key, value) in node.attributes().iter() {
                writer.write_str(" ")?;
                writer.write_str(key)?;
                writer.write_str("=\"")?;
                let val = value.str(source);
                writer.write_str(&html_escape(val))?;
                writer.write_str("\"")?;
            }
            writer.write_str(">")?;
        } else {
            writer.write_str("</span>")?;
        }
        Ok(ast::WalkStatus::Continue)
    }
}

impl<'r, W: TextWrite> NodeRenderer<'r, W> for InlineComponentHtmlRenderer {
    fn register_node_renderer_fn(self, nrr: &mut impl NodeRendererRegistry<'r, W>) {
        nrr.register_node_renderer_fn(
            std::any::TypeId::of::<InlineComponent>(),
            BoxRenderNode::new(self),
        );
    }
}

/// Extension function to register the inline component HTML renderer.
pub(crate) fn inline_component_html_renderer_extension()
-> impl html::RendererExtension<'static, String> {
    html::RendererExtensionFn::new(|renderer: &mut html::Renderer<'_, String>| {
        renderer.add_node_renderer(|| InlineComponentHtmlRenderer, NoRendererOptions);
    })
}

/// Renders span attribute nodes as `<span>` elements.
#[derive(Debug)]
struct SpanAttributeHtmlRenderer;

impl<W: TextWrite> RenderNode<W> for SpanAttributeHtmlRenderer {
    fn render_node<'a>(
        &self,
        writer: &mut W,
        source: &'a str,
        arena: &'a Arena,
        node_ref: NodeRef,
        entering: bool,
        _context: &mut renderer::Context,
    ) -> rushdown::Result<ast::WalkStatus> {
        let node = &arena[node_ref];
        if entering {
            writer.write_str("<span")?;
            for (key, value) in node.attributes().iter() {
                writer.write_str(" ")?;
                writer.write_str(key)?;
                writer.write_str("=\"")?;
                let val = value.str(source);
                writer.write_str(&html_escape(val))?;
                writer.write_str("\"")?;
            }
            writer.write_str(">")?;
        } else {
            writer.write_str("</span>")?;
        }
        Ok(ast::WalkStatus::Continue)
    }
}

impl<'r, W: TextWrite> NodeRenderer<'r, W> for SpanAttributeHtmlRenderer {
    fn register_node_renderer_fn(self, nrr: &mut impl NodeRendererRegistry<'r, W>) {
        nrr.register_node_renderer_fn(
            std::any::TypeId::of::<SpanAttributes>(),
            BoxRenderNode::new(self),
        );
    }
}

/// Extension function to register the span attribute HTML renderer.
pub(crate) fn span_attribute_html_renderer_extension()
-> impl html::RendererExtension<'static, String> {
    html::RendererExtensionFn::new(|renderer: &mut html::Renderer<'_, String>| {
        renderer.add_node_renderer(|| SpanAttributeHtmlRenderer, NoRendererOptions);
    })
}
