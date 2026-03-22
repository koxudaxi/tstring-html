from __future__ import annotations

import tomllib
from pathlib import Path

import pytest
from html_tstring import TemplateSemanticError as HtmlTemplateSemanticError
from html_tstring import check_template as check_html_template
from thtml_tstring import (
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
    thtml,
)


def load_cases() -> list[dict[str, str]]:
    repo_root = Path(__file__).resolve().parents[2]
    profile_data = tomllib.loads(
        (repo_root / "conformance" / "thtml" / "profiles.toml").read_text()
    )
    manifest = tomllib.loads(
        (
            repo_root / "conformance" / "thtml" / profile_data["manifest_path"]
        ).read_text()
    )
    return manifest["cases"]


THTML_CASES = load_cases()


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
def Button(*, children: object, **props: object) -> RawHtml:
    attrs = [f'kind="{props.get("kind", "primary")}"']
    for key, value in props.items():
        if key == "kind":
            continue
        if key == "class":
            tokens = _flatten_class_tokens(value)
            if tokens:
                attrs.append(f'class="{" ".join(tokens)}"')
            continue
        if value is True:
            attrs.append(key)
            continue
        if value is None or value is False:
            continue
        attrs.append(f'{key}="{value}"')
    return RawHtml(f"<button {' '.join(attrs)}>{children}</button>")


@component
def LocalButton(*, children: object, kind: str = "local") -> RawHtml:
    return RawHtml(f'<button kind="{kind}">{children}</button>')


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
def ShowLiteralMarkup(*, children: object) -> RawHtml:
    assert isinstance(children, RawHtml)
    return children


@component
def ShowProps(*, children: object, **props: object) -> RawHtml:
    return RawHtml(f"{props['class']}|{props['aria-label']}|{children}")


@component
def Stack(*, children: object) -> list[object]:
    return [RawHtml("<section>"), children, RawHtml("</section>")]


@component
def Empty(*, children: object) -> None:
    return None


@component
def Count(*, children: object) -> int:
    return 42


@component
def Ratio(*, children: object) -> float:
    return 2.5


@component
def Flag(*, children: object) -> bool:
    return True


@component
def Card(*, children: object, title: str) -> RawHtml:
    return RawHtml(f'<div class="card"><h2>{title}</h2>{children}</div>')


@component
def Badge(*, children: object) -> RawHtml:
    return RawHtml(f'<span class="badge">{children}</span>')


@component
def AutoBadge(*, children: object, tone: str = "info"):
    classes = ["badge", f"badge-{tone}"]
    return t'<span class="{classes}">{children}</span>'


@component
def AutoCard(*, children: object, title: str):
    return t'<div class="card"><h2>{title}</h2><div class="body">{children}</div></div>'


@component
def ExplicitBadge(*, children: object, tone: str = "info"):
    classes = ["badge", f"badge-{tone}"]
    return thtml(t'<span class="{classes}">{children}</span>')


@component
def ReturnText(*, children: object) -> str:
    return "<b>unsafe</b>"


def _caller_frame_render() -> str:
    value = "x"

    @component
    def Inline(*, children: object) -> RawHtml:
        return RawHtml(f"<span>{children}</span>")

    return html(t"<Inline>{value}</Inline>")


def _captured_renderable() -> Renderable:
    value = "captured"

    @component
    def Inline(*, children: object) -> RawHtml:
        return RawHtml(f"<span>{children}</span>")

    return thtml(t"<Inline>{value}</Inline>")


def _partial_scope_globals_only() -> str:
    value = "g"
    return html(t"<Inline>{value}</Inline>", globals={"Inline": _inline_component})


def _partial_scope_locals_only() -> str:
    value = "l"
    return html(t"<Inline>{value}</Inline>", locals={"Inline": _inline_component})


@component
def _inline_component(*, children: object) -> RawHtml:
    return RawHtml(f"<span>{children}</span>")


def _render_case(case_id: str) -> str:
    match case_id:
        case "component-basic-caller-scope":
            label = "Save"
            return html(t"<Button>{label}</Button>")
        case "component-explicit-scope":
            label = "Save"
            return render_html(
                t"<Button kind='secondary'>{label}</Button>",
                globals={"Button": Button},
                locals={},
            )
        case "locals-over-globals":
            label = "Save"
            return render_html(
                t"<Button>{label}</Button>",
                globals={"Button": Button},
                locals={"Button": LocalButton},
            )
        case "component-spread":
            label = "Save"
            props = {"kind": "secondary"}
            return html(
                t"<Button {props}>{label}</Button>",
                globals={"Button": Button},
                locals={},
            )
        case "component-class-spread":
            label = "Save"
            props = {"kind": "primary", "class": ["button", {"button--primary": True}]}
            return html(
                t"<Button {props}>{label}</Button>",
                globals={"Button": Button},
                locals={},
            )
        case "component-spread-override":
            label = "Save"
            props = {"kind": "secondary"}
            return html(
                t"<Button kind='primary' {props}>{label}</Button>",
                globals={"Button": Button},
                locals={},
            )
        case "html-element-attr-merge":
            attrs = {"data-id": "2"}
            spread_classes = ["extra", {"active": True}]
            tail = "tail"
            return html(
                t'<div class="base {spread_classes} {tail}" {attrs}>content</div>'
            )
        case "component-prop-html-names":
            label = "Save"
            return html(
                t"<ShowProps class='primary' aria-label='Save button'>"
                t"{label}</ShowProps>",
                globals={"ShowProps": ShowProps},
                locals={},
            )
        case "children-raw-html":
            raw = RawHtml("<b>x</b>")
            return html(
                t"<ShowRaw>{raw}</ShowRaw>", globals={"ShowRaw": ShowRaw}, locals={}
            )
        case "children-list":
            items = ["a", "b"]
            return html(
                t"<ShowList>{items}</ShowList>",
                globals={"ShowList": ShowList},
                locals={},
            )
        case "children-list-scalars":
            items = ["a", 1, 2.5, True, None, ["b", "c"]]
            return html(
                t"<ShowList>{items}</ShowList>",
                globals={"ShowList": ShowList},
                locals={},
            )
        case "children-fragment":
            fragment = Fragment([RawHtml("<i>x</i>"), "tail"])
            return html(
                t"<ShowFragment>{fragment}</ShowFragment>",
                globals={"ShowFragment": ShowFragment},
                locals={},
            )
        case "literal-child-markup-becomes-raw-html":
            return html(
                t"<ShowLiteralMarkup><span>child</span></ShowLiteralMarkup>",
                globals={"ShowLiteralMarkup": ShowLiteralMarkup},
                locals={},
            )
        case "component-return-list":
            label = "Save"
            return html(t"<Stack>{label}</Stack>", globals={"Stack": Stack}, locals={})
        case "component-return-none":
            label = "ignored"
            return html(t"<Empty>{label}</Empty>", globals={"Empty": Empty}, locals={})
        case "component-return-scalar":
            label = "ignored"
            return html(t"<Count>{label}</Count>", globals={"Count": Count}, locals={})
        case "component-return-float":
            label = "ignored"
            return html(t"<Ratio>{label}</Ratio>", globals={"Ratio": Ratio}, locals={})
        case "component-return-bool":
            label = "ignored"
            return html(t"<Flag>{label}</Flag>", globals={"Flag": Flag}, locals={})
        case "nested-components":
            label = "new"
            return html(
                t"<Card title='Status'><Badge>{label}</Badge></Card>",
                globals={"Card": Card, "Badge": Badge},
                locals={},
            )
        case "component-auto-wrap-template":
            label = "active"
            return html(
                t"<AutoCard title='Status'>"
                t"<AutoBadge tone='success'>{label}</AutoBadge>"
                t"</AutoCard>",
                globals={"AutoCard": AutoCard, "AutoBadge": AutoBadge},
                locals={},
            )
        case "component-explicit-renderable-wrap":
            label = "active"
            return html(
                t"<ExplicitBadge tone='success'>{label}</ExplicitBadge>",
                globals={"ExplicitBadge": ExplicitBadge},
                locals={},
            )
        case "renderable-captures-scope":
            return render_html(_captured_renderable())
        case "renderable-scope-frozen":
            captured = _captured_renderable()

            @component
            def Inline(*, children: object) -> RawHtml:
                return RawHtml(f"<div>{children}</div>")

            return render_html(captured)
        case "renderable-ignores-later-overrides":
            captured = _captured_renderable()

            @component
            def Inline(*, children: object) -> RawHtml:
                return RawHtml(f"<div>{children}</div>")

            return html(captured, globals={"Inline": Inline}, locals={})
        case "component-return-string-is-text":
            return html(
                t"<ReturnText />",
                globals={"ReturnText": ReturnText},
                locals={},
            )
        case "auto-wrap-uses-defining-globals":

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

            return render_with_shadow()
        case "html-renderable-inside-thtml":
            from html_tstring import html as html_renderable

            label = "active"
            snippet = html_renderable(t"<strong>{label}</strong>")
            return html(
                t"<ShowRaw>{snippet}</ShowRaw>",
                globals={"ShowRaw": ShowRaw},
                locals={},
            )
        case "compiled-render-scope-per-call":
            label = "first"
            compiled = compile_template(t"<Badge tone='info'>{label}</Badge>")

            @component
            def AltBadge(*, children: object, tone: str = "info") -> RawHtml:
                return RawHtml(f'<strong tone="{tone}">{children}</strong>')

            return compiled.render(["Two"], globals={"Badge": AltBadge}, locals={})
        case "partial-scope-globals-only":
            return _partial_scope_globals_only()
        case "partial-scope-locals-only":
            return _partial_scope_locals_only()
        case "caller-frame-local-component":
            return _caller_frame_render()
        case "format-roundtrip":
            label = "Save"
            return format_template(t"<Button>{label}</Button>")
        case _:
            raise AssertionError(f"Unhandled T-HTML render case: {case_id}")


def _assert_error(case_id: str, expected_error: str) -> None:
    error_type = {
        "TemplateParseError": TemplateParseError,
        "TemplateRuntimeError": TemplateRuntimeError,
        "TemplateSemanticError": TemplateSemanticError,
    }[expected_error]

    with pytest.raises(error_type):
        match case_id:
            case "missing-component":
                label = "Save"
                html(t"<Missing>{label}</Missing>", globals={}, locals={})
            case "non-callable-component":
                label = "Save"
                html(t"<Button>{label}</Button>", globals={"Button": 1}, locals={})
            case "component-spread-non-mapping":
                props = 123
                html(t"<Button {props} />", globals={"Button": Button}, locals={})
            case "raw-text-script-rejected":
                script = "alert('x')"
                check_template(t"<script>{script}</script>")
            case "raw-text-style-rejected":
                style = "body { color: red; }"
                check_template(t"<style>{style}</style>")
            case "raw-text-title-rejected":
                title = "Hello"
                check_template(t"<title>{title}</title>")
            case "raw-text-textarea-rejected":
                value = "user input"
                check_template(t"<textarea>{value}</textarea>")
            case "html-vs-thtml-split":
                check_html_template(t"<Button />")
            case "unquoted-dynamic-attr-rejected":
                kind = "primary"
                check_template(t"<Button kind={kind} />")
            case "parse-mismatched-tag":
                check_template(t"<div")
            case "parse-unclosed-tag":
                check_template(t"<Button")
            case _:
                raise AssertionError(f"Unhandled T-HTML error case: {case_id}")


@pytest.mark.parametrize("case", THTML_CASES, ids=lambda case: case["case_id"])
def test_thtml_conformance_cases(case: dict[str, str]) -> None:
    if case["execution_layer"] == "rust":
        pytest.skip("Rust-only conformance case")

    if "expected_error" in case:
        _assert_error(case["case_id"], case["expected_error"])
        return

    assert _render_case(case["case_id"]) == case["expected"]


def test_html_backend_and_thtml_backend_stay_split() -> None:
    with pytest.raises(HtmlTemplateSemanticError):
        check_html_template(t"<Button />")
