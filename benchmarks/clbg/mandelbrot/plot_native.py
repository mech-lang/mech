#!/usr/bin/env python3
"""Plot native compiler A/B timings without implying a full language cohort."""
import argparse
import json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    args = parser.parse_args()
    data = json.loads(args.report.read_text())
    by_id = {p["id"]: p for p in data["programs"]}
    ids = ["mech-unrolled-simd-loop-fma", "mech-unrolled-simd-loop-fma-boolean-masks",
           "mech-unrolled-simd-loop-fma-packed-bits", "mech-unrolled-simd-loop-fma-add-packed-bits", "mandelbrot-rust-4"]
    labels = ["Mech: SIMD + five-step unrolling + FMA", "Mech: + native Boolean masks",
              "Mech: + packed Boolean return", "Mech: + doubling by addition", "Rust #4 (unmodified)"]
    rows = [by_id[i] for i in ids]
    times = [p["median_seconds"] for p in rows]
    errors = [[p["median_seconds"] - p["min_seconds"] for p in rows],
              [p["max_seconds"] - p["median_seconds"] for p in rows]]
    fig, ax = plt.subplots(figsize=(11, 5.4), layout="constrained")
    y = list(range(len(rows)))
    ax.barh(y, times, color=["#e24b4b"] * 4 + ["#aa5724"], height=0.55, xerr=errors,
            error_kw={"ecolor": "#30363d", "capsize": 4, "linewidth": 1})
    ax.set_yticks(y, labels)
    ax.invert_yaxis()
    ax.set_xlim(0, max(p["max_seconds"] for p in rows) * 1.19)
    for i, p in enumerate(rows):
        ax.text(p["max_seconds"] + 0.015, i, f"{p['median_seconds']:.3f} s", va="center", fontsize=10)
    ax.set_xlabel("Elapsed seconds (complete process and PBM output)")
    ax.set_title(f"Mandelbrot: closing the gap to Rust\nApple M3; N={data['protocol']['size']:,}; {data['protocol']['samples']} shuffled fresh runs", loc="left", pad=15)
    ax.grid(axis="x", alpha=0.2)
    ax.set_axisbelow(True)
    fig.text(0.015, -0.04,
             "Bars: medians; whiskers: observed minimum–maximum. Eight workers for both languages.\n"
             "All complete N=16000 images are byte-identical. Mech remains 26 source lines; compiler/runtime excluded.\n"
             "Selected Rust/Mech A/B study; this is not the complete fastest/smallest language cohort.", fontsize=9, va="top")
    for ext in ("svg", "png"):
        fig.savefig(args.report.with_suffix(".native." + ext), dpi=170, bbox_inches="tight")
    plt.close(fig)


if __name__ == "__main__": main()
