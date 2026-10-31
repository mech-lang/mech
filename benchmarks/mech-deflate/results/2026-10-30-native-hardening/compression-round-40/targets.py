"""Classify the previously executed baseline's frame projections; no execution."""
from pathlib import Path
from collections import Counter
import hashlib
import json
import re
import sys

root = Path(sys.argv[1])
out = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(__file__).resolve().parent
out.mkdir(parents=True, exist_ok=True)
paths = {name: root / "analysis-inputs" / name for name in ["baseline-census.json", "baseline-descriptor.json"]}
census = json.loads(paths["baseline-census.json"].read_text())
wire = json.loads(paths["baseline-descriptor.json"].read_text())["descriptor"]
assert census["source_base"] == "552e537fcbcfc3811ce442c982010b5e1e999e6c"
assert census["source_artifact_sha256"] == "a2a551911949f8c5fdc64273a43662c2a9c4953b853136c19d196692fdac45a7"
categories, widths = Counter(), Counter()
eligible = {}
for row in census["rows"]:
    if not row["frame"] or not row["frame"].startswith("Project"):
        continue
    match = re.fullmatch(r"Project \{ source: NativeMatchValue\((\d+)\), ordinal: (\d+), output: NativeMatchValue\((\d+)\) \}", row["frame"])
    assert match, row
    source, ordinal, output = map(int, match.groups())
    value = wire["plan"]["values"][output]
    body = value["definition"]["body"]
    matrix = body.get("Matrix") if isinstance(body, dict) else None
    width = matrix["element"].get("UnsignedInteger") if matrix else None
    if value["scalar"] is not None:
        category = "scalar"
    elif width in ["W8", "W16", "W32", "W64"]:
        category = "managed_unsigned_matrix"
        widths[width] += row["admits"]
        if row["action"].startswith("Structural"):
            assert sum(helper["count"] for helper in row["helpers"]) == row["admits"]
            eligible[row["site"]] = {"source": source, "ordinal": ordinal, "output": output,
                                      "iteration": row["iteration"], "visits": row["admits"], "width": width}
    else:
        category = "other_managed_or_composite"
    categories[category] += row["admits"]
runs = []
for site, row in sorted(eligible.items()):
    previous = site - 1
    if runs and previous in eligible and runs[-1]["sites"][-1] == previous:
        prior = eligible[previous]
        term = wire["plan"]["blocks"][previous]["terminator"]
        if prior["source"] == row["source"] and prior["iteration"] == row["iteration"] and term == {"Scalar": {"Jump": site}}:
            runs[-1]["sites"].append(site)
            runs[-1]["ordinals"].append(row["ordinal"])
            runs[-1]["visits"].append(row["visits"])
            continue
    runs.append({"source": row["source"], "iteration": row["iteration"], "sites": [site],
                 "ordinals": [row["ordinal"]], "visits": [row["visits"]]})
runs = [run for run in runs if len(run["sites"]) >= 2]
for run in runs:
    assert len(set(run["visits"])) == 1, run
    run["maximum_per_field_visits_combined"] = (len(run["sites"]) - 1) * run["visits"][0]
data = {"baseline_compiler": census["source_base"], "source_pins": {name: hashlib.sha256(path.read_bytes()).hexdigest() for name, path in paths.items()},
        "project_visits_by_kind": dict(categories), "managed_unsigned_visits_by_width": dict(widths),
        "linear_structural_managed_runs": len(runs),
        "maximum_per_field_admissions_combined": sum(run["maximum_per_field_visits_combined"] for run in runs),
        "maximum_per_field_helpers_combined": sum(run["maximum_per_field_visits_combined"] for run in runs),
        "largest_runs": sorted(runs, key=lambda run: run["maximum_per_field_visits_combined"], reverse=True)[:30],
        "scope": "Read-only analysis of the already executed baseline census and same-build image, not a new timing or selector proof. Runs require further entry/predecessor, liveness, resource, alias, error-order, cleanup and retry acceptance before any bulk transport implementation. The count is an opportunity ceiling, not a promised speedup."}
(out / "targets.json").write_text(json.dumps(data, indent=2) + "\n")
print(json.dumps({key: value for key, value in data.items() if key not in ["largest_runs", "source_pins"]}, indent=2))
