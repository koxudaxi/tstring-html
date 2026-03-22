from html_tstring import compile_template


def test_compiled_html_template_can_render_multiple_value_sets() -> None:
    first = "first"
    compiled = compile_template(t"<div>{first}</div>")

    assert compiled.render(["<one>"]) == "<div>&lt;one&gt;</div>"
    assert compiled.render(["<two>"]) == "<div>&lt;two&gt;</div>"
    assert compiled.render_fragment(["<three>"]) == "<div>&lt;three&gt;</div>"
