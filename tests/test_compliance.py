"""Comark compliance suite driven by the shared fixtures (OMEP-0007).

The fixtures in ``tests/compliance/`` are the single source of truth for the
Comark behaviour contract; the Rust integration test in ``tests/compliance.rs``
consumes the very same files, in either the delimited-markdown (``*.md``) or
the JSON (``*.json``) format. Each case asserts the exact HTML produced by both
render paths and, optionally, a *partial* AST expectation: keys absent from a
fixture are never asserted, so additive AST changes cannot break the suite.

See ``tests/compliance/README.md`` for the formats and for how to add a case.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

import pytest

import oxydemark
from oxydemark import AstNode

FIXTURES_DIR = Path(__file__).parent / "compliance"

#: A fence opener or closer: at least three backticks plus an info string.
FENCE_RE = re.compile(r"^\s*(?P<backticks>`{3,})(?P<info>[^`]*)$")

#: Fence info string -> the case key it feeds.
FENCE_KEYS = {"comark": "markdown", "html": "html", "json ast": "ast"}

type Case = dict[str, Any]
type NodeSpec = dict[str, Any]


def _finish_case(name: str, prose: list[str], blocks: dict[str, str]) -> Case:
    """Validate an in-progress markdown fixture case and materialise it.

    Args:
        name: The case name, from its ``##`` heading.
        prose: The lines collected before the first fenced block.
        blocks: The fenced block bodies, keyed by case key.

    Returns:
        A case in the same shape as a JSON fixture case. The ``ast`` key is
        only present when the fixture declared a ``json ast`` block, so the
        absent-versus-``null`` distinction is preserved.

    Raises:
        AssertionError: If a required block is missing.
    """
    assert "markdown" in blocks, f"{name}: missing `comark` block"
    assert "html" in blocks, f"{name}: missing `html` block"

    case: Case = {"name": name, "markdown": blocks["markdown"], "html": blocks["html"]}
    if description := "\n".join(prose).strip():
        case["description"] = description
    if (raw := blocks.get("ast")) is not None:
        try:
            case["ast"] = json.loads(raw)
        except json.JSONDecodeError as error:
            raise AssertionError(f"{name}: invalid `json ast` block: {error}") from error
    return case


def _parse_markdown_fixture(source: str) -> list[Case]:
    """Parse a delimited-markdown fixture file into its cases.

    The grammar is documented in ``tests/compliance/README.md``: a ``##``
    heading opens a case, the prose that follows is its description, and the
    ``comark`` / ``html`` / ``json ast`` fenced blocks carry its data. Fences
    are CommonMark-like, so a longer run of backticks may wrap a fixture that
    itself contains a fenced block.

    Args:
        source: The whole fixture file.

    Returns:
        The declared cases, in file order.

    Raises:
        AssertionError: On any structural error.
    """
    lines = source.splitlines()
    cases: list[Case] = []
    name: str | None = None
    prose: list[str] = []
    blocks: dict[str, str] = {}
    index = 0

    while index < len(lines):
        line = lines[index]

        if line.startswith("## "):
            if name is not None:
                cases.append(_finish_case(name, prose, blocks))
            name = line[3:].strip()
            assert name, f"line {index + 1}: empty case name"
            prose, blocks = [], {}
            index += 1
            continue

        if match := FENCE_RE.match(line):
            info = match["info"].strip()
            assert name is not None, (
                f"line {index + 1}: fenced block {info!r} outside of a case"
            )
            key = FENCE_KEYS.get(info)
            assert key is not None, (
                f"{name}: unsupported fence info string {info!r} "
                "(expected `comark`, `html` or `json ast`)"
            )
            assert key not in blocks, f"{name}: duplicate `{info}` block"
            blocks[key], index = _read_block(lines, index, len(match["backticks"]), info)
            continue

        if name is not None and "markdown" not in blocks:
            prose.append(line)
        index += 1

    if name is not None:
        cases.append(_finish_case(name, prose, blocks))
    assert cases, "no case found (expected at least one `## <name>` heading)"
    return cases


def _read_block(lines: list[str], start: int, opener: int, info: str) -> tuple[str, int]:
    """Read the body of the fenced block opened at ``start``.

    Args:
        lines: All the lines of the fixture file.
        start: Index of the opening fence.
        opener: Length of the opening backtick run.
        info: The opening info string, for error messages.

    Returns:
        The block body, newline-terminated, and the index of the line after the
        closing fence.

    Raises:
        AssertionError: If the block is never closed.
    """
    body: list[str] = []
    index = start + 1
    while index < len(lines):
        match = FENCE_RE.match(lines[index])
        if match and not match["info"].strip() and len(match["backticks"]) >= opener:
            return "".join(f"{line}\n" for line in body), index + 1
        body.append(lines[index])
        index += 1
    raise AssertionError(f"line {start + 1}: unterminated `{info}` block")


def _load_cases() -> list[tuple[str, Case]]:
    """Collect every fixture case as an ``(id, case)`` pair.

    Returns:
        Pairs whose first element is a ``file::case`` identifier used as the
        pytest parameter id, ordered by file name then by declaration order.

    Raises:
        AssertionError: If no fixture file is found, if two files share a stem,
            or if a file declares two cases with the same name.
    """
    files = sorted(
        path
        for path in FIXTURES_DIR.iterdir()
        if path.suffix in {".json", ".md"} and path.name != "README.md"
    )
    assert files, f"no compliance fixtures found in {FIXTURES_DIR}"

    collected: list[tuple[str, Case]] = []
    stems: set[str] = set()
    for path in files:
        assert path.stem not in stems, (
            f"duplicate fixture name {path.stem!r}: test ids would collide"
        )
        stems.add(path.stem)
        source = path.read_text(encoding="utf-8")
        if path.suffix == ".md":
            cases = _parse_markdown_fixture(source)
        else:
            cases = json.loads(source)["cases"]
        names: set[str] = set()
        for case in cases:
            name = case["name"]
            assert name not in names, f"{path.stem}: duplicate case name {name!r}"
            names.add(name)
            collected.append((f"{path.stem}::{name}", case))
    return collected


CASES = _load_cases()


def _find_first(node: AstNode, kind: str) -> AstNode:
    """Return the first pre-order descendant of ``node`` with the given kind.

    Args:
        node: Root of the subtree to search.
        kind: The node kind to look for.

    Returns:
        The first matching node.

    Raises:
        AssertionError: If no descendant carries that kind.
    """
    for candidate in node.walk():
        if candidate.kind == kind:
            return candidate
    raise AssertionError(f"no descendant with kind {kind!r}")


def _assert_props(node: AstNode, expected: dict[str, Any] | None, path: str) -> None:
    """Assert the ``props`` expectation of a fixture node against ``node``.

    Args:
        node: The actual AST node.
        expected: ``None`` requires ``node.props`` to be ``None``; a mapping is
            matched as a subset.
        path: Breadcrumb used in assertion messages.
    """
    if expected is None:
        assert node.props is None, f"{path}: props should be None, got {node.props!r}"
        return

    assert node.props is not None, f"{path}: props is None, expected {expected!r}"
    for key, value in expected.items():
        assert key in node.props, f"{path}: prop {key!r} is missing"
        actual = node.props[key]
        # `True == 1` in Python, so compare the types before the values.
        assert isinstance(actual, type(value)) or actual is value is None, (
            f"{path}: prop {key!r} has type {type(actual).__name__}, "
            f"expected {type(value).__name__}"
        )
        assert actual == value, f"{path}: prop {key!r} is {actual!r}, expected {value!r}"


def _assert_node(node: AstNode, spec: NodeSpec, path: str) -> None:
    """Assert that ``node`` satisfies the partial expectation ``spec``.

    Only the keys present in ``spec`` are checked. ``attributes`` and ``props``
    are matched as subsets and ``children`` as a positional prefix, unless
    ``exact_children`` is set.

    Args:
        node: The actual AST node.
        spec: The partial expectation from the fixture.
        path: Breadcrumb used in assertion messages, e.g. ``root.children[0]``.
    """
    if (selector := spec.get("descend")) is not None:
        prefix, _, kind = selector.partition(":")
        assert prefix == "first", f"{path}: unsupported descend selector {selector!r}"
        anchored = _find_first(node, kind)
        inner = {key: value for key, value in spec.items() if key != "descend"}
        _assert_node(anchored, inner, f"{path}/first:{kind}")
        return

    if (kind := spec.get("kind")) is not None:
        assert node.kind == kind, f"{path}: kind is {node.kind!r}, expected {kind!r}"

    if (text := spec.get("text")) is not None:
        assert node.text == text, f"{path}: text is {node.text!r}, expected {text!r}"

    for key, value in spec.get("attributes", {}).items():
        assert key in node.attributes, (
            f"{path}: attribute {key!r} is missing (present: {sorted(node.attributes)})"
        )
        assert node.attributes[key] == value, (
            f"{path}: attribute {key!r} is {node.attributes[key]!r}, expected {value!r}"
        )

    for key in spec.get("absent_attributes", []):
        assert key not in node.attributes, (
            f"{path}: attribute {key!r} should be absent but is {node.attributes[key]!r}"
        )

    if "props" in spec:
        _assert_props(node, spec["props"], path)

    if (children := spec.get("children")) is not None:
        kinds = [child.kind for child in node.children]
        if spec.get("exact_children", False):
            assert len(node.children) == len(children), (
                f"{path}: expected exactly {len(children)} children, got {kinds}"
            )
        assert len(node.children) >= len(children), (
            f"{path}: expected at least {len(children)} children, got {kinds}"
        )
        for index, child_spec in enumerate(children):
            _assert_node(node.children[index], child_spec, f"{path}.children[{index}]")


@pytest.mark.parametrize(
    "case",
    [case for _, case in CASES],
    ids=[case_id for case_id, _ in CASES],
)
def test_comark_compliance(case: Case) -> None:
    """Render a fixture case on both paths and match its expected AST shape.

    Args:
        case: A single fixture case (``markdown``, ``html`` and optional
            ``ast``).
    """
    markdown: str = case["markdown"]
    expected_html: str = case["html"]

    assert oxydemark.markdown_to_html(markdown) == expected_html, "fast path HTML mismatch"

    ast = oxydemark.parse(markdown)
    assert oxydemark.render_ast(ast) == expected_html, "AST round-trip HTML mismatch"

    if (spec := case.get("ast")) is not None:
        _assert_node(ast, spec, "root")
