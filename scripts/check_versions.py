#!/usr/bin/env python3

from __future__ import annotations

import argparse
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]


def load_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def normalized_tag(tag: str | None) -> str | None:
    if tag is None:
        return None
    cleaned = tag.removeprefix("refs/tags/").removeprefix("v").removeprefix("V")
    return cleaned


def collect_versions() -> dict[str, str]:
    versions = {
        "workspace": load_toml(REPO_ROOT / "pyproject.toml")["project"]["version"],
        "html-tstring": load_toml(REPO_ROOT / "html-tstring" / "pyproject.toml")[
            "project"
        ]["version"],
        "thtml-tstring": load_toml(REPO_ROOT / "thtml-tstring" / "pyproject.toml")[
            "project"
        ]["version"],
        "tstring-html-bindings": load_toml(
            REPO_ROOT / "rust" / "tstring-html-bindings" / "pyproject.toml"
        )["project"]["version"],
        "rust-workspace": load_toml(REPO_ROOT / "rust" / "Cargo.toml")["workspace"][
            "package"
        ]["version"],
    }
    return versions


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", default=None)
    args = parser.parse_args()

    versions = collect_versions()
    expected = normalized_tag(args.tag)
    unique_versions = set(versions.values())
    if len(unique_versions) != 1:
        for name, version in versions.items():
            print(f"{name}: {version}", file=sys.stderr)
        print("version mismatch across workspace packages", file=sys.stderr)
        return 1

    version = unique_versions.pop()
    if expected is not None and version != expected:
        print(
            f"workspace version {version} does not match requested tag {expected}",
            file=sys.stderr,
        )
        return 1

    print(version)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
