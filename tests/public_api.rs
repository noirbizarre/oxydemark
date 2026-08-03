//! Integration test exercising the frozen public Rust surface (OMEP-0008 /
//! OMEP-0010) WITHOUT the `python` feature.
//!
//! This proves the crate is usable — and PyO3-independent — for downstream
//! Rust consumers (issue #20): it uses only crate-root items and is compiled
//! as an external `rlib` consumer without activating `--features python`.

use std::collections::HashMap;

use oxydemark::{
    AstNode, Heading, Meta, OxydeError, ParseResult, extract_summary, markdown_to_html, parse,
    parse_document, render_ast, slugify,
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
    // On the Rust surface, `frontmatter` is `Option<Meta>`, re-exported from
    // the crate root (issue #34).
    let fm = result.frontmatter.expect("frontmatter present");
    match fm {
        Meta::Mapping(map) => {
            assert!(map.get("title").is_some());
            assert!(map.get("count").is_some());
        }
        other => panic!("expected mapping frontmatter, got {other:?}"),
    }
}

#[test]
fn meta_is_nameable_by_downstream_consumers() {
    // Regression guard for issue #34: a downstream crate depending only on
    // `oxydemark` must be able to *name* `Meta` -- in a struct field, in a
    // function signature, and in a `match` -- without depending on `rushdown`.
    struct Page {
        metadata: Option<Meta>,
    }

    fn kind(meta: &Meta) -> &'static str {
        match meta {
            Meta::Mapping(_) => "mapping",
            Meta::Sequence(_) => "sequence",
            Meta::String(_) => "string",
            _ => "scalar",
        }
    }

    let page = Page {
        metadata: parse_document("---\ntitle: Hi\ntags:\n  - a\n---\nBody").frontmatter,
    };
    let meta = page.metadata.as_ref().expect("frontmatter present");
    assert_eq!(kind(meta), "mapping");

    let Meta::Mapping(map) = meta else {
        panic!("expected mapping frontmatter, got {meta:?}");
    };
    assert_eq!(kind(map.get("title").expect("title")), "string");
    assert_eq!(kind(map.get("tags").expect("tags")), "sequence");
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
        slugify("Overview", Some(&["overview".to_string()])),
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

#[test]
fn parse_document_exposes_headings_and_toc() {
    let result: ParseResult =
        parse_document("# Title\n\n## Setup\n\n## Usage\n\n### CLI\n\n### Library\n\n## FAQ");

    let flat: Vec<(u8, &str)> = result
        .headings
        .iter()
        .map(|h: &Heading| (h.level, h.id.as_str()))
        .collect();
    assert_eq!(
        flat,
        [
            (1, "title"),
            (2, "setup"),
            (2, "usage"),
            (3, "cli"),
            (3, "library"),
            (2, "faq"),
        ]
    );
    assert!(result.headings.iter().all(|h| h.children.is_empty()));
    assert_eq!(result.headings[0].text, "Title");

    assert_eq!(result.toc.len(), 1);
    assert_eq!(result.toc[0].id, "title");
    let children: Vec<&str> = result.toc[0]
        .children
        .iter()
        .map(|h| h.id.as_str())
        .collect();
    assert_eq!(children, ["setup", "usage", "faq"]);
    let nested: Vec<&str> = result.toc[0].children[1]
        .children
        .iter()
        .map(|h| h.id.as_str())
        .collect();
    assert_eq!(nested, ["cli", "library"]);
}

#[test]
fn parse_document_toc_anchors_match_rendered_ids() {
    let markdown = "# Overview\n\n## Overview\n";
    let result = parse_document(markdown);
    let html = markdown_to_html(markdown).expect("renders");

    for heading in &result.headings {
        assert!(
            html.contains(&format!("id=\"{}\"", heading.id)),
            "missing id {} in {html}",
            heading.id
        );
    }
    assert_eq!(result.headings[1].id, "overview-1");
}

#[test]
fn parse_document_exposes_summary() {
    let result = parse_document("Intro.\n\n<!-- more -->\n\nBody.");
    let summary = result.summary.expect("delimiter present");
    assert!(summary.contains("<p>Intro.</p>"));
    assert!(!summary.contains("Body"));
    assert_eq!(
        Some(summary),
        extract_summary("Intro.\n\n<!-- more -->\n\nBody.")
    );

    assert!(parse_document("No delimiter here.").summary.is_none());
}
