# Nesting

Nested block components fenced with runs of two or more colons.

Reference: docs/specs/OMEP-0007-comark-syntax.md#nested-components

## two-levels-with-distinct-colon-runs

The OMEP-0007 compliance table example: an opener closes on a line of exactly the same number of colons.

````comark
:::outer
::inner
Content
::
:::
````

````html
<div>
<div>
<p>Content</p>
</div>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "attributes": {
    "name": "outer"
  },
  "exact_children": true,
  "children": [
    {
      "kind": "block_component",
      "attributes": {
        "name": "inner"
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

## three-levels-with-increasing-colon-runs

The OMEP-0007 compliance table example: extra colons are a readability convention.

````comark
::level-1
:::level-2
::::level-3
Content
::::
:::
::
````

````html
<div>
<div>
<div>
<p>Content</p>
</div>
</div>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "attributes": {
    "name": "level-1"
  },
  "exact_children": true,
  "children": [
    {
      "kind": "block_component",
      "attributes": {
        "name": "level-2"
      },
      "exact_children": true,
      "children": [
        {
          "kind": "block_component",
          "attributes": {
            "name": "level-3"
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
  ]
}
````

## equal-colon-runs-close-innermost-first

Equal-colon nesting is valid; the first closing fence closes the innermost component.

````comark
::level-1
::level-2
Content
::
::

After
````

````html
<div>
<div>
<p>Content</p>
</div>
</div>
<p>After</p>
````

````json ast
{
  "kind": "document",
  "exact_children": true,
  "children": [
    {
      "kind": "block_component",
      "attributes": {
        "name": "level-1"
      },
      "exact_children": true,
      "children": [
        {
          "kind": "block_component",
          "attributes": {
            "name": "level-2"
          }
        }
      ]
    },
    {
      "kind": "paragraph"
    }
  ]
}
````

## content-after-a-component

The closing fence ends the component; following blocks are siblings of it.

````comark
::a
X
::

After
````

````html
<div>
<p>X</p>
</div>
<p>After</p>
````

````json ast
{
  "kind": "document",
  "exact_children": true,
  "children": [
    {
      "kind": "block_component",
      "attributes": {
        "name": "a"
      }
    },
    {
      "kind": "paragraph"
    }
  ]
}
````

## nested-components-inherit-their-own-attributes

````comark
:::outer{.o}
::inner{.i}
Content
::
:::
````

````html
<div class="o">
<div class="i">
<p>Content</p>
</div>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "attributes": {
    "name": "outer",
    "class": "o"
  },
  "children": [
    {
      "kind": "block_component",
      "attributes": {
        "name": "inner",
        "class": "i"
      }
    }
  ]
}
````
