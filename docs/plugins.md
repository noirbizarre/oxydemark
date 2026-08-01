# Writing OxydeMark plugins

OxydeMark is a pipeline. A plugin is any object that hooks into one or more of
its stages:

```
Markdown Input
    -> preprocess plugins        (text-level, str -> str)
    -> Rust parser / rushdown    (AST generation)
    -> transform plugins         (AST-level, AstNode -> AstNode)
    -> Rust renderer             (HTML generation)
    -> postprocess plugins       (HTML-level, str -> str)
    -> Final Output
```

```python
from oxydemark import OxydeEngine
from oxydemark.contrib import AdmonitionPlugin, MentionPlugin

engine = OxydeEngine(plugins=[AdmonitionPlugin(), MentionPlugin()])
html = engine.render("> [!NOTE]\n> Ping @alice\n")
```

## The `Plugin` protocol

```python
class Plugin(Protocol):
    def preprocess(self, markdown: str) -> str: ...
    def transform(self, ast: AstNode) -> AstNode: ...
    def postprocess(self, html: str) -> str: ...
```

`Plugin` is a **structural** `typing.Protocol`. You never need to subclass or
register anything: `OxydeEngine` dispatches with `hasattr`, so a plain class
implementing a single hook is a perfectly valid plugin.

```python
class Shouty:
    def postprocess(self, html: str) -> str:
        return html.upper()

OxydeEngine(plugins=[Shouty()]).render("hello")
```

Inheriting from `Plugin` is still useful if you want a type checker to verify
your signatures.

### Hook ordering

The engine runs **one phase at a time across the whole plugin list**, not one
plugin at a time. For `plugins=[A, B]`:

```
A.preprocess -> B.preprocess -> parse
    -> A.transform -> B.transform -> render
        -> A.postprocess -> B.postprocess
```

Ordering within a phase matters whenever two plugins touch the same content.

## Choosing the right hook

| Hook | Operates on | Use it when |
| ---- | ----------- | ----------- |
| `preprocess` | raw Markdown `str` | the construct has no AST representation yet: custom markers, macros, includes. |
| `transform` | `AstNode` tree | you need structure: adding, removing, re-typing or annotating nodes. |
| `postprocess` | rendered HTML `str` | the change is purely presentational and applies to the final markup. |

Prefer the *latest* stage that can do the job cleanly. Text-level rewriting is
blunt (it happily corrupts code blocks); AST-level work is precise.

## Escaping and HTML injection

Three rules govern what actually reaches the output:

1. `AstNode.text` is **HTML-escaped** by the renderer.
   `node.text = "<b>x</b>"` renders as `&lt;b&gt;x&lt;/b&gt;`.
2. Raw HTML present in the **source** Markdown is **stripped**
   (`<!-- raw HTML omitted -->`). A `preprocess` hook cannot smuggle markup in.
3. A node of kind `raw_html` emits its `text` **verbatim**. This is the only
   supported injection point, and it is your responsibility to escape any
   untrusted data placed in it.

Whenever the markup you want already has an AST representation (`link`,
`image`, `strong`, `block_component`, ...), build those nodes instead of
`raw_html`. `MentionPlugin` builds `link` nodes; `ShortcodePlugin` has to use
`raw_html` because an `<iframe>` has no AST equivalent.

## AST value semantics (important)

`AstNode` is a PyO3 class with **value semantics**. Every attribute access
returns a *copy*:

```python
node.children[0].text = "x"   # silently discarded
node.children.append(other)   # silently discarded
for n in node.walk():         # walk() yields copies too
    n.text = "x"              # silently discarded
```

The correct idiom is copy-out / mutate / write-back:

```python
def _modify(self, node: AstNode) -> None:
    if node.kind == "text" and node.text:
        node.text = node.text.upper()

    children = node.children      # copy out
    for child in children:
        self._modify(child)
    node.children = children      # write back -- mandatory
```

The same applies to `attributes`:

```python
attributes = node.attributes
attributes["class"] = "highlight"
node.attributes = attributes
```

`walk()` is therefore an **inspection-only** API. Use explicit recursion for
mutation.

### Text nodes are fragmented

The parser may emit several consecutive `text` siblings for what looks like one
run of characters in the source. For example `{{ youtube abc }}` parses as
`text("{{ youtube abc")` followed by `text(" }}")`.

A plugin matching a pattern that can span such a boundary must coalesce
adjacent `text` siblings before matching. `oxydemark/contrib/_text.py` shows
one way to do it, wrapped in a reusable `rewrite_text_nodes()` walker.

### Node kinds

The `kind` values are part of the public contract (OMEP-0008). Existing kinds
are never renamed; new ones may be added, so always handle unknown kinds
gracefully.

Block: `document`, `paragraph`, `heading`, `blockquote`, `list`, `list_item`,
`code_block`, `html_block`, `thematic_break`, `table`, `table_header`,
`table_body`, `table_row`, `table_cell`, `block_component`.

Inline: `text`, `emphasis`, `strong`, `strikethrough`, `code_span`, `link`,
`image`, `raw_html`, `softbreak`, `hardbreak`, `emoji`, `inline_component`,
`span_attributes`, `slot`.

Attributes on a node are rendered as HTML attributes for the kinds that support
them (`link`, `image`, `heading`, `block_component`, ...). Notably,
`block_component` renders as a `<div>` carrying every attribute except `name`,
which makes it a convenient generic container.

## Example plugins

The `oxydemark.contrib` namespace ships four worked examples, each focused on a
different layer. Read the source alongside `tests/test_contrib.py`.

### `AdmonitionPlugin` -- `preprocess` + `transform`

Turns GitHub-style alerts into styled blocks.

```markdown
> [!NOTE]
> Useful information.
```

```html
<div class="admonition admonition-note">
<div class="admonition-title">
Note</div>
<p>Useful information.</p>
</div>
```

It needs both hooks because the `[!NOTE]` marker is *not* detectable in the
AST: the parser splits it into three text nodes (`"["`, `"!NOTE"`, `"]"`) since
it looks like a link reference. Detection is therefore done on text
(`preprocess` rewrites the blockquote into a Comark `:::note` fence), while
classes and the title node are added on the AST (`transform`).

Configure the recognised markers with `AdmonitionPlugin(kinds={"danger": "Danger!"})`.

### `ShortcodePlugin` -- `transform`

Expands `{{ name argument }}` markers into raw HTML.

```markdown
Watch {{ youtube dQw4w9WgXcQ }} now.
```

Unknown shortcodes, and handlers returning an empty string, leave the marker
untouched. Add your own:

```python
ShortcodePlugin(shortcodes={"hi": lambda arg: f"<b>{arg}</b>"})
```

### `MentionPlugin` -- `transform`

Turns `@handle` into a `link` node (not raw HTML), skipping code spans, code
blocks and existing links.

```python
MentionPlugin(base_url="https://example.com/u/")
```

### `LazyImagesPlugin` -- `postprocess`

Adds `loading="lazy" decoding="async"` to `<img>` tags that lack them. This is
deliberately a string-level rewrite: it is a presentational tweak, and it also
catches images produced by other plugins via `raw_html`.

## Stability

`oxydemark.contrib` is a **provisional** surface (see
[OMEP-0008](specs/OMEP-0008-public-api.md)). It is public, documented and
tested, but intentionally excluded from `oxydemark.__all__` and covered by no
stability guarantee: these plugins may change or be removed in a MINOR release.
If you need long-term stability, copy the plugin into your own codebase.
