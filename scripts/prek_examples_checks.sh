#!/usr/bin/env bash

set -euo pipefail

uv run --group dev ruff format --check examples
uv run --group dev ruff check examples
