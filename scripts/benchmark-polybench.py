#!/usr/bin/env python3
"""Freeze a compiler candidate, measure the fixed split, and append aggregate history.

The Rust checked-call driver and C adapters retain their original timing/cache
boundaries. Run actors sequentially; never run this alongside builds or profiles.
"""
import argparse
import datetime
import hashlib
import json
import math
from pathlib import Path
import platform
import statistics
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "hosts/gpu/tests/fixtures/polybench"


def read(path):
    return json.loads(Path(path).read_text())


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, value):
    Path(path).write_text(json.dumps(value, indent=2, allow_nan=False) + "\n")


def utc():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def command(args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def compiler_fingerprint():
    paths = command(["git", "ls-files", "-co", "--exclude-standard"]).splitlines()
    files = {
        name: digest(ROOT / name)
        for name in sorted(set(paths))
        if Path(name).suffix in (".rs", ".toml") or Path(name).name == "Cargo.lock"
    }
    return hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest()


def protocol():
    spec = read(FIXTURE / "optimization-protocol.json")
    cases = {case["name"]: case for case in read(FIXTURE / "cases.json")["cases"]}
    optimize, holdout = spec["optimization"], spec["holdout"]
    assert len(set(optimize)) == len(optimize) == 15
    assert len(set(holdout)) == len(holdout) == 15
    assert not set(optimize) & set(holdout)
    assert set(optimize + holdout) == set(cases)
    for name, case in cases.items():
        assert digest(FIXTURE / f"{name}.mec") == case["mech_sha256"], name
    return spec, cases


def freeze(args):
    protocol()
    if args.output.exists():
        raise ValueError("a frozen candidate cannot be overwritten")
    write(args.output, {
        "schema": "mech.polybench.frozen-candidate.v1",
        "created_at": utc(), "label": args.label,
        "source_commit": command(["git", "rev-parse", "HEAD"]),
        "compiler_fingerprint": compiler_fingerprint(),
        "driver_sha256": digest(args.driver),
        "reference_driver_sha256": digest(args.reference_driver) if args.reference_driver else None,
        "protocol_sha256": digest(FIXTURE / "optimization-protocol.json"),
        "objective_sha256": digest(FIXTURE / "optimization-objective.json"),
        "fixture_sha256": digest(FIXTURE / "cases.json"),
        "baseline_sha256": digest(FIXTURE / "baseline-m3-mini.json"),
        "runner_sha256": digest(__file__),
        "rustc": command(["rustc", "--version"]),
        "machine": {"platform": platform.platform(), "processor": platform.machine()},
    })
    print(args.output)


def validate(actual, expected):
    if len(actual) != len(expected) or not all(
        isinstance(a, (int, float)) and math.isfinite(a) and math.isfinite(b)
        and math.isclose(a, b, rel_tol=1e-10, abs_tol=1e-9)
        for a, b in zip(actual, expected)
    ):
        raise ValueError("finite C live-out comparison failed")
    return {"count": len(expected), "finite": True,
            "max_absolute_error": max((abs(a - b) for a, b in zip(actual, expected)), default=0)}


def execute(argv, expected):
    started = time.perf_counter()
    process = subprocess.run(list(map(str, argv)), capture_output=True, text=True, timeout=180)
    if process.returncode:
        raise RuntimeError(f"driver exited {process.returncode}: {process.stderr[-2000:]}")
    result = json.loads(process.stdout.strip().splitlines()[-1])
    if result.get("status", "ok") != "ok":
        raise RuntimeError(str(result))
    validation = validate(result["values"], expected)
    sample, = result["samples_seconds"]
    if not math.isfinite(sample) or sample <= 0:
        raise ValueError("nonpositive or nonfinite timing")
    return {"seconds": sample, "process_seconds": time.perf_counter() - started,
            "validation": validation}


def samples_summary(values):
    return {"samples_seconds": values, "median_seconds": statistics.median(values),
            "minimum_seconds": min(values), "maximum_seconds": max(values)}


def run(args):
    spec, cases = protocol()
    frozen = read(args.freeze)
    for key, value in {
        "compiler_fingerprint": compiler_fingerprint(),
        "driver_sha256": digest(args.driver),
        "reference_driver_sha256": digest(args.reference_driver) if args.reference_driver else None,
        "protocol_sha256": digest(FIXTURE / "optimization-protocol.json"),
        "objective_sha256": digest(FIXTURE / "optimization-objective.json"),
        "fixture_sha256": digest(FIXTURE / "cases.json"),
        "baseline_sha256": digest(FIXTURE / "baseline-m3-mini.json"),
        "runner_sha256": digest(__file__),
    }.items():
        if frozen[key] != value:
            raise ValueError(f"candidate changed after freeze: {key}")
    if args.samples < 5:
        raise ValueError("qualification requires at least five samples")
    if args.output.exists():
        raise ValueError("a timing receipt cannot be overwritten")
    baseline = read(FIXTURE / "baseline-m3-mini.json")
    receipt = {"schema": "mech.polybench.partition-timings.v1", "created_at": utc(),
               "partition": args.partition, "freeze_sha256": digest(args.freeze),
               "candidate": frozen, "samples_per_comparison": args.samples,
               "dataset": spec["dataset"], "numeric_profile": spec["numeric_profile"],
               "timing_boundaries": baseline["timing_boundaries"],
               "validation_boundary": baseline["validation_boundary"],
               "baseline_replay": bool(args.reference_driver), "runs": {}, "complete": False}
    if args.reference_driver and digest(args.reference_driver) != baseline["driver_sha256"]:
        raise ValueError("reference executable is not the pinned baseline driver")
    write(args.output, receipt)
    with tempfile.TemporaryDirectory(prefix="mech-polybench-inputs-") as temp:
        for name in spec[args.partition]:
            case = cases[name]
            binary = args.c_build / name
            c_hash = digest(binary)
            if c_hash != baseline["runs"][f"{name}-mini"]["c_binary_sha256"]:
                raise ValueError(f"C executable differs from pinned baseline: {name}")
            inputs = Path(temp) / f"{name}.json"
            write(inputs, case["inputs"])
            source = FIXTURE / f"{name}.mec"
            observations = []
            comparisons = {}
            for mode in ("kernel", "fresh"):
                actors = ["mech", "c"] + (["reference"] if args.reference_driver else [])
                for pair in range(args.samples):
                    # Reverse each pair, then rotate a third actor if present.
                    order = actors if pair % 2 == 0 else actors[::-1]
                    if len(actors) == 3:
                        offset = pair % 3
                        order = order[offset:] + order[:offset]
                    for actor in order:
                        argv = ([binary, "1"] + (["fresh"] if mode == "fresh" else [])
                                if actor == "c" else
                                [args.driver if actor == "mech" else args.reference_driver,
                                 source, inputs, "1"])
                        observation = {"actor": actor, "mode": mode, "pair": pair,
                                       **execute(argv, case["expected"])}
                        observations.append(observation)
                comparison = {
                    actor: samples_summary([row["seconds"] for row in observations
                                            if row["actor"] == actor and row["mode"] == mode])
                    for actor in actors
                }
                comparison["median_mech_over_c"] = (
                    comparison["mech"]["median_seconds"] / comparison["c"]["median_seconds"])
                comparisons[mode] = comparison
            receipt["runs"][name] = {"source_sha256": digest(source),
                                    "c_binary_sha256": c_hash,
                                    "input_sha256": digest(inputs),
                                    "observations": observations, "comparisons": comparisons}
            write(args.output, receipt)
            print(json.dumps({"partition": args.partition, "kernel": name,
                              "mech_over_c": comparisons["kernel"]["median_mech_over_c"]}),
                  flush=True)
    receipt["complete"] = True
    receipt["completed_at"] = utc()
    write(args.output, receipt)


def geomean(values):
    return math.exp(statistics.mean(math.log(value) for value in values))


def aggregate(names, runs, baseline):
    rows = [runs[name] for name in names]
    speedups = [baseline["runs"][f"{name}-mini"]["comparisons"]["kernel"]["mech"]["median_seconds"]
                / runs[name]["comparisons"]["kernel"]["mech"]["median_seconds"] for name in names]
    ratios = [row["comparisons"]["kernel"]["median_mech_over_c"] for row in rows]
    result = {"count": len(names), "geomean_speedup_from_recorded_baseline": geomean(speedups),
              "median_speedup": statistics.median(speedups), "minimum_speedup": min(speedups),
              "maximum_speedup": max(speedups),
              "regressions_over_five_percent": sum(speed < 1 / 1.05 for speed in speedups),
              "geomean_mech_over_c_kernel": geomean(ratios),
              "geomean_mech_over_c_fresh": geomean([
                  row["comparisons"]["fresh"]["median_mech_over_c"] for row in rows]),
              "faster_than_c_kernel": sum(ratio < 1 for ratio in ratios),
              "worst_mech_over_c_kernel": max(ratios),
              "sum_mech_median_seconds": sum(row["comparisons"]["kernel"]["mech"]["median_seconds"]
                                             for row in rows)}
    if all("reference" in row["comparisons"]["kernel"] for row in rows):
        paired = [
            row["comparisons"]["kernel"]["reference"]["median_seconds"]
            / row["comparisons"]["kernel"]["mech"]["median_seconds"] for row in rows]
        result["geomean_speedup_from_contemporaneous_baseline"] = geomean(paired)
        result["contemporaneous_regressions_over_five_percent"] = sum(
            speed < 1 / 1.05 for speed in paired)
        result["minimum_contemporaneous_speedup"] = min(paired)
    return result


def goal_score(aggregates, frozen_objective_sha256=None):
    objective_sha256 = digest(FIXTURE / "optimization-objective.json")
    if frozen_objective_sha256 not in (None, objective_sha256):
        raise ValueError("the frozen objective differs from the current optimization goal")
    objective = read(FIXTURE / "optimization-objective.json")["primary"]
    group = aggregates[objective["partition"]]
    if group["count"] != 15:
        raise ValueError("the primary score requires all fifteen holdout programs")
    score = group[objective["aggregate"]]
    if not math.isfinite(score) or score <= 0:
        raise ValueError("the primary score must be finite and positive")
    target = objective["target_strictly_below"]
    return {"partition": objective["partition"], "metric": objective["aggregate"],
            "value": score, "target_strictly_below": target,
            "achieved": score < target,
            "objective_sha256": objective_sha256,
            "objective_frozen_with_candidate": frozen_objective_sha256 is not None,
            "sum_mech_median_seconds": group["sum_mech_median_seconds"]}


def summarize(args):
    spec, _ = protocol()
    baseline = read(FIXTURE / "baseline-m3-mini.json")
    left, right = read(args.optimization), read(args.holdout)
    assert left["complete"] and right["complete"]
    assert left["partition"] == "optimization" and right["partition"] == "holdout"
    assert left["freeze_sha256"] == right["freeze_sha256"]
    assert left["candidate"] == right["candidate"]
    for receipt in (left, right):
        assert receipt["candidate"]["protocol_sha256"] == digest(FIXTURE / "optimization-protocol.json")
        assert receipt["candidate"]["baseline_sha256"] == digest(FIXTURE / "baseline-m3-mini.json")
    assert set(left["runs"]) == set(spec["optimization"])
    assert set(right["runs"]) == set(spec["holdout"])
    runs = left["runs"] | right["runs"]
    groups = {key: spec[key] for key in ("optimization", "holdout")}
    groups["all"] = sorted(runs)
    result = {"schema": "mech.polybench.performance-progress.v1", "created_at": utc(),
              "candidate": left["candidate"], "freeze_sha256": left["freeze_sha256"],
              "optimization_receipt_sha256": digest(args.optimization),
              "holdout_receipt_sha256": digest(args.holdout),
              "aggregates": {key: aggregate(names, runs, baseline) for key, names in groups.items()}}
    result["goal"] = goal_score(result["aggregates"], result["candidate"].get("objective_sha256"))
    write(args.output, result)
    if args.history:
        history = read(args.history) if args.history.exists() else {
            "schema": "mech.polybench.performance-history.v1", "entries": []}
        label = result["candidate"]["label"]
        if any(row["candidate"]["label"] == label for row in history["entries"]):
            raise ValueError(f"history already contains {label}")
        history["entries"].append(result)
        write(args.history, history)
    print(json.dumps({"goal": result["goal"], "aggregates": result["aggregates"]}, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest="command", required=True)
    frozen = modes.add_parser("freeze")
    frozen.add_argument("--label", required=True)
    frozen.add_argument("--driver", type=Path, required=True)
    frozen.add_argument("--reference-driver", type=Path)
    frozen.add_argument("--output", type=Path, required=True)
    frozen.set_defaults(action=freeze)
    runner = modes.add_parser("run")
    runner.add_argument("--partition", choices=("optimization", "holdout"), required=True)
    runner.add_argument("--driver", type=Path, required=True)
    runner.add_argument("--reference-driver", type=Path)
    runner.add_argument("--c-build", type=Path, required=True)
    runner.add_argument("--freeze", type=Path, required=True)
    runner.add_argument("--samples", type=int, default=5)
    runner.add_argument("--output", type=Path, required=True)
    runner.set_defaults(action=run)
    summary = modes.add_parser("summarize")
    summary.add_argument("--optimization", type=Path, required=True)
    summary.add_argument("--holdout", type=Path, required=True)
    summary.add_argument("--output", type=Path, required=True)
    summary.add_argument("--history", type=Path)
    summary.set_defaults(action=summarize)
    args = parser.parse_args()
    args.action(args)


if __name__ == "__main__":
    main()
