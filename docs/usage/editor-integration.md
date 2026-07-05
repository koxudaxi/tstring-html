# Editor Integration (t-linter)

[t-linter](https://github.com/koxudaxi/t-linter) is a linter, formatter, and LSP server for Python template strings (PEP 750 t-strings). It uses the same Rust parser (`tstring-html-rs`, `tstring-thtml-rs`) as this project for `check` and `format` operations.

## Installation

```bash
pip install t-linter
```

Or with uv:

```bash
uv add t-linter
```

## CLI usage

### Check templates for errors

Validate t-string templates in your Python files:

```bash
t-linter check file.py
t-linter check src/
```

Output format options:

```bash
t-linter check file.py --format human    # default, human-readable
t-linter check file.py --format json     # machine-readable JSON
t-linter check file.py --format github   # GitHub Actions annotations
```

Use `--error-on-issues` to fail CI when problems are found:

```bash
t-linter check src/ --error-on-issues
```

### Format templates

Canonical formatting for HTML and T-HTML template literals:

```bash
t-linter format file.py
t-linter format src/
```

Dry-run to see what would change without modifying files:

```bash
t-linter format --check file.py
```

## LSP server

Start the built-in LSP server for real-time editor diagnostics and formatting:

```bash
t-linter lsp
```

The LSP provides:

- **Diagnostics** — syntax and semantic errors reported inline as you type
- **Formatting** — format-on-save or format-on-demand via your editor's formatting command

## VSCode integration

1. Install the binary:

    ```bash
    pip install t-linter
    ```

2. Install the [t-linter extension](https://marketplace.visualstudio.com/items?itemName=koxudaxi.t-linter) from the VSCode marketplace.

3. Recommended: set `"python.languageServer": "None"` in VSCode settings to avoid conflicts with other Python language servers.

4. Optionally configure `t-linter.serverPath` in settings if the binary is not on your `PATH`.

## Other editors

Any editor that supports the Language Server Protocol can use `t-linter lsp`. Configure your editor to start `t-linter lsp` as the language server for Python files.

## Configuration

Configure t-linter via `pyproject.toml`:

```toml
[tool.t-linter]
extend-exclude = ["generated", "vendor"]
ignore-file = ".t-linterignore"
```

## How it works with tstring-html

t-linter and tstring-html share the same Rust parsing and formatting backends:

- **tstring-html** is the **runtime library** — `render_html()` and `html()` parse and render templates at runtime
- **t-linter** is the **developer tooling** — `check` validates templates and `format` canonicalizes them at development time

The integration seam is:

- Backend Rust APIs accept `tstring_syntax::TemplateInput`
- Diagnostics use `tstring_syntax::SourceSpan`
- Tooling constructs `TemplateInput` from source text and preserves `raw_source`
- No Python runtime object is required for lint/format
- Reserved language ids: `"html"`, `"thtml"`, and `"tdom"`

For TDOM component prop type checking, tools should use
`tstring_tdom::interpolation_type_requirements_with_component_props`. The
backend walks component tags, normalizes prop names such as `data-user-id` to
`data_user_id`, and passes each concrete prop interpolation to the caller. The
caller resolves the component expression and returns the expected Python
annotation. This keeps TDOM syntax ownership in the backend while letting tools
such as t-linter use their own Python type-resolution model.
