---
status: accepted
date: 2026-03-21
decision-makers: [Axel H.]
---

# OMEP-0002: Task Management with mise

## Context and Problem Statement

The project needs a task runner to provide a consistent interface for common
development operations (build, test, lint, format, etc.). The chosen tool
should also manage language toolchain versions so that every contributor works
with the same Rust and Python versions regardless of what is installed
system-wide.

## Decision Drivers

* **Unified interface** -- One command to run any project task, regardless of
  the underlying tool.
* **Tool version management** -- Pin Rust, Python, and CLI tool versions
  per-project.
* **Low overhead** -- Fast startup, no heavyweight runtimes.
* **Cross-platform** -- Must work on Linux, macOS, and (ideally) Windows.
* **Convention over configuration** -- Sensible defaults; minimal boilerplate.

## Considered Options

* **Option A: Make / GNU Make** -- The traditional Unix build tool.
* **Option B: just** -- A modern command runner inspired by Make.
* **Option C: mise** -- A polyglot task runner and version manager
  (successor to `rtx`/`asdf`).

## Decision Outcome

Chosen option: **"mise"** (Option C), because it combines task running *and*
tool version management in a single tool, eliminating the need for separate
version managers (rustup overrides, pyenv, asdf).

### Consequences

* Good, because `mise.toml` is the single source of truth for tool versions
  and tasks.
* Good, because contributors only need to run `mise install` to get the exact
  toolchain.
* Good, because tasks are defined declaratively in TOML, which is consistent
  with the rest of the project's configuration.
* Bad, because mise is less widely known than Make; new contributors may need
  to install it.
* Neutral, because mise is actively maintained and has a growing adoption in
  the Rust/Python ecosystem.

### Confirmation

* The CI pipeline installs tools via mise to verify reproducibility.
* `mise run ci` succeeds locally before any PR is merged.

## Pros and Cons of the Options

### Option A: Make

* Good, because universally available on Unix systems.
* Good, because extremely well-documented.
* Bad, because Makefile syntax is error-prone (tabs vs. spaces, implicit
  rules).
* Bad, because no built-in version management for Rust/Python.
* Bad, because Windows support requires extra tooling (e.g. MSYS2).

### Option B: just

* Good, because clean, modern syntax (no tab sensitivity).
* Good, because fast and dependency-free.
* Bad, because no built-in tool version management.
* Bad, because task dependencies are less expressive than mise.

### Option C: mise (Chosen)

* Good, because task runner + version manager in one tool.
* Good, because TOML configuration is clean and well-structured.
* Good, because task dependencies (`depends`) allow composing complex
  workflows.
* Good, because supports shims so `cargo`, `python`, etc. automatically use
  the pinned versions.
* Neutral, because requires installation (`curl https://mise.run | sh` or
  package manager).

## More Information

**Defined tasks:**

| Task               | Command                          |
| ------------------ | -------------------------------- |
| `build`            | `cargo build`                    |
| `test`             | `cargo nextest run`              |
| `test:python`      | `uv run --group test pytest`     |
| `test:all`         | depends on test, test:python     |
| `lint`             | `cargo clippy -- -D warnings`    |
| `fmt`              | `cargo fmt`                      |
| `fmt:check`        | `cargo fmt -- --check`           |
| `ci`               | depends on fmt:check, lint, test |
| `changelog`        | `git cliff --output CHANGELOG.md`|
| `changelog:preview`| `git cliff --unreleased`         |
| `cover`            | `cargo llvm-cov`                 |
| `bench`            | `uv run --group bench pytest benchmarks/ --benchmark-only` |
| `setup`            | `prek install`                   |

**Managed tools:**

| Tool              | Purpose                    |
| ----------------- | -------------------------- |
| `rust`            | Compiler toolchain         |
| `python`          | Python interpreter         |
| `prek`            | Pre-commit hooks           |
| `cargo-binstall`  | Binary crate installer     |
| `git-cliff`       | Changelog generator        |
| `cargo-nextest`   | Test runner                |
| `cargo-llvm-cov`  | Code coverage              |

See: [mise documentation](https://mise.jdx.dev/).
