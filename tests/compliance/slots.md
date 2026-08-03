# Slots

Named slots, the explicit and implicit default slots, and slot markers outside a component body.

Reference: docs/specs/OMEP-0007-comark-syntax.md#slots

## named-slots-emit-data-slot-wrappers

The OMEP-0007 compliance table example: each `#name` marker opens a slot rendered as `<div data-slot="name">`.

````comark
::card
#header
## Card Title

#content
Main content here.
::
````

````html
<div>
<div data-slot="header">
<h2 id="card-title">Card Title</h2>
</div>
<div data-slot="content">
<p>Main content here.</p>
</div>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "attributes": {
    "name": "card"
  },
  "exact_children": true,
  "children": [
    {
      "kind": "slot",
      "attributes": {
        "name": "header"
      },
      "exact_children": true,
      "children": [
        {
          "kind": "heading",
          "attributes": {
            "id": "card-title",
            "level": "2"
          }
        }
      ]
    },
    {
      "kind": "slot",
      "attributes": {
        "name": "content"
      },
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

## explicit-default-slot-is-wrapped

An explicit `#default` marker behaves like any other named slot.

````comark
::card
#default
D
::
````

````html
<div>
<div data-slot="default">
<p>D</p>
</div>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "exact_children": true,
  "children": [
    {
      "kind": "slot",
      "attributes": {
        "name": "default"
      }
    }
  ]
}
````

## implicit-default-slot-is-not-wrapped

The OMEP-0007 compliance table example: content before any marker is not wrapped in a `slot` node.

````comark
::alert{type="info"}
This content goes to the default slot.
::
````

````html
<div>
<p>This content goes to the default slot.</p>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "attributes": {
    "name": "alert",
    "type": "info"
  },
  "exact_children": true,
  "children": [
    {
      "kind": "paragraph"
    }
  ]
}
````

## implicit-default-content-before-a-named-slot

Leading content stays unwrapped even when named slots follow it.

````comark
::card
Intro

#header
H
::
````

````html
<div>
<p>Intro</p>
<div data-slot="header">
<p>H</p>
</div>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "exact_children": true,
  "children": [
    {
      "kind": "paragraph"
    },
    {
      "kind": "slot",
      "attributes": {
        "name": "header"
      }
    }
  ]
}
````

## nested-component-inside-a-slot

A slot body is regular Markdown, so it may contain further components.

````comark
:::outer
#a
::inner
x
::
:::
````

````html
<div>
<div data-slot="a">
<div>
<p>x</p>
</div>
</div>
</div>
````

````json ast
{
  "descend": "first:slot",
  "attributes": {
    "name": "a"
  },
  "exact_children": true,
  "children": [
    {
      "kind": "block_component",
      "attributes": {
        "name": "inner"
      }
    }
  ]
}
````

## slot-marker-outside-a-component-is-plain-text

Slot markers are only recognised at the top level of a block component body.

````comark
#header

Text
````

````html
<p>#header</p>
<p>Text</p>
````

````json ast
{
  "kind": "document",
  "exact_children": true,
  "children": [
    {
      "kind": "paragraph"
    },
    {
      "kind": "paragraph"
    }
  ]
}
````
