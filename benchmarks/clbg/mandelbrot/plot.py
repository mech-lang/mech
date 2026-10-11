#!/usr/bin/env python3
"""Render Mandelbrot speed/size endpoints, with explicit cohort filters."""
import argparse
import json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter

COLORS = {"chapel": "#188754", "julia": "#9558b2", "rust": "#aa5724", "java": "#326994",
          "go": "#008b9b", "mech": "#e24b4b", "c": "#606770", "cpp": "#c39111"}
LABELS = {"chapel": "Chapel", "julia": "Julia", "rust": "Rust", "java": "Java (OpenJDK JIT)",
          "go": "Go", "mech": "Mech (Cranelift AOT; kernel size only)", "c": "C (GCC + LLVM)", "cpp": "C++ (GCC + LLVM)"}


def render(path, data, key, suffix, description):
    revised = data.get("evaluation_status", {}).get("kind") == "complete_source_program"
    labels = LABELS | ({"mech": "Mech (Cranelift AOT; complete source)"} if revised else {})
    combined = "measurement_batches" in data
    endpoints = data[key]
    by_id = {r["id"]: r for r in data["programs"]}
    candidates = data["programs"]
    if key == "nonheroic_endpoints": candidates = [r for r in candidates if not r["upstream_starred"]]
    if key == "exact_image_endpoints": candidates = [r for r in candidates if r["warmup_image"]["exact_reference_match"]]
    fig, ax = plt.subplots(figsize=(11.5, 7.6), layout="constrained")
    ax.set_xscale("log", base=2)
    ax.set_yscale("log", base=2)
    ax.xaxis.set_major_formatter(FuncFormatter(lambda x, _: f"{x:g}×"))
    ax.yaxis.set_major_formatter(FuncFormatter(lambda x, _: f"{x:g}×"))
    ax.grid(which="both", color="#dfe3e8", linewidth=0.7)
    ax.set_axisbelow(True)
    time_label = "Time / same-batch Rust #4" if combined else "Seconds"
    table = [f"| Language | Fastest | {time_label} | LOC | gzip bytes | Smallest | {time_label} | LOC | gzip bytes | Time tradeoff |",
             "| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |"]
    for language, endpoint in endpoints["languages"].items():
        rows = [r for r in candidates if r["language"] == language]
        source_count = len({r["path"] for r in rows})
        color = COLORS[language]
        ax.scatter([r["source"]["gzip1_bytes"] / endpoints["size_baseline_bytes"] for r in rows],
                   [r["comparison_time_ratio"] if combined else r["median_seconds"] / endpoints["time_baseline_seconds"] for r in rows],
                   color=color, s=22, alpha=0.25)
        fast, small = endpoint["points"]["fastest"], endpoint["points"]["smallest"]
        ax.plot([fast["size_ratio"], small["size_ratio"]], [fast["time_ratio"], small["time_ratio"]], color=color, linewidth=1.6)
        ax.scatter(fast["size_ratio"], fast["time_ratio"], color=color, s=65,
                   label=f"{labels[language]} ({source_count} sources, {len(rows)} builds)")
        if not endpoint["same_entry"]: ax.scatter(small["size_ratio"], small["time_ratio"], color=color, marker="s", s=55)
        f, s = by_id[endpoint["fastest"]], by_id[endpoint["smallest"]]
        ft, st = (f["comparison_time_ratio"], s["comparison_time_ratio"]) if combined else (f["median_seconds"], s["median_seconds"])
        table.append(f"| {labels[language]} | {f['id']} | {ft:.3f} | {f['source']['nonblank_noncomment_lines']} | {f['source']['gzip1_bytes']} | {s['id']} | {st:.3f} | {s['source']['nonblank_noncomment_lines']} | {s['source']['gzip1_bytes']} | {endpoint['smallest_time_over_fastest']:.2f}× |")
    if combined and "historical_mech_endpoints" in data and key != "nonheroic_endpoints":
        old = data["historical_mech_endpoints"]["points"]
        ax.plot([old[k]["size_ratio"] for k in ("fastest", "smallest")],
                [old[k]["time_ratio"] for k in ("fastest", "smallest")],
                color=COLORS["mech"], linestyle="--", alpha=0.45, linewidth=1.5,
                label="Previous Mech endpoints (baseline)")
        fast = endpoints["languages"]["mech"]["points"]["fastest"]
        small = endpoints["languages"]["mech"]["points"]["smallest"]
        ax.annotate(f"Improved Mech\n26 LOC · 338 gzip bytes\n{fast['time_ratio']:.3f}× Rust #4", (fast["size_ratio"], fast["time_ratio"]),
                    xytext=(18, 18), textcoords="offset points", fontsize=9, color=COLORS["mech"], va="bottom",
                    arrowprops={"arrowstyle": "-", "color": COLORS["mech"], "alpha": 0.6, "linewidth": 0.8})
        ax.annotate("Smallest Mech\n17 LOC · 197 gzip bytes", (small["size_ratio"], small["time_ratio"]),
                    xytext=(12, 17), textcoords="offset points", fontsize=9, color=COLORS["mech"],
                    arrowprops={"arrowstyle": "-", "color": COLORS["mech"], "alpha": 0.6, "linewidth": 0.8})
    p = data["protocol"]
    status = p.get("status", "")
    selected = "Selected configurations. " if "selected" in status.lower() or "controlled" in status.lower() else ""
    sample_label = p.get("samples_label") or f"median of {p['samples']} fresh runs"
    title = "Mandelbrot: execution time and complete source size" if revised else "Mandelbrot: execution time and source size (Mech kernel only)"
    ax.set_title(f"{title}\n{data['hardware']['cpu']}; N={p['size']:,}; {sample_label}", loc="left", fontsize=15, pad=15)
    ax.set_xlabel("Normalized compressed program size — smaller ←", fontsize=11)
    ax.set_ylabel("Time relative to same-batch Rust #4 — faster ↓" if combined else "Normalized elapsed time — faster ↓", fontsize=11)
    ax.legend(loc="upper right", fontsize=8.5, framealpha=0.95)
    caption = (
        "Circle: fastest; square: smallest; faint points: all builds in this cohort.\n"
        f"{selected}{description} Requested workers: {p['requested_threads']}; source policies are recorded in JSON.\n"
        + ("Complete Mech source defines the kernel, traversal operation, arguments and PBM output; generic library runtime excluded.\n"
           "New opt-in Cranelift build pathway. Complete output and startup/JIT timed; AOT build excluded.\n" if revised else
           "Complete PBM output and startup/JIT included; AOT build excluded. Mech size excludes host grid traversal and output composition.\n"
           "Mech kernel sizes do not establish a fair complete-program size advantage.\n")
    )
    if combined:
        caption += p["chart_note"] + "\n"
        caption += f"All full N=16000 images are byte-identical. {len(data['excluded'])} excluded baseline configurations; see report."
    else:
        caption += f"All pass the official N=200 byte comparison. {len(data['excluded'])} excluded configurations; see report."
    fig.text(0.015, -0.025, caption, fontsize=9, va="top", color="#424b57")
    stem = path.with_suffix("")
    target = stem.parent / (stem.name + suffix)
    for ext in ("svg", "png"): fig.savefig(str(target) + "." + ext, dpi=170, bbox_inches="tight")
    plt.close(fig)
    Path(str(target) + ".table.md").write_text("\n".join(table) + "\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("results", type=Path)
    parser.add_argument("--selected", action="store_true", help="label a selected subset explicitly")
    args = parser.parse_args()
    data = json.loads(args.results.read_text())
    prefix = "Selected configurations. " if args.selected else ""
    render(args.results, data, "endpoints", "", prefix + "All locally validated sources in this cohort, including starred submissions.")
    render(args.results, data, "nonheroic_endpoints", ".nonstarred", prefix + "Upstream starred submissions excluded.")
    render(args.results, data, "exact_image_endpoints", ".exact", prefix + "Only byte-identical full images relative to local Rust #4.")
    print(args.results.with_suffix(".svg"))


if __name__ == "__main__": main()
