# Comark compliance fixtures

Data-driven test cases for the Comark syntax specified in
[OMEP-0007](../../docs/specs/OMEP-0007-comark-syntax.md). They are the single
source of truth for the Comark behaviour contract and are consumed by **two**
harnesses running the very same files:

| Harness                   | Runner                                             |
| ------------------------- | -------------------------------------------------- |
| `tests/compliance.rs`     | `cargo nextest run --test compliance`               |
| `tests/test_compliance.py`| `uv run --group test pytest tests/test_compliance.py` |

Each case asserts the **exact** HTML produced by both render paths — the
rushdown fast path (`markdown_to_html`) and the standalone AST renderer
(`render_ast(parse(...))`) — plus an optional **partial** AST expectation.

## File layout

One JSON file per topic, named `<topic>.json`:

| File              | Covers                                                      |
| ----------------- | ----------------------------------------------------------- |
| `components.json` | block/inline components, inline `{...}` attributes, spans   |
| `slots.json`      | named slots, explicit and implicit default slots             |
| `props.json`      | typed block props (frontmatter and `yaml [props]` fences)    |
| `nesting.json`    | multi-colon fences and nested components                     |

## Schema

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

### Partial node spec

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

The top-level `ast` object is matched against the document root unless it uses
`descend`. Prefer `descend` to spelling out the `document` → … chain when a case
is about a single node, and use explicit `children` with `exact_children` when
the child *set* is itself the contract (slot ordering, nesting depth).

## Adding a case

1. Pick the topical file, or add a new `<topic>.json` — both harnesses discover
   `*.json` automatically.
2. Write `name`, an optional `description` explaining *why* the behaviour is
   what it is, and the `markdown` input.
3. Derive the expected HTML and **review it**:

   ```sh
   uv run --group test python -c \
     'import oxydemark; print(repr(oxydemark.markdown_to_html("::note\nx\n::")))'
   ```

   Never paste output blindly: the point of the suite is to encode the
   *intended* behaviour. If the current output is wrong, do not add the case —
   open an issue and add it together with the fix.
4. Add a minimal `ast` block asserting only what the case is about.
5. Run both harnesses:

   ```sh
   mise run test
   mise run test:python
   ```

## Gotchas

- Run the **Rust** harness first: it deserializes the fixtures with
  `deny_unknown_fields`, so it is what catches schema typos. The Python harness
  is permissive by construction.
- `html` values include the trailing newline.
- rushdown splits inline text across several `text` nodes (`"Main content"` +
  `" here."`), so avoid `text` assertions on inline leaves.
- Multi-line `markdown` and fenced props are written as `\n`-joined JSON
  strings; quotes and backslashes must be escaped.
- The fixtures are deliberately excluded from the published crate tarball and
  the PyPI sdist (see the `include` allow-list in `Cargo.toml`); the Rust
  harness skips itself when the directory is absent.

## Known gaps

- A component body consisting of *only* a YAML props block leaks its closing
  `::` fence as literal text, so all props fixtures declare body content. The
  case will be added together with the fix.
