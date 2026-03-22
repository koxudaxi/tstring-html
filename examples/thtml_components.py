from __future__ import annotations

from string.templatelib import Template

from thtml_tstring import Fragment, RawHtml, Renderable, component, thtml

type ComponentValue = (
    Template
    | Renderable
    | RawHtml
    | Fragment
    | list[object]
    | tuple[object, ...]
    | str
    | int
    | float
    | bool
    | None
)


@component
def Badge(*, children: object, tone: str = "info") -> ComponentValue:
    classes = ["badge", f"badge-{tone}"]
    return t'<span class="{classes}">{children}</span>'


name: str = "cached"
page = thtml(t"<div><Badge tone='info'>{name}</Badge></div>")

print(page.render())
# <div><span class="badge badge-info">cached</span></div>
