"""Example plugins shipped with OxydeMark.

This namespace exists to demonstrate the :class:`oxydemark.Plugin` protocol
against real, tested implementations.  Each plugin is deliberately small and
focuses on one pipeline layer:

===========================  ==========================  ==================================
Plugin                       Hooks                       Demonstrates
===========================  ==========================  ==================================
:class:`AdmonitionPlugin`    ``preprocess`` + ``transform``  text normalisation, then AST enrichment
:class:`ShortcodePlugin`     ``transform``               replacing text nodes with ``raw_html``
:class:`MentionPlugin`       ``transform``               splitting text into structural nodes
:class:`LazyImagesPlugin`    ``postprocess``             pure HTML-string rewriting
===========================  ==========================  ==================================

Stability
---------
``oxydemark.contrib`` is a **provisional** surface (see OMEP-0008).  It is
public and documented, but it is intentionally *not* part of
``oxydemark.__all__`` and carries no stability guarantee: these plugins may
change or be removed in a MINOR release.  If you need long-term stability,
copy the plugin into your own codebase.

See ``docs/plugins.md`` for the full plugin authoring guide.
"""

from __future__ import annotations

from oxydemark.contrib.admonitions import AdmonitionPlugin
from oxydemark.contrib.images import LazyImagesPlugin
from oxydemark.contrib.mentions import MentionPlugin
from oxydemark.contrib.shortcodes import Shortcode, ShortcodePlugin

__all__ = [
    "AdmonitionPlugin",
    "LazyImagesPlugin",
    "MentionPlugin",
    "Shortcode",
    "ShortcodePlugin",
]
