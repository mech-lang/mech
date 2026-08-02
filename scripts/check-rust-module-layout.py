#!/usr/bin/env python3
"""Reject Rust module layouts that would reintroduce forbidden boundaries."""

from __future__ import annotations

import json
from pathlib import Path
import subprocess
import sys


REPOSITORY_ROOT = Path(__file__).resolve().parent.parent


def workspace_source_roots() -> set[Path]:
    command = [
        "cargo",
        "+nightly-2026-03-03",
        "metadata",
        "--manifest-path",
        str(REPOSITORY_ROOT / "Cargo.toml"),
        "--format-version",
        "1",
        "--no-deps",
    ]
    metadata = json.loads(subprocess.check_output(command, text=True))
    roots = {
        Path(target["src_path"]).resolve().parent
        for package in metadata["packages"]
        for target in package["targets"]
    }
    if not roots:
        raise RuntimeError("cargo metadata did not report any Rust source roots")
    return roots


def relative(path: Path) -> str:
    return path.resolve().relative_to(REPOSITORY_ROOT).as_posix()


def main() -> int:
    failures: list[str] = []
    source_files: set[Path] = set()
    for root in workspace_source_roots():
        if root.is_relative_to(REPOSITORY_ROOT):
            source_files.update(root.rglob("*.rs"))

    for source_file in sorted(source_files):
        if source_file.with_suffix("").is_dir():
            failures.append(f"{relative(source_file)}: file/directory module collision")

    forbidden_paths = {
        "src/core/src/browser.rs": "browser authority belongs to mech-host-browser",
        "src/engine/src/functions.rs": "use engine/function/mod.rs",
        "src/engine/src/function_catalog.rs": "use engine/function/catalog.rs",
        "src/engine/src/function_environment.rs": "use engine/function/environment.rs",
        "src/engine/src/function_extensions.rs": "use engine/function/extensions.rs",
        "src/engine/src/function_resolver.rs": "use engine/function/resolver.rs",
        "src/engine/src/program_state.rs": "use engine/program/state.rs",
        "src/runtime/src/config_profile.rs": "use runtime/config/profile.rs",
        "src/runtime/src/config_spec.rs": "use runtime/config/spec.rs",
        "src/runtime/src/host_interface.rs": "use runtime/host/interface.rs",
        "src/runtime/src/host_delegation.rs": "use runtime/host/delegation/mod.rs",
        "src/runtime/src/host_delegation_crypto.rs": "use runtime/host/delegation/crypto.rs",
    }
    for path, rule in forbidden_paths.items():
        if (REPOSITORY_ROOT / path).exists():
            failures.append(f"{path}: {rule}")

    if failures:
        print("Rust module layout contract failed:", file=sys.stderr)
        print(*failures, sep="\n", file=sys.stderr)
        return 1

    print("Rust module layout contract passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
