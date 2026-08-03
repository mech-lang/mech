#!/usr/bin/env python3
"""Validate Cargo package contents without requiring unpublished registry entries."""

from __future__ import annotations

import json
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "target/package-archive-manifests"
PACKAGES = {
    "mech-core": ROOT / "src/core/Cargo.toml",
    "mech-engine": ROOT / "src/engine/Cargo.toml",
    "mech-runtime": ROOT / "src/runtime/Cargo.toml",
    "mech-build": ROOT / "src/build/Cargo.toml",
    "mech-math": ROOT / "machines/math/Cargo.toml",
    "mech-compare": ROOT / "machines/compare/Cargo.toml",
    "mech-logic": ROOT / "machines/logic/Cargo.toml",
    "mech-range": ROOT / "machines/range/Cargo.toml",
    "mech-matrix": ROOT / "machines/matrix/Cargo.toml",
    "mech-set": ROOT / "machines/set/Cargo.toml",
    "mech-string": ROOT / "machines/string/Cargo.toml",
    "mech-stats": ROOT / "machines/stats/Cargo.toml",
    "mech-combinatorics": ROOT / "machines/combinatorics/Cargo.toml",
    "mech-host-cli": ROOT / "hosts/cli/Cargo.toml",
    "mech-host-console": ROOT / "hosts/console/Cargo.toml",
    "mech-host-time": ROOT / "hosts/time/Cargo.toml",
    "mech-host-timer": ROOT / "hosts/timer/Cargo.toml",
    "mech-host-scene": ROOT / "hosts/scene/Cargo.toml",
    "mech-host-robot-arm": ROOT / "hosts/robot-arm/Cargo.toml",
}
REQUIRED_RESOURCES = {
    "mech-host-cli": ("host.mcfg",),
    "mech-host-console": ("host.mcfg",),
    "mech-host-time": ("host.mcfg",),
    "mech-host-timer": ("host.mcfg",),
    "mech-host-scene": ("host.mcfg",),
    "mech-host-robot-arm": ("host.mcfg",),
}


class ArchiveContractError(RuntimeError):
    pass


def command(arguments: list[str]) -> str:
    completed = subprocess.run(
        arguments,
        cwd=ROOT,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if completed.returncode:
        raise ArchiveContractError(
            f"command failed ({completed.returncode}): {' '.join(arguments)}\n"
            f"{completed.stderr.strip()}"
        )
    return completed.stdout


def manifest_package(package: str, manifest: Path) -> dict[str, str]:
    if not manifest.is_file():
        raise ArchiveContractError(f"{package} manifest is missing: {manifest.relative_to(ROOT)}")
    source = manifest.read_text(encoding="utf-8")
    section = re.search(r"^\[package\]\s*(.*?)(?=^\[|\Z)", source, re.MULTILINE | re.DOTALL)
    if section is None:
        raise ArchiveContractError(f"{package} manifest has no package table")
    name = re.search(r'^name\s*=\s*"([^"]+)"\s*$', section.group(1), re.MULTILINE)
    version = re.search(r'^version\s*=\s*"([^"]+)"\s*$', section.group(1), re.MULTILINE)
    if name is None or name.group(1) != package or version is None:
        raise ArchiveContractError(f"{package} manifest has an invalid package identity")
    return {
        "name": package,
        "version": version.group(1),
        "manifest_path": str(manifest.relative_to(ROOT)),
    }


def package_files(package: str, manifest: Path) -> list[str]:
    output = command(
        [
            "cargo",
            "+nightly-2026-03-03",
            "package",
            "--allow-dirty",
            "--no-verify",
            "--list",
            "--manifest-path",
            str(manifest),
        ]
    )
    files = [line.strip() for line in output.splitlines() if line.strip()]
    if not files:
        raise ArchiveContractError(f"{package} produced an empty Cargo package file list")
    if files != sorted(files):
        raise ArchiveContractError(f"{package} package file list is not deterministic")
    for value in files:
        path = PurePosixPath(value)
        if path.is_absolute() or ".." in path.parts:
            raise ArchiveContractError(f"{package} contains an unsafe archive path: {value}")
    return files


def validate_package(package: str, manifest: Path) -> dict[str, object]:
    metadata = manifest_package(package, manifest)
    files = package_files(package, manifest)
    required = {"Cargo.toml", "Cargo.toml.orig", *REQUIRED_RESOURCES.get(package, ())}
    missing = sorted(required.difference(files))
    if missing:
        raise ArchiveContractError(f"{package} omits required package files: {missing}")
    if not any(path.startswith("src/") and path.endswith(".rs") for path in files):
        raise ArchiveContractError(f"{package} package contains no Rust source")
    return {
        "schema": "mech.package-archive-file-list.v1",
        "package": package,
        "version": metadata["version"],
        "manifest_path": metadata["manifest_path"],
        "validation": "cargo-package-list",
        "registry_dependency_resolution": "intentionally-not-claimed",
        "required_resources": sorted(required),
        "file_count": len(files),
        "files": files,
    }


def main() -> int:
    try:
        OUTPUT.mkdir(parents=True, exist_ok=True)
        total_files = 0
        for package, manifest in PACKAGES.items():
            report = validate_package(package, manifest)
            total_files += report["file_count"]
            (OUTPUT / f"{package}.json").write_text(
                json.dumps(report, indent=2) + "\n",
                encoding="utf-8",
            )
            print(f"package archive contents: {package} ({report['file_count']} files)")
        print(
            f"validated {len(PACKAGES)} Cargo package file sets ({total_files} files); "
            "unpublished registry dependency resolution was not claimed"
        )
        return 0
    except (ArchiveContractError, OSError, KeyError, TypeError, ValueError) as error:
        print(f"package archive validation failed: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
