"""Recompute both unchanged timing cohorts, CPU, allocation and fresh profiles."""
from pathlib import Path
from collections import Counter
import csv, hashlib, json, re, statistics, sys
root = Path(sys.argv[1])
out = Path(sys.argv[2]) if len(sys.argv)>2 else Path(__file__).resolve().parent
out.mkdir(parents=True, exist_ok=True)
labels=["reference","baseline","candidate"]
locations={"reference":(root/"counters","reference"),"baseline":(root/"counters","before"),"candidate":(root/"counters","after")}

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
        if has("buffer_frame_action") or has("project_frame_batch"):
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


data = {"schema": "mech.borrowed-record-selector.v1", "chosen": "baseline",
        "stack_base": "eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b",
        "published_start": "b12f6c5afb2c859f6cde29ba566ddf09e70d2f79",
        "baseline_compiler": "b12f6c5afb2c859f6cde29ba566ddf09e70d2f79",
        "candidate_implementation": "f9f8e4a50fd4658e4c74a3a5341db526ca504463",
        "release": json.loads((root / "final-release.json").read_text()),
        "input_bytes": 8, "raw_output_bytes": 10, "level": 12,
        "primary": {}, "cpu": {}, "allocation": {}, "static": {}, "profiles": {}}
summary = json.loads((root / "linked-primary/summary.json").read_text())
pooled = {label: [] for label in ["before", "after"]}
for group in range(4):
    result = json.loads((root / f"linked-primary/group{group}/results.json").read_text())
    for label in pooled:
        rows = [row for row in result["all_observations"][label]
                if not row["warmup"] and not row["profile"]]
        assert len(rows) == 496
        assert all(row["exact_reference"] and row["independent_inflation"]
                   and row["capacity"] == 13 and row["work"] == 60260219686 for row in rows)
        pooled[label].extend(rows)
for label, rows in pooled.items():
    assert len(rows) == 1984
    assert statistics.median(row["seconds"] for row in rows) == summary["medians"][label]["seconds"]
data["primary"] = summary
cpu_keys = ["instructions", "cycles", "rss_after", "physical_after", "pageins"]
memory_keys = ["allocation_requests", "requested_bytes", "reallocations",
               "relocating_reallocations", "relocating_realloc_copy_upper_bound",
               "live_usable_before", "live_usable_after", "peak_live_usable",
               "incremental_peak_live_usable"]
for label in labels:
    rows = observations(label, "cpu")
    assert all(row["instructions"] > 0 and row["cycles"] > 0 and row["pageins"] == 0 for row in rows)
    if label != "reference":
        assert all(row["work"] == 60260219686 and row["exact_reference"] and
                   row["independent_inflation"] and row["capacity"] == 13 for row in rows)
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
        name = "before" if label == "baseline" else "after"
        meta = json.loads((root / "linked-primary" / name / "metadata.json").read_text())
        data["static"][label] = {key: meta[key] for key in ["abi_version", "typed_ir_version", "code_bytes", "stack_bytes",
            "ordinary_scalar_admission_groups", "ordinary_scalar_admission_sites"]}
        data["profiles"][label] = profile(root / "isolated-profile" / f"{name}.sample.txt")
        binding = json.loads((root / "linked-primary" / name / "binding.json").read_text())
        data["static"][label]["object_sha256"] = binding["pins"]["object.o"]["sha256"]
        for kind in ["cpu", "memory"]:
            build = json.loads((root / "counters" / name / (kind + "-build.json")).read_text())
            assert build["libraries"] == binding["libraries"], (label, kind, "library graph differs")
            assert "link-arg=" + str(Path(build["command"][-3]).parent.parent.parent / "linked-primary" / name / "object.o") in build["command"]
        host = json.loads((root / "host-code/result.json").read_text())[name]
        assert host["binary_sha256"] == binding["worker_sha256"]

assert data["static"]["baseline"]["object_sha256"] == data["static"]["candidate"]["object_sha256"]
data["changes_percent"] = {"primary_seconds":100 * (summary["comparisons"]["after_over_before"]["pooled_ratio"] - 1)}
for section, keys in [("cpu", cpu_keys), ("allocation", memory_keys)]:
    for key in keys:
        base = data[section]["baseline"][key]["median"]
        after = data[section]["candidate"][key]["median"]
        data["changes_percent"][key] = 100 * (after / base - 1) if base else None
data["diagnostic_scope"] = "Each hardware/memory variant uses the exact same coherent optimized libraries and unchanged same-build Object/image as its primary worker. Compute is explicitly frozen as well as all eight direct libraries; no Cargo artifact mutation. Hardware/memory instrumentation is separate and excluded from primary elapsed."
data["profile_scope"] = "Serial isolated 695-call original primary-worker main-thread samples after all counter workers finish. Disjoint weights include surrounding harness, not exact compression-timer percentages."
data["outcome"] = {"primary_elapsed_change_percent": data["changes_percent"]["primary_seconds"], "improving_preparations": summary["comparisons"]["after_over_before"]["improving_groups"], "improving_blocks": summary["comparisons"]["after_over_before"]["improving_blocks"], "instructions_change_percent": data["changes_percent"]["instructions"], "cycles_change_percent": data["changes_percent"]["cycles"], "allocation_requests_change_percent": data["changes_percent"]["allocation_requests"], "prepared_heap_change_bytes": data["allocation"]["candidate"]["live_usable_before"]["median"] - data["allocation"]["baseline"]["live_usable_before"]["median"]}
data["outcome"]["retained"] = False
data["outcome"]["reason"] = "Mixed elapsed (+0.208%), only one of four preparations and 25/64 blocks improve; no established positive gain. Not retained; return to the published baseline and proceed to exact record admission."
data["limitations"] = ["Only complete eight-byte compression; no larger-input throughput claim.",
    "Baseline sampling did not resolve worker symbols (1,244 weights); its categories are unavailable, not evidence of a workload shift. Candidate symbols resolved.",
    "Primary, CPU, allocation and sampling are separate cohorts; hardware reads include uncorrected boundary overhead.",
    "Primary and diagnostic builds share exact per-variant coherent libraries and same-build Object/images; instrumentation and observations remain separate.",
    "Prepared linked executables retain setup/compiler/catalog state; memory is not minimal runtime-only RAM.",
    "Total bytes copied and branch/cache misses are unmeasured; requested capacity is not initialized copying.",
    "Static generated Object and stack are unchanged; only host frame ownership transport changes."]
(out / "scorecard.json").write_text(json.dumps(data, indent=2) + "\n")
with (out / "scorecard.csv").open("w", newline="") as file:
    writer = csv.writer(file)
    writer.writerow(["metric", "reference", "baseline", "candidate", "change_percent", "scope"])
    writer.writerow(["seconds",summary["reference_seconds"],summary["medians"]["before"]["seconds"],summary["medians"]["after"]["seconds"],data["changes_percent"]["primary_seconds"],"four matched primary pairs; common reference"])
    for section, keys in [("cpu", cpu_keys), ("allocation", memory_keys)]:
        for key in keys:
            writer.writerow([key, *[data[section][label][key]["median"] for label in labels],
                             data["changes_percent"].get(key), "separate coherent diagnostic cohorts"])
    for key in ["code_bytes", "stack_bytes"]:
        writer.writerow([key, "not function matched", *[data["static"][label][key] for label in labels[1:]],0,"static generated compressor entry"])
print(json.dumps({"outcome":data["outcome"],"changes_percent":data["changes_percent"],"profiles":{name: value["disjoint_categories"] for name,value in data["profiles"].items()}},indent=2))
