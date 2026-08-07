#!/usr/bin/env python3
"""Verify that a stable release tag exactly matches the root package version."""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]


def root_version() -> str:
    manifest = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    package = manifest.split("[package]", 1)[1].split("[", 1)[0]
    match = re.search(r'^version\s*=\s*"([^"]+)"', package, re.MULTILINE)
    if match is None:
        raise RuntimeError("root package version is missing")
    return match.group(1)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--tag", required=True)
    args = parser.parse_args()
    expected = f"v{root_version()}"
    if args.tag != expected:
        print(
            f"release tag {args.tag!r} does not match root package version {expected!r}",
            file=sys.stderr,
        )
        return 1
    tagged_commit = subprocess.check_output(
        ["git", "rev-list", "-n", "1", args.tag], cwd=ROOT, text=True
    ).strip()
    head = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
    ).strip()
    if tagged_commit != head:
        print(
            f"release tag resolves to {tagged_commit}, but checkout is {head}",
            file=sys.stderr,
        )
        return 1
    print(f"stable release tag verified: {args.tag} at {head}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
