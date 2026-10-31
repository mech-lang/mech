"""Recompute both timing cohorts and the executable scorecard from raw receipts."""
from pathlib import Path
from collections import Counter
import csv
import hashlib
import json
import re
import statistics
import sys

root = Path(sys.argv[1])
out = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).resolve().parent
out.mkdir(parents=True, exist_ok=True)
labels = ["reference", "baseline", "initial", "refined"]
locations = {"reference": (root / "refined/counters", "reference"),
             "baseline": (root / "refined/counters", "before"),
             "initial": (root / "counters", "after"),
             "refined": (root / "refined/counters", "after")}


def observations(label, kind):
    directory, name = locations[label]
    directory = directory / (name + "-" + kind)
    stderr = label == "reference" or kind == "memory"
    text = (directory / ("run.stderr" if stderr else "run.stdout")).read_text()
    if stderr:
        rows = [json.loads(line.removeprefix("ALLOC_PROBE ")) for line in text.splitlines()
                if line.startswith("ALLOC_PROBE ")]
        if label != "reference":
            rows = rows[1:-1]
    else:
        rows = [json.loads(line) for line in text.splitlines() if line.startswith("{")]
        rows = [row for row in rows if not row["warmup"] and not row["profile"]]
    assert len(rows) == (8 if kind == "memory" and label != "reference" else 32)
    return rows


def describe(rows, keys):
    return {key: {"median": statistics.median(row[key] for row in rows),
                  "min": min(row[key] for row in rows), "max": max(row[key] for row in rows)}
            for key in keys}


def profile(path):
    graph = path.read_text().split("Call graph:\n", 1)[1].split("\nTotal number in stack", 1)[0]
    nodes, stack = [], []
    for line in graph.splitlines():
        match = re.match(r"^([ +!|:]*)(\d+) (.*)$", line)
        if not match:
            continue
        depth = len(match[1])
        while stack and nodes[stack[-1]]["depth"] >= depth:
            stack.pop()
        node = {"depth": depth, "count": int(match[2]),
                "symbol": match[3].split("  (in ", 1)[0], "children": [], "ancestors": stack[:]}
        if stack:
            nodes[stack[-1]]["children"].append(len(nodes))
        stack.append(len(nodes))
        nodes.append(node)
    categories, leaves = Counter(), Counter()
    for node in nodes:
        weight = node["count"] - sum(nodes[child]["count"] for child in node["children"])
        assert weight >= 0
        symbols = [nodes[ancestor]["symbol"] for ancestor in node["ancestors"]] + [node["symbol"]]
        has = lambda needle: any(needle in symbol for symbol in symbols)
        if has("buffer_frame_action"):
            category = "buffer_frame_actions"
        elif has("prepared_library_call"):
            category = "selected_library_call"
        elif has("10Invocation6helper"):
            category = "other_invocation_helper"
        elif has("10Invocation5visit"):
            category = "ordinary_control_visit"
        else:
            category = "remaining"
        categories[category] += weight
        leaves[node["symbol"]] += weight
    return {"sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "total_weights": sum(categories.values()), "disjoint_categories": dict(categories),
            "exclusive_top_leaves": leaves.most_common(30)}


data = {"schema": "mech.rejected-scalar-aggregate-admission.v1", "chosen": "baseline",
        "stack_base": "eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b",
        "published_start": "39f0a3f662cc5b4608b5b2372955bf10992c6cbd",
        "baseline_compiler": "552e537fcbcfc3811ce442c982010b5e1e999e6c",
        "initial_compiler": "411c2e1f01d9435a1c78d28f3a0a123dcd2ba56e",
        "refined_compiler": "97ab5892d136450fe260af8c16956f23488a7ec8",
        "restore_published_source": "f06fe15783556a7387c6575c29181dd41ab44f4a",
        "input_bytes": 8, "raw_output_bytes": 10, "level": 12,
        "first_pairs": json.loads((root / "linked-primary/summary.json").read_text()),
        "primary": json.loads((root / "refined-primary/summary.json").read_text()),
        "cpu": {}, "allocation": {}, "static": {}, "profiles": {}}
cpu_keys = ["instructions", "cycles", "rss_after", "physical_after", "lifetime_peak_physical", "pageins"]
memory_keys = ["allocation_requests", "requested_bytes", "deallocation_calls", "reallocations",
               "relocating_reallocations", "relocating_realloc_copy_upper_bound", "live_usable_before",
               "live_usable_after", "peak_live_usable", "incremental_peak_live_usable"]
for label in labels:
    rows = observations(label, "cpu")
    assert all(row["instructions"] > 0 and row["cycles"] > 0 and row["pageins"] == 0 for row in rows)
    data["cpu"][label] = describe(rows, cpu_keys)
    data["cpu"][label]["observations"] = len(rows)
    rows = observations(label, "memory")
    assert all(row["memory_instrumented"] for row in rows)
    histograms = [{str(entry["bytes"]): entry["count"] for entry in row["large_allocations"]}
                  for row in rows]
    assert all(histogram == histograms[0] for histogram in histograms)
    data["allocation"][label] = describe(rows, memory_keys)
    data["allocation"][label]["observations"] = len(rows)
    data["allocation"][label]["large_allocation_histogram"] = histograms[0]
    if label != "reference":
        meta = json.loads((root / "refined-primary" / label / "metadata.json").read_text())
        data["static"][label] = {key: meta[key] for key in ["code_bytes", "stack_bytes",
            "ordinary_scalar_admission_groups", "ordinary_scalar_admission_sites"]}
        data["static"][label]["native_pieces"] = len(meta["compiled_numeric_segments"])
data["profiles"] = {"baseline": profile(root / "refined/profile/before.sample.txt"),
                    "initial": profile(root / "profile/after.sample.txt"),
                    "refined": profile(root / "refined/profile/after.sample.txt")}
data["profile_scope"] = "Separate 695-call main-thread samples include outer harness; disjoint ancestry weights are not precise compression-timer percentages."
data["ratios_to_reference"], data["changes_from_baseline_percent"] = {}, {}
for label in labels[1:]:
    data["ratios_to_reference"][label], data["changes_from_baseline_percent"][label] = {}, {}
    for section, keys in [("cpu", ["instructions", "cycles"]), ("allocation", ["allocation_requests",
                         "requested_bytes", "incremental_peak_live_usable"])]:
        for key in keys:
            value = data[section][label][key]["median"]
            data["ratios_to_reference"][label][key] = value / data[section]["reference"][key]["median"]
            data["changes_from_baseline_percent"][label][key] = 100 * (value / data[section]["baseline"][key]["median"] - 1)
data["limitations"] = ["Both candidates regress all four matched triplets and are removed; prior confirmed gains stay.",
    "Time, CPU, allocation and profile cohorts are separate. Initial-candidate CPU/profile rows precede the refinement cohorts.",
    "Hardware counters include recorded empty-getter overhead without subtraction; logical work is a different quantity.",
    "Additional tracked heap excludes preparation, direct VM and allocator-private heaps; whole-task footprint is separate.",
    "Full compressor is linked; two tiny setup helpers retain JIT outside timing. No minimal runtime-only size claim.",
    "Only eight-byte complete compression; no larger-input throughput, total-copy-byte, branch/cache-miss or parity claim."]
(out / "scorecard.json").write_text(json.dumps(data, indent=2) + "\n")
with (out / "scorecard.csv").open("w", newline="") as file:
    writer = csv.writer(file)
    writer.writerow(["metric", "unit", *labels, "current_multiple_of_reference", "scope"])
    primary = data["primary"]
    writer.writerow(["primary_elapsed", "seconds", primary["reference_seconds"],
        *[primary["medians"][label]["seconds"] for label in labels[1:]],
        primary["medians"]["baseline"]["multiple_of_reference"], "four matched triplets; 5952 primary calls; baseline retained"])
    for section, keys in [("cpu", ["instructions", "cycles", "rss_after", "physical_after"]),
                          ("allocation", ["allocation_requests", "requested_bytes", "incremental_peak_live_usable"])]:
        for key in keys:
            values = [data[section][label][key]["median"] for label in labels]
            writer.writerow([key, "bytes" if "bytes" in key or "usable" in key or key.endswith("after") else "count",
                             *values, values[1] / values[0], "separate diagnostic cohorts; current is baseline"])
    for key in ["code_bytes", "stack_bytes", "native_pieces", "ordinary_scalar_admission_sites"]:
        writer.writerow([key, "static", "not function-matched", *[data["static"][label][key] for label in labels[1:]],
                         "not comparable", "generated compressor; reference whole executable text is 195456 bytes"])
print(json.dumps({"chosen": data["chosen"], "comparisons": primary["comparisons"],
                  "instruction_changes": {label: data["changes_from_baseline_percent"][label]["instructions"]
                                          for label in labels[1:]}}, indent=2))
