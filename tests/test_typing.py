"""Static-typing smoke test for the public API (OMEP-0008).

This test runs the ``ty`` type checker over a snippet that exercises the frozen
public surface. It verifies that the shipped ``_core.pyi`` stub and the inline
hints in ``api.py`` actually type-check against real usage, going beyond the
``__all__`` string-membership checks in ``test_public_api.py``.

The test is skipped when the ``ty`` executable is not available so that the
core suite still runs in minimal environments.
"""

from __future__ import annotations

import shutil
import subprocess
import sys
import textwrap
from pathlib import Path

import pytest

# --- Snippet exercising the public surface -------------------------------------

# Every assignment carries an explicit annotation so that a wrong stub (e.g. a
# changed return type) surfaces as a type error rather than passing silently.
SNIPPET = textwrap.dedent(
    """
    from __future__ import annotations

    import oxydemark
    from oxydemark import (
        AstNode,
        Heading,
        OxydeEngine,
        ParseResult,
        Plugin,
        extract_summary,
        markdown_to_html,
        parse,
        parse_document,
        render_ast,
        slugify,
    )

    node: AstNode = parse("# Title")
    kind: str = node.kind
    children: list[AstNode] = node.children
    walked: list[AstNode] = node.walk()

    html: str = render_ast(node)
    fast: str = markdown_to_html("# Title")

    result: ParseResult = parse_document("---\\ntitle: x\\n---\\n# Title")
    root: AstNode = result.root
    frontmatter: dict[str, object] | None = result.frontmatter
    headings: list[Heading] = result.headings
    toc: list[Heading] = result.toc
    level: int = headings[0].level
    heading_id: str = headings[0].id
    heading_text: str = headings[0].text
    nested: list[Heading] = toc[0].children
    doc_summary: str | None = result.summary

    anchor: str = slugify("Some Heading", ["existing"])
    summary: str | None = extract_summary("intro\\n\\n<!-- more -->\\n\\nrest")

    manual: AstNode = AstNode("paragraph", children=[], text=None)

    engine: OxydeEngine = OxydeEngine()
    rendered: str = engine.render("# Title")

    plugin: Plugin | None = None

    # Real plugins implement a strict subset of the hooks, which must satisfy
    # `Plugin` -- this is what caught `Plugin` being a three-method protocol.
    from oxydemark.contrib import (
        AdmonitionPlugin,
        LazyImagesPlugin,
        MentionPlugin,
        ShortcodePlugin,
    )

    only_postprocess: Plugin = LazyImagesPlugin()
    only_transform: Plugin = MentionPlugin()
    configured: OxydeEngine = OxydeEngine(
        [AdmonitionPlugin(), ShortcodePlugin(), MentionPlugin(), LazyImagesPlugin()]
    )
    _ = configured.render("# Title")

    _ = oxydemark.__all__
    """
).strip()


# --- Test ----------------------------------------------------------------------


class TestPublicApiTypes:
    def test_public_api_type_checks_with_ty(self, tmp_path: Path) -> None:
        """The public API snippet must type-check cleanly under ty."""
        ty = shutil.which("ty")
        if ty is None:
            pytest.skip("ty type checker is not installed")

        snippet = tmp_path / "public_api_usage.py"
        snippet.write_text(SNIPPET, encoding="utf-8")

        # Point ty at the interpreter running the tests so it resolves the same
        # installed ``oxydemark`` package (and its shipped ``_core.pyi`` stub).
        result = subprocess.run(
            [
                ty,
                "check",
                "--python",
                sys.prefix,
                "--python-version",
                f"{sys.version_info.major}.{sys.version_info.minor}",
                "--output-format",
                "concise",
                str(snippet),
            ],
            capture_output=True,
            text=True,
        )

        assert result.returncode == 0, (
            "ty reported type errors against the public API stubs:\n"
            f"--- stdout ---\n{result.stdout}\n--- stderr ---\n{result.stderr}"
        )
