# OxydeMark Enhancement Proposals (OMEPs)

OMEPs are design documents that capture important architectural and tooling
decisions for the OxydeMark project. They follow the
[MADR](https://adr.github.io/madr/) (Markdown Any Decision Records) format.

## What is an OMEP?

An OMEP records the **context**, **options considered**, and **rationale**
behind a significant decision. Think of them as the project's institutional
memory -- they explain *why* things are the way they are, not just *what* they
are.

OMEPs are inspired by [Python PEPs](https://peps.python.org/) and
[Zensical ZEPs](https://zensical.org/zep/) but scoped to project-level
decisions rather than language design.

## When to Write an OMEP

Write an OMEP when you are:

- Introducing a new tool, framework, or major dependency.
- Changing the project architecture or pipeline design.
- Establishing a new convention or process.
- Making a decision that future contributors will wonder about.

For small, self-evident changes (typo fixes, minor refactors), an OMEP is not
necessary.

## OMEP Lifecycle

| Status        | Meaning                                        |
| ------------- | ---------------------------------------------- |
| `proposed`    | Under discussion; PR is open                   |
| `accepted`    | Decision has been approved and is in effect     |
| `rejected`    | Proposal was considered but not adopted         |
| `deprecated`  | Decision is no longer relevant                  |
| `superseded`  | Replaced by another OMEP (link to successor)    |

## Numbering

OMEPs are numbered sequentially: `OMEP-0001`, `OMEP-0002`, etc. The filename
follows the pattern:

```
OMEP-NNNN-short-title.md
```

## Template

Use the template below when creating a new OMEP. All sections marked
*"optional"* may be removed if they do not apply.

---

```markdown
---
status: proposed
date: YYYY-MM-DD
decision-makers: [list of people]
---

# OMEP-NNNN: Short Title

## Context and Problem Statement

Describe the context, the problem, and why a decision is needed.

## Decision Drivers

* Driver 1
* Driver 2

## Considered Options

* Option A
* Option B
* Option C

## Decision Outcome

Chosen option: "Option X", because [justification].

### Consequences

* Good, because ...
* Bad, because ...

### Confirmation

How will we verify this decision is working?

## Pros and Cons of the Options

### Option A

* Good, because ...
* Bad, because ...

### Option B

* Good, because ...
* Bad, because ...

## More Information

Links, references, follow-up actions.
```

## Index

| OMEP | Title | Status |
| ---- | ----- | ------ |
| [0001](OMEP-0001-project-architecture.md) | Project Architecture | accepted |
| [0002](OMEP-0002-task-management.md) | Task Management with mise | accepted |
| [0003](OMEP-0003-changelog-management.md) | Changelog Management with git-cliff | accepted |
| [0004](OMEP-0004-pre-commit-hooks.md) | Pre-commit Hooks with prek | accepted |
| [0005](OMEP-0005-ci-cd-pipeline.md) | CI/CD Pipeline | accepted |
| [0006](OMEP-0006-markdown-parser.md) | Markdown Parser -- Rushdown | accepted |
| [0007](OMEP-0007-comark-syntax.md) | Extended Syntax -- Comark Specification | accepted |
| [0008](OMEP-0008-public-api.md) | Public API Stability & Versioning Policy | accepted |
| [0009](OMEP-0009-publishing.md) | Publishing & Distribution (crates.io + PyPI) | accepted |
| [0010](OMEP-0010-metadata-extraction.md) | Structured metadata extraction (TOC / anchors / summary) | accepted |
| [0011](OMEP-0011-documentation.md) | Documentation Site & API Reference | accepted |
| [0012](OMEP-0012-code-coverage.md) | Code Coverage Reporting | accepted |
