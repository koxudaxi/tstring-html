from html_tstring import (
    TemplateParseError,
    TemplateRuntimeError,
    TemplateSemanticError,
    check_template,
    compile_template,
    format_template,
    render_html,
)


def test_adjacent_interpolations_preserve_boundary_shape() -> None:
    first = "a"
    second = "b"
    assert render_html(t"{first}{second}") == "ab"


def test_leading_and_trailing_interpolations_render_correctly() -> None:
    first = "a"
    second = "b"
    assert render_html(t"{first}<span>{second}</span>") == "a<span>b</span>"


def test_cache_hit_uses_current_expression_label() -> None:
    first = "a"
    second = "b"

    compile_template(t"<div>{first}</div>")
    compiled = compile_template(t"<div>{second}</div>")

    try:
        compiled.render([])
    except TemplateSemanticError as exc:
        assert "second" in str(exc)
    else:
        raise AssertionError("expected missing runtime value error")


def test_parse_semantic_and_runtime_error_shapes() -> None:
    title = "safe"
    script = "alert('x')"

    try:
        check_template(t"<div")
    except TemplateParseError:
        pass
    else:
        raise AssertionError("expected malformed start tag parse failure")

    try:
        assert format_template(t"<div>{title}</div>") == "<div>{title}</div>"
    except TemplateSemanticError as exc:
        raise AssertionError(f"unexpected formatting failure: {exc}") from exc

    try:
        render_html(t"<div title={title}></div>")
    except TemplateSemanticError:
        pass
    else:
        raise AssertionError("expected unquoted attr rejection")

    try:
        check = render_html(t"<script>{script}</script>")
    except TemplateSemanticError:
        check = None
    else:
        raise AssertionError(f"expected raw-text rejection, got {check!r}")

    assert render_html(t"<title>{script}</title>") == "<title>alert('x')</title>"

    try:
        render_html(t"<Button />")
    except TemplateRuntimeError:
        pass
    except TemplateSemanticError:
        pass
    else:
        raise AssertionError("expected component failure")
