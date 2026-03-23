# T-strings for HTML

[![CI](https://github.com/koxudaxi/tstring-html/actions/workflows/ci.yml/badge.svg)](https://github.com/koxudaxi/tstring-html/actions/workflows/ci.yml)
[![PyPI - html-tstring](https://img.shields.io/pypi/v/html-tstring?label=html-tstring)](https://pypi.org/project/html-tstring/)
[![PyPI - thtml-tstring](https://img.shields.io/pypi/v/thtml-tstring?label=thtml-tstring)](https://pypi.org/project/thtml-tstring/)
[![Python 3.14+](https://img.shields.io/badge/python-3.14%2B-blue)](https://docs.python.org/3/whatsnew/3.14.html)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](https://github.com/koxudaxi/tstring-html/blob/main/LICENSE)

Parser-first HTML and T-HTML backends for [PEP 750](https://peps.python.org/pep-0750/) template strings.

Python 3.14 introduces t-strings. They look like f-strings but give you structured access to the interpolation values instead of concatenating them into a string. This project uses that structure to parse the template as HTML first, then insert escaped values into the right slots. The result is always valid HTML, and XSS is not possible.

## How it works

```python
from html_tstring import render_html

name = "<script>alert('xss')</script>"
page = render_html(t"<div class='greeting'>Hello, {name}!</div>")
# <div class='greeting'>Hello, &lt;script&gt;alert('xss')&lt;/script&gt;!</div>
```

The template is parsed into an AST. Interpolated values are validated and escaped, then placed into the AST. The rendered output is always well-formed HTML.

## What is T-HTML?

T-HTML is a small DSL on top of t-strings. It adds one rule: a tag whose name starts with an uppercase letter (e.g. `<Card>`) is treated as a component call. The tag name is looked up as a Python callable, attributes become keyword arguments, and nested content is passed as `children`.

There is no virtual DOM, no state, no build step. It is just a way to write reusable HTML fragments as functions and compose them with `<Tag>` syntax inside t-strings.

It borrows some familiar component-tag ergonomics, but it is not JSX at the parser/editor level.

```python
from string.templatelib import Template
from thtml_tstring import component, thtml

@component
def Badge(*, children: str, tone: str = "info") -> Template:
    return t'<span class="badge badge-{tone}">{children}</span>'

name = "cached"
result = thtml(t"<div><Badge tone='info'>{name}</Badge></div>")
html = result.render()
# <div><span class="badge badge-info">cached</span></div>
```

The `@component` decorator wraps `Template` return values into `Renderable` automatically, so you don't need `RawHtml` for component composition.

## Packages

| Package | What it does | Install |
|---------|--------------|---------|
| [html-tstring](https://pypi.org/project/html-tstring/) | Plain HTML rendering with auto-escaping | `pip install html-tstring` |
| [thtml-tstring](https://pypi.org/project/thtml-tstring/) | Adds component tags | `pip install thtml-tstring` |

`tstring-html-bindings` (the native extension) is pulled in automatically.

## See also

- [Installation](getting-started/installation.md)
- [Quick Start](getting-started/quick-start.md)
- [HTML usage](usage/html.md)
- [T-HTML components](usage/thtml.md)
- [Editor Integration (t-linter)](usage/editor-integration.md)
- [API Reference](reference/api.md)
- [Spec Conformance Status](reference/spec-conformance-status.md)
- [Architecture](architecture.md)

## Practical examples

The repository ships with typed examples that show realistic usage patterns:

- [HTML account settings](/Users/koudai/work/tstring-html/examples/html_account_settings.py)
- [HTML search results](/Users/koudai/work/tstring-html/examples/html_search_results.py)
- [T-HTML dashboard](/Users/koudai/work/tstring-html/examples/thtml_dashboard.py)
- [Basic HTML page](/Users/koudai/work/tstring-html/examples/html_page.py)
- [Basic T-HTML components](/Users/koudai/work/tstring-html/examples/thtml_components.py)

## Coverage snapshot

The current repo-local v1 conformance matrix includes:

- HTML `default`: 34 cases
- T-HTML `default`: 47 cases

That matrix currently covers:

- HTML escaping, `RawHtml`, `Renderable`, spread/class semantics, formatter raw-source fidelity, and seam-level semantic spans
- T-HTML component resolution, captured scope, explicit scope, `Renderable` composition, auto-wrap behavior, compiled-template scope behavior, and runtime-without-bindings boundaries
