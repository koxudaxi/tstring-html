# html-tstring

Low-level HTML renderer for PEP 750 template strings.

```python
from html_tstring import render_html

name = "world"
page = t"<div>Hello {name}</div>"
assert render_html(page) == "<div>Hello world</div>"
```
