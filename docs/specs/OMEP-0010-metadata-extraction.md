---
status: proposed
date: 2026-07-12
decision-makers: [Axel H.]
---

# OMEP-0010: Structured metadata extraction (TOC / anchors / summary)

## Context and Problem Statement

OxydePress, the primary downstream consumer of OxydeMark, needs structured
metadata derived from parsed documents in order to build pages: stable heading
anchors (for deep links and a "copy link" affordance), a table-of-contents tree
(for page navigation), an excerpt/summary (for listings and social previews),
and typed access to YAML frontmatter (for page configuration).

Today the parse result (`AstNode`, see OMEP-0008) exposes only the raw tree plus
a flat `metadata: Option<HashMap<String, String>>` on the document node. That
map is stringly-typed (every value is coerced to a string, see
`src/ast.rs:277`), headings carry no `id`, and there is no notion of a TOC or an
excerpt. As a result every consumer re-implements the same slugging, tree
building, and summary logic, and they do so inconsistently.

This metadata belongs in the OxydeMark core so that both the Rust (`rlib`) and
Python (`oxydemark._core`) surfaces share a single, tested implementation. This
OMEP specifies **the APIs and their AST / Python representations**. It does not
implement them; implementation is tracked as a follow-up issue, consistent with
how OMEP-0007 specified Phase 3 shapes ahead of the code.

## Decision Drivers

* **Shared implementation** -- Rust and Python consumers must get identical
  anchors, TOC, and summaries; the logic lives once, in the core.
* **Stable, deterministic anchors** -- Heading IDs must be stable across parses
  and unique within a document so links do not break.
* **Ergonomic consumption** -- OxydePress should read `result.toc`,
  `result.headings`, `result.summary`, and `result.frontmatter` without
  walking the tree by hand.
* **Typed frontmatter** -- Frontmatter values must preserve their native YAML
  types (numbers, booleans, lists, nested maps), not be flattened to strings as
  the current `metadata` map does.
* **Backward compatibility** -- The existing `AstNode` surface (OMEP-0008) and
  `parse()` return type must keep working; new metadata is additive.
* **Constraints from AGENTS.md** -- Rust edition 2024, `///` docs on public
  items, `Result<T, E>` over panics, clippy-clean; extensions in
  `src/extensions.rs`, AST bridge in `src/ast.rs`, PyO3 wiring in `src/lib.rs`.

## Considered Options

* **Option A: Plugins only** -- Leave metadata to Python plugins (anchors, TOC,
  summary each a plugin) and keep the core unaware.
* **Option B: Post-hoc helpers on `AstNode`** -- Add free functions/methods
  (`node.slug()`, `build_toc(node)`) that consumers call after `parse()`, but do
  not change the parse result shape.
* **Option C: A dedicated `ParseResult` carrying typed metadata** -- Introduce a
  richer parse result that bundles the `AstNode` root with computed `headings`,
  `toc`, `summary`, and typed `frontmatter`, populated by the core during (or
  immediately after) parsing.

## Decision Outcome

Chosen option: **Option C -- a dedicated parse result carrying typed metadata**,
because it gives both ecosystems one computed-once, tested source of truth and
an ergonomic surface (`result.toc`, `result.frontmatter`, ...) while remaining
additive: the underlying `AstNode` tree is unchanged and still reachable via
`result.root`. Anchors, being an intrinsic property of headings, are also
written back onto the heading nodes so tree-walking consumers and the HTML
renderer see them.

Because introducing a new return type from `parse()` would break the OMEP-0008
surface, the metadata is exposed through a **new function** and a **new result
type**; the existing `parse()` / `render_ast()` / `markdown_to_html()` functions
keep their current signatures.

### New public surface

Added to the surfaces frozen in OMEP-0008 (all additive):

| Item | Kind | Surface | Description |
| ---- | ---- | ------- | ----------- |
| `parse_document(markdown) -> ParseResult` | function | Rust + Python | Parse and compute structured metadata. |
| `ParseResult` | struct/class | Rust + Python | `root: AstNode`, `headings`, `toc`, `summary`, `frontmatter`. |
| `Heading` | struct/class | Rust + Python | `level`, `id`, `text`, `children`. |
| `slugify(text, existing=None) -> str` | function | Rust + Python | The anchor algorithm, exposed for reuse. |

`ParseResult` fields:

* `root: AstNode` -- the same tree `parse()` returns, with heading `id`
  attributes populated (see Anchors).
* `headings: list[Heading]` -- headings in document order (flat).
* `toc: list[Heading]` -- the nested table-of-contents tree (see TOC).
* `summary: str | None` -- HTML of the content before the summary delimiter, or
  `None` when no delimiter is present (see Summary).
* `frontmatter: dict | None` -- typed YAML frontmatter, or `None` (see
  Frontmatter).

On the **Rust** surface (PyO3-independent, see OMEP-0008), the signatures and
frontmatter type differ from Python:

* `parse_document(markdown: &str) -> ParseResult` -- no `Python` token, no
  `PyResult` (parsing is infallible).
* `ParseResult.frontmatter: Option<rushdown::ast::Meta>` -- the typed YAML
  frontmatter as a native Rust `Meta` value, preserving native YAML types. The
  Python binding converts this to a `dict` via a computed getter (using the
  internal `meta_to_py` converter); Python callers see `dict | None` as above.

`parse()` remains the tree-only fast path; `parse_document()` is the
metadata-aware path. This mirrors the existing `parse()` vs
`markdown_to_html()` split.

### Consequences

* Good, because consumers read typed metadata directly instead of re-walking the
  tree, and every consumer gets the same result.
* Good, because heading anchors are written onto the AST, so the renderer emits
  `<h2 id="...">` and TOC links resolve without extra work.
* Good, because it is purely additive to the OMEP-0008 surface: no existing
  signature changes.
* Neutral, because `parse_document()` does more work than `parse()`; consumers
  that only need the tree keep using `parse()`.
* Bad, because typed `frontmatter` and the current stringly-typed
  `AstNode.metadata` now coexist; `metadata` is documented as deprecated in
  favour of `ParseResult.frontmatter` (removal is a pre-1.0 follow-up per
  OMEP-0008's 0.x policy).
* Bad, because `frontmatter` requires a typed (`Py<PyDict>`-backed) value, the
  same representation challenge already noted for `props` in OMEP-0007; the two
  should share one implementation.

### Confirmation

* Rust unit tests under `#[cfg(test)]` in `src/ast.rs` cover the slug algorithm
  (including collisions and Unicode), TOC nesting, summary splitting, and typed
  frontmatter round-tripping.
* Python tests under `tests/` assert `parse_document(...).toc`, `.headings`,
  `.summary`, and `.frontmatter` shapes, extending the `tests/test_core.py`
  patterns.
* `ls docs/specs/OMEP-0010-metadata-extraction.md` confirms this OMEP exists.
* `mise run ci` stays green.

## Specification

### Heading slug / anchor algorithm

Every `heading` node receives a deterministic `id` attribute derived from its
rendered text content. The algorithm (`slugify`) is:

1. Collect the heading's text content by concatenating the text of all
   descendant text-bearing nodes (reusing the existing `collect_text` helper in
   `src/html_render.rs`). Emoji contribute their shortcode, not the Unicode
   character, so anchors stay ASCII-friendly.
2. Apply Unicode NFKD normalization and lowercase the result.
3. Replace any run of characters that is not `[a-z0-9]` with a single hyphen
   (`-`). Combining marks left by normalization are stripped.
4. Trim leading and trailing hyphens.
5. If the result is empty (e.g. a heading of only punctuation or emoji),
   fall back to `section`.

**ID collision handling.** IDs must be unique within a document. A per-document
set of already-assigned slugs is threaded through metadata computation. When a
freshly-computed slug is already taken, append `-N` where `N` is the smallest
integer `>= 1` that yields an unused slug:

```
"Overview"          -> "overview"
"Overview"  (again) -> "overview-1"
"Overview"  (again) -> "overview-2"
```

The suffixed candidate is itself checked against the set, so a document that
literally contains a `## Overview 1` heading still gets distinct IDs.

An **author-provided id wins.** If the heading already carries an `id` (via the
comark attribute syntax, e.g. `## Title {#custom}`, parsed by the
`attributes: true` option in `build_parser`, `src/lib.rs:37`), that id is used
verbatim and only participates in collision *detection* (it reserves its slot;
it is never itself renumbered). Generated slugs then avoid it.

`slugify(text: str, existing: set[str] | None = None) -> str` is exposed so
plugins and downstream code can produce anchors with identical semantics; when
`existing` is provided it applies the same `-N` disambiguation and the caller is
expected to add the returned slug to the set.

### Table-of-contents tree

`Heading` is the TOC node type on both surfaces:

| Field | Type | Description |
| ----- | ---- | ----------- |
| `level` | `int` (1--6) | Heading level. |
| `id` | `str` | The anchor id assigned above. |
| `text` | `str` | Plain-text heading label (same source as the slug, before slugging). |
| `children` | `list[Heading]` | Nested sub-headings. |

`ParseResult.headings` is the **flat** list in document order.
`ParseResult.toc` is the **nested** tree built from it:

* A heading of level `L` becomes a child of the nearest preceding heading whose
  level is `< L`.
* Headings whose level is not strictly greater than every open ancestor pop the
  stack until the parent constraint holds; top-level headings (no shallower
  ancestor) are roots of `toc`.
* **Level skips are tolerated.** `#` followed directly by `###` nests the
  `###` under the `#` (the skipped `##` level is simply absent); the tree is
  built structurally from relative levels, not by requiring contiguous levels.
* Headings inside block components / slots (OMEP-0007) are included and appear in
  document order like any other heading.

Example:

```markdown
# Title
## Setup
## Usage
### CLI
### Library
## FAQ
```

```python
result = oxydemark.parse_document(src)
[(h.level, h.id) for h in result.headings]
# [(1,'title'), (2,'setup'), (2,'usage'), (3,'cli'), (3,'library'), (2,'faq')]

result.toc[0].id                    # 'title'
[c.id for c in result.toc[0].children]      # ['setup', 'usage', 'faq']
[c.id for c in result.toc[0].children[1].children]  # ['cli', 'library']
```

### Summary extraction via `<!-- more -->`

The summary is the content that precedes an explicit delimiter comment:

* The delimiter is an HTML comment whose trimmed body is exactly `more`:
  `<!-- more -->`. Matching is case-insensitive and tolerant of internal
  whitespace (`<!--more-->`, `<!--   more   -->`).
* Only the **first** delimiter at the **top level** of the document (a direct
  child of the `document` node, i.e. its own `html_block` / `raw_html`) is
  significant; delimiters nested inside other blocks are ignored.
* `ParseResult.summary` is the **rendered HTML** of every top-level block that
  appears *before* the delimiter, produced by the same renderer as
  `render_ast()` so summary and full-body markup are consistent.
* When no delimiter is present, `summary` is `None` (callers decide whether to
  synthesise a fallback, e.g. first paragraph or first N characters; the core
  does not guess).
* The delimiter node itself is **not** removed from `root`; it renders to nothing
  (an HTML comment) and full-document rendering is unaffected.

```markdown
Intro paragraph shown in listings.

<!-- more -->

The rest of the article.
```

```python
result.summary   # "<p>Intro paragraph shown in listings.</p>\n"
```

### Typed frontmatter access

`ParseResult.frontmatter` exposes YAML frontmatter with native types:

* `frontmatter: dict | None` -- a mapping of top-level key to a native Python
  value (`str`, `int`, `float`, `bool`, `list`, `dict`, `None`), preserving the
  YAML structure. It is `None` when the document has no frontmatter block.
* This supersedes `AstNode.metadata`, which flattens every value to a string
  (`src/ast.rs:284`). `metadata` remains on the document node for backward
  compatibility but is documented as deprecated in favour of `frontmatter`.
* Frontmatter is parsed once by `rushdown-meta` (already enabled in
  `build_parser`, `src/lib.rs:32`); `frontmatter` reflects the same source
  without the string coercion.

> **Implementation note.** A typed mapping needs a `Py<PyDict>`-backed field,
> exactly the representation `props` requires in OMEP-0007. The conversion from
> the `rushdown-meta` value model to a native Python value should be implemented
> once and reused by both `frontmatter` and component `props`. This is tracked as
> a shared follow-up implementation issue; this OMEP only specifies the shape.

### Rendering interaction

* Heading `id`s populated on `root` are emitted by the renderer as
  `<h{level} id="...">`, so anchors are present in `render_ast(result.root)` and
  in `markdown_to_html` output once the anchoring pass is wired in.
* The summary is rendered from the same `AstNode` subtree as the body, so a block
  renders identically whether it appears in `summary` or the full document.

## Pros and Cons of the Options

### Option A: Plugins only

* Good, because the core stays minimal and metadata is opt-in.
* Bad, because every consumer re-implements slugging/TOC/summary, and they drift.
* Bad, because plugins cannot easily share a document-wide collision set or a
  typed frontmatter representation without duplicating core internals.
* Bad, because Rust-only (`rlib`) consumers get nothing.

### Option B: Post-hoc helpers on `AstNode`

* Good, because it is additive and needs no new result type.
* Good, because helpers are composable.
* Bad, because computing headings, TOC, and summary separately re-walks the tree
  multiple times and re-derives the collision set each call.
* Bad, because there is no natural home for typed `frontmatter` distinct from the
  stringly-typed `metadata`.

### Option C: Dedicated `ParseResult` (Chosen)

* Good, because metadata is computed once, together, and shared by both surfaces.
* Good, because consumption is ergonomic (`result.toc`, `result.frontmatter`).
* Good, because it is additive: `parse()` and the OMEP-0008 surface are unchanged.
* Neutral, because it adds a second parse entry point (`parse_document`).
* Bad, because it introduces `frontmatter`/`metadata` duality until `metadata` is
  removed pre-1.0.

## More Information

* Related: [OMEP-0001](OMEP-0001-project-architecture.md) (pipeline and
  Rust/Python split), [OMEP-0006](OMEP-0006-markdown-parser.md) (rushdown parser
  and `rushdown-meta` frontmatter), [OMEP-0007](OMEP-0007-comark-syntax.md)
  (heading attributes, and the typed `props` representation reused here for
  `frontmatter`), [OMEP-0008](OMEP-0008-public-api.md) (public API surface and
  0.x versioning policy this addition follows).
* Prior art on slug algorithms: GitHub's `gh-anchor` behaviour and Python
  `python-markdown`'s `toc` extension (lowercase, hyphenate, `-N`
  disambiguation).
* Follow-up actions:
  * Implement `slugify`, `parse_document`, `ParseResult`, and `Heading` in
    `src/ast.rs` with PyO3 wiring in `src/lib.rs`.
  * Implement the anchoring pass that writes heading `id`s onto `root`.
  * Share the `rushdown-meta`/YAML -> native Python conversion with OMEP-0007
    component `props`.
  * Add Rust `#[cfg(test)]` and Python `tests/` coverage per Confirmation.
  * Deprecate and later remove `AstNode.metadata` in favour of
    `ParseResult.frontmatter`.
