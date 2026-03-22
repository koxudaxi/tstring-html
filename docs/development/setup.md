# Development Setup

## Prerequisites

- Python 3.14
- [uv](https://docs.astral.sh/uv/)
- [Rust 1.94.0+](https://www.rust-lang.org/tools/install) (for building `tstring-html-bindings`)
- git

## Install dependencies

The repository uses a root `uv` workspace plus an in-repo Rust workspace:

```bash
uv sync --group dev
```

`uv sync` installs the `tstring-html-bindings` workspace package as part of the normal dependency graph. Use `maturin develop` only when you are working on the bindings package in isolation.

## Verification commands

### Python workspace

```bash
uv sync --group dev
uv run ruff format --check html-tstring thtml-tstring tests examples
uv run ruff check .
uv run pytest -q
uv run coverage run -m pytest -q && uv run coverage report
```

### Rust workspace

```bash
cargo test --manifest-path rust/Cargo.toml --workspace --tests
```

Conformance claims are backed by the shared manifests under `conformance/`. Both
Python and Rust test suites consume those manifests as evidence contracts for
the repo-local v1 language-spec matrix.

### Build distributions

```bash
uv build html-tstring --out-dir dist
uv build thtml-tstring --out-dir dist
uv build rust/tstring-html-bindings --out-dir dist
uvx twine check dist/*
```

## Guidelines

- Keep repository content in English only.
- Prefer small, reviewable commits.
- The authoritative backend behavior is protected by Python end-to-end tests and Rust crate tests — keep both layers covered when changing runtime behavior.
- Python package coverage target is 100%.
