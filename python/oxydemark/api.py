"""High-level Python API for OxydeMark."""

from __future__ import annotations

from typing import Protocol


class Plugin(Protocol):
    """Protocol that all OxydeMark plugins must satisfy."""

    def preprocess(self, markdown: str) -> str:
        """Transform raw Markdown *before* it reaches the Rust parser."""
        ...

    def postprocess(self, html: str) -> str:
        """Transform rendered HTML *after* it leaves the Rust renderer."""
        ...


class MarkdownEngine:
    """Pipeline-oriented Markdown engine with plugin support.

    Parameters
    ----------
    plugins:
        An ordered sequence of plugin instances.  Each plugin may
        implement ``preprocess`` and/or ``postprocess`` hooks.

    Example
    -------
    >>> engine = MarkdownEngine()
    >>> engine.render("# Hello")
    '<p># Hello</p>'
    """

    def __init__(self, plugins: list[Plugin] | None = None) -> None:
        self.plugins: list[Plugin] = plugins or []

    def render(self, markdown: str) -> str:
        """Run the full pipeline: preprocess -> Rust render -> postprocess."""
        from oxydemark._core import render as _render

        text = markdown
        for plugin in self.plugins:
            if hasattr(plugin, "preprocess"):
                text = plugin.preprocess(text)

        html = _render(text)

        for plugin in self.plugins:
            if hasattr(plugin, "postprocess"):
                html = plugin.postprocess(html)

        return html
