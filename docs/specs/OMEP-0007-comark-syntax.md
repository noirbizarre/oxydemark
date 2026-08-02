---
status: accepted
date: 2026-03-22
decision-makers: [Axel H.]
---

# OMEP-0007: Extended Syntax -- Comark Specification

## Context and Problem Statement

Standard CommonMark and GitHub Flavored Markdown lack several features that
modern content authoring demands: embedding custom components in Markdown,
attaching attributes (classes, IDs, styles) to elements, and wrapping inline
text in attributed spans. OxydeMark needs to adopt an extended Markdown syntax
specification that fills these gaps while remaining compatible with CommonMark.

## Decision Drivers

* **Component authoring** -- Authors must be able to embed custom UI components
  directly in Markdown (e.g. alerts, cards, badges) without falling back to raw
  HTML.
* **Attribute support** -- Elements should accept classes, IDs, data attributes,
  and inline styles via a clean, non-HTML syntax.
* **CommonMark compatibility** -- The extended syntax must be a strict superset
  of CommonMark; valid CommonMark documents must parse identically.
* **Established specification** -- Adopting a documented, community-tested
  syntax reduces ambiguity and avoids inventing yet another Markdown dialect.
* **Implementability in Rust** -- The syntax must be implementable as rushdown
  parser extensions (see OMEP-0006).

## Considered Options

* **Option A: Custom proprietary syntax** -- Design our own extensions from
  scratch.
* **Option B: MDX** -- JSX-in-Markdown, popularized by the React ecosystem.
* **Option C: Comark syntax specification** -- A well-documented superset of
  CommonMark/GFM with components, attributes, and span syntax.

## Decision Outcome

Chosen option: **"Comark syntax specification"** (Option C), because it
provides a clean, well-documented syntax for components and attributes that is
a strict superset of CommonMark, is framework-agnostic, and can be implemented
incrementally as rushdown parser extensions.

**Important:** We adopt the *syntax specification* from comark, not its
JavaScript/TypeScript implementation. The parsing is handled entirely by
rushdown extensions in Rust.

### Consequences

* Good, because the component syntax (`::block` / `:inline`) is clean and
  readable, unlike raw HTML or JSX.
* Good, because the attribute syntax (`{.class #id key="value"}`) is concise
  and already partially supported by rushdown's built-in attribute parser.
* Good, because the span attribute syntax (`[text]{.class}`) enables inline
  styling without custom components.
* Good, because the specification is thoroughly documented with AST examples,
  reducing implementation ambiguity.
* Good, because comark is framework-agnostic -- the syntax works for HTML,
  Vue, React, or any rendering target.
* Neutral, because implementing the full comark component syntax as rushdown
  extensions is non-trivial and will be done in phases.
* Bad, because comark is a relatively young specification; breaking changes are
  possible.

### Confirmation

* Phase 1 features (attributes, span attributes, frontmatter, emoji) are
  validated by unit tests comparing parsed AST against expected node trees.
* Phase 2 features (block components, inline components, slots) are validated
  by integration tests with representative documents.
* A compliance test suite derived from comark's own documentation examples
  ensures specification fidelity.

## Pros and Cons of the Options

### Option A: Custom Proprietary Syntax

* Good, because we have full control over the design.
* Bad, because it creates a fragmented ecosystem -- yet another Markdown
  dialect.
* Bad, because it requires writing and maintaining a full specification from
  scratch.
* Bad, because users cannot leverage existing tooling or documentation.

### Option B: MDX

* Good, because it is widely adopted in the React ecosystem.
* Good, because JSX is familiar to web developers.
* Bad, because it is tightly coupled to JavaScript/JSX -- poor fit for a
  Rust/Python project.
* Bad, because MDX is not a strict superset of CommonMark (JSX can conflict
  with Markdown syntax).
* Bad, because implementing a JSX parser in Rust adds significant complexity.

### Option C: Comark Syntax Specification (Chosen)

* Good, because it is a strict superset of CommonMark/GFM.
* Good, because the syntax is clean, readable, and well-documented.
* Good, because it is framework-agnostic.
* Good, because rushdown's extensibility makes it feasible to implement as
  parser/renderer extensions.
* Neutral, because the specification is young and may evolve.

## Syntax Overview

### Attributes on Elements

```markdown
**bold text**{.highlight #important}
[Link](url){target="_blank" rel="noopener"}
![Image](img.png){.responsive width="800"}
```

### Span Attributes

```markdown
This is [highlighted text]{.highlight style="color: blue"} in a paragraph.
```

### Block Components

```markdown
::alert{type="info"}
This is an alert message with **Markdown** support.
::
```

### Inline Components

```markdown
Check out this :badge[New]{color="blue"} feature.
```

### Component Slots

A block component body may be partitioned into named *slots*. A line whose only
content is `#slot-name` at the top level of a component body opens a slot; every
subsequent block until the next `#slot-name` marker (or the closing `::`) belongs
to that slot.

```markdown
::card
#header
## Card Title

#content
Main content here.
::
```

Content that appears *before* any `#slot-name` marker belongs to the implicit
`#default` slot. The default slot may also be named explicitly, which is useful
when mixing it with named slots:

```markdown
::card
#default
This is the **default** slot content.

#footer
Footer content here.
::
```

**Slot grammar:**

* A slot marker is a line matching `^#[A-Za-z][A-Za-z0-9_-]*$` at the top level
  of a block-component body (not nested inside another block).
* Slot names match `[A-Za-z][A-Za-z0-9_-]*`.
* The reserved name `default` denotes the default slot.
* Slot markers must come *after* any block props (see below) and *before* the
  closing `::`.

### YAML frontmatter props (Block Props)

For components with many or typed properties, a YAML block may be placed at the
very beginning of a component body. Two equivalent styles are supported.

**Frontmatter style** — delimited by `---`:

```markdown
::card
---
variant: elevated
count: 42
enabled: true
tags:
  - markdown
  - docs
---
Card content here.
::
```

**Codeblock style** — a fenced block tagged `yaml [props]`:

````markdown
::card
```yaml [props]
variant: elevated
count: 42
enabled: true
tags:
  - markdown
  - docs
```
Card content here.
::
````

**Block-props rules:**

* The YAML block must appear immediately after the opening `::component` line,
  **before** any slot markers or other content.
* Values are typed: scalars, arrays, and nested objects are preserved as their
  native types (not stringified).
* Block props are merged with inline `{…}` attributes. **Inline attributes take
  precedence** on key collisions.

```markdown
::card{.featured}
---
title: Featured Article
author: Jane Doe
---
Combines inline class `.featured` with typed YAML props.
::
```

### Nested Components

Block components nest by using a *run* of colons. An opener with a run of *n* ≥ 2
colons is closed by a line consisting solely of the same *n* colons. Nesting is
resolved by matching opener and closer colon counts, so a `:::` block closes on a
`:::` line, not on a bare `::`:

```markdown
:::outer
::inner{variant="compact"}
Content
::
:::
```

Deep nesting increases the colon run at each level for readability:

```markdown
::level-1
:::level-2
::::level-3
Content
::::
:::
::
```

Adding extra colons is a *readability convention*, not a requirement — the parser
resolves nesting structurally by matching openers to closers. Equal-colon nesting
is therefore also valid:

```markdown
::level-1
::level-2
Content
::
::
```

## AST Representation

OxydeMark exposes a tree-based `AstNode` to Python plugins (`src/ast.rs`). Unlike
comark's array-based AST (`[tag, props, ...children]`), OxydeMark nodes carry a
`kind` string, a `children` list, an `attributes` string→string map, and an
optional `metadata` map. Phase 1 and 2 already surface components as
`kind="block_component"` / `kind="inline_component"` / `kind="span_attributes"`
with the component name in `attributes["name"]`.

Phase 3 extends this model as follows.

### Component props

Inline `{…}` attributes continue to populate the `attributes` string map, exactly
as in Phase 2:

* `.class` shorthands merge into `attributes["class"]` (space-separated).
* `#id` populates `attributes["id"]`.
* `key="value"` pairs become `attributes[key] = value`.
* Boolean props (a bare `key`) are exposed as `:key = "true"` to match comark's
  convention of prefixing value-less/typed props with `:`.

Typed **block props** (YAML frontmatter/codeblock style) cannot be represented in
the `attributes: HashMap<String, String>` map because they may be numbers,
booleans, arrays, or nested objects. Phase 3 therefore specifies a new **typed
`props` field** on component nodes:

* `props: Option[dict]` — a nullable mapping of property name to a native Python
  value (`str`, `int`, `float`, `bool`, `list`, `dict`, or `None`).
* `props` is `None` when the component declares no block props.
* Merge order: block props are computed first, then inline `attributes` override
  colliding keys (inline attributes take precedence).

> **Implementation note.** The `props` field requires extending the `AstNode`
> struct in `src/ast.rs` (a PyO3-backed `Option<Py<PyDict>>` or equivalent). This
> is tracked as a separate implementation issue; this OMEP only specifies the
> shape.

### Slots

A named slot is a synthetic node with:

* `kind = "slot"`
* `attributes["name"]` = the slot name (e.g. `"header"`, `"content"`, or the
  reserved `"default"`)
* `children` = the parsed block content assigned to that slot

A block component that uses named slots has `slot` nodes as its direct children,
in document order:

```python
# ::card
# #header
# ## Card Title
#
# #content
# Main content here
# ::

card = ast.children[0]
assert card.kind == "block_component"
assert card.attributes["name"] == "card"

header, content = card.children  # both kind == "slot"
assert header.kind == "slot"
assert header.attributes["name"] == "header"
assert header.children[0].kind == "heading"      # ## Card Title

assert content.attributes["name"] == "content"
assert content.children[0].kind == "paragraph"   # Main content here
```

**Default-slot rule.** To keep simple components ergonomic:

* An *implicit* default slot (content before any `#name` marker, with no explicit
  `#default`) is **not** wrapped in a `slot` node — the content nodes are direct
  children of the component, exactly as in Phase 2. This preserves backward
  compatibility with existing `block_component` handling.
* An *explicit* `#default` marker produces a `slot` node with
  `attributes["name"] == "default"`.

### Nested components

A nested block component is simply a `block_component` node appearing in the
`children` of another `block_component` (or of a `slot`), to arbitrary depth:

```python
# :::outer
# ::inner
# Content
# ::
# :::

outer = ast.children[0]
assert outer.kind == "block_component"
assert outer.attributes["name"] == "outer"

inner = outer.children[0]
assert inner.kind == "block_component"
assert inner.attributes["name"] == "inner"
assert inner.children[0].kind == "paragraph"      # Content
```

### Rendering Rules

The default HTML renderer (`src/extensions.rs`) extends the existing Phase 2
behaviour (`<div>` for block components, `<span>` for inline components):

* **Slots.** Slots are a structuring concept, not an HTML element. The default
  renderer emits each slot's children inside a wrapper `<div data-slot="name">`
  within the enclosing component's `<div>`, so plugins and component frameworks
  can target slots. An implicit default slot (direct children) renders with no
  extra wrapper.
* **Block props.** Typed props are for plugin/component consumption and are
  **not** emitted as HTML attributes by the default renderer. Only HTML-valid
  inline attributes (`class`, `id`, `data-*`, `style`) are rendered onto the
  component element. `:`-prefixed and complex props are dropped from the default
  HTML output but remain available in the AST `props` field.
* **Nesting.** Nested components render as nested `<div>` (or `<span>`) elements,
  following child order.

## Compliance

A compliance suite derived from the comark documentation examples validates the
Phase 3 implementation. Each example below maps a comark source to the expected
OxydeMark `AstNode` shape. Where OxydeMark intentionally diverges from comark's
array AST, the mapping is noted.

Reference sources:

* [Comark component syntax](https://comark.dev/syntax/components) — slots, block
  props, data binding, nesting.
* [Comark AST specification](https://comark.dev/syntax/comark-ast) — canonical
  node shapes.

### Slots

Comark represents each slot as a `["template", { "name": ... }, ...]` node.
OxydeMark maps this to `kind="slot"` with `attributes["name"]`.

| Comark source | Comark AST | OxydeMark `AstNode` |
| ------------- | ---------- | ------------------- |
| `::card`<br>`#header`<br>`## Card Title`<br><br>`#content`<br>`Main content here`<br>`::` | `["card", {}, ["template", {"name":"header"}, ["h2", …]], ["template", {"name":"content"}, ["p", …]]]` | `block_component(name=card)` → children `slot(name=header)` → `heading`; `slot(name=content)` → `paragraph` |
| `::alert{type="info"}`<br>`This content goes to the default slot.`<br>`::` | `["alert", {"type":"info"}, ["p", …]]` | `block_component(name=alert, attributes.type=info)` → child `paragraph` (implicit default slot, no `slot` wrapper) |

### Block props

Comark keeps typed props in the element's props object; OxydeMark stores scalars
that are HTML-valid in `attributes` and the full typed set (arrays/objects/typed
scalars) in the new `props` field. Inline attributes take precedence on
collisions.

| Comark source | OxydeMark `AstNode` |
| ------------- | ------------------- |
| `::card`<br>`---`<br>`variant: elevated`<br>`count: 42`<br>`enabled: true`<br>`---`<br>`::` | `block_component(name=card, props={"variant":"elevated","count":42,"enabled":True})` |
| `::card{.featured}`<br>`---`<br>`title: Featured`<br>`variant: plain`<br>`---`<br>`::` | `attributes={"name":"card","class":"featured"}`, `props={"title":"Featured","variant":"plain"}` |
| `::component{disabled}` | `attributes={"name":"component",":disabled":"true"}` (boolean prop `:`-prefixed) |

### Nested components

Comark nests elements as children (`["outer", {}, ["inner", {}, …]]`). OxydeMark
nests `block_component` nodes as `children`.

| Comark source | OxydeMark `AstNode` |
| ------------- | ------------------- |
| `:::outer`<br>`::inner`<br>`Content`<br>`::`<br>`:::` | `block_component(name=outer)` → `block_component(name=inner)` → `paragraph` |
| `::level-1`<br>`:::level-2`<br>`::::level-3`<br>`Content`<br>`::::`<br>`:::`<br>`::` | three-deep `block_component` nesting (`level-1` → `level-2` → `level-3` → `paragraph`) |

The compliance suite is fixture-driven: JSON files under `tests/compliance/`
declare a Markdown input, the exact expected HTML and an optional partial AST
shape. They are consumed by two harnesses running the same files —
`tests/compliance.rs` (Rust) and `tests/test_compliance.py` (Python) — so both
language surfaces are held to a single contract. `tests/compliance/README.md`
documents the schema and how to add a case.

The hand-written unit tests in `src/api.rs` and `tests/test_core.py` are kept
alongside the fixtures on purpose: the overlap is a deliberate cross-check, not
an oversight.

## Implementation Phases

### Phase 1 (initial release)

Leverages existing rushdown features and ecosystem extensions:

* **YAML frontmatter** -- via `rushdown-meta`
* **Emoji shortcodes** -- via `rushdown-emoji`
* **Heading attributes** -- via rushdown's built-in `attributes` parser option

### Phase 2 (implemented)

Custom rushdown parser extensions implemented in `src/extensions.rs`:

* **Element attributes** -- `{.class #id key="value"}` on strong, emphasis,
  links, images, code spans (via rushdown's built-in `attributes` parser option)
* **Span attributes** -- `[text]{.class}` inline span syntax
* **Block components** -- `::component{props}` syntax
* **Inline components** -- `:component[content]{props}` syntax

### Phase 3 (implemented)

Specified by this OMEP (grammar in [Syntax Overview](#syntax-overview),
node shapes in [AST Representation](#ast-representation), output in
[Rendering Rules](#rendering-rules), and validation in [Compliance](#compliance))
and implemented in `src/extensions.rs`:

* **Component slots** -- `#slot-name` syntax within block components; maps to
  `kind="slot"` nodes.
* **YAML frontmatter props** -- typed YAML blocks inside components; maps to the
  new `props` field on component nodes.
* **Nested components** -- multi-colon nesting (`:::`, `::::`, etc.); maps to
  nested `block_component` children.

## More Information

* [Comark syntax documentation](https://comark.dev/syntax/markdown)
* [Comark component syntax](https://comark.dev/syntax/components)
* [Comark attribute syntax](https://comark.dev/syntax/attributes)
* [Comark AST specification](https://comark.dev/syntax/comark-ast)
* Related: [OMEP-0001](OMEP-0001-project-architecture.md) (project
  architecture), [OMEP-0006](OMEP-0006-markdown-parser.md) (rushdown parser)
