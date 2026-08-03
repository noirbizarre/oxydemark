//! Python-friendly AST node and conversion from rushdown's arena AST.
//!
//! This module defines [`AstNode`], a tree-based AST representation designed
//! for easy traversal and modification from Python plugins.  It also provides
//! the conversion logic from rushdown's arena-based AST into `AstNode` trees.

use std::collections::HashMap;

#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::{PyDict, PyList};
use rushdown::as_kind_data;
use rushdown::ast::{self, KindData, NodeRef, TextQualifier};
use rushdown_emoji::Emoji;

use crate::extensions::{BlockComponent, InlineComponent, Slot, SpanAttributes};

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
// Field accessors are defined as explicit `#[getter]`/`#[setter]` methods in the
// `python`-gated `#[pymethods]` block below (rather than `get_all`/`set_all` on
// the `#[pyclass]` attribute) so that the typed `props` field can be exposed as
// a native Python `dict` via a computed getter, mirroring
// [`ParseResult::frontmatter`].
#[cfg_attr(feature = "python", pyclass(from_py_object))]
#[derive(Clone, Debug)]
pub struct AstNode {
    /// The node kind (e.g. "document", "paragraph", "text", "heading").
    pub kind: String,

    /// Child nodes.
    pub children: Vec<AstNode>,

    /// Text content for leaf nodes (e.g. "text", "code_span").
    pub text: Option<String>,

    /// HTML attributes attached to this node.
    pub attributes: HashMap<String, String>,

    /// YAML frontmatter metadata (only present on the "document" node).
    ///
    /// **Deprecated** in favour of [`ParseResult::frontmatter`] (see
    /// OMEP-0010). This map is stringly-typed: every YAML value is coerced to a
    /// string, so non-string values (numbers, booleans, sequences, mappings)
    /// are lossy. It is retained for backward compatibility and will be removed
    /// in a pre-1.0 release. Use [`crate::parse_document`] and
    /// `ParseResult.frontmatter` for typed access.
    pub metadata: Option<HashMap<String, String>>,

    /// Typed block-component props from a leading YAML block (OMEP-0007).
    ///
    /// Present (a [`ast::Meta::Mapping`]) only on `block_component` nodes that
    /// declare a leading YAML block, and `None` otherwise. Unlike
    /// [`AstNode::attributes`], values preserve their native YAML types
    /// (numbers, booleans, sequences, mappings). Inline `{…}` attributes take
    /// precedence: keys that also appear as inline attributes are dropped from
    /// `props`. The Python binding exposes this as a read-only `dict`.
    pub props: Option<ast::Meta>,
}

impl AstNode {
    /// Create a new AST node.
    ///
    /// This is the pure-Rust constructor. The Python `AstNode(...)` constructor
    /// (with keyword defaults) is defined in the `python`-gated `#[pymethods]`
    /// block and delegates here.
    pub fn new(
        kind: String,
        children: Vec<AstNode>,
        text: Option<String>,
        attributes: HashMap<String, String>,
        metadata: Option<HashMap<String, String>>,
    ) -> Self {
        AstNode {
            kind,
            children,
            text,
            attributes,
            metadata,
            props: None,
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

    fn walk_recursive(&self, result: &mut Vec<AstNode>) {
        result.push(self.clone());
        for child in &self.children {
            child.walk_recursive(result);
        }
    }

    /// Human-readable representation shared by the Python `__repr__` and the
    /// crate's tests. Only compiled where it is actually used.
    #[cfg(any(test, feature = "python"))]
    pub(crate) fn repr_string(&self) -> String {
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

#[cfg(feature = "python")]
#[pymethods]
impl AstNode {
    /// Create a new AST node (Python constructor).
    #[new]
    #[pyo3(signature = (kind, children=None, text=None, attributes=None, metadata=None))]
    fn py_new(
        kind: String,
        children: Option<Vec<AstNode>>,
        text: Option<String>,
        attributes: Option<HashMap<String, String>>,
        metadata: Option<HashMap<String, String>>,
    ) -> Self {
        AstNode::new(
            kind,
            children.unwrap_or_default(),
            text,
            attributes.unwrap_or_default(),
            metadata,
        )
    }

    /// The node kind.
    #[getter]
    fn get_kind(&self) -> String {
        self.kind.clone()
    }

    #[setter]
    fn set_kind(&mut self, value: String) {
        self.kind = value;
    }

    /// Child nodes.
    #[getter]
    fn get_children(&self) -> Vec<AstNode> {
        self.children.clone()
    }

    #[setter]
    fn set_children(&mut self, value: Vec<AstNode>) {
        self.children = value;
    }

    /// Text content for leaf nodes.
    #[getter]
    fn get_text(&self) -> Option<String> {
        self.text.clone()
    }

    #[setter]
    fn set_text(&mut self, value: Option<String>) {
        self.text = value;
    }

    /// HTML attributes attached to this node.
    #[getter]
    fn get_attributes(&self) -> HashMap<String, String> {
        self.attributes.clone()
    }

    #[setter]
    fn set_attributes(&mut self, value: HashMap<String, String>) {
        self.attributes = value;
    }

    /// Deprecated stringly-typed YAML frontmatter (document node only).
    #[getter]
    fn get_metadata(&self) -> Option<HashMap<String, String>> {
        self.metadata.clone()
    }

    #[setter]
    fn set_metadata(&mut self, value: Option<HashMap<String, String>>) {
        self.metadata = value;
    }

    /// Typed block-component props as a Python `dict`, or `None` (read-only).
    ///
    /// Values preserve their native YAML types (`str`, `int`, `float`, `bool`,
    /// `list`, `dict`, `None`). This attribute has no setter.
    #[getter]
    fn get_props(&self, py: Python<'_>) -> PyResult<Option<Py<PyDict>>> {
        match &self.props {
            None => Ok(None),
            Some(ast::Meta::Mapping(map)) => {
                let dict = PyDict::new(py);
                for (k, v) in map.iter() {
                    dict.set_item(k, meta_to_py(py, v)?)?;
                }
                Ok(Some(dict.unbind()))
            }
            // `props` is always a mapping when set; be defensive rather than
            // panicking for any other shape.
            Some(other) => {
                let dict = PyDict::new(py);
                dict.set_item("value", meta_to_py(py, other)?)?;
                Ok(Some(dict.unbind()))
            }
        }
    }

    /// Walk the AST tree depth-first, returning a flat list of all nodes.
    #[pyo3(name = "walk")]
    fn py_walk(&self) -> Vec<AstNode> {
        self.walk()
    }

    fn __repr__(&self) -> String {
        self.repr_string()
    }
}

// ---------------------------------------------------------------------------
// Heading / table of contents (OMEP-0010)
// ---------------------------------------------------------------------------

/// A document heading, used both as a flat entry of
/// [`ParseResult::headings`] and as a node of the nested
/// [`ParseResult::toc`] tree (OMEP-0010).
///
/// # Examples
///
/// From Python:
/// ```python
/// result = oxydemark.parse_document("# Title\n## Setup")
/// [(h.level, h.id) for h in result.headings]  # [(1, 'title'), (2, 'setup')]
/// result.toc[0].children[0].id                # 'setup'
/// ```
#[cfg_attr(feature = "python", pyclass(skip_from_py_object))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heading {
    /// Heading level, 1 to 6.
    pub level: u8,

    /// The anchor id assigned to the heading (see [`crate::slugify`]).
    pub id: String,

    /// Plain-text heading label (the same text the slug is derived from,
    /// before slugging).
    pub text: String,

    /// Nested sub-headings.
    ///
    /// Always empty for entries of the flat [`ParseResult::headings`] list;
    /// populated only in the [`ParseResult::toc`] tree.
    pub children: Vec<Heading>,
}

#[cfg(feature = "python")]
#[pymethods]
impl Heading {
    /// Heading level, 1 to 6.
    #[getter]
    fn level(&self) -> u8 {
        self.level
    }

    /// The anchor id assigned to the heading.
    #[getter]
    fn id(&self) -> String {
        self.id.clone()
    }

    /// Plain-text heading label.
    #[getter]
    fn text(&self) -> String {
        self.text.clone()
    }

    /// Nested sub-headings (empty in the flat `headings` list).
    #[getter]
    fn children(&self) -> Vec<Heading> {
        self.children.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "Heading(level={}, id={:?}, text={:?}, children={})",
            self.level,
            self.id,
            self.text,
            self.children.len()
        )
    }
}

/// Collect every heading of the document, in document order, as a flat list.
///
/// Anchors must already have been assigned by
/// [`crate::extensions::assign_heading_anchors`]; the `id` is read from the
/// heading node's attributes. Headings nested inside block components and
/// slots are included, in document order like any other heading. The returned
/// entries always have an empty `children` list; use [`build_toc`] to derive
/// the nested tree.
pub(crate) fn collect_headings(
    arena: &ast::Arena,
    document_ref: NodeRef,
    source: &str,
) -> Vec<Heading> {
    let mut refs: Vec<NodeRef> = Vec::new();
    crate::extensions::collect_headings(arena, document_ref, &mut refs);

    refs.into_iter()
        .map(|heading_ref| {
            let node = &arena[heading_ref];
            let level = match node.kind_data() {
                KindData::Heading(h) => h.level(),
                // `collect_headings` only ever yields heading nodes.
                _ => 0,
            };
            let id = node
                .attributes()
                .get("id")
                .map(|v| v.str(source).to_string())
                .unwrap_or_default();
            Heading {
                level,
                id,
                text: crate::extensions::heading_text(arena, heading_ref, source),
                children: Vec::new(),
            }
        })
        .collect()
}

/// Build the nested table-of-contents tree from a flat, document-ordered
/// heading list (OMEP-0010).
///
/// A heading of level `L` becomes a child of the nearest preceding heading
/// whose level is strictly lower; headings with no shallower ancestor become
/// roots. Level skips are tolerated: `#` followed directly by `###` nests the
/// `###` under the `#`.
pub(crate) fn build_toc(flat: &[Heading]) -> Vec<Heading> {
    let mut roots: Vec<Heading> = Vec::new();
    // Path of indices from `roots` down to the currently open heading.
    let mut stack: Vec<usize> = Vec::new();

    for heading in flat {
        let node = Heading {
            level: heading.level,
            id: heading.id.clone(),
            text: heading.text.clone(),
            children: Vec::new(),
        };

        // Pop until the open ancestor is strictly shallower than `heading`.
        while !stack.is_empty() {
            let parent = heading_at(&roots, &stack);
            if parent.level < node.level {
                break;
            }
            stack.pop();
        }

        if stack.is_empty() {
            roots.push(node);
            stack.push(roots.len() - 1);
        } else {
            let parent = heading_at_mut(&mut roots, &stack);
            parent.children.push(node);
            let index = parent.children.len() - 1;
            stack.push(index);
        }
    }

    roots
}

/// Resolve the heading addressed by `path` (a list of child indices).
fn heading_at<'a>(roots: &'a [Heading], path: &[usize]) -> &'a Heading {
    let mut node = &roots[path[0]];
    for &index in &path[1..] {
        node = &node.children[index];
    }
    node
}

/// Mutable counterpart of [`heading_at`].
fn heading_at_mut<'a>(roots: &'a mut [Heading], path: &[usize]) -> &'a mut Heading {
    let mut node = &mut roots[path[0]];
    for &index in &path[1..] {
        node = &mut node.children[index];
    }
    node
}

// ---------------------------------------------------------------------------
// ParseResult (OMEP-0010)
// ---------------------------------------------------------------------------

/// The result of [`crate::parse_document`]: the AST tree plus structured,
/// typed document metadata.
///
/// This is the metadata-aware counterpart to [`crate::parse`]. It bundles the
/// same `AstNode` tree (reachable via [`ParseResult::root`]) with typed YAML
/// frontmatter, superseding the lossy stringly-typed [`AstNode::metadata`].
///
/// # Examples
///
/// From Python:
/// ```python
/// result = oxydemark.parse_document("---\ntitle: Hi\ncount: 5\n---\nBody")
/// result.frontmatter["title"]  # "Hi"
/// result.frontmatter["count"]  # 5 (an int, not "5")
/// result.root.kind             # "document"
/// ```
#[cfg_attr(feature = "python", pyclass)]
pub struct ParseResult {
    /// The parsed AST tree (identical to what [`crate::parse`] returns).
    ///
    /// Exposed to Python via a computed getter (see the `python`-gated
    /// `#[pymethods]` block) rather than a field-level `#[pyo3(get)]`, which
    /// cannot be `#[cfg_attr]`-gated in PyO3 0.28.
    pub root: AstNode,

    /// Every heading of the document, in document order, as a flat list.
    ///
    /// Entries always have an empty `children` list; the nesting lives in
    /// [`ParseResult::toc`].
    pub headings: Vec<Heading>,

    /// The nested table-of-contents tree derived from
    /// [`ParseResult::headings`].
    pub toc: Vec<Heading>,

    /// Rendered HTML of the content preceding the `<!-- more -->` delimiter,
    /// or `None` when the document has no top-level delimiter.
    ///
    /// Identical to what [`crate::extract_summary`] returns, but computed as
    /// part of the same parse.
    pub summary: Option<String>,

    /// Typed YAML frontmatter, or `None` when the document has no frontmatter.
    ///
    /// On the pure-Rust surface this is a [`rushdown::ast::Meta`] mapping,
    /// preserving native YAML types. The Python binding exposes it as a `dict`
    /// via a computed getter (see the `python`-gated `#[pymethods]` block),
    /// where values become native Python objects (`str`, `int`, `float`,
    /// `bool`, `list`, `dict`, `None`).
    pub frontmatter: Option<ast::Meta>,
}

#[cfg(feature = "python")]
#[pymethods]
impl ParseResult {
    /// The parsed AST tree.
    #[getter]
    fn root(&self) -> AstNode {
        self.root.clone()
    }

    /// Every heading of the document, in document order (flat).
    #[getter]
    fn headings(&self) -> Vec<Heading> {
        self.headings.clone()
    }

    /// The nested table-of-contents tree.
    #[getter]
    fn toc(&self) -> Vec<Heading> {
        self.toc.clone()
    }

    /// Rendered HTML of the content before `<!-- more -->`, or `None`.
    #[getter]
    fn summary(&self) -> Option<String> {
        self.summary.clone()
    }

    /// Typed YAML frontmatter as a Python `dict`, or `None`.
    #[getter]
    fn frontmatter(&self, py: Python<'_>) -> PyResult<Option<Py<PyDict>>> {
        match &self.frontmatter {
            None => Ok(None),
            Some(ast::Meta::Mapping(map)) => {
                let dict = PyDict::new(py);
                for (k, v) in map.iter() {
                    dict.set_item(k, meta_to_py(py, v)?)?;
                }
                Ok(Some(dict.unbind()))
            }
            // Document frontmatter is always a mapping; be defensive for any
            // other shape rather than panicking.
            Some(other) => {
                let dict = PyDict::new(py);
                dict.set_item("value", meta_to_py(py, other)?)?;
                Ok(Some(dict.unbind()))
            }
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "ParseResult(root={:?}, frontmatter={})",
            self.root.kind,
            if self.frontmatter.is_some() {
                "{...}"
            } else {
                "None"
            }
        )
    }
}

/// Recursively convert a rushdown [`ast::Meta`] value into a native Python
/// object.
///
/// Each YAML value keeps its native type:
///
/// * `Null` -> `None`
/// * `Bool` -> `bool`
/// * `Int` -> `int`
/// * `Float` -> `float`
/// * `String` -> `str`
/// * `Sequence` -> `list` (elements converted recursively)
/// * `Mapping` -> `dict` (values converted recursively; keys are `str`)
///
/// This converter is intentionally standalone so it can be reused for the
/// typed component `props` representation (OMEP-0007) in a later change.
#[cfg(feature = "python")]
pub(crate) fn meta_to_py(py: Python<'_>, meta: &ast::Meta) -> PyResult<Py<PyAny>> {
    let value: Py<PyAny> = match meta {
        ast::Meta::Null => py.None(),
        ast::Meta::Bool(b) => b.into_pyobject(py)?.to_owned().into_any().unbind(),
        ast::Meta::Int(i) => i.into_pyobject(py)?.into_any().unbind(),
        ast::Meta::Float(f) => f.into_pyobject(py)?.into_any().unbind(),
        ast::Meta::String(s) => s.into_pyobject(py)?.into_any().unbind(),
        ast::Meta::Sequence(items) => {
            let list = PyList::empty(py);
            for item in items {
                list.append(meta_to_py(py, item)?)?;
            }
            list.into_any().unbind()
        }
        ast::Meta::Mapping(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map.iter() {
                dict.set_item(k, meta_to_py(py, v)?)?;
            }
            dict.into_any().unbind()
        }
    };
    Ok(value)
}

/// Build the typed frontmatter for a document node as a pure-Rust
/// [`ast::Meta`] mapping.
///
/// Returns `None` when the node is not a document or carries no frontmatter (an
/// empty metadata map), otherwise a [`ast::Meta::Mapping`] of top-level keys to
/// typed values, preserving the frontmatter's insertion order. The Python
/// binding converts this to a `dict` via [`meta_to_py`].
pub(crate) fn document_meta(arena: &ast::Arena, document_ref: NodeRef) -> Option<ast::Meta> {
    let node = &arena[document_ref];
    if !matches!(node.kind_data(), KindData::Document(_)) {
        return None;
    }
    let meta = as_kind_data!(arena, document_ref, Document).metadata();
    if meta.is_empty() {
        return None;
    }
    Some(ast::Meta::Mapping(meta.clone()))
}

// ---------------------------------------------------------------------------
// Arena AST -> AstNode conversion
// ---------------------------------------------------------------------------

/// Map a rushdown `KindData` variant to a human-readable kind string.
///
/// `Emphasis` maps to "emphasis" and `Strong` to "strong".
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
        KindData::Emphasis(_) => "emphasis",
        KindData::Strong(_) => "strong",
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
                .downcast_ref::<Slot>()
                .is_some()
            {
                "slot"
            } else if (ext.as_ref() as &dyn std::any::Any)
                .downcast_ref::<SpanAttributes>()
                .is_some()
            {
                "span_attributes"
            } else {
                "unknown"
            }
        }
        _ => "unknown",
    }
}

/// Placeholder substituted for raw HTML coming from the Markdown source.
///
/// rushdown renders with `allows_unsafe: false` and replaces raw HTML with this
/// comment. Applying the same substitution when the arena is converted to the
/// `AstNode` tree keeps both render paths byte-identical and prevents unsanitised
/// source markup from reaching the standalone renderer. A `raw_html` node
/// *created by a plugin* is unaffected and still renders its own `text` verbatim.
const RAW_HTML_PLACEHOLDER: &str = "<!-- raw HTML omitted -->";

/// Extract text content from a node, if it is a text-bearing leaf node.
///
/// In rushdown, `Text` holds its content directly via `str(source)`.
/// `CodeSpan` content is stored in child `Text` nodes, so we return `None`
/// here and let child traversal handle it.
/// `RawHtml` and `HtmlBlock` yield [`RAW_HTML_PLACEHOLDER`] rather than the
/// source markup (the block form keeps rushdown's trailing newline).
/// `Emoji` extension nodes produce their Unicode character.
///
/// `CodeSpan` carries its content inline (rushdown 0.18 dropped the child
/// `Text` node), so its value is surfaced here to keep the exposed tree -- and
/// therefore [`crate::api::render_ast`] -- unchanged for plugin authors.
fn node_text(node: &ast::Node, source: &str) -> Option<String> {
    match node.kind_data() {
        KindData::Text(t) => Some(t.str(source).to_string()),
        KindData::CodeSpan(c) => Some(c.str(source).to_string()),
        KindData::RawHtml(_) => Some(RAW_HTML_PLACEHOLDER.to_string()),
        KindData::HtmlBlock(_) => Some(format!("{RAW_HTML_PLACEHOLDER}\n")),
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
            if let Some(title) = link.title().filter(|t| !t.str(source).is_empty()) {
                attrs.insert("title".to_string(), title.str(source).to_string());
            }
        }
        KindData::Image(img) => {
            attrs.insert("src".to_string(), img.destination().str(source).to_string());
            if let Some(title) = img.title().filter(|t| !t.str(source).is_empty()) {
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
            } else if let Some(slot) = (ext.as_ref() as &dyn std::any::Any).downcast_ref::<Slot>() {
                attrs.insert("name".to_string(), slot.name.clone());
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

/// Extract a block component's typed YAML props, if any (OMEP-0007).
///
/// Only `block_component` nodes carry props. Inline `{…}` attributes take
/// precedence: any prop key that also appears as an inline attribute
/// (`node.attributes()`) is dropped. Returns `None` when the node has no props
/// or every prop key collided with an inline attribute.
fn node_props(node: &ast::Node) -> Option<ast::Meta> {
    let KindData::Extension(ext) = node.kind_data() else {
        return None;
    };
    let bc = (ext.as_ref() as &dyn std::any::Any).downcast_ref::<BlockComponent>()?;
    let ast::Meta::Mapping(map) = bc.props.as_ref()? else {
        return None;
    };

    // Inline attribute keys (the rushdown attribute map, excluding the
    // synthetic "name" injected during AstNode conversion).
    let inline: std::collections::HashSet<&str> =
        node.attributes().iter().map(|(k, _)| k.as_str()).collect();

    let mut filtered = rushdown::util::StringMap::default();
    for (key, value) in map.iter() {
        if inline.contains(key.as_str()) {
            continue;
        }
        filtered.insert(key.clone(), value.clone());
    }

    if filtered.is_empty() {
        None
    } else {
        Some(ast::Meta::Mapping(filtered))
    }
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
    let props = node_props(node);

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
                props: None,
            });
        } else if is_hardbreak(child_node) {
            children.push(AstNode {
                kind: "hardbreak".to_string(),
                children: Vec::new(),
                text: None,
                attributes: HashMap::new(),
                metadata: None,
                props: None,
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
        props,
    }
}

/// Collect the top-level blocks that precede a `<!-- more -->` summary delimiter.
///
/// Only direct children of the `document` node are considered, and only the
/// **first** such delimiter is significant (per OMEP-0010); delimiters nested
/// inside other blocks are ignored. Returns the `AstNode`s appearing *before*
/// the delimiter when one is present, or `None` when the document has no
/// top-level delimiter. The returned vector may be empty when the delimiter is
/// itself the first top-level block.
pub(crate) fn extract_summary_blocks(
    arena: &ast::Arena,
    document_ref: NodeRef,
    source: &str,
) -> Option<Vec<AstNode>> {
    let mut before = Vec::new();
    let mut child = arena[document_ref].first_child();
    while let Some(child_ref) = child {
        if crate::extensions::is_more_marker(arena, child_ref, source) {
            return Some(before);
        }
        before.push(arena_to_ast_node(arena, child_ref, source));
        child = arena[child_ref].next_sibling();
    }
    None
}

#[cfg(test)]
mod toc_tests {
    use super::*;

    /// Build a flat heading entry (as [`collect_headings`] would produce).
    fn flat(level: u8, id: &str) -> Heading {
        Heading {
            level,
            id: id.to_string(),
            text: id.to_string(),
            children: Vec::new(),
        }
    }

    /// Collect the ids of a heading list, one level deep.
    fn ids(headings: &[Heading]) -> Vec<&str> {
        headings.iter().map(|h| h.id.as_str()).collect()
    }

    #[test]
    fn empty_input_yields_empty_tree() {
        assert!(build_toc(&[]).is_empty());
    }

    #[test]
    fn omep_example_nests_as_specified() {
        let flat_list = vec![
            flat(1, "title"),
            flat(2, "setup"),
            flat(2, "usage"),
            flat(3, "cli"),
            flat(3, "library"),
            flat(2, "faq"),
        ];
        let toc = build_toc(&flat_list);

        assert_eq!(ids(&toc), ["title"]);
        assert_eq!(ids(&toc[0].children), ["setup", "usage", "faq"]);
        assert_eq!(ids(&toc[0].children[1].children), ["cli", "library"]);
        assert!(toc[0].children[0].children.is_empty());
    }

    #[test]
    fn level_skips_are_tolerated() {
        let toc = build_toc(&[flat(1, "a"), flat(3, "b")]);
        assert_eq!(ids(&toc), ["a"]);
        assert_eq!(ids(&toc[0].children), ["b"]);
        assert_eq!(toc[0].children[0].level, 3);
    }

    #[test]
    fn siblings_at_top_level_are_multiple_roots() {
        let toc = build_toc(&[flat(1, "a"), flat(1, "b"), flat(2, "b1")]);
        assert_eq!(ids(&toc), ["a", "b"]);
        assert!(toc[0].children.is_empty());
        assert_eq!(ids(&toc[1].children), ["b1"]);
    }

    #[test]
    fn shallower_heading_pops_open_ancestors() {
        let toc = build_toc(&[flat(2, "a"), flat(4, "a1"), flat(3, "a2"), flat(2, "b")]);
        assert_eq!(ids(&toc), ["a", "b"]);
        assert_eq!(ids(&toc[0].children), ["a1", "a2"]);
    }

    #[test]
    fn document_starting_deep_still_has_roots() {
        let toc = build_toc(&[flat(3, "a"), flat(2, "b")]);
        assert_eq!(ids(&toc), ["a", "b"]);
    }

    #[test]
    fn flat_entries_are_left_untouched() {
        let flat_list = vec![flat(1, "a"), flat(2, "b")];
        let _ = build_toc(&flat_list);
        assert!(flat_list.iter().all(|h| h.children.is_empty()));
    }
}

#[cfg(all(test, feature = "python"))]
mod tests {
    use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyList, PyString};

    use super::*;
    use rushdown::ast::Meta;
    use rushdown::util::StringMap;

    #[test]
    fn meta_to_py_scalars_preserve_native_types() {
        Python::attach(|py| {
            assert!(meta_to_py(py, &Meta::Null).unwrap().is_none(py));

            let b = meta_to_py(py, &Meta::Bool(true)).unwrap();
            let b = b.bind(py);
            assert!(b.is_instance_of::<PyBool>());
            assert!(b.extract::<bool>().unwrap());

            let i = meta_to_py(py, &Meta::Int(5)).unwrap();
            let i = i.bind(py);
            assert!(i.is_instance_of::<PyInt>());
            assert_eq!(i.extract::<i64>().unwrap(), 5);

            let f = meta_to_py(py, &Meta::Float(1.5)).unwrap();
            let f = f.bind(py);
            assert!(f.is_instance_of::<PyFloat>());
            assert_eq!(f.extract::<f64>().unwrap(), 1.5);

            let s = meta_to_py(py, &Meta::String("hi".to_string())).unwrap();
            let s = s.bind(py);
            assert!(s.is_instance_of::<PyString>());
            assert_eq!(s.extract::<String>().unwrap(), "hi");
        });
    }

    #[test]
    fn meta_to_py_sequence_becomes_list() {
        Python::attach(|py| {
            let seq = Meta::Sequence(vec![Meta::Int(1), Meta::String("two".to_string())]);
            let obj = meta_to_py(py, &seq).unwrap();
            let list = obj.bind(py);
            assert!(list.is_instance_of::<PyList>());
            let list = list.cast::<PyList>().unwrap();
            assert_eq!(list.len(), 2);
            assert_eq!(list.get_item(0).unwrap().extract::<i64>().unwrap(), 1);
            assert_eq!(
                list.get_item(1).unwrap().extract::<String>().unwrap(),
                "two"
            );
        });
    }

    #[test]
    fn meta_to_py_mapping_becomes_nested_dict() {
        Python::attach(|py| {
            let mut inner = StringMap::default();
            inner.insert("name", Meta::String("Ada".to_string()));
            let mut outer = StringMap::default();
            outer.insert("author", Meta::Mapping(inner));
            outer.insert("draft", Meta::Bool(false));

            let obj = meta_to_py(py, &Meta::Mapping(outer)).unwrap();
            let dict = obj.bind(py);
            assert!(dict.is_instance_of::<PyDict>());
            let dict = dict.cast::<PyDict>().unwrap();

            let author = dict.get_item("author").unwrap().unwrap();
            let author = author.cast::<PyDict>().unwrap();
            assert_eq!(
                author
                    .get_item("name")
                    .unwrap()
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "Ada"
            );

            assert!(
                !dict
                    .get_item("draft")
                    .unwrap()
                    .unwrap()
                    .extract::<bool>()
                    .unwrap()
            );
        });
    }
}
