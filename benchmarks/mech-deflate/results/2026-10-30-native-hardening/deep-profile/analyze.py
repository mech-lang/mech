"""Rebuild the comparison from the archived, separately scoped observations.

Unpack every receipts.tar.gz in this directory into one scratch directory, then
pass that directory as the only argument. Does not run a benchmark or compiler.
"""
from pathlib import Path
from collections import Counter
import csv
import json
import re
import statistics
import sys

root = Path(sys.argv[1])
destination = Path(__file__).resolve().parent


def load_rows(name, allocation=False):
    filename = "run.stderr" if allocation else "run.stdout"
    lines = (root / name / filename).read_text().splitlines()
    if allocation:
        return [json.loads(line.removeprefix("ALLOC_PROBE ")) for line in lines
                if line.startswith("ALLOC_PROBE ")]
    return [row for line in lines if line.startswith("{")
            for row in [json.loads(line)]
            if not row.get("warmup") and not row.get("profile")]


def describe(rows, keys):
    return {key: {"median": statistics.median(row[key] for row in rows),
                  "min": min(row[key] for row in rows),
                  "max": max(row[key] for row in rows)}
            for key in keys}


cpu_keys = ["instructions", "cycles", "rss_before", "rss_after",
            "physical_before", "physical_after", "lifetime_peak_physical", "pageins"]
memory_keys = ["allocation_requests", "requested_bytes", "deallocation_calls",
               "reallocations", "relocating_reallocations",
               "relocating_realloc_copy_upper_bound", "live_usable_before",
               "live_usable_after", "peak_live_usable", "incremental_peak_live_usable"]
data = {"schema": "mech.complete-eight-byte-executable-profile.v1",
        "input_bytes": 8, "output_bytes": 10, "level": 12,
        "units": "Bytes are exact integers; displayed MB are decimal. Cycles and instructions are actual CPU counters, not logical work.",
        "cpu": {}, "allocation": {}}
data["pins"] = json.loads((root / "pins.json").read_text())
data["primary_paired_elapsed"] = json.loads((root / "primary-paired-summary.json").read_text())
data["cpu_boundary_calibration"] = {}
for label in ["round36", "round37"]:
    line = next(line for line in (root / ("cpu-" + label) / "run.stderr").read_text().splitlines()
                if line.startswith("CPU_BOUNDARY_CALIBRATION "))
    data["cpu_boundary_calibration"][label] = json.loads(line.split(" ", 1)[1])
for label, directory in [("round36", "cpu-round36"), ("round37", "cpu-round37"),
                         ("reference", "reference-cpu-run")]:
    rows = load_rows(directory, allocation=label == "reference")
    assert len(rows) == 32
    assert all(row["instructions"] > 0 and row["cycles"] > 0 and row["pageins"] == 0 for row in rows)
    summary = describe(rows, cpu_keys)
    summary["samples"] = len(rows)
    summary["ipc_ratio_of_medians"] = summary["instructions"]["median"] / summary["cycles"]["median"]
    data["cpu"][label] = summary

for label, directory in [("round36", "memory-round36"), ("round37", "memory-round37"),
                         ("reference", "reference-memory-run")]:
    rows = load_rows(directory, allocation=True)
    if label != "reference":
        rows = rows[1:-1]  # Same warm-up/final exclusion as the timing harness.
    assert len(rows) == (32 if label == "reference" else 8)
    assert all(row["memory_instrumented"] for row in rows)
    histograms = [{entry["bytes"]: entry["count"] for entry in row["large_allocations"]}
                  for row in rows]
    assert all(histogram == histograms[0] for histogram in histograms)
    summary = describe(rows, memory_keys)
    summary["samples"] = len(rows)
    summary["large_requested_size_histogram"] = histograms[0]
    data["allocation"][label] = summary

attribution = (root / "allocation-attribution/run.stderr").read_text()
stacks = re.findall(r"ALLOCATION_STACK index=(\d+) bytes=(\d+) frames=(\d+)\n(.*?)(?=ALLOCATION_STACK|MEASURING|CLEANUP|\Z)",
                    attribution, re.S)
counts = Counter()
for index, size, frame_count, stack in stacks:
    if "copy_buffer_with_sublist" in stack:
        path = "sequence_buffers::copy_buffer_with_sublist"
    elif "word_updates4copy" in stack:
        path = "native_collection_word_updates::copy"
    elif "uniform_words" in stack:
        path = "CollectedValues::reserve / State::fill_uniform_words"
    else:
        raise AssertionError("Unattributed large allocation: " + stack)
    counts[(int(size), path)] += 1
assert sum(counts.values()) == 45
data["large_allocation_attribution"] = [
    {"requested_bytes": size, "path": path, "count": count}
    for (size, path), count in sorted(counts.items())]
assert counts[(2442056, "sequence_buffers::copy_buffer_with_sublist")] == 39
assert counts[(524288, "native_collection_word_updates::copy")] == 4

static = {}
for label in ["reference", "mech-jit-runner", "mech-generated"]:
    text = (root / "binary-inspection" / (label + "-size.stdout")).read_text()
    match = re.search(r"Section (?:\(__TEXT, )?__text\)?: (\d+)", text)
    assert match, label
    code_bytes = int(match[1])
    symbols = (root / "binary-inspection" / (label + "-symbols.stdout")).read_text()
    static[label] = {"text_bytes": code_bytes,
                     "four_byte_instruction_slots_including_padding": code_bytes // 4,
                     "text_symbols_including_aliases": len(re.findall(r"^[0-9a-fA-F]+ [Tt] ", symbols, re.M))}
data["static"] = static
data["static"]["scope"] = "Fresh round37 generated Object, descriptor and metadata are byte-identical to round36; comparison receipt included. JIT runner contains compiler and verification code. Reference text includes its whole application/runtime. Slots include padding and are not retired instructions."
data["generated_object_verification"] = json.loads((root / "generated-object/comparison.json").read_text())

data["ratios_to_reference"] = {}
for label in ["round36", "round37"]:
    data["ratios_to_reference"][label] = {
        **{key: data["cpu"][label][key]["median"] / data["cpu"]["reference"][key]["median"]
           for key in ["instructions", "cycles"]},
        **{key: data["allocation"][label][key]["median"] / data["allocation"]["reference"][key]["median"]
           for key in ["allocation_requests", "requested_bytes", "incremental_peak_live_usable"]}}
data["opportunity_bounds"] = {
    "path_capacity_allocations_beyond_one": 38,
    "path_capacity_bytes_requested_beyond_one": 38 * 2442056,
    "tree_backing_copies": 4,
    "tree_initialized_bytes_copied": 4 * 524288,
    "note": "Requested path capacity is uninitialized spare storage, not equal to copied bytes or proportional elapsed cost. Real aliases may require some copies; these are investigation targets, not promised elimination/speedups."}
data["limitations"] = [
    "Eight-byte case only; no extrapolation to larger input throughput.",
    "Compression CPU samples are diagnostic cohorts, not a new paired elapsed-time result.",
    "Allocation instrumentation is separate; its CPU/time fields are excluded from CPU results.",
    "Tracked peak usable heap excludes direct VM mappings/libc-private heaps and internal transient overlap during realloc.",
    "Whole-Mech memory includes preparation/JIT/retained allocator state; not a runtime-only AOT compressor-memory result.",
    "No exact total memcpy bytes, cache-miss, branch-miss, memory-bandwidth or energy comparison claimed."]

(destination / "scorecard.json").write_text(json.dumps(data, indent=2) + "\n")
with (destination / "scorecard.csv").open("w", newline="") as output:
    writer = csv.writer(output)
    writer.writerow(["metric", "unit", "reference", "mech_round36", "mech_round37", "round37_multiple_of_reference", "scope"])
    primary = data["primary_paired_elapsed"]
    writer.writerow(["primary_elapsed", "seconds", primary["reference_seconds"],
                     primary["before"]["median_seconds"], primary["after"]["median_seconds"],
                     primary["after"]["multiple_of_roc"], "primary paired comparison; two improving and two regressing worker pairs"])
    for section, keys in [("cpu", ["instructions", "cycles"]),
                          ("allocation", ["allocation_requests", "requested_bytes", "incremental_peak_live_usable"])]:
        for key in keys:
            values = [data[section][label][key]["median"] for label in ["reference", "round36", "round37"]]
            writer.writerow([key, "bytes" if "bytes" in key or "usable" in key else "count", *values, values[-1] / values[0], "per compression; independent diagnostic cohorts"])
print(json.dumps(data["ratios_to_reference"], indent=2))
