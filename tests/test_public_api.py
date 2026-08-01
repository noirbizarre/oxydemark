"""Tests enforcing the public API contract frozen in OMEP-0008.

If these tests fail, the public surface has changed and OMEP-0008 must be
updated together with a breaking-change version bump (see the semver policy).
"""

from __future__ import annotations

import importlib
import os

import oxydemark


def _package_dir() -> str:
    """Return the on-disk directory of the installed ``oxydemark`` package."""
    spec = importlib.util.find_spec("oxydemark")
    assert spec is not None
    assert spec.submodule_search_locations is not None
    return spec.submodule_search_locations[0]

# The frozen public Python surface, per OMEP-0008, extended additively by
# OMEP-0010 (slugify, extract_summary, parse_document, ParseResult, Heading).
PUBLIC_SURFACE = frozenset(
    {
        "AstNode",
        "Heading",
        "OxydeEngine",
        "ParseResult",
        "Plugin",
        "extract_summary",
        "markdown_to_html",
        "parse",
        "parse_document",
        "render_ast",
        "slugify",
    }
)


def test_all_matches_frozen_surface():
    """oxydemark.__all__ must match the surface documented in OMEP-0008."""
    assert set(oxydemark.__all__) == PUBLIC_SURFACE


def test_all_names_are_importable():
    """Every name in __all__ must be a real attribute of the package."""
    for name in oxydemark.__all__:
        assert hasattr(oxydemark, name), f"{name} missing from oxydemark"


def test_all_has_no_duplicates():
    """__all__ must not contain duplicate entries."""
    assert len(oxydemark.__all__) == len(set(oxydemark.__all__))


def test_package_ships_py_typed_marker():
    """OMEP-0008 mandates a PEP 561 py.typed marker for the package."""
    package_dir = _package_dir()
    assert os.path.exists(os.path.join(package_dir, "py.typed")), (
        "py.typed marker missing; required by OMEP-0008 typing strategy"
    )


def test_package_ships_core_stub():
    """OMEP-0008 mandates a hand-written _core.pyi stub shipped with the package."""
    package_dir = _package_dir()
    assert os.path.exists(os.path.join(package_dir, "_core.pyi")), (
        "_core.pyi stub missing; required by OMEP-0008 typing strategy"
    )
