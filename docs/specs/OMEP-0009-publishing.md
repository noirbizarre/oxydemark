---
status: accepted
date: 2026-08-02
decision-makers: [Axel H.]
---

# OMEP-0009: Publishing & Distribution (crates.io + PyPI)

!!! note "Amended 2026-08-02"

    The original decision chose **T2 (tag-triggered releases)**. That has been
    reversed in favour of **T3: a Release-PR model orchestrated by
    [gh-ship](https://github.com/noirbizarre/gh-ship)**. The authentication
    (A2), wheel matrix (W2) and library-only (C1) decisions are unchanged.
    Sections marked *(amended)* below carry the current behaviour.

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
* **Option T2: Tag-triggered release workflow** -- Pushing an `X.Y.Z` tag runs
  a GitHub Actions workflow that builds, publishes to both indexes, and creates
  the GitHub Release.
* **Option T3: Release-PR model orchestrated by gh-ship** *(amended)* -- Every
  push to `main` runs `gh ship prepare`, which dispatches a workflow that
  computes the next version, regenerates the changelog and opens a **Release
  PR**. Merging it runs `gh ship release`, which tags the merge commit, drafts
  the GitHub Release, dispatches the publish workflow, then makes the release
  visible.

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

Chosen options: **T3 (Release-PR model orchestrated by gh-ship)**, **A2
(Trusted Publishing)**, **W2 (full platform/Python matrix + sdist)**, and **C1
(library-only for v0.2.0)**.

Together these define a release lifecycle in which `gh ship` owns the
*orchestration* (release branch, workflow dispatch, Release PR, tag, GitHub
Release) while this repository owns the *content* (version bump, changelog,
build, publish). Authentication is OIDC throughout; the full wheel matrix plus
an sdist and the crate are published on merge of the Release PR. v0.2.0 remains
library-only; a CLI is deferred.

### Release trigger and versioning *(amended)*

The release is driven by three workflows plus `.github/ship.yml`:

| File | Role |
| ---- | ---- |
| `.github/workflows/ship.yml` | Orchestrator. Runs `gh ship prepare` on every push to `main`, and `gh ship release` when the Release PR merges. |
| `.github/workflows/prepare-release.yml` | Ours. Computes the version, regenerates `CHANGELOG.md`, bumps the manifests, emits the release artifact. |
| `.github/workflows/release.yml` | Ours. Publishes to crates.io and PyPI and attaches assets to the draft release. |

* **No manual tagging.** `gh ship release` creates the tag on the **merge
  commit** of the Release PR -- never a remembered SHA, since a squash merge
  produces a new one. Tags remain unprefixed `X.Y.Z`, matching `cliff.toml`'s
  `tag_pattern`.
* **The version is derived, not chosen.** `prepare-release.yml` runs
  `git cliff --bumped-version` and compares the result against the last *tag*
  (`git describe --tags --abbrev=0`), not against the manifests -- the manifests
  already carry the version being prepared, so comparing to them would report
  "no change" for the very first release and make bootstrapping impossible.
* **Bootstrapping.** The repository carries no tags, so git-cliff returns
  `[bump] initial_tag` verbatim. That is set to **`0.2.0`**, not `0.1.0`,
  because `oxydemark` 0.1.0 is already published on crates.io (a name
  reservation predating this pipeline) and crates.io refuses to overwrite a
  released version.
* **Lockstep bump.** `prepare-release.yml` writes the version into
  `Cargo.toml`, `pyproject.toml`, `Cargo.lock` (`cargo update --workspace`) and
  `uv.lock` (`uv lock`) in one commit, `chore(release): X.Y.Z`. The version gate
  in `release.yml` still verifies tag/manifest agreement -- it now catches a
  faulty prepare run rather than a bad manual tag.
* Version numbers follow the 0.x policy from OMEP-0008: a breaking change to a
  public surface bumps MINOR (`0.1.z -> 0.2.0`); additive/fix changes bump
  PATCH. `cliff.toml` sets `features_always_bump_minor` and
  `breaking_always_bump_major`.
* **Notes are generated pre-merge.** `git cliff --unreleased --bump --strip all`
  produces the notes, which travel in the `ship.release.json` artifact through
  the Release PR body to the GitHub Release. What ships is therefore exactly
  what was reviewed. `CHANGELOG.md` is never edited by hand (OMEP-0003).
* **`release.yml` must keep its filename.** The crates.io and PyPI Trusted
  Publishers are bound to the workflow *filename*; renaming it to
  `publish-release.yml` (gh-ship's template name) would make both registries
  reject the OIDC token. `.github/ship.yml` therefore declares
  `workflows.publish: release`.
* **Contract with gh-ship.** Both dispatched workflows must declare
  `workflow_dispatch` (a `workflow_call`-only workflow cannot be started
  through the API) and must carry the `ship:${{ inputs.ship_id }}` nonce in
  their `run-name`, which is how gh-ship correlates a dispatch to a run.

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

`abi3` (the stable Python ABI) was adopted: the crate's `extension-module`
feature enables `pyo3/abi3-py312`, so maturin emits a single `cp312-abi3` wheel
per target that covers every supported Python (3.12, 3.13, 3.14 and later).
The matrix therefore varies by target only:

| Runner | Target |
| ------ | ------ |
| `ubuntu-latest` | `x86_64-unknown-linux-gnu` (manylinux) |
| `ubuntu-latest` | `aarch64-unknown-linux-gnu` (manylinux, cross) |
| `macos-13` | `x86_64-apple-darwin` |
| `macos-14` | `aarch64-apple-darwin` |
| `windows-latest` | `x86_64-pc-windows-msvc` |

* `abi3-py312` is gated on `extension-module`, not on `python`, so
  `cargo test --features python` keeps the version-specific ABI its
  `auto-initialize` dev-dependency requires.
* A **source distribution (sdist)** is also built (`maturin sdist`) so that
  platforms outside the matrix can build from source given a Rust toolchain.
* Each wheel is verified before upload (`.github/scripts/check_wheel.py`): it
  must be `abi3`-tagged and must contain `py.typed` and `_core.pyi`
  (OMEP-0008). The sdist is installed and imported in a clean virtualenv, which
  guards the `Cargo.toml` `include` allow-list.
* All wheels and the sdist are collected as workflow artifacts, then published
  in a single final `pypi` job with `pypa/gh-action-pypi-publish`.
* Authentication uses PyPI **Trusted Publishing** (OIDC via
  `pypa/gh-action-pypi-publish` or maturin's OIDC support); no PyPI API token
  secret is stored.

### Trusted Publishing *(amended)*

* Both indexes are configured with the GitHub repository, the workflow
  filename `release.yml`, and the `release` GitHub Environment as the Trusted
  Publisher. **The filename is part of the OIDC claim**, so `release.yml` may
  not be renamed and its publishing jobs must keep `environment: release`.
* The trigger type is *not* part of the claim, so moving `release.yml` from
  `on: push: tags` to `on: workflow_dispatch` (dispatched by gh-ship) does not
  affect either publisher.
* `oxydemark` does not yet exist on PyPI, so it is bootstrapped with a PyPI
  **pending publisher**: a Trusted Publisher registered against a project that
  does not exist, which reserves the name and converts to a normal publisher on
  the first upload. No manual placeholder release is needed.
* The `release` workflow requests `permissions: id-token: write` so GitHub mints
  the OIDC token used to authenticate to both crates.io and PyPI.
* Rationale: short-lived, scope-limited credentials eliminate the standing risk
  of leaked long-lived tokens and remove secret-rotation toil.
* gh-ship itself is *not* secretless: `ship.yml` and `prepare-release.yml` need
  a `SHIP_TOKEN` (GitHub App token or fine-grained PAT) in the `release`
  environment, because a pull request authored by the default `GITHUB_TOKEN`
  does not trigger workflows and the Release PR would show no CI results. It
  needs Contents, Actions, Pull requests and Issues read/write, plus Metadata
  read.

### CLI in v0.2.0: library-only

v0.2.0 ships **no** CLI binary. OMEP-0008 froze the public surfaces as the
parse/transform/render functions, `AstNode`, `OxydeEngine`, and `Plugin` -- all
library entry points. A CLI would add a new, separately versioned surface
(argument grammar, output flags, exit codes) that we are not ready to commit to
this early. A future OMEP may introduce an `oxydemark` CLI (as a Cargo `[[bin]]`
and/or a Python `[project.scripts]` console entry point) once the library
surface has stabilised.

### Consequences

* Good, because merging one reviewed pull request produces coherent, traceable
  releases across both ecosystems with release notes that match the changelog.
* Good, because the version bump and the release notes are **reviewable before
  they exist as a tag**: a mistake is a closed pull request, not a published
  version that neither registry lets you overwrite.
* Good, because Trusted Publishing removes all long-lived registry secrets from
  the repository.
* Good, because the full wheel matrix means Python consumers never need a Rust
  toolchain on the common platforms.
* Good, because reusing maturin, GitHub Actions, and git-cliff avoids new
  tooling in the *content* half; gh-ship is confined to orchestration and never
  parses a version or renders a changelog.
* Bad, because the wheel matrix (cross-compilation for aarch64, macOS, Windows)
  materially lengthens the release workflow and adds runner cost.
* Bad, because lockstep versioning can force a no-op bump on one ecosystem when
  only the other changed (already accepted in OMEP-0008).
* Bad, because gh-ship reintroduces one stored secret (`SHIP_TOKEN`) after A2
  had removed all of them, and adds a third-party dependency to the release
  path.
* Neutral, because deferring the CLI keeps scope small now but leaves a
  frequently-requested convenience for later.

### Confirmation

* `ls docs/specs/OMEP-0009-publishing.md` confirms the OMEP exists.
* `.github/ship.yml`, `.github/workflows/ship.yml`,
  `.github/workflows/prepare-release.yml` and `.github/workflows/release.yml`
  all exist; `gh ship validate` reports the setup and both dispatched workflows
  as conformant.
* `git cliff --bumped-version` prints `0.2.0` on a repository with no tags.
* `gh ship preview` dry-runs the preparation without mutating anything and
  reports `changed: true`, `version: 0.2.0`, `tag: 0.2.0`.
* On a pull request, CI runs `cargo publish --dry-run` and `maturin build` so
  packaging regressions are caught before a release is prepared, and renders the
  release notes so a `cliff.toml` regression fails there rather than mid-release.
* A version-consistency check fails the release if the tag, `Cargo.toml`, and
  `pyproject.toml` versions disagree.
* After the first release, `cargo add oxydemark` and `pip install oxydemark`
  resolve the same `X.Y.Z`, and a matching non-draft GitHub Release exists with
  git-cliff-generated notes and the wheels plus sdist attached.

## Pros and Cons of the Options

### Trigger: T1 (manual) vs T2 (tag-triggered) vs T3 (Release PR)

* T1 -- Good, because zero workflow to maintain. Bad, because error-prone,
  non-reproducible, and easy to publish mismatched versions to the two indexes.
* T2 -- Good, because reproducible, auditable, and atomic across ecosystems.
  Bad, because the tag is the point of no return: the version bump, the
  changelog and the notes are only observable *after* the tag exists, and
  neither crates.io nor PyPI permits overwriting a published version. It also
  relies on the maintainer bumping four files by hand, in lockstep, correctly.
* T3 (Chosen) -- Good, because the Release PR makes the bump, the regenerated
  changelog and the exact release notes reviewable before anything is
  published, and the notes travel in a single artifact from the PR body to the
  GitHub Release so what ships equals what was reviewed. Good, because tagging
  becomes a consequence of merging rather than a manual act. Bad, because it
  adds a third-party orchestrator and a stored `SHIP_TOKEN`. Neutral, because
  the publish workflow is largely the T2 one with its trigger swapped.

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
* [gh-ship](https://github.com/noirbizarre/gh-ship) and its
  [workflow contract](https://noirbizarre.github.io/gh-ship/workflows/) /
  [release artifact spec](https://noirbizarre.github.io/gh-ship/specifications/release-artifact/).
* Related: [OMEP-0003](OMEP-0003-changelog-management.md) (git-cliff release
  notes), [OMEP-0005](OMEP-0005-ci-cd-pipeline.md) (CI that already builds
  smoke-test wheels), [OMEP-0008](OMEP-0008-public-api.md) (lockstep versioning
  and the frozen surfaces being distributed).
* Follow-up actions:
  * ~~Add `.github/workflows/release.yml` implementing the publish
    pipeline.~~ Done: version gate, `crates-io`, `wheels`, `sdist`, `pypi` and
    `assets` jobs.
  * ~~Adopt gh-ship and add the Release-PR orchestration.~~ Done:
    `.github/ship.yml`, `ship.yml` and `prepare-release.yml`.
  * ~~Add `cargo publish --dry-run` and `maturin build`/`maturin sdist` checks
    to the PR CI.~~ Done: the `package`, `python` and `sdist` jobs in
    `ci.yml`.
  * **Outstanding (manual, one-time):**
    * Register the PyPI **pending publisher**: project `oxydemark`, repository
      `noirbizarre/oxydemark`, workflow `release.yml`, environment `release`.
    * Confirm the crates.io Trusted Publisher is bound to `release.yml` and
      environment `release`.
    * Create `SHIP_TOKEN` (GitHub App token preferred) and store it in the
      `release` environment.
  * Note: with maturin's default `sdist-generator = "cargo"`, the sdist file
    list is derived from `cargo package --list`. Any `include`/`exclude` in
    `[package]` therefore constrains the PyPI sdist as well as the crates.io
    tarball; `python/**` and `pyproject.toml` must stay included.
  * Author a future OMEP for an `oxydemark` CLI if/when demand justifies it.
