"""High-level Python API for OxydeMark."""

from __future__ import annotations

from typing import TYPE_CHECKING, Protocol, TypeAlias

from oxydemark._core import parse as _parse
from oxydemark._core import render_ast as _render_ast

if TYPE_CHECKING:
    from collections.abc import Callable

    from oxydemark._core import AstNode


class Preprocessor(Protocol):
    """Plugin protocol for the text-level `preprocess` hook."""

    def preprocess(self, markdown: str) -> str:
        """Transform raw Markdown *before* it reaches the Rust parser.

        Args:
            markdown: The Markdown source as produced by the previous plugin.

        Returns:
            The rewritten Markdown source.
        """
        ...


class Transformer(Protocol):
    """Plugin protocol for the AST-level `transform` hook."""

    def transform(self, ast: AstNode) -> AstNode:
        """Transform the AST *between* parsing and rendering.

        !!! important "`AstNode` has value semantics"

            `node.children` is a PyO3 getter that returns a *fresh copy* of the
            child list, and each element in it is a copy too. Consequently:

            ```python
            node.children[0].text = "x"   # silently discarded
            node.children.append(other)   # silently discarded
            ```

            Mutations must be applied to a local copy which is then reassigned:

            ```python
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
            ```

            The same applies to [`AstNode.walk`][oxydemark.AstNode.walk], which
            yields copies: it is useful for *inspection* only, never for
            mutation.

        See [`oxydemark.contrib`][] for complete, tested examples.

        Args:
            ast: The root of the tree as produced by the previous plugin.

        Returns:
            The (possibly modified) tree.
        """
        ...


class Postprocessor(Protocol):
    """Plugin protocol for the HTML-level `postprocess` hook."""

    def postprocess(self, html: str) -> str:
        """Transform rendered HTML *after* it leaves the Rust renderer.

        Args:
            html: The HTML as produced by the previous plugin.

        Returns:
            The rewritten HTML.
        """
        ...


Plugin: TypeAlias = Preprocessor | Transformer | Postprocessor
"""Any object satisfying at least one of the three plugin hooks.

Plugins may implement any combination of the hooks:

- `preprocess` ([`Preprocessor`][oxydemark.api.Preprocessor]): transform raw
  Markdown text before parsing.
- `transform` ([`Transformer`][oxydemark.api.Transformer]): modify the AST
  between parsing and rendering.
- `postprocess` ([`Postprocessor`][oxydemark.api.Postprocessor]): transform
  rendered HTML after rendering.

All hooks are optional; implement only the ones you need. `Plugin` is therefore
a *union* of the three single-hook protocols rather than a protocol requiring
all three -- most real plugins, including every one in
[`oxydemark.contrib`][], implement exactly one.

The protocols are **structural** and hooks are dispatched by `hasattr`, never
by `isinstance`. A plugin does not need to inherit from or register with
anything -- any object exposing at least one correctly named hook is a valid
plugin.

See [`oxydemark.contrib`][] for worked examples and the
[plugin authoring guide](../plugins.md) for the full picture.

## Hook ordering

[`OxydeEngine`][oxydemark.OxydeEngine] runs *one phase at a time* across the
whole plugin list, not one plugin at a time. For plugins `[A, B]` the call
order is `A.preprocess`, `B.preprocess`, parse, `A.transform`, `B.transform`,
render, `A.postprocess`, `B.postprocess`.

## Choosing the right hook

| Hook | Operates on | Use it when |
| --- | --- | --- |
| `preprocess` | raw Markdown `str` | the construct is not representable in the AST yet (custom markers, macros, includes, front-matter tweaks). |
| `transform` | [`AstNode`][oxydemark.AstNode] tree | you need structure: adding, removing, re-typing or annotating nodes. |
| `postprocess` | rendered HTML `str` | the change is purely presentational and applies to the final markup. |

## Escaping rules worth knowing

- [`AstNode.text`][oxydemark.AstNode] is **HTML-escaped** by the renderer.
- Raw HTML present in the *source* Markdown is **stripped**: the parser
  replaces it with a `<!-- raw HTML omitted -->` placeholder, which is what
  the resulting `"raw_html"` / `"html_block"` nodes carry as their `text`.
- A node of kind `"raw_html"` *created by a plugin* emits its `text`
  **verbatim**; it is the only supported way to inject markup from a
  `transform` hook.
"""


class OxydeEngine:
    """Pipeline-oriented Markdown engine with plugin support.

    The full pipeline is:

    ```text
    Markdown Input
        -> preprocess plugins (text-level)
        -> Rust parser / rushdown (AST generation)
        -> transform plugins (AST-level)
        -> Rust renderer (HTML generation)
        -> postprocess plugins (HTML-level)
        -> Final Output
    ```

    Attributes:
        plugins: The ordered plugin list the engine runs.

    Example:
        >>> engine = OxydeEngine()
        >>> html = engine.render("# Hello")
    """

    def __init__(self, plugins: list[Plugin] | None = None) -> None:
        """Build an engine.

        Args:
            plugins: An ordered sequence of plugin instances. Each plugin may
                implement `preprocess`, `transform`, and/or `postprocess`
                hooks.
        """
        self.plugins: list[Plugin] = plugins or []

    def render(self, markdown: str) -> str:
        """Run the full pipeline: preprocess, parse, transform, render, postprocess.

        Args:
            markdown: The Markdown source to render.

        Returns:
            The final HTML, after every plugin hook has run.
        """
        # Hooks are looked up by name rather than by `isinstance`, so a plugin
        # only implements what it needs. `Plugin` is a union of single-hook
        # protocols, so the lookups are written as `getattr` -- narrowing a
        # union member that does not declare the attribute would otherwise
        # leave it statically untyped.

        # 1. Preprocessing: text-level plugins.
        text = markdown
        for plugin in self.plugins:
            preprocess: Callable[[str], str] | None = getattr(
                plugin, "preprocess", None
            )
            if preprocess is not None:
                text = preprocess(text)

        # 2. Parse Markdown to AST (Rust / rushdown).
        ast = _parse(text)

        # 3. AST transformation: AST-level plugins.
        for plugin in self.plugins:
            transform: Callable[[AstNode], AstNode] | None = getattr(
                plugin, "transform", None
            )
            if transform is not None:
                ast = transform(ast)

        # 4. Render AST to HTML (Rust renderer).
        html = _render_ast(ast)

        # 5. Postprocessing: HTML-level plugins.
        for plugin in self.plugins:
            postprocess: Callable[[str], str] | None = getattr(
                plugin, "postprocess", None
            )
            if postprocess is not None:
                html = postprocess(html)

        return html
