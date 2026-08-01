"""Type stubs for the native `oxydemark._core` extension module.

This stub is the source of truth for static type checkers *and* for the
generated API reference, since the compiled Rust module cannot be introspected
from source. It must be kept in sync with the `#[pymethods]` in `src/ast.rs`
and the `#[pyfunction]` definitions in `src/python.rs` (see OMEP-0008).
"""

from __future__ import annotations

class AstNode:
    """A tree-based AST node representing a Markdown element.

    Attributes:
        kind: The node type, such as `"paragraph"`, `"text"` or `"heading"`.
        children: Direct child nodes. Reading this attribute returns a *copy*;
            mutations must be written back to take effect.
        text: Literal text content, for leaf nodes that carry any.
        attributes: Rendered HTML attributes, as strings.
        metadata: Deprecated (OMEP-0010). Stringly-typed frontmatter, retained
            for backward compatibility. Use
            [`parse_document`][oxydemark.parse_document] and its
            [`frontmatter`][oxydemark.ParseResult.frontmatter] property for
            typed access instead.
    """

    kind: str
    children: list[AstNode]
    text: str | None
    attributes: dict[str, str]
    metadata: dict[str, str] | None

    @property
    def props(self) -> dict[str, object] | None:
        """Typed block-component props from a leading YAML block (OMEP-0007).

        Present only on `block_component` nodes that declare a YAML block, and
        `None` otherwise. Values preserve their native YAML types (`str`,
        `int`, `float`, `bool`, `list`, `dict`, `None`). Inline `{…}`
        attributes take precedence on key collisions. Read-only.
        """

    def __init__(
        self,
        kind: str,
        children: list[AstNode] | None = None,
        text: str | None = None,
        attributes: dict[str, str] | None = None,
        metadata: dict[str, str] | None = None,
    ) -> None:
        """Build a node.

        Args:
            kind: The node type.
            children: Direct child nodes.
            text: Literal text content.
            attributes: Rendered HTML attributes.
            metadata: Deprecated stringly-typed frontmatter (OMEP-0010).
        """

    def walk(self) -> list[AstNode]:
        """Walk the tree depth-first, returning a flat list of all nodes.

        The returned nodes are *copies*, so this is useful for inspection only,
        never for mutation.

        Returns:
            Every node of the subtree, in depth-first document order.
        """

    def __repr__(self) -> str: ...

class Heading:
    """A document heading, flat entry or TOC tree node (OMEP-0010)."""

    @property
    def level(self) -> int:
        """Heading level, 1 to 6."""

    @property
    def id(self) -> str:
        """The anchor id assigned to the heading."""

    @property
    def text(self) -> str:
        """Plain-text heading label, the text the slug is derived from."""

    @property
    def children(self) -> list[Heading]:
        """Nested sub-headings.

        Always empty for entries of the flat
        [`ParseResult.headings`][oxydemark.ParseResult.headings] list;
        populated only in [`ParseResult.toc`][oxydemark.ParseResult.toc].
        """

    def __repr__(self) -> str: ...

class ParseResult:
    """The AST tree plus structured, typed document metadata (OMEP-0010)."""

    @property
    def root(self) -> AstNode:
        """The parsed AST tree, identical to what [`parse`][oxydemark.parse] returns."""

    @property
    def headings(self) -> list[Heading]:
        """Every heading of the document, in document order (flat)."""

    @property
    def toc(self) -> list[Heading]:
        """The nested table-of-contents tree."""

    @property
    def summary(self) -> str | None:
        """Rendered HTML of the content before a top-level `<!-- more -->` delimiter.

        `None` when the document has no delimiter.
        """

    @property
    def frontmatter(self) -> dict[str, object] | None:
        """Typed YAML frontmatter, or `None` when the document has none.

        Values preserve their native YAML types (`str`, `int`, `float`, `bool`,
        `list`, `dict`, `None`).
        """

    def __repr__(self) -> str: ...

def parse(markdown: str) -> AstNode:
    """Parse Markdown input into an AST node tree.

    Args:
        markdown: The Markdown source to parse.

    Returns:
        The root node of the parsed tree.
    """

def parse_document(markdown: str) -> ParseResult:
    """Parse Markdown and compute structured, typed metadata (OMEP-0010).

    Args:
        markdown: The Markdown source to parse.

    Returns:
        The AST tree bundled with the flat heading list, the nested
        table-of-contents tree, the summary HTML and typed YAML frontmatter.
    """

def render_ast(node: AstNode) -> str:
    """Render an AST node tree to an HTML string.

    Args:
        node: The root node to render.

    Returns:
        The rendered HTML.
    """

def markdown_to_html(markdown: str) -> str:
    """Convert Markdown directly to HTML, the fast path with no AST exposure.

    Args:
        markdown: The Markdown source to convert.

    Returns:
        The rendered HTML.
    """

def slugify(text: str, existing: list[str] | None = None) -> str:
    """Generate a URL-friendly anchor slug from a text (OMEP-0010).

    Args:
        text: The label to derive the slug from.
        existing: Already-assigned slugs, used to de-duplicate by appending a
            numeric suffix.

    Returns:
        A unique, URL-friendly slug.
    """

def extract_summary(markdown: str) -> str | None:
    """Extract the rendered-HTML summary before a top-level `<!-- more -->` delimiter.

    Args:
        markdown: The Markdown source to inspect.

    Returns:
        The rendered summary HTML, or `None` when the document has no top-level
        delimiter.
    """
