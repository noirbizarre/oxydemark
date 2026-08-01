"""Lazy images plugin: add ``loading="lazy"`` to rendered ``<img>`` tags.

Demonstrates the ``postprocess`` hook.  This is deliberately *not* an AST
transform: the change is a pure presentational attribute on the final markup,
and doing it on the HTML string also covers images that reached the output
through other plugins (shortcodes, raw HTML nodes, ...).
"""

from __future__ import annotations

import re

__all__ = ["LazyImagesPlugin"]

#: Matches an ``<img`` opening tag and captures its attribute text.
_IMG_RE = re.compile(r"<img(?P<attrs>\s[^>]*?)?(?P<close>/?>)", re.IGNORECASE)

#: Detects an already present ``loading`` attribute.
_LOADING_RE = re.compile(r"\bloading\s*=", re.IGNORECASE)


class LazyImagesPlugin:
    """Add lazy-loading hints to every ``<img>`` that lacks them.

    Parameters
    ----------
    decoding:
        Value for the ``decoding`` attribute, or ``None`` to skip it.
        Defaults to ``"async"``.
    """

    def __init__(self, decoding: str | None = "async") -> None:
        self.decoding: str | None = decoding

    def postprocess(self, html: str) -> str:
        """Rewrite ``<img>`` tags, leaving already-annotated ones untouched."""
        return _IMG_RE.sub(self._rewrite, html)

    def _rewrite(self, match: re.Match[str]) -> str:
        attrs = match["attrs"] or ""
        if _LOADING_RE.search(attrs):
            return match[0]
        added = ' loading="lazy"'
        if self.decoding is not None and "decoding=" not in attrs.lower():
            added += f' decoding="{self.decoding}"'
        return f"<img{attrs.rstrip()}{added}{match['close']}"
