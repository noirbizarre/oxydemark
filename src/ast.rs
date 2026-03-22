//! Python-friendly AST node and conversion from rushdown's arena AST.
//!
//! This module defines [`AstNode`], a tree-based AST representation designed
//! for easy traversal and modification from Python plugins.  It also provides
//! the conversion logic from rushdown's arena-based AST into `AstNode` trees.

use std::collections::HashMap;

use pyo3::prelude::*;
use rushdown::as_kind_data;
use rushdown::ast::{self, KindData, NodeRef, TextQualifier};
use rushdown_emoji::Emoji;

use crate::extensions::{BlockComponent, InlineComponent, SpanAttributes};

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
    pub fn walk(&self) -> Vec<AstNode> {
        let mut result = Vec::new();
        self.walk_recursive(&mut result);
        result
    }

    pub fn __repr__(&self) -> String {
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
pub(crate) fn arena_to_ast_node(arena: &ast::Arena, node_ref: NodeRef, source: &str) -> AstNode {
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
