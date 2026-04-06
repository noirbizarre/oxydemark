"""High-level Python API for OxydeMark."""

from __future__ import annotations

from typing import TYPE_CHECKING, Protocol

if TYPE_CHECKING:
    from oxydemark._core import AstNode


class Plugin(Protocol):
    """Protocol that all OxydeMark plugins must satisfy.

    Plugins may implement any combination of the three hooks:

    - ``preprocess``: transform raw Markdown text before parsing.
    - ``transform``: modify the AST between parsing and rendering.
    - ``postprocess``: transform rendered HTML after rendering.

    All hooks are optional; implement only the ones you need.
    """

    def preprocess(self, markdown: str) -> str:
        """Transform raw Markdown *before* it reaches the Rust parser."""
        return markdown

    def transform(self, ast: AstNode) -> AstNode:
        """Transform the AST *between* parsing and rendering.

        Receives the full AST tree and must return the (possibly modified)
        tree.  Use recursive traversal with explicit reassignment to modify
        the tree in place (PyO3 value semantics require reassigning
        ``children`` after modification):

        .. code-block:: python

            def transform(self, ast):
                self._modify(ast)
                return ast

            def _modify(self, node):
                if node.kind == "text" and node.text:
                    node.text = node.text.replace("@", "<span>@</span>")
                children = node.children
                for child in children:
                    self._modify(child)
                node.children = children
        """
        return ast

    def postprocess(self, html: str) -> str:
        """Transform rendered HTML *after* it leaves the Rust renderer."""
        return html


class OxydeEngine:
    """Pipeline-oriented Markdown engine with plugin support.

    The full pipeline is::

        Markdown Input
            -> preprocess plugins (text-level)
            -> Rust parser / rushdown (AST generation)
            -> transform plugins (AST-level)
            -> Rust renderer (HTML generation)
            -> postprocess plugins (HTML-level)
            -> Final Output

    Parameters
    ----------
    plugins:
        An ordered sequence of plugin instances.  Each plugin may
        implement ``preprocess``, ``transform``, and/or ``postprocess``
        hooks.

    Example
    -------
    >>> engine = OxydeEngine()
    >>> html = engine.render("# Hello")
    """

    def __init__(self, plugins: list[Plugin] | None = None) -> None:
        self.plugins: list[Plugin] = plugins or []

    def render(self, markdown: str) -> str:
        """Run the full pipeline: preprocess -> parse -> transform -> render -> postprocess."""
        from oxydemark._core import parse as _parse
        from oxydemark._core import render_ast as _render_ast

        # 1. Preprocessing: text-level plugins.
        text = markdown
        for plugin in self.plugins:
            if hasattr(plugin, "preprocess"):
                text = plugin.preprocess(text)

        # 2. Parse Markdown to AST (Rust / rushdown).
        ast = _parse(text)

        # 3. AST transformation: AST-level plugins.
        for plugin in self.plugins:
            if hasattr(plugin, "transform"):
                ast = plugin.transform(ast)

        # 4. Render AST to HTML (Rust renderer).
        html = _render_ast(ast)

        # 5. Postprocessing: HTML-level plugins.
        for plugin in self.plugins:
            if hasattr(plugin, "postprocess"):
                html = plugin.postprocess(html)

        return html
