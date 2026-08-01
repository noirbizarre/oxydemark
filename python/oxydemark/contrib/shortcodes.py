"""Shortcode plugin: expand ``{{ name argument }}`` into raw HTML.

Demonstrates replacing a ``text`` node with a mix of ``text`` and ``raw_html``
nodes during the ``transform`` hook.

Input::

    Watch {{ youtube dQw4w9WgXcQ }} now.

Output::

    <p>Watch <div class="embed">...<iframe ...></iframe></div> now.</p>

Why ``transform`` and not ``preprocess``?
-----------------------------------------
HTML emitted by a ``preprocess`` hook is *stripped* by the parser (raw HTML in
the source is not trusted), and HTML assigned to ``AstNode.text`` is *escaped*
by the renderer.  The only way to inject markup is a node of kind
``raw_html``, whose ``text`` is emitted verbatim.  This makes the shortcode
expansion an AST-level concern.
"""

from __future__ import annotations

import re
from typing import TYPE_CHECKING

from oxydemark._core import AstNode
from oxydemark.contrib._text import rewrite_text_nodes

if TYPE_CHECKING:
    from collections.abc import Callable, Mapping

__all__ = ["Shortcode", "ShortcodePlugin"]

#: A shortcode handler: takes the raw argument, returns an HTML fragment.
type Shortcode = Callable[[str], str]

#: Matches ``{{ name argument }}``.
_SHORTCODE_RE = re.compile(r"\{\{\s*(?P<name>[a-z][a-z0-9_-]*)\s+(?P<arg>[^}]*?)\s*\}\}")

#: Conservative YouTube video id.
_YOUTUBE_ID_RE = re.compile(r"^[A-Za-z0-9_-]{1,32}$")

#: Node kinds whose subtree must be left verbatim.
_OPAQUE_KINDS = frozenset({"code_block", "code_span", "html_block", "raw_html"})


def youtube(video_id: str) -> str:
    """Render a YouTube embed, or an empty string for an invalid id."""
    if not _YOUTUBE_ID_RE.match(video_id):
        return ""
    return (
        '<div class="embed embed-youtube">'
        f'<iframe src="https://www.youtube.com/embed/{video_id}"'
        ' title="YouTube video player" frameborder="0" allowfullscreen></iframe>'
        "</div>"
    )


#: Handlers enabled when no explicit mapping is given.
DEFAULT_SHORTCODES: Mapping[str, Shortcode] = {"youtube": youtube}


class ShortcodePlugin:
    """Expand ``{{ name argument }}`` markers into raw HTML nodes.

    Parameters
    ----------
    shortcodes:
        Mapping of shortcode name to a handler returning an HTML fragment.
        Defaults to :data:`DEFAULT_SHORTCODES`.  Unknown names, and handlers
        returning an empty string, leave the marker untouched.
    """

    def __init__(self, shortcodes: Mapping[str, Shortcode] | None = None) -> None:
        self.shortcodes: dict[str, Shortcode] = (
            dict(shortcodes) if shortcodes is not None else dict(DEFAULT_SHORTCODES)
        )

    def transform(self, ast: AstNode) -> AstNode:
        """Replace shortcode markers found in ``text`` nodes."""
        rewrite_text_nodes(ast, opaque_kinds=_OPAQUE_KINDS, split=self._split)
        return ast

    def _split(self, text: str) -> list[AstNode]:
        """Split ``text`` into ``text``/``raw_html`` nodes around shortcodes."""
        nodes: list[AstNode] = []
        cursor = 0
        for match in _SHORTCODE_RE.finditer(text):
            handler = self.shortcodes.get(match["name"])
            html = handler(match["arg"]) if handler is not None else ""
            if not html:
                continue  # Unknown or rejected: leave the marker in the text.
            if before := text[cursor : match.start()]:
                nodes.append(AstNode(kind="text", text=before))
            nodes.append(AstNode(kind="raw_html", text=html))
            cursor = match.end()
        if not nodes:
            return [AstNode(kind="text", text=text)]
        if remainder := text[cursor:]:
            nodes.append(AstNode(kind="text", text=remainder))
        return nodes
