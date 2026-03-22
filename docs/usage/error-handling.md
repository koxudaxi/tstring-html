# Error Handling

f-strings silently produce broken or dangerous HTML. t-string backends catch these problems at parse time or render time.

## Error hierarchy

```
TemplateError
├── TemplateParseError      : template syntax is invalid
├── TemplateSemanticError   : valid syntax but invalid usage
└── TemplateRuntimeError    : runtime failure (T-HTML only)
```

The Rust backend produces diagnostics with source spans. The Python exceptions expose the primary error message. Editor integrations (t-linter) use the Rust diagnostics directly.

## Parse errors

Raised when the template HTML is syntactically broken:

```python
from html_tstring import check_template, TemplateParseError

try:
    check_template(t"<div></span>")
except TemplateParseError as e:
    print(e)  # Mismatched closing tag </span>. Expected </div>.
```

## Semantic errors

Raised when syntax is valid but the usage is wrong:

```python
from html_tstring import check_template, TemplateSemanticError

# Component tags are not allowed in the HTML backend
try:
    check_template(t"<Button />")
except TemplateSemanticError as e:
    print(e)  # Component tag <Button> is only valid in the T-HTML backend.

# Interpolation inside raw-text elements is rejected
script = "alert('x')"
try:
    check_template(t"<script>{script}</script>")
except TemplateSemanticError as e:
    print(e)  # Interpolations are not allowed inside <script>.
```

## Runtime errors

Raised during T-HTML rendering when a component cannot be found or called:

```python
from thtml_tstring import TemplateRuntimeError, html

label = "Save"
try:
    html(t"<Missing>{label}</Missing>", globals={}, locals={})
except TemplateRuntimeError as e:
    print(e)  # Unknown component 'Missing'.
```

## XSS prevention

t-strings prevent XSS the same way parameterized SQL prevents injection.
Values go into the parsed AST, not into a raw string, so an attacker
cannot break out of a value slot:

```python
from html_tstring import render_html

# Malicious input
user_input = '<script>alert("xss")</script>'

# f-string: VULNERABLE
fstring = f"<div>{user_input}</div>"
# <div><script>alert("xss")</script></div>

# t-string: SAFE
tstring = render_html(t"<div>{user_input}</div>")
# <div>&lt;script&gt;alert(&quot;xss&quot;)&lt;/script&gt;</div>
```
