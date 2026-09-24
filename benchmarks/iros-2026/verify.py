#!/usr/bin/env python3
"""Verify the frozen IROS EKF evidence and representative source set."""

from __future__ import annotations

import hashlib
import json
import statistics
import subprocess
import xml.etree.ElementTree as ET
from pathlib import Path


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
ARCHIVE = ROOT / "benchmarks/archive/compute/parallel-ekf"


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def chart_values(path: Path) -> dict[str, float]:
    labels: dict[str, str] = {}
    values: dict[str, float] = {}
    for node in ET.parse(path).getroot().iter():
        if not node.tag.endswith("text") or node.text is None:
            continue
        y = node.attrib.get("y")
        css_class = node.attrib.get("class")
        if y is not None and css_class == "label":
            labels[y] = node.text
        elif y is not None and css_class == "value":
            values[y] = float(node.text)
    return {label: values[y] for y, label in labels.items() if y in values}


def close(actual: float, expected: float, tolerance: float = 0.005) -> None:
    if abs(actual - expected) > tolerance:
        raise AssertionError(f"expected {expected}, found {actual}")


def main() -> None:
    manifest = load_json(HERE / "manifest.json")

    for source in manifest["representative_sources"]:
        path = ROOT / source["path"]
        if not path.is_file():
            raise AssertionError(f"missing representative source: {path}")
        actual = sha256(path)
        if actual != source["sha256"]:
            raise AssertionError(f"source hash changed for {source['path']}: {actual}")

    counts = load_json(ARCHIVE / "source-size-audit/counts.json")
    matched = manifest["matched_checked_comparison"]
    assert counts["ekf.mec"]["normalized_characters"] == matched["mech_normalized_source_characters"]
    assert counts["rust_simd.rs"]["normalized_characters"] == matched["rust_normalized_source_characters"]

    fused = load_json(ARCHIVE / "results/apple-m1-fused-reference-controls-2026-08-31.json")
    mech = statistics.median(fused["rows"]["mech_fused_checked"]["throughput_millions"])
    rust = statistics.median(fused["rows"]["rust_fused_checked"]["throughput_millions"])
    close(mech, matched["mech_million_turns_per_second"], 0.0005)
    close(rust, matched["rust_million_turns_per_second"], 0.0005)

    charts = {
        "checked": ARCHIVE / "charts/parallel-ekf-cross-language-checked.svg",
        "unchecked": ARCHIVE / "charts/parallel-ekf-cross-language-unchecked.svg",
    }
    for mode, path in charts.items():
        rows = chart_values(path)
        for label, expected in manifest["chart_assertions"][mode].items():
            if label not in rows:
                raise AssertionError(f"missing {mode} chart row: {label}")
            close(rows[label], expected)

    result = subprocess.run(
        ["git", "merge-base", "--is-ancestor", manifest["integration_base"], "HEAD"],
        cwd=ROOT,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError("current branch is not based on the recorded v0.4 integration head")

    print("IROS EKF evidence verified")
    print(f"  matched checked throughput: Mech {mech:.3f} vs Rust {rust:.3f} M turns/s")
    print(
        "  normalized application source: "
        f"Mech {matched['mech_normalized_source_characters']:,} vs "
        f"Rust {matched['rust_normalized_source_characters']:,} characters"
    )
    print(f"  representative source hashes: {len(manifest['representative_sources'])}")
    print("  checked and unchecked mega-chart assertions: passed")


if __name__ == "__main__":
    main()
