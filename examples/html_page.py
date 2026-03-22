from __future__ import annotations

from html_tstring import render_html

title: str = "tstring-html"
body: str = "Hello from HTML"

page = t"""\
<main class="page">
  <h1>{title}</h1>
  <p>{body}</p>
</main>
"""

print(render_html(page))
# <main class="page">
#   <h1>tstring-html</h1>
#   <p>Hello from HTML</p>
# </main>
