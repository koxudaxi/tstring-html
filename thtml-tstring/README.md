# thtml-tstring

Higher-level T-HTML renderer for PEP 750 template strings.

```python
from thtml_tstring import component, thtml

@component
def Button(*, children: str, kind: str = "primary"):
    classes = [kind]
    return t'<button class="{classes}">{children}</button>'

label = "Save"
page = thtml(t"<Button kind='primary'>{label}</Button>")
assert page.render() == '<button class="primary">Save</button>'
```
