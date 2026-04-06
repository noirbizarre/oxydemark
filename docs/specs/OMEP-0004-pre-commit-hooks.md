---
status: accepted
date: 2026-03-21
decision-makers: [Axel H.]
---

# OMEP-0004: Pre-commit Hooks with prek

## Context and Problem Statement

Code quality checks (formatting, linting, commit message validation) should
run automatically before code reaches CI to shorten the feedback loop. This
requires a framework for managing and executing Git hooks.

The tool must support:

- Running hooks written in different languages (Rust, Python, shell).
- Validating commit messages against the Conventional Commits convention.
- Integrating with the existing pre-commit hook ecosystem.
- Providing a smooth developer experience (fast, no manual setup).

## Decision Drivers

* **Speed** -- Hooks run on every commit; they must be fast.
* **No runtime dependencies** -- Avoid requiring contributors to install
  Python or Node.js just for hooks.
* **Ecosystem compatibility** -- Reuse the vast library of existing
  `pre-commit` hooks.
* **TOML configuration** -- Consistent with the rest of the project's config
  files.
* **Built-in hooks** -- Some common checks (trailing whitespace, EOF fixer)
  should work offline with zero setup.

## Considered Options

* **Option A: Raw Git hooks** -- Shell scripts in `.git/hooks/`.
* **Option B: pre-commit (Python)** -- The original pre-commit framework.
* **Option C: prek** -- A Rust reimplementation of pre-commit, fully
  compatible with pre-commit configs.

## Decision Outcome

Chosen option: **"prek"** (Option C), because it is faster than pre-commit,
ships as a single binary with no runtime dependencies, supports TOML
configuration natively, and provides built-in Rust-native implementations of
common hooks.

### Consequences

* Good, because a single static binary -- no Python/Node required for hooks.
* Good, because `prek.toml` (TOML) is consistent with `Cargo.toml`,
  `mise.toml`, `cliff.toml`.
* Good, because built-in hooks (trailing-whitespace, end-of-file-fixer,
  check-toml, check-yaml) work offline and are significantly faster than their
  Python equivalents.
* Good, because fully compatible with existing pre-commit hook repositories.
* Bad, because prek is newer and less widely known than pre-commit; some
  contributors may be unfamiliar with it.
* Neutral, because prek is already adopted by major projects (CPython,
  FastAPI, Ruff, Airflow).

### Confirmation

* `mise run setup` installs hooks via `prek install`.
* Contributors can verify hooks work by running `prek run --all-files`.
* CI does not depend on prek (it runs cargo commands directly), so hook
  failures are caught locally before push.

## Pros and Cons of the Options

### Option A: Raw Git Hooks

* Good, because zero dependencies.
* Bad, because manual management -- hooks are not version-controlled by
  default (`.git/hooks/` is not tracked).
* Bad, because no isolation or language management.
* Bad, because difficult to share and maintain across contributors.

### Option B: pre-commit (Python)

* Good, because mature ecosystem, widely adopted.
* Good, because huge library of community hooks.
* Bad, because requires Python installed and a virtual environment.
* Bad, because slower due to Python startup and per-hook virtualenv creation.
* Bad, because YAML-only configuration.

### Option C: prek (Chosen)

* Good, because single binary, no runtime dependencies.
* Good, because faster hook execution (Rust-native built-in hooks, parallel
  execution by priority).
* Good, because TOML-native configuration (`prek.toml`).
* Good, because fully compatible with pre-commit hook repos and YAML configs.
* Good, because built-in hooks work offline.
* Neutral, because newer project, but actively maintained with rapid adoption.

## More Information

**Configuration file:** `prek.toml`

**Configured hooks:**

| Hook | Source | Stage |
| ---- | ------ | ----- |
| `trailing-whitespace` | builtin | pre-commit |
| `end-of-file-fixer` | builtin | pre-commit |
| `check-toml` | builtin | pre-commit |
| `check-yaml` | builtin | pre-commit |
| `check-merge-conflict` | builtin | pre-commit |
| `check-added-large-files` | builtin | pre-commit |
| `conventional-pre-commit` | conventional-pre-commit | commit-msg |
| `cargo-fmt` | local (system) | pre-commit |
| `cargo-clippy` | local (system) | pre-commit |

**Installation:** `mise run setup` (which runs `prek install`).

See: [prek documentation](https://prek.j178.dev/).
