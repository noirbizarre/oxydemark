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

| Task                | Description                              |
| ------------------- | ---------------------------------------- |
| `mise run build`    | Build the Rust crate                     |
| `mise run test`     | Run the test suite with nextest          |
| `mise run lint`     | Run clippy lints                         |
| `mise run fmt`      | Format Rust source code                  |
| `mise run fmt:check`| Check formatting without modifying files |
| `mise run ci`       | Run all CI checks locally                |
| `mise run changelog`| Generate `CHANGELOG.md`                  |
| `mise run coverage` | Generate code coverage report            |
| `mise run setup`    | Install pre-commit hooks                 |

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

## License

By contributing, you agree that your contributions will be licensed under the
[MIT License](LICENSE).
