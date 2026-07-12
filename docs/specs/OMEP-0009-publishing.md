---
status: proposed
date: 2026-07-12
decision-makers: [Axel H.]
---

# OMEP-0009: Publishing & Distribution (crates.io + PyPI)

## Context and Problem Statement

OxydeMark is unpublished. Both of its consumable surfaces (the Rust `rlib`
crate and the Python package built by maturin, per OMEP-0008) exist only in the
repository. CI already builds wheels for a smoke-test (OMEP-0005), but there is
no defined path from a merged commit to artifacts that downstream projects --
notably OxydePress -- can `cargo add` or `pip install`.

We need a release pipeline that publishes the same version, in lockstep, to
both [crates.io](https://crates.io/) and [PyPI](https://pypi.org/), covering:

1. The **crates.io publish** flow for the `oxydemark` crate.
2. The **PyPI wheel/sdist matrix** -- which platforms and Python versions we
   build binary wheels for, plus a source distribution.
3. **Authentication** -- how the release job authenticates with each index.
4. **Versioning and release triggering** -- how a version is chosen, tagged,
   and turned into a GitHub Release, consistent with the git-cliff changelog
   (OMEP-0003) and the 0.x semver policy (OMEP-0008).
5. Whether v0.2.0 ships a **CLI binary** or remains library-only.

This OMEP defines that pipeline. It builds directly on the versioning contract
frozen in OMEP-0008 (lockstep `0.MINOR.PATCH`, both manifests share one
version) and the CI foundations in OMEP-0005.

## Decision Drivers

* **Two indexes, one version** -- crates.io and PyPI must always receive the
  same version at the same time; a release is atomic across ecosystems
  (OMEP-0008 lockstep versioning).
* **Broad wheel coverage** -- Python consumers should not need a Rust toolchain;
  binary wheels must cover the common platforms and supported Python versions
  (3.12+, per AGENTS.md).
* **Secretless authentication** -- Prefer OIDC-based Trusted Publishing over
  long-lived API tokens stored as repository secrets, to reduce credential
  leakage risk.
* **Reproducible with existing tooling** -- Reuse maturin, GitHub Actions, and
  git-cliff already adopted in prior OMEPs rather than adding new tools.
* **Tag-driven and auditable** -- Releases are triggered by a version tag and
  produce a GitHub Release whose notes match the changelog, so every published
  artifact is traceable to a commit.
* **Minimal scope for v0.2.0** -- The first release should ship the surfaces we
  already commit to (OMEP-0008) without expanding the maintenance burden.

## Considered Options

### Publishing trigger & versioning

* **Option T1: Manual publish** -- Maintainer runs `cargo publish` and
  `maturin publish` locally, ad hoc.
* **Option T2: Tag-triggered release workflow** -- Pushing a `vX.Y.Z` tag runs
  a GitHub Actions workflow that builds, publishes to both indexes, and creates
  the GitHub Release.

### Authentication

* **Option A1: API tokens as GitHub secrets** -- Store a crates.io token and a
  PyPI token as repository secrets.
* **Option A2: Trusted Publishing (OIDC)** -- Use PyPI Trusted Publishers and
  crates.io Trusted Publishing so the workflow mints short-lived credentials
  via GitHub's OIDC identity; no secrets stored.

### Wheel matrix

* **Option W1: Linux-only wheel + sdist** -- Ship one manylinux wheel plus an
  sdist; other platforms build from source.
* **Option W2: Full platform/Python matrix** -- Build wheels for Linux
  (x86_64, aarch64), macOS (x86_64, arm64), and Windows (x86_64) across
  Python 3.12 and 3.13, plus an sdist.

### CLI in v0.2.0

* **Option C1: Library-only** -- Ship the crate and the Python package; no
  executable.
* **Option C2: Ship an `oxydemark` CLI binary** -- Add a `[[bin]]`/console
  script exposing a command-line converter in v0.2.0.

## Decision Outcome

Chosen options: **T2 (tag-triggered workflow)**, **A2 (Trusted Publishing)**,
**W2 (full platform/Python matrix + sdist)**, and **C1 (library-only for
v0.2.0)**.

Together these define a single `release` workflow, triggered by a `vX.Y.Z` tag,
that authenticates via OIDC, builds the full wheel matrix plus an sdist and the
crate, publishes them, and cuts a GitHub Release whose body is the git-cliff
changelog section for that version. v0.2.0 remains library-only; a CLI is
deferred.

### Release trigger and versioning

* Releases are cut by pushing an annotated tag of the form `vX.Y.Z` (e.g.
  `v0.2.0`) to `main`. The tag is the single trigger for the `release`
  workflow.
* Before tagging, the maintainer bumps the version **in lockstep** in both
  `Cargo.toml` and `pyproject.toml` to the same `X.Y.Z` (OMEP-0008). The
  workflow verifies that the two manifest versions and the tag all agree and
  fails fast otherwise.
* Version numbers follow the 0.x policy from OMEP-0008: a breaking change to a
  public surface bumps MINOR (`0.1.z -> 0.2.0`); additive/fix changes bump
  PATCH.
* The GitHub Release notes are generated with `git cliff --tag vX.Y.Z` so the
  release body matches `CHANGELOG.md` (OMEP-0003). The changelog is not edited
  by hand.

### crates.io publish flow

* The crate is published with `cargo publish` from a dedicated job.
* Because the crate declares `crate-type = ["cdylib", "rlib"]` for the PyO3
  build, publishing to crates.io targets the `rlib` consumers; the `cdylib`
  output is irrelevant to `cargo add` users and is simply ignored by them.
* `cargo publish --dry-run` (packaging + verification build) also runs in CI on
  pull requests so packaging problems surface before release.
* Authentication uses crates.io **Trusted Publishing** (GitHub OIDC): the
  workflow exchanges its OIDC token for a short-lived crates.io token; no
  `CARGO_REGISTRY_TOKEN` secret is stored.

### PyPI wheel/sdist matrix

Built with maturin (via `PyO3/maturin-action`) and published to PyPI.

| Platform | Architectures | Python |
| -------- | ------------- | ------ |
| Linux (manylinux) | `x86_64`, `aarch64` | 3.12, 3.13 |
| macOS | `x86_64`, `arm64` (or a universal2 wheel) | 3.12, 3.13 |
| Windows | `x86_64` | 3.12, 3.13 |

* A **source distribution (sdist)** is also built (`maturin sdist`) so that
  platforms outside the matrix can build from source given a Rust toolchain.
* Where practical, `abi3` (the stable Python ABI, `abi3-py312`) is used to
  emit a single wheel per platform that covers 3.12+, shrinking the matrix; the
  logical support target remains Python 3.12 and 3.13.
* All wheels and the sdist are collected as workflow artifacts, then published
  in a single final job with `maturin upload` / the maturin-action publish
  step.
* Authentication uses PyPI **Trusted Publishing** (OIDC via
  `pypa/gh-action-pypi-publish` or maturin's OIDC support); no PyPI API token
  secret is stored.

### Trusted Publishing

* Both indexes are configured with the GitHub repository, the `release`
  workflow filename, and (optionally) a dedicated `release` GitHub Environment
  as the Trusted Publisher.
* The `release` workflow requests `permissions: id-token: write` so GitHub mints
  the OIDC token used to authenticate to both crates.io and PyPI.
* Rationale: short-lived, scope-limited credentials eliminate the standing risk
  of leaked long-lived tokens and remove secret-rotation toil.

### CLI in v0.2.0: library-only

v0.2.0 ships **no** CLI binary. OMEP-0008 froze the public surfaces as the
parse/transform/render functions, `AstNode`, `OxydeEngine`, and `Plugin` -- all
library entry points. A CLI would add a new, separately versioned surface
(argument grammar, output flags, exit codes) that we are not ready to commit to
this early. A future OMEP may introduce an `oxydemark` CLI (as a Cargo `[[bin]]`
and/or a Python `[project.scripts]` console entry point) once the library
surface has stabilised.

### Consequences

* Good, because a single tag push produces coherent, traceable releases across
  both ecosystems with release notes that match the changelog.
* Good, because Trusted Publishing removes all long-lived registry secrets from
  the repository.
* Good, because the full wheel matrix means Python consumers never need a Rust
  toolchain on the common platforms.
* Good, because reusing maturin, GitHub Actions, and git-cliff avoids new
  tooling.
* Bad, because the wheel matrix (cross-compilation for aarch64, macOS, Windows)
  materially lengthens the release workflow and adds runner cost.
* Bad, because lockstep versioning can force a no-op bump on one ecosystem when
  only the other changed (already accepted in OMEP-0008).
* Neutral, because deferring the CLI keeps scope small now but leaves a
  frequently-requested convenience for later.

### Confirmation

* `ls docs/specs/OMEP-0009-publishing.md` confirms the OMEP exists.
* A `release` workflow (`.github/workflows/release.yml`) exists and is triggered
  by `v*` tags.
* On a pull request, CI runs `cargo publish --dry-run` and `maturin build` so
  packaging regressions are caught before a tag is cut.
* A version-consistency check fails the release if the tag, `Cargo.toml`, and
  `pyproject.toml` versions disagree.
* After the first tagged release, `cargo add oxydemark` and
  `pip install oxydemark` resolve the same `X.Y.Z`, and a matching GitHub
  Release exists with git-cliff-generated notes.

## Pros and Cons of the Options

### Trigger: T1 (manual) vs T2 (tag-triggered)

* T1 -- Good, because zero workflow to maintain. Bad, because error-prone,
  non-reproducible, and easy to publish mismatched versions to the two indexes.
* T2 (Chosen) -- Good, because reproducible, auditable, and atomic across
  ecosystems. Neutral, because it requires an extra workflow file.

### Auth: A1 (tokens) vs A2 (Trusted Publishing)

* A1 -- Good, because simple and universally supported. Bad, because long-lived
  secrets must be stored and rotated and can leak.
* A2 (Chosen) -- Good, because secretless, short-lived, scope-limited
  credentials. Neutral, because it requires one-time Trusted Publisher setup on
  each index.

### Matrix: W1 (Linux + sdist) vs W2 (full matrix + sdist)

* W1 -- Good, because fast and cheap builds. Bad, because macOS/Windows users
  must compile from source (Rust toolchain required), a poor first-run
  experience.
* W2 (Chosen) -- Good, because turnkey installs on all common platforms.
  Bad, because slower, costlier release builds and more moving parts.

### CLI: C1 (library-only) vs C2 (ship CLI)

* C1 (Chosen) -- Good, because keeps the committed surface small and matches
  OMEP-0008. Neutral, because a CLI can be added later without breaking
  anything.
* C2 -- Good, because immediately useful from a shell. Bad, because it adds a
  new versioned interface to design, document, and support before the library
  has settled.

## More Information

* [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing).
* [PyPI Trusted Publishers](https://docs.pypi.org/trusted-publishers/).
* [maturin -- distribution & GitHub Actions](https://www.maturin.rs/distribution).
* [`PyO3/maturin-action`](https://github.com/PyO3/maturin-action).
* [`pypa/gh-action-pypi-publish`](https://github.com/pypa/gh-action-pypi-publish).
* [Cargo -- publishing on crates.io](https://doc.rust-lang.org/cargo/reference/publishing.html).
* Related: [OMEP-0003](OMEP-0003-changelog-management.md) (git-cliff release
  notes), [OMEP-0005](OMEP-0005-ci-cd-pipeline.md) (CI that already builds
  smoke-test wheels), [OMEP-0008](OMEP-0008-public-api.md) (lockstep versioning
  and the frozen surfaces being distributed).
* Follow-up actions:
  * Add `.github/workflows/release.yml` implementing the tag-triggered pipeline.
  * Configure Trusted Publishers on crates.io and PyPI.
  * Add `cargo publish --dry-run` and `maturin build`/`maturin sdist` checks to
    the PR CI.
  * Author a future OMEP for an `oxydemark` CLI if/when demand justifies it.
