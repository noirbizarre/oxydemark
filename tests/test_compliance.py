"""Comark compliance suite driven by the shared JSON fixtures (OMEP-0007).

The fixtures in ``tests/compliance/`` are the single source of truth for the
Comark behaviour contract; the Rust integration test in ``tests/compliance.rs``
consumes the very same files. Each case asserts the exact HTML produced by both
render paths and, optionally, a *partial* AST expectation: keys absent from a
fixture are never asserted, so additive AST changes cannot break the suite.

See ``tests/compliance/README.md`` for the schema and for how to add a case.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest

import oxydemark
from oxydemark import AstNode

FIXTURES_DIR = Path(__file__).parent / "compliance"

type Case = dict[str, Any]
type NodeSpec = dict[str, Any]


def _load_cases() -> list[tuple[str, Case]]:
    """Collect every fixture case as an ``(id, case)`` pair.

    Returns:
        Pairs whose first element is a ``file::case`` identifier used as the
        pytest parameter id, ordered by file name then by declaration order.

    Raises:
        AssertionError: If no fixture file is found, or if a file declares two
            cases with the same name.
    """
    files = sorted(FIXTURES_DIR.glob("*.json"))
    assert files, f"no compliance fixtures found in {FIXTURES_DIR}"

    collected: list[tuple[str, Case]] = []
    for path in files:
        fixture = json.loads(path.read_text(encoding="utf-8"))
        names: set[str] = set()
        for case in fixture["cases"]:
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
