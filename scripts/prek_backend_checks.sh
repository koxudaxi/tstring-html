#!/usr/bin/env bash

set -euo pipefail

package_dir="$1"
shift

cd "$package_dir"

uv run --group dev ruff format --check . ../examples
uv run --group dev ruff check . ../examples

ty_args=(src tests)
for example_file in "$@"; do
  ty_args+=("../examples/${example_file}")
done

uv run --group dev ty check "${ty_args[@]}"
