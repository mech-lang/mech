#!/usr/bin/env python3
"""Run the declared frame comparison once; retain every observation and failure."""
import argparse
import hashlib
import json
import statistics
import subprocess
from pathlib import Path


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_compilers_stopped():
    names = subprocess.check_output(["ps", "-Ao", "comm="], text=True).splitlines()
    active = [name.strip() for name in names if Path(name.strip()).name in {"rustc", "cargo", "complete-reality"}]
    if active:
        raise RuntimeError(f"measurement requires stopped compilers/compression jobs: {active}")


def run(executable, case, form, steps, samples, repetitions, destination):
    check_compilers_stopped()
    command = [str(executable), case, form, str(steps), str(samples), str(repetitions)]
    result = subprocess.run(command, capture_output=True, text=True)
    destination.with_suffix(".stdout").write_text(result.stdout)
    destination.with_suffix(".stderr").write_text(result.stderr)
    destination.with_suffix(".json").write_text(json.dumps({"command": command, "exit": result.returncode}, indent=2) + "\n")
    result.check_returncode()
    return [json.loads(line) for line in result.stdout.splitlines() if line.strip()]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("baseline", type=Path)
    parser.add_argument("candidate", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--steps", type=int, required=True, help="visit count frozen after diagnostics")
    args = parser.parse_args()
    assert args.steps > 0
    args.baseline = args.baseline.resolve(strict=True)
    args.candidate = args.candidate.resolve(strict=True)
    args.output.mkdir(parents=True, exist_ok=False)
    metadata = {"baseline_sha256": digest(args.baseline), "candidate_sha256": digest(args.candidate),
                "steps": args.steps, "rounds": 3, "samples_per_round": 10,
                "warmups_per_process": 1, "discarded_observations": 0}
    (args.output / "protocol.json").write_text(json.dumps(metadata, indent=2) + "\n")
    observations = []
    try:
        slopes = []
        for role in ["baseline", "candidate"]:
            for form in ["original", "decoded"]:
                counts = []
                for steps in [0, args.steps]:
                    rows = run(getattr(args, role), "scalar-loop", form, steps, 1, 1,
                               args.output / f"slope-{role}-{form}-{steps}")
                    observations.extend(dict(row, role=role, phase="slope") for row in rows)
                    counts.append([(row["allocation_requests"], row["requested_bytes"]) for row in rows if not row["warmup"]][0])
                slopes.append({"role": role, "form": form, "zero_per_visit_allocation": counts[0] == counts[1], "costs": counts})
        cases = [("exclusive", 1), ("aliased", 1), ("scalar-loop", 100),
                 ("scalar-u64", 10000), ("scalar-i64", 10000), ("scalar-f64", 10000), ("scalar-bool", 10000)]
        for round_number in range(3):
            for index, (case, repetitions) in enumerate(cases):
                for form_index, form in enumerate(["original", "decoded"]):
                    order = ["baseline", "candidate"] if (round_number + index + form_index) % 2 == 0 else ["candidate", "baseline"]
                    for role in order:
                        rows = run(getattr(args, role), case, form, args.steps, 10, repetitions,
                                   args.output / f"round-{round_number}-{case}-{form}-{role}")
                        assert len(rows) == 11 and sum(row["warmup"] for row in rows) == 1
                        observations.extend(dict(row, role=role, phase="primary", round=round_number) for row in rows)
                        print(f"round={round_number} case={case} form={form} role={role}", flush=True)
        results = []
        for case, _ in cases:
            for form in ["original", "decoded"]:
                selected = [row for row in observations if row["phase"] == "primary" and not row["warmup"] and row["case"] == case and row["form"] == form]
                baseline = sorted((row for row in selected if row["role"] == "baseline"), key=lambda row: (row["round"], row["sample"]))
                candidate = sorted((row for row in selected if row["role"] == "candidate"), key=lambda row: (row["round"], row["sample"]))
                assert len(baseline) == len(candidate) == 30
                assert len({row["artifact_sha256"] for row in baseline + candidate}) == 1
                paired_time_ratio = statistics.median(c["seconds"] / b["seconds"] for b, c in zip(baseline, candidate))
                ratios = {field: statistics.median(row[field] for row in candidate) / statistics.median(row[field] for row in baseline)
                          for field in ["allocation_requests", "requested_bytes"]}
                mixed = case in {"exclusive", "aliased"}
                passed = (paired_time_ratio < 1 and all(ratio <= 0.7 for ratio in ratios.values())) if mixed else (paired_time_ratio <= 1.05 and all(ratio <= 1 for ratio in ratios.values()))
                results.append({"case": case, "form": form, "passed": passed, "paired_median_time_ratio": paired_time_ratio,
                                "ratios": ratios, "baseline_medians": {field: statistics.median(row[field] for row in baseline) for field in ["seconds", "allocation_requests", "requested_bytes"]},
                                "candidate_medians": {field: statistics.median(row[field] for row in candidate) for field in ["seconds", "allocation_requests", "requested_bytes"]}})
        summary = {"passed": all(row["passed"] for row in results) and all(row["zero_per_visit_allocation"] for row in slopes), "cases": results, "scalar_slopes": slopes}
        (args.output / "results.json").write_text(json.dumps(summary, indent=2) + "\n")
        print(json.dumps(summary, indent=2))
        if not summary["passed"]:
            raise RuntimeError("declared performance gates failed; retain this cohort")
    except BaseException as error:
        (args.output / "failure.json").write_text(json.dumps({"error": repr(error)}, indent=2) + "\n")
        raise
    finally:
        (args.output / "observations.jsonl").write_text("".join(json.dumps(row) + "\n" for row in observations))


if __name__ == "__main__":
    main()
