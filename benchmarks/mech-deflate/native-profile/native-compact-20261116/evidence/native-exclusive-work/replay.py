"""Check exact artifacts and report per-thread exclusive compiler elapsed time."""

import argparse
from collections import defaultdict
from hashlib import file_digest
import json
from pathlib import Path


ARTIFACTS = ("program.mecb", "program.o", "metadata.json", "mech-worker")
FACTS = ("graph", "definitions", "read roles", "fusion reads", "semantic actions", "use counts",
         "scalar aggregates", "direct action outputs")
BACKEND_STAGES = {"cranelift_optimize", "cranelift_lower_codegen"}


def digest(path):
    with path.open("rb") as stream:
        return file_digest(stream, "sha256").hexdigest()


def checked_artifacts(old, shared, fixed, fixed_bytecode):
    result = {}
    for name in ARTIFACTS:
        path = fixed_bytecode if name == "program.mecb" else fixed / name
        expected = digest(path)
        for folder in (old, shared):
            assert digest(folder / name) == expected, (folder, name)
        result[name] = expected
    return result


def duration(event):
    return event["end_ns"] - event["start_ns"]


def token():
    return {"count": 0, "inclusive_ns": 0, "self_ns": 0}


def add(row, net, total):
    row["count"] += 1
    row["inclusive_ns"] += total
    row["self_ns"] += net


def summarize_thread(thread, native_bounds):
    events = thread["events"]
    assert events and events[0]["parent"] is None
    child_ns = defaultdict(int)
    last_end = {}
    for idx, event in enumerate(events):
        start, end = event["start_ns"], event["end_ns"]
        assert type(start) is int and type(end) is int and native_bounds[0] <= start <= end <= native_bounds[1]
        parent = event["parent"]
        if parent is None:
            assert idx == 0, (thread["thread"], idx)
            continue
        assert type(parent) is int and 0 <= parent < idx
        scope = events[parent]
        assert scope["start_ns"] <= start <= end <= scope["end_ns"]
        assert last_end.get(parent, scope["start_ns"]) <= start
        inherited_job = scope["job"]
        if inherited_job is not None:
            assert inherited_job == event["job"]
        last_end[parent] = end
        child_ns[parent] += end - start

    exclusive = defaultdict(int)
    cranelift = defaultdict(token)
    facts = {key: token() for key in FACTS}
    detail = defaultdict(token)
    assigned = []
    for idx, event in enumerate(events):
        total = duration(event)
        own = total - child_ns[idx]
        assert own >= 0, (thread["thread"], idx)
        category = event["category"]
        stage = category
        if category == "cranelift_pass":
            parent = event["parent"]
            stage = assigned[parent] if parent is not None else "cranelift_other"
            if stage not in BACKEND_STAGES and stage != "cfg_cleanup":
                stage = "cranelift_other"
        assigned.append(stage)
        exclusive[stage] += own
        add(detail[(category, event["detail"])], own, total)
        if category in BACKEND_STAGES or category == "cranelift_pass":
            add(cranelift[event["detail"]], own, total)
        if category == "immutable_facts":
            add(facts[event["detail"]], own, total)
    assert sum(exclusive.values()) == duration(events[0])
    return {"elapsed_ns": duration(events[0]), "exclusive_ns": dict(exclusive), "facts": facts,
            "cranelift_passes": dict(cranelift),
            "details": [{"category": k[0], "detail": k[1], **v} for k, v in sorted(detail.items())]}


def event_union(events):
    intervals = sorted((event["start_ns"], event["end_ns"]) for event in events)
    union = []
    for start, end in intervals:
        if union and start <= union[-1][1]:
            union[-1] = union[-1][0], max(end, union[-1][1])
        else:
            union.append((start, end))
    return union


def overlap(a, b):
    return sum(max(0, min(stop_a, stop_b) - max(start_a, start_b))
               for start_a, stop_a in a for start_b, stop_b in b)


def summarize_jobs(traces):
    jobs = defaultdict(dict)
    worker_intervals = []
    waits = []
    kinds = {("coordinator_wait", "enqueue"): "submit", ("coordinator_wait", "receive completion"): "receive"}
    for thread, events in traces.items():
        for event in events:
            category = event["category"]
            job = event["job"]
            kind = kinds.get((category, event["detail"]))
            if category == "coordinator_wait":
                waits.append(event)
            elif category == "worker_compute":
                kind = "work"
                worker_intervals.append(event)
            elif category == "coordination" and event["detail"] == "worker reply":
                kind = "reply"
            if kind is not None:
                assert job is not None and kind not in jobs[job]
                jobs[job][kind] = event | {"thread": thread}
    assert len(worker_intervals) == len(jobs)
    for number, job in jobs.items():
        assert {"submit", "work", "reply", "receive"} == job.keys(), number
        submit, work, reply, receive = [job[key] for key in ("submit", "work", "reply", "receive")]
        assert submit["thread"] == receive["thread"] == "coordinator"
        assert work["thread"] == reply["thread"] != "coordinator"
        assert submit["start_ns"] <= work["start_ns"] <= work["end_ns"] <= reply["start_ns"] <= receive["end_ns"]
        assert submit["end_ns"] <= receive["start_ns"]
        job["queue_delay_ns_lower"] = max(0, work["start_ns"] - submit["end_ns"])
        job["queue_delay_ns_upper"] = work["start_ns"] - submit["start_ns"]
    active = event_union(worker_intervals)
    waiting = event_union(waits)
    return {"jobs": dict(sorted(jobs.items())), "worker_active_union_ns": sum(end - start for start, end in active),
            "worker_sum_elapsed_ns": sum(map(duration, worker_intervals)),
            "coordinator_wait_union_ns": sum(end - start for start, end in waiting),
            "coordinator_wait_overlap_worker_ns": overlap(active, waiting)}


def report(folder):
    path = folder / "native-exclusive.json"
    traces = json.loads(path.read_text())
    by_name = {row["thread"]: row["events"] for row in traces}
    assert len(by_name) == len(traces) == 3 and set(by_name) == {"coordinator", "worker-0", "worker-1"}
    native = by_name["coordinator"][0]
    assert native["category"] == "native"
    native_bounds = native["start_ns"], native["end_ns"]
    result = {row["thread"]: summarize_thread(row, native_bounds) for row in traces}
    compile_receipt = folder / "compile.json"
    receipt = json.loads(compile_receipt.read_text())
    native_ns = int(next(row["seconds"] for row in receipt["phases"] if row["phase"] == "native") * 1e9)
    assert receipt["mode"] == "compact" and 0 <= native_ns - duration(native)
    return {"native_wall_elapsed_ns": duration(native), "compile_receipt_native_ns": native_ns,
            "native_unspanned_boundary_ns": native_ns - duration(native),
            "trace_sha256": digest(path), "compile_receipt_sha256": digest(compile_receipt),
            "threads": result, **summarize_jobs(by_name)}


def bindings(old, shared):
    result = {}
    specs = []
    for role, folder in (("old", old), ("shared", shared)):
        path = folder / "capture-inputs.json"
        spec = json.loads(path.read_text())
        assert spec["role"] == role
        assert spec["flags"]["MECH_NATIVE_EXCLUSIVE_TRACE"] == str(folder / "native-exclusive.json")
        specs.append(spec)
        result[role] = {"capture_sha256": digest(path), "compiler_sha256": spec["compiler_sha256"]}
    left, right = specs
    for key in ("reference_recipe_sha256", "source_sha256"):
        assert left[key] == right[key], key
    normalized = [{key: value for key, value in spec["flags"].items() if key != "MECH_NATIVE_EXCLUSIVE_TRACE"}
                  for spec in specs]
    assert normalized[0] == normalized[1]
    return result


def fact_delta(old, shared):
    result = {}
    for thread in old["threads"]:
        before = old["threads"][thread]["facts"]
        after = shared["threads"][thread]["facts"]
        result[thread] = {k: {"old": before[k], "shared": after[k],
                               "count_change": after[k]["count"] - before[k]["count"],
                               "self_ns_change": after[k]["self_ns"] - before[k]["self_ns"]} for k in FACTS}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("old", type=Path)
    parser.add_argument("shared", type=Path)
    parser.add_argument("fixed", type=Path)
    parser.add_argument("fixed_bytecode", type=Path)
    options = parser.parse_args()
    old_path, shared_path = options.old.resolve(), options.shared.resolve()
    files = checked_artifacts(old_path, shared_path, options.fixed, options.fixed_bytecode)
    capture = bindings(old_path, shared_path)
    old = report(old_path)
    shared = report(shared_path)
    print(json.dumps({"materialization_delta_by_thread": fact_delta(old, shared),
                      "unchanged_artifacts": files, "captures": capture, "old": old, "shared": shared}, indent=2))


if __name__ == "__main__":
    main()
