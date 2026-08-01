"""Shared traversal helpers for the contrib plugins.

Private module: not part of any public surface.

Two subtleties are handled here once, so the individual plugins stay readable:

1. **Value semantics.** `AstNode.children` is a PyO3 getter returning a
   *copy*. Mutations must be applied to a local list which is then reassigned
   to the node.
2. **Text-node fragmentation.** The parser is free to emit several consecutive
   `text` nodes for what is a single run of characters in the source (for
   example `"{{ youtube abc"` followed by `" }}"`). A plugin matching a pattern
   that spans such a boundary must therefore coalesce adjacent `text` siblings
   before matching.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

from oxydemark._core import AstNode

if TYPE_CHECKING:
    from collections.abc import Callable, Collection

__all__ = ["rewrite_text_nodes"]


def rewrite_text_nodes(
    node: AstNode,
    *,
    opaque_kinds: Collection[str],
    split: Callable[[str], list[AstNode]],
) -> None:
    """Recursively replace `text` nodes using a split callable.

    Args:
        node: Subtree root. Modified in place.
        opaque_kinds: Node kinds whose subtree must be left completely
            untouched.
        split: Callable turning a run of text into the nodes replacing it. It
            must return `[AstNode(kind="text", text=...)]` when nothing
            matched.
    """
    if node.kind in opaque_kinds:
        return

    rebuilt: list[AstNode] = []
    for child in _coalesce_text(node.children):
        if child.kind == "text" and child.text:
            rebuilt.extend(split(child.text))
        else:
            rewrite_text_nodes(child, opaque_kinds=opaque_kinds, split=split)
            rebuilt.append(child)
    # Write the list back: the getter above handed us a copy.
    node.children = rebuilt


def _coalesce_text(children: list[AstNode]) -> list[AstNode]:
    """Merge runs of adjacent `text` siblings into a single node."""
    merged: list[AstNode] = []
    buffer: list[str] = []

    def flush() -> None:
        if buffer:
            merged.append(AstNode(kind="text", text="".join(buffer)))
            buffer.clear()

    for child in children:
        if child.kind == "text" and child.text is not None and not child.attributes:
            buffer.append(child.text)
        else:
            flush()
            merged.append(child)
    flush()
    return merged
