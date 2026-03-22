# API Reference

Both packages expose similar interfaces. The T-HTML package adds component-aware rendering and the `Renderable` type.

All functions that accept a template take a `string.templatelib.Template` (the object produced by a `t"..."` literal). The wrapper packages define type aliases `HtmlTemplate` and `ThtmlTemplate` as `Annotated[Template, "html"]` / `Annotated[Template, "thtml"]` for documentation purposes, but at runtime any `Template` is accepted.

## string.templatelib (standard library)

For reference, PEP 750 defines these types in `string.templatelib`:

```python
class Template:
    strings: tuple[str, ...]
    interpolations: tuple[Interpolation, ...]

    @property
    def values(self) -> tuple[object, ...]: ...
    def __iter__(self) -> Iterator[str | Interpolation]: ...

class Interpolation:
    value: object
    expression: str
    conversion: Literal["a", "r", "s"] | None
    format_spec: str
```

A `t"Hello {name}"` literal produces a `Template` with `strings=("Hello ", "")` and one `Interpolation` whose `value` is the runtime value of `name`.

## html-tstring

### `render_html`

Parse a `Template` (or render a `Renderable`) and return safe HTML.

```python
from html_tstring import render_html

def render_html(template: Template | Renderable) -> str: ...
```

### `render_fragment`

Render a `Template` or `Renderable` as an HTML fragment.

```python
from html_tstring import render_fragment

def render_fragment(template: Template | Renderable) -> str: ...
```

### `html`

Create a `Renderable` from a plain HTML template. This is the deferred counterpart to `render_html`.

```python
from html_tstring import html

def html(template: Template) -> Renderable: ...
```

### `check_template`

Validate template syntax and semantics without rendering.

```python
from html_tstring import check_template

def check_template(template: Template) -> None: ...
```

Raises `TemplateParseError` or `TemplateSemanticError` on failure.

### `format_template`

Reconstruct the template source with canonical formatting.

```python
from html_tstring import format_template

def format_template(template: Template, *, line_length: int = 80) -> str: ...
```

`line_length` only affects formatting. The Python API does not read project config.

### `compile_template`

Pre-compile a template for repeated rendering with different values.

```python
from html_tstring import compile_template

def compile_template(template: Template) -> CompiledHtmlTemplate: ...
```

`CompiledHtmlTemplate` methods:

```python
def render(self, values: list[object]) -> str: ...
def render_fragment(self, values: list[object]) -> str: ...
```

## thtml-tstring

### `thtml`

Create a `Renderable` from a T-HTML template. Component scope is captured at creation time.

```python
from thtml_tstring import thtml

def thtml(
    template: Template,
    *,
    globals: dict[str, Any] | None = None,
    locals: dict[str, Any] | None = None,
) -> Renderable: ...
```

| Parameter | Type | Description |
|-----------|------|-------------|
| `template` | `Template` | A PEP 750 `t"..."` literal |
| `globals` | `dict[str, Any] \| None` | Globals for component resolution. Default: caller's `f_globals` |
| `locals` | `dict[str, Any] \| None` | Locals for component resolution. Default: caller's `f_locals` |

Scope is captured once at creation time and frozen. Rendering does not re-inspect the caller frame.

### `html`

Eager string renderer. Returns a `str` directly (not a `Renderable`).

```python
from thtml_tstring import html

def html(
    template: Template,
    *,
    globals: dict[str, Any] | None = None,
    locals: dict[str, Any] | None = None,
) -> str: ...
```

This is the existing eager API, kept for backward compatibility.

### `render_html`

Alias for `html()` with the same signature.

### `check_template`

Validate T-HTML template syntax and semantics.

```python
from thtml_tstring import check_template

def check_template(template: Template) -> None: ...
```

### `format_template`

Reconstruct T-HTML template source with canonical formatting.

```python
from thtml_tstring import format_template

def format_template(template: Template, *, line_length: int = 80) -> str: ...
```

`line_length` only affects formatting. The Python API does not read project config.

### `compile_template`

Pre-compile a T-HTML template.

```python
from thtml_tstring import compile_template

def compile_template(template: Template) -> CompiledThtmlTemplate: ...
```

`CompiledThtmlTemplate` methods:

```python
def render(
    self,
    values: list[object],
    globals: dict[str, Any] | None = None,
    locals: dict[str, Any] | None = None,
) -> str: ...
```

### `component`

Decorator for component functions. Supports bare form and factory form.

```python
from thtml_tstring import component

# bare form (default backend="thtml")
@component
def MyComponent(*, children: str, **props: object) -> Template:
    return t"<div>{children}</div>"

# factory form
@component(backend="thtml")
def MyComponent(*, children: str, **props: object) -> Template:
    return t"<div>{children}</div>"
```

When the decorated function returns a `Template`, the decorator wraps it into a `Renderable`. Other return types (`Renderable`, `RawHtml`, `str`, `Fragment`, `list`, `None`, scalars) pass through unchanged.

Attribute names are forwarded literally from template syntax. In v1, names like `class`,
`aria-label`, and `data-id` are not remapped to Python identifiers such as `class_` or
`aria_label`, so components that need those values should accept `**props`.

### `spread`

Identity function for explicit spread documentation:

```python
from thtml_tstring import spread

def spread(value: Mapping[str, Any]) -> Mapping[str, Any]: ...
```

## Shared Types

### `Renderable`

Represents safe, render-ready HTML built from a parsed template. Produced by `thtml()`, `html()` (from html-tstring), or the `@component` auto-wrap.

```python
from thtml_tstring import Renderable  # or from html_tstring

class Renderable:
    def render(self) -> str: ...
```

The renderer embeds `Renderable` values as HTML without escaping, because they are always built from validated templates. This is different from `RawHtml`, which takes an arbitrary external string on trust.

### `Fragment`

Container for multiple child values:

```python
from html_tstring import Fragment  # or from thtml_tstring

class Fragment:
    items: list[object]
    def __init__(self, items: list[object]) -> None: ...
```

### `RawHtml`

Escape hatch for injecting external trusted HTML strings. Use for SVG icons, sanitizer output, third-party widget markup. Not needed for component composition.

```python
from html_tstring import RawHtml  # or from thtml_tstring

class RawHtml:
    value: str
    def __init__(self, value: str) -> None: ...
```

### Type Aliases

| Type | Definition | Package |
|------|-----------|---------|
| `HtmlTemplate` | `Annotated[Template, "html"]` | `html_tstring` |
| `ThtmlTemplate` | `Annotated[Template, "thtml"]` | `thtml_tstring` |

These are for documentation and type-checker hints only. At runtime any `Template` is accepted.

## Exceptions

```
TemplateError
├── TemplateParseError      : template syntax is invalid
├── TemplateSemanticError   : valid syntax but invalid usage
└── TemplateRuntimeError    : runtime failure (T-HTML component resolution, scope capture)
```

| Exception | When |
|-----------|------|
| `TemplateParseError` | Template HTML is syntactically invalid (unclosed tag, mismatched closing tag) |
| `TemplateSemanticError` | Valid syntax but invalid usage (component in HTML backend, raw-text interpolation) |
| `TemplateRuntimeError` | Runtime failure in the T-HTML bindings layer (unknown component, non-callable component, scope capture failure) |

HTML render-time contract violations such as invalid spread types or unsupported `class` values are
currently surfaced as `TemplateSemanticError`, because they originate from the Rust backend's
semantic error channel.

All exceptions are defined in `tstring-html-bindings` and re-exported by both packages.
