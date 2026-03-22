# Installation

Requires Python 3.14+ ([PEP 750](https://peps.python.org/pep-0750/) t-strings).

## Install a backend

Pick the package you need:

=== "HTML"

    ```bash
    pip install html-tstring
    ```

=== "T-HTML (Components)"

    ```bash
    pip install thtml-tstring
    ```

Or with [uv](https://docs.astral.sh/uv/):

=== "HTML"

    ```bash
    uv add html-tstring
    ```

=== "T-HTML (Components)"

    ```bash
    uv add thtml-tstring
    ```

Each package automatically pulls in `tstring-html-bindings` (native extension).

## Platform wheels

Release automation publishes pre-built wheels for:

| Platform | Architecture |
|----------|-------------|
| Linux | x86_64 (GNU) |
| macOS | Apple Silicon (arm64) |
| Windows | x86_64 |

Other environments require a local Rust 1.94.0+ toolchain to build `tstring-html-bindings` from source.

## Installing both packages

You can install both packages in the same environment:

```bash
pip install html-tstring thtml-tstring
```

They share the same `tstring-html-bindings` dependency, so there is no conflict.
