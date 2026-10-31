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


data = {"schema": "mech.prepared-record-selector.v1", "chosen": "candidate",
        "stack_base": "eacb00fcb3352fbbf3d4dddb7b983f4c130f4a7b",
        "published_start": "5083c1732a15852aa7017d1c9d2685082feba2ef",
        "baseline_compiler": "552e537fcbcfc3811ce442c982010b5e1e999e6c",
        "candidate_implementation": "326576c01541820f3f98839880b90ba4e1e4edb4",
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
assert summary["comparisons"]["after_over_before"]["improving_groups"] == 4
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
        pins = json.loads((root / "diagnostic-libs" / name / "image/pins.json").read_text())
        data.setdefault("diagnostic_image_pins", {})[label] = pins
assert data["static"]["baseline"]["object_sha256"] == data["static"]["candidate"]["object_sha256"]
data["changes_percent"] = {"primary_seconds":100 * (summary["comparisons"]["after_over_before"]["pooled_ratio"] - 1)}
for section, keys in [("cpu", cpu_keys), ("allocation", memory_keys)]:
    for key in keys:
        base = data[section]["baseline"][key]["median"]
        after = data[section]["candidate"][key]["median"]
        data["changes_percent"][key] = 100 * (after / base - 1) if base else None
data["diagnostic_scope"] = "Matched optimized coherent Compute/GPU rebuilds of the same production sources/features, original frozen Engine libraries, fresh same-build Object/images. Both emitted Objects match their primary Objects byte-for-byte. Original primary workers are unchanged. Original eight-library freezing omitted a required transitive Compute artifact, so these diagnostics use separately recorded coherent rebuilds; an after-only original-graph link remains unavailable."
data["profile_scope"] = "Separate serial 695-call original primary-worker main-thread samples include surrounding harness. Disjoint weights are not precise compression-timer percentages. Only isolated-profile is used; an earlier overlapped pair is excluded."
data["outcome"] = "Retain the 0.90895% matched elapsed reduction (all four preparations, 54/64 descriptive blocks improve). Diagnostic cycles fall 0.416%, instructions rise 1.046%, compression allocations and incremental tracked peak are unchanged, prepared tracked heap rises 1,110,160 bytes. No parity or larger-input result."
data["limitations"] = ["Only complete eight-byte compression; no larger-input throughput claim.",
    "Primary, CPU, allocation and sampling are separate cohorts; hardware reads include uncorrected boundary overhead.",
    "Diagnostic coherent library rebuild configuration is distinct from the frozen primary binaries and explicitly pinned.",
    "Prepared linked executables retain setup/compiler/catalog state; memory is not minimal runtime-only RAM.",
    "Total bytes copied and branch/cache misses are unmeasured; requested capacity is not initialized copying.",
    "Static generated Object and stack are unchanged; host code changes and prepared metadata account for this candidate."]
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
