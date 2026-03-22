from thtml_tstring import RawHtml, compile_template, component


@component
def Badge(*, children: str, tone: str = "info") -> RawHtml:
    return RawHtml(f'<span tone="{tone}">{children}</span>')


def test_compiled_thtml_template_can_render_multiple_value_sets() -> None:
    label = "first"
    compiled = compile_template(t"<Badge tone='info'>{label}</Badge>")

    assert compiled.render(["One"], globals={"Badge": Badge}, locals={}) == (
        '<span tone="info">One</span>'
    )
    assert compiled.render(["Two"], globals={"Badge": Badge}, locals={}) == (
        '<span tone="info">Two</span>'
    )


@component
def AltBadge(*, children: str, tone: str = "info") -> RawHtml:
    return RawHtml(f'<strong tone="{tone}">{children}</strong>')


def test_compiled_thtml_template_uses_render_scope_per_call() -> None:
    label = "first"
    compiled = compile_template(t"<Badge tone='info'>{label}</Badge>")

    assert compiled.render(["One"], globals={"Badge": Badge}, locals={}) == (
        '<span tone="info">One</span>'
    )
    assert compiled.render(["Two"], globals={"Badge": AltBadge}, locals={}) == (
        '<strong tone="info">Two</strong>'
    )
