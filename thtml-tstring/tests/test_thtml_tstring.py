import pytest
from html_tstring import check_template as html_check_template
from thtml_tstring import (
    CompiledThtmlTemplate,
    Fragment,
    RawHtml,
    Renderable,
    TemplateParseError,
    TemplateRuntimeError,
    TemplateSemanticError,
    check_template,
    compile_template,
    component,
    format_template,
    html,
    render_html,
    spread,
    thtml,
)


def _flatten_class_tokens(value: object) -> list[str]:
    if value is None or value is False:
        return []
    if isinstance(value, str):
        return [part for part in value.split() if part]
    if isinstance(value, dict):
        return [str(name) for name, truthy in value.items() if truthy]
    if isinstance(value, (list, tuple)):
        tokens: list[str] = []
        for item in value:
            tokens.extend(_flatten_class_tokens(item))
        return tokens
    return [str(value)]


@component
def Button(*, children: str, **props: object) -> RawHtml:
    attrs: list[str] = []
    for key, value in props.items():
        if key == "kind":
            continue
        if key == "class":
            class_tokens = _flatten_class_tokens(value)
            if class_tokens:
                attrs.append(f'class="{" ".join(class_tokens)}"')
            continue
        if value is True:
            attrs.append(key)
            continue
        if value is None or value is False:
            continue
        attrs.append(f'{key}="{value}"')
    kind = str(props.get("kind", "primary"))
    attr_text = f" {' '.join(attrs)}" if attrs else ""
    return RawHtml(f'<button kind="{kind}"{attr_text}>{children}</button>')


@component
def Stack(*, children: str) -> list[object]:
    return [RawHtml("<section>"), children, RawHtml("</section>")]


@component
def ReturnScalar(*, children: object) -> int:
    return 42


@component
def ReturnFloat(*, children: object) -> float:
    return 2.5


@component
def ReturnBool(*, children: object) -> bool:
    return True


@component
def ReturnNone(*, children: object) -> None:
    return None


@component
def ShowRaw(*, children: object) -> RawHtml:
    assert isinstance(children, RawHtml)
    return children


@component
def ShowList(*, children: object) -> RawHtml:
    assert isinstance(children, list)
    return RawHtml(",".join(str(item) for item in children))


@component
def ShowFragment(*, children: object) -> RawHtml:
    assert isinstance(children, Fragment)
    return RawHtml("<ok />")


@component
def ShowProps(*, children: object, **props: object) -> RawHtml:
    return RawHtml(f"{props['class']}|{props['aria-label']}|{children}")


@component
def AutoBadge(*, children: object, tone: str = "info"):
    classes = ["badge", f"badge-{tone}"]
    return t'<span class="{classes}">{children}</span>'


@component
def AutoCard(*, children: object, title: str):
    return t'<div class="card"><h2>{title}</h2><div class="body">{children}</div></div>'


@component(registry={"AutoBadge": AutoBadge})
def RegistryCard(*, children: object):
    return t'<div class="card"><AutoBadge>{children}</AutoBadge></div>'


@component
def ExplicitBadge(*, children: object, tone: str = "info"):
    classes = ["badge", f"badge-{tone}"]
    return thtml(t'<span class="{classes}">{children}</span>')


@component(backend="html")
def HtmlSnippet(*, children: object):
    return t"<em>{children}</em>"


@component
def ReturnText(*, children: object) -> str:
    return "<b>unsafe</b>"


def render_with_caller_scope() -> str:
    value = "x"

    @component
    def Inline(*, children: str, tone: str = "info") -> RawHtml:
        return RawHtml(f'<span tone="{tone}">{children}</span>')

    return html(t"<Inline>{value}</Inline>")


def render_nested_components() -> str:
    label = "new"

    @component
    def Badge(*, children: str) -> RawHtml:
        return RawHtml(f'<span class="badge">{children}</span>')

    @component
    def Card(*, children: str, title: str) -> RawHtml:
        return RawHtml(f'<div class="card"><h2>{title}</h2>{children}</div>')

    return html(t"<Card title='Status'><Badge>{label}</Badge></Card>")


def build_captured_renderable() -> Renderable:
    value = "captured"

    @component
    def Inline(*, children: object) -> RawHtml:
        return RawHtml(f"<span>{children}</span>")

    return thtml(t"<Inline>{value}</Inline>")


def test_render_thtml_component_with_frame_lookup() -> None:
    label = "Save"
    assert render_html(t"<Button kind='primary'>{label}</Button>") == (
        '<button kind="primary">Save</button>'
    )
    assert html(t"<Button kind='primary'>{label}</Button>") == (
        '<button kind="primary">Save</button>'
    )
    assert (
        html(
            t"<Button kind='primary'>{label}</Button>",
            globals={"Button": Button},
            locals={},
        )
        == '<button kind="primary">Save</button>'
    )
    assert render_with_caller_scope() == '<span tone="info">x</span>'


def test_check_format_compile_and_render_with_explicit_scope() -> None:
    label = "Save"
    attrs = {"kind": "secondary"}
    template = t"<Button {attrs}>{label}</Button>"

    check_template(template)
    assert format_template(template) == "<Button {attrs}>{label}</Button>"

    compiled = compile_template(template)
    assert isinstance(compiled, CompiledThtmlTemplate)
    assert repr(compiled) == "CompiledThtmlTemplate()"
    assert (
        compiled.render(
            [attrs, label],
            globals={"Button": Button},
            locals={},
        )
        == '<button kind="secondary">Save</button>'
    )


def test_format_template_accepts_kw_only_line_length() -> None:
    assert (
        format_template(
            t'<Panel data-a="12345" data-b="67890"></Panel>',
            line_length=20,
        )
        == '<Panel\n  data-a="12345"\n  data-b="67890"\n></Panel>'
    )

    with pytest.raises(TypeError):
        format_template(t"<Panel></Panel>", 20)  # type: ignore[misc]


def test_render_html_with_explicit_scope_and_helpers() -> None:
    label = "Save"
    attrs = spread({"kind": "secondary"})
    assert attrs == {"kind": "secondary"}
    redecorated = component(Button)
    assert callable(redecorated)
    assert getattr(redecorated, "__name__", None) == getattr(Button, "__name__", None)
    class_attrs = spread(
        {"kind": "primary", "class": ["button", {"button--primary": True}]}
    )
    assert (
        render_html(
            t"<Button {attrs}>{label}</Button>",
            globals={"Button": Button},
            locals={},
        )
        == '<button kind="secondary">Save</button>'
    )
    assert (
        render_html(
            t"<Button {attrs}>{label}</Button>",
            registry={"Button": Button},
        )
        == '<button kind="secondary">Save</button>'
    )
    assert (
        render_html(
            t"<Button {class_attrs}>{label}</Button>",
            globals={"Button": Button},
            locals={},
        )
        == '<button kind="primary" class="button button--primary">Save</button>'
    )
    assert (
        render_html(
            t"<Button kind='primary' {attrs}>{label}</Button>",
            globals={"Button": Button},
            locals={},
        )
        == '<button kind="secondary">Save</button>'
    )


def test_render_thtml_component_lookup_failure_and_type_error() -> None:
    label = "Save"
    try:
        html(t"<Missing>{label}</Missing>", globals={}, locals={})
    except TemplateRuntimeError as exc:
        assert "Unknown component" in str(exc)
    else:
        raise AssertionError("expected unknown component error")

    try:
        html(t"<Broken>{label}</Broken>", globals={"Broken": 123}, locals={})
    except TemplateRuntimeError as exc:
        assert "Unknown component" in str(exc) or "callable" in str(exc)
    else:
        raise AssertionError("expected non-callable component error")

    try:
        render_html("not-a-template", globals={}, locals={})
    except TypeError as exc:
        assert "requires a PEP 750 Template object" in str(exc)
    else:
        raise AssertionError("expected TypeError")

    for api in (html, render_html, thtml):
        with pytest.raises(TypeError, match="registry="):
            api(
                t"<Button />",
                globals={"Button": Button},
                registry={"Button": Button},
            )  # type: ignore[call-arg]

    compiled = compile_template(t"<Button />")
    with pytest.raises(TypeError, match="registry="):
        compiled.render([], globals={"Button": Button}, registry={"Button": Button})


def test_component_return_normalization_accepts_sequences() -> None:
    label = "Save"
    assert html(t"<Stack>{label}</Stack>") == "<section>Save</section>"
    assert html(t"<ReturnScalar>{label}</ReturnScalar>") == "42"
    assert html(t"<ReturnFloat>{label}</ReturnFloat>") == "2.5"
    assert html(t"<ReturnBool>{label}</ReturnBool>") == "true"
    assert html(t"<ReturnNone>{label}</ReturnNone>") == ""


def test_children_normalization_preserves_runtime_structure() -> None:
    raw = RawHtml("<b>x</b>")
    items = ["a", ["b", "c"], None]
    scalar_items = ["a", 1, 2.5, True, None, ["b", "c"]]
    fragment = Fragment([RawHtml("<i>x</i>"), "tail"])

    assert html(t"<ShowRaw>{raw}</ShowRaw>") == "<b>x</b>"
    assert html(t"<ShowList>{items}</ShowList>") == "a,b,c"
    assert html(t"<ShowList>{scalar_items}</ShowList>") == "a,1,2.5,True,b,c"
    assert html(t"<ShowFragment>{fragment}</ShowFragment>") == "<ok />"
    assert html(t"<ShowRaw><span>child</span></ShowRaw>") == "<span>child</span>"


def test_component_spread_rejects_non_mapping_values() -> None:
    props = 123
    try:
        html(t"<Button {props} />", globals={"Button": Button}, locals={})
    except TemplateRuntimeError as exc:
        assert "Spread attributes require a mapping-like value" in str(exc)
    else:
        raise AssertionError("expected spread runtime error")


def test_component_props_keep_html_facing_names() -> None:
    label = "Save"
    assert (
        html(
            t"<ShowProps class='primary' aria-label='Save button'>{label}</ShowProps>",
            globals={"ShowProps": ShowProps},
            locals={},
        )
        == "primary|Save button|Save"
    )


def test_nested_components_render_inside_each_other() -> None:
    assert render_nested_components() == (
        '<div class="card"><h2>Status</h2><span class="badge">new</span></div>'
    )


def test_component_template_return_auto_wraps_without_raw_html() -> None:
    label = "active"
    assert (
        html(
            t"<AutoCard title='Status'>"
            t"<AutoBadge tone='success'>{label}</AutoBadge>"
            t"</AutoCard>",
            globals={"AutoCard": AutoCard, "AutoBadge": AutoBadge},
            locals={},
        )
        == '<div class="card"><h2>Status</h2><div class="body">'
        '<span class="badge badge-success">active</span></div></div>'
    )


def test_renderable_child_composition_and_nested_containers() -> None:
    label = "active"
    badge = thtml(
        t"<AutoBadge tone='success'>{label}</AutoBadge>",
        globals={"AutoBadge": AutoBadge},
        locals={},
    )
    assert isinstance(badge, Renderable)
    assert render_html(badge) == '<span class="badge badge-success">active</span>'

    items = [badge, Fragment([badge, "tail"])]
    assert html(
        t"<ShowList>{items}</ShowList>",
        globals={"ShowList": ShowList},
        locals={},
    ) == (
        '<span class="badge badge-success">active</span>,'
        '<span class="badge badge-success">active</span>,tail'
    )


def test_component_decorator_and_explicit_wrap_produce_same_html() -> None:
    label = "active"
    auto = html(
        t"<AutoBadge tone='success'>{label}</AutoBadge>",
        globals={"AutoBadge": AutoBadge},
        locals={},
    )
    explicit = html(
        t"<ExplicitBadge tone='success'>{label}</ExplicitBadge>",
        globals={"ExplicitBadge": ExplicitBadge},
        locals={},
    )
    assert auto == explicit == '<span class="badge badge-success">active</span>'


def test_registry_supports_large_project_resolution_without_frame_lookup() -> None:
    label = "active"
    assert (
        html(
            t"<RegistryBadge tone='success'>{label}</RegistryBadge>",
            registry={"RegistryBadge": AutoBadge},
        )
        == '<span class="badge badge-success">active</span>'
    )
    renderable = thtml(
        t"<RegistryBadge tone='success'>{label}</RegistryBadge>",
        registry={"RegistryBadge": AutoBadge},
    )
    assert render_html(renderable) == '<span class="badge badge-success">active</span>'


def test_registry_isolated_same_name_components() -> None:
    label = "active"

    @component
    def Badge(*, children: object):  # noqa: N802
        return t"<strong>{children}</strong>"

    assert html(t"<Badge>{label}</Badge>", registry={"Badge": AutoBadge}) == (
        '<span class="badge badge-info">active</span>'
    )
    assert html(t"<Badge>{label}</Badge>", registry={"Badge": Badge}) == (
        "<strong>active</strong>"
    )


def test_registry_disables_caller_frame_lookup() -> None:
    label = "active"
    with pytest.raises(TemplateRuntimeError):
        html(t"<AutoBadge>{label}</AutoBadge>", registry={})


def test_component_registry_freezes_nested_component_resolution() -> None:
    def AutoBadge(*, children: object) -> RawHtml:  # noqa: N802
        return RawHtml(f"<div>{children}</div>")

    assert (
        html(
            t"<RegistryCard>ok</RegistryCard>",
            registry={"RegistryCard": RegistryCard, "AutoBadge": AutoBadge},
        )
        == '<div class="card"><span class="badge badge-info">ok</span></div>'
    )


def test_component_backend_html_auto_wrap_renders_html_children() -> None:
    label = "active"
    assert (
        html(
            t"<HtmlSnippet>{label}</HtmlSnippet>",
            globals={"HtmlSnippet": HtmlSnippet},
            locals={},
        )
        == "<em>active</em>"
    )


def test_string_returning_component_stays_text_and_raw_html_still_works() -> None:
    assert html(t"<ReturnText />", globals={"ReturnText": ReturnText}, locals={}) == (
        "&lt;b&gt;unsafe&lt;/b&gt;"
    )
    raw = RawHtml("<strong>safe</strong>")
    assert html(t"<ShowRaw>{raw}</ShowRaw>") == "<strong>safe</strong>"


def test_thtml_constructor_captures_scope_for_later_render() -> None:
    renderable = build_captured_renderable()
    assert render_html(renderable) == "<span>captured</span>"
    assert html(renderable) == "<span>captured</span>"


def test_thtml_constructor_freezes_scope_when_components_are_rebound() -> None:
    captured = build_captured_renderable()

    @component
    def Inline(*, children: object) -> RawHtml:
        return RawHtml(f"<div>{children}</div>")

    assert render_html(captured) == "<span>captured</span>"
    assert (
        html(captured, globals={"Inline": Inline}, locals={}) == "<span>captured</span>"
    )


def test_component_auto_wrap_uses_defining_module_globals_for_nested_tags() -> None:
    @component
    def Outer(*, children: object):
        return t"<AutoBadge>{children}</AutoBadge>"

    def render_with_shadow() -> str:
        def AutoBadge(*, children: object) -> RawHtml:  # noqa: N802
            return RawHtml(f"<div>{children}</div>")

        return html(
            t"<Outer>ok</Outer>",
            globals={"Outer": Outer, "AutoBadge": AutoBadge},
            locals={"AutoBadge": AutoBadge},
        )

    assert render_with_shadow() == '<span class="badge badge-info">ok</span>'


def test_renderable_cross_backend_composition_works_in_thtml() -> None:
    from html_tstring import html as html_renderable

    label = "active"
    snippet = html_renderable(t"<strong>{label}</strong>")
    assert html(t"<ShowRaw>{snippet}</ShowRaw>") == "<strong>active</strong>"


def test_thtml_constructor_raises_when_frame_capture_is_unavailable(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def broken_getframe(depth: int) -> object:
        raise ValueError("no frame")

    monkeypatch.setattr("thtml_tstring._runtime.sys._getframe", broken_getframe)
    with pytest.raises(TemplateRuntimeError):
        thtml(t"<AutoBadge>broken</AutoBadge>")


def test_component_rejects_unknown_backend() -> None:
    with pytest.raises(ValueError):
        component(backend="xml")

    with pytest.raises(ValueError):
        component(backend="html", registry={"Button": Button})


def test_parse_semantic_and_format_errors_cover_documented_v1_behavior() -> None:
    label = "Save"

    with pytest.raises(TemplateParseError):
        check_template(t"<div></span>")

    for template in [
        t"<script>{label}</script>",
        t"<style>{label}</style>",
        t"<textarea>{label}</textarea>",
    ]:
        with pytest.raises(TemplateSemanticError):
            check_template(template)

    assert render_html(t"<title>{label}</title>") == "<title>Save</title>"

    with pytest.raises(TemplateSemanticError):
        html_check_template(t"<Button>{label}</Button>")

    assert format_template(t"<Button>{label}</Button>") == "<Button>{label}</Button>"
