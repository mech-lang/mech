#!/usr/bin/env python3
"""Build and package one standard/full distribution for the current platform."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time


ROOT = Path(__file__).resolve().parents[1]
TOOLCHAIN = "nightly-2026-03-03"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--channel", choices=("stable", "nightly"), required=True)
    parser.add_argument("--distribution", choices=("standard", "full"), required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--date")
    return parser.parse_args()


def root_version() -> str:
    manifest = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    package = manifest.split("[package]", 1)[1].split("[", 1)[0]
    match = re.search(r'^version\s*=\s*"([^"]+)"', package, re.MULTILINE)
    if match is None:
        raise RuntimeError("root package version is missing")
    return match.group(1)


def host_target() -> str:
    output = subprocess.check_output(
        ["rustc", f"+{TOOLCHAIN}", "-vV"], cwd=ROOT, text=True
    )
    for line in output.splitlines():
        if line.startswith("host: "):
            return line.removeprefix("host: ")
    raise RuntimeError("rustc did not report its host target")


def build(distribution: str) -> tuple[Path, float]:
    command = [
        "cargo",
        f"+{TOOLCHAIN}",
        "build",
        "--locked",
        "--release",
        "--bin",
        "mech",
    ]
    if distribution == "full":
        command.extend(["--no-default-features", "--features", "distribution-full"])
    started = time.monotonic()
    subprocess.run(command, cwd=ROOT, check=True)
    elapsed = time.monotonic() - started
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    if not target_dir.is_absolute():
        target_dir = ROOT / target_dir
    executable = target_dir / "release" / ("mech.exe" if os.name == "nt" else "mech")
    if not executable.is_file():
        raise RuntimeError(f"release executable was not produced: {executable}")
    return executable, elapsed


def package(args: argparse.Namespace, executable: Path, target: str) -> Path:
    contract = json.loads(
        (ROOT / "tests" / "architecture" / "distributions" / f"{args.distribution}.json")
        .read_text(encoding="utf-8")
    )
    commit = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
    ).strip()
    command = [
        sys.executable,
        str(ROOT / "scripts" / "package-distribution.py"),
        "--binary",
        str(executable),
        "--channel",
        args.channel,
        "--distribution",
        args.distribution,
        "--version",
        root_version(),
        "--commit",
        commit,
        "--toolchain",
        TOOLCHAIN,
        "--target",
        target,
        "--runtime-factory-count",
        str(contract["runtime_factory_count"]),
        "--source-specializer-count",
        str(contract["source_specializer_count"]),
        "--output-dir",
        str(args.output_dir),
    ]
    if args.channel == "nightly":
        if not args.date:
            raise RuntimeError("--date is required for nightly artifacts")
        command.extend(["--date", args.date])
    elif args.date:
        raise RuntimeError("--date is only valid for nightly artifacts")
    output = subprocess.check_output(command, cwd=ROOT, text=True).strip()
    archive = Path(output)
    if not archive.is_file():
        raise RuntimeError(f"distribution archive was not produced: {archive}")
    return archive


def main() -> int:
    args = parse_args()
    try:
        executable, elapsed = build(args.distribution)
        target = host_target()
        archive = package(args, executable, target)
    except (OSError, RuntimeError, subprocess.CalledProcessError, KeyError) as error:
        print(f"distribution artifact build failed: {error}", file=sys.stderr)
        return 1
    print(
        json.dumps(
            {
                "archive": str(archive),
                "build_seconds": round(elapsed, 3),
                "distribution": args.distribution,
                "executable_bytes": executable.stat().st_size,
                "target": target,
            },
            sort_keys=True,
        )
    )
    if output_path := os.environ.get("GITHUB_OUTPUT"):
        with Path(output_path).open("a", encoding="utf-8") as output:
            output.write(f"archive={archive}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
