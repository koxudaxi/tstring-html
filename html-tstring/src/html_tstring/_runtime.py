from __future__ import annotations

from string.templatelib import Template
from typing import Annotated, TypeIs, cast

from . import _bindings
from ._bindings import CompiledHtmlTemplate, Renderable

type HtmlTemplate = Annotated[Template, "html"]


def _is_template(value: object) -> TypeIs[Template]:
    return isinstance(value, Template)


def _validate_template(template: object, api_name: str) -> HtmlTemplate:
    if _is_template(template):
        return template
    raise TypeError(
        f"{api_name} requires a PEP 750 Template object. "
        f"Got {type(template).__name__} instead."
    )


def _is_renderable(value: object) -> TypeIs[Renderable]:
    return isinstance(value, Renderable)


def check_template(template: HtmlTemplate) -> None:
    _bindings.check_html_template(_validate_template(template, "check_template"))


def format_template(template: HtmlTemplate, *, line_length: int = 80) -> str:
    checked = _validate_template(template, "format_template")
    return _bindings.format_html_template(checked, line_length=line_length)


def compile_template(template: HtmlTemplate) -> CompiledHtmlTemplate:
    checked = _validate_template(template, "compile_template")
    return _bindings.compile_html_template(checked)


def html(template: HtmlTemplate) -> Renderable:
    return Renderable("html", _validate_template(template, "html"))


def render_html(template: HtmlTemplate | Renderable) -> str:
    if _is_renderable(template):
        return cast(Renderable, template).render()
    return _bindings.render_html_template(_validate_template(template, "render_html"))


def render_fragment(template: HtmlTemplate | Renderable) -> str:
    if _is_renderable(template):
        return cast(Renderable, template).render_fragment()
    checked = _validate_template(template, "render_fragment")
    return _bindings.render_html_fragment(checked)
