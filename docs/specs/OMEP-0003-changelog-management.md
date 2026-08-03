---
status: accepted
date: 2026-03-21
decision-makers: [Axel H.]
---

# OMEP-0003: Changelog Management with git-cliff

## Context and Problem Statement

The project needs an automated changelog to communicate changes to users and
contributors between releases. The changelog should be generated from the Git
history rather than maintained by hand, to reduce friction and ensure
completeness.

This requires:

1. A **commit message convention** that encodes the *type* of change.
2. A **generator tool** that parses commits and produces a structured
   changelog.

## Decision Drivers

* **Automation** -- No manual changelog editing; the Git history *is* the
  source of truth.
* **Conventional Commits** -- The project already mandates Conventional
  Commits (see pre-commit hooks); the changelog tool must understand them.
* **Customizable output** -- The generated file should follow
  [Keep a Changelog](https://keepachangelog.com/) conventions.
* **Rust-native** -- Prefer tools from the Rust ecosystem for consistency
  and performance.
* **CI integration** -- The tool must be runnable in CI to validate that
  unreleased changes are parseable.

## Considered Options

* **Option A: Manual CHANGELOG.md** -- Developers edit the file by hand in
  each PR.
* **Option B: conventional-changelog (Node.js)** -- A widely-used Node tool
  for generating changelogs.
* **Option C: git-cliff** -- A Rust-native changelog generator with Tera
  templates.

## Decision Outcome

Chosen option: **"git-cliff"** (Option C), because it is fast, Rust-native,
highly configurable via TOML + Tera templates, and understands Conventional
Commits out of the box.

### Consequences

* Good, because the changelog is always up to date with the Git history.
* Good, because the TOML configuration (`cliff.toml`) lives alongside the
  rest of the project config.
* Good, because Tera templates allow full control over output format.
* Bad, because contributors must write proper Conventional Commit messages
  (mitigated by pre-commit hook validation).
* Neutral, because the generated `CHANGELOG.md` should not be edited by hand;
  this is a workflow change for contributors accustomed to manual changelogs.

### Confirmation

* CI renders the next release's notes on every push
  (`git cliff --unreleased --bump --strip all`) to verify that recent commits
  are parseable and that the template still works.
* `mise run changelog` regenerates the full file locally;
  `mise run changelog:preview` shows just the next release's notes.

!!! note "Amended 2026-08-02 (OMEP-0009)"

    `cliff.toml` was replaced with the configuration shared with
    [gh-ship](https://github.com/noirbizarre/gh-ship), so the two projects
    render release notes identically. Two consequences for this OMEP:

    * The output no longer follows the *Keep a Changelog* layout literally.
      Compare links are inline in each release heading rather than collected
      into a link-reference footer, groups carry emoji labels, and each entry
      links to its commit. The spirit -- a human-readable, chronological,
      grouped changelog -- is unchanged.
    * `git-cliff` is no longer only a reporting tool: `--bumped-version` is what
      derives the next version in `prepare-release.yml`, and `cliff.toml` shells
      out to [`typos`](https://github.com/crate-ci/typos) as a commit
      preprocessor, so that binary is now required wherever git-cliff runs.

## Pros and Cons of the Options

### Option A: Manual CHANGELOG.md

* Good, because no tooling required.
* Bad, because prone to human error and omissions.
* Bad, because creates merge conflicts when multiple PRs touch the file.
* Bad, because slows down the release process.

### Option B: conventional-changelog (Node.js)

* Good, because mature ecosystem with many presets.
* Good, because widely adopted.
* Bad, because introduces a Node.js runtime dependency into a Rust project.
* Bad, because configuration is spread across multiple JSON/JS files.

### Option C: git-cliff (Chosen)

* Good, because single static binary, no runtime dependencies.
* Good, because native Conventional Commits support.
* Good, because Tera template engine is powerful and familiar to Rust
  developers.
* Good, because TOML config is consistent with `Cargo.toml`, `mise.toml`,
  `prek.toml`.
* Neutral, because smaller community than conventional-changelog, but
  actively maintained and growing.

## More Information

**Configuration file:** `cliff.toml`

**Commit type mapping:**

Groups are rendered in the order below, which is why each carries an
`<!-- N -->` ordering marker in `cliff.toml`.

| Commit prefix | Changelog group |
| ------------- | --------------- |
| `feat`        | 💫 Features      |
| `fix`         | 🐛 Bug Fixes     |
| `perf`        | ⚡ Performance   |
| `refactor`    | 🔨 Refactor      |
| `doc`         | 📚 Documentation |
| `test`        | 🧪 Tests         |
| `style`       | 🎨 Style         |
| `build`       | 🏗️ Build         |
| `ci`          | 🔧 CI            |
| `chore`       | 🧹 Chores        |
| `revert`      | ⏪ Reverts       |

`chore(release):` and `chore(deps)` are skipped entirely: the former is the
release commit itself, the latter is dependency noise. Both rules are matched
before the general `^chore` rule.

See: [git-cliff documentation](https://git-cliff.org/).
