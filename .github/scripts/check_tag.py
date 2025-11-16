#!/usr/bin/env python3
"""Validate that the git tag matches crate versions.

Usage:
    python .github/scripts/check_tag.py [tag]

If no tag is supplied explicitly, the script falls back to the
`GITHUB_REF_NAME` environment variable (as provided by GitHub Actions).
"""

from __future__ import annotations

import os
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def read_package_version(path: Path) -> str:
    try:
        data = tomllib.loads(path.read_text("utf-8"))
    except FileNotFoundError as exc:  # pragma: no cover - defensive guard for CI
        raise SystemExit(f"unable to read {path}: {exc}") from exc

    try:
        return data["package"]["version"]
    except KeyError as exc:  # pragma: no cover - ensures clear error reporting
        raise SystemExit(f"missing package.version in {path}") from exc


def resolve_tag() -> str:
    tag = sys.argv[1] if len(sys.argv) > 1 else os.getenv("GITHUB_REF_NAME")
    if not tag:
        raise SystemExit("release tag not provided via argument or GITHUB_REF_NAME")
    return tag


def parse_tag(tag: str) -> tuple[str, str]:
    if tag.startswith("v"):
        return "release", tag[1:]

    if "-" not in tag:
        raise SystemExit(
            "release tags must either start with 'v' or use the '<kind>-<version>' format"
        )

    kind, version = tag.split("-", 1)

    if kind not in {"server", "sdk", "protos", "docker"}:
        raise SystemExit(f"unsupported release tag kind '{kind}'")

    if not version:
        raise SystemExit(f"release tag '{tag}' is missing a version segment")

    return kind, version


def main() -> None:
    tag = resolve_tag()
    kind, version = parse_tag(tag)

    checks: dict[str, list[tuple[Path, str]]] = {
        "server": [(ROOT / "Cargo.toml", "turn-server")],
        "sdk": [(ROOT / "sdk" / "Cargo.toml", "turn-server-sdk")],
        "protos": [(ROOT / "protos" / "Cargo.toml", "turn-server-protos")],
        "docker": [(ROOT / "Cargo.toml", "turn-server")],
        "release": [(ROOT / "Cargo.toml", "turn-server")],
    }

    targets = checks.get(kind, [])
    if not targets:
        print(f"no crate version checks configured for tag '{tag}'")
        return

    errors: list[str] = []
    for manifest, crate_name in targets:
        crate_version = read_package_version(manifest)
        if crate_version != version:
            errors.append(
                f"tag version {version} does not match {crate_name} crate version {crate_version}"
            )

    if errors:
        raise SystemExit("\n".join(errors))

    checked = ", ".join(crate for _, crate in targets)
    print(f"tag {tag} validated against crate(s): {checked}")


if __name__ == "__main__":
    main()
