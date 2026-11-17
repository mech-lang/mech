"""Check the same parser binaries on smaller, checked-in source documents."""

import json
from pathlib import Path
from statistics import median
import subprocess
import sys

q = Path(__file__).resolve().parent
study = q / "stale-cache-accounting"
checkouts = q / "native-review-pr/checkout"
output = study / "small-inputs"
output.mkdir()
drivers = {"control": q / "equivalent-vector-phases/parser-stages/control", "accounting": study / "parser"}
orders = [("control", "accounting"), ("accounting", "control")] * 3
inputs = ("tests/fixtures/syntax-source-boundary/executable.mec",
          "tests/fixtures/syntax-source-boundary/resolver-index.mec",
          "src/syntax/tests/fixtures/document/accepted/unicode.mec")
fields = ("tree_sha256", "source_sha256", "parser_steps", "events")
results = {}
for input_name in inputs:
    source = checkouts / input_name
    bucket = output / source.stem
    bucket.mkdir()
    trials = []
    for number, order in enumerate(orders, 1):
        for actor in order:
            dest = bucket / f"{number}-{actor}.json"
            cmd = list(map(str, [drivers[actor], source, dest]))
            subprocess.run(cmd, check=True)
            trials.append(json.loads(dest.read_text()) | {"actor": actor, "command": cmd})
    assert len({tuple(row[field] for field in fields) for row in trials}) == 1
    result = {"trials": trials, "medians_seconds": {
        actor: median(row["parse_seconds"] for row in trials if row["actor"] == actor) for actor in drivers
    }}
    results[input_name] = result
    print(input_name, result["medians_seconds"], flush=True)
(output / "report.json").write_text(json.dumps({"orders": orders,"inputs": results}, indent=2) + "\n")
