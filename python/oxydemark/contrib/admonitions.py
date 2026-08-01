"""Admonition plugin: GitHub-style alerts rendered as ``<div class="admonition">``.

Demonstrates the ``preprocess`` and ``transform`` hooks working together.

Input::

    > [!NOTE]
    > Useful information.

Output::

    <div class="admonition admonition-note">
    <div class="admonition-title">Note</div>
    <p>Useful information.</p>
    </div>

Why two hooks?
--------------
The ``[!NOTE]`` marker cannot be reliably detected in the AST: the parser
splits it into three separate ``text`` nodes (``"["``, ``"!NOTE"``, ``"]"``)
because it looks like a link reference.  Detection therefore happens at the
**text** layer (``preprocess``), which rewrites the blockquote into a Comark
``:::note`` block component.  Styling the resulting node -- adding classes and
a title -- is a structural concern and happens at the **AST** layer
(``transform``).
"""

from __future__ import annotations

import re
from typing import TYPE_CHECKING

from oxydemark._core import AstNode

if TYPE_CHECKING:
    from collections.abc import Mapping

__all__ = ["AdmonitionPlugin"]

#: Default markers recognised by :class:`AdmonitionPlugin`, mapped to their title.
DEFAULT_KINDS: Mapping[str, str] = {
    "note": "Note",
    "tip": "Tip",
    "important": "Important",
    "warning": "Warning",
    "caution": "Caution",
}

#: Matches an alert opener such as ``> [!NOTE]`` (optionally indented).
_ALERT_RE = re.compile(r"^ {0,3}> ?\[!(?P<kind>[A-Za-z]+)\] *$")

#: Matches a blockquote continuation line, capturing the content after ``> ``.
_QUOTE_RE = re.compile(r"^ {0,3}> ?(?P<content>.*)$")


class AdmonitionPlugin:
    """Turn GitHub-style alerts into styled admonition blocks.

    Parameters
    ----------
    kinds:
        Mapping of lowercase marker name to the title rendered in the
        admonition header.  Defaults to :data:`DEFAULT_KINDS`.  Markers absent
        from this mapping are left untouched as ordinary blockquotes.
    """

    def __init__(self, kinds: Mapping[str, str] | None = None) -> None:
        self.kinds: dict[str, str] = dict(kinds) if kinds is not None else dict(DEFAULT_KINDS)

    def preprocess(self, markdown: str) -> str:
        """Rewrite ``> [!KIND]`` blockquotes into Comark ``:::kind`` fences."""
        lines = markdown.split("\n")
        out: list[str] = []
        index = 0
        while index < len(lines):
            match = _ALERT_RE.match(lines[index])
            kind = match["kind"].lower() if match else None
            if kind is None or kind not in self.kinds:
                out.append(lines[index])
                index += 1
                continue

            # Consume the remaining blockquote lines as the admonition body.
            index += 1
            body: list[str] = []
            while index < len(lines) and (quoted := _QUOTE_RE.match(lines[index])):
                body.append(quoted["content"])
                index += 1

            out.append(f":::{kind}")
            out.extend(body)
            out.append(":::")
        return "\n".join(out)

    def transform(self, ast: AstNode) -> AstNode:
        """Add admonition classes and a title node to admonition components."""
        self._decorate(ast)
        return ast

    def _decorate(self, node: AstNode) -> None:
        # ``AstNode.children`` returns a *copy* (PyO3 value semantics), so the
        # list must be mutated locally and reassigned. See docs/plugins.md.
        children = node.children
        for child in children:
            self._decorate(child)

        if node.kind == "block_component":
            kind = node.attributes.get("name", "")
            title = self.kinds.get(kind)
            if title is not None:
                attributes = node.attributes
                attributes["class"] = f"admonition admonition-{kind}"
                node.attributes = attributes
                children.insert(0, _title_node(title))

        node.children = children


def _title_node(title: str) -> AstNode:
    """Build the ``<div class="admonition-title">`` header node."""
    return AstNode(
        kind="block_component",
        children=[AstNode(kind="text", text=title)],
        attributes={"class": "admonition-title"},
    )
