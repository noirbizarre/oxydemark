# Comark compliance fixtures

Data-driven test cases for the Comark syntax specified in
[OMEP-0007](../../docs/specs/OMEP-0007-comark-syntax.md). They are the single
source of truth for the Comark behaviour contract and are consumed by **two**
harnesses running the very same files:

| Harness                    | Runner                                                |
| -------------------------- | ----------------------------------------------------- |
| `tests/compliance.rs`      | `cargo nextest run --test compliance`                 |
| `tests/test_compliance.py` | `uv run --group test pytest tests/test_compliance.py` |

Each case asserts the **exact** HTML produced by both render paths — the
rushdown fast path (`markdown_to_html`) and the standalone AST renderer
(`render_ast(parse(...))`) — plus an optional **partial** AST expectation.

## File layout

One file per topic. Both harnesses discover `*.md` and `*.json` automatically
(`README.md` excluded), and two fixtures may not share a stem, since the stem
forms the first half of the test id.

| File            | Covers                                                    |
| --------------- | --------------------------------------------------------- |
| `components.md` | block/inline components, inline `{...}` attributes, spans |
| `slots.md`      | named slots, explicit and implicit default slots          |
| `props.md`      | typed block props (frontmatter and `yaml [props]` fences) |
| `nesting.md`    | multi-colon fences and nested components                  |
| `core.md`       | core Markdown void elements and raw HTML sanitisation     |

## Markdown format

Prefer this format: the Markdown input and the expected HTML are written
verbatim, which keeps multi-line cases readable and reviewable.

````markdown
# Slots

Named slots and the implicit default slot.
Reference: docs/specs/OMEP-0007-comark-syntax.md#slots

## explicit-default-slot-is-wrapped

An explicit `#default` marker behaves like any other named slot.

`````comark
::card
#default
D
::
`````

`````html
<div>
<div data-slot="default">
<p>D</p>
</div>
</div>
`````

`````json ast
{
  "descend": "first:block_component",
  "exact_children": true,
  "children": [{ "kind": "slot", "attributes": { "name": "default" } }]
}
`````
````

Grammar, as implemented by both parsers:

| Construct                | Meaning                                                                              |
| ------------------------ | ------------------------------------------------------------------------------------ |
| `# Title`                | File title, informational                                                              |
| prose before the 1st case | File description, informational                                                       |
| `Reference: …`           | Optional provenance, informational                                                     |
| `## <name>`              | Opens a case; the name must be unique within the file and forms the test id            |
| prose after `## <name>`  | The case description, i.e. *why* the behaviour is what it is                           |
| ` ```comark `            | **Required**, the Markdown input                                                       |
| ` ```html `              | **Required**, the exact expected HTML                                                  |
| ` ```json ast `          | Optional partial node spec, same schema as the JSON format (see below)                 |

- Fences follow CommonMark: an opener is a run of **three or more** backticks
  and closes on a run of at least the same length with an empty info string.
  Use four or more backticks when the case itself contains a fenced block, as
  the `yaml [props]` fixtures do.
- A block body is the enclosed lines, always newline-terminated.
- Any other info string, a fence outside a case, a missing `comark`/`html`
  block or a duplicate block is a hard error naming the file and the case.

## JSON format

Still supported, and the format to reach for when a case must assert an input
*without* a trailing newline, which the fenced form cannot express.

```jsonc
{
  "description": "What this file covers",
  "reference": "docs/specs/OMEP-0007-comark-syntax.md#slots",   // optional
  "cases": [
    {
      "name": "kebab-case-unique-within-file",   // required, used in test ids
      "description": "One line",                  // optional
      "markdown": "…",                            // required, the input
      "html": "…",                                // required, the exact output
      "ast": { /* partial node spec, see below */ }   // optional
    }
  ]
}
```

## Partial node spec

Every key is optional and **a key left out is never asserted**, which keeps
fixtures immune to unrelated, additive AST changes.

| Key                 | Semantics                                                                                                       |
| ------------------- | --------------------------------------------------------------------------------------------------------------- |
| `kind`              | exact match on `AstNode.kind`                                                                                     |
| `text`              | exact match on `AstNode.text`                                                                                     |
| `attributes`        | **subset** match: each listed key must exist with that exact value; extra attributes are ignored                  |
| `absent_attributes` | list of keys that must **not** be present                                                                         |
| `props`             | `null` requires `props` to be unset; an object is a **subset** match using native JSON types                       |
| `children`          | positional **prefix** match against `AstNode.children`                                                            |
| `exact_children`    | `true` additionally requires the child count to match                                                             |
| `descend`           | `"first:<kind>"` re-anchors the match on the first pre-order descendant of that kind                              |

The top-level spec is matched against the document root unless it uses
`descend`. Prefer `descend` to spelling out the `document` → … chain when a case
is about a single node, and use explicit `children` with `exact_children` when
the child *set* is itself the contract (slot ordering, nesting depth).

## Adding a case

1. Pick the topical file, or add a new `<topic>.md`.
2. Add a `## <name>` heading, a one-line description explaining *why* the
   behaviour is what it is, and the ` ```comark ` input.
3. Derive the expected HTML and **review it**:

   ```sh
   uv run --group test python -c \
     'import oxydemark; print(oxydemark.markdown_to_html("::note\nx\n::"), end="")'
   ```

   Never paste output blindly: the point of the suite is to encode the
   *intended* behaviour. If the current output is wrong, do not add the case —
   open an issue and add it together with the fix.
4. Add a minimal ` ```json ast ` block asserting only what the case is about.
5. Run both harnesses:

   ```sh
   mise run test
   mise run test:python
   ```

## Gotchas

- Run the **Rust** harness first: it deserializes the node specs with
  `deny_unknown_fields`, so it is what catches schema typos. The Python harness
  is permissive by construction.
- The `html` block includes the trailing newline the renderer emits, which the
  fenced form gives for free.
- A ` ```comark ` block always ends with a newline. That is inert for every
  current case; use the JSON format if a case ever depends on its absence.
- rushdown splits inline text across several `text` nodes (`"Main content"` +
  `" here."`), so avoid `text` assertions on inline leaves.
- A `---` line at the very *start* of a document is claimed by the frontmatter
  parser and can never be a thematic break. Use `***`, or put content before
  the `---`.
- Raw HTML in the source is replaced by a `<!-- raw HTML omitted -->`
  placeholder, both in the HTML and in the `text` of the resulting `raw_html` /
  `html_block` nodes.
- The fixtures are deliberately excluded from the published crate tarball and
  the PyPI sdist (see the `include` allow-list in `Cargo.toml`); the Rust
  harness skips itself when the directory is absent.
