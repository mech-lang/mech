#!/usr/bin/env python3
"""Replay an immutable historical Gate B implementation on the F0 machine."""

from __future__ import annotations

import json
import os
import subprocess
import tempfile
from pathlib import Path

from f0_evidence import (
    ROOT,
    attach_provenance,
    copy_criterion_evidence,
    sha256_file,
)


def replay_historical_gate_b(
    commit: str,
    context: dict,
    output_directory: Path,
    runner_python: Path,
    numpy_python: Path,
    sample_size: int = 10,
    warm_up_time: float = 1.0,
    measurement_time: float = 3.0,
) -> dict:
    output_directory.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="mech-f0-historical-gate-b-") as directory:
        checkout = Path(directory) / "checkout"
        subprocess.run(
            ["git", "clone", "--shared", "--no-checkout", str(ROOT), str(checkout)],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        branch = f"qualification/f0-historical-{commit[:12]}"
        subprocess.run(
            ["git", "switch", "-c", branch, commit],
            cwd=checkout,
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        report_path = output_directory / "report.json"
        raw_path = output_directory / "criterion.log"
        structural_path = output_directory / "structural.log"
        runner_log_path = output_directory / "runner.log"
        environment = dict(os.environ)
        target_directory = ROOT / "target/f0-historical-gate-b" / commit
        environment["CARGO_TARGET_DIR"] = str(target_directory)
        process = subprocess.run(
            [
                str(runner_python),
                str(checkout / "scripts/run-gate-b-benchmarks.py"),
                "--phase",
                "B2-resident-turn",
                "--sample-size",
                str(sample_size),
                "--warm-up-time",
                str(warm_up_time),
                "--measurement-time",
                str(measurement_time),
                "--python",
                str(numpy_python),
                "--output",
                str(report_path),
                "--raw-output",
                str(raw_path),
                "--raw-structural-output",
                str(structural_path),
            ],
            cwd=checkout,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        runner_log_path.write_text(process.stdout, encoding="utf-8")
        if process.returncode:
            raise RuntimeError(
                f"historical Gate B replay {commit} failed:\n{process.stdout}"
            )
        report = json.loads(report_path.read_text(encoding="utf-8"))
        logical_root = context.get("raw_evidence_prefix")
        logical_root = (
            f"{logical_root}/historical/{output_directory.name}"
            if logical_root
            else str(output_directory)
        )
        criterion_reference = copy_criterion_evidence(
            target_directory / "criterion",
            output_directory / "criterion-samples",
            f"{logical_root}/criterion-samples",
        )
        replay = {
            "implementation_commit": commit,
            "implementation_tree": subprocess.check_output(
                ["git", "rev-parse", f"{commit}^{{tree}}"], cwd=ROOT, text=True
            ).strip(),
            "raw_output_path": f"{logical_root}/criterion.log",
            "raw_output_sha256": sha256_file(raw_path),
            "raw_structural_output_path": f"{logical_root}/structural.log",
            "raw_structural_output_sha256": sha256_file(structural_path),
            "runner_log_path": f"{logical_root}/runner.log",
            "runner_log_sha256": sha256_file(runner_log_path),
            "criterion_samples": criterion_reference,
        }
        attach_provenance(report, context)
        report_path.write_text(
            json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )
        replay.update(
            {
                "report_path": f"{logical_root}/report.json",
                "report_sha256": sha256_file(report_path),
            }
        )
        report["historical_replay"] = replay
        return report
