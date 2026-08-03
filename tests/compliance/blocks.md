# Core Markdown blocks

Block constructs whose HTML depends on data the arena carries but the `AstNode`
tree historically dropped: fence info strings, list ordering and tightness, task
markers and table cell roles. Every case here exists to keep the fast path and
the standalone AST renderer byte-identical.

## fenced-code-language

A fence info string becomes a `language-` class, and the code content must
survive the arena-to-`AstNode` conversion so plugins can read it

`````comark
```rust
fn main() {}
```
`````

`````html
<pre><code class="language-rust">fn main() {}
</code></pre>
`````

`````json ast
{
  "descend": "first:code_block",
  "text": "fn main() {}\n",
  "attributes": {
    "info": "rust"
  }
}
`````

## fenced-code-no-info

Without an info string there is no class, and `info` stays absent rather than
being set to an empty string

`````comark
```
plain
```
`````

`````html
<pre><code>plain
</code></pre>
`````

`````json ast
{
  "descend": "first:code_block",
  "text": "plain\n",
  "absent_attributes": ["info"]
}
`````

## fenced-code-info-with-arguments

Only the first word of the info string becomes the language class, but the
full info string is preserved in the AST for plugins to parse

`````comark
```python title="example.py"
code
```
`````

`````html
<pre><code class="language-python">code
</code></pre>
`````

`````json ast
{
  "descend": "first:code_block",
  "attributes": {
    "info": "python title=\"example.py\""
  }
}
`````

## fenced-code-is-escaped

Code content is HTML-escaped, never interpreted as markup

`````comark
```
<b>&"x"</b>
```
`````

`````html
<pre><code>&lt;b&gt;&amp;&quot;x&quot;&lt;/b&gt;
</code></pre>
`````

## indented-code

An indented code block carries its content the same way a fenced one does

````comark
    indented code
````

````html
<pre><code>indented code
</code></pre>
````

````json ast
{
  "descend": "first:code_block",
  "text": "indented code\n"
}
````

## ordered-list

An ordered list renders as `<ol>`, so the AST must record the marker kind

````comark
1. one
2. two
````

````html
<ol>
<li>one</li>
<li>two</li>
</ol>
````

````json ast
{
  "descend": "first:list",
  "attributes": {
    "ordered": "true",
    "tight": "true",
    "start": "1"
  }
}
````

## ordered-list-start

A list that does not start at 1 emits a `start` attribute; a list starting at 1
does not

````comark
5. five
6. six
````

````html
<ol start="5">
<li>five</li>
<li>six</li>
</ol>
````

````json ast
{
  "descend": "first:list",
  "attributes": {
    "ordered": "true",
    "start": "5"
  }
}
````

## bullet-list-tight

A tight list drops the `<p>` wrappers its items nonetheless carry in the AST

````comark
- a
- b
````

````html
<ul>
<li>a</li>
<li>b</li>
</ul>
````

````json ast
{
  "descend": "first:list",
  "attributes": {
    "ordered": "false",
    "tight": "true"
  }
}
````

## bullet-list-loose

A loose list keeps the `<p>` wrappers and opens each item on its own line

````comark
- a

- b
````

````html
<ul>
<li>
<p>a</p>
</li>
<li>
<p>b</p>
</li>
</ul>
````

````json ast
{
  "descend": "first:list",
  "attributes": {
    "tight": "false"
  }
}
````

## nested-tight-list

Inside a tight item, an unwrapped paragraph is followed by a newline when a
sibling block follows it

````comark
- a
  - b
````

````html
<ul>
<li>a
<ul>
<li>b</li>
</ul>
</li>
</ul>
````

## task-list

Task markers render as disabled checkboxes, so the checked state must reach the
AST

````comark
- [ ] todo
- [x] done
````

````html
<ul>
<li><input disabled="" type="checkbox"> todo</li>
<li><input checked="" disabled="" type="checkbox"> done</li>
</ul>
````

````json ast
{
  "descend": "first:list_item",
  "attributes": {
    "task": "active"
  }
}
````

## task-list-loose

In a loose list the checkbox is emitted inside the item's paragraph

````comark
- [ ] todo

- [x] done
````

````html
<ul>
<li>
<p><input disabled="" type="checkbox"> todo</p>
</li>
<li>
<p><input checked="" disabled="" type="checkbox"> done</p>
</li>
</ul>
````

## table-header-cells

Cells inside a `table_header` render as `<th>`, which the standalone renderer
derives from the ancestry rather than from the node

````comark
| A |
|---|
| b |
````

````html
<table>
<thead>
<tr>
<th>A</th>
</tr>
</thead>
<tbody>
<tr>
<td>b</td>
</tr>
</tbody>
</table>
````

## table-alignment

Column alignment becomes an inline `text-align` style on every cell of the
column, header and body alike

````comark
| A | B |
|:--|--:|
| 1 | 2 |
````

````html
<table>
<thead>
<tr>
<th style="text-align: left;">A</th>
<th style="text-align: right;">B</th>
</tr>
</thead>
<tbody>
<tr>
<td style="text-align: left;">1</td>
<td style="text-align: right;">2</td>
</tr>
</tbody>
</table>
````

````json ast
{
  "descend": "first:table_cell",
  "attributes": {
    "align": "left"
  }
}
````
