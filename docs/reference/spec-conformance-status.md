# Spec Conformance Status

Last updated: 2026-03-22

This repository makes profile-scoped conformance claims rather than claiming
coverage against the full external HTML ecosystem.

## Current Summary

| Format | Profile | Claim | Evidence basis |
| --- | --- | --- | --- |
| HTML | `default` | 100% | Repo manifest plus Python conformance runners and Rust seam/backend conformance runners |
| T-HTML | `default` | 100% | Repo manifest plus Python runtime conformance runners and Rust seam/backend conformance runners |

Current manifest totals:

- HTML `default`: 34 cases
- T-HTML `default`: 47 cases

The current matrix now explicitly covers:

- HTML escaping, `RawHtml`, `Renderable`, fragment insertion, spread merge, class normalization, boolean attrs, parse errors, formatter raw-source fidelity, cache-key derivation, and semantic diagnostic spans
- T-HTML component resolution, explicit and captured scope, spread props, children normalization, component return normalization, `Renderable` composition, auto-wrap behavior, compiled-template per-render scope, parse/semantic errors, and runtime-without-bindings boundaries

## Evidence Contract

The source of truth is:

- `conformance/<format>/profiles.toml`
- `conformance/<format>/profiles/<profile>/spec-map.toml`

Each manifest records:

- the repo-local v1 rule being exercised
- the concrete case id
- the expected result or expected error
- which execution layer must enforce it: `python`, `rust`, or `both`
- optional notes and provenance details

Python and Rust conformance runners both resolve those manifests through the
same contract.

Representative recently-added case groups:

- HTML
  - `renderable-attribute-escaped`
  - `renderable-spread-escaped`
  - `thtml-renderable-child`
  - `format-rich-roundtrip`
  - `semantic-attr-primary-span`
  - `semantic-component-primary-span`
- T-HTML
  - `renderable-scope-frozen`
  - `renderable-ignores-later-overrides`
  - `auto-wrap-uses-defining-globals`
  - `html-renderable-inside-thtml`
  - `compiled-render-scope-per-call`
  - `partial-scope-globals-only`
  - `partial-scope-locals-only`
  - `semantic-attr-primary-span`
  - `semantic-raw-text-primary-span`

## Scope Notes

- These are repo-local v1 conformance claims, not full WHATWG HTML or browser
  compatibility claims.
- External specifications may inform the design, but upstream spec text and
  fixture bodies are not copied into this repository.
- T-HTML is a Python-first runtime. Its Rust-side conformance evidence focuses
  on syntax, formatting, diagnostics, static validation, cache-key behavior,
  and the explicit runtime-without-bindings boundary.

## Verification Commands

- `uv sync --group dev`
- `uv run pytest -q`
- `cargo test --manifest-path rust/Cargo.toml --workspace --tests`

Targeted conformance-only verification:

- `uv run pytest html-tstring/tests/test_html_conformance.py thtml-tstring/tests/test_thtml_conformance.py -q`
- `cargo test --manifest-path rust/Cargo.toml --test conformance -- --nocapture`
