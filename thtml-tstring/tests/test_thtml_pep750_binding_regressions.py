from thtml_tstring import RawHtml, component, html


@component
def Inline(*, children: str) -> RawHtml:
    return RawHtml(f"<span>{children}</span>")


def render_with_local_component() -> str:
    value = "x"
    return html(t"<Inline>{value}</Inline>")


def render_with_explicit_component() -> str:
    value = "y"
    return html(t"<Inline>{value}</Inline>", globals={"Inline": Inline}, locals={})


def test_immediate_caller_frame_lookup_supports_local_components() -> None:
    assert render_with_local_component() == "<span>x</span>"


def test_explicit_scope_lookup_supports_local_components() -> None:
    assert render_with_explicit_component() == "<span>y</span>"


def test_partial_scope_capture_supports_globals_only() -> None:
    value = "g"
    assert (
        html(t"<Inline>{value}</Inline>", globals={"Inline": Inline})
        == "<span>g</span>"
    )


def test_partial_scope_capture_supports_locals_only() -> None:
    value = "l"
    assert (
        html(t"<Inline>{value}</Inline>", locals={"Inline": Inline}) == "<span>l</span>"
    )
