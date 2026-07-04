# Conformance Assets

This directory contains the reproducible evidence used for repo-local v1
language-spec conformance across the HTML and T-HTML backends.

## Structure

- `conformance/<format>/profiles.toml` is the profile index for that format.
- `conformance/<format>/profiles/<profile>/spec-map.toml` is the per-profile manifest.

Each per-profile `spec-map.toml` manifest is the source of truth for:

- the repo-local v1 rule being exercised
- the concrete case id
- the expected result or expected error
- which execution layer must enforce it (`rust`, `python`, or `both`)
- optional notes for provenance or representability details

Each `profiles.toml` index is the source of truth for:

- which profiles are supported for that format
- which profile is the default
- where each per-profile manifest lives under `[profiles.<profile>]`, relative to
  `conformance/<format>`

## Current Status

The repository currently claims:

- HTML `default`
- T-HTML `default`

Current manifest totals:

- HTML `default`: 34 cases
- T-HTML `default`: 47 cases

The manifests currently cover, among other things:

- HTML escaping, `RawHtml`, `Renderable`, fragment insertion, spread/class semantics, parse failures, formatter fidelity, cache-key derivation, and semantic span diagnostics
- T-HTML component resolution, captured scope, explicit scope, children normalization, component return normalization, `Renderable` composition, decorator auto-wrap, compiled-template scope behavior, and runtime-without-bindings boundaries

## Verification

Python conformance checks run as part of the package `pytest` suites:

```bash
uv run pytest -q
```

Rust conformance checks run as workspace integration tests:

```bash
cargo test --manifest-path rust/Cargo.toml --workspace --tests
```

Targeted conformance-only verification is also available:

```bash
uv run pytest html-tstring/tests/test_html_conformance.py thtml-tstring/tests/test_thtml_conformance.py -q
cargo test --manifest-path rust/Cargo.toml --test conformance -- --nocapture
```

## Scope and licensing

These manifests act as the v1 spec matrix for this repository rather than as a
copy of an upstream HTML standard corpus.

- the source of truth stays inside this repository
- external standards may be referenced conceptually
- upstream spec text and fixture bodies are not copied into this tree
