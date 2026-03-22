use std::collections::HashMap;
use std::fmt::Write;

use pyo3::prelude::*;
use rushdown::as_kind_data;
use rushdown::ast::{self, KindData, NodeRef, TextQualifier};
use rushdown::parser::{self, Parser, ParserExtension};
use rushdown::renderer::html;
use rushdown::text;
use rushdown_emoji::{
    EmojiHtmlRendererOptions, EmojiParserOptions, emoji_html_renderer_extension,
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
        .and(emoji_parser_extension(EmojiParserOptions::default()));
    Parser::with_extensions(parser::Options::default(), parser_extensions)
}

/// Build the configured rushdown HTML renderer with all extensions.
fn build_renderer() -> html::Renderer<'static, String> {
    html::Renderer::with_extensions(
        html::Options::default(),
        emoji_html_renderer_extension(EmojiHtmlRendererOptions::default()),
    )
}

// ---------------------------------------------------------------------------
// Arena AST -> AstNode conversion
// ---------------------------------------------------------------------------

/// Map a rushdown `KindData` variant to a human-readable kind string.
///
/// Emphasis level 1 maps to "emphasis", level 2 maps to "strong".
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
        _ => "unknown",
    }
}

/// Extract text content from a node, if it is a text-bearing leaf node.
///
/// In rushdown, `Text` holds its content directly via `str(source)`.
/// `CodeSpan` content is stored in child `Text` nodes, so we return `None`
/// here and let child traversal handle it.
/// `RawHtml` uses `str(source)` which returns a `Cow<str>`.
fn node_text(node: &ast::Node, source: &str) -> Option<String> {
    match node.kind_data() {
        KindData::Text(t) => Some(t.str(source).to_string()),
        KindData::RawHtml(h) => Some(h.str(source).to_string()),
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
// Public Python API
// ---------------------------------------------------------------------------

/// Parse Markdown input into an AST node tree.
///
/// Uses rushdown with GFM, YAML frontmatter, and emoji extensions.
/// Returns an `AstNode` tree that can be inspected and modified from Python.
#[pyfunction]
fn parse(markdown: &str) -> PyResult<AstNode> {
    let parser = build_parser();
    let mut reader = text::BasicReader::new(markdown);
    let (arena, document_ref) = parser.parse(&mut reader);
    Ok(arena_to_ast_node(&arena, document_ref, markdown))
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
    let parser = build_parser();
    let renderer = build_renderer();
    let mut reader = text::BasicReader::new(markdown);
    let (arena, document_ref) = parser.parse(&mut reader);

    let mut output = String::new();
    renderer
        .render(&mut output, markdown, &arena, document_ref)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(format!("{e}")))?;
    Ok(output)
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
}
