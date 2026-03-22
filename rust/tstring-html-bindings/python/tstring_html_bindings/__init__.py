from __future__ import annotations

from string.templatelib import Template
from typing import Literal

from .tstring_html_bindings import (
    CompiledHtmlTemplate,
    CompiledThtmlTemplate,
    Fragment,
    RawHtml,
    TemplateError,
    TemplateParseError,
    TemplateRuntimeError,
    TemplateSemanticError,
    check_html_template,
    check_thtml_template,
    compile_html_template,
    compile_thtml_template,
    format_html_template,
    format_thtml_template,
    render_html_fragment,
    render_html_template,
    render_thtml_template,
)


class Renderable:
    __slots__ = ("backend", "template", "_globals", "_locals")

    __tstring_renderable__ = True

    def __init__(
        self,
        backend: Literal["html", "thtml"],
        template: Template,
        *,
        globals: dict[str, object] | None = None,
        locals: dict[str, object] | None = None,
    ) -> None:
        if backend not in {"html", "thtml"}:
            raise ValueError("Renderable backend must be 'html' or 'thtml'.")
        self.backend = backend
        self.template = template
        self._globals = globals
        self._locals = locals

    def render(self) -> str:
        if self.backend == "html":
            return render_html_template(self.template)
        if self._globals is None or self._locals is None:
            raise TemplateRuntimeError(
                "T-HTML Renderable is missing captured globals/locals."
            )
        return render_thtml_template(
            self.template, globals=self._globals, locals=self._locals
        )

    def render_fragment(self) -> str:
        if self.backend == "html":
            return render_html_fragment(self.template)
        return self.render()

    def __repr__(self) -> str:
        return f"Renderable(backend={self.backend!r})"


__all__ = [
    "CompiledHtmlTemplate",
    "CompiledThtmlTemplate",
    "Fragment",
    "RawHtml",
    "Renderable",
    "TemplateError",
    "TemplateParseError",
    "TemplateRuntimeError",
    "TemplateSemanticError",
    "check_html_template",
    "check_thtml_template",
    "compile_html_template",
    "compile_thtml_template",
    "format_html_template",
    "format_thtml_template",
    "render_html_fragment",
    "render_html_template",
    "render_thtml_template",
]
