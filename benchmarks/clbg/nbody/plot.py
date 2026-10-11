#!/usr/bin/env python3
"""Render a measured n-body time/size plot and endpoint table."""
import argparse
import json
from pathlib import Path
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import FuncFormatter

COLORS = {"chapel": "#188754", "julia": "#9558b2", "rust": "#aa5724",
          "java": "#326994", "go": "#008b9b", "mech": "#e24b4b", "c": "#606770", "cpp": "#d79d16"}
LABELS = {"chapel": "Chapel", "julia": "Julia", "rust": "Rust", "java": "Java (OpenJDK JIT)",
          "go": "Go", "mech": "Mech v0.4 + adapter", "c": "C (GCC)", "cpp": "C++ (g++)"}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("results", type=Path)
    args = parser.parse_args()
    data = json.loads(args.results.read_text())
    mode = data["protocol"].get("mech_mode", "resident")
    if mode != "resident":
        LABELS["mech"] = {"aot": "Mech prototype (Cranelift AOT)", "aot-simd": "Mech prototype (Cranelift SIMD AOT)", "llvm-aot": "Mech prototype (LLVM AOT)", "jit": "Mech prototype (Cranelift JIT)", "jit-simd": "Mech prototype (Cranelift SIMD JIT)"}[mode]
    mech_only = data["protocol"].get("mech_size_scope", "full-program") == "mech-only"
    if mech_only:
        LABELS["mech"] = LABELS["mech"].replace(" + adapter", "") + " (.mec only)"
    native_flags = data["protocol"].get("mech_native_flags", [])
    if "--fp-contract" in native_flags:
        LABELS["mech"] += "; FMA"
    c_flags = data["protocol"].get("c_tuning_flags", [])
    compare_cc = data["protocol"].get("c_configurations", "").startswith("GCC and LLVM")
    if compare_cc:
        LABELS["c"] = "C (GCC + LLVM)"
        LABELS["cpp"] = "C++ (g++ + LLVM)"
    if c_flags:
        LABELS["c"] += "; tuned"
        LABELS["cpp"] += "; tuned"
    endpoints = data["endpoints"]
    fig, ax = plt.subplots(figsize=(11.5, 7.4), layout="constrained")
    ax.set_xscale("log", base=2)
    ax.set_yscale("log", base=2)
    ax.xaxis.set_major_formatter(FuncFormatter(lambda x, _: f"{x:g}×"))
    ax.yaxis.set_major_formatter(FuncFormatter(lambda x, _: f"{x:g}×"))
    ax.grid(which="both", color="#dfe3e8", linewidth=0.7)
    ax.set_axisbelow(True)
    for language, e in endpoints["languages"].items():
        color = COLORS[language]
        candidates = [r for r in data["programs"] if r["language"] == language]
        implementations = len({r.get("upstream_id", r["id"]) for r in candidates})
        cohort_label = (f"{implementations} sources, {len(candidates)} configurations"
                        if implementations != len(candidates) else f"{implementations} variants")
        ax.scatter([r["source"]["gzip1_bytes"] / endpoints["size_baseline_bytes"] for r in candidates],
                   [r["median_seconds"] / endpoints["time_baseline_seconds"] for r in candidates],
                   color=color, s=25, alpha=0.3)
        fast, small = e["points"]["fastest"], e["points"]["smallest"]
        ax.plot([fast["size_ratio"], small["size_ratio"]], [fast["time_ratio"], small["time_ratio"]],
                color=color, linewidth=1.5)
        if e["candidate_count"] == 1:
            ax.scatter(fast["size_ratio"], fast["time_ratio"], color=color, marker="D", s=90,
                       label=LABELS[language] + " (one implementation)")
        elif e["same_entry"]:
            ax.scatter(fast["size_ratio"], fast["time_ratio"], color=color, marker="o", s=80,
                       label=LABELS[language] + f" ({cohort_label}; coincident)")
        else:
            ax.scatter(fast["size_ratio"], fast["time_ratio"], color=color, marker="o", s=65,
                       label=LABELS[language] + f" ({cohort_label})")
            ax.scatter(small["size_ratio"], small["time_ratio"], color=color, marker="s", s=55)
    protocol = data["protocol"]
    title = "n-body: execution time and program size"
    ax.set_title(title + f"\n{data['hardware']['cpu']}, N = {protocol['steps']:,}, median of {protocol['samples']} runs",
                 loc="left", fontsize=15, pad=15)
    ax.set_xlabel("Normalized compressed program size — smaller ←", fontsize=11, labelpad=10)
    ax.set_ylabel("Normalized elapsed time — faster ↓", fontsize=11, labelpad=10)
    if max(r["source"]["gzip1_bytes"] / endpoints["size_baseline_bytes"] for r in data["programs"]) <= 2:
        ax.set_xticks([1, 1.25, 1.5, 1.75, 2])
    ax.legend(loc="upper right", fontsize=8.5, framealpha=0.95)
    size_caption = "Mech: complete .mec program; generic runtime excluded. Others: complete submissions." if mech_only else "Mech size includes its authored adapter. Selected local cohort."
    policy_caption = "Mech flags: " + (", ".join(flag.removeprefix("--") for flag in native_flags) or "strict arithmetic")
    if c_flags: policy_caption += ". C/C++: fast-math, unrolling, LTO."
    if compare_cc: policy_caption += ". C/C++: GCC/LLVM, native O3 and fast-math/unrolling/LTO."
    caption = ("Circle: fastest; square: smallest; diamond: only one implementation. Faint points: all candidates.\n"
               f"Fresh processes; includes startup/JIT. {size_caption}\n"
               f"{protocol['status']}; v0.4 integration {data['git_commit'][:10]}; {len(data['excluded'])} exclusions.\n{policy_caption}")
    fig.text(0.015, -0.025, caption, fontsize=9, va="top", color="#424b57")
    for extension in ("svg", "png"):
        fig.savefig(args.results.with_suffix("." + extension), dpi=170, bbox_inches="tight")
    by_id = {r["id"]: r for r in data["programs"]}
    rows = ["| Language | Fastest program | Time (s) | Lines | gzip bytes | Smallest program | Time (s) | gzip bytes | Smallest runtime / fastest |",
            "| --- | --- | ---: | ---: | ---: | --- | ---: | ---: | ---: |"]
    for language, e in endpoints["languages"].items():
        fast, small = by_id[e["fastest"]], by_id[e["smallest"]]
        rows.append(f"| {LABELS[language]} | {e['fastest']} | {fast['median_seconds']:.3f} | "
                    f"{fast['source']['nonblank_noncomment_lines']} | {fast['source']['gzip1_bytes']} | "
                    f"{e['smallest']} | {small['median_seconds']:.3f} | {small['source']['gzip1_bytes']} | "
                    f"{e['smallest_time_over_fastest']:.3f}× |")
    args.results.with_suffix(".table.md").write_text("\n".join(rows) + "\n")
    print(args.results.with_suffix(".svg"))

if __name__ == "__main__":
    main()
