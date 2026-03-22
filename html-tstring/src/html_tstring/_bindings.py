from __future__ import annotations

try:
    import tstring_html_bindings as _bindings
except ImportError:  # pragma: no cover
    from tstring_html_bindings import _bindings  # type: ignore[attr-defined]

TemplateError = _bindings.TemplateError
TemplateParseError = _bindings.TemplateParseError
TemplateSemanticError = _bindings.TemplateSemanticError
TemplateRuntimeError = _bindings.TemplateRuntimeError
Fragment = _bindings.Fragment
RawHtml = _bindings.RawHtml
Renderable = _bindings.Renderable
CompiledHtmlTemplate = _bindings.CompiledHtmlTemplate

check_html_template = _bindings.check_html_template
format_html_template = _bindings.format_html_template
compile_html_template = _bindings.compile_html_template
render_html_template = _bindings.render_html_template
render_html_fragment = _bindings.render_html_fragment

__all__ = [
    "CompiledHtmlTemplate",
    "Fragment",
    "RawHtml",
    "Renderable",
    "TemplateError",
    "TemplateParseError",
    "TemplateRuntimeError",
    "TemplateSemanticError",
    "check_html_template",
    "compile_html_template",
    "format_html_template",
    "render_html_fragment",
    "render_html_template",
]
