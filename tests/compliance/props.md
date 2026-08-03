# Props

Typed block props declared as YAML frontmatter or as a `yaml [props]` fence.

Reference: docs/specs/OMEP-0007-comark-syntax.md#block-props

## frontmatter-props-keep-native-types

The OMEP-0007 compliance table example: scalars keep their YAML types and never leak into `attributes`.

````comark
::card
---
variant: elevated
count: 42
enabled: true
---
Body
::
````

````html
<div>
<p>Body</p>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "attributes": {
    "name": "card"
  },
  "absent_attributes": [
    "variant",
    "count",
    "enabled"
  ],
  "props": {
    "variant": "elevated",
    "count": 42,
    "enabled": true
  }
}
````

## frontmatter-props-floats-and-null

````comark
::card
---
ratio: 1.5
nothing: null
---
Body
::
````

````html
<div>
<p>Body</p>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "props": {
    "ratio": 1.5,
    "nothing": null
  }
}
````

## fenced-props-sequences-and-mappings

The `yaml [props]` fence style is equivalent to frontmatter and supports nested structures.

````comark
::card
```yaml [props]
tags:
  - a
  - b
obj:
  k: v
```
Body
::
````

````html
<div>
<p>Body</p>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "props": {
    "tags": [
      "a",
      "b"
    ],
    "obj": {
      "k": "v"
    }
  }
}
````

## props-combined-with-inline-attributes

The OMEP-0007 compliance table example: inline attributes and block props coexist.

````comark
::card{.featured}
---
title: Featured
variant: plain
---
Body
::
````

````html
<div class="featured">
<p>Body</p>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "attributes": {
    "name": "card",
    "class": "featured"
  },
  "props": {
    "title": "Featured",
    "variant": "plain"
  }
}
````

## inline-attributes-win-over-colliding-props

Merge order: block props first, then inline attributes override colliding keys, which are dropped from `props`.

````comark
::card{variant="inline"}
---
variant: yaml
title: T
---
Body
::
````

````html
<div>
<p>Body</p>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "attributes": {
    "variant": "inline"
  },
  "props": {
    "title": "T"
  }
}
````

## props-are-never-rendered-as-html

Typed props are plugin data only, even when their names look like HTML attributes.

````comark
::card
---
id: from-props
class: from-props
---
Body
::
````

````html
<div>
<p>Body</p>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "absent_attributes": [
    "id",
    "class"
  ],
  "props": {
    "id": "from-props",
    "class": "from-props"
  }
}
````

## props-are-null-without-a-yaml-block

`props` is `None`/`null` when the component declares no block props.

````comark
::card{a="1"}
Body
::
````

````html
<div>
<p>Body</p>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "props": null
}
````

## props-followed-by-named-slots

Block props must come before any slot marker.

````comark
::card
---
title: T
---
#header
H
::
````

````html
<div>
<div data-slot="header">
<p>H</p>
</div>
</div>
````

````json ast
{
  "descend": "first:block_component",
  "props": {
    "title": "T"
  },
  "exact_children": true,
  "children": [
    {
      "kind": "slot",
      "attributes": {
        "name": "header"
      }
    }
  ]
}
````
