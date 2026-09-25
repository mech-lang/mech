#!/usr/bin/env python3
"""Plot a complete, matched ten-process Mech backend campaign.

The source JSON is validated before matplotlib is imported. No archived or
fallback values are used. SVG labels remain editable text.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import statistics
from pathlib import Path


ROWS = (
    ("metal", "Metal GPU"),
    ("simd-jit-8w", "SIMD JIT 8w"),
    ("simd-aot", "SIMD AOT 1w"),
    ("scalar-jit", "Scalar JIT"),
    ("evaluator", "Evaluator"),
)
MODES = ("checked", "unchecked")
COLORS = {"checked": "#F6C04E", "unchecked": "#996900"}
BACKGROUND = "#0b1113"
FOREGROUND = "#e8edf0"
MUTED = "#afbec5"
GRID = "#314147"
INSTANCES = 500_000
TURNS = 40
SAMPLES = 10
WARMUP = 5
LIMITS = (0.1, 1000.0)
GUARDS = {"finite-candidate!", "positive-covariance!", "symmetric-covariance!"}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def positive_number(value: object, name: str) -> float:
    require(
        isinstance(value, (int, float)) and not isinstance(value, bool),
        f"{name} must be numeric",
    )
    require(math.isfinite(value) and value > 0, f"{name} must be finite and positive")
    return float(value)


def validate_campaign(document: dict) -> dict[tuple[str, str], tuple[float, float]]:
    """Recompute the plotted medians/MADs from all 100 validated raw samples."""
    require(document.get("schema_version") == 1, "unsupported campaign schema")
    require(document.get("status") == "complete", "campaign is not complete")
    require(not document.get("preflight_only"), "preflight-only evidence cannot be plotted")
    for key, expected in (
        ("instances", INSTANCES), ("turns", TURNS), ("samples_per_case", SAMPLES),
    ):
        require(document.get(key) == expected, f"{key} must be {expected}")

    expected_cases = {(backend, mode) for backend, _ in ROWS for mode in MODES}
    preflight = document.get("preflight_records", [])
    require(len(preflight) == len(expected_cases), "ten preflight records are required")
    preflight_cases = []
    for record in preflight:
        case = (record.get("backend"), record.get("mode"))
        preflight_cases.append(case)
        require(record.get("status") == "ok", f"failed preflight: {case}")
        validation = record.get("measurement", {}).get("validation", {})
        require(validation.get("passed") is True, f"unvalidated preflight: {case}")
        require(validation.get("instances") == 4092, f"preflight must test 4092 instances: {case}")
        require(validation.get("turns") == TURNS + WARMUP, f"preflight must test 45 turns: {case}")
        if case[1] == "checked":
            rollback = validation.get("nan_rollback", {})
            require(rollback.get("passed") is True, f"NaN rollback was not verified: {case}")
            require(rollback.get("all_state_unchanged") is True, f"rollback changed state: {case}")
            require(rollback.get("fault_lane") == 4091, f"wrong observed fault lane: {case}")
    require(
        len(set(preflight_cases)) == len(expected_cases)
        and set(preflight_cases) == expected_cases,
        "preflight cases are missing or duplicated",
    )

    records = document.get("records", [])
    require(len(records) == SAMPLES * len(expected_cases), "exactly 100 measured records are required")
    by_case = {case: [] for case in expected_cases}
    by_round = {index: {} for index in range(1, SAMPLES + 1)}
    for index, record in enumerate(records, 1):
        case = (record.get("backend"), record.get("mode"))
        require(case in expected_cases, f"unknown sample case {case}")
        require(record.get("status") == "ok", f"sample {index} failed")
        require(record.get("returncode") == 0, f"sample {index} process failed")
        round_index, position = record.get("round"), record.get("position")
        require(round_index in by_round, f"invalid round in sample {index}")
        require(isinstance(position, int) and 1 <= position <= 10, f"invalid schedule position in sample {index}")
        require(position not in by_round[round_index], f"duplicate schedule position in round {round_index}")
        by_round[round_index][position] = case

        measurement = record.get("measurement", {})
        expected_values = {
            "backend": case[0], "mode": case[1], "instances": INSTANCES,
            "turns": TURNS, "total_filter_turns": INSTANCES * TURNS,
            "attempted_turns": TURNS + WARMUP, "faults": 0,
            "warmup_turns": WARMUP, "warmup_in_measured_session": True,
            "measured_start_after_turn": WARMUP,
            "state_components": INSTANCES * 12,
            "publication": "one completed publication per turn",
        }
        for key, expected in expected_values.items():
            require(measurement.get(key) == expected, f"sample {index}: wrong {key}")
        elapsed = measurement.get("elapsed_ns")
        require(isinstance(elapsed, int) and not isinstance(elapsed, bool) and elapsed > 0,
                f"sample {index}: elapsed_ns must be a positive integer")
        throughput = positive_number(
            measurement.get("throughput_million_filter_turns_per_second"),
            f"sample {index} throughput",
        )
        require(
            math.isclose(throughput, INSTANCES * TURNS * 1000.0 / elapsed, rel_tol=1e-12),
            f"sample {index}: throughput disagrees with elapsed time",
        )
        checksum = measurement.get("checksum")
        require(isinstance(checksum, (int, float)) and math.isfinite(checksum),
                f"sample {index}: invalid checksum")
        removed = measurement.get("removed_source_guards")
        require(isinstance(removed, list), f"sample {index}: missing guard-removal metadata")
        require(set(removed) == (set() if case[1] == "checked" else GUARDS),
                f"sample {index}: wrong guard-removal policy")
        if case[0] == "simd-aot":
            library_hash = record.get("loaded_library", {}).get("sha256", "")
            require(len(library_hash) == 64, f"sample {index}: missing loaded AOT-library hash")
        by_case[case].append(throughput)

    schedule = document.get("schedule")
    require(isinstance(schedule, list) and len(schedule) == SAMPLES, "missing complete schedule")
    for round_index, positions in by_round.items():
        require(set(positions.values()) == expected_cases, f"round {round_index} omits or repeats a case")
        expected_order = [tuple(case) for case in schedule[round_index - 1]]
        require([positions[index] for index in range(1, 11)] == expected_order,
                f"round {round_index} differs from the recorded schedule")

    result = {}
    for case, samples in by_case.items():
        require(len(samples) == SAMPLES, f"{case} must have exactly ten samples")
        median = statistics.median(samples)
        mad = statistics.median(abs(value - median) for value in samples)
        require(LIMITS[0] < median <= LIMITS[1], f"{case} median is outside the fixed log axis")
        summary = document.get("summary", {}).get(case[0], {}).get(case[1], {})
        require(summary.get("n") == SAMPLES, f"{case}: stored summary has the wrong n")
        for key, expected in (
            ("median_million_filter_turns_per_second", median),
            ("mad_million_filter_turns_per_second", mad),
        ):
            value = summary.get(key)
            require(isinstance(value, (int, float)) and math.isclose(value, expected, rel_tol=1e-12, abs_tol=1e-15),
                    f"{case}: summary {key} differs from raw samples")
        result[case] = (median, mad)
    return result


def indicator(median: float, mad: float) -> str:
    digits = 3 if median < 10 or 0 < mad < 0.01 else 2
    spread = "<0.001" if 0 < mad < 0.001 else f"{mad:.{digits}f}"
    return f"{median:.{digits}f} ± {spread}"


def plot(
    values: dict, input_path: Path, output_prefix: Path, document: dict,
    font_dir: Path | None = None,
) -> tuple[Path, Path]:
    # Keep plotting imports out of validation-only checks and benchmark runs.
    import matplotlib
    matplotlib.use("Agg")
    from matplotlib import font_manager, pyplot as plt
    from matplotlib.patches import Patch
    from matplotlib.ticker import FixedLocator, FuncFormatter, NullLocator

    if font_dir is not None:
        for font_path in sorted(font_dir.glob("*.ttf")):
            font_manager.fontManager.addfont(str(font_path))
    available = {font.name for font in font_manager.fontManager.ttflist}
    heading_font = "Fira Code" if "Fira Code" in available else "DejaVu Sans Mono"
    matplotlib.rcParams.update({
        "figure.facecolor": BACKGROUND, "axes.facecolor": BACKGROUND,
        "savefig.facecolor": BACKGROUND, "text.color": FOREGROUND,
        "axes.labelcolor": MUTED, "xtick.color": MUTED, "ytick.color": FOREGROUND,
        "font.family": "DejaVu Sans", "font.size": 12,
        "svg.fonttype": "none", "svg.hashsalt": "mech-backend-pairs",
        "hatch.linewidth": 0.75,
    })
    figure = plt.figure(figsize=(11.8, 6.8))
    bars = figure.add_axes((0.19, 0.245, 0.565, 0.565))
    labels = figure.add_axes((0.785, 0.245, 0.19, 0.565), sharey=bars)
    bars.set_xscale("log")
    bars.set_xlim(*LIMITS)
    bars.set_ylim(len(ROWS) - 0.45, -0.55)
    bars.xaxis.set_major_locator(FixedLocator([0.1, 1, 10, 100, 1000]))
    bars.xaxis.set_major_formatter(FuncFormatter(lambda value, _: f"{value:g}"))
    bars.xaxis.set_minor_locator(NullLocator())
    bars.set_axisbelow(True)
    bars.grid(axis="x", color=GRID, linewidth=0.65)
    bars.tick_params(axis="both", which="both", length=0, pad=9)
    bars.set_yticks(range(len(ROWS)), [label for _, label in ROWS])
    bars.set_xlabel("M filter-turns/s (logarithmic)", labelpad=12)
    for spine in bars.spines.values():
        spine.set_visible(False)
    labels.set_xlim(0, 1)
    labels.axis("off")

    for row, (backend, _) in enumerate(ROWS):
        for mode, offset in (("checked", 0.155), ("unchecked", -0.155)):
            median, mad = values[(backend, mode)]
            rectangles = bars.barh(
                row + offset, median - LIMITS[0], left=LIMITS[0], height=0.275,
                color=COLORS[mode],
                edgecolor=BACKGROUND if backend == "metal" else "none",
                linewidth=0.3, hatch="////" if backend == "metal" else None,
            )
            rectangles[0].set_gid(f"bar-{backend}-{mode}")
            label = labels.text(
                0.99, row + offset, indicator(median, mad),
                ha="right", va="center", fontsize=12,
            )
            label.set_gid(f"value-{backend}-{mode}")

    figure.text(0.045, 0.94, "Mech backends", fontsize=22, fontweight="bold", fontfamily=heading_font)
    figure.text(0.045, 0.885, "Same EKF source; checked and unchecked execution", color=MUTED, fontsize=12)
    figure.text(0.973, 0.835, "median ± MAD", ha="right", fontsize=11, color=MUTED, fontfamily=heading_font)
    figure.legend(
        handles=[Patch(facecolor=COLORS[mode], label=mode) for mode in MODES],
        loc="lower left", bbox_to_anchor=(0.035, 0.116), frameon=False, ncol=2,
        handlelength=1.8, handleheight=1.0, columnspacing=2.5,
    )
    figure.text(
        0.045, 0.079,
        "500,000 filters · 5 warmup turns + 40 timed turns",
        fontsize=10, color=MUTED,
    )
    figure.text(
        0.045, 0.041,
        "Resident state; publication after each turn. ± denotes median absolute deviation. 1w / 8w: one / eight CPU workers.",
        fontsize=9, color=MUTED,
    )
    svg = Path(str(output_prefix) + ".svg")
    png = Path(str(output_prefix) + ".png")
    description = (
        "Paired checked and unchecked Mech EKF backend throughput, logarithmic axis. "
        "Every value is the median of ten raw fresh-process samples; ± is unscaled MAD, not a confidence interval. "
        "GPU bars have parallel diagonal hatching. Numerical labels are editable SVG text. "
        f"Evidence: {input_path.name}; SHA-256 {hashlib.sha256(input_path.read_bytes()).hexdigest()}; "
        f"binary SHA-256 {document.get('provenance', {}).get('binary_sha256', 'not supplied')}."
    )
    figure.savefig(svg, metadata={"Title": "Mech backend comparison", "Description": description, "Date": None})
    figure.savefig(png, dpi=200, metadata={"Title": "Mech backend comparison", "Description": description})
    plt.close(figure)
    return svg, png


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output-prefix", type=Path, required=True)
    parser.add_argument("--font-dir", type=Path, help="load local .ttf fonts without installing them")
    arguments = parser.parse_args()
    input_path = arguments.input.resolve()
    output_prefix = arguments.output_prefix.resolve()
    font_dir = arguments.font_dir.resolve() if arguments.font_dir is not None else None
    if font_dir is not None and not font_dir.is_dir():
        parser.error(f"font directory not found: {font_dir}")
    for suffix in (".svg", ".png"):
        target = Path(str(output_prefix) + suffix)
        if target.exists():
            parser.error(f"refusing to overwrite {target}")
    try:
        document = json.loads(input_path.read_text(encoding="utf-8"))
        require(isinstance(document, dict), "campaign JSON must be an object")
        values = validate_campaign(document)
    except (OSError, ValueError, TypeError, KeyError) as error:
        parser.error(str(error))
    output_prefix.parent.mkdir(parents=True, exist_ok=True)
    for path in plot(values, input_path, output_prefix, document, font_dir):
        print(path)


if __name__ == "__main__":
    main()
