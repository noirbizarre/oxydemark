# Python API

The `oxydemark` package is the frozen public Python surface: everything listed
in `oxydemark.__all__`, and nothing else. The native `oxydemark._core` module is
an implementation detail and must not be imported directly.

## Engine and plugins

::: oxydemark.OxydeEngine

::: oxydemark.Plugin

::: oxydemark.api.Preprocessor

::: oxydemark.api.Transformer

::: oxydemark.api.Postprocessor

## Parsing and rendering

::: oxydemark.parse

::: oxydemark.parse_document

::: oxydemark.render_ast

::: oxydemark.markdown_to_html

## Metadata helpers

::: oxydemark.slugify

::: oxydemark.extract_summary

## AST and metadata types

::: oxydemark.AstNode

::: oxydemark.ParseResult

::: oxydemark.Heading

## Example plugins

!!! warning "Provisional surface"

    `oxydemark.contrib` is public and documented, but it is intentionally *not*
    part of `oxydemark.__all__` and carries no stability guarantee: these
    plugins may change or be removed in a MINOR release. If you need long-term
    stability, copy the plugin into your own codebase.

::: oxydemark.contrib
    options:
      members:
        - AdmonitionPlugin
        - ShortcodePlugin
        - MentionPlugin
        - LazyImagesPlugin
        - Shortcode

### Defaults

The `kinds=` and `shortcodes=` arguments **replace** these defaults rather than
extending them; spread the mapping to add entries.

::: oxydemark.contrib.admonitions.DEFAULT_KINDS

::: oxydemark.contrib.shortcodes.DEFAULT_SHORTCODES
