from __future__ import annotations

from dataclasses import dataclass
from html import escape

from html_tstring import Fragment, RawHtml, html, render_html


@dataclass(frozen=True)
class SearchResult:
    title: str
    href: str
    snippet: str
    highlighted_term: str


def highlighted_snippet(result: SearchResult) -> Fragment:
    marker = escape(result.highlighted_term)
    safe_snippet = escape(result.snippet).replace(
        marker,
        f"<mark>{marker}</mark>",
    )
    return Fragment([RawHtml(safe_snippet)])


def render_results(query: str, results: list[SearchResult]) -> str:
    cards = Fragment(
        [
            html(t"""\
<article class="search-result">
  <h2><a href="{result.href}">{result.title}</a></h2>
  <p>{highlighted_snippet(result)}</p>
</article>
""")
            for result in results
        ]
    )
    return render_html(t"""\
<main class="search-page">
  <header>
    <h1>Search</h1>
    <p>Results for "{query}"</p>
  </header>
  <section class="search-results">{cards}</section>
</main>
""")


items = [
    SearchResult(
        title="Parser-first HTML rendering",
        href="/docs/parser-first",
        snippet="Parser-first rendering keeps html templates safe and explicit.",
        highlighted_term="html",
    ),
    SearchResult(
        title="Component composition guide",
        href="/docs/components",
        snippet="Compose html and thtml components without RawHtml(render_html(...)).",
        highlighted_term="html",
    ),
]

print(render_results("html", items))
# <main class="search-page">
#   <header>
#     <h1>Search</h1>
#     <p>Results for "html"</p>
#   </header>
#   <section class="search-results"><article class="search-result">
#   <h2><a href="/docs/parser-first">Parser-first HTML rendering</a></h2>
#   <p>Parser-first rendering keeps <mark>html</mark> templates safe and explicit.</p>
# </article>
# <article class="search-result">
#   <h2><a href="/docs/components">Component composition guide</a></h2>
#   <p>Compose <mark>html</mark> and t<mark>html</mark> components without RawHtml(render_<mark>html</mark>(...)).</p>  # noqa: E501
# </article>
# </section>
# </main>
