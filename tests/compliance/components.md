# Components

Block components, inline components, span attributes and the inline `{...}` attribute syntax.

Reference: docs/specs/OMEP-0007-comark-syntax.md#syntax-overview

## block-component-minimal

A `::name` fence produces a `block_component` rendered as a plain `div`.

````comark
::note
Content
::
````

````html
<div>
<p>Content</p>
</div>
````

````json ast
{
  "kind": "document",
  "exact_children": true,
  "children": [
    {
      "kind": "block_component",
      "attributes": {
        "name": "note"
      },
      "props": null,
      "exact_children": true,
      "children": [
        {
          "kind": "paragraph"
        }
      ]
    }
  ]
}
````

## block-component-attributes-are-filtered-and-sorted

Class shorthands merge, `#id` becomes `id`, and rendered attributes are sorted.

````comark
::card{#c .a .b data-z=1}
text
::
````

````html
<div class="a b" data-z="1" id="c">
<p>text</p>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "attributes": {
    "name": "card",
    "id": "c",
    "class": "a b",
    "data-z": "1"
  }
}
````

## block-component-attribute-values-are-escaped

Attribute values are HTML-escaped on output but kept verbatim in the AST.

````comark
::card{title="a & <b>"}
text
::
````

````html
<div title="a &amp; &lt;b&gt;">
<p>text</p>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "attributes": {
    "title": "a & <b>"
  }
}
````

## block-component-boolean-prop

OMEP-0007: a value-less prop is exposed as `:key = "true"` and dropped from the HTML.

````comark
::component{disabled}
x
::
````

````html
<div>
<p>x</p>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "attributes": {
    "name": "component",
    ":disabled": "true"
  },
  "absent_attributes": [
    "disabled"
  ],
  "props": null
}
````

## block-component-non-html-attributes-stay-in-the-ast

Only `class`, `id`, `data-*` and `style` reach the HTML; everything else remains available to plugins.

````comark
::card{disabled foo="bar"}
text
::
````

````html
<div>
<p>text</p>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "attributes": {
    "name": "card",
    ":disabled": "true",
    "foo": "bar"
  }
}
````

## inline-component-with-content

`:name[content]{...}` renders as a `span` carrying only the HTML-valid attributes.

````comark
Some :icon[star]{.big disabled} here
````

````html
<p>Some <span class="big">star</span> here</p>
````

````json ast
{
  "descend": "first:inline_component",
  "attributes": {
    "name": "icon",
    "class": "big",
    ":disabled": "true"
  }
}
````

## inline-component-without-content

A component whose attributes are all non-HTML renders as an empty `span`.

````comark
Go :icon{type="star"} now
````

````html
<p>Go <span></span> now</p>
````

````json ast
{
  "descend": "first:inline_component",
  "attributes": {
    "name": "icon",
    "type": "star"
  }
}
````

## inline-component-in-a-sentence

The comark documentation example.

````comark
Check out this :badge[New]{color="blue"} feature.
````

````html
<p>Check out this <span>New</span> feature.</p>
````

````json ast
{
  "descend": "first:inline_component",
  "attributes": {
    "name": "badge",
    "color": "blue"
  }
}
````

## span-attributes-class-and-style

The comark documentation example for span attributes.

````comark
This is [highlighted text]{.highlight style="color: blue"} in a paragraph.
````

````html
<p>This is <span class="highlight" style="color: blue">highlighted text</span> in a paragraph.</p>
````

````json ast
{
  "descend": "first:span_attributes",
  "attributes": {
    "class": "highlight",
    "style": "color: blue"
  }
}
````

## span-attributes-class-and-id

````comark
A [text]{.hl #x} b
````

````html
<p>A <span class="hl" id="x">text</span> b</p>
````

````json ast
{
  "descend": "first:span_attributes",
  "attributes": {
    "class": "hl",
    "id": "x"
  }
}
````

## span-attributes-are-filtered

Boolean props are `:`-prefixed on spans too, and never rendered.

````comark
A [text]{.hl disabled} b
````

````html
<p>A <span class="hl">text</span> b</p>
````

````json ast
{
  "descend": "first:span_attributes",
  "attributes": {
    "class": "hl",
    ":disabled": "true"
  }
}
````
