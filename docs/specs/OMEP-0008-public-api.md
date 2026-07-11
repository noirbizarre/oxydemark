---
status: accepted
date: 2026-07-11
decision-makers: [Axel H.]
---

# OMEP-0008: Public API Stability & Versioning Policy

## Context and Problem Statement

OxydeMark is consumed on two fronts: as a Rust `rlib` crate (via the `rlib`
crate type declared in `Cargo.toml`) and as a Python package (built with
maturin, native module `oxydemark._core`). Downstream projects -- notably
OxydePress -- will depend on both surfaces.

Today, neither surface is explicitly documented as *the* stable API. The crate
root (`src/lib.rs`) does not re-export a curated public surface, there is no
`py.typed` marker or type stubs shipped with the Python package, and there is
no written semver policy. OxydeMark is pre-1.0 (`0.1.0` in both `Cargo.toml`
and `pyproject.toml`).

Without an explicit contract, downstream consumers cannot tell which items are
supported versus incidental, and we cannot make internal refactors with
confidence about what we are allowed to break. We need to (1) freeze the
intended public Rust and Python surfaces, (2) decide a stub/typing strategy for
Python, and (3) define a semantic-versioning policy appropriate for the 0.x
phase together with the criteria for cutting 1.0.

## Decision Drivers

* **Explicit contract** -- Consumers must know exactly which items are stable
  and which are internal implementation details.
* **Two consistent surfaces** -- The Rust and Python APIs should expose the same
  conceptual operations (parse, transform, render) so the two ecosystems stay
  aligned.
* **Typed Python** -- Consumers using type checkers (mypy, pyright) must get
  accurate types for the compiled native module, which cannot be introspected
  from source.
* **Refactor freedom pre-1.0** -- We are still iterating on internals
  (extensions, arena conversion); the policy must allow breaking changes while
  the project is young, but signal them clearly.
* **Predictable path to 1.0** -- Downstream projects need to know what
  stabilisation means and when to expect it.
* **Alignment with existing conventions** -- AGENTS.md already mandates
  Conventional Commits, Python 3.12+, and type hints on all public functions;
  this OMEP formalises the surface those rules apply to.

## Considered Options

* **Option A: No explicit surface (status quo)** -- Treat every public item as
  incidental; document nothing; consumers depend at their own risk.
* **Option B: Freeze surfaces + `py.typed` inline types, semver from 1.0
  onward** -- Declare the public surfaces, ship inline type information, but only
  start honouring semver at 1.0.
* **Option C: Freeze surfaces + `py.typed` inline types + explicit 0.x semver
  policy with 1.0 criteria** -- Declare the frozen Rust and Python surfaces,
  ship a `py.typed` marker with inline type hints (plus a `.pyi` stub only for
  the compiled `_core` module), and define a 0.x versioning contract now.

## Decision Outcome

Chosen option: **Option C**, because it gives downstream consumers an explicit,
typed contract immediately while preserving the freedom to iterate that a
pre-1.0 project needs. It documents *both* surfaces so the Rust and Python
ecosystems stay in lockstep, and it states unambiguously what 1.0 will mean.

### Public Rust surface

The following items constitute the supported Rust surface and are to be
re-exported from the crate root (`oxydemark::*`). Everything else in the crate
(the `extensions`, `html_render`, and `ast` internal helpers, the thread-local
parser/renderer caches, and the arena-conversion functions) is **private** and
may change without notice.

| Item | Kind | Description |
| ---- | ---- | ----------- |
| `parse(markdown: &str) -> PyResult<AstNode>` | function | Parse Markdown into an `AstNode` tree. |
| `render_ast(node: &AstNode) -> String` | function | Render an `AstNode` tree to HTML. |
| `markdown_to_html(markdown: &str) -> PyResult<String>` | function | Convert Markdown to HTML in one pass (fast path). |
| `AstNode` | struct | Tree-based AST node (`kind`, `children`, `text`, `attributes`, `metadata`) with `new()`, `walk()`, and `__repr__()`. |

Notes and constraints on the Rust surface:

* The parser is configured with GFM, YAML frontmatter (`rushdown-meta`), emoji
  (`rushdown-emoji`), and the Comark extensions (OMEP-0007). The *set* of
  enabled extensions is part of the observable behaviour but the concrete
  extension *types* (`BlockComponent`, `InlineComponent`, `SpanAttributes`,
  etc.) are **not** part of the public surface.
* `AstNode.kind` string values (e.g. `"document"`, `"paragraph"`, `"heading"`,
  `"emphasis"`, `"strong"`, `"emoji"`, `"block_component"`,
  `"inline_component"`, `"span_attributes"`, `"softbreak"`, `"hardbreak"`) are
  part of the contract: existing kinds will not be renamed within a minor
  series once 1.0 is reached. New kinds may be added.
* `PyResult` is exposed because the primary consumer is the Python binding; a
  future pure-Rust error type is explicitly out of scope for this OMEP and
  tracked as a follow-up.

### Public Python surface

The public Python surface is everything re-exported from the `oxydemark`
package top level and listed in its `__all__`:

| Item | Source | Description |
| ---- | ------ | ----------- |
| `parse(markdown: str) -> AstNode` | `oxydemark._core` | Parse Markdown into an `AstNode` tree. |
| `render_ast(ast: AstNode) -> str` | `oxydemark._core` | Render an `AstNode` tree to HTML. |
| `markdown_to_html(markdown: str) -> str` | `oxydemark._core` | Convert Markdown to HTML in one pass. |
| `AstNode` | `oxydemark._core` | AST node class: constructor `AstNode(kind, children=None, text=None, attributes=None, metadata=None)`, attributes `kind`/`children`/`text`/`attributes`/`metadata`, method `walk()`. |
| `OxydeEngine` | `oxydemark.api` | Pipeline engine: `OxydeEngine(plugins=None)`, `render(markdown: str) -> str`. |
| `Plugin` | `oxydemark.api` | `Protocol` describing optional `preprocess`/`transform`/`postprocess` hooks. |

Notes and constraints on the Python surface:

* `oxydemark._core` is an **internal** module name (the compiled extension). It
  is not part of the public contract and must not be imported directly by
  consumers; import from `oxydemark` instead.
* The `Plugin` protocol is structural: plugins are duck-typed and need only
  implement the hooks they use. This is a public, stable contract.
* Anything not listed in `oxydemark.__all__` is private.

### Typing / stub strategy

* Ship a **`py.typed`** marker file in `python/oxydemark/` so type checkers
  treat the package as typed (PEP 561).
* The pure-Python module (`api.py`) already carries inline type hints; those are
  the source of truth for `OxydeEngine` and `Plugin`.
* The compiled `_core` module cannot be introspected from source by static type
  checkers, so ship a hand-written **`python/oxydemark/_core.pyi`** stub
  declaring `AstNode`, `parse`, `render_ast`, and `markdown_to_html`. This is
  the single stub file we maintain; the rest of the package relies on inline
  hints.
* Both `py.typed` and `_core.pyi` are packaged by maturin (they live under
  `python-source`).

### Semver policy (0.x and criteria for 1.0)

While OxydeMark is in the `0.x` series, versioning follows the widely-used 0.x
interpretation of Semantic Versioning:

* `0.MINOR.PATCH`.
* A **breaking change** to any public surface (Rust or Python, as frozen above)
  bumps the **MINOR** version (`0.1.z -> 0.2.0`).
* **Additive, backward-compatible** changes and bug fixes bump the **PATCH**
  version (`0.1.0 -> 0.1.1`).
* Both the crate (`Cargo.toml`) and the Python package (`pyproject.toml`) are
  released with the **same version number**, in lockstep.
* Breaking changes must be flagged in commit messages per Conventional Commits
  (`feat!:`/`fix!:` or a `BREAKING CHANGE:` footer) so `git-cliff` surfaces them
  in the changelog (OMEP-0003).
* Changes to *private* items (internal modules, extension types, caches) are not
  breaking changes and do not, on their own, trigger a MINOR bump.

Criteria for cutting **1.0** (all must hold):

1. The Rust and Python surfaces above have been stable across at least two
   consecutive minor releases with no breaking changes.
2. A dedicated, first-class Rust error type replaces the leaked `PyResult` on
   the Rust surface (the follow-up noted above).
3. `AstNode.kind` values and node structure are documented and covered by tests
   asserting the contract.
4. `py.typed` + `_core.pyi` pass a type-check gate in CI (e.g. mypy/pyright).
5. Downstream OxydePress integration has validated the surface in real use.

Once 1.0 is reached, standard SemVer applies: breaking changes bump MAJOR.

### Consequences

* Good, because downstream consumers (OxydePress) get an explicit, typed,
  versioned contract for both the crate and the package.
* Good, because internal modules remain free to change without a breaking-change
  bump, preserving refactor velocity pre-1.0.
* Good, because the Rust and Python surfaces are documented as mirror images,
  keeping the two ecosystems consistent.
* Bad, because a hand-written `_core.pyi` stub must be kept in sync with the
  Rust `#[pymethods]`; drift is possible until an automated check exists.
* Bad, because leaking `PyResult` on the Rust surface is a wart we now owe a 1.0
  follow-up to remove.
* Neutral, because lockstep versioning couples crate and package releases; this
  is simpler to reason about but occasionally forces a no-op bump on one side.

### Confirmation

* A Python test (`tests/test_public_api.py`) asserts that `oxydemark.__all__`
  exactly matches the frozen public surface listed here and that each name is
  importable from the top-level package.
* `ls docs/specs/OMEP-0008-public-api.md` confirms the OMEP exists.
* CI continues to run the Rust and Python suites (`mise run ci`).
* The typing gate (mypy/pyright over the package with its stub) is a 1.0 exit
  criterion tracked separately.

## Pros and Cons of the Options

### Option A: No explicit surface (status quo)

* Good, because zero upfront work.
* Bad, because consumers cannot distinguish stable from incidental API.
* Bad, because it blocks a confident OxydePress integration.
* Bad, because no typing story for the compiled module.

### Option B: Freeze surfaces + `py.typed`, semver only from 1.0

* Good, because it documents and types the surfaces now.
* Bad, because "no semver until 1.0" gives 0.x consumers no signal about
  breaking changes, which is exactly the phase OxydePress will consume.

### Option C: Freeze surfaces + `py.typed` + explicit 0.x semver (Chosen)

* Good, because consumers get a contract *and* a breaking-change signal during
  the 0.x phase.
* Good, because it sets concrete, testable 1.0 criteria.
* Neutral, because it requires maintaining one hand-written stub file.
* Bad, because it commits us to lockstep versioning and a 1.0 follow-up for the
  Rust error type.

## More Information

* [Semantic Versioning 2.0.0](https://semver.org/) -- including the 0.x clause.
* [PEP 561 -- Distributing and Packaging Type Information](https://peps.python.org/pep-0561/).
* [PyO3 User Guide](https://pyo3.rs/) -- class and function exposure.
* Related: [OMEP-0001](OMEP-0001-project-architecture.md) (architecture and the
  Rust/Python split), [OMEP-0003](OMEP-0003-changelog-management.md)
  (Conventional Commits / changelog), [OMEP-0007](OMEP-0007-comark-syntax.md)
  (extended syntax whose node kinds appear on the AST surface).
* Follow-up actions:
  * Add `python/oxydemark/py.typed` and `python/oxydemark/_core.pyi`.
  * Re-export the public items from the crate root in `src/lib.rs`.
  * Introduce a first-class Rust error type before 1.0.
  * Add a type-check gate to CI.
