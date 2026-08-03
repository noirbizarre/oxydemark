# Contributing to OxydeMark

Thank you for your interest in contributing to OxydeMark! This guide will help
you get started.

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- [Python](https://www.python.org/) >= 3.12
- [mise](https://mise.jdx.dev/) -- task runner and tool version manager
- [maturin](https://www.maturin.rs/) -- build backend for Rust/Python packages

## Development Setup

1. **Clone the repository:**

   ```sh
   git clone https://github.com/noirbizarre/oxydemark.git
   cd oxydemark
   ```

2. **Install tools via mise:**

   ```sh
   mise install
   ```

3. **Install pre-commit hooks:**

   ```sh
   mise run setup
   ```

4. **Build the project:**

   ```sh
   mise run build
   ```

5. **Run the tests:**

   ```sh
   mise run test
   ```

## Available Tasks

All tasks are managed through [mise](https://mise.jdx.dev/). Run `mise tasks`
to see the full list.

| Task                    | Description                              |
| ----------------------- | ---------------------------------------- |
| `mise run build`        | Build the Rust crate                     |
| `mise run test`         | Run the Rust test suite with nextest     |
| `mise run test:python`  | Run the Python test suite with pytest    |
| `mise run test:all`     | Run both Rust and Python test suites     |
| `mise run lint`         | Run clippy lints                         |
| `mise run fmt`          | Format Rust source code                  |
| `mise run fmt:check`    | Check formatting without modifying files |
| `mise run typos`        | Spell-check sources, docs and commit messages |
| `mise run ci`           | Run all CI checks locally                |
| `mise run changelog`    | Generate `CHANGELOG.md` for the next version |
| `mise run changelog:preview` | Preview the next version's release notes |
| `mise run release:preview` | Dry-run the release preparation       |
| `mise run cover`        | Generate the Rust coverage report        |
| `mise run cover:python` | Generate the Python coverage report      |
| `mise run cover:all`    | Generate both coverage reports           |
| `mise run bench`        | Run Python benchmarks                    |
| `mise run docs`         | Build the docs site and rustdoc reference |
| `mise run docs:serve`   | Preview the docs site locally            |
| `mise run setup`        | Install pre-commit hooks                 |

## Code Style

### Rust

- Code is formatted with `rustfmt` (default settings).
- Linting is enforced by `clippy` with `-D warnings` (all warnings are errors).
- Run `mise run fmt` to auto-format and `mise run lint` to check.

### Python

- Follow [PEP 8](https://peps.python.org/pep-0008/) conventions.
- Type hints are required for all public APIs.

## Commit Messages

This project uses [Conventional Commits](https://www.conventionalcommits.org/).
Every commit message must follow this format:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**Allowed types:** `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`,
`build`, `ci`, `chore`.

**Examples:**

```
feat(parser): add heading support
fix: handle empty input without panic
docs: update CONTRIBUTING.md with commit guidelines
```

Pre-commit hooks validate commit messages automatically. If a commit is
rejected, adjust the message to match the convention.

## Pull Request Process

1. Create a feature branch from `main`.
2. Make your changes, ensuring all checks pass (`mise run ci`).
3. Write or update tests as needed.
4. Push your branch and open a pull request against `main`.
5. Fill out the PR template; link any related issues.
6. A maintainer will review and merge once all checks pass.

## Compliance Fixtures

The Comark syntax contract (OMEP-0007) is covered by fixture-driven tests: JSON
files in [`tests/compliance/`](tests/compliance/) are consumed by both the Rust
integration test `tests/compliance.rs` and the pytest suite
`tests/test_compliance.py`.

When you change component, slot, prop or nesting behaviour, add a fixture case
rather than a hand-written test. See
[`tests/compliance/README.md`](tests/compliance/README.md) for the schema and a
step-by-step guide.

## Proposing Changes (OMEPs)

Significant design decisions are tracked as **OMEPs** (OxydeMark Enhancement
Proposals) in [`docs/specs/`](docs/specs/). OMEPs use the
[MADR](https://adr.github.io/madr/) format.

To propose a change:

1. Copy the template from the [OMEP README](docs/specs/README.md).
2. Create a new file: `docs/specs/OMEP-NNNN-short-title.md`.
3. Fill in all sections and open a PR for discussion.

See the existing OMEPs for examples.

## Changelog

The changelog is generated automatically from commit messages using
[git-cliff](https://git-cliff.org/). You do **not** need to edit
`CHANGELOG.md` by hand -- just write proper Conventional Commit messages.

`cliff.toml` also drives version selection, so the commit type you choose has
consequences: a `feat` bumps the minor version, a breaking change bumps the
major, and `chore(deps)` / `chore(release)` commits are excluded from the notes
entirely.

## Releasing

Releases are orchestrated by [gh-ship](https://github.com/noirbizarre/gh-ship)
using a Release-PR model (OMEP-0009). Maintainers never tag by hand.

1. **Every push to `main`** triggers 🚢 Ship, which runs `gh ship prepare`. That
   dispatches 🚀 Prepare Release, which derives the next version with
   `git cliff --bumped-version`, regenerates `CHANGELOG.md`, bumps `Cargo.toml`,
   `pyproject.toml`, `Cargo.lock` and `uv.lock` in lockstep, and opens or
   updates the **Release PR** from `release/next`.
   If there is nothing to release the run reports `changed: false` and exits 0.
2. **Review the Release PR.** The version bump and the exact release notes are
   both visible there. This is the last reversible point: neither crates.io nor
   PyPI lets a published version be overwritten.
3. **Merge it.** 🚢 Ship then runs `gh ship release`, which tags the merge
   commit, creates the GitHub Release as a draft, dispatches 📦 Publish Release
   (crates.io, PyPI, and the wheels/sdist attached as assets), and finally makes
   the release visible.

To see what the next release would look like without touching anything:

```sh
gh extension install noirbizarre/gh-ship
mise run release:preview     # or: gh ship preview --json
gh ship status               # where a release currently stands
```

Note that `release/next` is a staging branch owned by gh-ship: anything pushed
to it by hand is discarded.

## License

By contributing, you agree that your contributions will be licensed under the
[MIT License](LICENSE).
