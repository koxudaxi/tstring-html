from __future__ import annotations

import sys
from functools import wraps
from string.templatelib import Template
from typing import Annotated, Any, Callable, Mapping, TypeIs, TypeVar, cast

from . import _bindings
from ._bindings import CompiledThtmlTemplate, Renderable, TemplateRuntimeError

type ThtmlTemplate = Annotated[Template, "thtml"]
ComponentT = TypeVar("ComponentT", bound=Callable[..., object])


def _is_template(value: object) -> TypeIs[Template]:
    return isinstance(value, Template)


def _is_renderable(value: object) -> TypeIs[Renderable]:
    return isinstance(value, Renderable)


def _validate_template(template: object, api_name: str) -> ThtmlTemplate:
    if _is_template(template):
        return template
    raise TypeError(
        f"{api_name} requires a PEP 750 Template object. "
        f"Got {type(template).__name__} instead."
    )


def _capture_scope(
    globals: Mapping[str, Any] | None,
    locals: Mapping[str, Any] | None,
    *,
    frame_depth: int,
) -> tuple[dict[str, Any], dict[str, Any]]:
    if globals is not None and locals is not None:
        return dict(globals), dict(locals)

    try:
        frame = sys._getframe(frame_depth)
    except AttributeError, ValueError:
        raise TemplateRuntimeError(
            "Caller-frame inspection failed. "
            "On runtimes without frame inspection you must pass registry=, "
            "or both globals= and locals= explicitly."
        ) from None

    captured_globals = dict(globals) if globals is not None else dict(frame.f_globals)
    captured_locals = dict(locals) if locals is not None else dict(frame.f_locals)
    return captured_globals, captured_locals


def _normalize_scope_overrides(
    *,
    globals: Mapping[str, Any] | None,
    locals: Mapping[str, Any] | None,
    registry: Mapping[str, object] | None,
    api_name: str,
) -> tuple[Mapping[str, Any] | None, Mapping[str, Any] | None]:
    if registry is not None:
        if globals is not None or locals is not None:
            raise TypeError(
                f"{api_name} does not allow combining registry= "
                "with globals= or locals=."
            )
        return dict(registry), {}
    return globals, locals


def _make_thtml_renderable(
    template: object,
    *,
    globals: Mapping[str, Any] | None,
    locals: Mapping[str, Any] | None,
    frame_depth: int,
    api_name: str,
) -> Renderable:
    checked = _validate_template(template, api_name)
    captured_globals, captured_locals = _capture_scope(
        globals,
        locals,
        frame_depth=frame_depth,
    )
    return Renderable(
        "thtml",
        checked,
        globals=captured_globals,
        locals=captured_locals,
    )


def component(
    func: ComponentT | None = None,
    *,
    backend: str = "thtml",
    registry: Mapping[str, object] | None = None,
) -> ComponentT | Callable[[ComponentT], ComponentT]:
    if backend not in {"thtml", "html"}:
        raise ValueError("component backend must be 'thtml' or 'html'.")
    if backend == "html" and registry is not None:
        raise ValueError("component registry= is only supported for backend='thtml'.")

    captured_registry = dict(registry) if registry is not None else None

    def decorator(inner: ComponentT) -> ComponentT:
        @wraps(inner)
        def wrapped(*args: Any, **kwargs: Any) -> object:
            result = inner(*args, **kwargs)
            if _is_template(result):
                if backend == "thtml":
                    resolved_globals = (
                        captured_registry
                        if captured_registry is not None
                        else cast(Mapping[str, Any], inner.__globals__)
                    )
                    return _make_thtml_renderable(
                        result,
                        globals=resolved_globals,
                        locals={},
                        frame_depth=0,
                        api_name="component",
                    )
                from html_tstring import html as html_renderable

                return html_renderable(result)
            return result

        return cast(ComponentT, wrapped)

    if func is None:
        return decorator
    return decorator(func)


def spread(value: Mapping[str, Any]) -> Mapping[str, Any]:
    return value


def check_template(template: ThtmlTemplate) -> None:
    _bindings.check_thtml_template(_validate_template(template, "check_template"))


def format_template(template: ThtmlTemplate, *, line_length: int = 80) -> str:
    checked = _validate_template(template, "format_template")
    return _bindings.format_thtml_template(checked, line_length=line_length)


def compile_template(template: ThtmlTemplate) -> CompiledThtmlTemplate:
    checked = _validate_template(template, "compile_template")
    return _bindings.compile_thtml_template(checked)


def thtml(
    template: ThtmlTemplate,
    *,
    globals: Mapping[str, Any] | None = None,
    locals: Mapping[str, Any] | None = None,
    registry: Mapping[str, object] | None = None,
) -> Renderable:
    normalized_globals, normalized_locals = _normalize_scope_overrides(
        globals=globals,
        locals=locals,
        registry=registry,
        api_name="thtml",
    )
    return _make_thtml_renderable(
        template,
        globals=normalized_globals,
        locals=normalized_locals,
        frame_depth=3,
        api_name="thtml",
    )


def render_html(
    template: ThtmlTemplate | Renderable,
    *,
    globals: Mapping[str, Any] | None = None,
    locals: Mapping[str, Any] | None = None,
    registry: Mapping[str, object] | None = None,
) -> str:
    if _is_renderable(template):
        return cast(Renderable, template).render()

    checked = _validate_template(template, "render_html")
    normalized_globals, normalized_locals = _normalize_scope_overrides(
        globals=globals,
        locals=locals,
        registry=registry,
        api_name="render_html",
    )
    captured_globals, captured_locals = _capture_scope(
        normalized_globals,
        normalized_locals,
        frame_depth=2,
    )
    return _bindings.render_thtml_template(
        checked,
        globals=captured_globals,
        locals=captured_locals,
    )


def html(
    template: ThtmlTemplate | Renderable,
    *,
    globals: Mapping[str, Any] | None = None,
    locals: Mapping[str, Any] | None = None,
    registry: Mapping[str, object] | None = None,
) -> str:
    if _is_renderable(template):
        return cast(Renderable, template).render()
    checked = _validate_template(template, "html")
    normalized_globals, normalized_locals = _normalize_scope_overrides(
        globals=globals,
        locals=locals,
        registry=registry,
        api_name="html",
    )
    captured_globals, captured_locals = _capture_scope(
        normalized_globals,
        normalized_locals,
        frame_depth=2,
    )
    return _bindings.render_thtml_template(
        checked,
        globals=captured_globals,
        locals=captured_locals,
    )
