# T-strings for HTML

[![CI](https://github.com/koxudaxi/tstring-html/actions/workflows/ci.yml/badge.svg)](https://github.com/koxudaxi/tstring-html/actions/workflows/ci.yml)
[![PyPI - html-tstring](https://img.shields.io/pypi/v/html-tstring?label=html-tstring)](https://pypi.org/project/html-tstring/)
[![PyPI - thtml-tstring](https://img.shields.io/pypi/v/thtml-tstring?label=thtml-tstring)](https://pypi.org/project/thtml-tstring/)
[![Python 3.14+](https://img.shields.io/badge/python-3.14%2B-blue)](https://docs.python.org/3/whatsnew/3.14.html)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE)

> Maintainer update: Open to opportunities. [koxudaxi.dev](https://koxudaxi.dev/?utm_source=github_readme&utm_medium=top&utm_campaign=open_to_work)

Parser-first HTML and T-HTML backends for
[PEP 750](https://peps.python.org/pep-0750/) template strings.

Documentation: [tstring-html.koxudaxi.dev](https://tstring-html.koxudaxi.dev/)

Python 3.14 introduces t-strings. They look like f-strings but give you
structured access to the interpolation values instead of concatenating them
into a string. This project uses that structure to parse the template as HTML
first, then insert escaped values into the right slots. The result is always
valid HTML, and XSS is not possible.

## Packages

| Package | What it does |
|---------|--------------|
| **html-tstring** | Plain HTML rendering with auto-escaping |
| **thtml-tstring** | Adds component tags on top of html-tstring |

`tstring-html-bindings` (the native extension) is pulled in automatically.

## Installation

Requires Python 3.14+.

```bash
pip install html-tstring
pip install thtml-tstring
```

Or with uv:

```bash
uv add html-tstring
uv add thtml-tstring
```

Pre-built wheels are available for Linux x86_64, macOS Apple Silicon, and
Windows x86_64. On other platforms you need Rust 1.94.0+ to build
`tstring-html-bindings` from source.

## Quick start

### Safe HTML rendering

Interpolated values are HTML-escaped automatically:

```python
from html_tstring import render_html

name = "<script>alert('xss')</script>"
page = render_html(t"<div class='greeting'>Hello, {name}!</div>")
# <div class='greeting'>Hello, &lt;script&gt;alert('xss')&lt;/script&gt;!</div>
```

### Class normalization

The `class` attribute accepts strings, lists, and conditional dicts,
similar to `clsx` in the JS ecosystem:

```python
from html_tstring import render_html

classes = ["btn", {"btn-primary": True, "btn-disabled": False}]
page = render_html(t'<button class="{classes}">Click</button>')
# <button class="btn btn-primary">Click</button>
```

### Spread attributes

Pass a dict as a bare interpolation to spread it across the tag:

```python
from html_tstring import render_html

attrs = {"data-id": "42", "hidden": False, "class": "extra"}
page = render_html(t'<div class="base" {attrs}>content</div>')
# <div class="base extra" data-id="42">content</div>
```

## What is T-HTML?

T-HTML is a small DSL on top of t-strings. It adds one rule: a tag whose
name starts with an uppercase letter (e.g. `<Card>`) is treated as a
component call. The tag name is looked up as a Python callable, attributes
become keyword arguments, and nested content is normalized and passed as `children`.

It borrows some familiar component-tag ergonomics, but it is not JSX at the
parser/editor level.

There is no virtual DOM, no state management, no build step. It is just a
way to write reusable HTML fragments as functions and compose them with
familiar `<Tag>` syntax inside t-strings.

```python
from string.templatelib import Template
from thtml_tstring import component, thtml

@component
def Card(*, children: str, title: str) -> Template:
    return t"""
        <div class="card">
          <div class="card-header"><h3>{title}</h3></div>
          <div class="card-body">{children}</div>
        </div>
    """

@component
def Badge(*, children: str, tone: str = "info") -> Template:
    return t'<span class="badge badge-{tone}">{children}</span>'

@component
def Button(*, children: str, **props: object) -> Template:
    return t'<button {props}>{children}</button>'

# Compose them with component tags
user = "Alice"
status = "active"
result = thtml(t"""
<Card title='User Profile'>
  <p>Name: {user}</p>
  <p>Status: <Badge tone='success'>{status}</Badge></p>
  <Button type='submit'>Save</Button>
</Card>
""")

html = result.render()
```

The `@component` decorator wraps `Template` return values into a
`Renderable` automatically. You can also create a `Renderable` explicitly
with `thtml()` when you need to control scope or backend:

```python
from thtml_tstring import Renderable

@component
def Badge(*, children: str, tone: str = "info") -> Renderable:
    # explicit wrap, equivalent to the auto-wrap above
    return thtml(t'<span class="badge badge-{tone}">{children}</span>')
```

`RawHtml` still exists for injecting external trusted HTML strings, but
it is no longer needed for component composition.

Components are resolved from the caller's scope by default. For larger
projects, prefer `registry=` so resolution does not rely on ambient names:

```python
thtml(
    t"<Button>Save</Button>",
    registry={"Button": my_button_component},
)
```

You can still pass the scope explicitly for tests or framework integration:

```python
thtml(
    t"<Button>Save</Button>",
    globals={"Button": my_button_component},
    locals={},
)
```

`registry=` is mutually exclusive with `globals=` / `locals=`.

## Editor integration (t-linter)

[t-linter](https://github.com/koxudaxi/t-linter) is a linter, formatter, and
LSP server for t-strings. It uses the same Rust backends as this project for
`check` and `format`.

```bash
pip install t-linter
```

Check templates for errors:

```bash
t-linter check src/
```

Format HTML / T-HTML template literals:

```bash
t-linter format src/
```

Start the LSP server for real-time editor diagnostics:

```bash
t-linter lsp
```

A [VSCode extension](https://marketplace.visualstudio.com/items?itemName=koxudaxi.t-linter) is also available.

## Documentation

Full docs: [tstring-html.koxudaxi.dev](https://tstring-html.koxudaxi.dev)

- [Installation](https://tstring-html.koxudaxi.dev/getting-started/installation/)
- [Quick Start](https://tstring-html.koxudaxi.dev/getting-started/quick-start/)
- [HTML Usage](https://tstring-html.koxudaxi.dev/usage/html/)
- [T-HTML Components](https://tstring-html.koxudaxi.dev/usage/thtml/)
- [Editor Integration (t-linter)](https://tstring-html.koxudaxi.dev/usage/editor-integration/)
- [API Reference](https://tstring-html.koxudaxi.dev/reference/api/)
- [Spec Conformance Status](https://tstring-html.koxudaxi.dev/reference/spec-conformance-status/)
- [Architecture](https://tstring-html.koxudaxi.dev/architecture/)

## Practical examples

The repository includes typed examples that show how these APIs look in
real code, including spread attrs, `Renderable` composition, `@component`
auto-wrap, and explicit `thtml(...)` usage:

- [HTML account settings](/Users/koudai/work/tstring-html/examples/html_account_settings.py)
- [HTML search results](/Users/koudai/work/tstring-html/examples/html_search_results.py)
- [T-HTML dashboard](/Users/koudai/work/tstring-html/examples/thtml_dashboard.py)
- [Basic HTML page](/Users/koudai/work/tstring-html/examples/html_page.py)
- [Basic T-HTML components](/Users/koudai/work/tstring-html/examples/thtml_components.py)

## Conformance

This repository makes repo-local v1 conformance claims rather than claiming
full coverage of the external HTML ecosystem.

- HTML `default` profile: 34 manifest cases
- T-HTML `default` profile: 47 manifest cases

The current matrix covers parser/backend seam behavior, `Renderable`
composition, scope capture, formatter `raw_source` fidelity, semantic spans,
and runtime boundaries. See the live status page:

- [Spec Conformance Status](/Users/koudai/work/tstring-html/docs/reference/spec-conformance-status.md)
- [Conformance assets](/Users/koudai/work/tstring-html/conformance/README.md)

## Development

```bash
uv sync --group dev
cargo test --manifest-path rust/Cargo.toml --workspace --tests
uv run pytest -q
uv run coverage run -m pytest -q && uv run coverage report
uv run ruff check .
```

## License

MIT
