# T-HTML (Components)

## What is T-HTML?

T-HTML is a small DSL on top of PEP 750 t-strings. It adds one rule to plain HTML templates: a tag whose name starts with an uppercase letter is treated as a component call instead of a literal HTML element.

When the renderer sees `<Card title="hello">`, it looks up `Card` as a Python callable, passes `title="hello"` as a keyword argument, normalizes the nested content, and passes it as `children`. That is the whole model. There is no virtual DOM, no reactivity, no build step.

The `thtml-tstring` package provides this on top of the same parser and escaping used by `html-tstring`.

T-HTML is not JSX at the parser/editor level. It borrows some familiar component-tag ergonomics, but it stays within PEP 750 template strings.

## Defining components

A component is any callable that takes keyword arguments and returns something renderable. With `@component`, you can return a `Template` directly:

```python
from string.templatelib import Template
from thtml_tstring import component

@component
def Button(*, children: str, kind: str = "primary") -> Template:
    return t'<button class="btn btn-{kind}">{children}</button>'
```

The `@component` decorator checks the return value. If it is a `Template`, the decorator auto-wraps it into a `Renderable` (the same thing `thtml()` produces). If it is already a `Renderable`, `RawHtml`, `str`, or another supported type, it passes through as-is.

The decorator is a no-op for typing purposes. `@component` is `@component(backend="thtml")` by default.

If a component returns a template that contains nested component tags, you can freeze explicit nested resolution with `@component(registry=...)`:

```python
@component
def Icon() -> Template:
    return t'<span class="icon" aria-hidden="true">*</span>'

@component(registry={"Icon": Icon})
def ToolbarButton(*, children: str) -> Template:
    return t'<button><Icon />{children}</button>'
```

## Renderable

`Renderable` is a shared type that represents safe, render-ready HTML. It is produced by `thtml()`, `html()` (from html-tstring), or the `@component` auto-wrap.

Unlike `RawHtml` (which takes an arbitrary external string on trust), a `Renderable` is always built from a parsed and validated template. The renderer can embed it as HTML without escaping.

```python
from thtml_tstring import thtml

# thtml() returns a Renderable
result = thtml(t"<div>hello</div>")
html = result.render()
```

## Rendering with `thtml()` and `render_html()`

`thtml()` creates a `Renderable`. Call `.render()` to get the HTML string, or pass it to `render_html()`:

```python
from html_tstring import render_html
from thtml_tstring import thtml

result = thtml(t"<Button kind='success'>Save</Button>")

# either works
html = result.render()
html = render_html(result)
```

The existing eager API `thtml_tstring.html()` still works and returns a `str` directly:

```python
from thtml_tstring import html

# eager, returns str
page = html(t"<Button kind='success'>Save</Button>")
```

## Component resolution

By default, component names are looked up in the caller's local and global scope:

```python
from thtml_tstring import thtml

# Button is found in the current scope automatically
result = thtml(t"<Button>Save</Button>")
```

For larger projects, prefer `registry=` so component resolution does not rely on ambient names:

```python
result = thtml(t"<Button>Save</Button>", registry={"Button": Button})
```

You can also pass the scope explicitly, which is useful in tests and framework integration:

```python
thtml(t"<Button>Save</Button>", globals={"Button": Button}, locals={})
```

Lookup order: `locals` first, then `globals`. The resolved name must be callable.

`registry=` is mutually exclusive with `globals=` / `locals=`. Mixed use raises `TypeError`. When `registry=` is supplied, T-HTML resolves component names only from that mapping and does not fall back to caller-frame inspection.

For `thtml()`, the scope is captured at creation time, not at render time. If the caller frame is not available, a `TemplateRuntimeError` is raised with a message to pass `globals=`/`locals=` explicitly.

## How components are called

```python
# Template: <Button kind="primary">{label}</Button>
# Call:     Button(kind="primary", children="Save")
```

Each attribute becomes a keyword argument. Children are normalized before they are passed as `children=`: strings stay strings, `Renderable` and `RawHtml` stay HTML-capable values, `Fragment` and iterables flatten recursively, and `None` disappears. Spread attributes are merged into the keyword arguments.

Attribute names are forwarded as-is from the template. `class`, `aria-label`, and other names that are not valid Python identifiers are not renamed automatically. Components that need those values should accept `**props`.

## Spread attributes on components

```python
from thtml_tstring import thtml

props = {"kind": "danger", "aria-label": "Delete"}
result = thtml(t"<Button {props}>Delete</Button>")
# Button(**{"kind": "danger", "aria-label": "Delete", "children": "Delete"})
```

## Component return values

Components can return:

- `Template` : auto-wrapped into `Renderable` by `@component`
- `Renderable` : passed through as-is
- `RawHtml` : inserted without escaping (for external trusted HTML)
- `str` : escaped as text (not raw HTML)
- `Fragment`, `list`, `tuple` : recursively flattened and rendered
- `None` : empty output
- `int`, `float`, `bool` : converted to escaped text

The recommended pattern is to return a `Template`:

```python
from string.templatelib import Template
from thtml_tstring import component

@component
def Stack(*, children: str) -> Template:
    return t"<section>{children}</section>"
```

If you need more control, return a `Renderable` explicitly:

```python
from thtml_tstring import Renderable, component, thtml

@component
def Stack(*, children: str) -> Renderable:
    return thtml(t"<section>{children}</section>")
```

Both produce the same output.

## Nested components

Components can contain other components:

```python
from string.templatelib import Template
from thtml_tstring import component, thtml

@component
def Card(*, children: str, title: str) -> Template:
    return t"""
        <div class="card">
          <h2>{title}</h2>
          <div class="card-body">{children}</div>
        </div>
    """

@component
def Badge(*, children: str) -> Template:
    return t'<span class="badge">{children}</span>'

label = "new"
result = thtml(t"<Card title='Status'><Badge>{label}</Badge></Card>")
html = result.render()
```

## Decorator vs explicit wrap

Use `@component` for the common case. The decorator auto-wraps `Template` returns, so component definitions stay short.

Use `thtml()` (or `html()` from html-tstring) explicitly when you need to:

- Control which scope is used for component resolution
- Fix the backend (`"html"` vs `"thtml"`)
- Build a `Renderable` outside a component function

```python
from thtml_tstring import Renderable

@component
def Badge(*, children: str, tone: str = "info") -> Renderable:
    # explicit scope control
    return thtml(
        t'<span class="badge badge-{tone}">{children}</span>',
        globals={"InnerComponent": InnerComponent},
    )
```

For larger component trees, prefer `registry=` over ambient lookup:

```python
result = thtml(
    t"<Badge tone='info'>ok</Badge>",
    registry={"Badge": Badge},
)
```

## RawHtml

`RawHtml` is for injecting external trusted HTML strings that did not come from a t-string template. Typical uses: SVG icons loaded from files, HTML from a sanitizer, third-party widget markup.

```python
from html_tstring import RawHtml, render_html

icon = RawHtml('<svg><circle r="5"/></svg>')
render_html(t"<div>{icon}</div>")
```

For component composition, use `@component` + `Template` returns or `thtml()` instead of `RawHtml`.

## Low-level APIs

`thtml-tstring` also exposes the same tooling functions as `html-tstring`:

```python
from thtml_tstring import check_template, format_template, compile_template

check_template(t"<Button />")   # validate T-HTML syntax
format_template(t"<Button />")  # canonical formatting
format_template(t"<Panel a='1' b='2'></Panel>", line_length=20)
```

## See also

- [HTML usage](html.md) for escaping, attributes, and spread details
- [Error Handling](error-handling.md)
- [API Reference](../reference/api.md)
