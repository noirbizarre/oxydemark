//! AstNode-to-HTML rendering.
//!
//! This module provides a standalone HTML renderer that operates on the
//! Python-friendly [`AstNode`](crate::ast::AstNode) tree.  It is used when
//! Python plugins have modified the AST and the result needs to be serialised
//! back to HTML.

use std::fmt::Write;

use crate::ast::AstNode;

/// Structural context threaded down the tree while rendering.
///
/// rushdown's renderer derives some markup from a node's ancestry rather than
/// from the node itself. The equivalent ancestry is carried here so that both
/// render paths stay byte-identical.
#[derive(Clone, Copy, Default)]
struct Ctx {
    /// Set while rendering the rows and cells of a `table_header`, which makes
    /// cells render as `<th>` instead of `<td>`.
    in_table_header: bool,
}

/// Render an `AstNode` tree to an HTML string.
///
/// This is a standalone Rust renderer that works on the Python-friendly
/// `AstNode` tree, independent of rushdown's renderer. It is used when
/// Python plugins have modified the AST.
pub(crate) fn render_ast_to_html(node: &AstNode) -> String {
    let mut output = String::new();
    render_node(&mut output, node, Ctx::default());
    output
}

fn render_node(w: &mut String, node: &AstNode, ctx: Ctx) {
    // Only the table rows of a `table_header` inherit the header context; every
    // other node starts from a clean slate.
    let child_ctx = match node.kind.as_str() {
        "table_header" => Ctx {
            in_table_header: true,
        },
        "table_row" => ctx,
        _ => Ctx::default(),
    };
    match node.kind.as_str() {
        "document" => render_children(w, node, child_ctx),
        "paragraph" => render_paragraph(w, node, false, None, false),
        "heading" => {
            let level = node
                .attributes
                .get("level")
                .and_then(|v| v.parse::<u8>().ok())
                .unwrap_or(1);
            let tag = format!("h{level}");
            let _ = write!(w, "<{tag}");
            render_html_attributes(w, node);
            w.push('>');
            render_children(w, node, child_ctx);
            let _ = writeln!(w, "</{tag}>");
        }
        "blockquote" => {
            w.push_str("<blockquote>\n");
            render_children(w, node, child_ctx);
            w.push_str("</blockquote>\n");
        }
        "list" => render_list(w, node),
        // A `list_item` reached outside a `list` has no tightness to inherit.
        "list_item" => render_list_item(w, node, None),
        "code_block" => {
            w.push_str("<pre><code");
            // rushdown derives the language class from the first word of the
            // fence info string.
            if let Some(language) = node
                .attributes
                .get("info")
                .and_then(|info| info.split(' ').next())
                .filter(|language| !language.is_empty())
            {
                let _ = write!(w, " class=\"language-{}\"", html_escape_attr(language));
            }
            w.push('>');
            if let Some(ref t) = node.text {
                w.push_str(&html_escape(t));
            }
            render_children(w, node, child_ctx);
            w.push_str("</code></pre>\n");
        }
        "thematic_break" => {
            w.push_str("<hr>\n");
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
            w.push_str("<br>\n");
        }
        "emphasis" => {
            w.push_str("<em>");
            render_children(w, node, child_ctx);
            w.push_str("</em>");
        }
        "strong" => {
            w.push_str("<strong>");
            render_children(w, node, child_ctx);
            w.push_str("</strong>");
        }
        "strikethrough" => {
            w.push_str("<del>");
            render_children(w, node, child_ctx);
            w.push_str("</del>");
        }
        "link" => {
            let href = node.attributes.get("href").map_or("", |v| v.as_str());
            let _ = write!(w, "<a href=\"{}\"", html_escape_attr(href));
            render_html_attributes(w, node);
            w.push('>');
            render_children(w, node, child_ctx);
            w.push_str("</a>");
        }
        "image" => {
            let src = node.attributes.get("src").map_or("", |v| v.as_str());
            let _ = write!(w, "<img src=\"{}\"", html_escape_attr(src));
            // `alt` comes right after `src` (and is always emitted, even when
            // empty) to match rushdown's attribute order byte for byte.
            let alt = collect_text(node);
            let _ = write!(w, " alt=\"{}\"", html_escape_attr(&alt));
            render_html_attributes(w, node);
            w.push('>');
        }
        "code_span" => {
            w.push_str("<code>");
            if let Some(ref t) = node.text {
                w.push_str(&html_escape(t));
            }
            render_children(w, node, child_ctx);
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
            render_children(w, node, child_ctx);
        }
        "table" => {
            w.push_str("<table>\n");
            render_children(w, node, child_ctx);
            w.push_str("</table>\n");
        }
        "table_header" => {
            w.push_str("<thead>\n");
            render_children(w, node, child_ctx);
            w.push_str("</thead>\n");
        }
        "table_body" => {
            w.push_str("<tbody>\n");
            render_children(w, node, child_ctx);
            w.push_str("</tbody>\n");
        }
        "table_row" => {
            w.push_str("<tr>\n");
            render_children(w, node, child_ctx);
            w.push_str("</tr>\n");
        }
        "table_cell" => {
            let tag = if ctx.in_table_header { "th" } else { "td" };
            let _ = write!(w, "<{tag}");
            if let Some(align) = node.attributes.get("align").filter(|a| !a.is_empty()) {
                let _ = write!(w, " style=\"text-align: {};\"", html_escape_attr(align));
            }
            w.push('>');
            render_children(w, node, child_ctx);
            let _ = writeln!(w, "</{tag}>");
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
            render_html_attributes(w, node);
            w.push_str(">\n");
            render_children(w, node, child_ctx);
            w.push_str("</div>\n");
        }
        "slot" => {
            // Slots render as <div data-slot="name"> wrappers.
            let name = node
                .attributes
                .get("name")
                .map(String::as_str)
                .unwrap_or("");
            let _ = writeln!(w, "<div data-slot=\"{}\">", html_escape_attr(name));
            render_children(w, node, child_ctx);
            w.push_str("</div>\n");
        }
        "inline_component" => {
            // Passthrough: render as a bare <span> with attributes.
            w.push_str("<span");
            render_html_attributes(w, node);
            w.push('>');
            render_children(w, node, child_ctx);
            w.push_str("</span>");
        }
        "span_attributes" => {
            // Span attributes: render as <span> with attributes.
            w.push_str("<span");
            render_html_attributes(w, node);
            w.push('>');
            render_children(w, node, child_ctx);
            w.push_str("</span>");
        }
        _ => {
            // Unknown node types: render children transparently.
            render_children(w, node, child_ctx);
        }
    }
}

/// Render a `list` node as `<ul>`/`<ol>`, propagating tightness to its items.
fn render_list(w: &mut String, node: &AstNode) {
    let ordered = node.attributes.get("ordered").is_some_and(|v| v == "true");
    let tag = if ordered { "ol" } else { "ul" };
    // A list with no explicit `tight` attribute (e.g. one built by a plugin) is
    // treated as tight, which is the common Markdown case.
    let tight = node.attributes.get("tight").is_none_or(|v| v == "true");
    let _ = write!(w, "<{tag}");
    if ordered && let Some(start) = node.attributes.get("start").filter(|s| *s != "1") {
        let _ = write!(w, " start=\"{}\"", html_escape_attr(start));
    }
    w.push_str(">\n");
    for child in &node.children {
        if child.kind == "list_item" {
            render_list_item(w, child, Some(tight));
        } else {
            render_node(w, child, Ctx::default());
        }
    }
    let _ = writeln!(w, "</{tag}>");
}

/// Render a `list_item`, unwrapping paragraphs when the parent list is tight.
///
/// `list_tight` is `None` when the item has no `list` parent, which suppresses
/// the newline rushdown only emits for items inside a list.
fn render_list_item(w: &mut String, node: &AstNode, list_tight: Option<bool>) {
    w.push_str("<li>");
    let tight = list_tight == Some(true);
    if list_tight.is_some()
        && let Some(first) = node.children.first()
        && (!tight || first.kind != "paragraph")
    {
        w.push('\n');
    }
    // The task checkbox is emitted by the item's first paragraph, as rushdown
    // does, so that it lands inside the `<p>` of a loose list.
    let mut task = node.attributes.get("task").map(String::as_str);
    for (index, child) in node.children.iter().enumerate() {
        if child.kind == "paragraph" {
            let has_next = index + 1 < node.children.len();
            render_paragraph(w, child, tight, task.take(), has_next);
        } else {
            render_node(w, child, Ctx::default());
        }
    }
    w.push_str("</li>\n");
}

/// Render a `paragraph`, optionally without its `<p>` wrapper.
///
/// Paragraphs directly inside a tight list item render bare, followed by a
/// newline when another sibling follows (e.g. a nested list).
fn render_paragraph(
    w: &mut String,
    node: &AstNode,
    bare: bool,
    task: Option<&str>,
    has_next: bool,
) {
    if !bare {
        w.push_str("<p>");
    }
    if let Some(task) = task {
        w.push_str(if task == "completed" {
            r#"<input checked="" disabled="" type="checkbox"> "#
        } else {
            r#"<input disabled="" type="checkbox"> "#
        });
    }
    render_children(w, node, Ctx::default());
    if !bare {
        w.push_str("</p>\n");
    } else if has_next && !node.children.is_empty() {
        w.push('\n');
    }
}

fn render_children(w: &mut String, node: &AstNode, ctx: Ctx) {
    for child in &node.children {
        render_node(w, child, ctx);
    }
}

/// Render HTML attributes from the node, filtered and ordered deterministically.
///
/// Only keys accepted by [`is_renderable_attribute`] are emitted, and they are
/// sorted by name so this renderer matches the fast path byte for byte.
fn render_html_attributes(w: &mut String, node: &AstNode) {
    let mut keys: Vec<&String> = node
        .attributes
        .keys()
        .filter(|key| is_renderable_attribute(key))
        .collect();
    keys.sort_unstable();
    for key in keys {
        let _ = write!(
            w,
            " {}=\"{}\"",
            key,
            html_escape_attr(&node.attributes[key])
        );
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

/// Returns `true` when an attribute key is safe to emit into default HTML output.
///
/// Per OMEP-0007 only HTML-valid inline attributes reach the component element.
/// Internal keys (such as the synthetic `name`) and `:`-prefixed typed or
/// boolean props are dropped from HTML while remaining available in the AST.
pub(crate) fn is_renderable_attribute(key: &str) -> bool {
    matches!(
        key,
        "class" | "id" | "style" | "title" | "role" | "lang" | "dir"
    ) || key.starts_with("data-")
        || key.starts_with("aria-")
}
