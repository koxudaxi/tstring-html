# HTML

The `html-tstring` package renders PEP 750 t-string templates to safe HTML.

## Basic rendering

```python
from html_tstring import render_html

name = "world"
page = render_html(t"<h1>Hello, {name}!</h1>")
# <h1>Hello, world!</h1>
```

Interpolated text is HTML-escaped automatically:

```python
user_input = '<img src=x onerror="alert(1)">'
safe = render_html(t"<p>{user_input}</p>")
# <p>&lt;img src=x onerror="alert(1)"&gt;</p>
```

## Raw HTML

Use `RawHtml` when you have trusted HTML that should not be escaped:

```python
from html_tstring import RawHtml, render_html

icon = RawHtml('<svg><circle r="5"/></svg>')
render_html(t"<span>{icon}</span>")
# <span><svg><circle r="5"/></svg></span>
```

`RawHtml` works in child and fragment positions. In attributes and spread values it is treated as a plain string and escaped normally. Interpolation inside `<script>`, `<style>`, and `<textarea>` is rejected. `<title>{...}</title>` is allowed, but interpolated values are still escaped as text.

## Attributes

### Dynamic attribute values

Quoted dynamic attribute values are always supported:

```python
from html_tstring import render_html

href = "/dashboard"
render_html(t'<a href="{href}">Dashboard</a>')
# <a href="/dashboard">Dashboard</a>
```

A dynamic attribute interpolation must be quoted in v1. Unquoted dynamic attributes are rejected during validation:

```python
title = "safe & sound"
render_html(t'<div title="{title}"></div>')
# <div title="safe &amp; sound"></div>
```

### Boolean attributes

`True` renders a bare attribute. `False` and `None` omit it:

```python
from html_tstring import render_html

disabled = True
hidden = False
render_html(t"<button disabled={disabled} hidden={hidden}>OK</button>")
# <button disabled>OK</button>
```

### Class normalization

The `class` attribute accepts strings, lists, and dict-like mappings:

```python
from html_tstring import render_html

classes = ["btn", {"btn-primary": True, "btn-disabled": False}]
render_html(t'<button class="{classes}">Click</button>')
# <button class="btn btn-primary">Click</button>
```

## Spread attributes

A bare interpolation inside a start tag is treated as a spread:

```python
from html_tstring import render_html

attrs = {"data-id": "42", "class": "extra", "hidden": False}
render_html(t'<div class="base" {attrs}>content</div>')
# <div class="base extra" data-id="42">content</div>
```

Spread merge rules:

- Attributes are applied left to right in source order
- For non-`class` attributes, later values replace earlier ones
- For `class`, values are concatenated in source order
- `True` renders a bare attribute, `False`/`None` omits it

## Fragment rendering

`render_fragment` works the same as `render_html`:

```python
from html_tstring import render_fragment

name = "world"
render_fragment(t"<p>{name}</p>")
# <p>world</p>
```

## Fragment children

Use `Fragment` to pass multiple children:

```python
from html_tstring import Fragment, RawHtml, render_html

children = Fragment([RawHtml("<em>first</em>"), "second"])
render_html(t"<div>{children}</div>")
# <div><em>first</em>second</div>
```

## Template validation

`check_template` validates syntax without rendering:

```python
from html_tstring import check_template

check_template(t"<div>valid</div>")  # OK
check_template(t"<Button />")       # TemplateSemanticError: component tags need thtml-tstring
```

## Template formatting

`format_template` gives back the template source with canonical formatting:

```python
from html_tstring import format_template

name = "world"
format_template(t"<div>{name}</div>")
# '<div>{name}</div>'

format_template(
    t'<div data-a="12345" data-b="67890"></div>',
    line_length=20,
)
# '<div\n  data-a="12345"\n  data-b="67890"\n></div>'
```

## Raw-text elements

Interpolation inside `<script>`, `<style>`, and `<textarea>` is rejected:

```python
from html_tstring import check_template, TemplateSemanticError

script = "alert('x')"
check_template(t"<script>{script}</script>")
# raises TemplateSemanticError
```

`<title>` is the one exception. It accepts interpolations, but they are rendered as escaped text rather than raw HTML:

```python
from html_tstring import RawHtml, render_html

title = RawHtml("<b>safe</b>")
render_html(t"<title>{title}</title>")
# <title>&lt;b&gt;safe&lt;/b&gt;</title>
```
