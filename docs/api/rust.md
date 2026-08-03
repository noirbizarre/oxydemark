# Rust API

The `oxydemark` crate is built as both a `cdylib` (the Python extension module)
and an `rlib`. Downstream crates -- notably OxydePress -- link the `rlib` and use
the parser, the AST and the renderer **without any PyO3 dependency**: the
bindings live behind the optional `python` feature, and the default feature set
is empty.

<div class="grid cards" markdown>

- **[Browse the rustdoc reference :material-arrow-right:](../rust/oxydemark/index.html)**

    Generated with `cargo doc --no-deps` and published alongside this site.

</div>

## Frozen surface

Everything re-exported from the crate root is public and frozen under
[OMEP-0008](../specs/OMEP-0008-public-api.md); every other module is private.

| Item | Kind | Purpose |
| --- | --- | --- |
| `parse` | function | Parse Markdown into an `AstNode` tree. |
| `parse_document` | function | Parse and compute structured metadata (OMEP-0010). |
| `render_ast` | function | Render an `AstNode` tree to HTML. |
| `markdown_to_html` | function | Convert Markdown to HTML in one pass. |
| `slugify` | function | Derive a unique, URL-friendly anchor slug. |
| `extract_summary` | function | Extract the summary before a `<!-- more -->` delimiter. |
| `AstNode` | struct | The AST node type. |
| `Heading` | struct | A heading entry or TOC tree node. |
| `ParseResult` | struct | AST plus headings, TOC, summary and frontmatter. |
| `OxydeError` | enum | The crate's error type. |
| `Meta` | enum | Typed metadata value (re-exported from `rushdown`), used by `ParseResult::frontmatter` and `AstNode::props`. |

## Usage

```toml
[dependencies]
oxydemark = "0.1"
```

```rust
use oxydemark::{markdown_to_html, parse};

let html = markdown_to_html("# Hello")?;
let root = parse("# Hello")?;
```

## Feature flags

| Feature | Default | Effect |
| --- | --- | --- |
| *(none)* | yes | Pure Rust, no PyO3 dependency. |
| `python` | no | Builds the PyO3 binding layer. |
| `extension-module` | no | Implies `python`, plus `pyo3/extension-module`, used by maturin for the wheel. |
