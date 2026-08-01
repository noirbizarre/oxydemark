"""Tests guarding the generated API reference (OMEP-0011).

The reference is produced by mkdocstrings/griffe from the docstrings in
``python/oxydemark`` and, for the native module, from ``_core.pyi``. These
tests are cheap guards against silently losing reference content; they do not
build the site.
"""

from __future__ import annotations

import inspect
import tomllib
from pathlib import Path

import pytest

import oxydemark
from oxydemark import contrib

REPO_ROOT = Path(__file__).resolve().parent.parent
DOCS_DIR = REPO_ROOT / "docs"


def _docstring(name: str, namespace: object = oxydemark) -> str | None:
    return inspect.getdoc(getattr(namespace, name))


@pytest.mark.parametrize("name", sorted(oxydemark.__all__))
def test_public_surface_is_documented(name: str) -> None:
    """Every frozen public name carries a non-empty docstring."""
    doc = _docstring(name)
    assert doc, f"oxydemark.{name} has no docstring"


@pytest.mark.parametrize("name", sorted(contrib.__all__))
def test_contrib_surface_is_documented(name: str) -> None:
    """Every provisional contrib plugin carries a non-empty docstring."""
    doc = _docstring(name, contrib)
    assert doc, f"oxydemark.contrib.{name} has no docstring"


@pytest.mark.parametrize("name", sorted(oxydemark.__all__))
def test_public_surface_uses_google_docstrings(name: str) -> None:
    """Docstrings use Google sections, never leftover reST or NumPy markup."""
    doc = _docstring(name) or ""
    for marker in (":class:", ":func:", ":meth:", ":mod:", ":data:", ".. important::"):
        assert marker not in doc, f"oxydemark.{name} still uses reST markup {marker!r}"
    assert "Parameters\n---" not in doc, f"oxydemark.{name} still uses a NumPy section"


@pytest.mark.parametrize(
    "page",
    ["index.md", "plugins.md", "api/index.md", "api/python.md", "api/rust.md"],
)
def test_documentation_pages_exist(page: str) -> None:
    """Pages referenced by the site navigation are present."""
    assert (DOCS_DIR / page).is_file()


def test_api_page_documents_the_whole_public_surface() -> None:
    """``docs/api/python.md`` has an mkdocstrings block per public name."""
    page = (DOCS_DIR / "api" / "python.md").read_text()
    for name in oxydemark.__all__:
        assert f"::: oxydemark.{name}" in page, f"oxydemark.{name} is missing from the API page"


def test_site_navigation_lists_every_omep() -> None:
    """Every OMEP in ``docs/specs`` is reachable from the site navigation."""
    config = tomllib.loads((REPO_ROOT / "zensical.toml").read_text())
    nav = str(config["project"]["nav"])
    for omep in sorted(DOCS_DIR.glob("specs/OMEP-*.md")):
        assert f"specs/{omep.name}" in nav, f"{omep.name} is missing from the navigation"
