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

    The protocol is **structural** and hooks are dispatched by
    :func:`hasattr`, never by :func:`isinstance`.  A plugin therefore does not
    need to inherit from or register with anything -- any object exposing at
    least one correctly named hook is a valid plugin.

    Hook ordering
    -------------
    :class:`OxydeEngine` runs *one phase at a time* across the whole plugin
    list, not one plugin at a time.  For plugins ``[A, B]`` the call order is
    ``A.preprocess``, ``B.preprocess``, parse, ``A.transform``,
    ``B.transform``, render, ``A.postprocess``, ``B.postprocess``.

    Choosing the right hook
    -----------------------
    ==============  =====================  =====================================
    Hook            Operates on            Use it when
    ==============  =====================  =====================================
    ``preprocess``  raw Markdown ``str``   the construct is not representable in
                                           the AST yet (custom markers, macros,
                                           includes, front-matter tweaks).
    ``transform``   :class:`AstNode` tree  you need structure: adding, removing,
                                           re-typing or annotating nodes.
    ``postprocess`` rendered HTML ``str``  the change is purely presentational
                                           and applies to the final markup.
    ==============  =====================  =====================================

    Escaping rules worth knowing:

    - ``AstNode.text`` is **HTML-escaped** by the renderer.
    - Raw HTML present in the *source* Markdown is **stripped**.
    - A node of kind ``"raw_html"`` emits its ``text`` **verbatim**; it is the
      only supported way to inject markup from a ``transform`` hook.

    See :mod:`oxydemark.contrib` for worked examples and ``docs/plugins.md``
    for the full authoring guide.
    """

    def preprocess(self, markdown: str) -> str:
        """Transform raw Markdown *before* it reaches the Rust parser."""
        return markdown

    def transform(self, ast: AstNode) -> AstNode:
        """Transform the AST *between* parsing and rendering.

        Receives the full AST tree and must return the (possibly modified)
        tree.

        .. important::
           :class:`AstNode` has **value semantics**.  ``node.children`` is a
           PyO3 getter that returns a *fresh copy* of the child list, and each
           element in it is a copy too.  Consequently::

               node.children[0].text = "x"   # silently discarded
               node.children.append(other)   # silently discarded

           Mutations must be applied to a local copy which is then reassigned::

               def transform(self, ast):
                   self._modify(ast)
                   return ast

               def _modify(self, node):
                   if node.kind == "text" and node.text:
                       node.text = node.text.upper()
                   children = node.children      # copy out
                   for child in children:
                       self._modify(child)
                   node.children = children      # write back -- mandatory

           The same applies to :meth:`AstNode.walk`, which yields copies:
           it is useful for *inspection* only, never for mutation.

        See :mod:`oxydemark.contrib` for complete, tested examples.
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
