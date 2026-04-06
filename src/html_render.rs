//! AstNode-to-HTML rendering.
//!
//! This module provides a standalone HTML renderer that operates on the
//! Python-friendly [`AstNode`](crate::ast::AstNode) tree.  It is used when
//! Python plugins have modified the AST and the result needs to be serialised
//! back to HTML.

use std::fmt::Write;

use crate::ast::AstNode;

/// Render an `AstNode` tree to an HTML string.
///
/// This is a standalone Rust renderer that works on the Python-friendly
/// `AstNode` tree, independent of rushdown's renderer. It is used when
/// Python plugins have modified the AST.
pub(crate) fn render_ast_to_html(node: &AstNode) -> String {
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
            render_html_attributes(w, node, &["href"]);
            w.push('>');
            render_children(w, node);
            w.push_str("</a>");
        }
        "image" => {
            let src = node.attributes.get("src").map_or("", |v| v.as_str());
            write!(w, "<img src=\"{}\"", html_escape_attr(src)).unwrap();
            render_html_attributes(w, node, &["src"]);
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
        "span_attributes" => {
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
pub(crate) fn collect_text(node: &AstNode) -> String {
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
pub(crate) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Escape HTML special characters in attribute values.
fn html_escape_attr(s: &str) -> String {
    html_escape(s)
}
