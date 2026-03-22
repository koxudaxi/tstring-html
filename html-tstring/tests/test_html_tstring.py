from html_tstring import (
    CompiledHtmlTemplate,
    Fragment,
    RawHtml,
    Renderable,
    TemplateSemanticError,
    check_template,
    compile_template,
    format_template,
    html,
    render_fragment,
    render_html,
)


def test_render_html_escapes_children() -> None:
    name = "<world>"
    assert render_html(t"<div>{name}</div>") == "<div>&lt;world&gt;</div>"


def test_format_template_round_trips_source() -> None:
    name = "world"
    assert format_template(t"<div>{name}</div>") == "<div>{name}</div>"


def test_format_template_accepts_kw_only_line_length() -> None:
    assert (
        format_template(
            t'<div data-a="12345" data-b="67890"></div>',
            line_length=20,
        )
        == '<div\n  data-a="12345"\n  data-b="67890"\n></div>'
    )

    try:
        format_template(t"<div></div>", 20)  # type: ignore[misc]
    except TypeError:
        pass
    else:
        raise AssertionError("expected kw-only line_length")


def test_check_compile_and_render_fragment_round_trip() -> None:
    name = "world"
    template = t"<div>{name}</div>"
    check_template(template)

    compiled = compile_template(template)
    assert isinstance(compiled, CompiledHtmlTemplate)
    assert repr(compiled) == "CompiledHtmlTemplate()"
    assert compiled.render(["<world>"]) == "<div>&lt;world&gt;</div>"
    assert compiled.render_fragment(["<world>"]) == "<div>&lt;world&gt;</div>"
    assert render_fragment(template) == "<div>world</div>"


def test_render_html_allows_raw_html_and_type_errors() -> None:
    value = RawHtml("<strong>safe</strong>")
    assert render_html(t"<div>{value}</div>") == "<div><strong>safe</strong></div>"

    classes = ["primary", {"active": True, "disabled": False}, "", None]
    assert render_html(t'<div class="{classes}"></div>') == (
        '<div class="primary active"></div>'
    )

    try:
        render_html("not-a-template")
    except TypeError as exc:
        assert "requires a PEP 750 Template object" in str(exc)
    else:
        raise AssertionError("expected TypeError")


def test_renderable_constructor_and_render_entrypoints() -> None:
    name = "world"
    snippet = html(t"<strong>{name}</strong>")

    assert isinstance(snippet, Renderable)
    assert render_html(snippet) == "<strong>world</strong>"
    assert render_fragment(snippet) == "<strong>world</strong>"

    children = Fragment([snippet, "tail"])
    assert render_html(t"<div>{children}</div>") == (
        "<div><strong>world</strong>tail</div>"
    )


def test_render_html_fragment_and_spread_attrs() -> None:
    attrs = {
        "data-name": "koudai",
        "hidden": False,
        "class": ["primary", {"active": True, "disabled": False}],
    }
    children = Fragment([RawHtml("<em>safe</em>"), "tail"])
    assert render_html(t"<div {attrs}>{children}</div>") == (
        '<div data-name="koudai" class="primary active"><em>safe</em>tail</div>'
    )


def test_render_html_escapes_raw_html_in_attributes_and_spreads() -> None:
    raw_value = RawHtml("<b>x</b>")
    attrs = {"title": raw_value}
    assert render_html(t'<div title="{raw_value}"></div>') == (
        '<div title="&lt;b&gt;x&lt;/b&gt;"></div>'
    )
    assert render_html(t"<div {attrs}></div>") == (
        '<div title="&lt;b&gt;x&lt;/b&gt;"></div>'
    )


def test_renderable_stringifies_and_escapes_in_attributes_and_spreads() -> None:
    name = "world"
    snippet = html(t"<strong>{name}</strong>")
    attrs = {"title": snippet}

    assert render_html(t'<div title="{snippet}"></div>') == (
        '<div title="&lt;strong&gt;world&lt;/strong&gt;"></div>'
    )
    assert render_html(t"<div {attrs}></div>") == (
        '<div title="&lt;strong&gt;world&lt;/strong&gt;"></div>'
    )


def test_render_html_boolean_and_quoted_attribute_values() -> None:
    visible = True
    disabled = False
    title = "safe & sound"

    assert render_html(
        t'<button hidden="{visible}" disabled="{disabled}">go</button>'
    ) == ("<button hidden>go</button>")
    assert render_html(t'<div title="{title}"></div>') == (
        '<div title="safe &amp; sound"></div>'
    )


def test_render_html_rejects_unquoted_dynamic_attrs_and_components() -> None:
    title = "safe"
    visible = True

    try:
        check_template(t"<div title={title}></div>")
    except TemplateSemanticError as exc:
        assert "quoted" in str(exc).lower()
    else:
        raise AssertionError("expected unquoted attr rejection")

    try:
        check_template(t"<Button />")
    except TemplateSemanticError as exc:
        assert "component" in str(exc).lower()
    else:
        raise AssertionError("expected component rejection")

    try:
        check_template(t"<button hidden={visible}>go</button>")
    except TemplateSemanticError as exc:
        assert "quoted" in str(exc).lower()
    else:
        raise AssertionError("expected unquoted boolean attr rejection")


def test_render_html_rejects_spread_type_and_class_bool() -> None:
    attrs = 1

    try:
        render_html(t"<div {attrs}></div>")
    except TemplateSemanticError as exc:
        assert "mapping" in str(exc).lower() or "spread" in str(exc).lower()
    else:
        raise AssertionError("expected non-mapping spread rejection")

    try:
        render_html(t'<button class="{True}"></button>')
    except TemplateSemanticError as exc:
        assert "class" in str(exc).lower()
    else:
        raise AssertionError("expected class bool rejection")


def test_render_html_rejects_raw_text_interpolation() -> None:
    script = "alert('x')"
    textarea = "hello"

    for template in [
        t"<script>{script}</script>",
        t"<style>{script}</style>",
        t"<title>{script}</title>",
        t"<textarea>{textarea}</textarea>",
    ]:
        try:
            check_template(template)
        except TemplateSemanticError as exc:
            message = str(exc).lower()
            assert "interpolation" in message or "raw text" in message
        else:
            raise AssertionError("expected raw-text interpolation rejection")


def test_check_template_rejects_raw_text_interpolation() -> None:
    script = "alert('x')"
    try:
        check_template(t"<script>{script}</script>")
    except TemplateSemanticError as exc:
        assert "Interpolations are not allowed inside <script>" in str(exc)
    else:
        raise AssertionError("expected raw-text interpolation rejection")


def test_raw_html_does_not_bypass_raw_text_rejection() -> None:
    script = RawHtml("alert('x')")

    for template in [
        t"<script>{script}</script>",
        t"<style>{script}</style>",
        t"<title>{script}</title>",
        t"<textarea>{script}</textarea>",
    ]:
        try:
            check_template(template)
        except TemplateSemanticError as exc:
            assert "interpolation" in str(exc).lower() or "raw text" in str(exc).lower()
        else:
            raise AssertionError("expected raw-text interpolation rejection")


def test_render_html_embeds_thtml_renderable_as_safe_child_html() -> None:
    from thtml_tstring import RawHtml as ThtmlRawHtml
    from thtml_tstring import component as thtml_component
    from thtml_tstring import thtml

    @thtml_component
    def Badge(*, children: str) -> ThtmlRawHtml:
        return ThtmlRawHtml(f"<span>{children}</span>")

    label = "active"
    badge = thtml(t"<Badge>{label}</Badge>", globals={"Badge": Badge}, locals={})

    assert render_html(t"<div>{badge}</div>") == "<div><span>active</span></div>"
