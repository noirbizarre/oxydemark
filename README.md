# OxydeMark

[![CI](https://github.com/noirbizarre/oxydemark/actions/workflows/ci.yml/badge.svg)](https://github.com/noirbizarre/oxydemark/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/noirbizarre/oxydemark/graph/badge.svg)](https://codecov.io/gh/noirbizarre/oxydemark)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![prek](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/j178/prek/master/docs/assets/badge-v0.json)](https://github.com/j178/prek)

Extensible Markdown pipelines powered by Rust.

**[Documentation](https://noirbizarre.github.io/oxydemark/)** |
**[API reference](https://noirbizarre.github.io/oxydemark/api/)**

## Overview

OxydeMark is a Markdown processing engine built around an AST pipeline
architecture. It combines a high-performance **Rust core** for parsing and
rendering with **Python bindings** (via PyO3) for a flexible plugin system.

```
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

> **Note:** OxydeMark is in early development (pre-alpha). Installation from
> source is currently the only option.

```sh
# Clone the repository
git clone https://github.com/noirbizarre/oxydemark.git
cd oxydemark

# Install tools and build
mise install
maturin develop
```

## Quick Start

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
mentions, lazy images). See the [plugin guide](docs/plugins.md) for the full
authoring documentation, including the AST value-semantics rules.

## Development

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full development guide.

```sh
# Set up tools and hooks
mise install
mise run setup

# Common tasks
mise run build        # Build the Rust crate
mise run test         # Run tests
mise run lint         # Run clippy
mise run fmt          # Format code
mise run ci           # Run all checks
mise run docs         # Build the documentation site
```

## Project Structure

```
oxydemark/
├── src/                    # Rust core
│   ├── lib.rs              # PyO3 module, parser/renderer wiring
│   ├── ast.rs              # AstNode definition, arena-to-tree conversion
│   ├── extensions.rs       # Comark parser/renderer extensions
│   └── html_render.rs      # AST-to-HTML renderer
├── python/oxydemark/       # Python package
│   ├── __init__.py         # Re-exports from native module
│   ├── api.py              # OxydeEngine, plugin protocol
│   └── contrib/            # Example plugins (provisional surface)
├── docs/                   # Documentation site sources
│   ├── plugins.md          # Plugin authoring guide
│   ├── api/                # API reference pages
│   └── specs/              # OMEPs (design decisions)
├── .github/workflows/      # CI pipeline
├── Cargo.toml              # Rust crate configuration
├── pyproject.toml          # Python build config (maturin)
├── mise.toml               # Task runner and tool versions
├── zensical.toml           # Documentation site configuration
├── cliff.toml              # Changelog generation
├── prek.toml               # Pre-commit hooks
├── CONTRIBUTING.md         # Contribution guidelines
└── LICENSE                 # MIT License
```

## Design Decisions

Architectural and tooling decisions are documented as **OMEPs** (OxydeMark
Enhancement Proposals) in [`docs/specs/`](docs/specs/). See the
[OMEP index](docs/specs/README.md) for the full list.

## License

[MIT](LICENSE)
