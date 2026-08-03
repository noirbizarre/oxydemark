---
status: accepted
date: 2026-08-03
decision-makers: [Axel H.]
---

# OMEP-0012: Code Coverage Reporting

## Context and Problem Statement

OxydeMark is a dual-language project: a Rust core (`src/`) and a Python package
(`python/oxydemark/`) that adds the plugin pipeline and the contrib plugins
(OMEP-0001). Both are shipped to users, both are tested in CI -- but neither is
measured. `mise run cover` existed as a bare `cargo llvm-cov`, producing a
terminal summary nobody looked at, and CI produced nothing at all.

Two consequences follow. First, a pull request can delete tests, or add an
untested branch, without anything saying so. Second, and more insidious for a
pipeline architecture, "the project is at 85%" is not actionable: it hides the
fact that the HTML renderer might be at 40% while the AST is at 95%. The
interesting signal is per-layer, not per-repository.

A third, project-specific wrinkle: the PyO3 binding layer (`src/python.rs`) is
behind a cargo feature (OMEP-0008). A default-feature coverage run does not
compile it, so a naive setup would report the bindings as 0% covered -- or,
depending on the tool, omit them entirely and silently inflate the total.

## Decision Drivers

* **Per-layer attribution** -- Coverage must be reported per architectural
  layer, so a regression points at the parser, the renderer or the contrib
  plugins rather than at "the repository".
* **Both languages** -- The Python package is a first-class deliverable, not a
  thin wrapper; leaving it unmeasured would report a number that describes half
  the product.
* **Feature-complete Rust measurement** -- The binding layer must be measured,
  which means merging the two feature configurations that CI already runs.
* **No false drops** -- Several reports land per commit; the tooling must not
  judge a commit against a partially uploaded report.
* **Minimal maintenance** -- Prefer a hosted service with a config file over a
  self-hosted dashboard or hand-rolled artifact publishing.
* **Local parity** -- A contributor must be able to reproduce the CI numbers
  with a `mise` task (OMEP-0002).

## Considered Options

* **Option A: Codecov** -- Hosted coverage service, config in
  `.github/codecov.yml`, native flags and components.
* **Option B: Coveralls** -- Hosted coverage service with a comparable GitHub
  integration.
* **Option C: Artifact-only HTML reports** -- Generate `llvm-cov`/`coverage.py`
  HTML in CI and upload it as a workflow artifact, no external service.

## Decision Outcome

Chosen option: **"Codecov"** (Option A), because it is the only one of the three
that natively models both dimensions this project needs: **flags** to keep the
two language reports distinguishable, and **components** to attribute coverage
to an architectural layer without splitting the repository into crates.

The setup is:

* **Rust** -- `cargo llvm-cov nextest --no-report` is run twice, once with the
  default features and once with `--features python`, then a single
  `cargo llvm-cov report --lcov` merges both profiles. This is what makes
  `src/python.rs` measurable.
* **Python** -- `pytest --cov` via `pytest-cov`, with a `[tool.coverage.paths]`
  remap so measurements recorded against the installed package in
  `site-packages` are reported as `python/oxydemark/...`, matching the paths the
  components declare.
* **Flags** -- `rust` (scoped to `src/`) and `python` (scoped to `python/`),
  both with `carryforward: false`: every leg runs on every commit, so a missing
  report means a broken run, not an untested language, and carrying a stale one
  forward would hide it.
* **Components** -- seven, mirroring the pipeline of OMEP-0001: `core`, `ast`,
  `extensions`, `renderer`, `bindings`, `python-api`, `python-contrib`.
* **Uploads** -- one leg per language: Rust `stable` and Python `3.12`. The
  other matrix legs (Rust `nightly`, Python `3.13`/`3.14`) still run the full
  suite -- they prove the code compiles and passes there -- but they exercise
  the same sources and would produce a report identical to the uploading leg.
  Uploading them would only add latency and inflate the `after_n_builds`
  bookkeeping below.
* **Test results** -- both legs also upload their JUnit report
  (`.config/nextest.toml` for Rust, `--junitxml` for Python), so failing tests
  are annotated in the pull request diff.

### Consequences

* Good, because a regression is attributed to a layer, which is the granularity
  at which the code is actually reviewed.
* Good, because the binding layer is measured rather than reported as a
  permanent 0%.
* Good, because the patch status (80%, no threshold) makes untested new code a
  blocking, visible fact on the pull request.
* Good, because uploads are tokenless on a public repository -- no secret to
  provision or rotate.
* Bad, because it adds a third-party dependency to the pull request status
  checks: a Codecov outage shows up as pending checks.
* Bad, because the Rust job now compiles with instrumentation and runs the
  suite through `llvm-cov`, which is measurably slower than a plain
  `cargo nextest run`.
* Neutral, because `codecov.notify.after_n_builds: 2` delays the status until
  both languages have reported. Adding a third upload later means bumping that
  number, or the first report will be judged against a complete base and show a
  drop that did not happen.

### Confirmation

* `mise run cover` produces `lcov.info` containing an `SF:src/python.rs` entry,
  proving the two feature configurations were merged.
* `mise run cover:python` produces a `coverage.xml` whose filenames are
  source-tree relative (`python/oxydemark/...`), proving the path remap works.
* `mise run cover:all` reproduces both CI reports locally.
* Every pull request carries the project, patch and per-component Codecov
  statuses, and the badge in `README.md` reflects `main`.

## Pros and Cons of the Options

### Option A: Codecov (Chosen)

* Good, because flags and components are first-class, configured declaratively
  in `.github/codecov.yml`.
* Good, because it ingests LCOV and Cobertura, so the Rust and Python tools
  need no adapter.
* Good, because it also ingests JUnit test results, replacing a separate
  reporting action.
* Good, because tokenless upload for public repositories.
* Bad, because it is an external service in the critical path of merging.

### Option B: Coveralls

* Good, because a simple, long-lived GitHub integration.
* Good, because it supports parallel builds with a finish webhook.
* Bad, because it has no equivalent of components: per-layer attribution would
  have to be faked by splitting the upload into several "repos" or reading
  directory listings by hand.
* Bad, because its multi-language merging is less ergonomic than flags.

### Option C: Artifact-only HTML reports

* Good, because no external service and no account.
* Good, because the full HTML report is browsable per run.
* Bad, because there is no trend, no base comparison, and therefore no way to
  fail a pull request on a coverage drop.
* Bad, because nobody downloads a CI artifact to review a diff; the signal
  would exist but never be read.

## More Information

**Configuration files:**

| File | Role |
| ---- | ---- |
| `.github/codecov.yml` | Flags, components, project/patch statuses |
| `.config/nextest.toml` | `ci` profile with JUnit output |
| `pyproject.toml` | `pytest-cov` dependency and `[tool.coverage.*]` config |
| `mise.toml` | `cover`, `cover:python`, `cover:all` tasks |

**Follow-up:** `src/extensions.rs` and `src/html_render.rs` have no inline
`#[cfg(test)]` modules and are covered only indirectly, through
`tests/compliance.rs` and the Python suite. Their component percentages are
expected to be the first to expose gaps, and are the natural place to start
adding unit tests.

See: [Codecov components](https://docs.codecov.com/docs/components),
[cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov).
