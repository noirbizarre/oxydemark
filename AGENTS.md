# AGENTS.md -- OxydeMark

This file provides context for AI coding agents working on the OxydeMark
codebase.

## Project Overview

OxydeMark is an extensible Markdown processing engine built around an AST
pipeline architecture. It has a high-performance Rust core with Python bindings
via PyO3/maturin.

**Architecture flow:**

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

## Repository Layout

```
oxydemark/
├── src/                    # Rust core
│   ├── lib.rs              # PyO3 module, parser/renderer wiring
│   ├── ast.rs              # AstNode definition, arena-to-tree conversion
│   ├── extensions.rs       # Comark parser/renderer extensions
│   └── html_render.rs      # AST-to-HTML renderer
├── python/oxydemark/       # Python package
│   ├── __init__.py         # Re-exports from native module
│   └── api.py              # OxydeEngine class, plugin protocol
├── docs/                   # Documentation site sources (zensical.toml)
│   ├── index.md            # Landing page
│   ├── plugins.md          # Plugin authoring guide
│   ├── api/                # API reference pages (mkdocstrings + rustdoc)
│   └── specs/              # OMEPs (design decisions, MADR format)
├── .github/workflows/      # CI + release pipelines
├── .github/ship.yml        # gh-ship release orchestration config
├── Cargo.toml              # Rust crate configuration
├── pyproject.toml           # Python build config (maturin backend)
├── mise.toml               # Task runner and tool versions
├── zensical.toml           # Documentation site configuration
├── cliff.toml              # Changelog + version bumping (git-cliff)
├── typos.toml              # Spell-checker config (crate-ci/typos)
├── prek.toml               # Pre-commit hooks
├── CONTRIBUTING.md         # Contribution guidelines
└── LICENSE                 # MIT License
```

## Key Technologies

| Component      | Technology                     |
| -------------- | ------------------------------ |
| Core language  | Rust (stable, edition 2024)    |
| Python binding | PyO3 0.28 + maturin           |
| Task runner    | mise                           |
| Pre-commit     | prek                           |
| Changelog      | git-cliff (Conventional Commits) |
| Testing        | cargo-nextest, cargo-llvm-cov  |
| CI             | GitHub Actions                 |

## Build & Test Commands

All tasks are defined in `mise.toml`. Use `mise run <task>`:

```sh
mise run build          # cargo build
mise run test           # cargo nextest run (Rust, pure-Rust, no python feature)
mise run test:features  # cargo nextest run --features python (Rust + PyO3 layer)
mise run test:python    # pytest (Python only)
mise run test:all       # Rust (both configs) and Python test suites
mise run lint           # cargo clippy -- -D warnings
mise run fmt            # cargo fmt
mise run fmt:check      # cargo fmt -- --check
mise run ci             # runs fmt:check + lint + test
mise run changelog      # generate CHANGELOG.md for the next version
mise run changelog:preview  # preview the next version's release notes
mise run typos          # spell-check sources, docs and commit messages
mise run release:preview    # dry-run the release preparation (gh ship preview)
mise run cover          # cargo llvm-cov
mise run bench          # Python benchmarks comparing Markdown libraries
mise run docs           # Build the docs site (zensical) + rustdoc into site/
mise run docs:serve     # Preview the docs site locally
mise run setup          # install pre-commit hooks via prek
```

## Coding Conventions

### Rust
- Edition 2024, stable toolchain.
- Format with `rustfmt` (default config).
- All clippy warnings are errors (`-D warnings`).
- Public items must have doc comments (`///`).
- Prefer `Result<T, E>` over panicking.

### Python
- Minimum version: Python 3.12.
- Type hints on all public functions and methods.
- Follow PEP 8 conventions.

### Commits
- Use [Conventional Commits](https://www.conventionalcommits.org/).
- Format: `<type>[optional scope]: <description>`
- Valid types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`,
  `build`, `ci`, `chore`.

## Design Decisions

Architectural and tooling decisions are documented as OMEPs (OxydeMark
Enhancement Proposals) in `docs/specs/`. Read these before making significant
changes to understand the rationale behind current design choices.

## Important Notes for Agents

- The `oxydemark._core` Python module is the compiled Rust extension. It only
  exists after building with `maturin develop`. LSP errors about this import
  are expected in a fresh checkout.
- PyO3 is optional and gated behind cargo features (OMEP-0008): the `python`
  feature builds the binding layer (used by `cargo test --features python`),
  and `extension-module` (maturin) builds the wheel. A plain `cargo build`,
  `cargo nextest run`, and downstream `oxydemark` crate deps are PyO3-free. The
  public Rust API (`parse`, `render_ast`, `markdown_to_html`, `parse_document`,
  `slugify`, `extract_summary`, `AstNode`, `ParseResult`, `OxydeError`) lives in
  `src/api.rs`/`src/ast.rs`/`src/error.rs` and is re-exported from `src/lib.rs`.
- Do not edit `CHANGELOG.md` by hand; it is generated by `git cliff`. Do not
  bump versions in `Cargo.toml`/`pyproject.toml` by hand either, and never
  create a release tag: `.github/workflows/prepare-release.yml` does the
  lockstep bump and `gh ship release` creates the tag (OMEP-0009). Releases
  follow a Release-PR model orchestrated by
  [gh-ship](https://github.com/noirbizarre/gh-ship); `.github/workflows/release.yml`
  must keep that exact filename because the crates.io and PyPI Trusted
  Publishers are bound to it.
- `cliff.toml` shells out to `typos` as a commit preprocessor, so git-cliff
  fails without that binary on PATH. It is provided by mise.
- Python docstrings use **Google style** and must not repeat types (OMEP-0011):
  types are declared once in annotations and in `python/oxydemark/_core.pyi`.
  The stub is what the API reference renders, since the mkdocstrings handler
  runs with `allow_inspection = false`.
- Pre-commit hooks validate formatting, linting, and commit message conventions
  automatically.
- When proposing new features or architectural changes, create an OMEP in
  `docs/specs/` following the MADR template.
