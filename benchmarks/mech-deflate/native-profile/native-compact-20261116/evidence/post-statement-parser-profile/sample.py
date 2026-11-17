"""Run the frozen BestChoice diagnostic after the serializer has cleared the host."""
from collections import defaultdict
from pathlib import Path
from statistics import median
import json
import os
import subprocess
import sys

from prepare import checkout, control, digest, document, pin, receipt, source, source_dir, study, verify

identity_fields = ("source_sha256", "source_bytes", "tree_sha256", "parser_steps", "events")
work_fields = (
    "replays", "inclusive_work", "fuel_charges", "rewinds", "node_id_allocations",
    "token_id_allocations", "cache_event_and_byte_work", "live_output_events",
    "matched", "committed", "halted", "no_match", "clean",
    "provisional_candidates", "provisional_work", "max_candidate_bytes",
    "max_candidate_events", "warm_origins", "historical_peaks_raised", "peak_budget_shortfall",
    "different_origin", "different_work", "different_effects",
)
trial_fields = ("halted_trial", "no_match_trial", "committed_trial", "matched_trial", "without_replay", "with_replay")
root_fields = (
    "grammar_steps", "child_work", "peak_grammar_frames", "fuel_remaining", "checkpoint_rewinds",
    "rewound_bytes", "peak_events", "peak_open_markers", "peak_rule_depth",
    "next_node_before_final_sink", "next_token_before_final_sink",
)
rounds = (("control", "time"), ("time", "control"), ("time", "control"), ("control", "time"))
env = os.environ | {"CARGO_MANIFEST_DIR": str(checkout / "src/syntax"), "CARGO_PKG_VERSION": "0.3.5"}


def write(path, data):
    path.write_text(json.dumps(data, indent=2) + "\n")


def run(args, log):
    command = list(map(str, args))
    with log.open("wb") as output:
        subprocess.run(command, cwd=checkout, env=env, stdout=output, stderr=subprocess.STDOUT, check=True)
    return command


def build(mode):
    out = study / mode
    out.mkdir(exist_ok=False)
    library = out / "libmech_syntax.rlib"
    binary = out / "parser"
    build_args = [
        str(source_dir / "lib.rs") if arg.endswith("/candidate/source/lib.rs")
        else str(library) if arg.endswith("/candidate/libmech_syntax.rlib")
        else f"metadata=best-choice-{mode}" if arg == "metadata=batch-journal-candidate"
        else arg
        for arg in receipt["build"]
    ] + ["--cfg", f'best_choice_probe="{mode}"']
    link_args = [
        f"mech_syntax={library}" if arg.startswith("mech_syntax=")
        else str(binary) if arg.endswith("/candidate/parser")
        else arg
        for arg in receipt["link"]
    ]
    assert str(source_dir / "lib.rs") in build_args and str(library) in build_args
    assert str(binary) in link_args
    build_args = run(build_args, out / "build.log")
    link_args = run(link_args, out / "link.log")
    write(out / "build.json", {
        "pin": pin,
        "source_blobs_before": verify(),
        "diagnostic_overlay": {
            str(path): digest(source_dir / document / path)
            for path in ("mod.rs", "continuation.rs", "best_choice_probe.rs")
        },
        "library_sha256": digest(library),
        "binary_sha256": digest(binary),
        "build": build_args,
        "link": link_args,
    })


def verify_build(mode):
    out = study / mode
    binary = out / "parser"
    binding = json.loads((out / "build.json").read_text())
    assert binding["pin"] == pin
    assert binding["binary_sha256"] == digest(binary)
    assert binding["library_sha256"] == digest(out / "libmech_syntax.rlib")
    assert all(digest(source_dir / document / path) == h for path, h in binding["diagnostic_overlay"].items())
    return binary


def sample(binary, out, label):
    metrics = out / f"{label}.json"
    logs = out / f"{label}.log"
    command = run([binary, source, metrics], logs)
    tree = json.loads(metrics.read_text())
    assert all(tree[key] == receipt["exact_tree_and_work"][key] for key in identity_fields), label
    out = {
        "probe": command,
        "observable": tree,
    }
    for line in logs.read_text().splitlines():
        key, *values = line.split()
        if key == "best-choice-clock":
            assert "grammar" not in out
            out["grammar"] = dict(zip(("window_ns", "outside_ns"), map(int, values), strict=True))
        elif key == "best-choice-time":
            owner, alternate, *counts = values
            rows = out.setdefault("timed_replays", {})
            label = f"{owner}/{alternate}"
            assert label not in rows
            rows[label] = dict(zip(("exclusive_ns", "outermost_ns", "inclusive_union_ns"), map(int, counts), strict=True))
        elif key == "best-choice-work":
            owner, alternate, *counts = values
            rows = out.setdefault("counted_replays", {})
            label = f"{owner}/{alternate}"
            assert label not in rows
            rows[label] = dict(zip(work_fields, map(int, counts), strict=True))
        elif key == "best-choice-trials":
            owner, *counts = values
            rows = out.setdefault("counted_trials", {})
            assert owner not in rows
            rows[owner] = dict(zip(trial_fields, map(int, counts), strict=True))
        elif key == "post-statement-profile":
            phase, *spans = values
            assert phase in ("root", "finish") and len(spans) == 4
            buckets = out.setdefault("document_phase_ns", {})
            assert phase not in buckets
            buckets[phase] = list(map(int, spans))
        elif key == "best-choice-root":
            assert "root_work" not in out
            out["root_work"] = dict(zip(root_fields, map(int, values), strict=True))
        else:
            raise AssertionError(line)
    return out


def time():
    verify()
    binary = verify_build("time")
    out = study / "time-runs"
    out.mkdir(exist_ok=False)
    report = []
    controls = []
    for index, roles in enumerate(rounds, 1):
        for role in roles:
            result = sample(binary if role == "time" else control, out, f"{index}-{role}")
            if role == "time":
                rows = result["timed_replays"]
                exclusive = sum(row["exclusive_ns"] for row in rows.values())
                outermost = sum(row["outermost_ns"] for row in rows.values())
                assert all(row["inclusive_union_ns"] >= max(row["exclusive_ns"], row["outermost_ns"]) for row in rows.values())
                assert exclusive == outermost
                accounted = result["grammar"]["outside_ns"] + exclusive
                assert 0 < accounted <= result["grammar"]["window_ns"]
                result["grammar"]["covered_ns"] = accounted
                result["grammar"]["replay_ns"] = exclusive
                report.append(result)
            else:
                assert "grammar" not in result and "root_work" not in result
                controls.append(result)
    write(out / "report.json", {"pin": pin, "rounds": rounds, "control": controls, "time": report})


def census():
    verify()
    binary = verify_build("census")
    out = study / "count-runs"
    out.mkdir(exist_ok=False)
    rows = sample(binary, out, "census")
    total_by_owner = defaultdict(int)
    for family, record in rows["counted_replays"].items():
        owner, _ = family.split("/")
        total_by_owner[owner] += record["replays"]
        assert sum(record[name] for name in ("matched", "committed", "halted", "no_match")) == record["replays"]
        assert record["provisional_candidates"] <= record["clean"] <= record["matched"]
    for owner, record in rows["counted_trials"].items():
        assert total_by_owner[owner] == record["with_replay"]
    assert receipt["exact_tree_and_work"]["parser_steps"] + rows["root_work"]["fuel_remaining"] == 4_000_000
    write(out / "report.json", {"pin": pin, **rows})


def summarize():
    time_rows = json.loads((study / "time-runs/report.json").read_text())
    counts = json.loads((study / "count-runs/report.json").read_text())
    keys = {key for row in time_rows["time"] for key in row["timed_replays"]}
    assert keys == counts["counted_replays"].keys()
    detailed = {}
    min_records = len(time_rows["time"])
    for key in sorted(keys):
        observed = [row["timed_replays"][key] for row in time_rows["time"]]
        assert len(observed) == min_records
        detailed[key] = {
            **{k: median(row[k] for row in observed) for k in observed[0]},
            **counts["counted_replays"][key],
        }
    result = {
        "pin": pin,
        "control_probe_median_s": median(row["observable"]["parse_seconds"] for row in time_rows["control"]),
        "timed_probe_median_s": median(row["observable"]["parse_seconds"] for row in time_rows["time"]),
        "census_probe_s": counts["observable"]["parse_seconds"],
        **{k: median(row["grammar"][k] for row in time_rows["time"]) for k in time_rows["time"][0]["grammar"]},
        "families": dict(sorted(detailed.items(), key=lambda item: item[1]["exclusive_ns"], reverse=True)),
        "trials": counts["counted_trials"],
        "root": counts["root_work"],
    }
    write(study / "report.json", result)
    print(json.dumps(result, indent=2), flush=True)


if __name__ == "__main__":
    command, *args = sys.argv[1:]
    if command == "build":
        mode, = args
        assert mode in ("time", "census")
        build(mode)
    else:
        {"time": time, "census": census, "report": summarize}[command]()
