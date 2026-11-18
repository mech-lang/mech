"""Pair two independent parser changes with the same published control."""

from collections import Counter
import json
import os
from pathlib import Path
import platform
from statistics import median
import subprocess
import sys
import time

stale_study = Path("/private/tmp/mech-stale-token-work.k695fls4")
index_study = Path("/private/tmp/mech-dense-index.5noolrp2")
sys.path.insert(0, str(stale_study))
from study import check, control, recorded, sha, source, write

actors = {
    "control": control / "parser",
    "page_index": index_study / "candidate/parser",
    "stale_token": stale_study / "candidate/parser",
}
receipts = {
    "control": recorded,
    "page_index": json.loads((index_study / "build.json").read_text()),
    "stale_token": json.loads((stale_study / "build.json").read_text()),
}
out = Path(__file__).resolve().parent / "matched-candidate-timings"
orders = [
    ("control", "page_index", "stale_token"),
    ("page_index", "stale_token", "control"),
    ("stale_token", "control", "page_index"),
    ("control", "stale_token", "page_index"),
    ("stale_token", "page_index", "control"),
    ("page_index", "control", "stale_token"),
]
assert all(Counter(column) == Counter(dict.fromkeys(actors, 2)) for column in zip(*orders))
assert len(set(orders)) == 6
bindings = {role: sha(binary) for role, binary in actors.items()}
assert all(bindings[role] == receipt["probe_sha256"] for role, receipt in receipts.items())
out.mkdir()
write(out / "plan.json", {
    "scope": "Standalone canonical parser with fixed source, pinned Rust opt-level 3 and mimalloc.",
    "host": {"platform": platform.platform(), "architecture": platform.machine()},
    "source_revision": "b72b7419b39b22765fe1eed6b5c636e63964b905",
    "input": str(source), "input_sha256": sha(source),
    "probe_sha256": bindings,
    "candidate_source_patch_sha256": {
        role: receipts[role]["source_patch_sha256"] for role in actors if role != "control"
    },
    "orders": orders,
    "excluded_warmups": 1,
})


def sample(role, label):
    output = out / f"{label}-{role}.json"
    command = list(map(str, [actors[role], source, output]))
    with output.with_suffix(".log").open("wb") as log:
        start = time.perf_counter()
        with subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT) as worker:
            _, status, usage = os.wait4(worker.pid, 0)
            worker.returncode = os.waitstatus_to_exitcode(status)
            assert worker.returncode == 0, (role, label)
        elapsed = time.perf_counter() - start
    row = json.loads(output.read_text())
    check(row)
    print(role, label, row["parse_seconds"], usage.ru_maxrss, flush=True)
    return {
        "role": role, "command": command, "wall_time_seconds": elapsed,
        "peak_rss_bytes": usage.ru_maxrss, **row,
    }


warmups = {role: sample(role, "warmup") for role in actors}
trials = [{role: sample(role, f"order-{index}") for role in order}
          for index, order in enumerate(orders, 1)]
results = {}
for role in actors:
    values = {
        "median_parse_seconds": median(row[role]["parse_seconds"] for row in trials),
        "median_peak_rss_bytes": median(row[role]["peak_rss_bytes"] for row in trials),
    }
    if role != "control":
        ratios = [row[role]["parse_seconds"] / row["control"]["parse_seconds"] for row in trials]
        values |= {
            "paired_ratios": ratios,
            "median_paired_ratio": median(ratios),
            "faster_pairs": sum(ratio < 1 for ratio in ratios),
        }
    results[role] = values
write(out / "report.json", {"results": results, "warmups": warmups, "trials": trials, "orders": orders})
print(json.dumps(results, indent=2), flush=True)
