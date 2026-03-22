# Quick Start

## HTML rendering

Interpolated values are HTML-escaped automatically:

```python
from html_tstring import render_html

name = "<b>world</b>"
page = render_html(t"<div>{name}</div>")
# <div>&lt;b&gt;world&lt;/b&gt;</div>
```

## Raw HTML

Use `RawHtml` to inject external trusted HTML without escaping:

```python
from html_tstring import RawHtml, render_html

icon = RawHtml('<svg xmlns="http://www.w3.org/2000/svg"><circle r="10"/></svg>')
page = render_html(t"<div>{icon}</div>")
# <div><svg xmlns="http://www.w3.org/2000/svg"><circle r="10"/></svg></div>
```

## Class normalization

The `class` attribute accepts strings, lists, and conditional dicts:

```python
from html_tstring import render_html

classes = ["primary", {"active": True, "disabled": False}]
page = render_html(t'<button class="{classes}">Click</button>')
# <button class="primary active">Click</button>
```

## Spread attributes

```python
from html_tstring import render_html

attrs = {"data-id": "42", "hidden": False, "class": "extra"}
page = render_html(t'<div class="base" {attrs}>content</div>')
# <div class="base extra" data-id="42">content</div>
```

## T-HTML components

T-HTML adds component syntax to t-strings. A tag that starts with an uppercase letter is resolved to a Python callable. Attributes become keyword arguments, nested content becomes `children`.

With `@component`, you can return a `Template` directly. The decorator auto-wraps it into a `Renderable`:

```python
from string.templatelib import Template
from thtml_tstring import component, thtml

@component
def Button(*, children: str, kind: str = "primary") -> Template:
    return t'<button class="btn btn-{kind}">{children}</button>'

label = "Save"
result = thtml(t"<form><Button kind='success'>{label}</Button></form>")
html = result.render()
# <form><button class="btn btn-success">Save</button></form>
```

You can also wrap explicitly with `thtml()` instead of relying on the decorator:

```python
from thtml_tstring import Renderable

@component
def Button(*, children: str, kind: str = "primary") -> Renderable:
    return thtml(t'<button class="btn btn-{kind}">{children}</button>')
```

Both forms produce the same output.

## See also

- [HTML usage](../usage/html.md)
- [T-HTML usage](../usage/thtml.md)
- [Error Handling](../usage/error-handling.md)
- [API Reference](../reference/api.md)
