# Core Markdown

Core Markdown constructs where the two render paths historically disagreed: HTML5 void elements and raw HTML sanitisation

## image

Images are void elements in HTML5 style, with alt right after src

````comark
![alt](pic.png)
````

````html
<p><img src="pic.png" alt="alt"></p>
````

````json ast
{
  "descend": "first:image",
  "attributes": {
    "src": "pic.png"
  }
}
````

## image-empty-alt

An empty alt text is still emitted, as alt is required markup

````comark
![](pic.png)
````

````html
<p><img src="pic.png" alt=""></p>
````

````json ast
{
  "descend": "first:image",
  "attributes": {
    "src": "pic.png"
  }
}
````

## image-title

A link title becomes a title attribute, after alt

````comark
![alt](pic.png "Title")
````

````html
<p><img src="pic.png" alt="alt" title="Title"></p>
````

````json ast
{
  "descend": "first:image",
  "attributes": {
    "src": "pic.png",
    "title": "Title"
  }
}
````

## thematic-break

Thematic breaks render as an HTML5 void <hr>

````comark
***
````

````html
<hr>
````

````json ast
{
  "descend": "first:thematic_break"
}
````

## thematic-break-dashes

A --- rule is only a thematic break once content precedes it; a leading --- opens a frontmatter block

````comark
a

---

b
````

````html
<p>a</p>
<hr>
<p>b</p>
````

````json ast
{
  "descend": "first:thematic_break"
}
````

## hard-break

Two trailing spaces produce an HTML5 void <br>

````comark
a  
b
````

````html
<p>a<br>
b</p>
````

````json ast
{
  "descend": "first:hardbreak"
}
````

## html-block-omitted

Raw HTML blocks from the source are replaced by a placeholder comment; the AST carries the placeholder, not the source markup

````comark
<div>x</div>
````

````html
<!-- raw HTML omitted -->
````

````json ast
{
  "descend": "first:html_block",
  "text": "<!-- raw HTML omitted -->\n"
}
````

## inline-html-omitted

Inline raw HTML is replaced by the same placeholder, without a trailing newline

````comark
x <b>y</b> z
````

````html
<p>x <!-- raw HTML omitted -->y<!-- raw HTML omitted --> z</p>
````

````json ast
{
  "descend": "first:raw_html",
  "text": "<!-- raw HTML omitted -->"
}
````
