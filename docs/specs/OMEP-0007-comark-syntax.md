---
status: proposed
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

```markdown
::card
#header
## Card Title

#content
Main content here.
::
```

### Nested Components

```markdown
:::outer
::inner{variant="compact"}
Content
::
:::
```

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

### Phase 3 (future)

* **Component slots** -- `#slot-name` syntax within block components
* **YAML frontmatter props** -- YAML blocks inside components
* **Nested components** -- multi-colon nesting (`:::`, `::::`, etc.)

## More Information

* [Comark syntax documentation](https://comark.dev/syntax/markdown)
* [Comark component syntax](https://comark.dev/syntax/components)
* [Comark attribute syntax](https://comark.dev/syntax/attributes)
* [Comark AST specification](https://comark.dev/syntax/comark-ast)
* Related: [OMEP-0001](OMEP-0001-project-architecture.md) (project
  architecture), [OMEP-0006](OMEP-0006-markdown-parser.md) (rushdown parser)
