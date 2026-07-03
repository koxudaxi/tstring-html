from __future__ import annotations

import tomllib
from pathlib import Path

import pytest
from html_tstring import (
    Fragment,
    RawHtml,
    TemplateParseError,
    TemplateSemanticError,
    check_template,
    format_template,
    html,
    render_fragment,
    render_html,
)


def load_cases() -> list[dict[str, str]]:
    repo_root = Path(__file__).resolve().parents[2]
    profile_data = tomllib.loads(
        (repo_root / "conformance" / "html" / "profiles.toml").read_text()
    )
    manifest = tomllib.loads(
        (repo_root / "conformance" / "html" / profile_data["manifest_path"]).read_text()
    )
    return manifest["cases"]


HTML_CASES = load_cases()


def _render_case(case_id: str) -> str:
    match case_id:
        case "escaped-child":
            name = "<world>"
            return render_html(t"<div>{name}</div>")
        case "raw-html-child":
            value = RawHtml("<strong>safe</strong>")
            return render_html(t"<div>{value}</div>")
        case "renderable-child":
            name = "safe"
            value = html(t"<strong>{name}</strong>")
            return render_html(t"<div>{value}</div>")
        case "render-html-accepts-renderable":
            name = "safe"
            value = html(t"<strong>{name}</strong>")
            return render_html(value)
        case "raw-html-attribute-escaped":
            value = RawHtml("<b>x</b>")
            return render_html(t'<div title="{value}"></div>')
        case "raw-html-spread-escaped":
            attrs = {"title": RawHtml("<b>x</b>")}
            return render_html(t"<div {attrs}></div>")
        case "renderable-attribute-escaped":
            name = "world"
            snippet = html(t"<strong>{name}</strong>")
            return render_html(t'<div title="{snippet}"></div>')
        case "renderable-spread-escaped":
            name = "world"
            snippet = html(t"<strong>{name}</strong>")
            attrs = {"title": snippet}
            return render_html(t"<div {attrs}></div>")
        case "thtml-renderable-child":
            from thtml_tstring import RawHtml as ThtmlRawHtml
            from thtml_tstring import component as thtml_component
            from thtml_tstring import thtml

            @thtml_component
            def Badge(*, children: str) -> ThtmlRawHtml:
                return ThtmlRawHtml(f"<span>{children}</span>")

            label = "active"
            badge = thtml(
                t"<Badge>{label}</Badge>",
                globals={"Badge": Badge},
                locals={},
            )
            return render_html(t"<div>{badge}</div>")
        case "quoted-dynamic-attribute":
            href = "/dashboard"
            return render_html(t'<a href="{href}">Dashboard</a>')
        case "boolean-attribute-bare-and-omitted":
            visible = True
            hidden = False
            missing = None
            return render_html(
                t'<button disabled="{visible}" hidden="{hidden}" '
                t'data-x="{missing}">OK</button>'
            )
        case "class-normalization":
            classes = ["btn", {"btn-primary": True}, "extra", {"active": True}]
            return render_html(t'<button class="{classes}"></button>')
        case "spread-merge-order":
            attrs = {"data-id": "2"}
            spread_classes = ["extra", {"active": True}]
            tail = "tail"
            return render_html(
                t'<div class="base {spread_classes} {tail}" {attrs}></div>'
            )
        case "fragment-children":
            children = Fragment([RawHtml("<em>first</em>"), "second"])
            return render_html(t"<div>{children}</div>")
        case "render-fragment":
            name = "world"
            return render_fragment(t"<p>{name}</p>")
        case "comment-and-doctype":
            return render_html(t"<!DOCTYPE html><!--x--><div>ok</div>")
        case "raw-text-title-escaped":
            title = RawHtml("<safe>")
            return render_html(t"<title>{title}</title>")
        case "format-roundtrip":
            name = "world"
            return format_template(t"<div>{name}</div>")
        case _:
            raise AssertionError(f"Unhandled HTML render case: {case_id}")


def _assert_error(case_id: str, expected_error: str) -> None:
    error_type = {
        "TemplateParseError": TemplateParseError,
        "TemplateSemanticError": TemplateSemanticError,
    }[expected_error]

    with pytest.raises(error_type) as exc_info:
        match case_id:
            case "component-rejected":
                check_template(t"<Button />")
            case "raw-text-script-rejected":
                script = "alert('x')"
                check_template(t"<script>{script}</script>")
            case "raw-text-style-rejected":
                style = "body { color: red; }"
                check_template(t"<style>{style}</style>")
            case "raw-text-textarea-rejected":
                value = "user input"
                check_template(t"<textarea>{value}</textarea>")
            case "parse-mismatched-tag":
                check_template(t"<div></span>")
            case "parse-unclosed-tag":
                check_template(t"<div")
            case "unquoted-dynamic-attr-rejected":
                title = "safe & sound"
                check_template(t"<div title={title}></div>")
            case "spread-non-mapping-error":
                attrs = 1
                render_html(t"<div {attrs}></div>")
            case "spread-invalid-attribute-name-error":
                attrs = {"x onmouseover=alert(1)": "y"}
                render_html(t"<div {attrs}></div>")
            case "class-bool-rejected":
                value = True
                render_html(t'<button class="{value}"></button>')
            case _:
                raise AssertionError(f"Unhandled HTML error case: {case_id}")

    if case_id == "spread-invalid-attribute-name-error":
        assert "attribute name" in (message := str(exc_info.value).lower()), message


@pytest.mark.parametrize("case", HTML_CASES, ids=lambda case: case["case_id"])
def test_html_conformance_cases(case: dict[str, str]) -> None:
    if case["execution_layer"] == "rust":
        pytest.skip("Rust-only conformance case")

    if "expected_error" in case:
        _assert_error(case["case_id"], case["expected_error"])
        return

    assert _render_case(case["case_id"]) == case["expected"]
