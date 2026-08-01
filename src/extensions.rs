//! Comark extension types, parsers, and HTML renderers.
//!
//! This module provides the custom Markdown extensions used by OxydeMark:
//! - **Block components**: `::name{attrs}\ncontent\n::`
//! - **Inline components**: `:name[content]{attrs}`
//! - **Span attributes**: `[text]{.class #id key="val"}`
//!
//! Each extension includes an AST node type, a rushdown parser extension,
//! and a rushdown HTML renderer extension.

use std::cell::RefCell;
use std::fmt;

use std::collections::HashSet;

use rushdown::ast::{
    self, Arena, BlockText, KindData, NodeKind, NodeRef, NodeType, PrettyPrint, pp_indent,
};
use rushdown::parser::{
    self, BlockParser, Context, InlineParser, NoParserOptions, Parser, ParserExtension, State,
};
use rushdown::renderer::html;
use rushdown::renderer::{
    self as renderer, BoxRenderNode, NoRendererOptions, NodeRenderer, NodeRendererRegistry,
    RenderNode, TextWrite,
};
use rushdown::text::{self, Reader};
use rushdown::{as_kind_data, as_type_data};
use rushdown_emoji::Emoji;
use rushdown_meta::{MetaParserOptions, meta_parser_extension};

use crate::html_render::html_escape;

// ---------------------------------------------------------------------------
// Extension node types
// ---------------------------------------------------------------------------

/// A block component node: `::name{attrs}\ncontent\n::`.
///
/// Represents a container block introduced by `::component_name{attributes}`.
/// The component body runs until a closing line made of the same number of
/// colons as the opener (OMEP-0007 multi-colon nesting). All component semantics
/// (name, attributes) are carried in the AST for Python plugins to consume.
#[derive(Debug)]
pub(crate) struct BlockComponent {
    pub(crate) name: String,
    /// Typed block props parsed from a leading YAML block (OMEP-0007).
    ///
    /// Always a [`ast::Meta::Mapping`] when `Some`, or `None` when the component
    /// declares no YAML props. Precedence against inline `{…}` attributes is
    /// resolved during AST conversion (see `src/ast.rs`).
    pub(crate) props: Option<ast::Meta>,
    /// Number of colons in the opening fence (`>= 2`).
    ///
    /// The component closes on a line made of exactly this many colons.
    pub(crate) colons: usize,
    /// Parse-time state: `true` while the block is still open.
    ///
    /// rushdown calls `cont()` outermost-first and has no public notion of block
    /// openness, so the parser tracks it here to resolve a closing fence against
    /// the innermost matching level.
    pub(crate) open: bool,
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
        writeln!(w, "{}name: {}", pp_indent(level), self.name)?;
        if let Some(ast::Meta::Mapping(map)) = &self.props {
            writeln!(w, "{}props: {}", pp_indent(level), map.len())?;
        }
        Ok(())
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

/// A named slot node inside a block component: `#slot-name`.
///
/// A slot partitions a block component body into named sections. The slot
/// name is stored on the node and surfaced to Python as `attributes["name"]`.
/// Content following the marker (until the next marker or the closing `::`)
/// becomes the slot's children.
#[derive(Debug)]
pub(crate) struct Slot {
    pub(crate) name: String,
}

impl NodeKind for Slot {
    fn typ(&self) -> NodeType {
        NodeType::ContainerBlock
    }

    fn kind_name(&self) -> &'static str {
        "Slot"
    }
}

impl PrettyPrint for Slot {
    fn pretty_print(&self, w: &mut dyn fmt::Write, _source: &str, level: usize) -> fmt::Result {
        writeln!(w, "{}name: {}", pp_indent(level), self.name)
    }
}

impl From<Slot> for KindData {
    fn from(e: Slot) -> Self {
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
///
/// Openers may use any run of `n >= 2` colons; the component then closes on a
/// line made of exactly `n` colons (OMEP-0007 nested components).
#[derive(Debug)]
struct BlockComponentParser;

/// If `trimmed` is made solely of colons, return the run length (`>= 2`).
///
/// Such a line is a component closing fence candidate; the run length selects
/// which nesting level it closes.
fn colon_run_len(trimmed: &str) -> Option<usize> {
    if trimmed.len() >= 2 && trimmed.bytes().all(|b| b == b':') {
        Some(trimmed.len())
    } else {
        None
    }
}

/// Return the [`BlockComponent`] data of `node_ref`, if any.
fn as_block_component(arena: &Arena, node_ref: NodeRef) -> Option<&BlockComponent> {
    match arena[node_ref].kind_data() {
        KindData::Extension(ext) => {
            (ext.as_ref() as &dyn std::any::Any).downcast_ref::<BlockComponent>()
        }
        _ => None,
    }
}

/// Return `true` if a still-open descendant component was opened with `colons`.
///
/// Open blocks always sit on the `last_child` chain, so the walk descends it and
/// stops at the first component already closed.
fn has_open_descendant_with_colons(arena: &Arena, node_ref: NodeRef, colons: usize) -> bool {
    let mut current = arena[node_ref].last_child();
    while let Some(node) = current {
        if let Some(component) = as_block_component(arena, node) {
            if !component.open {
                return false;
            }
            if component.colons == colons {
                return true;
            }
        }
        current = arena[node].last_child();
    }
    false
}

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

        // The opening fence is a run of at least two colons.
        let colons = trimmed.bytes().take_while(|&b| b == b':').count();
        if colons < 2 {
            return None;
        }

        let rest = trimmed[colons..].trim();
        if rest.is_empty() {
            // A bare colon run is a closing marker, not an opening.
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

        // We commit to opening the component here. Copy out the borrowed slices
        // before advancing the reader (which invalidates `line`).
        let name = name.to_string();
        let attr_str = attr_str.map(str::to_string);

        // Consume the opening `::name{…}` line, then an optional leading YAML
        // props block that must appear immediately after it (OMEP-0007).
        reader.advance_line();
        let props = consume_block_props(reader).and_then(|body| parse_props_yaml(&body));

        let node_ref = arena.new_node(BlockComponent {
            name,
            props,
            colons,
            open: true,
        });

        // Parse and attach inline attributes.
        if let Some(attrs) = attr_str {
            parse_component_attributes(&attrs, &mut arena[node_ref]);
        }

        Some((node_ref, State::HAS_CHILDREN))
    }

    fn cont(
        &self,
        arena: &mut Arena,
        node_ref: NodeRef,
        reader: &mut text::BasicReader,
        _ctx: &mut Context,
    ) -> Option<State> {
        let (line_bytes, _seg) = reader.peek_line_bytes()?;
        let line = std::str::from_utf8(&line_bytes).ok()?;
        let trimmed = line.trim();

        // Closing marker: a line made of exactly as many colons as the opener,
        // resolved against the innermost still-open matching level.
        if let Some(colons) = colon_run_len(trimmed) {
            let matches = as_block_component(arena, node_ref)
                .is_some_and(|component| component.colons == colons);
            if matches && !has_open_descendant_with_colons(arena, node_ref, colons) {
                // Consume the fence but stay on the line, so the driver does not
                // offer it to inner parsers as a lazy paragraph continuation.
                reader.advance_to_eol();
                return None; // Close this block.
            }
        }

        // Continue accepting children.
        Some(State::HAS_CHILDREN)
    }

    fn close(
        &self,
        arena: &mut Arena,
        node_ref: NodeRef,
        _reader: &mut text::BasicReader,
        _ctx: &mut Context,
    ) {
        if let Some(node) = arena.get_mut(node_ref)
            && let KindData::Extension(ext) = node.kind_data_mut()
            && let Some(component) =
                (ext.as_mut() as &mut dyn std::any::Any).downcast_mut::<BlockComponent>()
        {
            component.open = false;
        }
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
// Block component YAML props
// ---------------------------------------------------------------------------

/// Build a minimal rushdown parser that only understands YAML frontmatter.
///
/// Used to type a component's leading YAML block by reusing the
/// `rushdown-meta` code path (see [`parse_props_yaml`]).
fn build_props_parser() -> Parser {
    let extensions = meta_parser_extension(MetaParserOptions::default());
    Parser::with_extensions(parser::Options::default(), extensions)
}

thread_local! {
    /// Lazily-constructed, thread-local YAML-props parser.
    ///
    /// Kept separate from the main document parser so it only carries the
    /// frontmatter extension, and cached to avoid rebuilding it per component.
    static PROPS_PARSER: RefCell<Parser> = RefCell::new(build_props_parser());
}

/// Parse a raw YAML body into a typed [`ast::Meta`] mapping.
///
/// The body is wrapped as a synthetic `---\n{body}\n---\n` document and parsed
/// with the frontmatter-only [`PROPS_PARSER`], reusing the `rushdown-meta`
/// typing path. Returns `None` on parse failure, empty metadata, or a
/// non-mapping top-level value.
fn parse_props_yaml(body: &str) -> Option<ast::Meta> {
    let source = format!("---\n{body}\n---\n");
    PROPS_PARSER.with(|parser| {
        let parser = parser.borrow();
        let mut reader = text::BasicReader::new(&source);
        let (arena, document_ref) = parser.parse(&mut reader);
        let meta = as_kind_data!(arena, document_ref, Document).metadata();
        if meta.is_empty() {
            None
        } else {
            Some(ast::Meta::Mapping(meta.clone()))
        }
    })
}

/// Consume a leading YAML props block from a block-component body.
///
/// The `reader` must be positioned on the first line *after* the opening
/// `::name` line. Two block styles are recognized (OMEP-0007):
///
/// * `---`-delimited frontmatter, and
/// * a fenced code block whose info string is exactly `yaml [props]`.
///
/// The block must appear *immediately* after the opening line: a blank first
/// line yields `None`. On a well-formed block the raw body (without the
/// delimiters/fence) is returned and the reader is advanced past the closing
/// delimiter. When no block is present or the block is unterminated, the reader
/// position is restored and `None` is returned.
fn consume_block_props(reader: &mut text::BasicReader) -> Option<String> {
    let saved = reader.position();

    let (line_bytes, _seg) = reader.peek_line_bytes()?;
    let line = std::str::from_utf8(&line_bytes).ok()?;
    let trimmed = line.trim();

    let closing: ClosingMarker = if trimmed == "---" {
        ClosingMarker::Frontmatter
    } else if is_yaml_props_fence(trimmed) {
        ClosingMarker::Fence
    } else {
        // No leading YAML block (includes a blank first line).
        return None;
    };

    // Consume the opening delimiter/fence line.
    reader.advance_line();

    let mut body = String::new();
    loop {
        let Some((line_bytes, _seg)) = reader.peek_line_bytes() else {
            // Reached EOF without a closing delimiter: not a props block.
            reader.set_position(saved.0, saved.1);
            return None;
        };
        let Ok(line) = std::str::from_utf8(&line_bytes) else {
            reader.set_position(saved.0, saved.1);
            return None;
        };
        let trimmed = line.trim();

        // A closing colon run ends the component before the block terminates.
        if colon_run_len(trimmed).is_some() {
            reader.set_position(saved.0, saved.1);
            return None;
        }

        let is_close = match closing {
            ClosingMarker::Frontmatter => trimmed == "---",
            ClosingMarker::Fence => trimmed == "```",
        };
        if is_close {
            reader.advance_line();
            return Some(body);
        }

        body.push_str(line);
        if !line.ends_with('\n') {
            body.push('\n');
        }
        reader.advance_line();
    }
}

/// The delimiter that terminates a leading YAML props block.
enum ClosingMarker {
    /// A `---` frontmatter block, closed by another `---` line.
    Frontmatter,
    /// A ```` ```yaml [props] ```` fenced block, closed by a ```` ``` ```` line.
    Fence,
}

/// Return `true` if `trimmed` is an opening ```` ```yaml [props] ```` fence.
///
/// The info string must be exactly `yaml [props]` (a single space is tolerated
/// as arbitrary internal whitespace). A plain ```` ```yaml ```` block is *not*
/// a props fence and is left to normal parsing as component content.
fn is_yaml_props_fence(trimmed: &str) -> bool {
    let Some(info) = trimmed.strip_prefix("```") else {
        return false;
    };
    // Reject longer/mismatched fences like ```` ```` ```` handled elsewhere.
    if info.starts_with('`') {
        return false;
    }
    let mut tokens = info.split_whitespace();
    matches!(
        (tokens.next(), tokens.next(), tokens.next()),
        (Some("yaml"), Some("[props]"), None)
    )
}

// ---------------------------------------------------------------------------
// Slot parser
// ---------------------------------------------------------------------------

/// If `line` is a slot marker (`#slot-name` as the sole content of the line),
/// return the slot name.
///
/// A slot marker matches `^#[A-Za-z][A-Za-z0-9_-]*$` after trimming surrounding
/// whitespace. This deliberately excludes ATX headings (`# Heading`), which
/// require a space after the `#`.
fn slot_marker_name(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let name = trimmed.strip_prefix('#')?;
    let mut chars = name.chars();
    let first = chars.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    if chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
        Some(name)
    } else {
        None
    }
}

/// Return `true` if `node_ref` refers to a [`BlockComponent`] extension node.
fn is_block_component(arena: &Arena, node_ref: NodeRef) -> bool {
    as_block_component(arena, node_ref).is_some()
}

/// Parses component slots: `#slot-name` markers inside a block component body.
///
/// A slot marker opens a [`Slot`] container whose children are the blocks that
/// follow it, up to the next slot marker or the component's closing `::`. Slot
/// markers are only recognized when the direct parent is a [`BlockComponent`],
/// so `#slot-name` lines outside components are left to the normal parsers.
#[derive(Debug)]
struct SlotParser;

impl BlockParser for SlotParser {
    fn trigger(&self) -> &[u8] {
        b"#"
    }

    fn open(
        &self,
        arena: &mut Arena,
        parent_ref: NodeRef,
        reader: &mut text::BasicReader,
        _ctx: &mut Context,
    ) -> Option<(NodeRef, State)> {
        // Slots only exist at the top level of a block-component body.
        if !is_block_component(arena, parent_ref) {
            return None;
        }

        let (line_bytes, _seg) = reader.peek_line_bytes()?;
        let line = std::str::from_utf8(&line_bytes).ok()?;
        let name = slot_marker_name(line)?;

        let node_ref = arena.new_node(Slot {
            name: name.to_string(),
        });

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

        // The closing colon run of the enclosing component or the next slot
        // marker both end this slot; the line is left for the parent/sibling
        // parser.
        if colon_run_len(trimmed).is_some() || slot_marker_name(line).is_some() {
            return None;
        }

        Some(State::HAS_CHILDREN)
    }

    fn can_interrupt_paragraph(&self) -> bool {
        true
    }
}

/// Parser extension function for component slots.
pub(crate) fn slot_parser_extension() -> impl ParserExtension {
    parser::ParserExtensionFn::new(|parser: &mut Parser| {
        parser.add_block_parser(
            || Box::new(SlotParser) as Box<dyn BlockParser>,
            NoParserOptions,
            550,
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
// Heading anchor / slug assignment
// ---------------------------------------------------------------------------

/// Assign deterministic, collision-safe anchor `id`s to every heading.
///
/// Walks the arena in document order and populates each `heading` node's `id`
/// attribute per the OMEP-0010 algorithm ([`crate::slug`]). Author-provided ids
/// (e.g. `## Title {#custom}`, parsed via the `attributes: true` option) are
/// honoured verbatim: they reserve their slot in the per-document set and are
/// never renumbered, and generated slugs avoid them. Headings nested inside
/// block components and slots are included.
pub(crate) fn assign_heading_anchors(arena: &mut Arena, root: NodeRef, source: &str) {
    // Pass 1: reserve every author-provided id so generated slugs avoid them.
    let mut used: HashSet<String> = HashSet::new();
    let mut headings: Vec<NodeRef> = Vec::new();
    collect_headings(arena, root, &mut headings);
    for &heading_ref in &headings {
        if let Some(id) = arena[heading_ref]
            .attributes()
            .get("id")
            .map(|v| v.str(source).to_string())
        {
            used.insert(id);
        }
    }

    // Pass 2: generate slugs for headings without an author-provided id.
    for &heading_ref in &headings {
        if arena[heading_ref].attributes().get("id").is_some() {
            continue;
        }
        let text = heading_text(arena, heading_ref, source);
        let slug = crate::slug::slugify_unique(&text, &mut used);
        arena[heading_ref]
            .attributes_mut()
            .insert("id", text::Value::from(slug));
    }
}

/// Collect all `heading` node refs under `node_ref` in document order.
fn collect_headings(arena: &Arena, node_ref: NodeRef, out: &mut Vec<NodeRef>) {
    if matches!(arena[node_ref].kind_data(), KindData::Heading(_)) {
        out.push(node_ref);
    }
    let mut child = arena[node_ref].first_child();
    while let Some(child_ref) = child {
        collect_headings(arena, child_ref, out);
        child = arena[child_ref].next_sibling();
    }
}

/// Concatenate the text content of a heading for slug generation.
///
/// Descendant `Text` and `RawHtml` nodes contribute their raw content; emoji
/// extension nodes contribute their shortcode (e.g. `wave`) rather than the
/// Unicode character, keeping anchors ASCII-friendly.
fn heading_text(arena: &Arena, node_ref: NodeRef, source: &str) -> String {
    let mut out = String::new();
    collect_heading_text(arena, node_ref, source, &mut out);
    out
}

/// Recursive helper for [`heading_text`].
fn collect_heading_text(arena: &Arena, node_ref: NodeRef, source: &str, out: &mut String) {
    match arena[node_ref].kind_data() {
        KindData::Text(t) => out.push_str(t.str(source)),
        KindData::RawHtml(h) => out.push_str(&h.str(source)),
        KindData::Extension(ext) => {
            if let Some(emoji) = (ext.as_ref() as &dyn std::any::Any).downcast_ref::<Emoji>()
                && let Some(sc) = emoji.shortcode()
            {
                out.push_str(sc);
            }
        }
        _ => {}
    }

    let mut child = arena[node_ref].first_child();
    while let Some(child_ref) = child {
        collect_heading_text(arena, child_ref, source, out);
        child = arena[child_ref].next_sibling();
    }
}

// ---------------------------------------------------------------------------
// Summary / excerpt delimiter (`<!-- more -->`)
// ---------------------------------------------------------------------------

/// Determine whether an arena node is a `<!-- more -->` summary delimiter.
///
/// Per OMEP-0010, the delimiter is an HTML comment whose trimmed body is exactly
/// `more`. Matching is case-insensitive and tolerant of internal whitespace, so
/// `<!-- more -->`, `<!--more-->`, and `<!--   MORE   -->` all match. Both the
/// block form (`HtmlBlock`, its own line) and an inline `RawHtml` comment are
/// recognised; the block form is the idiomatic case and its raw text is read
/// directly from the arena because it is not surfaced through `str(source)`.
pub(crate) fn is_more_marker(arena: &Arena, node_ref: NodeRef, source: &str) -> bool {
    let raw = match arena[node_ref].kind_data() {
        KindData::HtmlBlock(block) => match block.value() {
            BlockText::Source => {
                let mut text = String::new();
                for line in as_type_data!(arena, node_ref, Block).source().iter() {
                    text.push_str(&line.str(source));
                }
                text
            }
            BlockText::Owned(value) => value.clone(),
        },
        KindData::RawHtml(html) => html.str(source).to_string(),
        _ => return false,
    };

    is_more_comment(&raw)
}

/// Check whether `raw` is exactly an HTML comment whose body is `more`.
///
/// Case-insensitive and whitespace-tolerant, per OMEP-0010.
fn is_more_comment(raw: &str) -> bool {
    let trimmed = raw.trim();
    let Some(inner) = trimmed
        .strip_prefix("<!--")
        .and_then(|s| s.strip_suffix("-->"))
    else {
        return false;
    };
    inner.trim().eq_ignore_ascii_case("more")
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

/// Renders slots as `<div data-slot="name">` wrappers.
#[derive(Debug)]
struct SlotHtmlRenderer;

impl<W: TextWrite> RenderNode<W> for SlotHtmlRenderer {
    fn render_node<'a>(
        &self,
        writer: &mut W,
        _source: &'a str,
        arena: &'a Arena,
        node_ref: NodeRef,
        entering: bool,
        _context: &mut renderer::Context,
    ) -> rushdown::Result<ast::WalkStatus> {
        if entering {
            let name = match arena[node_ref].kind_data() {
                KindData::Extension(ext) => (ext.as_ref() as &dyn std::any::Any)
                    .downcast_ref::<Slot>()
                    .map(|slot| slot.name.as_str())
                    .unwrap_or_default(),
                _ => "",
            };
            writer.write_str("<div data-slot=\"")?;
            writer.write_str(&html_escape(name))?;
            writer.write_str("\">\n")?;
        } else {
            writer.write_str("</div>\n")?;
        }
        Ok(ast::WalkStatus::Continue)
    }
}

impl<'r, W: TextWrite> NodeRenderer<'r, W> for SlotHtmlRenderer {
    fn register_node_renderer_fn(self, nrr: &mut impl NodeRendererRegistry<'r, W>) {
        nrr.register_node_renderer_fn(std::any::TypeId::of::<Slot>(), BoxRenderNode::new(self));
    }
}

/// Extension function to register the slot HTML renderer.
pub(crate) fn slot_html_renderer_extension() -> impl html::RendererExtension<'static, String> {
    html::RendererExtensionFn::new(|renderer: &mut html::Renderer<'_, String>| {
        renderer.add_node_renderer(|| SlotHtmlRenderer, NoRendererOptions);
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
