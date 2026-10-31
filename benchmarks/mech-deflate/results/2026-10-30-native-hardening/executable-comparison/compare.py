"""Recompute the retained-executable comparison from checked-in scorecards.

Usage: python3 compare.py [OUTPUT_DIRECTORY]
No compiler, benchmark, network access or third-party package is used.
"""

import csv
import hashlib
import json
from pathlib import Path
import sys

packet = Path(__file__).resolve().parent
root = packet.parent
scorecard_path = root / "compression-round-43/scorecard.json"
deep_path = root / "deep-profile/scorecard.json"
static_path = packet / "static.json"
score = json.loads(scorecard_path.read_text())
deep = json.loads(deep_path.read_text())
static = json.loads(static_path.read_text())
assert static["mech"]["file_sha256"] == score["static"]["candidate"]["object_sha256"]
rows = []


def add(metric, unit, reference, mech, scope, previous=None):
    rows.append({"metric": metric, "unit": unit, "reference": reference,
                 "mech": mech, "mech_over_reference": mech / reference,
                 "mech_minus_reference": mech - reference, "previous_mech": previous,
                 "change_percent": 100 * (mech / previous - 1) if previous else None, "scope": scope})


add("elapsed", "seconds", score["primary"]["reference_seconds"],
    score["primary"]["medians"]["after"]["seconds"], "primary compression timer",
    score["primary"]["medians"]["before"]["seconds"])
for metric in ["instructions", "cycles", "rss_after", "physical_after"]:
    add(metric, "bytes" if metric.endswith("after") else "count",
        score["cpu"]["reference"][metric]["median"],
        score["cpu"]["candidate"][metric]["median"],
        "whole prepared task after compression" if metric.endswith("after")
        else "hardware counter delta at compression boundaries; separate coherent CPU cohort",
        score["cpu"]["baseline"][metric]["median"])
for metric in ["allocation_requests", "requested_bytes", "incremental_peak_live_usable",
               "peak_live_usable"]:
    add(metric, "count" if metric == "allocation_requests" else "bytes",
        score["allocation"]["reference"][metric]["median"],
        score["allocation"]["candidate"][metric]["median"],
        "tracked allocator; separate coherent memory cohort; peak excludes private heaps/direct VM",
        score["allocation"]["baseline"][metric]["median"])
add("text_bytes", "bytes", deep["static"]["reference"]["text_bytes"],
    score["static"]["candidate"]["code_bytes"],
    "scope differs: whole reference executable text versus generated Mech compressor entry")
for metric in static["mech"]["categories"]:
    add(metric, "static sites", static["reference"]["categories"][metric],
        static["mech"]["categories"][metric],
        "disassembled sites including unreached/error paths and padding; not dynamic counts")

result = {
    "schema": "mech.retained-executable-comparison.v1",
    "compiler_source": score["release"]["source_head"],
    "implementation": score["candidate_implementation"],
    "previous_compiler_source": score["baseline_compiler"],
    "publication_start": score["published_start"],
    "reference_compiler_revision": deep["pins"]["reference_compiler_revision_recorded_at_original_build"],
    "reference_package_revision": deep["pins"]["reference_package_revision"],
    "input_bytes": 8, "raw_output_bytes": 10, "level": 12,
    "new_timing_claimed": False,
    "description": "Latest measured linked-executable primary cohort plus separate diagnostic cohorts with the exact same per-variant coherent libraries. Static inventory reused only after proving emitted Object byte identity.",
    "primary_counts": {"mech": score["primary"]["observations_per_variant"], "reference": 20000},
    "cpu_counts": {name: score["cpu"][name]["observations"] for name in ["baseline", "candidate", "reference"]},
    "allocation_counts": {name: score["allocation"][name]["observations"] for name in ["baseline", "candidate", "reference"]},
    "rows": rows,
    "prepared_live_heap_bytes": {name: score["allocation"][name]["live_usable_before"]["median"] for name in ["baseline", "candidate", "reference"]},
    "large_backing_request_counts": {
        "path": {"mech": 35, "reference": 1},
        "tree": {"mech": 5, "reference": 1},
        "cache": {"mech": 1, "reference": 1}},
    "extra_path_requested_capacity_bytes": 34 * 2442056,
    "tree_initialized_bytes_copied_by_four_fallbacks": 4 * 262144 * 2,
    "mech_profile_sample_weights": score["profiles"]["candidate"]["disjoint_categories"],
    "mech_profile_total_sample_weights": score["profiles"]["candidate"]["total_weights"],
    "input_files": [{"path": str(path.relative_to(root)),
                     "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
                    for path in [scorecard_path, deep_path, static_path]],
    "limitations": [
        "Elapsed, CPU, allocation and sampling are separate cohorts, not simultaneous observations.",
        score["diagnostic_scope"],
        "Reported hardware instruction counts include about 5010 instructions of uncorrected boundary-read overhead.",
        "Requested capacity is not initialized data copied; total memcpy/input-copy bytes remain unmeasured.",
        "Prepared task includes compiler/catalog/setup state; it is not a minimal runtime-only application.",
        "Static instruction sites include padding and unreached paths; static categories do not measure branch/cache misses.",
        "Only the complete eight-byte case; no larger-input throughput or scaling prediction."]}

out = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else packet
out.mkdir(parents=True, exist_ok=True)
(out / "comparison.json").write_text(json.dumps(result, indent=2) + "\n")
with (out / "comparison.csv").open("w", newline="") as stream:
    writer = csv.DictWriter(stream, fieldnames=list(rows[0]))
    writer.writeheader()
    writer.writerows(rows)
print(json.dumps({"compiler_source": result["compiler_source"], "metrics": len(rows),
                  "new_timing_claimed": False, "output": str(out)}, indent=2))
