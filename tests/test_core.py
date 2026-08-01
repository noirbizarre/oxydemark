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
# parse_document / ParseResult (OMEP-0010)
# ---------------------------------------------------------------------------


class TestParseDocument:
    """Tests for the typed frontmatter accessor on the parse result."""

    def test_no_frontmatter(self):
        result = oxydemark.parse_document("Just text")
        assert result.frontmatter is None

    def test_root_is_document(self):
        result = oxydemark.parse_document("---\ntitle: Hi\n---\n\nBody")
        assert result.root.kind == "document"
        # The tree is reachable and equivalent to parse().
        kinds = [c.kind for c in result.root.children]
        assert "paragraph" in kinds

    def test_string_value(self):
        result = oxydemark.parse_document("---\ntitle: Hello\n---\n\nContent")
        assert result.frontmatter is not None
        assert result.frontmatter["title"] == "Hello"
        assert isinstance(result.frontmatter["title"], str)

    def test_int_value_preserves_type(self):
        result = oxydemark.parse_document("---\ncount: 5\n---\n\nContent")
        assert result.frontmatter["count"] == 5
        assert isinstance(result.frontmatter["count"], int)
        assert not isinstance(result.frontmatter["count"], bool)

    def test_float_value_preserves_type(self):
        result = oxydemark.parse_document("---\nratio: 1.5\n---\n\nContent")
        assert result.frontmatter["ratio"] == 1.5
        assert isinstance(result.frontmatter["ratio"], float)

    def test_bool_value_preserves_type(self):
        result = oxydemark.parse_document("---\ndraft: true\n---\n\nContent")
        assert result.frontmatter["draft"] is True

    def test_null_value(self):
        result = oxydemark.parse_document("---\nsubtitle: null\n---\n\nContent")
        assert result.frontmatter["subtitle"] is None

    def test_list_value(self):
        md = "---\ntags:\n  - a\n  - b\n---\n\nContent"
        result = oxydemark.parse_document(md)
        assert result.frontmatter["tags"] == ["a", "b"]

    def test_nested_mapping(self):
        md = "---\nauthor:\n  name: Ada\n  age: 36\n---\n\nContent"
        result = oxydemark.parse_document(md)
        author = result.frontmatter["author"]
        assert author == {"name": "Ada", "age": 36}
        assert isinstance(author["age"], int)

    def test_multiple_keys_preserve_order(self):
        md = "---\ntitle: Hello\ncount: 3\ndraft: false\n---\n\nContent"
        result = oxydemark.parse_document(md)
        assert list(result.frontmatter.keys()) == ["title", "count", "draft"]
        assert result.frontmatter["count"] == 3
        assert result.frontmatter["draft"] is False


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

    def test_triple_colon_block_component(self):
        ast = oxydemark.parse(":::note\nContent\n:::")
        nodes = ast.walk()
        bc = next(n for n in nodes if n.kind == "block_component")
        assert bc.attributes["name"] == "note"

    def test_nested_components(self):
        ast = oxydemark.parse(":::outer\n::inner\nContent\n::\n:::")
        outer = next(n for n in ast.children if n.kind == "block_component")
        assert outer.attributes["name"] == "outer"

        inner = outer.children[0]
        assert inner.kind == "block_component"
        assert inner.attributes["name"] == "inner"
        assert inner.children[0].kind == "paragraph"

    def test_nested_components_deep(self):
        ast = oxydemark.parse(
            "::level-1\n:::level-2\n::::level-3\nContent\n::::\n:::\n::"
        )
        level1 = next(n for n in ast.children if n.kind == "block_component")
        level2 = level1.children[0]
        level3 = level2.children[0]
        assert [n.attributes["name"] for n in (level1, level2, level3)] == [
            "level-1",
            "level-2",
            "level-3",
        ]

    def test_nested_components_render(self):
        html = oxydemark.markdown_to_html(":::outer\n::inner\nx\n::\n:::")
        assert "<div>\n<div>\n<p>x</p>\n</div>\n</div>" in html

    def _block_component(self, ast):
        return next(n for n in ast.walk() if n.kind == "block_component")

    def test_props_none_by_default(self):
        ast = oxydemark.parse("::note\nContent\n::")
        assert self._block_component(ast).props is None

    def test_props_none_when_inline_only(self):
        ast = oxydemark.parse("::note{.info}\nBody\n::")
        bc = self._block_component(ast)
        assert bc.props is None
        assert bc.attributes["class"] == "info"

    def test_props_frontmatter_style_native_types(self):
        ast = oxydemark.parse(
            "::card\n---\nvariant: elevated\ncount: 42\nenabled: true\n---\nBody\n::"
        )
        props = self._block_component(ast).props
        assert props is not None
        assert props["variant"] == "elevated"
        assert props["count"] == 42
        assert isinstance(props["count"], int)
        assert not isinstance(props["count"], bool)
        assert props["enabled"] is True

    def test_props_codeblock_style(self):
        ast = oxydemark.parse(
            "::card\n```yaml [props]\nvariant: elevated\ncount: 42\n```\nBody\n::"
        )
        props = self._block_component(ast).props
        assert props is not None
        assert props["variant"] == "elevated"
        assert props["count"] == 42

    def test_props_typed_sequences_and_mappings(self):
        ast = oxydemark.parse(
            "::card\n---\ntags:\n  - a\n  - b\nobj:\n  k: v\n---\n::"
        )
        props = self._block_component(ast).props
        assert props is not None
        assert props["tags"] == ["a", "b"]
        assert props["obj"] == {"k": "v"}

    def test_props_inline_attribute_takes_precedence(self):
        ast = oxydemark.parse(
            '::card{variant="inline"}\n---\nvariant: yaml\ntitle: T\n---\n::'
        )
        bc = self._block_component(ast)
        # Inline attribute wins and lives in `attributes`.
        assert bc.attributes["variant"] == "inline"
        # The colliding key is dropped from typed props.
        assert bc.props is not None
        assert "variant" not in bc.props
        assert bc.props["title"] == "T"

    def test_props_is_read_only(self):
        ast = oxydemark.parse("::card\n---\nvariant: x\n---\n::")
        bc = self._block_component(ast)
        with pytest.raises(AttributeError):
            bc.props = {"other": 1}

    def test_props_not_emitted_as_html(self):
        html = oxydemark.markdown_to_html(
            "::card{.featured}\n---\ntitle: Secret\n---\nBody\n::"
        )
        assert 'class="featured"' in html
        assert "title" not in html


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


class TestHeadingAnchors:
    """Tests for deterministic heading anchor/slug ids."""

    def test_heading_gets_slug_id(self):
        ast = oxydemark.parse("# Overview")
        heading = ast.children[0]
        assert heading.kind == "heading"
        assert heading.attributes["id"] == "overview"

    def test_multi_word_heading_slug(self):
        ast = oxydemark.parse("# Hello World")
        assert ast.children[0].attributes["id"] == "hello-world"

    def test_duplicate_headings_get_suffixes(self):
        ast = oxydemark.parse("## Overview\n\n## Overview\n\n## Overview")
        headings = [c for c in ast.children if c.kind == "heading"]
        ids = [h.attributes["id"] for h in headings]
        assert ids == ["overview", "overview-1", "overview-2"]

    def test_author_provided_id_wins(self):
        ast = oxydemark.parse("## Title {#custom}\n\n## Custom")
        headings = [c for c in ast.children if c.kind == "heading"]
        assert headings[0].attributes["id"] == "custom"
        assert headings[1].attributes["id"] == "custom-1"

    def test_unicode_heading_normalized(self):
        ast = oxydemark.parse("# Café")
        assert ast.children[0].attributes["id"] == "cafe"

    def test_punctuation_only_falls_back_to_section(self):
        ast = oxydemark.parse("# ...")
        assert ast.children[0].attributes["id"] == "section"

    def test_fast_path_emits_id(self):
        html = oxydemark.markdown_to_html("# Title")
        assert 'id="title"' in html

    def test_ast_round_trip_emits_id(self):
        ast = oxydemark.parse("# Overview")
        html = oxydemark.render_ast(ast)
        assert 'id="overview"' in html


class TestSlugify:
    """Tests for the public slugify() function."""

    def test_basic(self):
        assert oxydemark.slugify("Hello World") == "hello-world"

    def test_unicode(self):
        assert oxydemark.slugify("Café") == "cafe"

    def test_empty_falls_back(self):
        assert oxydemark.slugify("...") == "section"

    def test_disambiguation_with_existing(self):
        assert oxydemark.slugify("Overview", ["overview"]) == "overview-1"

    def test_no_collision_without_existing(self):
        assert oxydemark.slugify("Overview") == "overview"


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


class TestSummary:
    """Tests for the public extract_summary() function (OMEP-0010)."""

    def test_splits_at_delimiter(self):
        src = "Intro paragraph shown in listings.\n\n<!-- more -->\n\nThe rest."
        summary = oxydemark.extract_summary(src)
        assert summary is not None
        assert "<p>Intro paragraph shown in listings.</p>" in summary
        assert "The rest" not in summary

    def test_none_without_delimiter(self):
        assert oxydemark.extract_summary("Just a plain paragraph.") is None

    def test_whitespace_and_case_tolerant(self):
        for delimiter in ("<!--more-->", "<!--   MORE   -->", "<!-- More -->"):
            summary = oxydemark.extract_summary(f"Intro.\n\n{delimiter}\n\nBody.")
            assert summary is not None
            assert "<p>Intro.</p>" in summary
            assert "Body" not in summary

    def test_ignores_nested_delimiter(self):
        src = "> Intro.\n>\n> <!-- more -->\n\nBody."
        assert oxydemark.extract_summary(src) is None

    def test_uses_first_top_level_delimiter(self):
        src = "First.\n\n<!-- more -->\n\nSecond.\n\n<!-- more -->\n\nThird."
        summary = oxydemark.extract_summary(src)
        assert summary is not None
        assert "<p>First.</p>" in summary
        assert "Second" not in summary
        assert "Third" not in summary

    def test_matches_render_ast_prefix(self):
        src = "# Heading\n\nIntro.\n\n<!-- more -->\n\nBody."
        summary = oxydemark.extract_summary(src)
        expected = oxydemark.render_ast(oxydemark.parse("# Heading\n\nIntro."))
        assert summary == expected

    def test_empty_when_delimiter_is_first_block(self):
        assert oxydemark.extract_summary("<!-- more -->\n\nBody.") == ""


# ---------------------------------------------------------------------------
# Headings / table of contents (OMEP-0010)
# ---------------------------------------------------------------------------


TOC_SOURCE = "# Title\n\n## Setup\n\n## Usage\n\n### CLI\n\n### Library\n\n## FAQ"


class TestHeadings:
    """Tests for the flat ParseResult.headings list (OMEP-0010)."""

    def test_flat_list_in_document_order(self):
        result = oxydemark.parse_document(TOC_SOURCE)
        assert [(h.level, h.id) for h in result.headings] == [
            (1, "title"),
            (2, "setup"),
            (2, "usage"),
            (3, "cli"),
            (3, "library"),
            (2, "faq"),
        ]

    def test_flat_entries_have_no_children(self):
        result = oxydemark.parse_document(TOC_SOURCE)
        assert all(h.children == [] for h in result.headings)

    def test_text_is_the_plain_label(self):
        result = oxydemark.parse_document("## Hello **world**")
        assert result.headings[0].text == "Hello world"
        assert result.headings[0].id == "hello-world"

    def test_empty_without_headings(self):
        result = oxydemark.parse_document("Just a paragraph.")
        assert result.headings == []
        assert result.toc == []

    def test_ids_match_rendered_html(self):
        src = "# Overview\n\n## Overview"
        result = oxydemark.parse_document(src)
        html = oxydemark.markdown_to_html(src)
        assert [h.id for h in result.headings] == ["overview", "overview-1"]
        for heading in result.headings:
            assert f'id="{heading.id}"' in html

    def test_author_provided_id_is_used(self):
        result = oxydemark.parse_document("# Title {#custom}")
        assert result.headings[0].id == "custom"

    def test_headings_inside_block_components(self):
        src = "# Title\n\n::note\n## Inside\n::\n\n## After"
        result = oxydemark.parse_document(src)
        assert [h.id for h in result.headings] == ["title", "inside", "after"]

    def test_repr(self):
        result = oxydemark.parse_document("# Title")
        assert "Heading(" in repr(result.headings[0])


class TestToc:
    """Tests for the nested ParseResult.toc tree (OMEP-0010)."""

    def test_nesting_matches_spec_example(self):
        result = oxydemark.parse_document(TOC_SOURCE)
        assert [h.id for h in result.toc] == ["title"]
        assert [c.id for c in result.toc[0].children] == ["setup", "usage", "faq"]
        assert [c.id for c in result.toc[0].children[1].children] == ["cli", "library"]

    def test_level_skips_are_tolerated(self):
        result = oxydemark.parse_document("# Title\n\n### Deep")
        assert [c.id for c in result.toc[0].children] == ["deep"]
        assert result.toc[0].children[0].level == 3

    def test_multiple_roots(self):
        result = oxydemark.parse_document("# One\n\n# Two\n\n## Two One")
        assert [h.id for h in result.toc] == ["one", "two"]
        assert result.toc[0].children == []
        assert [c.id for c in result.toc[1].children] == ["two-one"]

    def test_shallower_heading_closes_ancestors(self):
        src = "## A\n\n#### A1\n\n### A2\n\n## B"
        result = oxydemark.parse_document(src)
        assert [h.id for h in result.toc] == ["a", "b"]
        assert [c.id for c in result.toc[0].children] == ["a1", "a2"]


class TestParseResultSummary:
    """Tests for the summary folded into parse_document (OMEP-0010)."""

    def test_matches_extract_summary(self):
        src = "Intro.\n\n<!-- more -->\n\nBody."
        result = oxydemark.parse_document(src)
        assert result.summary == oxydemark.extract_summary(src)
        assert result.summary is not None
        assert "<p>Intro.</p>" in result.summary

    def test_none_without_delimiter(self):
        assert oxydemark.parse_document("No delimiter.").summary is None

    def test_root_still_renders_full_document(self):
        result = oxydemark.parse_document("Intro.\n\n<!-- more -->\n\nBody.")
        html = oxydemark.render_ast(result.root)
        assert "Intro." in html
        assert "Body." in html
