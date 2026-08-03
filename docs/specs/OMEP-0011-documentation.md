---
status: accepted
date: 2026-08-01
decision-makers: [Axel H.]
---

# OMEP-0011: Documentation Site & API Reference

## Context and Problem Statement

OMEP-0008 froze the public Rust and Python surfaces and mandated `py.typed` +
`_core.pyi` stubs, but no API reference is generated from them. The project
ships a growing body of hand-written Markdown (`docs/plugins.md`, the OMEP
series) that is only readable on GitHub, and the docstrings that back the
frozen Python surface are not published anywhere.

Two distinct audiences need documentation:

- **Python consumers**, who need `oxydemark.parse`, `OxydeEngine`, the `Plugin`
  protocol and the AST types documented with signatures and types.
- **Rust consumers** (notably OxydePress, which links the `rlib`), who need the
  crate-root surface documented -- a job rustdoc already does well.

We therefore need a static site generator that renders the existing Markdown,
plus a Python API reference generator, plus a home for rustdoc output.

## Decision Drivers

* The existing documentation is Markdown; a Markdown-native generator avoids a
  rewrite.
* Docstrings for the native `oxydemark._core` module live in the `.pyi` stub,
  not in the compiled module, so the generator must analyse sources statically.
* The repository already standardises on `uv` + PEP 735 dependency groups and
  on `mise` for task running; the docs toolchain must fit that.
* The project's long-term intent is to *dogfood* itself: once OxydeMark and
  OxydePress are ready, they should render this very site.
* Rust and Python references should be published as a single site.

## Considered Options

* **Option A** -- Zensical + mkdocstrings (Python handler), plus rustdoc.
* **Option B** -- Sphinx + autodoc/napoleon (with MyST for Markdown), plus
  rustdoc.
* **Option C** -- pdoc, plus rustdoc.
* **Option D** -- rustdoc only, no Python reference.

## Decision Outcome

Chosen option: **Option A -- Zensical + mkdocstrings**, because it is
Markdown-native (the existing `docs/` tree becomes the site with no rewrite),
its mkdocstrings integration can be driven by griffe's *static* analysis, which
reads `python/oxydemark/_core.pyi` and therefore surfaces the stub docstrings
that are already the OMEP-0008 source of truth, and it is configured in TOML
alongside the rest of the repository's tooling.

Concretely, the Python handler is configured with `allow_inspection = false`.
By default griffe *inspects* the compiled `_core` extension when it is present
in the source tree, which yields only the one-line PyO3 doc comments and loses
every `Args:`/`Returns:` section. Disabling inspection forces static analysis of
the stub and makes the build independent of whether the extension has been
compiled.

Rustdoc output (`cargo doc --no-deps`) is copied into the built site under
`/rust/`, so a single GitHub Pages artifact serves both references.

This decision also mandates **Google-style docstrings** across the Python
package, with **no type repetition**: types are declared once in annotations and
stubs, and `Args:`/`Returns:`/`Attributes:` sections carry names and prose only.
The pre-existing reST/NumPy-flavoured docstrings were converted accordingly.

### Consequences

* Good, because `docs/plugins.md` and the OMEP series become first-class pages
  without conversion work.
* Good, because the `.pyi` stub is documented directly, keeping a single source
  of truth for signatures, types and prose.
* Good, because Zensical shares Material for MkDocs' philosophy and authoring
  extensions, which the team already knows.
* Good, because Zensical is a natural stepping stone towards self-hosting: when
  OxydeMark/OxydePress can render the site, the content needs no migration.
* Bad, because Zensical's mkdocstrings support is explicitly *preliminary*;
  notably backlinks are not yet supported.
* Bad, because Zensical is pre-1.0 and its configuration surface may still move.
* Bad, because docstring drift between `_core.pyi` and the PyO3 doc comments in
  `src/ast.rs` / `src/python.rs` is still possible; the OMEP-0008 sync
  requirement stands.

### Confirmation

`mise run docs` builds the Zensical site and the rustdoc output into `site/`,
and the CI `docs` job builds both on every pull request. A pytest guard asserts
that every name in `oxydemark.__all__` carries a non-empty docstring, so the
reference cannot silently lose content.

Publication is handled by a separate workflow, `.github/workflows/docs.yml`,
which builds the same targets on push to `main` and deploys them to the
`github-pages` environment at <https://noirbizarre.github.io/oxydemark/>.

## Pros and Cons of the Options

### Option A -- Zensical + mkdocstrings

* Good, because Markdown-native, so existing content is reused as-is.
* Good, because griffe reads `.pyi` stubs statically, no compiled module needed.
* Good, because TOML configuration matches the rest of the tooling.
* Good, because it is the successor to Material for MkDocs, with an active
  roadmap for API documentation.
* Bad, because mkdocstrings support is preliminary and the tool is pre-1.0.

### Option B -- Sphinx + autodoc

* Good, because it is the most mature Python documentation toolchain, and it
  handles the pre-existing reST docstrings natively.
* Good, because intersphinx interop is excellent.
* Bad, because all existing Markdown would need MyST, and reST authoring is a
  poor fit for a *Markdown engine's* own documentation.
* Bad, because autodoc imports modules at build time, which pushes docstrings
  towards the PyO3 layer rather than the stub.

### Option C -- pdoc

* Good, because zero configuration.
* Bad, because it uses runtime introspection, so it would read the compiled
  `_core` module and miss the stub docstrings entirely.
* Bad, because it cannot host the hand-written guides and OMEPs.

### Option D -- rustdoc only

* Good, because it is nearly free.
* Bad, because it leaves the Python surface -- the primary consumer API --
  undocumented, which fails the OMEP-0008 1.0 criteria.

## More Information

* Zensical: <https://zensical.org>
* mkdocstrings Python handler: <https://mkdocstrings.github.io/python/usage/>
* OMEP-0008 (public API stability) defines the surface documented here.
* Follow-up: revisit once OxydeMark + OxydePress can render the site themselves,
  and once Zensical's API documentation rework lands.
