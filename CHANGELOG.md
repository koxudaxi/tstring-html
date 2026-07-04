# Changelog

All notable changes to this project are documented in this file.
## [0.1.0](https://github.com/koxudaxi/tstring-html/releases/tag/0.1.0) - 2026-03-22

**Full Changelog**: https://github.com/koxudaxi/tstring-html/commits/0.1.0

---
## [0.2.0](https://github.com/koxudaxi/tstring-html/releases/tag/0.2.0) - 2026-07-04

## Breaking Changes

### Default Runtime Behavior Changes
* Spread attribute names are now validated - Spread attributes (e.g., `{attrs}` in `<div {attrs}>`) now validate that all keys conform to valid HTML attribute name grammar. Invalid attribute names (containing spaces, special characters, or other non-standard characters) will raise `TemplateSemanticError` for HTML templates or `TemplateRuntimeError` for T-HTML component spreads. This security improvement prevents XSS injection via malformed attribute names but may break code that previously passed invalid keys. (#13)
  ```python
  # Previously worked (but was a security vulnerability)
  # Now raises TemplateSemanticError
  attrs = {"x onmouseover=alert(1)": "y"}
  render_html(t"<div {attrs}></div>")
  ```
* Dangerous URL schemes now rejected with TemplateSemanticError - URL attributes (`href`, `src`, `action`, `formaction`, `xlink:href`, `cite`, `poster`, `background`, `manifest`) now reject `javascript:`, `vbscript:`, and `data:` URL schemes. This applies to both direct attribute values and spread attributes, and normalizes whitespace/control characters before checking:
```python
href = "javascript:alert(1)"
render_html(t'<a href="{href}">x</a>')  # Now raises TemplateSemanticError

# Also catches obfuscation attempts
href = "java\n script:alert(1)"  # Still detected and rejected
```
(#14)

### Rendered Output Changes
* Attribute ampersand escaping now always escapes ampersands - Previously, entity-looking text like `&amp;` in attribute values was preserved as-is. Now it is always escaped to `&amp;amp;`. For example, `title="Tom &amp; Jerry"` now renders as `title="Tom &amp;amp; Jerry"`. (#14)
* Formatter uses single quotes for attributes containing double quotes - When formatting templates with `format_template()`, attributes containing double quotes now use single-quote delimiters instead of escaping with `&quot;`. For example, `<div title="say &quot;hi&quot;">` is now formatted as `<div title='say "hi"'>`. (#14)
* SVG element case normalization - SVG tag names and attributes are now normalized to their canonical camelCase form (e.g., `clippath` → `clipPath`, `viewbox` → `viewBox`). Templates containing SVG elements with lowercase names will produce different output. (#16)
* SVG self-closing tags expanded - SVG elements using self-closing syntax (e.g., `<rect />`) now render with explicit close tags (e.g., `<rect></rect>`). This affects all non-void elements within SVG namespace. (#16)

### Python API Changes
* Raw Template values as children now raise TemplateRuntimeError - Passing a raw `Template` (t-string) value directly as a child interpolation now raises `TemplateRuntimeError`. Users must explicitly wrap with `html()` or `thtml()` first:
```python
# Before: may have produced undefined output
child = t"<span>x</span>"
render_html(t"<div>{child}</div>")  # Now raises TemplateRuntimeError

# After: explicit wrapping required
render_html(t"<div>{html(child)}</div>")
```
(#14)
* bytes and bytearray values as children now raise TemplateRuntimeError - Binary values are now rejected at runtime instead of being rendered as integer sequences:
```python
# Before: might have rendered as "[65, 66]" or similar
render_html(t"<div>{b'AB'}</div>")  # Now raises TemplateRuntimeError
```
(#14)
* Conversion or format_spec on structured values now raises TemplateRuntimeError - Using PEP 750 conversions (`!r`, `!s`, `!a`) or format specifications on structured values (`Renderable`, `RawHtml`, `Fragment`, `dict`, iterables) now raises `TemplateRuntimeError`:
```python
raw = RawHtml("<b>x</b>")
render_html(t"<div>{raw!r}</div>")  # Now raises TemplateRuntimeError
```
(#14)

### Rust Backend Changes
* Component tag mismatch validation deferred to runtime - Templates with different component start and end expressions (e.g., `<{Card}></{Alias}>`) are no longer rejected at parse time with `TemplateParseError`. Instead, these parse successfully and validation is deferred to runtime. Code relying on static detection of mismatched component tags must now handle this at runtime. (#16)

## What's Changed
* Reject invalid spread attribute names by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/13
* Harden HTML runtime rendering by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/14
* Publish packages from releases by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/15
* Support tdom SVG context by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/16


**Full Changelog**: https://github.com/koxudaxi/tstring-html/compare/0.1.10...0.2.0

---

## [0.1.10](https://github.com/koxudaxi/tstring-html/releases/tag/0.1.10) - 2026-07-02

## What's Changed
* Bump tstring-syntax to 0.2.2 by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/12


**Full Changelog**: https://github.com/koxudaxi/tstring-html/compare/0.1.9...0.1.10

---

## [0.1.9](https://github.com/koxudaxi/tstring-html/releases/tag/0.1.9) - 2026-03-24

## What's Changed
* Publish tdom crate by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/11


**Full Changelog**: https://github.com/koxudaxi/tstring-html/compare/0.1.8...0.1.9

---

## [0.1.8](https://github.com/koxudaxi/tstring-html/releases/tag/0.1.8) - 2026-03-24

## What's Changed
* Add tdom backend by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/10


**Full Changelog**: https://github.com/koxudaxi/tstring-html/compare/0.1.7...0.1.8

---

## [0.1.7](https://github.com/koxudaxi/tstring-html/releases/tag/0.1.7) - 2026-03-24

## What's Changed
* Add explicit T-HTML registries by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/9


**Full Changelog**: https://github.com/koxudaxi/tstring-html/compare/0.1.6...0.1.7

---

## [0.1.6](https://github.com/koxudaxi/tstring-html/releases/tag/0.1.6) - 2026-03-23

## What's Changed
* Allow title interpolation by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/6
* Harden title tag checks by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/7


**Full Changelog**: https://github.com/koxudaxi/tstring-html/compare/0.1.5...0.1.6

---

## [0.1.5](https://github.com/koxudaxi/tstring-html/releases/tag/0.1.5) - 2026-03-23

## What's Changed
* Fix Rust publish dependency order by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/5


**Full Changelog**: https://github.com/koxudaxi/tstring-html/compare/0.1.4...0.1.5

---

## [0.1.4](https://github.com/koxudaxi/tstring-html/releases/tag/0.1.4) - 2026-03-23

## What's Changed
* docs: add documentation link to README.md by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/1
* docs: add t-linter section to README by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/2
* Add doc-based formatter by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/3
* Align release draft with structured-data by @koxudaxi in https://github.com/koxudaxi/tstring-html/pull/4

## New Contributors
* @koxudaxi made their first contribution in https://github.com/koxudaxi/tstring-html/pull/1

**Full Changelog**: https://github.com/koxudaxi/tstring-html/compare/0.1.3...0.1.4

---

## [0.1.3](https://github.com/koxudaxi/tstring-html/releases/tag/0.1.3) - 2026-03-22

**Full Changelog**: https://github.com/koxudaxi/tstring-html/compare/0.1.1...0.1.3

---

## [0.1.1](https://github.com/koxudaxi/tstring-html/releases/tag/0.1.1) - 2026-03-22

**Full Changelog**: https://github.com/koxudaxi/tstring-html/compare/0.1.0...0.1.1

---


