"""Tests for the high-level Python API (OxydeEngine and plugins)."""

from __future__ import annotations

import oxydemark
from oxydemark.api import OxydeEngine


# ---------------------------------------------------------------------------
# OxydeEngine basics
# ---------------------------------------------------------------------------


class TestOxydeEngine:
    """Tests for OxydeEngine without plugins."""

    def test_render_simple(self):
        engine = OxydeEngine()
        html = engine.render("Hello")
        assert "<p>" in html
        assert "Hello" in html

    def test_render_heading(self):
        engine = OxydeEngine()
        html = engine.render("# Title")
        assert "<h1" in html
        assert "Title" in html

    def test_render_empty(self):
        engine = OxydeEngine()
        html = engine.render("")
        assert html.strip() == ""

    def test_render_complex_markdown(self):
        engine = OxydeEngine()
        md = (
            "# Heading\n\nA paragraph with **bold** and *italic*.\n\n- item 1\n- item 2"
        )
        html = engine.render(md)
        assert "<h1" in html
        assert "<strong>" in html
        assert "<em>" in html
        assert "<li>" in html

    def test_no_plugins_by_default(self):
        engine = OxydeEngine()
        assert engine.plugins == []

    def test_accepts_plugin_list(self):
        class DummyPlugin:
            pass

        engine = OxydeEngine(plugins=[DummyPlugin()])
        assert len(engine.plugins) == 1


# ---------------------------------------------------------------------------
# Preprocess plugins
# ---------------------------------------------------------------------------


class TestPreprocessPlugin:
    """Tests for plugins that implement preprocess()."""

    def test_preprocess_modifies_text(self):
        class UpperPlugin:
            def preprocess(self, markdown: str) -> str:
                return markdown.upper()

        engine = OxydeEngine(plugins=[UpperPlugin()])
        html = engine.render("hello")
        assert "HELLO" in html

    def test_preprocess_replaces_content(self):
        class ReplacePlugin:
            def preprocess(self, markdown: str) -> str:
                return markdown.replace("foo", "bar")

        engine = OxydeEngine(plugins=[ReplacePlugin()])
        html = engine.render("foo")
        assert "bar" in html
        assert "foo" not in html

    def test_multiple_preprocess_plugins_chain(self):
        class AddPrefix:
            def preprocess(self, markdown: str) -> str:
                return f"PREFIX {markdown}"

        class AddSuffix:
            def preprocess(self, markdown: str) -> str:
                return f"{markdown} SUFFIX"

        engine = OxydeEngine(plugins=[AddPrefix(), AddSuffix()])
        html = engine.render("middle")
        assert "PREFIX" in html
        assert "SUFFIX" in html
        assert "middle" in html


# ---------------------------------------------------------------------------
# Transform plugins (AST-level)
# ---------------------------------------------------------------------------


class TestTransformPlugin:
    """Tests for plugins that implement transform()."""

    def test_transform_modifies_text_nodes(self):
        class ReplaceText:
            def transform(self, ast: oxydemark.AstNode) -> oxydemark.AstNode:
                self._replace(ast)
                return ast

            def _replace(self, node: oxydemark.AstNode) -> None:
                if node.kind == "text" and node.text:
                    node.text = node.text.replace("hello", "goodbye")
                # Must re-read children, modify clones, and reassign.
                children = node.children
                for child in children:
                    self._replace(child)
                node.children = children

        engine = OxydeEngine(plugins=[ReplaceText()])
        html = engine.render("hello world")
        assert "goodbye" in html
        assert "hello" not in html

    def test_transform_adds_children(self):
        class AddFootnote:
            def transform(self, ast: oxydemark.AstNode) -> oxydemark.AstNode:
                text_node = oxydemark.AstNode(kind="text", text="[footnote]")
                para = oxydemark.AstNode(kind="paragraph", children=[text_node])
                # Must reassign children (append to a copy, then set).
                children = ast.children
                children.append(para)
                ast.children = children
                return ast

        engine = OxydeEngine(plugins=[AddFootnote()])
        html = engine.render("Main content")
        assert "Main content" in html
        assert "[footnote]" in html

    def test_transform_can_filter_nodes(self):
        """A transform that removes emphasis nodes."""

        class RemoveEmphasis:
            def transform(self, ast: oxydemark.AstNode) -> oxydemark.AstNode:
                self._strip(ast)
                return ast

            def _strip(self, node: oxydemark.AstNode) -> None:
                new_children = []
                for child in node.children:
                    if child.kind == "emphasis":
                        # Replace emphasis with its text children.
                        new_children.extend(child.children)
                    else:
                        self._strip(child)
                        new_children.append(child)
                node.children = new_children

        engine = OxydeEngine(plugins=[RemoveEmphasis()])
        html = engine.render("*emphasized* text")
        assert "<em>" not in html
        assert "emphasized" in html
        assert "text" in html


# ---------------------------------------------------------------------------
# Postprocess plugins
# ---------------------------------------------------------------------------


class TestPostprocessPlugin:
    """Tests for plugins that implement postprocess()."""

    def test_postprocess_modifies_html(self):
        class WrapDiv:
            def postprocess(self, html: str) -> str:
                return f"<div class='wrapper'>{html}</div>"

        engine = OxydeEngine(plugins=[WrapDiv()])
        html = engine.render("Hello")
        assert html.startswith("<div class='wrapper'>")
        assert html.endswith("</div>")
        assert "Hello" in html

    def test_postprocess_replaces_tags(self):
        class ReplaceStrong:
            def postprocess(self, html: str) -> str:
                return html.replace("<strong>", "<b>").replace("</strong>", "</b>")

        engine = OxydeEngine(plugins=[ReplaceStrong()])
        html = engine.render("**bold**")
        assert "<b>" in html
        assert "<strong>" not in html


# ---------------------------------------------------------------------------
# Mixed plugins (multiple hooks)
# ---------------------------------------------------------------------------


class TestMixedPlugins:
    """Tests for plugins implementing multiple hooks and plugin ordering."""

    def test_plugin_with_all_hooks(self):
        class FullPlugin:
            def preprocess(self, markdown: str) -> str:
                return markdown.replace("INPUT", "processed")

            def transform(self, ast: oxydemark.AstNode) -> oxydemark.AstNode:
                self._upper(ast)
                return ast

            def _upper(self, node: oxydemark.AstNode) -> None:
                if node.kind == "text" and node.text:
                    node.text = node.text.upper()
                children = node.children
                for child in children:
                    self._upper(child)
                node.children = children

            def postprocess(self, html: str) -> str:
                return f"<!-- generated -->\n{html}"

        engine = OxydeEngine(plugins=[FullPlugin()])
        html = engine.render("INPUT")
        assert "PROCESSED" in html
        assert html.startswith("<!-- generated -->")

    def test_plugin_ordering_matters(self):
        class Plugin1:
            def preprocess(self, markdown: str) -> str:
                return markdown + " [from-p1]"

        class Plugin2:
            def preprocess(self, markdown: str) -> str:
                return markdown + " [from-p2]"

        engine = OxydeEngine(plugins=[Plugin1(), Plugin2()])
        html = engine.render("base")
        # Both plugins should have added their text.
        assert "[from-p1]" in html
        assert "[from-p2]" in html

    def test_partial_plugin_preprocess_only(self):
        """A plugin with only preprocess should not break the pipeline."""

        class PreOnly:
            def preprocess(self, markdown: str) -> str:
                return markdown.replace("X", "Y")

        engine = OxydeEngine(plugins=[PreOnly()])
        html = engine.render("X marks the spot")
        assert "Y" in html

    def test_partial_plugin_postprocess_only(self):
        """A plugin with only postprocess should not break the pipeline."""

        class PostOnly:
            def postprocess(self, html: str) -> str:
                return html.replace("Hello", "Goodbye")

        engine = OxydeEngine(plugins=[PostOnly()])
        html = engine.render("Hello")
        assert "Goodbye" in html

    def test_mixed_partial_plugins(self):
        class PrePlugin:
            def preprocess(self, markdown: str) -> str:
                return f"**{markdown}**"

        class PostPlugin:
            def postprocess(self, html: str) -> str:
                return html.strip()

        engine = OxydeEngine(plugins=[PrePlugin(), PostPlugin()])
        html = engine.render("text")
        assert "<strong>" in html
        # Postprocess stripped whitespace.
        assert not html.endswith("\n")
