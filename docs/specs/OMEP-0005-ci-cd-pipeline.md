---
status: accepted
date: 2026-03-21
decision-makers: [Axel H.]
---

# OMEP-0005: CI/CD Pipeline

## Context and Problem Statement

The project needs a Continuous Integration pipeline that automatically verifies
every push and pull request. The pipeline must cover Rust formatting, linting,
testing (on multiple toolchains), Python wheel building, and changelog
validation.

The project is hosted on GitHub, so the CI solution should integrate natively
with the GitHub platform.

## Decision Drivers

* **GitHub-native** -- The repository is on GitHub; the CI tool should
  integrate seamlessly (status checks, PR annotations, caching).
* **Matrix testing** -- Rust stable and nightly should both be tested to catch
  regressions early.
* **Fast feedback** -- Jobs should run in parallel where possible.
* **Reproducibility** -- CI should use the same tool versions as local
  development (pinned via mise or action versions).
* **Minimal maintenance** -- Prefer well-maintained community actions over
  custom scripts.

## Considered Options

* **Option A: GitHub Actions** -- GitHub's built-in CI/CD platform.
* **Option B: GitLab CI** -- GitLab's CI system (would require mirroring).
* **Option C: CircleCI** -- Third-party CI service with GitHub integration.

## Decision Outcome

Chosen option: **"GitHub Actions"** (Option A), because the project is hosted
on GitHub, Actions are free for public repositories, and the Rust ecosystem
has excellent community-maintained actions (`dtolnay/rust-toolchain`,
`Swatinem/rust-cache`, `taiki-e/install-action`).

### Consequences

* Good, because zero additional service to manage -- everything is in
  `.github/workflows/`.
* Good, because tight integration with GitHub (status checks block merges,
  annotations appear in PR diffs).
* Good, because community actions for Rust toolchain and caching are
  battle-tested.
* Good, because matrix strategies allow testing stable + nightly in parallel.
* Bad, because GitHub Actions YAML can be verbose for complex workflows.
* Neutral, because free for public repos; costs only apply if the repo goes
  private.

### Confirmation

* Every push to `main` and every PR triggers the workflow.
* Branch protection rules require all CI jobs to pass before merging.
* The workflow can be run locally via `mise run ci` (for the check/lint/test
  subset).

## Pros and Cons of the Options

### Option A: GitHub Actions (Chosen)

* Good, because native GitHub integration (no external service).
* Good, because free for public repositories.
* Good, because rich marketplace of community actions.
* Good, because matrix builds for multi-toolchain testing.
* Neutral, because YAML syntax, but well-documented.

### Option B: GitLab CI

* Good, because powerful pipeline DSL.
* Bad, because requires mirroring or migrating the repository.
* Bad, because adds operational complexity.

### Option C: CircleCI

* Good, because fast builds, good caching.
* Bad, because third-party dependency (account, billing).
* Bad, because less tight GitHub integration than Actions.

## More Information

**Workflow file:** `.github/workflows/ci.yml`

**Jobs:**

| Job | Description | Toolchain |
| --- | ----------- | --------- |
| `check` | `cargo fmt --check` + `cargo clippy` | stable |
| `test` | `cargo nextest run` | stable, nightly (matrix) |
| `build` | `cargo build` | stable |
| `python` | Build wheel with maturin, smoke test import | stable + Python 3.12 |
| `changelog` | `git cliff --unreleased` validation | N/A |

**Key actions used:**

| Action | Purpose |
| ------ | ------- |
| `actions/checkout@v4` | Clone the repository |
| `dtolnay/rust-toolchain` | Install Rust toolchain |
| `Swatinem/rust-cache@v2` | Cache cargo registry and target dir |
| `taiki-e/install-action` | Install cargo-nextest, git-cliff |
| `actions/setup-python@v5` | Install Python for wheel build |

**Triggers:** `push` to `main`, `pull_request` targeting `main`.

See: [GitHub Actions documentation](https://docs.github.com/en/actions).
