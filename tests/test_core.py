"""Tests for the native Rust module exposed via oxydemark._core."""

from __future__ import annotations

import pytest

import oxydemark


# ---------------------------------------------------------------------------
# parse()
# ---------------------------------------------------------------------------


class TestParse:
    """Tests for the parse() function."""

    def test_returns_document_node(self):
        ast = oxydemark.parse("Hello")
        assert ast.kind == "document"

    def test_simple_paragraph(self):
        ast = oxydemark.parse("Hello")
        para = ast.children[0]
        assert para.kind == "paragraph"
        text_node = para.children[0]
        assert text_node.kind == "text"
        assert text_node.text == "Hello"

    def test_heading(self):
        ast = oxydemark.parse("# Title")
        heading = ast.children[0]
        assert heading.kind == "heading"
        assert heading.attributes["level"] == "1"

    def test_heading_levels(self):
        for level in range(1, 7):
            ast = oxydemark.parse(f"{'#' * level} H{level}")
            heading = ast.children[0]
            assert heading.attributes["level"] == str(level)

    def test_emphasis(self):
        ast = oxydemark.parse("*emphasized*")
        para = ast.children[0]
        kinds = [c.kind for c in para.children]
        assert "emphasis" in kinds

    def test_strong(self):
        ast = oxydemark.parse("**bold**")
        para = ast.children[0]
        kinds = [c.kind for c in para.children]
        assert "strong" in kinds

    def test_link(self):
        ast = oxydemark.parse("[click](https://example.com)")
        para = ast.children[0]
        link = next(c for c in para.children if c.kind == "link")
        assert link.attributes["href"] == "https://example.com"
        text_node = next(c for c in link.children if c.kind == "text")
        assert text_node.text == "click"

    def test_link_with_title(self):
        ast = oxydemark.parse('[click](https://example.com "a title")')
        para = ast.children[0]
        link = next(c for c in para.children if c.kind == "link")
        assert link.attributes["title"] == "a title"

    def test_image(self):
        ast = oxydemark.parse("![alt](image.png)")
        para = ast.children[0]
        img = next(c for c in para.children if c.kind == "image")
        assert img.attributes["src"] == "image.png"

    def test_code_span(self):
        ast = oxydemark.parse("Use `code` here")
        para = ast.children[0]
        kinds = [c.kind for c in para.children]
        assert "code_span" in kinds

    def test_blockquote(self):
        ast = oxydemark.parse("> quoted")
        assert ast.children[0].kind == "blockquote"

    def test_unordered_list(self):
        ast = oxydemark.parse("- one\n- two\n- three")
        list_node = ast.children[0]
        assert list_node.kind == "list"
        items = [c for c in list_node.children if c.kind == "list_item"]
        assert len(items) == 3

    def test_code_block(self):
        ast = oxydemark.parse("```\ncode\n```")
        kinds = [c.kind for c in ast.children]
        assert "code_block" in kinds

    def test_thematic_break(self):
        ast = oxydemark.parse("***")
        kinds = [c.kind for c in ast.children]
        assert "thematic_break" in kinds

    def test_strikethrough(self):
        ast = oxydemark.parse("~~deleted~~")
        para = ast.children[0]
        kinds = [c.kind for c in para.children]
        assert "strikethrough" in kinds

    def test_table(self):
        ast = oxydemark.parse("| A | B |\n|---|---|\n| 1 | 2 |")
        kinds = [c.kind for c in ast.children]
        assert "table" in kinds

    def test_empty_input(self):
        ast = oxydemark.parse("")
        assert ast.kind == "document"

    def test_multiple_paragraphs(self):
        ast = oxydemark.parse("Para one\n\nPara two")
        paragraphs = [c for c in ast.children if c.kind == "paragraph"]
        assert len(paragraphs) == 2

    def test_frontmatter(self):
        ast = oxydemark.parse("---\ntitle: Hello\n---\n\nContent")
        assert ast.metadata is not None
        assert ast.metadata["title"] == "Hello"

    def test_frontmatter_multiple_keys(self):
        md = "---\ntitle: Hello\nauthor: World\n---\n\nContent"
        ast = oxydemark.parse(md)
        assert ast.metadata["title"] == "Hello"
        assert ast.metadata["author"] == "World"

    def test_no_frontmatter(self):
        ast = oxydemark.parse("Just text")
        assert ast.metadata is None


# ---------------------------------------------------------------------------
# AstNode
# ---------------------------------------------------------------------------


class TestAstNode:
    """Tests for AstNode construction and behavior."""

    def test_construct_with_defaults(self):
        node = oxydemark.AstNode(kind="test")
        assert node.kind == "test"
        assert node.children == []
        assert node.text is None
        assert node.attributes == {}
        assert node.metadata is None

    def test_construct_with_all_fields(self):
        child = oxydemark.AstNode(kind="text", text="hello")
        node = oxydemark.AstNode(
            kind="paragraph",
            children=[child],
            text=None,
            attributes={"class": "intro"},
            metadata={"key": "value"},
        )
        assert node.kind == "paragraph"
        assert len(node.children) == 1
        assert node.children[0].text == "hello"
        assert node.attributes["class"] == "intro"
        assert node.metadata["key"] == "value"

    def test_fields_are_mutable(self):
        node = oxydemark.AstNode(kind="text", text="before")
        node.text = "after"
        assert node.text == "after"
        node.kind = "modified"
        assert node.kind == "modified"

    def test_repr(self):
        node = oxydemark.AstNode(kind="text", text="hello")
        r = repr(node)
        assert "text" in r
        assert "hello" in r

    def test_repr_no_text(self):
        child = oxydemark.AstNode(kind="text")
        node = oxydemark.AstNode(kind="document", children=[child])
        r = repr(node)
        assert "document" in r
        assert "children=1" in r


# ---------------------------------------------------------------------------
# walk()
# ---------------------------------------------------------------------------


class TestWalk:
    """Tests for AstNode.walk()."""

    def test_walk_returns_list(self):
        ast = oxydemark.parse("Hello")
        nodes = ast.walk()
        assert isinstance(nodes, list)
        assert len(nodes) >= 3  # document, paragraph, text

    def test_walk_includes_root(self):
        ast = oxydemark.parse("Hello")
        nodes = ast.walk()
        assert nodes[0].kind == "document"

    def test_walk_depth_first_order(self):
        ast = oxydemark.parse("**bold**")
        kinds = [n.kind for n in ast.walk()]
        assert kinds == ["document", "paragraph", "strong", "text"]

    def test_walk_finds_all_text_nodes(self):
        ast = oxydemark.parse("Hello **world** and *more*")
        text_nodes = [n for n in ast.walk() if n.kind == "text"]
        assert len(text_nodes) >= 3

    def test_walk_on_leaf_node(self):
        node = oxydemark.AstNode(kind="text", text="leaf")
        nodes = node.walk()
        assert len(nodes) == 1
        assert nodes[0].kind == "text"


# ---------------------------------------------------------------------------
# markdown_to_html()
# ---------------------------------------------------------------------------


class TestMarkdownToHtml:
    """Tests for the fast-path markdown_to_html()."""

    def test_simple_paragraph(self):
        html = oxydemark.markdown_to_html("Hello")
        assert "<p>" in html
        assert "Hello" in html

    def test_heading(self):
        html = oxydemark.markdown_to_html("# Title")
        assert "<h1" in html
        assert "Title" in html

    def test_emphasis(self):
        html = oxydemark.markdown_to_html("*em*")
        assert "<em>" in html

    def test_strong(self):
        html = oxydemark.markdown_to_html("**bold**")
        assert "<strong>" in html

    def test_link(self):
        html = oxydemark.markdown_to_html("[text](https://example.com)")
        assert "<a" in html
        assert "https://example.com" in html

    def test_image(self):
        html = oxydemark.markdown_to_html("![alt](pic.png)")
        assert "<img" in html
        assert "pic.png" in html

    def test_code_block(self):
        html = oxydemark.markdown_to_html("```\ncode\n```")
        assert "<code>" in html

    def test_inline_code(self):
        html = oxydemark.markdown_to_html("Use `code` here")
        assert "<code>" in html

    def test_blockquote(self):
        html = oxydemark.markdown_to_html("> quote")
        assert "<blockquote>" in html

    def test_list(self):
        html = oxydemark.markdown_to_html("- a\n- b")
        assert "<li>" in html

    def test_thematic_break(self):
        html = oxydemark.markdown_to_html("***")
        assert "<hr" in html

    def test_strikethrough(self):
        html = oxydemark.markdown_to_html("~~deleted~~")
        assert "<del>" in html

    def test_table(self):
        html = oxydemark.markdown_to_html("| A | B |\n|---|---|\n| 1 | 2 |")
        assert "<table>" in html
        assert "<td>" in html

    def test_empty_input(self):
        html = oxydemark.markdown_to_html("")
        assert html.strip() == ""


# ---------------------------------------------------------------------------
# render() (legacy)
# ---------------------------------------------------------------------------


class TestRenderLegacy:
    """Tests for the legacy render() function."""

    def test_basic(self):
        html = oxydemark.render("Hello")
        assert "Hello" in html

    def test_equivalent_to_markdown_to_html(self):
        md = "# Test\n\nSome *text* with **bold**."
        assert oxydemark.render(md) == oxydemark.markdown_to_html(md)


# ---------------------------------------------------------------------------
# render_ast()
# ---------------------------------------------------------------------------


class TestRenderAst:
    """Tests for render_ast() (AST -> HTML path)."""

    def test_round_trip_paragraph(self):
        ast = oxydemark.parse("Hello")
        html = oxydemark.render_ast(ast)
        assert "<p>" in html
        assert "Hello" in html

    def test_round_trip_heading(self):
        ast = oxydemark.parse("# Title")
        html = oxydemark.render_ast(ast)
        assert "<h1" in html
        assert "Title" in html

    def test_round_trip_link(self):
        ast = oxydemark.parse("[click](https://example.com)")
        html = oxydemark.render_ast(ast)
        assert '<a href="https://example.com"' in html
        assert "click" in html

    def test_round_trip_emphasis(self):
        ast = oxydemark.parse("*em*")
        html = oxydemark.render_ast(ast)
        assert "<em>" in html

    def test_round_trip_strong(self):
        ast = oxydemark.parse("**bold**")
        html = oxydemark.render_ast(ast)
        assert "<strong>" in html

    def test_round_trip_code_block(self):
        ast = oxydemark.parse("```\ncode\n```")
        html = oxydemark.render_ast(ast)
        assert "<pre><code>" in html

    def test_round_trip_blockquote(self):
        ast = oxydemark.parse("> quote")
        html = oxydemark.render_ast(ast)
        assert "<blockquote>" in html

    def test_round_trip_list(self):
        ast = oxydemark.parse("- x\n- y")
        html = oxydemark.render_ast(ast)
        assert "<li>" in html

    def test_round_trip_strikethrough(self):
        ast = oxydemark.parse("~~gone~~")
        html = oxydemark.render_ast(ast)
        assert "<del>" in html
        assert "gone" in html

    def test_round_trip_table(self):
        ast = oxydemark.parse("| A | B |\n|---|---|\n| 1 | 2 |")
        html = oxydemark.render_ast(ast)
        assert "<table>" in html
        assert "<td>" in html

    def test_render_synthetic_ast(self):
        """Build an AST from Python and render it."""
        text_node = oxydemark.AstNode(kind="text", text="hello world")
        para = oxydemark.AstNode(kind="paragraph", children=[text_node])
        doc = oxydemark.AstNode(kind="document", children=[para])
        html = oxydemark.render_ast(doc)
        assert "<p>" in html
        assert "hello world" in html

    def test_render_synthetic_heading(self):
        text_node = oxydemark.AstNode(kind="text", text="My Title")
        heading = oxydemark.AstNode(
            kind="heading",
            children=[text_node],
            attributes={"level": "2"},
        )
        doc = oxydemark.AstNode(kind="document", children=[heading])
        html = oxydemark.render_ast(doc)
        assert "<h2" in html
        assert "My Title" in html

    def test_render_synthetic_link(self):
        text_node = oxydemark.AstNode(kind="text", text="link text")
        link = oxydemark.AstNode(
            kind="link",
            children=[text_node],
            attributes={"href": "https://example.com"},
        )
        para = oxydemark.AstNode(kind="paragraph", children=[link])
        doc = oxydemark.AstNode(kind="document", children=[para])
        html = oxydemark.render_ast(doc)
        assert '<a href="https://example.com"' in html
        assert "link text" in html
