---
status: accepted
date: 2026-03-21
decision-makers: [Axel H.]
---

# OMEP-0001: Project Architecture

## Context and Problem Statement

OxydeMark aims to be an extensible Markdown processing engine. We need to
decide on the overall architecture: which language(s) to use, how to structure
the codebase, and how the processing pipeline should work.

The key tension is between **performance** (favoring a systems language) and
**extensibility / developer experience** (favoring a dynamic language with a
rich ecosystem).

## Decision Drivers

* **Performance** -- Markdown parsing and rendering must be fast, even for
  large documents.
* **Extensibility** -- Users must be able to write plugins without learning
  Rust.
* **Pipeline architecture** -- Processing should flow through well-defined
  stages with hooks at each boundary.
* **Separation of concerns** -- Parsing, transformation, and rendering should
  be independent modules.
* **AST as the central abstraction** -- All transformations operate on a
  shared AST, making the system composable and inspectable.

## Considered Options

* **Option A: Pure Rust** -- Entire stack in Rust, plugins via trait objects or
  WASM.
* **Option B: Pure Python** -- Use an existing Python Markdown library
  (e.g. `markdown-it-py`) and extend it.
* **Option C: Rust core with Python bindings via PyO3** -- High-performance
  Rust engine exposed to Python through a native extension module.

## Decision Outcome

Chosen option: **"Rust core with Python bindings via PyO3"** (Option C),
because it delivers Rust-level performance for the hot path (parsing and
rendering) while giving plugin authors the full Python ecosystem for
preprocessing and postprocessing.

### Consequences

* Good, because the Rust core can be optimized independently of the Python
  layer.
* Good, because Python users get a familiar API (`OxydeEngine`, plugin
  protocol) without needing to know Rust.
* Good, because the architecture naturally separates concerns: Rust owns
  parsing/rendering, Python owns orchestration/plugins.
* Bad, because contributors need *some* familiarity with both Rust and Python.
* Bad, because the build process is more complex (maturin, cross-compilation
  for wheels).

### Confirmation

* `cargo test` validates the Rust core.
* A Python smoke test (`python -c "import oxydemark; ..."`) confirms the
  binding works after each wheel build.
* CI runs both Rust and Python checks.

## Pros and Cons of the Options

### Option A: Pure Rust

* Good, because single-language stack simplifies the build.
* Good, because maximum possible performance.
* Bad, because Rust plugin authoring has a steep learning curve for most users.
* Bad, because the Rust ecosystem for dynamic plugin loading (WASM, dylib) is
  still maturing.

### Option B: Pure Python

* Good, because lowest barrier to entry for contributors.
* Good, because rich ecosystem of existing Markdown libraries.
* Bad, because Python is orders of magnitude slower for parsing large
  documents.
* Bad, because difficult to achieve the performance goals.

### Option C: Rust Core with Python Bindings (Chosen)

* Good, because parsing/rendering performance is near-native.
* Good, because Python plugin system is easy to use and well-understood.
* Good, because PyO3 + maturin is a mature, well-supported approach.
* Neutral, because two-language builds add CI complexity, but maturin abstracts
  most of it.
* Bad, because debugging across the FFI boundary can be challenging.

## More Information

**Pipeline architecture:**

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

The pipeline supports three plugin hook points: text-level preprocessing
before parsing, AST-level transformation between parsing and rendering, and
HTML-level postprocessing after rendering.

**Project structure:**

```
src/                    # Rust core
├── lib.rs              # Crate root, public re-exports
├── api.rs              # Public API, rushdown integration
├── ast.rs              # AstNode and arena-to-tree conversion
└── python.rs           # PyO3 module (`python` feature)
python/oxydemark/       # Python package
├── __init__.py         # Re-exports from _core
└── api.py              # OxydeEngine, plugin protocols
Cargo.toml              # Rust crate config (cdylib + rlib)
pyproject.toml          # maturin build backend
```

**Key dependencies:**

| Dependency | Role |
| ---------- | ---- |
| PyO3 0.28 | Rust ↔ Python FFI bindings |
| maturin | Build backend for mixed Rust/Python packages |
| rushdown 0.18 | Markdown parser and HTML renderer (CommonMark + GFM) |
| rushdown-meta =0.9.9 | YAML frontmatter extension |
| rushdown-emoji =0.9.8 | Emoji shortcode extension |

See also: [PyO3 User Guide](https://pyo3.rs/),
[maturin docs](https://www.maturin.rs/),
[OMEP-0006](OMEP-0006-markdown-parser.md) (parser choice),
[OMEP-0007](OMEP-0007-comark-syntax.md) (extended syntax).

!!! note "Layout since OMEP-0008"

    The original sketch put the whole Rust core in `src/lib.rs`. Feature-gating
    the bindings (OMEP-0008) split it: `lib.rs` is now only the crate root and
    its public re-exports, and the PyO3 layer lives in `src/python.rs` behind
    the optional `python` feature. The tree above reflects the current layout.
