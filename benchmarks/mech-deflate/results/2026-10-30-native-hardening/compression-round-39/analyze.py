"""Recreate the three linked-executable scorecard from unpacked receipts."""
from pathlib import Path
import csv
import json
import statistics
import sys

root = Path(sys.argv[1])
out = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).resolve().parent
out.mkdir(parents=True, exist_ok=True)
labels = ["reference", "baseline", "initial", "refined"]


def observations(label, kind):
    name = "reference-" + kind if label == "reference" else label + "-" + kind
    directory = root / "counters" / name
    stderr = label == "reference" or kind == "memory"
    text = (directory / ("run.stderr" if stderr else "run.stdout")).read_text()
    if stderr:
        rows = [json.loads(line.removeprefix("ALLOC_PROBE ")) for line in text.splitlines()
                if line.startswith("ALLOC_PROBE ")]
        if label != "reference":
            rows = rows[1:-1]
    else:
        rows = [row for line in text.splitlines() if line.startswith("{")
                for row in [json.loads(line)] if not row["warmup"] and not row["profile"]]
    assert len(rows) == (8 if kind == "memory" and label != "reference" else 32)
    return rows


def describe(rows, keys):
    return {key: {"median": statistics.median(row[key] for row in rows),
                  "min": min(row[key] for row in rows), "max": max(row[key] for row in rows)}
            for key in keys}


data = {"schema": "mech.complete-eight-byte-linked-triplet.v1", "input_bytes": 8,
        "raw_output_bytes": 10, "level": 12, "chosen": "initial",
        "stack_base": "eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b",
        "published_start": "369583114cdc10b60372eebaff4b63e3a2721fb6",
        "initial_implementation": "552e537fcbcfc3811ce442c982010b5e1e999e6c",
        "refinement_experiment": "892275fec5b462ca76f0ba47a34dfefb6096c6bc",
        "restore_initial_selection": "21b20630b88f0321b025eec8784f25ad24813202",
        "primary": json.loads((root / "linked-primary/summary.json").read_text()),
        "cpu": {}, "allocation": {}, "static": {}, "calibration": {}}
cpu_keys = ["instructions", "cycles", "rss_after", "physical_after",
            "lifetime_peak_physical", "pageins"]
memory_keys = ["allocation_requests", "requested_bytes", "deallocation_calls", "reallocations",
               "relocating_reallocations", "relocating_realloc_copy_upper_bound",
               "live_usable_before", "live_usable_after", "peak_live_usable",
               "incremental_peak_live_usable"]
for label in labels:
    rows = observations(label, "cpu")
    assert all(row["instructions"] > 0 and row["cycles"] > 0 and row["pageins"] == 0 for row in rows)
    data["cpu"][label] = describe(rows, cpu_keys)
    data["cpu"][label]["observations"] = len(rows)
    rows = observations(label, "memory")
    assert all(row["memory_instrumented"] for row in rows)
    histograms = [{entry["bytes"]: entry["count"] for entry in row["large_allocations"]}
                  for row in rows]
    assert all(histogram == histograms[0] for histogram in histograms)
    data["allocation"][label] = describe(rows, memory_keys)
    data["allocation"][label]["observations"] = len(rows)
    data["allocation"][label]["large_allocation_histogram"] = histograms[0]
    if label != "reference":
        meta = json.loads((root / "linked-primary" / label / "metadata.json").read_text())
        data["static"][label] = {key: meta[key] for key in ["code_bytes", "stack_bytes"]}
        data["static"][label]["native_pieces"] = len(meta["compiled_numeric_segments"])
        text = (root / "counters" / (label + "-cpu/run.stderr")).read_text()
        line = next(line for line in text.splitlines() if line.startswith("CPU_BOUNDARY_CALIBRATION "))
        data["calibration"][label] = json.loads(line.split(" ", 1)[1])
data["ratios_to_reference"] = {}
data["changes_from_baseline_percent"] = {}
for label in labels[1:]:
    data["ratios_to_reference"][label] = {}
    data["changes_from_baseline_percent"][label] = {}
    for section, keys in [("cpu", ["instructions", "cycles"]),
                          ("allocation", ["allocation_requests", "requested_bytes",
                                          "incremental_peak_live_usable"])]:
        for key in keys:
            value = data[section][label][key]["median"]
            data["ratios_to_reference"][label][key] = value / data[section]["reference"][key]["median"]
            data["changes_from_baseline_percent"][label][key] = 100 * (
                value / data[section]["baseline"][key]["median"] - 1)
data["profile"] = json.loads((root / "linked-profile-comparison.json").read_text())
data["limitations"] = [
    "New matched linked configuration: compare only within it; no JIT-to-linked compiler gain.",
    "CPU, memory, profile and primary elapsed observations are separate cohorts.",
    "CPU values include the measured getter overhead; no silent calibration subtraction.",
    "Incremental peak is tracked usable heap above compression entry, not whole-task RSS or exact all-memory peak.",
    "Two tiny setup helpers retain JIT outside compression; no minimal runtime-only binary claim.",
    "Only complete eight-byte compression is timed; no larger-input throughput or parity claim.",
    "All three candidates and the unsuccessful profitability experiment remain recorded."]
(out / "scorecard.json").write_text(json.dumps(data, indent=2) + "\n")
with (out / "scorecard.csv").open("w", newline="") as file:
    writer = csv.writer(file)
    writer.writerow(["metric", "unit", "reference", "baseline", "initial", "refined",
                     "chosen_multiple_of_reference", "chosen_change_from_baseline_percent", "scope"])
    primary = data["primary"]
    writer.writerow(["primary_elapsed", "seconds", primary["reference_seconds"],
                     *[primary["medians"][label]["seconds"] for label in labels[1:]],
                     primary["medians"]["initial"]["multiple_of_reference"],
                     100 * (primary["comparisons"]["initial_over_baseline"]["pooled_ratio"] - 1),
                     "four prepared triplets; 5952 primary calls; all groups/blocks improve versus baseline"])
    for section, keys in [("cpu", ["instructions", "cycles"]),
                          ("allocation", ["allocation_requests", "requested_bytes",
                                          "incremental_peak_live_usable"])]:
        for key in keys:
            writer.writerow([key, "bytes" if "bytes" in key or "usable" in key else "count",
                             *[data[section][label][key]["median"] for label in labels],
                             data["ratios_to_reference"]["initial"][key],
                             data["changes_from_baseline_percent"]["initial"][key],
                             "independent diagnostic cohort; not primary elapsed"])
    for key in ["rss_after", "physical_after", "lifetime_peak_physical"]:
        values = [data["cpu"][label][key]["median"] for label in labels]
        writer.writerow([key, "bytes", *values, values[2] / values[0],
                         100 * (values[2] / values[1] - 1),
                         "whole prepared task, not incremental compressor heap; setup/compiler/catalog retained"])
    writer.writerow(["generated_compressor_code", "bytes", "not function-matched",
                     *[data["static"][label]["code_bytes"] for label in labels[1:]],
                     "not comparable", 100 * (data["static"]["initial"]["code_bytes"] /
                                              data["static"]["baseline"]["code_bytes"] - 1),
                     "generated entry alone; reference whole executable text is 195456 bytes"])
print(json.dumps(data["changes_from_baseline_percent"]["initial"], indent=2))
