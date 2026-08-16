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
        command = [
            "cargo",
            "+nightly-2026-03-03",
            "run",
            "--quiet",
            "--offline",
        ]
        if release:
            command.append("--release")
        command.extend(
            [
                "--manifest-path",
                str(checkout / "tests/fixtures/d2-contract-generator/Cargo.toml"),
            ]
        )
        if arguments:
            command.append("--")
            command.extend(arguments)
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
