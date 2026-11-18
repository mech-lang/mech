"""One bounded same-worker study of the recorded periodic-case timing drift."""
from argparse import ArgumentParser
from hashlib import file_digest
import json
import os
from pathlib import Path
import subprocess
import sys
from time import time_ns

ROOT = Path(__file__).resolve().parent
Q = ROOT.parents[1]
RETAINED = Q / "backend-work-hardened/worker-checks/retained"
TIMER = Q / "compare-library.py"
FIXTURES = Q / "fixtures.json"
PEERS = Q / "pinned-peers/pins.json"
NAMES = (
    "prefix-periodic-1024", "prefix-periodic-4096", "prefix-periodic-8192", "prefix-periodic-16384",
    "prefix-periodic-32767", "prefix-natural-4096", "prefix-natural-8192", "prefix-natural-16384",
    "random-1024", "prefix-random-4096", "heldout-random-65537",
)


def sha(path):
    with Path(path).open("rb") as f:
        return file_digest(f, "sha256").hexdigest()


def record(path, body):
    with path.open("x") as f:
        json.dump(body, f, indent=2)
        f.write("\n")


def conditions():
    data = {"utc_ns": time_ns(), "load_average": os.getloadavg()}
    for name in ("therm", "batt"):
        command = ["/usr/bin/pmset", "-g", name]
        run = subprocess.run(command, capture_output=True, text=True, check=False, timeout=5)
        data[name] = {"exit": run.returncode, "stdout": run.stdout, "stderr": run.stderr}
    return data


def main():
    cli = ArgumentParser(description=__doc__)
    cli.add_argument("action", choices=("prepare", "acquire"))
    args = cli.parse_args()
    plan_file = ROOT / "plan.json"
    active = ROOT / "sentinel-timer.py"
    if args.action == "prepare":
        case_names = {item["name"] for item in json.loads(FIXTURES.read_bytes())}
        assert len(case_names) == 81 and set(NAMES) <= case_names
        origin = TIMER.read_text()
        marker = "selected = {case['name'] for case in cases}"
        assert origin.count(marker) == 1
        # Preserve index-based seeds and original named order by filtering at the point of use.
        with active.open("x") as f:
            f.write(origin.replace(marker, f"selected = set({NAMES!r})"))
        p = json.loads(PEERS.read_bytes())
        for name in ("roc", "libdeflate"):
            row = p["actors"][name]
            assert sha(row["command"][0]) == row["binary_sha256"]
        assert sha(FIXTURES) == p["input_inventory_sha256"]
        worker = sha(RETAINED / "mech-worker")
        assert worker == "674d88602946ae07bbb00ccb56a60e2a5353f73b5e2d3b5db9a2b6b98ac237c9"
        record(plan_file, {
            "scope": "Same fixed reference on both actor slots. Partial diagnostic for recorded drift.",
            "cases": NAMES, "time_block_minutes": [60, 90],
            "repetitions": 1, "blocks_per_case": 96, "orders": ("forward", "reverse"),
            "inputs_sha256": sha(FIXTURES), "peer_pins_sha256": sha(PEERS),
            "original_timer_sha256": sha(TIMER), "scoped_timer_sha256": sha(active),
            "worker_sha256": worker, "metadata_sha256": sha(RETAINED / "metadata.json"),
            "hypothesis": "The large periodic drift came from transient host or acquisition behavior.",
            "comparison": "Per-actor absolute and paired timing for the same worker and fixed C peer "
                          "against the archived historical, frame-copy, and owner-exchange orders.",
            "capture": "Pre/post order workload and thermal state where readable. Both raw orders are retained.",
        })
        print(sha(plan_file))
        return
    plan = json.loads(plan_file.read_bytes())
    assert sha(TIMER) == plan["original_timer_sha256"] and sha(active) == plan["scoped_timer_sha256"]
    assert sha(FIXTURES) == plan["inputs_sha256"] and sha(PEERS) == plan["peer_pins_sha256"]
    assert sha(RETAINED / "mech-worker") == plan["worker_sha256"]
    assert sha(RETAINED / "metadata.json") == plan["metadata_sha256"]
    source = active.read_bytes()
    for order in plan["orders"]:
        role = "fixed-reference-drift-" + order
        record(ROOT / (order + "-before.json"), conditions())
        try:
            sys.argv = [str(TIMER), role, str(RETAINED), str(RETAINED)]
            if order == "reverse":
                sys.argv.append("reverse")
            exec(compile(source, str(active), "exec"), {"__name__": "__main__", "__file__": str(TIMER)})
        finally:
            record(ROOT / (order + "-after.json"), conditions())


if __name__ == "__main__":
    main()
