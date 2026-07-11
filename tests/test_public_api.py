"""Tests enforcing the public API contract frozen in OMEP-0008.

If these tests fail, the public surface has changed and OMEP-0008 must be
updated together with a breaking-change version bump (see the semver policy).
"""

from __future__ import annotations

import importlib

import oxydemark

# The frozen public Python surface, per OMEP-0008.
PUBLIC_SURFACE = frozenset(
    {
        "AstNode",
        "OxydeEngine",
        "Plugin",
        "markdown_to_html",
        "parse",
        "render_ast",
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
    spec = importlib.util.find_spec("oxydemark")
    assert spec is not None
    assert spec.submodule_search_locations is not None
    package_dir = spec.submodule_search_locations[0]
    import os

    assert os.path.exists(os.path.join(package_dir, "py.typed")), (
        "py.typed marker missing; required by OMEP-0008 typing strategy"
    )
