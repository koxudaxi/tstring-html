from __future__ import annotations

from dataclasses import dataclass
from string.templatelib import Template
from typing import Literal

from thtml_tstring import Fragment, RawHtml, Renderable, component, thtml

type Tone = Literal["info", "success", "warning"]
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


@dataclass(frozen=True)
class Metric:
    label: str
    value: int
    tone: Tone = "info"


@component
def Badge(*, children: object, tone: Tone = "info") -> ComponentValue:
    classes = ["badge", f"badge-{tone}"]
    return t'<span class="{classes}">{children}</span>'


@component
def MetricCard(*, children: object, label: str, tone: Tone = "info") -> ComponentValue:
    return t"""\
<section class="metric-card">
  <div class="metric-card__label">{label}</div>
  <div class="metric-card__value"><Badge tone="{tone}">{children}</Badge></div>
</section>
"""


@component
def DashboardShell(*, children: object, title: str) -> ComponentValue:
    return thtml(t"""\
<main class="dashboard-shell">
  <header class="dashboard-shell__header">
    <h1>{title}</h1>
  </header>
  <section class="dashboard-shell__body">{children}</section>
</main>
""")


def dashboard_cards(metrics: list[Metric]) -> Fragment:
    return Fragment(
        [
            thtml(
                t'<MetricCard label="{metric.label}" tone="{metric.tone}">'
                t"{metric.value}</MetricCard>"
            )
            for metric in metrics
        ]
    )


metrics = [
    Metric(label="Deploys", value=12, tone="success"),
    Metric(label="Queued jobs", value=4, tone="warning"),
    Metric(label="Open incidents", value=1, tone="info"),
]

page = thtml(
    t'<DashboardShell title="Ops dashboard">{dashboard_cards(metrics)}</DashboardShell>'
)

print(page.render())
# <main class="dashboard-shell">
#   <header class="dashboard-shell__header">
#     <h1>Ops dashboard</h1>
#   </header>
#   <section class="dashboard-shell__body"><section class="metric-card">
#   <div class="metric-card__label">Deploys</div>
#   <div class="metric-card__value"><span class="badge badge-success">12</span></div>
# </section>
# <section class="metric-card">
#   <div class="metric-card__label">Queued jobs</div>
#   <div class="metric-card__value"><span class="badge badge-warning">4</span></div>
# </section>
# <section class="metric-card">
#   <div class="metric-card__label">Open incidents</div>
#   <div class="metric-card__value"><span class="badge badge-info">1</span></div>
# </section>
# </section>
# </main>
