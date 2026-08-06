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


def run_build(command: list[str]) -> float:
    started = time.monotonic()
    subprocess.run(command, cwd=ROOT, check=True)
    return time.monotonic() - started


def clean_target_directory() -> Path:
    configured = os.environ.get("CARGO_TARGET_DIR")
    if configured is None:
        raise RuntimeError(
            "CARGO_TARGET_DIR must identify an empty directory so the clean build "
            "measurement is unambiguous"
        )
    target_dir = Path(configured)
    if not target_dir.is_absolute():
        target_dir = ROOT / target_dir
    if target_dir.exists() and any(target_dir.iterdir()):
        raise RuntimeError(f"clean build target directory is not empty: {target_dir}")
    return target_dir


def build(distribution: str) -> tuple[Path, float, float]:
    target_dir = clean_target_directory()
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
    clean_elapsed = run_build(command)
    incremental_elapsed = run_build(command)
    executable = target_dir / "release" / ("mech.exe" if os.name == "nt" else "mech")
    if not executable.is_file():
        raise RuntimeError(f"release executable was not produced: {executable}")
    return executable, clean_elapsed, incremental_elapsed


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
        executable, clean_elapsed, incremental_elapsed = build(args.distribution)
        target = host_target()
        archive = package(args, executable, target)
        contract = json.loads(
            (
                ROOT
                / "tests"
                / "architecture"
                / "distributions"
                / f"{args.distribution}.json"
            ).read_text(encoding="utf-8")
        )
    except (OSError, RuntimeError, subprocess.CalledProcessError, KeyError) as error:
        print(f"distribution artifact build failed: {error}", file=sys.stderr)
        return 1
    report = {
        "archive": str(archive),
        "archive_bytes": archive.stat().st_size,
        "clean_build_seconds": round(clean_elapsed, 3),
        "distribution": args.distribution,
        "executable_bytes": executable.stat().st_size,
        "incremental_build_seconds": round(incremental_elapsed, 3),
        "resolved_package_count": contract["dependency_count"],
        "runtime_factory_count": contract["runtime_factory_count"],
        "source_specializer_count": contract["source_specializer_count"],
        "target": target,
    }
    args.output_dir.mkdir(parents=True, exist_ok=True)
    report_path = args.output_dir / f"distribution-build-report-{args.distribution}-{target}.json"
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))
    if output_path := os.environ.get("GITHUB_OUTPUT"):
        with Path(output_path).open("a", encoding="utf-8") as output:
            output.write(f"archive={archive}\n")
            output.write(f"report={report_path}\n")
    if summary_path := os.environ.get("GITHUB_STEP_SUMMARY"):
        with Path(summary_path).open("a", encoding="utf-8") as summary:
            summary.write(
                "| Distribution | Target | Clean build | Incremental build | "
                "Executable | Archive | Packages | Factories | Specializers |\n"
            )
            summary.write("|---|---|---:|---:|---:|---:|---:|---:|---:|\n")
            summary.write(
                f"| {args.distribution} | {target} | {clean_elapsed:.3f}s | "
                f"{incremental_elapsed:.3f}s | {report['executable_bytes']} B | "
                f"{report['archive_bytes']} B | {report['resolved_package_count']} | "
                f"{report['runtime_factory_count']} | "
                f"{report['source_specializer_count']} |\n"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
