"""Serial diagnostic acquisition. Never changes the selected latency worker."""
import argparse
from hashlib import sha256
import json
import os
from pathlib import Path
import statistics
import subprocess

CASES = ["prefix-natural-1024", "prefix-natural-4096", "prefix-natural-8192",
         "prefix-random-4096", "prefix-random-16384", "prefix-random-65536",
         "prefix-random-1048576", "16384-distribution-change", "320-overlap"]


def digest(path):
    return sha256(Path(path).read_bytes()).hexdigest()


def validate_events(value):
    events, counts = value["events"], value["counts"]
    live, signature, peak, total, zeroed = {}, [], 0, 0, 0
    live_bytes = 0
    for event in events:
        key, size = event["pointer"], event["bytes"]
        assert event["total_ns"] >= event["payload_ns"] >= 0
        signature.append({k: v for k, v in event.items() if k not in ("pointer", "total_ns", "payload_ns")})
        if event["op"] == "allocate":
            assert key not in live
            live[key] = event
            live_bytes += size
            peak = max(peak, live_bytes)
            total += size
            zeroed += size if event["zeroed"] else 0
        else:
            previous = live.pop(key)
            assert previous["bytes"] == size and previous["mapped"] == event["mapped"]
            live_bytes -= size
    assert not live and live_bytes == 0
    assert sum(e["op"] == "transfer" for e in events) == 1
    assert sum(e["op"] == "allocate" for e in events) == counts["payload_allocation_requests"]
    assert total == counts["requested_payload_bytes"]
    assert zeroed == counts["requested_zero_initialized_bytes"]
    assert peak == counts["peak_payload_bytes"]
    return signature


def summarize(values):
    measured = [v for v in values if not v["warmup"]]
    assert measured
    result = {"samples": len(measured), "warmup_samples": len(values) - len(measured),
        "median_phases_ns": {k: statistics.median(v["phases_ns"][k] for v in measured)
            for k in measured[0]["phases_ns"]},
        "median_vm_delta": {k: statistics.median(v["vm_delta"][k] for v in measured)
            for k in measured[0]["vm_delta"]}, "counts": measured[0]["counts"]}
    assert all(v["counts"] == result["counts"] for v in values)
    if measured[0]["events"]:
        groups = {(e["op"], e["mapped"]) for e in measured[0]["events"]}
        result["median_runtime_groups"] = {op + ("_mapped" if mapped else "_allocator"):
            {"calls": sum(e["op"] == op and e["mapped"] == mapped for e in measured[0]["events"]),
             **{field: statistics.median(sum(e[field] for e in v["events"]
                if e["op"] == op and e["mapped"] == mapped) for v in measured)
                for field in ("bytes", "total_ns", "payload_ns")}}
            for op, mapped in sorted(groups)}
    return result


def main():
    parser = argparse.ArgumentParser(__doc__)
    parser.add_argument("config", type=Path)
    parser.add_argument("diagnostics", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--samples", type=int, default=32)
    parser.add_argument("--cases", nargs="*", default=CASES)
    parser.add_argument("--orders", nargs="*", default=["forward", "reverse"])
    args = parser.parse_args()
    config = json.loads(args.config.read_text())
    build = json.loads((args.diagnostics / "build.json").read_text())
    assert build["complete"]
    assert all(digest(path) == pin for path, pin in config["inputs_sha256"].items())
    assert all(digest(v["worker"]) == v["worker_sha256"] for v in build["roles"].values())
    fixtures = {c["name"]: c for c in json.loads(Path(config["fixtures"]).read_text())}
    oracle = {c["case"]: c for c in json.loads(Path(config["C_oracle"]).read_text())}
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    plan = {"config_sha256": digest(args.config), "build_sha256": digest(args.diagnostics / "build.json"),
        "controller_sha256": digest(__file__), "samples": args.samples,
        "minimum_warmup_seconds": 0.2, "retained_warmup_records": 10,
        "cases": args.cases, "orders": args.orders, "load_before": os.getloadavg(),
        "scope": "Separate diagnostic hosts with coarse clocks and task-event snapshots; runtime event clocks only in mech-clocks. Not win qualification or aligned Mech/C semantic phase times. All actors serial; fresh heap/compressor construction through cleanup, returned output destruction excluded."}
    (out / "plan.json").write_text(json.dumps(plan, indent=2) + "\n")
    rows, signatures = [], {}
    for order in args.orders:
        case_order = args.cases if order == "forward" else list(reversed(args.cases))
        for index, name in enumerate(case_order):
            actors = ["mech-control", "mech-counts", "mech-clocks", "libdeflate"]
            if order == "reverse":
                actors.reverse()
            offset = index % len(actors)
            actors = actors[offset:] + actors[:offset]
            for actor in actors:
                folder = out / order / name / actor
                folder.mkdir(parents=True)
                worker = build["roles"]["mech-control" if actor == "libdeflate" else actor]["worker"]
                command = [worker, actor, config["metadata"], config["fixtures"], name,
                           str(args.samples), str(folder / "output.bin")]
                with (folder / "stdout.jsonl").open("w") as output, (folder / "stderr.log").open("w") as error:
                    subprocess.run(command, stdout=output, stderr=error, check=True)
                values = [json.loads(line) for line in (folder / "stdout.jsonl").read_text().splitlines()]
                calibration, values = values[0], values[1:]
                assert len(values) == args.samples + 10
                expected_hash = oracle[name]["output_sha256"] if actor == "libdeflate" else fixtures[name]["expected_sha256"]
                assert digest(folder / "output.bin") == expected_hash
                for value in values:
                    assert value["case"] == name and value["actor"] == actor
                    assert all(value[k] for k in ("exact_stable_bytes", "independent_inflation", "borrowed_aliases_unchanged"))
                    if actor.startswith("mech-"):
                        assert value["counts"]["nonreturned_private_bytes_at_return"] == 0
                    if actor in ("mech-counts", "mech-clocks"):
                        signature = validate_events(value)
                        if actor == "mech-counts":
                            assert all(e["total_ns"] == e["payload_ns"] == 0 for e in value["events"])
                        if name in signatures:
                            assert signatures[name] == signature
                        signatures[name] = signature
                row = {"order": order, "case": name, "actor": actor,
                    "folder": str(folder.relative_to(out)), "command": command,
                    "stdout_sha256": digest(folder / "stdout.jsonl"), "output_sha256": expected_hash,
                    "calibration_median_ns": statistics.median(calibration["calibration_empty_clock_ns"]),
                    "page_bytes": calibration["page_bytes"], **summarize(values)}
                rows.append(row)
                (out / "summary.json").write_text(json.dumps({"plan": plan, "rows": rows,
                    "complete": False}, indent=2) + "\n")
                print(order, name, actor, row["median_phases_ns"]["total"], "ns", flush=True)
    result = {"plan": plan, "rows": rows, "event_signatures": signatures,
        "load_after": os.getloadavg(), "complete": True}
    (out / "summary.json").write_text(json.dumps(result, indent=2) + "\n")
    print("Complete", len(rows), "diagnostic batches", flush=True)


if __name__ == "__main__":
    main()
