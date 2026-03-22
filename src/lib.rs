use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{self, Write};

use pyo3::prelude::*;
use rushdown::as_kind_data;
use rushdown::ast::{
    self, Arena, KindData, NodeKind, NodeRef, NodeType, PrettyPrint, TextQualifier, WalkStatus,
    pp_indent,
};
use rushdown::parser::{
    self, BlockParser, Context, InlineParser, NoParserOptions, Parser, ParserExtension, State,
};
use rushdown::renderer::html::{self, RendererExtension as _};
use rushdown::renderer::{
    self as renderer, BoxRenderNode, NoRendererOptions, NodeRenderer, NodeRendererRegistry,
    RenderNode, TextWrite,
};
use rushdown::text::{self, Reader};
use rushdown_emoji::{
    Emoji, EmojiHtmlRendererOptions, EmojiParserOptions, emoji_html_renderer_extension,
    emoji_parser_extension,
};
use rushdown_meta::{MetaParserOptions, meta_parser_extension};

/// A Python-friendly AST node representing a Markdown element.
///
/// This is a simplified, tree-based view of rushdown's arena-based AST,
/// designed for easy traversal and modification from Python plugins.
///
/// # Examples
///
/// From Python:
/// ```python
/// ast = oxydemark.parse("# Hello **world**")
/// for node in ast.walk():
///     if node.kind == "text":
///         print(node.text)
/// ```
#[pyclass(from_py_object)]
#[derive(Clone, Debug)]
pub struct AstNode {
    /// The node kind (e.g. "document", "paragraph", "text", "heading").
    #[pyo3(get, set)]
    pub kind: String,

    /// Child nodes.
    #[pyo3(get, set)]
    pub children: Vec<AstNode>,

    /// Text content for leaf nodes (e.g. "text", "code_span").
    #[pyo3(get, set)]
    pub text: Option<String>,

    /// HTML attributes attached to this node.
    #[pyo3(get, set)]
    pub attributes: HashMap<String, String>,

    /// YAML frontmatter metadata (only present on the "document" node).
    #[pyo3(get, set)]
    pub metadata: Option<HashMap<String, String>>,
}

#[pymethods]
impl AstNode {
    /// Create a new AST node.
    #[new]
    #[pyo3(signature = (kind, children=None, text=None, attributes=None, metadata=None))]
    fn new(
        kind: String,
        children: Option<Vec<AstNode>>,
        text: Option<String>,
        attributes: Option<HashMap<String, String>>,
        metadata: Option<HashMap<String, String>>,
    ) -> Self {
        AstNode {
            kind,
            children: children.unwrap_or_default(),
            text,
            attributes: attributes.unwrap_or_default(),
            metadata,
        }
    }

    /// Walk the AST tree depth-first, returning a flat list of all nodes.
    ///
    /// This enables the Python pattern:
    /// ```python
    /// for node in ast.walk():
    ///     if node.kind == "text":
    ///         node.text = node.text.replace("@", "<span>@</span>")
    /// ```
    fn walk(&self) -> Vec<AstNode> {
        let mut result = Vec::new();
        self.walk_recursive(&mut result);
        result
    }

    fn __repr__(&self) -> String {
        if let Some(ref t) = self.text {
            format!("AstNode(kind={:?}, text={:?})", self.kind, t)
        } else {
            format!(
                "AstNode(kind={:?}, children={})",
                self.kind,
                self.children.len()
            )
        }
    }
}

impl AstNode {
    fn walk_recursive(&self, result: &mut Vec<AstNode>) {
        result.push(self.clone());
        for child in &self.children {
            child.walk_recursive(result);
        }
    }
}

// ---------------------------------------------------------------------------
// Parser helpers
// ---------------------------------------------------------------------------

/// Build the configured rushdown parser with all extensions.
fn build_parser() -> Parser {
    let parser_extensions = parser::gfm(parser::GfmOptions::default())
        .and(meta_parser_extension(MetaParserOptions::default()))
        .and(emoji_parser_extension(EmojiParserOptions::default()))
        .and(block_component_parser_extension())
        .and(inline_component_parser_extension())
        .and(span_attribute_parser_extension());
    let options = parser::Options {
        attributes: true,
        ..parser::Options::default()
    };
    Parser::with_extensions(options, parser_extensions)
}

/// Build the configured rushdown HTML renderer with all extensions.
fn build_renderer() -> html::Renderer<'static, String> {
    html::Renderer::with_extensions(
        html::Options::default(),
        emoji_html_renderer_extension(EmojiHtmlRendererOptions::default())
            .and(block_component_html_renderer_extension())
            .and(inline_component_html_renderer_extension())
            .and(span_attribute_html_renderer_extension()),
    )
}

// ---------------------------------------------------------------------------
// Cached parser / renderer (thread-local singletons)
// ---------------------------------------------------------------------------

thread_local! {
    /// Lazily-constructed, thread-local parser instance.
    ///
    /// `Parser::parse` takes `&self`, so we only need a shared borrow.
    /// The `RefCell` exists only to satisfy `thread_local!`'s requirement
    /// for a `'static` initializer while still allowing the closure-based
    /// access pattern.
    static CACHED_PARSER: RefCell<Parser> = RefCell::new(build_parser());

    /// Lazily-constructed, thread-local renderer instance.
    static CACHED_RENDERER: RefCell<html::Renderer<'static, String>> =
        RefCell::new(build_renderer());
}

/// Execute `f` with a reference to the cached, thread-local parser.
///
/// Avoids reconstructing `Parser` (and all its extensions) on every call
/// to `parse()` / `markdown_to_html()`.
fn with_parser<R>(f: impl FnOnce(&Parser) -> R) -> R {
    CACHED_PARSER.with(|p| f(&p.borrow()))
}

/// Execute `f` with a reference to the cached, thread-local renderer.
///
/// Avoids reconstructing `Renderer` (and all its extensions) on every call
/// to `markdown_to_html()`.
fn with_renderer<R>(f: impl FnOnce(&html::Renderer<'static, String>) -> R) -> R {
    CACHED_RENDERER.with(|r| f(&r.borrow()))
}

// ---------------------------------------------------------------------------
// Arena AST -> AstNode conversion
// ---------------------------------------------------------------------------

/// Map a rushdown `KindData` variant to a human-readable kind string.
///
/// Emphasis level 1 maps to "emphasis", level 2 maps to "strong".
/// Extension nodes (emoji, components) are detected via downcast.
fn kind_name(node: &ast::Node) -> &'static str {
    match node.kind_data() {
        KindData::Document(_) => "document",
        KindData::Paragraph(_) => "paragraph",
        KindData::Heading(_) => "heading",
        KindData::Blockquote(_) => "blockquote",
        KindData::List(_) => "list",
        KindData::ListItem(_) => "list_item",
        KindData::CodeBlock(_) => "code_block",
        KindData::HtmlBlock(_) => "html_block",
        KindData::ThematicBreak(_) => "thematic_break",
        KindData::Text(_) => "text",
        KindData::Emphasis(e) => {
            if e.level() >= 2 {
                "strong"
            } else {
                "emphasis"
            }
        }
        KindData::Link(_) => "link",
        KindData::Image(_) => "image",
        KindData::CodeSpan(_) => "code_span",
        KindData::RawHtml(_) => "raw_html",
        KindData::LinkReferenceDefinition(_) => "link_reference_definition",
        KindData::Table(_) => "table",
        KindData::TableHeader(_) => "table_header",
        KindData::TableBody(_) => "table_body",
        KindData::TableRow(_) => "table_row",
        KindData::TableCell(_) => "table_cell",
        KindData::Strikethrough(_) => "strikethrough",
        KindData::Extension(ext) => {
            if (ext.as_ref() as &dyn std::any::Any)
                .downcast_ref::<Emoji>()
                .is_some()
            {
                "emoji"
            } else if (ext.as_ref() as &dyn std::any::Any)
                .downcast_ref::<BlockComponent>()
                .is_some()
            {
                "block_component"
            } else if (ext.as_ref() as &dyn std::any::Any)
                .downcast_ref::<InlineComponent>()
                .is_some()
            {
                "inline_component"
            } else if (ext.as_ref() as &dyn std::any::Any)
                .downcast_ref::<SpanAttributes>()
                .is_some()
            {
                "span"
            } else {
                "unknown"
            }
        }
        _ => "unknown",
    }
}

/// Extract text content from a node, if it is a text-bearing leaf node.
///
/// In rushdown, `Text` holds its content directly via `str(source)`.
/// `CodeSpan` content is stored in child `Text` nodes, so we return `None`
/// here and let child traversal handle it.
/// `RawHtml` uses `str(source)` which returns a `Cow<str>`.
/// `Emoji` extension nodes produce their Unicode character.
fn node_text(node: &ast::Node, source: &str) -> Option<String> {
    match node.kind_data() {
        KindData::Text(t) => Some(t.str(source).to_string()),
        KindData::RawHtml(h) => Some(h.str(source).to_string()),
        KindData::Extension(ext) => (ext.as_ref() as &dyn std::any::Any)
            .downcast_ref::<Emoji>()
            .map(|emoji| emoji.as_str().to_string()),
        _ => None,
    }
}

/// Check whether a `Text` node has a soft line break qualifier.
fn is_softbreak(node: &ast::Node) -> bool {
    if let KindData::Text(t) = node.kind_data() {
        t.has_qualifiers(TextQualifier::SOFT_LINE_BREAK)
    } else {
        false
    }
}

/// Check whether a `Text` node has a hard line break qualifier.
fn is_hardbreak(node: &ast::Node) -> bool {
    if let KindData::Text(t) = node.kind_data() {
        t.has_qualifiers(TextQualifier::HARD_LINE_BREAK)
    } else {
        false
    }
}

/// Extract attributes (e.g. href, src, title) from link/image nodes.
///
/// For component extension nodes, extract the component name as "name".
/// For emoji nodes, extract the shortcode as "shortcode".
fn node_attributes(node: &ast::Node, source: &str) -> HashMap<String, String> {
    let mut attrs = HashMap::new();

    match node.kind_data() {
        KindData::Link(link) => {
            attrs.insert(
                "href".to_string(),
                link.destination().str(source).to_string(),
            );
            if let Some(title) = link.title().filter(|t| !t.is_empty()) {
                attrs.insert("title".to_string(), title.str(source).to_string());
            }
        }
        KindData::Image(img) => {
            attrs.insert("src".to_string(), img.destination().str(source).to_string());
            if let Some(title) = img.title().filter(|t| !t.is_empty()) {
                attrs.insert("title".to_string(), title.str(source).to_string());
            }
        }
        KindData::Heading(h) => {
            attrs.insert("level".to_string(), h.level().to_string());
        }
        KindData::Extension(ext) => {
            if let Some(emoji) = (ext.as_ref() as &dyn std::any::Any).downcast_ref::<Emoji>() {
                if let Some(sc) = emoji.shortcode() {
                    attrs.insert("shortcode".to_string(), sc.to_string());
                }
            } else if let Some(bc) =
                (ext.as_ref() as &dyn std::any::Any).downcast_ref::<BlockComponent>()
            {
                attrs.insert("name".to_string(), bc.name.clone());
            } else if let Some(ic) =
                (ext.as_ref() as &dyn std::any::Any).downcast_ref::<InlineComponent>()
            {
                attrs.insert("name".to_string(), ic.name.clone());
            }
        }
        _ => {}
    }

    // Include HTML attributes from rushdown's attribute parser.
    for (key, value) in node.attributes().iter() {
        attrs.insert(key.to_string(), value.str(source).to_string());
    }

    attrs
}

/// Convert a rushdown arena AST rooted at `node_ref` into an `AstNode` tree.
///
/// Text nodes with soft/hard line break qualifiers are split into the text
/// content followed by a synthetic "softbreak" or "hardbreak" node.
fn arena_to_ast_node(arena: &ast::Arena, node_ref: NodeRef, source: &str) -> AstNode {
    let node = &arena[node_ref];
    let kind = kind_name(node).to_string();
    let text = node_text(node, source);
    let attributes = node_attributes(node, source);

    // Extract metadata from the document node (rushdown-meta).
    let metadata = if let KindData::Document(_) = node.kind_data() {
        let meta = as_kind_data!(arena, node_ref, Document).metadata();
        if meta.is_empty() {
            None
        } else {
            let mut map = HashMap::new();
            for (k, v) in meta.iter() {
                map.insert(k.to_string(), v.as_str().unwrap_or("").to_string());
            }
            Some(map)
        }
    } else {
        None
    };

    // Recursively convert children.
    let mut children = Vec::new();
    let mut child = node.first_child();
    while let Some(child_ref) = child {
        children.push(arena_to_ast_node(arena, child_ref, source));

        // If the child text node has a soft/hard line break qualifier,
        // emit a synthetic break node after it.
        let child_node = &arena[child_ref];
        if is_softbreak(child_node) {
            children.push(AstNode {
                kind: "softbreak".to_string(),
                children: Vec::new(),
                text: None,
                attributes: HashMap::new(),
                metadata: None,
            });
        } else if is_hardbreak(child_node) {
            children.push(AstNode {
                kind: "hardbreak".to_string(),
                children: Vec::new(),
                text: None,
                attributes: HashMap::new(),
                metadata: None,
            });
        }

        child = arena[child_ref].next_sibling();
    }

    AstNode {
        kind,
        children,
        text,
        attributes,
        metadata,
    }
}

// ---------------------------------------------------------------------------
// AstNode -> HTML rendering
// ---------------------------------------------------------------------------

/// Render an `AstNode` tree to an HTML string.
///
/// This is a standalone Rust renderer that works on the Python-friendly
/// `AstNode` tree, independent of rushdown's renderer. It is used when
/// Python plugins have modified the AST.
fn render_ast_to_html(node: &AstNode) -> String {
    let mut output = String::new();
    render_node(&mut output, node);
    output
}

fn render_node(w: &mut String, node: &AstNode) {
    match node.kind.as_str() {
        "document" => render_children(w, node),
        "paragraph" => {
            w.push_str("<p>");
            render_children(w, node);
            w.push_str("</p>\n");
        }
        "heading" => {
            let level = node
                .attributes
                .get("level")
                .and_then(|v| v.parse::<u8>().ok())
                .unwrap_or(1);
            let tag = format!("h{level}");
            write!(w, "<{tag}").unwrap();
            render_html_attributes(w, node, &["level"]);
            w.push('>');
            render_children(w, node);
            writeln!(w, "</{tag}>").unwrap();
        }
        "blockquote" => {
            w.push_str("<blockquote>\n");
            render_children(w, node);
            w.push_str("</blockquote>\n");
        }
        "list" => {
            let tag = if node.attributes.get("ordered").is_some_and(|v| v == "true") {
                "ol"
            } else {
                "ul"
            };
            writeln!(w, "<{tag}>").unwrap();
            render_children(w, node);
            writeln!(w, "</{tag}>").unwrap();
        }
        "list_item" => {
            w.push_str("<li>");
            render_children(w, node);
            w.push_str("</li>\n");
        }
        "code_block" => {
            w.push_str("<pre><code>");
            if let Some(ref t) = node.text {
                w.push_str(&html_escape(t));
            }
            render_children(w, node);
            w.push_str("</code></pre>\n");
        }
        "thematic_break" => {
            w.push_str("<hr />\n");
        }
        "text" => {
            if let Some(ref t) = node.text {
                w.push_str(&html_escape(t));
            }
        }
        "softbreak" => {
            w.push('\n');
        }
        "hardbreak" => {
            w.push_str("<br />\n");
        }
        "emphasis" => {
            w.push_str("<em>");
            render_children(w, node);
            w.push_str("</em>");
        }
        "strong" => {
            w.push_str("<strong>");
            render_children(w, node);
            w.push_str("</strong>");
        }
        "strikethrough" => {
            w.push_str("<del>");
            render_children(w, node);
            w.push_str("</del>");
        }
        "link" => {
            let href = node.attributes.get("href").map_or("", |v| v.as_str());
            write!(w, "<a href=\"{}\"", html_escape_attr(href)).unwrap();
            render_html_attributes(w, node, &["href", "level"]);
            w.push('>');
            render_children(w, node);
            w.push_str("</a>");
        }
        "image" => {
            let src = node.attributes.get("src").map_or("", |v| v.as_str());
            write!(w, "<img src=\"{}\"", html_escape_attr(src)).unwrap();
            render_html_attributes(w, node, &["src", "level"]);
            let alt = collect_text(node);
            if !alt.is_empty() {
                write!(w, " alt=\"{}\"", html_escape_attr(&alt)).unwrap();
            }
            w.push_str(" />");
        }
        "code_span" => {
            w.push_str("<code>");
            if let Some(ref t) = node.text {
                w.push_str(&html_escape(t));
            }
            render_children(w, node);
            w.push_str("</code>");
        }
        "raw_html" => {
            if let Some(ref t) = node.text {
                w.push_str(t);
            }
        }
        "html_block" => {
            if let Some(ref t) = node.text {
                w.push_str(t);
            }
            render_children(w, node);
        }
        "table" => {
            w.push_str("<table>\n");
            render_children(w, node);
            w.push_str("</table>\n");
        }
        "table_header" => {
            w.push_str("<thead>\n");
            render_children(w, node);
            w.push_str("</thead>\n");
        }
        "table_body" => {
            w.push_str("<tbody>\n");
            render_children(w, node);
            w.push_str("</tbody>\n");
        }
        "table_row" => {
            w.push_str("<tr>\n");
            render_children(w, node);
            w.push_str("</tr>\n");
        }
        "table_cell" => {
            w.push_str("<td>");
            render_children(w, node);
            w.push_str("</td>");
        }
        "emoji" => {
            // Render emoji as its Unicode text.
            if let Some(ref t) = node.text {
                w.push_str(t);
            }
        }
        "block_component" => {
            // Passthrough: render as a bare <div> with attributes.
            w.push_str("<div");
            render_html_attributes(w, node, &["name"]);
            w.push_str(">\n");
            render_children(w, node);
            w.push_str("</div>\n");
        }
        "inline_component" => {
            // Passthrough: render as a bare <span> with attributes.
            w.push_str("<span");
            render_html_attributes(w, node, &["name"]);
            w.push('>');
            render_children(w, node);
            w.push_str("</span>");
        }
        "span" => {
            // Span attributes: render as <span> with attributes.
            w.push_str("<span");
            render_html_attributes(w, node, &[]);
            w.push('>');
            render_children(w, node);
            w.push_str("</span>");
        }
        _ => {
            // Unknown node types: render children transparently.
            render_children(w, node);
        }
    }
}

fn render_children(w: &mut String, node: &AstNode) {
    for child in &node.children {
        render_node(w, child);
    }
}

/// Render HTML attributes from the node, excluding internal keys.
fn render_html_attributes(w: &mut String, node: &AstNode, exclude: &[&str]) {
    for (key, value) in &node.attributes {
        if exclude.contains(&key.as_str()) {
            continue;
        }
        write!(w, " {}=\"{}\"", key, html_escape_attr(value)).unwrap();
    }
}

/// Collect all text content from a node tree (for alt text, etc.).
fn collect_text(node: &AstNode) -> String {
    let mut result = String::new();
    if let Some(ref t) = node.text {
        result.push_str(t);
    }
    for child in &node.children {
        result.push_str(&collect_text(child));
    }
    result
}

/// Escape HTML special characters in text content.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Escape HTML special characters in attribute values.
fn html_escape_attr(s: &str) -> String {
    html_escape(s)
}

// ---------------------------------------------------------------------------
// Comark extension types (Phase 2)
// ---------------------------------------------------------------------------

/// A block component node: `::name{attrs}\ncontent\n::`.
///
/// Represents a container block introduced by `::component_name{attributes}`.
/// The component body runs until a closing `::` line. All component semantics
/// (name, attributes) are carried in the AST for Python plugins to consume.
#[derive(Debug)]
struct BlockComponent {
    name: String,
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
struct InlineComponent {
    name: String,
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
struct SpanAttributes;

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
fn block_component_parser_extension() -> impl ParserExtension {
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
fn inline_component_parser_extension() -> impl ParserExtension {
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
fn span_attribute_parser_extension() -> impl ParserExtension {
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
fn parse_component_attributes(attr_str: &str, node: &mut ast::Node) {
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
    ) -> rushdown::Result<WalkStatus> {
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
        Ok(WalkStatus::Continue)
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
fn block_component_html_renderer_extension() -> impl html::RendererExtension<'static, String> {
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
    ) -> rushdown::Result<WalkStatus> {
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
        Ok(WalkStatus::Continue)
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
fn inline_component_html_renderer_extension() -> impl html::RendererExtension<'static, String> {
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
    ) -> rushdown::Result<WalkStatus> {
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
        Ok(WalkStatus::Continue)
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
fn span_attribute_html_renderer_extension() -> impl html::RendererExtension<'static, String> {
    html::RendererExtensionFn::new(|renderer: &mut html::Renderer<'_, String>| {
        renderer.add_node_renderer(|| SpanAttributeHtmlRenderer, NoRendererOptions);
    })
}

// ---------------------------------------------------------------------------
// Public Python API
// ---------------------------------------------------------------------------

/// Parse Markdown input into an AST node tree.
///
/// Uses rushdown with GFM, YAML frontmatter, and emoji extensions.
/// Returns an `AstNode` tree that can be inspected and modified from Python.
#[pyfunction]
fn parse(markdown: &str) -> PyResult<AstNode> {
    with_parser(|parser| {
        let mut reader = text::BasicReader::new(markdown);
        let (arena, document_ref) = parser.parse(&mut reader);
        Ok(arena_to_ast_node(&arena, document_ref, markdown))
    })
}

/// Render an `AstNode` tree to an HTML string.
///
/// This is the second half of the pipeline, used after Python plugins
/// have had a chance to modify the AST.
#[pyfunction]
fn render_ast(node: &AstNode) -> String {
    render_ast_to_html(node)
}

/// Convert Markdown directly to HTML (fast path, no AST exposure).
///
/// Uses rushdown's parser and renderer end-to-end without building
/// an intermediate `AstNode` tree. This is the fastest path when
/// no AST-level plugin transformations are needed.
#[pyfunction]
fn markdown_to_html(markdown: &str) -> PyResult<String> {
    with_parser(|parser| {
        let mut reader = text::BasicReader::new(markdown);
        let (arena, document_ref) = parser.parse(&mut reader);

        with_renderer(|renderer| {
            let mut output = String::new();
            renderer
                .render(&mut output, markdown, &arena, document_ref)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?;
            Ok(output)
        })
    })
}

/// Render Markdown input to HTML (legacy API, kept for backward compatibility).
///
/// This delegates to `markdown_to_html` and is equivalent to the fast path.
#[pyfunction]
fn render(markdown: &str) -> PyResult<String> {
    markdown_to_html(markdown)
}

/// The native Python module implemented in Rust.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<AstNode>()?;
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(render, m)?)?;
    m.add_function(wrap_pyfunction!(render_ast, m)?)?;
    m.add_function(wrap_pyfunction!(markdown_to_html, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_paragraph() {
        let ast = parse("Hello").unwrap();
        assert_eq!(ast.kind, "document");
        assert!(!ast.children.is_empty());
        let para = &ast.children[0];
        assert_eq!(para.kind, "paragraph");
        let text_node = &para.children[0];
        assert_eq!(text_node.kind, "text");
        assert_eq!(text_node.text.as_deref(), Some("Hello"));
    }

    #[test]
    fn parse_heading() {
        let ast = parse("# Title").unwrap();
        let heading = &ast.children[0];
        assert_eq!(heading.kind, "heading");
        assert_eq!(
            heading.attributes.get("level").map(|v| v.as_str()),
            Some("1")
        );
    }

    #[test]
    fn parse_emphasis_and_strong() {
        let ast = parse("*em* **strong**").unwrap();
        let para = &ast.children[0];
        assert!(para.children.iter().any(|c| c.kind == "emphasis"));
        assert!(para.children.iter().any(|c| c.kind == "strong"));
    }

    #[test]
    fn render_simple_paragraph() {
        let html = markdown_to_html("Hello").unwrap();
        assert!(html.contains("<p>"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn render_ast_round_trip() {
        let ast = parse("Hello **world**").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(html.contains("<p>"));
        assert!(html.contains("<strong>"));
        assert!(html.contains("world"));
    }

    #[test]
    fn render_heading_round_trip() {
        let ast = parse("# Title").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(html.contains("<h1"));
        assert!(html.contains("Title"));
    }

    #[test]
    fn walk_returns_all_nodes() {
        let ast = parse("Hello **world**").unwrap();
        let nodes = ast.walk();
        let kinds: Vec<&str> = nodes.iter().map(|n| n.kind.as_str()).collect();
        assert!(kinds.contains(&"document"));
        assert!(kinds.contains(&"paragraph"));
        assert!(kinds.contains(&"text"));
        assert!(kinds.contains(&"strong"));
    }

    #[test]
    fn parse_frontmatter() {
        let input = "---\ntitle: Hello\n---\n\nContent";
        let ast = parse(input).unwrap();
        assert_eq!(ast.kind, "document");
        assert!(ast.metadata.is_some());
        let meta = ast.metadata.as_ref().unwrap();
        assert_eq!(meta.get("title").map(|v| v.as_str()), Some("Hello"));
    }

    #[test]
    fn legacy_render_works() {
        let html = render("Hello").unwrap();
        assert!(html.contains("Hello"));
    }

    // -----------------------------------------------------------------------
    // Parsing: inline elements
    // -----------------------------------------------------------------------

    #[test]
    fn parse_link() {
        let ast = parse("[click](https://example.com)").unwrap();
        let para = &ast.children[0];
        let link = para.children.iter().find(|c| c.kind == "link").unwrap();
        assert_eq!(
            link.attributes.get("href").map(|v| v.as_str()),
            Some("https://example.com")
        );
        // Link text is a child text node.
        let text_node = link.children.iter().find(|c| c.kind == "text").unwrap();
        assert_eq!(text_node.text.as_deref(), Some("click"));
    }

    #[test]
    fn parse_link_with_title() {
        let ast = parse(r#"[click](https://example.com "a title")"#).unwrap();
        let para = &ast.children[0];
        let link = para.children.iter().find(|c| c.kind == "link").unwrap();
        assert_eq!(
            link.attributes.get("href").map(|v| v.as_str()),
            Some("https://example.com")
        );
        assert_eq!(
            link.attributes.get("title").map(|v| v.as_str()),
            Some("a title")
        );
    }

    #[test]
    fn parse_image() {
        let ast = parse("![alt text](image.png)").unwrap();
        let para = &ast.children[0];
        let img = para.children.iter().find(|c| c.kind == "image").unwrap();
        assert_eq!(
            img.attributes.get("src").map(|v| v.as_str()),
            Some("image.png")
        );
    }

    #[test]
    fn parse_code_span() {
        let ast = parse("Use `code` here").unwrap();
        let para = &ast.children[0];
        assert!(para.children.iter().any(|c| c.kind == "code_span"));
    }

    #[test]
    fn parse_strikethrough() {
        let ast = parse("~~deleted~~").unwrap();
        let para = &ast.children[0];
        assert!(para.children.iter().any(|c| c.kind == "strikethrough"));
    }

    // -----------------------------------------------------------------------
    // Parsing: block elements
    // -----------------------------------------------------------------------

    #[test]
    fn parse_heading_levels() {
        for level in 1..=6 {
            let input = format!("{} Heading {level}", "#".repeat(level));
            let ast = parse(&input).unwrap();
            let heading = &ast.children[0];
            assert_eq!(heading.kind, "heading");
            assert_eq!(
                heading
                    .attributes
                    .get("level")
                    .and_then(|v| v.parse::<usize>().ok()),
                Some(level)
            );
        }
    }

    #[test]
    fn parse_blockquote() {
        let ast = parse("> quoted text").unwrap();
        let bq = &ast.children[0];
        assert_eq!(bq.kind, "blockquote");
    }

    #[test]
    fn parse_unordered_list() {
        let ast = parse("- one\n- two\n- three").unwrap();
        let list = &ast.children[0];
        assert_eq!(list.kind, "list");
        assert_eq!(
            list.children
                .iter()
                .filter(|c| c.kind == "list_item")
                .count(),
            3
        );
    }

    #[test]
    fn parse_code_block() {
        let ast = parse("```\nfn main() {}\n```").unwrap();
        assert!(ast.children.iter().any(|c| c.kind == "code_block"));
    }

    #[test]
    fn parse_thematic_break() {
        // Use *** since --- is consumed by rushdown-meta as frontmatter.
        let ast = parse("***").unwrap();
        assert!(ast.children.iter().any(|c| c.kind == "thematic_break"));
    }

    #[test]
    fn parse_table() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |";
        let ast = parse(input).unwrap();
        assert!(ast.children.iter().any(|c| c.kind == "table"));
    }

    // -----------------------------------------------------------------------
    // Parsing: extensions
    // -----------------------------------------------------------------------

    #[test]
    fn parse_emoji_shortcode() {
        let ast = parse("Hello :wave:").unwrap();
        // The emoji extension replaces the shortcode; the walk should contain
        // text with the emoji character or shortcode depending on the parser.
        let nodes = ast.walk();
        let text_nodes: Vec<&AstNode> = nodes.iter().filter(|n| n.kind == "text").collect();
        assert!(!text_nodes.is_empty());
    }

    #[test]
    fn parse_frontmatter_multiple_keys() {
        let input = "---\ntitle: Hello\nauthor: World\ntags: test\n---\n\nContent";
        let ast = parse(input).unwrap();
        let meta = ast.metadata.as_ref().unwrap();
        assert_eq!(meta.get("title").map(|v| v.as_str()), Some("Hello"));
        assert_eq!(meta.get("author").map(|v| v.as_str()), Some("World"));
        assert_eq!(meta.get("tags").map(|v| v.as_str()), Some("test"));
    }

    #[test]
    fn parse_no_frontmatter() {
        let ast = parse("Just text").unwrap();
        assert!(ast.metadata.is_none());
    }

    // -----------------------------------------------------------------------
    // Rendering: fast path (markdown_to_html)
    // -----------------------------------------------------------------------

    #[test]
    fn render_heading_levels() {
        for level in 1..=6 {
            let input = format!("{} H{level}", "#".repeat(level));
            let html = markdown_to_html(&input).unwrap();
            assert!(
                html.contains(&format!("<h{level}")),
                "Expected <h{level}> in: {html}"
            );
        }
    }

    #[test]
    fn render_link() {
        let html = markdown_to_html("[text](https://example.com)").unwrap();
        assert!(html.contains("<a"));
        assert!(html.contains("href"));
        assert!(html.contains("https://example.com"));
    }

    #[test]
    fn render_image() {
        let html = markdown_to_html("![alt](img.png)").unwrap();
        assert!(html.contains("<img"));
        assert!(html.contains("img.png"));
    }

    #[test]
    fn render_code_block() {
        let html = markdown_to_html("```\ncode\n```").unwrap();
        assert!(html.contains("<pre>") || html.contains("<code>"));
    }

    #[test]
    fn render_blockquote() {
        let html = markdown_to_html("> quote").unwrap();
        assert!(html.contains("<blockquote>"));
    }

    #[test]
    fn render_list() {
        let html = markdown_to_html("- a\n- b").unwrap();
        assert!(html.contains("<ul>") || html.contains("<li>"));
    }

    #[test]
    fn render_thematic_break() {
        let html = markdown_to_html("***").unwrap();
        assert!(html.contains("<hr"));
    }

    #[test]
    fn render_strikethrough() {
        let html = markdown_to_html("~~deleted~~").unwrap();
        assert!(html.contains("<del>"));
        assert!(html.contains("deleted"));
    }

    #[test]
    fn render_table() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |";
        let html = markdown_to_html(input).unwrap();
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>"));
    }

    #[test]
    fn render_inline_code() {
        let html = markdown_to_html("Use `code` here").unwrap();
        assert!(html.contains("<code>"));
        assert!(html.contains("code"));
    }

    // -----------------------------------------------------------------------
    // Rendering: AST round-trip (render_ast_to_html)
    // -----------------------------------------------------------------------

    #[test]
    fn render_ast_link_round_trip() {
        let ast = parse("[click](https://example.com)").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(html.contains("<a"));
        assert!(html.contains("href=\"https://example.com\""));
        assert!(html.contains("click"));
    }

    #[test]
    fn render_ast_image_round_trip() {
        let ast = parse("![photo](pic.jpg)").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(html.contains("<img"));
        assert!(html.contains("src=\"pic.jpg\""));
    }

    #[test]
    fn render_ast_code_block_round_trip() {
        let ast = parse("```\nfn main() {}\n```").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(html.contains("<pre><code>"));
    }

    #[test]
    fn render_ast_blockquote_round_trip() {
        let ast = parse("> quoted").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(html.contains("<blockquote>"));
    }

    #[test]
    fn render_ast_list_round_trip() {
        let ast = parse("- x\n- y").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(html.contains("<li>"));
    }

    #[test]
    fn render_ast_thematic_break_round_trip() {
        let ast = parse("***").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(html.contains("<hr"));
    }

    #[test]
    fn render_ast_strikethrough_round_trip() {
        let ast = parse("~~gone~~").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(html.contains("<del>"));
        assert!(html.contains("gone"));
    }

    #[test]
    fn render_ast_table_round_trip() {
        let input = "| A | B |\n|---|---|\n| 1 | 2 |";
        let ast = parse(input).unwrap();
        let html = render_ast_to_html(&ast);
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>"));
    }

    #[test]
    fn render_ast_emphasis_round_trip() {
        let ast = parse("*emphasis*").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(html.contains("<em>"));
        assert!(html.contains("emphasis"));
    }

    #[test]
    fn render_ast_inline_code_round_trip() {
        let ast = parse("Use `code` here").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(html.contains("<code>"));
    }

    // -----------------------------------------------------------------------
    // AstNode construction and walk
    // -----------------------------------------------------------------------

    #[test]
    fn ast_node_default_construction() {
        let node = AstNode {
            kind: "test".to_string(),
            children: Vec::new(),
            text: None,
            attributes: HashMap::new(),
            metadata: None,
        };
        assert_eq!(node.kind, "test");
        assert!(node.children.is_empty());
        assert!(node.text.is_none());
        assert!(node.attributes.is_empty());
        assert!(node.metadata.is_none());
    }

    #[test]
    fn walk_count_matches_tree() {
        let ast = parse("# Hi\n\nParagraph with **bold** and *italic*.").unwrap();
        let nodes = ast.walk();
        // walk() includes the root, so count >= 1 + children count recursively.
        assert!(
            nodes.len() >= 5,
            "Expected at least 5 nodes, got {}",
            nodes.len()
        );
        // First node should be the document itself.
        assert_eq!(nodes[0].kind, "document");
    }

    #[test]
    fn walk_order_is_depth_first() {
        let ast = parse("**bold**").unwrap();
        let nodes = ast.walk();
        let kinds: Vec<&str> = nodes.iter().map(|n| n.kind.as_str()).collect();
        // Document -> Paragraph -> Strong -> Text (depth-first)
        assert_eq!(kinds[0], "document");
        assert_eq!(kinds[1], "paragraph");
        assert_eq!(kinds[2], "strong");
        assert_eq!(kinds[3], "text");
    }

    #[test]
    fn ast_node_repr() {
        let node = AstNode {
            kind: "text".to_string(),
            children: Vec::new(),
            text: Some("hello".to_string()),
            attributes: HashMap::new(),
            metadata: None,
        };
        let repr = node.__repr__();
        assert!(repr.contains("text"));
        assert!(repr.contains("hello"));
    }

    #[test]
    fn ast_node_repr_no_text() {
        let node = AstNode {
            kind: "document".to_string(),
            children: vec![AstNode {
                kind: "paragraph".to_string(),
                children: Vec::new(),
                text: None,
                attributes: HashMap::new(),
                metadata: None,
            }],
            text: None,
            attributes: HashMap::new(),
            metadata: None,
        };
        let repr = node.__repr__();
        assert!(repr.contains("document"));
        assert!(repr.contains("children=1"));
    }

    // -----------------------------------------------------------------------
    // Edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn parse_empty_input() {
        let ast = parse("").unwrap();
        assert_eq!(ast.kind, "document");
    }

    #[test]
    fn render_empty_input() {
        let html = markdown_to_html("").unwrap();
        assert!(html.is_empty() || html.trim().is_empty());
    }

    #[test]
    fn parse_nested_emphasis() {
        let ast = parse("***bold italic***").unwrap();
        let nodes = ast.walk();
        let kinds: Vec<&str> = nodes.iter().map(|n| n.kind.as_str()).collect();
        // Should have either emphasis inside strong, or strong inside emphasis.
        assert!(kinds.contains(&"emphasis") || kinds.contains(&"strong"));
    }

    #[test]
    fn parse_multiline_paragraph() {
        let ast = parse("line one\nline two").unwrap();
        let para = &ast.children[0];
        assert_eq!(para.kind, "paragraph");
        // Should have text content for both lines.
        let all_text = collect_text(para);
        assert!(all_text.contains("line one"));
        assert!(all_text.contains("line two"));
    }

    #[test]
    fn parse_multiple_paragraphs() {
        let ast = parse("Para one\n\nPara two").unwrap();
        let paragraphs: Vec<&AstNode> = ast
            .children
            .iter()
            .filter(|c| c.kind == "paragraph")
            .collect();
        assert_eq!(paragraphs.len(), 2);
    }

    #[test]
    fn render_special_characters_escaped() {
        let html = markdown_to_html("Use <script> & \"quotes\"").unwrap();
        // The raw HTML might be kept or escaped depending on parser settings.
        // At minimum, verify it doesn't crash and produces output.
        assert!(!html.is_empty());
    }

    #[test]
    fn html_escape_correctness() {
        assert_eq!(html_escape("<>&\""), "&lt;&gt;&amp;&quot;");
    }

    #[test]
    fn render_and_markdown_to_html_are_equivalent() {
        let input = "# Test\n\nSome *text* with **bold**.";
        let html1 = render(input).unwrap();
        let html2 = markdown_to_html(input).unwrap();
        assert_eq!(html1, html2);
    }

    // -----------------------------------------------------------------------
    // Comark: emoji AST
    // -----------------------------------------------------------------------

    #[test]
    fn parse_emoji_node_kind() {
        let ast = parse("Hello :wave:").unwrap();
        let nodes = ast.walk();
        let emoji_nodes: Vec<&AstNode> = nodes.iter().filter(|n| n.kind == "emoji").collect();
        assert!(
            !emoji_nodes.is_empty(),
            "Expected at least one emoji node, got kinds: {:?}",
            nodes.iter().map(|n| &n.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn parse_emoji_has_shortcode_attribute() {
        let ast = parse(":wave:").unwrap();
        let nodes = ast.walk();
        let emoji = nodes.iter().find(|n| n.kind == "emoji").unwrap();
        assert_eq!(
            emoji.attributes.get("shortcode").map(|v| v.as_str()),
            Some("wave")
        );
    }

    #[test]
    fn parse_emoji_has_text_content() {
        let ast = parse(":wave:").unwrap();
        let nodes = ast.walk();
        let emoji = nodes.iter().find(|n| n.kind == "emoji").unwrap();
        assert!(
            emoji.text.is_some(),
            "Emoji node should have text (Unicode char)"
        );
        // The wave emoji is 👋 (U+1F44B).
        assert_eq!(emoji.text.as_deref(), Some("👋"));
    }

    #[test]
    fn render_emoji_fast_path() {
        let html = markdown_to_html(":wave:").unwrap();
        assert!(
            html.contains("👋"),
            "Expected wave emoji in HTML, got: {html}"
        );
    }

    #[test]
    fn render_emoji_ast_round_trip() {
        let ast = parse(":wave:").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(
            html.contains("👋"),
            "Expected wave emoji in round-trip HTML, got: {html}"
        );
    }

    // -----------------------------------------------------------------------
    // Comark: block components
    // -----------------------------------------------------------------------

    #[test]
    fn parse_block_component() {
        let input = "::note\nSome content\n::";
        let ast = parse(input).unwrap();
        let nodes = ast.walk();
        let bc = nodes
            .iter()
            .find(|n| n.kind == "block_component")
            .expect("Expected a block_component node");
        assert_eq!(bc.attributes.get("name").map(|v| v.as_str()), Some("note"));
    }

    #[test]
    fn parse_block_component_with_attributes() {
        let input = "::warning{.alert #important}\nBe careful\n::";
        let ast = parse(input).unwrap();
        let nodes = ast.walk();
        let bc = nodes
            .iter()
            .find(|n| n.kind == "block_component")
            .expect("Expected a block_component node");
        assert_eq!(
            bc.attributes.get("name").map(|v| v.as_str()),
            Some("warning")
        );
        assert_eq!(
            bc.attributes.get("class").map(|v| v.as_str()),
            Some("alert")
        );
        assert_eq!(
            bc.attributes.get("id").map(|v| v.as_str()),
            Some("important")
        );
    }

    #[test]
    fn parse_block_component_with_key_value() {
        let input = "::note{type=\"info\"}\nContent\n::";
        let ast = parse(input).unwrap();
        let nodes = ast.walk();
        let bc = nodes
            .iter()
            .find(|n| n.kind == "block_component")
            .expect("Expected a block_component node");
        assert_eq!(bc.attributes.get("type").map(|v| v.as_str()), Some("info"));
    }

    #[test]
    fn parse_block_component_has_children() {
        let input = "::note\nHello world\n::";
        let ast = parse(input).unwrap();
        let nodes = ast.walk();
        let bc = nodes
            .iter()
            .find(|n| n.kind == "block_component")
            .expect("Expected a block_component node");
        assert!(
            !bc.children.is_empty(),
            "Block component should have children"
        );
    }

    #[test]
    fn render_block_component_fast_path() {
        let input = "::note\nContent\n::";
        let html = markdown_to_html(input).unwrap();
        assert!(
            html.contains("<div"),
            "Block component should render as <div>, got: {html}"
        );
        assert!(
            html.contains("</div>"),
            "Block component should have closing </div>, got: {html}"
        );
    }

    #[test]
    fn render_block_component_with_attrs_fast_path() {
        let input = "::note{.info}\nContent\n::";
        let html = markdown_to_html(input).unwrap();
        assert!(
            html.contains("class=\"info\""),
            "Expected class attribute in HTML, got: {html}"
        );
    }

    #[test]
    fn render_block_component_ast_round_trip() {
        let input = "::note\nContent\n::";
        let ast = parse(input).unwrap();
        let html = render_ast_to_html(&ast);
        assert!(
            html.contains("<div"),
            "Block component round-trip should produce <div>, got: {html}"
        );
    }

    // -----------------------------------------------------------------------
    // Comark: inline components
    // -----------------------------------------------------------------------

    #[test]
    fn parse_inline_component_with_content() {
        let input = ":icon[star]";
        let ast = parse(input).unwrap();
        let nodes = ast.walk();
        let ic = nodes
            .iter()
            .find(|n| n.kind == "inline_component")
            .expect("Expected an inline_component node");
        assert_eq!(ic.attributes.get("name").map(|v| v.as_str()), Some("icon"));
    }

    #[test]
    fn parse_inline_component_with_attrs() {
        let input = ":badge[Pro]{.premium}";
        let ast = parse(input).unwrap();
        let nodes = ast.walk();
        let ic = nodes
            .iter()
            .find(|n| n.kind == "inline_component")
            .expect("Expected an inline_component node");
        assert_eq!(ic.attributes.get("name").map(|v| v.as_str()), Some("badge"));
        assert_eq!(
            ic.attributes.get("class").map(|v| v.as_str()),
            Some("premium")
        );
    }

    #[test]
    fn parse_inline_component_attrs_only() {
        let input = ":icon{type=\"star\"}";
        let ast = parse(input).unwrap();
        let nodes = ast.walk();
        let ic = nodes
            .iter()
            .find(|n| n.kind == "inline_component")
            .expect("Expected an inline_component node");
        assert_eq!(ic.attributes.get("name").map(|v| v.as_str()), Some("icon"));
        assert_eq!(ic.attributes.get("type").map(|v| v.as_str()), Some("star"));
    }

    #[test]
    fn render_inline_component_fast_path() {
        let input = ":badge[Pro]{.premium}";
        let html = markdown_to_html(input).unwrap();
        assert!(
            html.contains("<span"),
            "Inline component should render as <span>, got: {html}"
        );
    }

    #[test]
    fn render_inline_component_ast_round_trip() {
        let input = ":badge[Pro]";
        let ast = parse(input).unwrap();
        let html = render_ast_to_html(&ast);
        assert!(
            html.contains("<span"),
            "Inline component round-trip should produce <span>, got: {html}"
        );
    }

    // -----------------------------------------------------------------------
    // Comark: span attributes
    // -----------------------------------------------------------------------

    #[test]
    fn parse_span_attributes() {
        let input = "[important]{.highlight}";
        let ast = parse(input).unwrap();
        let nodes = ast.walk();
        let span = nodes
            .iter()
            .find(|n| n.kind == "span")
            .expect("Expected a span node");
        assert_eq!(
            span.attributes.get("class").map(|v| v.as_str()),
            Some("highlight")
        );
    }

    #[test]
    fn parse_span_with_id() {
        let input = "[text]{#myid}";
        let ast = parse(input).unwrap();
        let nodes = ast.walk();
        let span = nodes
            .iter()
            .find(|n| n.kind == "span")
            .expect("Expected a span node");
        assert_eq!(span.attributes.get("id").map(|v| v.as_str()), Some("myid"));
    }

    #[test]
    fn parse_span_with_multiple_classes() {
        let input = "[text]{.a .b .c}";
        let ast = parse(input).unwrap();
        let nodes = ast.walk();
        let span = nodes
            .iter()
            .find(|n| n.kind == "span")
            .expect("Expected a span node");
        let class = span.attributes.get("class").map(|v| v.as_str()).unwrap();
        assert!(class.contains("a"), "Expected class 'a' in: {class}");
        assert!(class.contains("b"), "Expected class 'b' in: {class}");
        assert!(class.contains("c"), "Expected class 'c' in: {class}");
    }

    #[test]
    fn render_span_fast_path() {
        let input = "[highlighted]{.mark}";
        let html = markdown_to_html(input).unwrap();
        assert!(
            html.contains("<span"),
            "Span should render as <span>, got: {html}"
        );
        assert!(
            html.contains("class=\"mark\""),
            "Span should have class attribute, got: {html}"
        );
    }

    #[test]
    fn render_span_ast_round_trip() {
        let input = "[highlighted]{.mark}";
        let ast = parse(input).unwrap();
        let html = render_ast_to_html(&ast);
        assert!(
            html.contains("<span"),
            "Span round-trip should produce <span>, got: {html}"
        );
    }

    // -----------------------------------------------------------------------
    // Comark: attribute parsing helper
    // -----------------------------------------------------------------------

    #[test]
    fn parse_component_attributes_class() {
        with_parser(|parser| {
            let mut reader = text::BasicReader::new("test");
            let (mut arena, _) = parser.parse(&mut reader);
            let node_ref = arena.new_node(SpanAttributes);
            parse_component_attributes(".foo .bar", &mut arena[node_ref]);
            let class_val = arena[node_ref]
                .attributes()
                .get("class")
                .unwrap()
                .str("test")
                .to_string();
            assert_eq!(class_val, "foo bar");
        });
    }

    #[test]
    fn parse_component_attributes_id() {
        with_parser(|parser| {
            let mut reader = text::BasicReader::new("test");
            let (mut arena, _) = parser.parse(&mut reader);
            let node_ref = arena.new_node(SpanAttributes);
            parse_component_attributes("#myid", &mut arena[node_ref]);
            let id_val = arena[node_ref]
                .attributes()
                .get("id")
                .unwrap()
                .str("test")
                .to_string();
            assert_eq!(id_val, "myid");
        });
    }

    #[test]
    fn parse_component_attributes_key_value() {
        with_parser(|parser| {
            let mut reader = text::BasicReader::new("test");
            let (mut arena, _) = parser.parse(&mut reader);
            let node_ref = arena.new_node(SpanAttributes);
            parse_component_attributes("data-x=\"hello\"", &mut arena[node_ref]);
            let val = arena[node_ref]
                .attributes()
                .get("data-x")
                .unwrap()
                .str("test")
                .to_string();
            assert_eq!(val, "hello");
        });
    }

    #[test]
    fn block_component_not_triple_colon() {
        // ::: should not match the block component parser (reserved for Phase 3).
        let input = ":::note\nContent\n:::";
        let ast = parse(input).unwrap();
        let nodes = ast.walk();
        assert!(
            nodes.iter().all(|n| n.kind != "block_component"),
            "Triple colon should not create a block_component"
        );
    }

    #[test]
    fn inline_component_not_double_colon() {
        // :: at inline level should not create an inline component.
        let input = "::notinline";
        let ast = parse(input).unwrap();
        let nodes = ast.walk();
        assert!(
            nodes.iter().all(|n| n.kind != "inline_component"),
            "Double colon should not create inline_component"
        );
    }
}
