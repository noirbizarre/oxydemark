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


# ---------------------------------------------------------------------------
# Comark: emoji AST
# ---------------------------------------------------------------------------


class TestEmojiAST:
    """Tests for emoji nodes in the AST."""

    def test_emoji_node_kind(self):
        ast = oxydemark.parse("Hello :wave:")
        nodes = ast.walk()
        emoji_nodes = [n for n in nodes if n.kind == "emoji"]
        assert len(emoji_nodes) >= 1

    def test_emoji_shortcode_attribute(self):
        ast = oxydemark.parse(":wave:")
        nodes = ast.walk()
        emoji = next(n for n in nodes if n.kind == "emoji")
        assert emoji.attributes["shortcode"] == "wave"

    def test_emoji_text_content(self):
        ast = oxydemark.parse(":wave:")
        nodes = ast.walk()
        emoji = next(n for n in nodes if n.kind == "emoji")
        assert emoji.text == "\U0001f44b"  # 👋

    def test_emoji_fast_path_render(self):
        html = oxydemark.markdown_to_html(":wave:")
        assert "\U0001f44b" in html

    def test_emoji_ast_round_trip(self):
        ast = oxydemark.parse(":wave:")
        html = oxydemark.render_ast(ast)
        assert "\U0001f44b" in html


# ---------------------------------------------------------------------------
# Comark: block components
# ---------------------------------------------------------------------------


class TestBlockComponents:
    """Tests for block component parsing and rendering."""

    def test_parse_block_component(self):
        ast = oxydemark.parse("::note\nSome content\n::")
        nodes = ast.walk()
        bc = next((n for n in nodes if n.kind == "block_component"), None)
        assert bc is not None
        assert bc.attributes["name"] == "note"

    def test_block_component_with_class(self):
        ast = oxydemark.parse("::warning{.alert}\nBe careful\n::")
        nodes = ast.walk()
        bc = next(n for n in nodes if n.kind == "block_component")
        assert bc.attributes["name"] == "warning"
        assert bc.attributes["class"] == "alert"

    def test_block_component_with_id(self):
        ast = oxydemark.parse("::note{#important}\nContent\n::")
        nodes = ast.walk()
        bc = next(n for n in nodes if n.kind == "block_component")
        assert bc.attributes["id"] == "important"

    def test_block_component_with_key_value(self):
        ast = oxydemark.parse('::note{type="info"}\nContent\n::')
        nodes = ast.walk()
        bc = next(n for n in nodes if n.kind == "block_component")
        assert bc.attributes["type"] == "info"

    def test_block_component_has_children(self):
        ast = oxydemark.parse("::note\nHello world\n::")
        nodes = ast.walk()
        bc = next(n for n in nodes if n.kind == "block_component")
        assert len(bc.children) > 0

    def test_block_component_fast_path_render(self):
        html = oxydemark.markdown_to_html("::note\nContent\n::")
        assert "<div" in html
        assert "</div>" in html

    def test_block_component_ast_round_trip(self):
        ast = oxydemark.parse("::note\nContent\n::")
        html = oxydemark.render_ast(ast)
        assert "<div" in html

    def test_triple_colon_not_block_component(self):
        ast = oxydemark.parse(":::note\nContent\n:::")
        nodes = ast.walk()
        assert all(n.kind != "block_component" for n in nodes)


# ---------------------------------------------------------------------------
# Comark: inline components
# ---------------------------------------------------------------------------


class TestInlineComponents:
    """Tests for inline component parsing and rendering."""

    def test_parse_inline_component_with_content(self):
        ast = oxydemark.parse(":icon[star]")
        nodes = ast.walk()
        ic = next((n for n in nodes if n.kind == "inline_component"), None)
        assert ic is not None
        assert ic.attributes["name"] == "icon"

    def test_inline_component_with_attrs(self):
        ast = oxydemark.parse(":badge[Pro]{.premium}")
        nodes = ast.walk()
        ic = next(n for n in nodes if n.kind == "inline_component")
        assert ic.attributes["name"] == "badge"
        assert ic.attributes["class"] == "premium"

    def test_inline_component_attrs_only(self):
        ast = oxydemark.parse(':icon{type="star"}')
        nodes = ast.walk()
        ic = next(n for n in nodes if n.kind == "inline_component")
        assert ic.attributes["name"] == "icon"
        assert ic.attributes["type"] == "star"

    def test_inline_component_fast_path_render(self):
        html = oxydemark.markdown_to_html(":badge[Pro]{.premium}")
        assert "<span" in html

    def test_inline_component_ast_round_trip(self):
        ast = oxydemark.parse(":badge[Pro]")
        html = oxydemark.render_ast(ast)
        assert "<span" in html


# ---------------------------------------------------------------------------
# Comark: span attributes
# ---------------------------------------------------------------------------


class TestSpanAttributes:
    """Tests for span attribute parsing and rendering."""

    def test_parse_span_with_class(self):
        ast = oxydemark.parse("[important]{.highlight}")
        nodes = ast.walk()
        span = next((n for n in nodes if n.kind == "span_attributes"), None)
        assert span is not None
        assert span.attributes["class"] == "highlight"

    def test_span_with_id(self):
        ast = oxydemark.parse("[text]{#myid}")
        nodes = ast.walk()
        span = next(n for n in nodes if n.kind == "span_attributes")
        assert span.attributes["id"] == "myid"

    def test_span_with_multiple_classes(self):
        ast = oxydemark.parse("[text]{.a .b .c}")
        nodes = ast.walk()
        span = next(n for n in nodes if n.kind == "span_attributes")
        class_attr = span.attributes["class"]
        assert "a" in class_attr
        assert "b" in class_attr
        assert "c" in class_attr

    def test_span_fast_path_render(self):
        html = oxydemark.markdown_to_html("[highlighted]{.mark}")
        assert "<span" in html
        assert 'class="mark"' in html

    def test_span_ast_round_trip(self):
        ast = oxydemark.parse("[highlighted]{.mark}")
        html = oxydemark.render_ast(ast)
        assert "<span" in html
