"""Mention plugin: turn ``@handle`` into a link.

Demonstrates splitting a ``text`` node into structural nodes (``text`` +
``link``) rather than injecting raw HTML -- the safer option whenever the
target markup already has an AST representation.

Input::

    Ping @alice about it.

Output::

    <p>Ping <a href="https://github.com/alice" class="mention">@alice</a> about it.</p>
"""

from __future__ import annotations

import re

from oxydemark._core import AstNode
from oxydemark.contrib._text import rewrite_text_nodes

__all__ = ["MentionPlugin"]

#: Matches ``@handle`` when not preceded by a word character (avoids emails).
_MENTION_RE = re.compile(r"(?<![\w@/-])@(?P<handle>[A-Za-z0-9][A-Za-z0-9-]{0,38})\b")

#: Node kinds whose subtree must be left verbatim (already a link, or code).
_OPAQUE_KINDS = frozenset({"code_block", "code_span", "html_block", "link", "raw_html"})


class MentionPlugin:
    """Linkify ``@handle`` mentions found in text nodes.

    Parameters
    ----------
    base_url:
        Prefix the handle is appended to. Defaults to ``https://github.com/``.
    """

    def __init__(self, base_url: str = "https://github.com/") -> None:
        self.base_url: str = base_url

    def transform(self, ast: AstNode) -> AstNode:
        """Replace mentions found in ``text`` nodes with ``link`` nodes."""
        rewrite_text_nodes(ast, opaque_kinds=_OPAQUE_KINDS, split=self._split)
        return ast

    def _split(self, text: str) -> list[AstNode]:
        """Split ``text`` into ``text``/``link`` nodes around mentions."""
        nodes: list[AstNode] = []
        cursor = 0
        for match in _MENTION_RE.finditer(text):
            if before := text[cursor : match.start()]:
                nodes.append(AstNode(kind="text", text=before))
            nodes.append(self._link(match["handle"]))
            cursor = match.end()
        if not nodes:
            return [AstNode(kind="text", text=text)]
        if remainder := text[cursor:]:
            nodes.append(AstNode(kind="text", text=remainder))
        return nodes

    def _link(self, handle: str) -> AstNode:
        """Build the ``<a class="mention">`` node for a handle."""
        return AstNode(
            kind="link",
            children=[AstNode(kind="text", text=f"@{handle}")],
            attributes={"href": f"{self.base_url}{handle}", "class": "mention"},
        )
