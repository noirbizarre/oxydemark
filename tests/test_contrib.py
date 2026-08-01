"""Tests for the example plugins shipped in :mod:`oxydemark.contrib`.

``oxydemark.contrib`` is a provisional surface (OMEP-0008): public and
documented, but excluded from ``oxydemark.__all__``.
"""

from __future__ import annotations

from oxydemark import OxydeEngine
from oxydemark.contrib import (
    AdmonitionPlugin,
    LazyImagesPlugin,
    MentionPlugin,
    ShortcodePlugin,
)


class TestAdmonitionPlugin:
    """GitHub-style alerts become styled admonition blocks."""

    def test_note_becomes_admonition(self):
        engine = OxydeEngine(plugins=[AdmonitionPlugin()])
        html = engine.render("> [!NOTE]\n> Useful information.\n")
        assert '<div class="admonition admonition-note">' in html
        assert "Useful information." in html
        assert "<blockquote>" not in html

    def test_injects_title_node(self):
        engine = OxydeEngine(plugins=[AdmonitionPlugin()])
        html = engine.render("> [!WARNING]\n> Careful.\n")
        assert '<div class="admonition-title">' in html
        assert "Warning" in html

    def test_marker_is_not_rendered(self):
        engine = OxydeEngine(plugins=[AdmonitionPlugin()])
        html = engine.render("> [!TIP]\n> Body.\n")
        assert "[!TIP]" not in html

    def test_inline_markup_is_preserved(self):
        engine = OxydeEngine(plugins=[AdmonitionPlugin()])
        html = engine.render("> [!NOTE]\n> Some **bold** and `code`.\n")
        assert "<strong>bold</strong>" in html
        assert "<code>code</code>" in html

    def test_plain_blockquote_is_untouched(self):
        engine = OxydeEngine(plugins=[AdmonitionPlugin()])
        html = engine.render("> Just a quote.\n")
        assert "<blockquote>" in html
        assert "admonition" not in html

    def test_unknown_marker_is_untouched(self):
        engine = OxydeEngine(plugins=[AdmonitionPlugin()])
        html = engine.render("> [!SPAM]\n> Body.\n")
        assert "<blockquote>" in html
        assert "admonition" not in html

    def test_custom_kinds_mapping(self):
        engine = OxydeEngine(plugins=[AdmonitionPlugin(kinds={"danger": "Danger!"})])
        html = engine.render("> [!DANGER]\n> Boom.\n")
        assert '<div class="admonition admonition-danger">' in html
        assert "Danger!" in html

    def test_custom_kinds_replaces_defaults(self):
        engine = OxydeEngine(plugins=[AdmonitionPlugin(kinds={"danger": "Danger!"})])
        html = engine.render("> [!NOTE]\n> Body.\n")
        assert "<blockquote>" in html

    def test_surrounding_content_is_preserved(self):
        engine = OxydeEngine(plugins=[AdmonitionPlugin()])
        html = engine.render("Before\n\n> [!NOTE]\n> Inside.\n\nAfter\n")
        assert "Before" in html
        assert "Inside." in html
        assert "After" in html

    def test_multiple_admonitions(self):
        engine = OxydeEngine(plugins=[AdmonitionPlugin()])
        html = engine.render("> [!NOTE]\n> One.\n\n> [!TIP]\n> Two.\n")
        assert "admonition-note" in html
        assert "admonition-tip" in html


class TestShortcodePlugin:
    """``{{ name argument }}`` markers expand to raw HTML."""

    def test_youtube_shortcode_emits_unescaped_html(self):
        engine = OxydeEngine(plugins=[ShortcodePlugin()])
        html = engine.render("{{ youtube dQw4w9WgXcQ }}")
        assert '<iframe src="https://www.youtube.com/embed/dQw4w9WgXcQ"' in html
        assert "&lt;iframe" not in html

    def test_surrounding_text_is_preserved(self):
        engine = OxydeEngine(plugins=[ShortcodePlugin()])
        html = engine.render("Watch {{ youtube abc123 }} now.")
        assert "Watch " in html
        assert " now." in html

    def test_unknown_shortcode_is_left_verbatim(self):
        engine = OxydeEngine(plugins=[ShortcodePlugin()])
        html = engine.render("{{ unknown value }}")
        assert "{{ unknown value }}" in html

    def test_invalid_argument_is_left_verbatim(self):
        engine = OxydeEngine(plugins=[ShortcodePlugin()])
        html = engine.render("{{ youtube bad id! }}")
        assert "{{ youtube bad id! }}" in html
        assert "<iframe" not in html

    def test_multiple_shortcodes_in_one_paragraph(self):
        engine = OxydeEngine(plugins=[ShortcodePlugin()])
        html = engine.render("{{ youtube aaa }} and {{ youtube bbb }}")
        assert html.count("<iframe") == 2

    def test_custom_shortcode_handler(self):
        plugin = ShortcodePlugin(shortcodes={"hi": lambda arg: f"<b>{arg}</b>"})
        html = OxydeEngine(plugins=[plugin]).render("{{ hi there }}")
        assert "<b>there</b>" in html

    def test_shortcode_inside_code_span_is_untouched(self):
        engine = OxydeEngine(plugins=[ShortcodePlugin()])
        html = engine.render("`{{ youtube abc }}`")
        assert "<iframe" not in html
        assert "{{ youtube abc }}" in html

    def test_shortcode_inside_emphasis_is_expanded(self):
        engine = OxydeEngine(plugins=[ShortcodePlugin()])
        html = engine.render("*{{ youtube abc }}*")
        assert "<iframe" in html


class TestMentionPlugin:
    """``@handle`` mentions become links."""

    def test_mention_becomes_link(self):
        engine = OxydeEngine(plugins=[MentionPlugin()])
        html = engine.render("Ping @alice about it.")
        assert '<a href="https://github.com/alice" class="mention">@alice</a>' in html

    def test_custom_base_url(self):
        engine = OxydeEngine(plugins=[MentionPlugin(base_url="https://example.com/u/")])
        html = engine.render("Hi @bob")
        assert 'href="https://example.com/u/bob"' in html

    def test_multiple_mentions(self):
        engine = OxydeEngine(plugins=[MentionPlugin()])
        html = engine.render("@alice and @bob")
        assert html.count('class="mention"') == 2

    def test_mention_in_code_span_is_untouched(self):
        engine = OxydeEngine(plugins=[MentionPlugin()])
        html = engine.render("`@alice`")
        assert "mention" not in html
        assert "<code>@alice</code>" in html

    def test_mention_in_code_block_is_untouched(self):
        engine = OxydeEngine(plugins=[MentionPlugin()])
        html = engine.render("```\n@alice\n```\n")
        assert "mention" not in html

    def test_mention_inside_existing_link_is_untouched(self):
        engine = OxydeEngine(plugins=[MentionPlugin()])
        html = engine.render("[@alice](https://example.com)")
        assert 'href="https://example.com"' in html
        assert "mention" not in html

    def test_email_is_not_mangled(self):
        engine = OxydeEngine(plugins=[MentionPlugin()])
        html = engine.render("Mail me at alice@example.com please.")
        assert "mention" not in html

    def test_surrounding_text_is_preserved(self):
        engine = OxydeEngine(plugins=[MentionPlugin()])
        html = engine.render("Hey @alice, welcome!")
        assert "Hey " in html
        assert ", welcome!" in html

    def test_text_without_mentions_is_unchanged(self):
        engine = OxydeEngine(plugins=[MentionPlugin()])
        assert engine.render("Plain text.") == OxydeEngine().render("Plain text.")


class TestLazyImagesPlugin:
    """Rendered images get lazy-loading hints."""

    def test_adds_loading_and_decoding(self):
        engine = OxydeEngine(plugins=[LazyImagesPlugin()])
        html = engine.render("![alt](img.png)")
        assert 'loading="lazy"' in html
        assert 'decoding="async"' in html

    def test_preserves_existing_attributes(self):
        engine = OxydeEngine(plugins=[LazyImagesPlugin()])
        html = engine.render("![alt](img.png)")
        assert 'src="img.png"' in html
        assert 'alt="alt"' in html

    def test_existing_loading_attribute_is_kept(self):
        plugin = LazyImagesPlugin()
        assert plugin.postprocess('<img src="a.png" loading="eager">') == (
            '<img src="a.png" loading="eager">'
        )

    def test_is_idempotent(self):
        plugin = LazyImagesPlugin()
        once = plugin.postprocess('<img src="a.png">')
        assert plugin.postprocess(once) == once

    def test_decoding_can_be_disabled(self):
        plugin = LazyImagesPlugin(decoding=None)
        assert plugin.postprocess('<img src="a.png">') == '<img src="a.png" loading="lazy">'

    def test_html_without_images_is_unchanged(self):
        plugin = LazyImagesPlugin()
        assert plugin.postprocess("<p>hello</p>") == "<p>hello</p>"


class TestContribComposition:
    """All contrib plugins can be combined in a single engine."""

    SOURCE = (
        "> [!NOTE]\n"
        "> Ping @alice about {{ youtube abc123 }}\n"
        "\n"
        "![alt](img.png)\n"
    )

    def test_all_plugins_together(self):
        engine = OxydeEngine(
            plugins=[AdmonitionPlugin(), ShortcodePlugin(), MentionPlugin(), LazyImagesPlugin()]
        )
        html = engine.render(self.SOURCE)
        assert '<div class="admonition admonition-note">' in html
        assert 'class="mention"' in html
        assert "<iframe" in html
        assert 'loading="lazy"' in html

    def test_ast_plugin_order_is_irrelevant_here(self):
        forward = OxydeEngine(plugins=[AdmonitionPlugin(), ShortcodePlugin(), MentionPlugin()])
        reverse = OxydeEngine(plugins=[AdmonitionPlugin(), MentionPlugin(), ShortcodePlugin()])
        assert forward.render(self.SOURCE) == reverse.render(self.SOURCE)
