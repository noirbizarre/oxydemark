"""Benchmarks comparing oxydemark, markdown-it-py, and Markdown.

Run with:
    mise run bench
    # or directly:
    uv run --group bench pytest benchmarks/ --benchmark-only
"""

from __future__ import annotations

import markdown
import markdown_it
import pytest

import oxydemark
from oxydemark.api import OxydeEngine

# ---------------------------------------------------------------------------
# Sample documents
# ---------------------------------------------------------------------------

SHORT = "Hello **world**, this is *Markdown*."

MEDIUM = """\
# Introduction

This is a **medium-length** Markdown document with various elements.

## Features

- Item one with *emphasis*
- Item two with `inline code`
- Item three with [a link](https://example.com)

> A blockquote with some text that spans
> multiple lines for testing purposes.

Here is some `inline code` and a [link](https://example.com "title").

## Code Block

```python
def hello():
    print("Hello, world!")
```

---

Final paragraph with **bold**, *italic*, and ~~strikethrough~~.
"""

LONG = """\
# Project Documentation

## Overview

This document serves as a comprehensive guide to the project architecture,
implementation details, and usage patterns. It includes various Markdown
constructs to provide a realistic benchmark scenario.

## Table of Contents

- [Installation](#installation)
- [Configuration](#configuration)
- [API Reference](#api-reference)
- [Examples](#examples)
- [Troubleshooting](#troubleshooting)

## Installation

Install the package using pip:

```bash
pip install example-package
```

Or with optional dependencies:

```bash
pip install example-package[full]
```

### Requirements

| Package   | Version | Purpose          |
|-----------|---------|------------------|
| Python    | >= 3.12 | Runtime          |
| Rust      | stable  | Native extension |
| maturin   | >= 1.8  | Build backend    |

## Configuration

Configuration is done via a TOML file:

```toml
[project]
name = "example"
version = "1.0.0"

[project.features]
enable_cache = true
max_retries = 3
```

> **Note:** All configuration values can be overridden via environment
> variables using the `EXAMPLE_` prefix.

## API Reference

### `process(input: str) -> str`

Process the input string and return the result.

**Parameters:**
- `input` (*str*) -- The input string to process.

**Returns:** The processed string.

**Example:**

```python
from example import process

result = process("Hello, world!")
print(result)
```

### `configure(**kwargs) -> None`

Update the global configuration.

**Parameters:**
- `cache` (*bool*) -- Enable or disable caching.
- `timeout` (*int*) -- Request timeout in seconds.

## Examples

### Basic Usage

```python
from example import Client

client = Client()
response = client.get("/api/data")
print(response.json())
```

### Advanced Usage

For more complex scenarios, use the `Pipeline` class:

```python
from example import Pipeline, Step

pipeline = Pipeline([
    Step("validate", validator),
    Step("transform", transformer),
    Step("output", renderer),
])

result = pipeline.run(input_data)
```

## Troubleshooting

### Common Issues

1. **Import Error**: Make sure the native extension is built.
   Run `maturin develop` to build and install in development mode.

2. **Performance Issues**: Enable caching with `configure(cache=True)`.

3. **Connection Timeout**: Increase the timeout value:
   ```python
   configure(timeout=30)
   ```

### FAQ

*Q: Does it support Python 3.11?*
A: No, the minimum supported version is **Python 3.12**.

*Q: Can I use it without Rust?*
A: No, the Rust extension is required for core functionality.

---

For more information, visit the [project homepage](https://example.com)
or file an issue on [GitHub](https://github.com/example/project/issues).

*Last updated: 2025-01-01*
"""

# ---------------------------------------------------------------------------
# Pre-built renderer instances (amortise construction cost)
# ---------------------------------------------------------------------------

_md_it = markdown_it.MarkdownIt()
_md_stdlib = markdown.Markdown(
    extensions=["extra", "codehilite", "tables", "fenced_code"]
)
_engine = OxydeEngine()


def _render_stdlib(text: str) -> str:
    """Render using stdlib Markdown, resetting state between calls."""
    _md_stdlib.reset()
    return _md_stdlib.convert(text)


# ---------------------------------------------------------------------------
# Benchmarks: short document
# ---------------------------------------------------------------------------


class TestShortDocument:
    """Benchmarks on a single short paragraph."""

    def test_oxydemark(self, benchmark):
        benchmark(oxydemark.markdown_to_html, SHORT)

    def test_oxydemark_ast_round_trip(self, benchmark):
        def run():
            ast = oxydemark.parse(SHORT)
            return oxydemark.render_ast(ast)

        benchmark(run)

    def test_oxydemark_engine(self, benchmark):
        benchmark(_engine.render, SHORT)

    def test_markdown_it_py(self, benchmark):
        benchmark(_md_it.render, SHORT)

    def test_markdown_stdlib(self, benchmark):
        benchmark(_render_stdlib, SHORT)


# ---------------------------------------------------------------------------
# Benchmarks: medium document
# ---------------------------------------------------------------------------


class TestMediumDocument:
    """Benchmarks on a medium document with mixed elements."""

    def test_oxydemark(self, benchmark):
        benchmark(oxydemark.markdown_to_html, MEDIUM)

    def test_oxydemark_ast_round_trip(self, benchmark):
        def run():
            ast = oxydemark.parse(MEDIUM)
            return oxydemark.render_ast(ast)

        benchmark(run)

    def test_oxydemark_engine(self, benchmark):
        benchmark(_engine.render, MEDIUM)

    def test_markdown_it_py(self, benchmark):
        benchmark(_md_it.render, MEDIUM)

    def test_markdown_stdlib(self, benchmark):
        benchmark(_render_stdlib, MEDIUM)


# ---------------------------------------------------------------------------
# Benchmarks: long document
# ---------------------------------------------------------------------------


class TestLongDocument:
    """Benchmarks on a long document with tables, code blocks, and lists."""

    def test_oxydemark(self, benchmark):
        benchmark(oxydemark.markdown_to_html, LONG)

    def test_oxydemark_ast_round_trip(self, benchmark):
        def run():
            ast = oxydemark.parse(LONG)
            return oxydemark.render_ast(ast)

        benchmark(run)

    def test_oxydemark_engine(self, benchmark):
        benchmark(_engine.render, LONG)

    def test_markdown_it_py(self, benchmark):
        benchmark(_md_it.render, LONG)

    def test_markdown_stdlib(self, benchmark):
        benchmark(_render_stdlib, LONG)


# ---------------------------------------------------------------------------
# Benchmarks: parse only (where applicable)
# ---------------------------------------------------------------------------


class TestParseOnly:
    """Benchmarks for parsing only (no rendering)."""

    def test_oxydemark_parse_short(self, benchmark):
        benchmark(oxydemark.parse, SHORT)

    def test_oxydemark_parse_medium(self, benchmark):
        benchmark(oxydemark.parse, MEDIUM)

    def test_oxydemark_parse_long(self, benchmark):
        benchmark(oxydemark.parse, LONG)

    def test_markdown_it_py_parse_short(self, benchmark):
        benchmark(_md_it.parse, SHORT)

    def test_markdown_it_py_parse_medium(self, benchmark):
        benchmark(_md_it.parse, MEDIUM)

    def test_markdown_it_py_parse_long(self, benchmark):
        benchmark(_md_it.parse, LONG)
