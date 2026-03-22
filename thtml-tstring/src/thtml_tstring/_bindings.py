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
CompiledThtmlTemplate = _bindings.CompiledThtmlTemplate

check_thtml_template = _bindings.check_thtml_template
format_thtml_template = _bindings.format_thtml_template
compile_thtml_template = _bindings.compile_thtml_template
render_thtml_template = _bindings.render_thtml_template

__all__ = [
    "CompiledThtmlTemplate",
    "Fragment",
    "RawHtml",
    "Renderable",
    "TemplateError",
    "TemplateParseError",
    "TemplateRuntimeError",
    "TemplateSemanticError",
    "check_thtml_template",
    "compile_thtml_template",
    "format_thtml_template",
    "render_thtml_template",
]
