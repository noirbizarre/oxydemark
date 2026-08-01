# OxydeMark

Extensible Markdown pipelines powered by Rust.

OxydeMark is a Markdown processing engine built around an AST pipeline
architecture. It combines a high-performance **Rust core** for parsing and
rendering with **Python bindings** (via PyO3) for a flexible plugin system.

```text
Markdown Input
    -> Preprocessing Plugins (Python, text-level)
    -> Rust Parser / rushdown (AST generation)
    -> AST exposed to Python (AstNode tree)
    -> AST Transformation Plugins (Python, AST-level)
    -> Rust Renderer (HTML generation)
    -> Postprocessing Plugins (Python, HTML-level)
    -> Final Output
```

## Installation

!!! note

    OxydeMark is in early development (pre-alpha). Installation from source is
    currently the only option.

```sh
git clone https://github.com/noirbizarre/oxydemark.git
cd oxydemark
mise install
maturin develop
```

## Quick start

```python
from oxydemark import OxydeEngine

engine = OxydeEngine()

md = """
# Hello OxydeMark

This is **extensible Markdown**.
"""

html = engine.render(md)
print(html)
```

For a one-shot conversion without the plugin pipeline, use
[`markdown_to_html`][oxydemark.markdown_to_html]; to inspect or transform the
tree, use [`parse`][oxydemark.parse] and [`render_ast`][oxydemark.render_ast];
to extract headings, a table of contents, a summary and typed frontmatter, use
[`parse_document`][oxydemark.parse_document].

## Plugins

A plugin is any object implementing one or more of the `preprocess`,
`transform` and `postprocess` hooks. No base class, no registration:

```python
from oxydemark import OxydeEngine
from oxydemark.contrib import AdmonitionPlugin, MentionPlugin


class Shouty:
    def postprocess(self, html: str) -> str:
        return html.upper()


engine = OxydeEngine(plugins=[AdmonitionPlugin(), MentionPlugin(), Shouty()])
print(engine.render("> [!NOTE]\n> Ping @alice\n"))
```

`oxydemark.contrib` ships four worked examples (admonitions, shortcodes,
mentions, lazy images). See the [plugin guide](plugins.md) for the full
authoring documentation, including the AST value-semantics rules.

## Where to go next

- [Plugin guide](plugins.md) -- write your own pipeline hooks.
- [API reference](api/index.md) -- the frozen Python and Rust surfaces.
- [Design decisions](specs/README.md) -- the OMEP series.
