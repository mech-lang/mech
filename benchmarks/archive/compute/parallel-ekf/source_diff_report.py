#!/usr/bin/env python3
"""Measure source edits behind the parallel-EKF benchmark variants."""

from __future__ import annotations

import argparse
import difflib
import html
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
BASE_MECH = ROOT / "hosts/gpu/fixtures/ekf-kernel.mec"
MECH_SUPPORT_BASE = "1ca47cdeef1cef071a891babf5423af03f51466f"
MECH_SUPPORT_FILES = (
    "Cargo.lock",
    "hosts/gpu/Cargo.toml",
    "hosts/gpu/examples/parallel_ekf_benchmark.rs",
    "hosts/gpu/src/batched/mod.rs",
    "hosts/gpu/src/metal.rs",
)

COLORS = {
    "Mech": "#f4c430",
    "Rust": "#dea584",
    "Python": "#3776ab",
    "Python + NumPy (scalar)": "#3776ab",
    "NumPy": "#4d77cf",
    "Julia": "#9558b2",
    "Lua": "#000080",
    "LuaJIT": "#5ba37f",
    "Taichi": "#e36b6b",
}


VARIANTS = [
    {
        "language": "Mech",
        "baseline": "hosts/gpu/fixtures/ekf-kernel.mec",
        "advanced": "hosts/gpu/fixtures/ekf-kernel.mec",
        "baseline_label": "same high-level `.mec` program",
        "advanced_label": "same `.mec`; native backend selected at build",
        "note": "The source recurrence does not change. Native Metal specialization is backend support, not a second Mech program.",
    },
    {
        "language": "Rust",
        "baseline": "hosts/gpu/examples/parallel_ekf_rust_scalar.rs",
        "advanced": "hosts/gpu/examples/parallel_ekf_rust_simd.rs",
        "baseline_label": "fixed-shape scalar control",
        "advanced_label": "packed four-lane SIMD control",
        "note": "The advanced control changes the value representation and execution loop.",
    },
    {
        "language": "Python + NumPy (scalar)",
        "baseline": "benchmarks/archive/compute/parallel-ekf/numpy_scalar.py",
        "advanced": "benchmarks/archive/compute/parallel-ekf/numpy_scalar.py",
        "baseline_label": "Python outer loop",
        "advanced_label": "same source; interpreter/runtime only",
        "note": "This is not pure Python: the outer loop is Python, while NumPy supplies the scalar matrix operations. NumPy vectorization is reported as its own row.",
    },
    {
        "language": "NumPy",
        "baseline": "benchmarks/archive/compute/parallel-ekf/numpy_scalar.py",
        "advanced": "benchmarks/archive/compute/parallel-ekf/numpy_vectorized.py",
        "baseline_label": "per-filter scalar loop",
        "advanced_label": "batched fixed-shape vectorized operations",
        "note": "This is a whole-program rewrite around NumPy array operations.",
    },
    {
        "language": "Julia",
        "baseline": "benchmarks/archive/compute/parallel-ekf/julia_scalar.jl",
        "advanced": "benchmarks/archive/compute/parallel-ekf/julia_simd_intrinsics.jl",
        "baseline_label": "generic scalar Julia",
        "advanced_label": "explicit four-lane SIMD.jl intrinsics",
        "note": "The advanced source introduces an explicit packed value type and lane loop.",
    },
    {
        "language": "LuaJIT",
        "baseline": "benchmarks/archive/compute/parallel-ekf/luajit_scalar.lua",
        "advanced": "benchmarks/archive/compute/parallel-ekf/luajit_fast.lua",
        "baseline_label": "generic matrix helper loop",
        "advanced_label": "flat fixed-shape scalarized state",
        "note": "The advanced source removes helper-level matrix temporaries and writes each component directly.",
    },
    {
        "language": "Lua",
        "baseline": "benchmarks/archive/compute/parallel-ekf/luajit_fast.lua",
        "advanced": "benchmarks/archive/compute/parallel-ekf/luajit_fast.lua",
        "baseline_label": "same flat source under PUC Lua",
        "advanced_label": "same flat source under PUC Lua",
        "note": "The Lua comparison isolates the runtime: the source is identical to the LuaJIT flat control.",
    },
    {
        "language": "Taichi",
        "baseline": "benchmarks/archive/compute/parallel-ekf/taichi_comparable.py",
        "advanced": "benchmarks/archive/compute/parallel-ekf/taichi_optimized.py",
        "baseline_label": "Vector/Matrix resident fields",
        "advanced_label": "scalar SoA fields and unrolled 3x3 arithmetic",
        "note": "This is the source-specialized Taichi control; it still uses stock Taichi 1.7.4 and per-turn sync.",
    },
]


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def mech_support_delta() -> dict[str, object]:
    """Count the backend implementation delta separately from `.mec` edits."""
    command = ["git", "diff", "--numstat", MECH_SUPPORT_BASE, "HEAD", "--", *MECH_SUPPORT_FILES]
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=False)
    additions = deletions = 0
    files = []
    if result.returncode == 0:
        for line in result.stdout.splitlines():
            fields = line.split("\t", 2)
            if len(fields) != 3:
                continue
            added, deleted, path = fields
            if added.isdigit() and deleted.isdigit():
                additions += int(added)
                deletions += int(deleted)
                files.append(path)
    return {
        "base_commit": MECH_SUPPORT_BASE,
        "head_commit": "HEAD",
        "files": files,
        "added_lines": additions,
        "deleted_lines": deletions,
        "changed_line_slots": additions + deletions,
        "note": "This is backend/compiler support for native Metal and the direct benchmark path; generated WGSL/MSL is an artifact, not a second user program.",
    }


def diff_metrics(old: str, new: str) -> dict[str, int]:
    old_lines = old.splitlines(keepends=True)
    new_lines = new.splitlines(keepends=True)
    line_slots = added_lines = deleted_lines = 0
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(
        None, old_lines, new_lines, autojunk=False
    ).get_opcodes():
        if tag != "equal":
            line_slots += max(i2 - i1, j2 - j1)
            deleted_lines += i2 - i1
            added_lines += j2 - j1
    # Character-level SequenceMatcher becomes quadratic for the larger Rust
    # and Taichi controls. Count character slots in the changed line blocks
    # instead; this is deterministic and measures the edit surface directly.
    changed_chars = 0
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(
        None, old_lines, new_lines, autojunk=False
    ).get_opcodes():
        if tag != "equal":
            changed_chars += max(
                sum(len(line) for line in old_lines[i1:i2]),
                sum(len(line) for line in new_lines[j1:j2]),
            )
    return {
        "changed_line_slots": line_slots,
        "added_lines": added_lines,
        "deleted_lines": deleted_lines,
        "changed_chars": changed_chars,
    }


def scalar_throughput(cross: dict, label: str) -> dict[str, float | None]:
    row = cross.get("summary", {}).get("scalar_outer_loop", {}).get(label)
    if row is None:
        return {"checked": None, "unchecked": None}
    # The scalar summary stores one mode per label. The caller maps explicit
    # checked/unchecked labels where both modes exist.
    mode = "checked" if label.endswith(" checked") else "unchecked"
    return {"checked": row["ekf_turns_per_second"] / 1e6 if mode == "checked" else None,
            "unchecked": row["ekf_turns_per_second"] / 1e6 if mode == "unchecked" else None}


def throughput_rows(cross: dict, native: dict, taichi: dict, lua: dict) -> dict[str, dict[str, float | None]]:
    scalar = cross["summary"]["scalar_outer_loop"]
    native_rows = {row["label"]: row for row in native["rows"]}
    result: dict[str, dict[str, float | None]] = {
        "Mech": {"checked": native_rows["Mech native Metal, checked"]["throughput_millions"], "unchecked": native_rows["Mech native Metal, unchecked"]["throughput_millions"]},
        "Rust": {"checked": scalar["Rust packed SIMD checked"]["ekf_turns_per_second"] / 1e6, "unchecked": scalar["Rust packed SIMD unchecked"]["ekf_turns_per_second"] / 1e6},
        "Python + NumPy (scalar)": {"checked": None, "unchecked": scalar["NumPy scalar outer loop"]["ekf_turns_per_second"] / 1e6},
        "NumPy": {"checked": scalar["NumPy vectorized fixed-shape checked"]["ekf_turns_per_second"] / 1e6, "unchecked": scalar["NumPy vectorized fixed-shape unchecked"]["ekf_turns_per_second"] / 1e6},
        "Julia": {"checked": scalar["Julia SIMD.jl intrinsics checked"]["ekf_turns_per_second"] / 1e6, "unchecked": scalar["Julia SIMD.jl intrinsics unchecked"]["ekf_turns_per_second"] / 1e6},
        "LuaJIT": {"checked": scalar["LuaJIT fixed-shape flat checked"]["ekf_turns_per_second"] / 1e6, "unchecked": scalar["LuaJIT fixed-shape flat unchecked"]["ekf_turns_per_second"] / 1e6},
        "Lua": {"checked": lua["rows"][0]["throughput_millions"], "unchecked": lua["rows"][1]["throughput_millions"]},
        "Taichi": {"checked": taichi["rows"][0]["throughput_millions"], "unchecked": taichi["rows"][1]["throughput_millions"]},
    }
    return result


def build_report(cross: dict, native: dict, taichi: dict, lua: dict) -> dict:
    base = read(BASE_MECH)
    throughputs = throughput_rows(cross, native, taichi, lua)
    rows = []
    for variant in VARIANTS:
        baseline_path = ROOT / variant["baseline"]
        advanced_path = ROOT / variant["advanced"]
        baseline = read(baseline_path)
        advanced = read(advanced_path)
        rows.append(
            {
                **variant,
                "baseline_lines": len(baseline.splitlines()),
                "baseline_chars": len(baseline),
                "advanced_lines": len(advanced.splitlines()),
                "advanced_chars": len(advanced),
                "baseline_to_advanced": diff_metrics(baseline, advanced),
                "baseline_to_base_mech": diff_metrics(base, baseline),
                "advanced_to_base_mech": diff_metrics(base, advanced),
                "throughput_millions": throughputs[variant["language"]],
            }
        )
    return {
        "schema_version": 1,
        "base_mech": str(BASE_MECH.relative_to(ROOT)),
        "definition": "Changed line slots count the larger side of each non-equal diff block; changed characters count the larger character span within those changed line blocks. This is an edit-size measure, not a claim about semantic difficulty.",
        "mech_backend_support_delta": mech_support_delta(),
        "rows": rows,
    }


def markdown(report: dict) -> str:
    lines = [
        "# Parallel EKF source-edit cost",
        "",
        "This report measures source edits behind the benchmark variants. `Changed lines` is the number of line positions touched by a baseline-to-advanced diff; `changed chars` counts character slots in those changed line blocks. File size is included only for context. The base reference is `hosts/gpu/fixtures/ekf-kernel.mec`.",
        "",
        "## Variant matrix",
        "",
        "| Language | Baseline source | Baseline lines | Baseline chars | Advanced source | Advanced lines | Advanced chars | Changed lines | Changed chars | Baseline vs Mech lines/chars | Advanced vs Mech lines/chars | Checked M/s | Unchecked M/s |",
        "| --- | --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for row in report["rows"]:
        throughput = row["throughput_millions"]
        checked = "--" if throughput["checked"] is None else f"{throughput['checked']:.3f}"
        unchecked = "--" if throughput["unchecked"] is None else f"{throughput['unchecked']:.3f}"
        lines.append(
            f"| {row['language']} | `{Path(row['baseline']).name}` | {row['baseline_lines']} | {row['baseline_chars']:,} | `{Path(row['advanced']).name}` | {row['advanced_lines']} | {row['advanced_chars']:,} | {row['baseline_to_advanced']['changed_line_slots']} | {row['baseline_to_advanced']['changed_chars']:,} | {row['baseline_to_base_mech']['changed_line_slots']} / {row['baseline_to_base_mech']['changed_chars']:,} | {row['advanced_to_base_mech']['changed_line_slots']} / {row['advanced_to_base_mech']['changed_chars']:,} | {checked} | {unchecked} |"
        )
    lines += ["", "## Interpretation", ""]
    for row in report["rows"]:
        edit = row["baseline_to_advanced"]
        lines.append(f"- **{row['language']}**: {row['note']} Baseline -> advanced touches **{edit['changed_line_slots']} lines / {edit['changed_chars']} characters**.")
    support = report["mech_backend_support_delta"]
    lines += ["", f"## Mech backend support footprint", "", f"The high-level Mech source delta is zero, but the native-Metal backend support changed **{support['changed_line_slots']} line slots** ({support['added_lines']} added / {support['deleted_lines']} deleted) across the backend files in the report JSON. This is intentionally reported separately: generated WGSL/MSL is a build artifact, not a second user program.", "", "The Mech row deliberately reports zero high-level source edits: the same `.mec` recurrence feeds the scalar, SIMD, JIT, WGPU, and native-Metal backends. Conversely, Taichi, Julia, Rust, and LuaJIT advanced rows include their source-level layout or execution changes.", ""]
    return "\n".join(lines)


def svg(report: dict) -> str:
    rows = sorted(report["rows"], key=lambda row: row["baseline_to_advanced"]["changed_line_slots"])
    width, left, right, top, row_height, bottom = 1700, 240, 140, 120, 40, 90
    chart_width = width - left - right
    max_lines = max(1, max(row["baseline_to_advanced"]["changed_line_slots"] for row in rows))
    max_chars = max(1, max(row["baseline_to_advanced"]["changed_chars"] for row in rows))
    panel_width = (chart_width - 80) / 2
    height = top + row_height * len(rows) + bottom

    def esc(value: object) -> str:
        return html.escape(str(value), quote=True)

    lines = [
        f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">',
        '<rect width="100%" height="100%" fill="#080c14"/>',
        '<style>text{font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;fill:#e8edf5}.muted{fill:#91a0b5}.grid{stroke:#263246;stroke-width:1}.label{font-size:14px}.value{font-size:13px;font-variant-numeric:tabular-nums}</style>',
        '<text x="42" y="42" font-size="26" font-weight="700">Parallel EKF source-edit cost</text>',
        '<text x="42" y="68" class="muted" font-size="15">Changed source between baseline and advanced variants; zero means the runtime/backend changed instead of the program source</text>',
        f'<text x="{left}" y="98" font-size="15" font-weight="600">Changed line slots</text>',
        f'<text x="{left + panel_width + 80}" y="98" font-size="15" font-weight="600">Changed characters</text>',
    ]
    for tick in range(0, 6):
        frac = tick / 5
        lx = left + panel_width * frac
        cx = left + panel_width + 80 + panel_width * frac
        lines.append(f'<line x1="{lx:.1f}" y1="{top - 12}" x2="{lx:.1f}" y2="{height - bottom}" class="grid"/>')
        lines.append(f'<line x1="{cx:.1f}" y1="{top - 12}" x2="{cx:.1f}" y2="{height - bottom}" class="grid"/>')
        lines.append(f'<text x="{lx:.1f}" y="{height - bottom + 22}" text-anchor="middle" class="muted" font-size="12">{int(max_lines * frac)}</text>')
        lines.append(f'<text x="{cx:.1f}" y="{height - bottom + 22}" text-anchor="middle" class="muted" font-size="12">{int(max_chars * frac):,}</text>')
    for index, row in enumerate(rows):
        y = top + index * row_height
        color = COLORS[row["language"]]
        line_value = row["baseline_to_advanced"]["changed_line_slots"]
        char_value = row["baseline_to_advanced"]["changed_chars"]
        line_width = panel_width * line_value / max_lines
        char_width = panel_width * char_value / max_chars
        lines.append(f'<text x="{left - 14}" y="{y + 17}" text-anchor="end" class="label">{esc(row["language"])}</text>')
        lines.append(f'<rect x="{left}" y="{y + 3}" width="{max(1, line_width):.1f}" height="22" rx="3" fill="{color}" opacity="0.9"/>')
        lines.append(f'<text x="{min(left + line_width + 8, left + panel_width - 8):.1f}" y="{y + 18}" class="value">{line_value}</text>')
        char_x = left + panel_width + 80
        lines.append(f'<rect x="{char_x}" y="{y + 3}" width="{max(1, char_width):.1f}" height="22" rx="3" fill="{color}" opacity="0.9"/>')
        lines.append(f'<text x="{min(char_x + char_width + 8, char_x + panel_width - 8):.1f}" y="{y + 18}" class="value">{char_value:,}</text>')
    support = report["mech_backend_support_delta"]
    lines.append(f'<text x="42" y="{height - 35}" class="muted" font-size="12">Mech uses the same high-level `.mec` source for every backend; native-Metal support changed {support["changed_line_slots"]} backend line slots and is intentionally not counted as program edits.</text>')
    lines.append('</svg>')
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("cross_language", type=Path)
    parser.add_argument("native", type=Path)
    parser.add_argument("taichi", type=Path)
    parser.add_argument("lua", type=Path)
    parser.add_argument("output_directory", type=Path)
    args = parser.parse_args()
    report = build_report(
        json.loads(args.cross_language.read_text(encoding="utf-8")),
        json.loads(args.native.read_text(encoding="utf-8")),
        json.loads(args.taichi.read_text(encoding="utf-8")),
        json.loads(args.lua.read_text(encoding="utf-8")),
    )
    args.output_directory.mkdir(parents=True, exist_ok=True)
    (args.output_directory / "parallel-ekf-source-diff-report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    (args.output_directory / "parallel-ekf-source-diff-report.md").write_text(markdown(report), encoding="utf-8")
    (args.output_directory / "parallel-ekf-source-edit-cost.svg").write_text(svg(report), encoding="utf-8")


if __name__ == "__main__":
    main()
