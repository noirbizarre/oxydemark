"""Type stubs for the native ``oxydemark._core`` extension module.

This stub is the source of truth for static type checkers, since the compiled
Rust module cannot be introspected from source. It must be kept in sync with
the ``#[pymethods]`` in ``src/ast.rs`` and the ``#[pyfunction]`` definitions in
``src/lib.rs`` (see OMEP-0008).
"""

from __future__ import annotations

class AstNode:
    """A tree-based AST node representing a Markdown element."""

    kind: str
    children: list[AstNode]
    text: str | None
    attributes: dict[str, str]
    #: Deprecated (OMEP-0010): stringly-typed frontmatter, retained for backward
    #: compatibility. Use ``parse_document(...).frontmatter`` for typed access.
    metadata: dict[str, str] | None
    #: Typed block-component props from a leading YAML block (OMEP-0007), or
    #: ``None``. Present only on ``block_component`` nodes that declare a YAML
    #: block. Values preserve their native YAML types (``str``, ``int``,
    #: ``float``, ``bool``, ``list``, ``dict``, ``None``). Inline ``{…}``
    #: attributes take precedence on key collisions. Read-only.
    @property
    def props(self) -> dict[str, object] | None: ...

    def __init__(
        self,
        kind: str,
        children: list[AstNode] | None = None,
        text: str | None = None,
        attributes: dict[str, str] | None = None,
        metadata: dict[str, str] | None = None,
    ) -> None: ...
    def walk(self) -> list[AstNode]:
        """Walk the AST tree depth-first, returning a flat list of all nodes."""
        ...

    def __repr__(self) -> str: ...

class ParseResult:
    """The AST tree plus structured, typed document metadata (OMEP-0010)."""

    #: The parsed AST tree (identical to what ``parse`` returns).
    root: AstNode
    #: Typed YAML frontmatter, or ``None`` when the document has no frontmatter.
    #: Values preserve their native YAML types (``str``, ``int``, ``float``,
    #: ``bool``, ``list``, ``dict``, ``None``).
    frontmatter: dict[str, object] | None

    def __repr__(self) -> str: ...

def parse(markdown: str) -> AstNode:
    """Parse Markdown input into an AST node tree."""
    ...

def parse_document(markdown: str) -> ParseResult:
    """Parse Markdown and compute structured, typed metadata (OMEP-0010).

    Returns a ``ParseResult`` bundling the AST tree with typed YAML frontmatter.
    """
    ...

def render_ast(node: AstNode) -> str:
    """Render an ``AstNode`` tree to an HTML string."""
    ...

def markdown_to_html(markdown: str) -> str:
    """Convert Markdown directly to HTML (fast path, no AST exposure)."""
    ...

def slugify(text: str, existing: list[str] | None = None) -> str:
    """Generate a URL-friendly anchor slug from ``text`` (OMEP-0010)."""
    ...

def extract_summary(markdown: str) -> str | None:
    """Extract the rendered-HTML summary before a top-level ``<!-- more -->`` delimiter.

    Returns ``None`` when the document has no top-level delimiter.
    """
    ...
