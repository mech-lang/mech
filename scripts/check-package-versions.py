#!/usr/bin/env python3
"""Check active Mech package identities without rewriting historical evidence.

Release versions belong to Cargo manifests and local package lock entries.
They are deliberately independent of bytecode/ABI versions and archived
benchmark toolchain metadata. No Cargo resolution or network access is needed.
"""

from __future__ import annotations

import argparse
from pathlib import Path
import sys
import tomllib


def load(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def dependency_tables(document: dict):
    for key in ("dependencies", "dev-dependencies", "build-dependencies"):
        yield document.get(key, {})
    for target in document.get("target", {}).values():
        yield from dependency_tables(target)


def check(root: Path) -> list[str]:
    version = load(root / "Cargo.toml")["package"]["version"]
    release_paths = {root / "Cargo.toml"}
    for directory in ("src", "hosts", "machines"):
        release_paths.update((root / directory).glob("*/Cargo.toml"))
    fixture = root / "tests/fixtures/native-live-host/Cargo.toml"
    if fixture.exists():
        release_paths.add(fixture)
    manifests = {path: load(path) for path in sorted(release_paths)}
    packages = {data["package"]["name"] for data in manifests.values()}
    errors = []
    for path, data in manifests.items():
        if data["package"]["version"] != version:
            errors.append(f"{path.relative_to(root)}: package version must be {version}")

    # The standalone renderer keeps its own version, but any explicit
    # requirements on release crates must use the current line. Other fixture
    # consumers use path-only requirements and are checked through their locks.
    renderer = root / "benchmarks/iros-2026/blog/render/Cargo.toml"
    if renderer.exists():
        manifests[renderer] = load(renderer)
    for path, data in manifests.items():
        for dependencies in dependency_tables(data):
            for alias, dependency in dependencies.items():
                name = dependency.get("package", alias) if isinstance(dependency, dict) else alias
                if name not in packages:
                    continue
                required = dependency.get("version") if isinstance(dependency, dict) else dependency
                if required is not None and required != version:
                    errors.append(f"{path.relative_to(root)}: {alias} requirement must be {version}")

    lock_paths = {root / "Cargo.lock", root / "benchmarks/iros-2026/blog/render/Cargo.lock"}
    lock_paths.update((root / "tests/fixtures").glob("*/Cargo.lock"))
    for path in sorted(lock_paths):
        if not path.exists():
            continue
        for package in load(path).get("package", []):
            if package["name"] in packages and "source" not in package and package["version"] != version:
                errors.append(f"{path.relative_to(root)}: {package['name']} lock version must be {version}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    root = parser.parse_args().root.resolve()
    errors = check(root)
    for error in errors:
        print(error, file=sys.stderr)
    if not errors:
        print(f"PASS: active Mech packages, internal requirements and local lock entries match {load(root / 'Cargo.toml')['package']['version']}")
    return int(bool(errors))


if __name__ == "__main__":
    raise SystemExit(main())
