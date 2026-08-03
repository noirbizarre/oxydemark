---
status: accepted
date: 2026-03-22
decision-makers: [Axel H.]
---

# OMEP-0006: Markdown Parser -- Rushdown

## Context and Problem Statement

OMEP-0001 established a Rust core with Python bindings as the project
architecture, but did not prescribe a specific Markdown parsing library for the
Rust side. We need to choose a parser that aligns with OxydeMark's core
requirements: AST-based processing, extensibility, CommonMark compliance, and
high performance.

## Decision Drivers

* **AST-based architecture** -- The parser must produce a concrete, walkable
  AST (not a streaming / event-based API) so that transformations can be
  applied by both Rust and Python plugins.
* **Extensibility** -- Plugin authors must be able to add custom block-level
  parsers, inline-level parsers, AST transformers, and renderers *from outside
  the crate*, without forking.
* **Standards compliance** -- Full CommonMark 0.31.2 compliance is mandatory;
  GitHub Flavored Markdown (GFM) support is highly desirable.
* **Performance** -- Parsing and rendering must be fast, even for large
  documents.
* **Ecosystem** -- Existing extensions (frontmatter, emoji, syntax
  highlighting, etc.) reduce the amount of custom code we need to write.

## Considered Options

* **Option A: pulldown-cmark** -- Streaming / event-based parser.
* **Option B: comrak** -- AST-based, C-compatible API.
* **Option C: markdown-rs** -- Event-based, Rust-native.
* **Option D: rushdown** -- AST-based, extensible, by the author of goldmark.

## Decision Outcome

Chosen option: **"rushdown"** (Option D), because it is the only Rust Markdown
parser that simultaneously provides a concrete AST, an extensible
parser/renderer architecture via constructor injection, full CommonMark 0.31.2
compliance, GFM support, and best-in-class performance.

### Consequences

* Good, because rushdown's AST can be walked and converted to a Python-friendly
  tree (`AstNode`) for plugin consumption.
* Good, because rushdown's `ParserExtension` / `RendererExtension` traits allow
  adding custom syntax (needed for OMEP-0007 comark syntax) without forking.
* Good, because the ecosystem already provides extensions we need:
  `rushdown-meta` (YAML frontmatter) and `rushdown-emoji` (`:shortcode:`
  emojis).
* Good, because rushdown benchmarks at ~3.3 ms on the CommonMark spec suite,
  faster than comrak (~4.2 ms) and pulldown-cmark (~6.0 ms).
* Neutral, because rushdown is relatively new, but it is actively maintained,
  fuzz-tested, and authored by the creator of goldmark (the de-facto Go
  Markdown parser).
* Bad, because the arena-based AST needs a conversion layer to expose a
  Python-friendly tree structure.

### Confirmation

* `cargo test` validates that rushdown parses and renders CommonMark correctly.
* A Python round-trip test confirms `parse()` returns a valid `AstNode` tree
  and `render_ast()` produces correct HTML.
* Benchmark tests compare rushdown performance against the previous placeholder.

## Pros and Cons of the Options

### Option A: pulldown-cmark

* Good, because it is the most widely used Rust Markdown parser.
* Good, because it is fast (~6.0 ms).
* Bad, because it is event/streaming-based -- no concrete AST to walk or
  expose to Python.
* Bad, because extensibility is very limited; adding custom syntax requires
  post-processing the event stream.

### Option B: comrak

* Good, because it produces a concrete AST.
* Good, because it supports CommonMark + GFM + extensions.
* Bad, because its extension API is less composable -- extensions are
  compile-time features, not runtime-injected parsers.
* Neutral, because performance is acceptable (~4.2 ms) but not best-in-class.

### Option C: markdown-rs

* Good, because it is a pure-Rust implementation.
* Bad, because it is event-based, not AST-based.
* Bad, because performance is poor (~89.7 ms), an order of magnitude slower.

### Option D: rushdown (Chosen)

* Good, because it builds a clean, arena-based AST designed for traversal and
  manipulation.
* Good, because custom parsers, transformers, and renderers can be injected at
  runtime via `ParserExtension` / `RendererExtension` traits.
* Good, because it is the fastest (~3.3 ms) while maintaining full compliance.
* Good, because the ecosystem provides ready-made extensions.
* Neutral, because the arena-based AST requires a bridge to Python-friendly
  objects.

## Ecosystem

The following rushdown extensions are adopted as initial dependencies:

| Crate | Version | Role |
| ----- | ------- | ---- |
| `rushdown` | 0.18 | Core parser and HTML renderer |
| `rushdown-meta` | =0.9.9 | YAML frontmatter parsing |
| `rushdown-emoji` | =0.9.8 | `:shortcode:` emoji support (parser + renderer) |

Additional extensions available for future integration:

| Crate | Role |
| ----- | ---- |
| `rushdown-footnote` | Footnote syntax |
| `rushdown-highlighting` | Syntax highlighting for code blocks |
| `rushdown-diagram` | Diagram visualization (e.g. MermaidJS) |

### Dependency pinning

`rushdown-meta` and `rushdown-emoji` raise their own `rushdown` requirement
across **patch** releases of a single minor line (`rushdown-meta` 0.9.3 requires
`rushdown ^0.11`, 0.9.9 requires `^0.18`). A caret requirement such as
`rushdown-meta = "0.9"` is therefore not expressible: it floats to the newest
patch, which demands a `rushdown` major we do not use, and Cargo resolves *two*
incompatible `rushdown` copies into the downstream graph. The repository's own
`Cargo.lock` hides this, because lockfiles are not honoured for library
consumers — which is exactly how a published `oxydemark` 0.2 shipped
uncompilable downstream (issue #33).

The companion crates are consequently pinned **exactly**:

```toml
rushdown = "0.18"
rushdown-meta = "=0.9.9"
rushdown-emoji = "=0.9.8"
```

The exact pin is not conservatism, it is the only faithful expression of the
constraint. `rushdown::ast::Meta` is part of oxydemark's public API
(`ParseResult::frontmatter`, `AstNode::props`), so a `rushdown` major bump is a
breaking change for oxydemark too and must be a deliberate, released decision.

Two CI guards keep this honest:

* the `package` job in `ci.yml` runs `cargo update && cargo build`, reproducing
  the fresh resolution a downstream consumer performs — it fails if the pins are
  ever loosened back to a caret;
* `deps.yml` runs weekly on nightly with `cargo update --breaking`, deliberately
  breaking the pins. A red run means a new `rushdown` line is out and an upgrade
  is due. It is not attached to pull requests, so upstream releases never break
  contributor CI.

## More Information

* [rushdown documentation](https://docs.rs/rushdown)
* [rushdown repository](https://github.com/yuin/rushdown)
* [rushdown-meta documentation](https://docs.rs/rushdown-meta)
* [rushdown-emoji documentation](https://docs.rs/rushdown-emoji)
* [CommonMark 0.31.2 specification](https://spec.commonmark.org/0.31.2/)
* Related: [OMEP-0001](OMEP-0001-project-architecture.md) (project
  architecture), [OMEP-0007](OMEP-0007-comark-syntax.md) (comark syntax)
