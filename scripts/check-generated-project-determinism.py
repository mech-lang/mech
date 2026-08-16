#!/usr/bin/env python3
"""Generate every native Cargo project twice and compare frozen files."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
FILES = [
    "Cargo.toml",
    "build-plan.json",
    "program.mecb",
    "rust-toolchain.toml",
    "src/main.rs",
    "src/catalog.rs",
    "src/runtime.rs",
]
EXPECTED_LAYOUT = set(FILES + ["Cargo.lock"])
EXPECTED_ENTRIES = EXPECTED_LAYOUT | {"src"}
EXPECTED_PROJECT_COUNT = 15


def generate() -> dict[str, Path]:
    process = subprocess.run(
        [
            "cargo",
            "+nightly-2026-03-03",
            "test",
            "-p",
            "mech-build",
            "--features",
            "full-hosts",
            "--test",
            "generate_native_projects",
            "--",
            "--nocapture",
            "--test-threads=1",
        ],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    if process.returncode:
        raise RuntimeError(f"project generation failed:\n{process.stdout}")
    projects: dict[str, Path] = {}
    for line in process.stdout.splitlines():
        marker = "MECH_NATIVE_PROJECT="
        if marker not in line:
            continue
        path = Path(line.split(marker, 1)[1].strip())
        plan = json.loads((path / "build-plan.json").read_text(encoding="utf-8"))
        expected_path = (
            ROOT / "target/mech-native/projects" / plan["plan_sha256"]
        )
        if path != expected_path:
            raise RuntimeError(
                f"{plan['binary_name']}: project root {path} != {expected_path}"
            )
        projects[plan["binary_name"]] = path
    if len(projects) != EXPECTED_PROJECT_COUNT:
        raise RuntimeError(
            f"expected {EXPECTED_PROJECT_COUNT} generated projects, found {len(projects)}"
        )
    return projects


def snapshot(projects: dict[str, Path]) -> dict[str, dict[str, bytes]]:
    result: dict[str, dict[str, bytes]] = {}
    root_bytes = str(ROOT).encode()
    for binary, project in projects.items():
        for path in project.rglob("*"):
            if path.is_symlink():
                raise RuntimeError(f"{binary}: generated layout contains symlink {path}")
        actual = {
            str(path.relative_to(project))
            for path in project.rglob("*")
        }
        if actual != EXPECTED_ENTRIES:
            raise RuntimeError(
                f"{binary}: generated layout {sorted(actual)} != {sorted(EXPECTED_ENTRIES)}"
            )
        if not (project / "src").is_dir():
            raise RuntimeError(f"{binary}: generated src entry is not a directory")
        for name in EXPECTED_LAYOUT:
            if not (project / name).is_file():
                raise RuntimeError(f"{binary}: generated {name} is not a regular file")
        files = {name: (project / name).read_bytes() for name in FILES}
        for name in ["Cargo.toml", "build-plan.json", "src/main.rs", "src/catalog.rs", "src/runtime.rs"]:
            if root_bytes in files[name]:
                raise RuntimeError(f"{binary}: {name} contains an absolute workspace path")
        result[binary] = files
    return result


def remove_frozen_files(projects: dict[str, Path]) -> None:
    """Force the second process to recreate every required project file."""

    for binary, project in projects.items():
        for name in sorted(EXPECTED_LAYOUT):
            path = project / name
            if path.is_symlink() or not path.is_file():
                raise RuntimeError(f"{binary}: refusing to remove non-regular {path}")
            path.unlink()


def main() -> int:
    try:
        first_projects = generate()
        first = snapshot(first_projects)
        remove_frozen_files(first_projects)
        second_projects = generate()
        second = snapshot(second_projects)
        if first_projects != second_projects:
            raise RuntimeError("plan-addressed project roots changed between processes")
        if first != second:
            for binary in sorted(first):
                for name in FILES:
                    if first[binary][name] != second[binary][name]:
                        raise RuntimeError(f"{binary}: {name} changed between generations")
            raise RuntimeError("generated project bytes changed between generations")
    except (OSError, ValueError, KeyError, RuntimeError) as error:
        print(f"generated project determinism contract failed: {error}", file=sys.stderr)
        return 1
    print(
        "generated project determinism contract passed "
        f"({EXPECTED_PROJECT_COUNT} projects, 2 processes)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
