//! Integration test exercising the frozen public Rust surface (OMEP-0008 /
//! OMEP-0010) WITHOUT the `python` feature.
//!
//! This proves the crate is usable — and PyO3-independent — for downstream
//! Rust consumers (issue #20): it uses only crate-root items and is compiled
//! as an external `rlib` consumer without activating `--features python`.

use std::collections::HashMap;

use oxydemark::{
    AstNode, OxydeError, ParseResult, extract_summary, markdown_to_html, parse, parse_document,
    render_ast, slugify,
};

#[test]
fn parse_returns_document_tree() {
    let ast: AstNode = parse("# Hello **world**");
    assert_eq!(ast.kind, "document");
    let nodes = ast.walk();
    let kinds: Vec<&str> = nodes.iter().map(|n| n.kind.as_str()).collect();
    assert!(kinds.contains(&"heading"));
    assert!(kinds.contains(&"strong"));
}

#[test]
fn render_ast_round_trips() {
    let ast = parse("Hello **world**");
    let html = render_ast(&ast);
    assert!(html.contains("<p>"));
    assert!(html.contains("<strong>"));
    assert!(html.contains("world"));
}

#[test]
fn markdown_to_html_fast_path() {
    let html: String = markdown_to_html("# Title").expect("render should succeed");
    assert!(html.contains("<h1"));
    assert!(html.contains("Title"));
}

#[test]
fn markdown_to_html_error_is_oxyde_error() {
    // Type-level assertion: the fallible surface returns the pure-Rust
    // `OxydeError`, not `PyResult`.
    fn assert_result(r: Result<String, OxydeError>) -> Result<String, OxydeError> {
        r
    }
    let _ = assert_result(markdown_to_html("ok"));
}

#[test]
fn parse_document_exposes_typed_frontmatter() {
    let result: ParseResult = parse_document("---\ntitle: Hi\ncount: 5\n---\nBody");
    assert_eq!(result.root.kind, "document");
    // On the Rust surface, `frontmatter` is `Option<rushdown::ast::Meta>`.
    let fm = result.frontmatter.expect("frontmatter present");
    match fm {
        rushdown::ast::Meta::Mapping(map) => {
            assert!(map.get("title").is_some());
            assert!(map.get("count").is_some());
        }
        other => panic!("expected mapping frontmatter, got {other:?}"),
    }
}

#[test]
fn parse_document_without_frontmatter_is_none() {
    let result = parse_document("Just body text.");
    assert!(result.frontmatter.is_none());
}

#[test]
fn slugify_public() {
    assert_eq!(slugify("Hello World", None), "hello-world");
    assert_eq!(
        slugify("Overview", Some(vec!["overview".to_string()])),
        "overview-1"
    );
}

#[test]
fn extract_summary_public() {
    let summary = extract_summary("Intro.\n\n<!-- more -->\n\nBody.").expect("delimiter present");
    assert!(summary.contains("<p>Intro.</p>"));
    assert!(!summary.contains("Body"));
    assert!(extract_summary("No delimiter here.").is_none());
}

#[test]
fn ast_node_constructor_and_fields() {
    let node = AstNode::new(
        "text".to_string(),
        Vec::new(),
        Some("hi".to_string()),
        HashMap::new(),
        None,
    );
    assert_eq!(node.kind, "text");
    assert_eq!(node.text.as_deref(), Some("hi"));
}
