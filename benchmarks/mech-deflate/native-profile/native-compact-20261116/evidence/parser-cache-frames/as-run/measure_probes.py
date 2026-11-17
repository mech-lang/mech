"""Compare the parser candidates in balanced orders before source-through-link tests."""

from collections import Counter
import json
import os
from statistics import median
import subprocess
import time

from study_tools import expected, sha, source, study, write

actors = ("before", "candidate")
orders = [list(actors), list(reversed(actors))] * 4
assert all(Counter(column) == Counter({name: 4 for name in actors}) for column in zip(*orders))
out = study / "probe-timings"
out.mkdir()
bins = {name: study / name / "parser" for name in actors}
bindings = {name: sha(exe) for name, exe in bins.items()}
write(out / "plan.json", {"scope":"Standalone parser, fixed source and allocator",
                         "source_sha256":sha(source),"probe_sha256":bindings,"orders":orders,
                         "criterion":"Advance after a lower paired parse median with at least six faster pairs and exact tree, event and step identities."})


def measure(actor, label):
    result = out / f"{label}-{actor}.json"
    command = [bins[actor], source, result]
    with result.with_suffix(".log").open("wb") as log:
        start = time.perf_counter()
        with subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT) as worker:
            _, status, usage = os.wait4(worker.pid, 0)
            worker.returncode = os.waitstatus_to_exitcode(status)
            assert worker.returncode == 0, (actor, label)
        elapsed = time.perf_counter() - start
    row = json.loads(result.read_text())
    for name in ("tree_sha256", "source_sha256", "parser_steps", "events"):
        assert row[name] == expected[name], (actor, label, name)
    print(actor, label, row["parse_seconds"], usage.ru_maxrss, flush=True)
    return {"actor":actor, "command":list(map(str, command)), "wall_time_seconds":elapsed,
            "peak_rss_bytes":usage.ru_maxrss, **row}


warmups = [measure(actor, "warmup") for actor in actors]
timed = [{actor: measure(actor, f"order-{n}") for actor in order} for n, order in enumerate(orders, 1)]
report = {
    "warmups":warmups, "orders":orders, "trials":timed,
    "median_parse_seconds": {actor:median(row[actor]["parse_seconds"] for row in timed) for actor in actors},
    "median_peak_rss_bytes": {actor:median(row[actor]["peak_rss_bytes"] for row in timed) for actor in actors},
    "paired_parse_ratio": {actor: [row[actor]["parse_seconds"]/row["before"]["parse_seconds"] for row in timed]
                           for actor in actors[1:]},
}
write(out / "report.json", report)
print(json.dumps({key:report[key] for key in ("median_parse_seconds", "median_peak_rss_bytes", "paired_parse_ratio")},
                 indent=2))
