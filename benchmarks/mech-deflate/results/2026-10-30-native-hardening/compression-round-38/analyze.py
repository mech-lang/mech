"""Rebuild this round's scorecard from unpacked receipts, without execution.

Extract all receipts.tar.gz below this packet into one scratch directory, then
pass it as the first argument. An optional second argument changes the output
directory (useful for checking an independent reconstruction).
"""
from pathlib import Path
import csv
import json
import re
import statistics
import sys

root = Path(sys.argv[1])
destination = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).resolve().parent
destination.mkdir(parents=True, exist_ok=True)
primary_path = root / "workers/summary.json"
if not primary_path.exists():
    primary_path = root / "summary.json"  # worker cohort archive preserves its local layout


def rows(label, allocation=False):
    reference = label.startswith("reference-")
    filename = "run.stderr" if reference or allocation else "run.stdout"
    lines = (root / "diagnostics" / label / filename).read_text().splitlines()
    if reference or allocation:
        result = [json.loads(line.removeprefix("ALLOC_PROBE ")) for line in lines
                  if line.startswith("ALLOC_PROBE ")]
        if not reference:
            result = result[1:-1]
    else:
        result = [item for line in lines if line.startswith("{")
                  for item in [json.loads(line)]
                  if not item.get("warmup") and not item.get("profile")]
    assert len(result) == (8 if allocation and not reference else 32), label
    return result


def describe(observations, keys):
    return {key: {"median": statistics.median(item[key] for item in observations),
                  "min": min(item[key] for item in observations),
                  "max": max(item[key] for item in observations)}
            for key in keys}


cpu_keys = ["instructions", "cycles", "rss_before", "rss_after", "physical_before",
            "physical_after", "lifetime_peak_physical", "pageins"]
allocation_keys = ["allocation_requests", "requested_bytes", "deallocation_calls",
                   "reallocations", "relocating_reallocations",
                   "relocating_realloc_copy_upper_bound", "live_usable_before",
                   "live_usable_after", "peak_live_usable", "incremental_peak_live_usable"]
data = {"schema": "mech.complete-eight-byte-executable-profile.v2",
        "input_bytes": 8, "raw_output_bytes": 10, "level": 12,
        "base": "eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b",
        "starting_published_head": "ba198160dc51162da49d962866fe4a19e0aa051c",
        "before_compiler_source": "3a54f1a1d64db9eee01db97763a3f95a138db93a",
        "ownership_change": "f64c9e65f26e6fc47cd16c0ea419643712196fbd",
        "after_compiler_source": "dc77024f29f373f0938695531ab2b7c975bb1e7e",
        "revision_ir_work_abi_bytecode": [131, 95, 37, 14],
        "primary": json.loads(primary_path.read_text()),
        "cpu": {}, "allocation": {}, "cpu_boundary_calibration": {}}

for label in ["reference", "before", "after"]:
    directory = "reference-cpu" if label == "reference" else "cpu-" + label
    observations = rows(directory)
    assert all(item["instructions"] > 0 and item["cycles"] > 0 and item["pageins"] == 0
               for item in observations)
    result = describe(observations, cpu_keys)
    result["samples"] = len(observations)
    result["ipc_ratio_of_medians"] = result["instructions"]["median"] / result["cycles"]["median"]
    data["cpu"][label] = result
    if label != "reference":
        text = (root / "diagnostics" / directory / "run.stderr").read_text()
        line = next(line for line in text.splitlines() if line.startswith("CPU_BOUNDARY_CALIBRATION "))
        data["cpu_boundary_calibration"][label] = json.loads(line.split(" ", 1)[1])
    directory = "reference-memory" if label == "reference" else "memory-" + label
    observations = rows(directory, allocation=True)
    assert all(item["memory_instrumented"] for item in observations)
    histograms = [{entry["bytes"]: entry["count"] for entry in item["large_allocations"]}
                  for item in observations]
    assert all(histogram == histograms[0] for histogram in histograms)
    result = describe(observations, allocation_keys)
    result["samples"] = len(observations)
    result["large_requested_size_histogram"] = histograms[0]
    data["allocation"][label] = result

data["ratios_to_reference"] = {}
data["after_change_percent"] = {}
for label in ["before", "after"]:
    data["ratios_to_reference"][label] = {}
    for section, keys in [("cpu", ["instructions", "cycles"]),
                          ("allocation", ["allocation_requests", "requested_bytes",
                                          "incremental_peak_live_usable"])]:
        for key in keys:
            current = data[section][label][key]["median"]
            reference = data[section]["reference"][key]["median"]
            data["ratios_to_reference"][label][key] = current / reference
            if label == "after":
                before = data[section]["before"][key]["median"]
                data["after_change_percent"][key] = 100 * (current / before - 1)
data["after_change_percent"]["primary_elapsed"] = 100 * (data["primary"]["pooled_ratio"] - 1)
data["generated_object"] = json.loads((root / "generated-object/comparison.json").read_text())
data["profile"] = json.loads((root / "partitioned-profile.json").read_text())
data["ownership_census"] = json.loads((root / "census-comparison.json").read_text())
linked_cpu = [item for line in (root / "linked/cpu.stdout").read_text().splitlines()
              if line.startswith("{") for item in [json.loads(line)]
              if not item.get("warmup") and not item.get("profile")]
linked_allocation = [json.loads(line.removeprefix("ALLOC_PROBE "))
                     for line in (root / "linked/memory.stderr").read_text().splitlines()
                     if line.startswith("ALLOC_PROBE ")][1:-1]
assert len(linked_cpu) == 32 and len(linked_allocation) == 8
assert all(item["pageins"] == 0 and item["work"] == 44443824446 for item in linked_cpu)
assert all(item["allocation_requests"] == 13994 and item["requested_bytes"] == 98586819
           and item["incremental_peak_live_usable"] == 20575432 for item in linked_allocation)
data["linked_compressor_diagnostic"] = {
    "scope": json.loads((root / "linked/pins.json").read_text())["scope"],
    "cpu": describe(linked_cpu, cpu_keys),
    "allocation": describe(linked_allocation, allocation_keys),
    "cpu_observations": len(linked_cpu), "allocation_observations": len(linked_allocation),
    "instructions_multiple_of_reference": statistics.median(item["instructions"] for item in linked_cpu)
        / data["cpu"]["reference"]["instructions"]["median"],
    "cycles_multiple_of_reference": statistics.median(item["cycles"] for item in linked_cpu)
        / data["cpu"]["reference"]["cycles"]["median"],
    "primary_elapsed_comparison_claimed": False}
data["static"] = {}
for label, filename in [("reference_whole_executable", "static/reference.stdout"),
                        ("linked_diagnostic_whole_executable", "static/linked.stdout"),
                        ("jit_diagnostic_whole_executable", "generated-object/runner-sections.stdout"),
                        ("generated_compressor_object", "generated-object/sections.stdout")]:
    text = (root / filename).read_text()
    match = re.search(r"Section (?:\(__TEXT, )?__text\)?: (\d+)", text)
    assert match, label
    size = int(match[1])
    data["static"][label] = {"text_bytes": size,
                            "four_byte_instruction_slots_including_padding": size // 4}
data["static"]["scope"] = (
    "Whole diagnostic executables include compiler, catalog, verification and setup helpers; "
    "generated compressor Object is a single entry. Static instruction slots include padding, "
    "and are distinct from actual executed instructions. No minimal runtime-only binary-size claim.")
data["scope"] = [
    "CPU, allocation, sample profile and primary elapsed measurements are separate cohorts.",
    "CPU fields are actual hardware counters and include the measured getter overhead; no silent calibration subtraction.",
    "Incremental peak is tracked live usable heap above the compression entry baseline, not whole-task RSS.",
    "Whole-task footprint includes compiler/JIT preparation and swapped memory; not runtime-only AOT peak memory.",
    "Requested path capacity is distinct from initialized payload copying.",
    "No cache/branch-miss, total-copy-byte, energy, streaming-throughput or larger-input result is claimed.",
    "Primary descriptive blocks share four independently prepared worker pairs; no latency speedup established."]

(destination / "scorecard.json").write_text(json.dumps(data, indent=2) + "\n")
with (destination / "scorecard.csv").open("w", newline="") as output:
    writer = csv.writer(output)
    writer.writerow(["metric", "unit", "reference", "before", "after",
                     "after_multiple_of_reference", "after_change_percent", "scope"])
    primary = data["primary"]
    writer.writerow(["primary_elapsed", "seconds", primary["reference_seconds"],
                     primary["before"]["median_seconds"], primary["after"]["median_seconds"],
                     primary["after"]["multiple_of_roc"], data["after_change_percent"]["primary_elapsed"],
                     "four independent pairs; 3968 primary calls; slightly slower pooled median"])
    for section, keys in [("cpu", ["instructions", "cycles"]),
                          ("allocation", ["allocation_requests", "requested_bytes",
                                          "incremental_peak_live_usable"])]:
        for key in keys:
            values = [data[section][label][key]["median"] for label in ["reference", "before", "after"]]
            writer.writerow([key, "bytes" if "bytes" in key or "usable" in key else "count",
                             *values, values[-1] / values[0], data["after_change_percent"][key],
                             "independent diagnostic cohort, not primary elapsed"])
    for section, key in [("cpu", "instructions"), ("cpu", "cycles"),
                         ("cpu", "rss_after"), ("cpu", "physical_after"),
                         ("allocation", "live_usable_before"),
                         ("allocation", "incremental_peak_live_usable")]:
        linked = data["linked_compressor_diagnostic"][section][key]["median"]
        reference = data[section]["reference"][key]["median"]
        writer.writerow(["linked_" + key, "count" if key in ["instructions", "cycles"] else "bytes",
                         reference, "", linked, linked / reference, "",
                         "linked compressor; two tiny setup helpers retain JIT outside compression"])
print(json.dumps(data["after_change_percent"], indent=2))
