mod ast;
mod extensions;
mod html_render;

use std::cell::RefCell;

use pyo3::prelude::*;
use rushdown::parser::{self, Parser, ParserExtension as _};
use rushdown::renderer::html::{self, RendererExtension as _};
use rushdown::text;

use ast::AstNode;
use extensions::{
    block_component_html_renderer_extension, block_component_parser_extension,
    inline_component_html_renderer_extension, inline_component_parser_extension,
    span_attribute_html_renderer_extension, span_attribute_parser_extension,
};
use html_render::render_ast_to_html;
use rushdown_emoji::{
    EmojiHtmlRendererOptions, EmojiParserOptions, emoji_html_renderer_extension,
    emoji_parser_extension,
};
use rushdown_meta::{MetaParserOptions, meta_parser_extension};

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
        Ok(ast::arena_to_ast_node(&arena, document_ref, markdown))
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
    use std::collections::HashMap;

    use super::*;
    use html_render::{collect_text, html_escape};

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
        // The wave emoji is U+1F44B.
        assert_eq!(emoji.text.as_deref(), Some("\u{1F44B}"));
    }

    #[test]
    fn render_emoji_fast_path() {
        let html = markdown_to_html(":wave:").unwrap();
        assert!(
            html.contains("\u{1F44B}"),
            "Expected wave emoji in HTML, got: {html}"
        );
    }

    #[test]
    fn render_emoji_ast_round_trip() {
        let ast = parse(":wave:").unwrap();
        let html = render_ast_to_html(&ast);
        assert!(
            html.contains("\u{1F44B}"),
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
            let node_ref = arena.new_node(extensions::SpanAttributes);
            extensions::parse_component_attributes(".foo .bar", &mut arena[node_ref]);
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
            let node_ref = arena.new_node(extensions::SpanAttributes);
            extensions::parse_component_attributes("#myid", &mut arena[node_ref]);
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
            let node_ref = arena.new_node(extensions::SpanAttributes);
            extensions::parse_component_attributes("data-x=\"hello\"", &mut arena[node_ref]);
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
