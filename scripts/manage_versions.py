#!/usr/bin/env python3
"""Manage lockstep package versions across the monorepo."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VERSION_RE = r"(?P<version>\d+\.\d+\.\d+)"


class Rule:
    def __init__(self, path: str, pattern: str, replacement: str, label: str) -> None:
        self.path = ROOT / path
        self.pattern = re.compile(pattern, re.MULTILINE)
        self.replacement = replacement
        self.label = label

    def read(self) -> str:
        return self.path.read_text(encoding="utf-8")

    def extract_all(self) -> list[str]:
        matches = [
            match.group("version")
            for match in self.pattern.finditer(self.read())
        ]
        if not matches:
            raise ValueError(f"Pattern not found for {self.label} in {self.path}")
        return matches

    def replace(self, version: str) -> bool:
        content = self.read()
        updated, count = self.pattern.subn(
            self.replacement.format(version=version),
            content,
        )
        if count == 0:
            raise ValueError(f"Pattern not found for {self.label} in {self.path}")
        if updated != content:
            self.path.write_text(updated, encoding="utf-8")
            return True
        return False


def rust_path_dependency_rule(
    path: str,
    dependency: str,
    dependency_path: str,
    label: str,
) -> Rule:
    return Rule(
        path,
        (
            rf'({re.escape(dependency)} = \{{ version = "){VERSION_RE}'
            rf'(", path = "{re.escape(dependency_path)}" \}})'
        ),
        r"\g<1>{version}\g<3>",
        label,
    )


RULES = [
    Rule(
        "pyproject.toml",
        rf'^(version = "){VERSION_RE}(")$',
        r"\g<1>{version}\g<3>",
        "workspace version",
    ),
    Rule(
        "pyproject.toml",
        rf'("html-tstring>=){VERSION_RE}(")',
        r'\g<1>{version}\g<3>',
        "workspace dependency html-tstring",
    ),
    Rule(
        "pyproject.toml",
        rf'("thtml-tstring>=){VERSION_RE}(")',
        r'\g<1>{version}\g<3>',
        "workspace dependency thtml-tstring",
    ),
    Rule(
        "html-tstring/pyproject.toml",
        rf'^(version = "){VERSION_RE}(")$',
        r"\g<1>{version}\g<3>",
        "html-tstring version",
    ),
    Rule(
        "html-tstring/pyproject.toml",
        rf'("tstring-html-bindings>=){VERSION_RE}(")',
        r'\g<1>{version}\g<3>',
        "html-tstring dependency tstring-html-bindings",
    ),
    Rule(
        "thtml-tstring/pyproject.toml",
        rf'^(version = "){VERSION_RE}(")$',
        r"\g<1>{version}\g<3>",
        "thtml-tstring version",
    ),
    Rule(
        "thtml-tstring/pyproject.toml",
        rf'("tstring-html-bindings>=){VERSION_RE}(")',
        r'\g<1>{version}\g<3>',
        "thtml-tstring dependency tstring-html-bindings",
    ),
    Rule(
        "rust/tstring-html-bindings/pyproject.toml",
        rf'^(version = "){VERSION_RE}(")$',
        r"\g<1>{version}\g<3>",
        "tstring-html-bindings Python package version",
    ),
    Rule(
        "rust/Cargo.toml",
        rf'^(version = "){VERSION_RE}(")$',
        r"\g<1>{version}\g<3>",
        "Rust workspace version",
    ),
    rust_path_dependency_rule(
        "rust/tstring-html-rs/Cargo.toml",
        "tstring-format-doc",
        "../tstring-format-doc-rs",
        "rust tstring-html dependency tstring-format-doc",
    ),
    rust_path_dependency_rule(
        "rust/tstring-tdom-rs/Cargo.toml",
        "tstring-format-doc",
        "../tstring-format-doc-rs",
        "rust tstring-tdom dependency tstring-format-doc",
    ),
    rust_path_dependency_rule(
        "rust/tstring-thtml-rs/Cargo.toml",
        "tstring-html",
        "../tstring-html-rs",
        "rust tstring-thtml dependency tstring-html",
    ),
    rust_path_dependency_rule(
        "rust/tstring-html-bindings/Cargo.toml",
        "tstring-html",
        "../tstring-html-rs",
        "rust bindings dependency tstring-html",
    ),
    rust_path_dependency_rule(
        "rust/tstring-html-bindings/Cargo.toml",
        "tstring-thtml",
        "../tstring-thtml-rs",
        "rust bindings dependency tstring-thtml",
    ),
    rust_path_dependency_rule(
        "rust/backend-e2e-tests/Cargo.toml",
        "tstring-html",
        "../tstring-html-rs",
        "backend e2e dependency tstring-html",
    ),
    rust_path_dependency_rule(
        "rust/backend-e2e-tests/Cargo.toml",
        "tstring-tdom",
        "../tstring-tdom-rs",
        "backend e2e dependency tstring-tdom",
    ),
    rust_path_dependency_rule(
        "rust/backend-e2e-tests/Cargo.toml",
        "tstring-thtml",
        "../tstring-thtml-rs",
        "backend e2e dependency tstring-thtml",
    ),
]


def normalize_tag(tag: str | None) -> str | None:
    if tag is None:
        return None
    return tag.removeprefix("refs/tags/").removeprefix("v").removeprefix("V")


def check(version_from_tag: str | None) -> int:
    versions: dict[str, list[str]] = {}
    for rule in RULES:
        versions[f"{rule.path}:{rule.label}"] = rule.extract_all()

    unique_versions = sorted({item for items in versions.values() for item in items})
    if len(unique_versions) != 1:
        for label, items in versions.items():
            print(f"{label}: {items}", file=sys.stderr)
        print("version mismatch across workspace packages", file=sys.stderr)
        return 1

    version = unique_versions[0]
    expected = normalize_tag(version_from_tag)
    if expected is not None and version != expected:
        print(
            f"workspace version {version} does not match requested tag {expected}",
            file=sys.stderr,
        )
        return 1

    print(version)
    return 0


def set_version(version: str) -> int:
    cleaned = normalize_tag(version)
    if cleaned is None or not re.fullmatch(VERSION_RE, cleaned):
        print(
            f"Expected semantic version like 0.1.0, got: {version}",
            file=sys.stderr,
        )
        return 1

    changed = False
    for rule in RULES:
        changed = rule.replace(cleaned) or changed

    if changed:
        print(f"Updated workspace versions to {cleaned}")
    else:
        print(f"Workspace already at {cleaned}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Manage lockstep versions for tstring-html."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    check_parser = subparsers.add_parser(
        "check",
        help="Validate lockstep versions.",
    )
    check_parser.add_argument(
        "--tag",
        default=None,
        help="Optional git tag to compare against.",
    )

    set_parser = subparsers.add_parser("set", help="Rewrite lockstep versions.")
    set_parser.add_argument(
        "version",
        help="Semantic version like 0.1.0 or tag like v0.1.0.",
    )

    return parser


def main() -> int:
    args = build_parser().parse_args()
    if args.command == "check":
        return check(args.tag)
    if args.command == "set":
        return set_version(args.version)
    raise AssertionError(f"Unsupported command: {args.command}")


if __name__ == "__main__":
    raise SystemExit(main())
