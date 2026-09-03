#!/usr/bin/env python3
"""Run the retired D2 executor from its immutable implementation commit."""

from __future__ import annotations

import io
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile


ROOT = Path(__file__).resolve().parents[1]
D2_HEAD = "96fd051608f9d9df9eb4e9b345af7c23279c6c67"
HISTORICAL_DEPENDENCY_PINS = (("tinyvec", "1.12.0"),)


def historical_dependency_pin_commands(manifest: Path) -> list[list[str]]:
    return [
        [
            "cargo",
            "+nightly-2026-03-03",
            "update",
            "--manifest-path",
            str(manifest),
            "-p",
            package,
            "--precise",
            version,
        ]
        for package, version in HISTORICAL_DEPENDENCY_PINS
    ]


def historical_cargo_commands(
    manifest: Path, *arguments: str, release: bool = False
) -> tuple[list[str], list[str]]:
    fetch = [
        "cargo",
        "+nightly-2026-03-03",
        "fetch",
        "--manifest-path",
        str(manifest),
    ]
    run = [
        "cargo",
        "+nightly-2026-03-03",
        "run",
        "--quiet",
        "--locked",
        "--offline",
    ]
    if release:
        run.append("--release")
    run.extend(["--manifest-path", str(manifest)])
    if arguments:
        run.append("--")
        run.extend(arguments)
    return fetch, run


def run_historical_d2_fixture(*arguments: str, release: bool = False) -> str:
    archive = subprocess.run(
        ["git", "archive", "--format=tar", D2_HEAD],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
    ).stdout
    with tempfile.TemporaryDirectory(prefix="mech-d2-historical-") as directory:
        checkout = Path(directory)
        with tarfile.open(fileobj=io.BytesIO(archive), mode="r:") as contents:
            contents.extractall(checkout, filter="data")
        environment = dict(os.environ)
        environment["CARGO_TARGET_DIR"] = str(ROOT / "target/d2-historical-evidence")
        manifest = checkout / "tests/fixtures/d2-contract-generator/Cargo.toml"
        fetch_command, command = historical_cargo_commands(
            manifest, *arguments, release=release
        )
        for pin_command in historical_dependency_pin_commands(manifest):
            pin = subprocess.run(
                pin_command,
                cwd=checkout,
                env=environment,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
            )
            if pin.returncode != 0:
                raise RuntimeError(
                    "historical D2 dependency lock materialization failed: "
                    + (pin.stdout + pin.stderr).strip()
                )
        fetch = subprocess.run(
            fetch_command,
            cwd=checkout,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if fetch.returncode != 0:
            raise RuntimeError(
                "historical D2 dependency fetch failed: "
                + (fetch.stdout + fetch.stderr).strip()
            )
        process = subprocess.run(
            command,
            cwd=checkout,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
        if process.returncode != 0:
            raise RuntimeError((process.stdout + process.stderr).strip())
        return process.stdout
