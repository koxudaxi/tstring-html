# Architecture

## Overview

[PEP 750](https://peps.python.org/pep-0750/) template strings for HTML and T-HTML.

The template is parsed into an AST. Interpolated values are validated and escaped, then placed into the AST. The output is always well-formed HTML, and XSS is not possible.

```python
from html_tstring import render_html

name = "<script>alert('xss')</script>"
safe = render_html(t"<div>{name}</div>")
# <div>&lt;script&gt;alert('xss')&lt;/script&gt;</div>
```

## Layers

```
┌─────────────────────────────────────────────────────────┐
│  Python API                                             │
│  html-tstring / thtml-tstring                           │
│  render_html() / html() / thtml() / @component          │
└──────────────────────┬──────────────────────────────────┘
                       │ PEP 750 Template / Renderable
┌──────────────────────▼──────────────────────────────────┐
│  PyO3 Bindings (tstring-html-bindings)                  │
│  Template extraction, LRU cache, runtime bridging       │
│  Fragment, RawHtml, Renderable, exception classes        │
└──────────────────────┬──────────────────────────────────┘
                       │ TemplateInput + RuntimeContext
┌──────────────────────▼──────────────────────────────────┐
│  Rust Parsers                                           │
│  tstring-html-rs (HTML) / tstring-thtml-rs (T-HTML)     │
│  ↕                                                      │
│  tstring-syntax (spans, diagnostics, TemplateInput)     │
└─────────────────────────────────────────────────────────┘
```

### Python API (html-tstring, thtml-tstring)

Thin wrappers over the Rust bindings. They check that the input is a `Template`, call into Rust, and return the result:

```python
# html-tstring
render_html(template | renderable) -> str
render_fragment(template | renderable) -> str
html(template) -> Renderable          # deferred
check_template(template) -> None
format_template(template, *, line_length=80) -> str
compile_template(template) -> CompiledHtmlTemplate

# thtml-tstring
thtml(template, *, registry=None, globals=None, locals=None) -> Renderable  # deferred
html(template | renderable, *, registry=None, globals=None, locals=None) -> str  # eager
render_html(template | renderable, *, registry=None, globals=None, locals=None) -> str  # eager
check_template(template) -> None
format_template(template, *, line_length=80) -> str
compile_template(template) -> CompiledThtmlTemplate
```

### Renderable

`Renderable` is a shared Python type that represents safe, render-ready HTML built from a parsed template. It is produced by `thtml()`, `html()` (from html-tstring), or the `@component` decorator's auto-wrap.

Unlike `RawHtml` (which takes an arbitrary external string on trust), `Renderable` is always built from a validated template. The renderer embeds it as HTML without escaping.

`Renderable` holds:

- The backend kind (`"html"` or `"thtml"`)
- The original PEP 750 `Template`
- For T-HTML: captured `globals` / `locals` for component resolution

The `@component` decorator checks the return value of the wrapped function. If it is a `Template`, the decorator converts it to a `Renderable`. Other types pass through.

### PyO3 Bindings (tstring-html-bindings)

One crate that is both the PyO3 adapter and the maturin build target:

| Responsibility | Details |
| --- | --- |
| Template extraction | Converts PEP 750 `Template` to `TemplateInput` with runtime values |
| Parse cache | 256-entry LRU per backend (HTML / T-HTML) |
| Component resolution | Frame inspection for caller scope, explicit override |
| Shared primitives | `Fragment`, `RawHtml`, `Renderable`, exception hierarchy |
| Contract | `__contract_version__ = 2` with symbol list |

When the bindings encounter a `Renderable` as a runtime value (in children, component returns, etc.), they call `.render()` and treat the result as safe HTML, similar to `RawHtml`.

### Rust Parsers

**tstring-syntax** (external crate) defines shared types:

```rust
struct TemplateInput { segments: Vec<TemplateSegment> }
struct TemplateInterpolation { expression, conversion, format_spec, raw_source }
struct SourceSpan { start: SourcePosition, end: SourcePosition }
enum ErrorKind { Parse, Semantic, Unrepresentable }
```

**tstring-html-rs** is the core crate:

| Part | What it does |
| --- | --- |
| Lexer/Parser | Token-based streaming parser, classifies tags as HTML or component |
| AST | Document, Element, ComponentTag, Attribute, SpreadAttribute, RawTextElement, etc. |
| Validation | Rejects component tags, unquoted dynamic attrs, and raw-text interpolation except for `<title>` |
| Formatter | Reconstructs source from AST using `raw_source` |
| Renderer | HTML escaping, class normalization, spread merge, boolean attrs |
| Escaping | `&amp;` `&lt;` `&gt;` `&quot;` |

**tstring-thtml-rs** adds component semantics:

| Part | What it does |
| --- | --- |
| Validation | Allows component tags, validates attributes, and applies the same raw-text rules as HTML |
| Formatting | T-HTML formatter with component syntax |
| Compile | `CompiledThtmlTemplate` with the parsed document |

`Renderable` lives entirely in the Python / bindings layer. The Rust backend does not know about it.

## Data Flow

```
Template (Python t-string)
  → extract strings + interpolations (PyO3)
  → build TemplateInput + RuntimeContext
  → Rust parser: build HTML AST with interpolation slots
  → semantic validation
  → render: walk AST, escape values, normalize attributes
  → output: safe HTML string
```

For T-HTML with components:

```
Template (Python t-string)
  → extract + parse (same as HTML)
  → walk AST in bindings layer
  → ComponentTag → resolve callable from captured scope
  → call component(**props, children=rendered_children)
  → if return is Template → @component wraps to Renderable → .render()
  → normalize return value → render to HTML
  → non-component nodes → delegate to HTML renderer
```

## Project Layout

```
rust/
  tstring-html-rs/           HTML parser, AST, renderer, formatter
  tstring-thtml-rs/          T-HTML semantic layer
  tstring-html-bindings/     PyO3 extension module + shared Python types
  backend-e2e-tests/         Cross-crate integration tests
html-tstring/                Python HTML package
thtml-tstring/               Python T-HTML package
conformance/                 Conformance manifests
examples/                    Runnable examples
```

## Design Decisions

### Why parse first?

String concatenation can produce broken HTML and is vulnerable to XSS. Parsing the template before inserting values makes both problems impossible.

### Renderable vs RawHtml

Both bypass escaping when embedded in HTML, but they serve different purposes:

- `Renderable` is produced by the renderer itself (via `thtml()`, `html()`, or `@component`). It is always built from a parsed, validated template. Safe by construction.
- `RawHtml` is an escape hatch for external HTML strings that the user guarantees are safe. No parsing or validation happens.

Before `Renderable`, component composition required `RawHtml(render_html(t"..."))`. Now components just return `t"..."` and the decorator handles the rest.

### AST-direct rendering

In v1, `CompiledHtmlTemplate` and `CompiledThtmlTemplate` wrap the parsed AST directly. Rendering walks the AST on each call. A compiled IR with static text ops and dynamic slots is a future optimization. The opaque `CompiledTemplate` types make this a non-breaking change.

That AST/IR cost is intentional for SSR, static generation, validation, and future AOT-style optimization. Browser-side environments can have a different tradeoff because they already have a native HTML parser and `<template>` element available.

### Parse cache

`template.strings` (the static parts) is the cache key. The same template with different values reuses the parsed AST. 256-entry LRU per backend.

### Component resolution and scope capture

T-HTML components are resolved by name. `thtml()` captures the caller's scope at creation time using `sys._getframe(1)`. The `@component` decorator uses the decorated function's module globals as default scope.

Explicit `registry=` is available as the preferred large-project path for component resolution. `globals=`/`locals=` overrides remain available for tests and framework integration. The scope is frozen at creation time and not re-inspected at render time.

### External tool integration

When the backend is used outside the Python runtime (e.g. by t-linter), callers provide `TemplateInput` directly:

- `check_template` validates structure and returns diagnostics with source spans
- `format_template` re-renders canonical HTML/T-HTML, preserving interpolation source from `raw_source`
- Missing `raw_source` is a semantic error

### Error model

```
TemplateError
├── TemplateParseError
├── TemplateSemanticError
└── TemplateRuntimeError
```

Errors carry `Diagnostic` objects with source spans and expression labels.

## Dependencies

### Python

```
html-tstring  ──┐
                 ├──▶ tstring-html-bindings
thtml-tstring ──┘
```

### Rust

```
tstring-html-bindings
  ├─ tstring-html-rs ─── tstring-syntax
  └─ tstring-thtml-rs ── tstring-html-rs + tstring-syntax
```

Notable versions: pyo3 0.27.1, tstring-syntax 0.2.1, Rust edition 2024.

## Escaping Policy

| Context | Escaping |
| --- | --- |
| Text children | `&` `<` `>` |
| Attribute values | `&` `<` `>` `"` (always double-quoted) |
| `RawHtml` | No escaping (external trusted content) |
| `Renderable` | No escaping (renderer-built safe HTML) |
| Raw-text elements | `<title>` interpolation allowed as escaped text; `<script>`, `<style>`, and `<textarea>` still reject interpolation |

## Testing

```bash
cargo test --manifest-path rust/Cargo.toml --workspace --tests
uv run pytest -q
uv run ruff check .
```

## Build & CI

| Workflow | Trigger | What it does |
| --- | --- | --- |
| `ci.yml` | push, PR | test, lint, coverage, build, twine check |
| `docs.yaml` | push, PR (docs), release | build docs, deploy to Cloudflare Pages |
| `publish-python.yml` | tag push | builds Python artifacts, then publishes `tstring-html-bindings`, `html-tstring`, and `thtml-tstring` via separate Trusted Publisher environments |
| `publish-rust.yml` | tag push | crates.io |
