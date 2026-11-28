#!/usr/bin/env python3
"""Record performance acceptance separately from immutable PolyBench measurements.

Run after benchmark-polybench.py summarize. Correctness qualification and source
freezing remain prerequisites. From round47, only optimization selects/promotes;
holdout remains a separately reported generalization measurement.
"""
import argparse
import datetime
import hashlib
import gzip
import json
import math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "hosts/gpu/tests/fixtures/polybench"
SCHEMA = "mech.polybench.performance-acceptance.v2"
PARTITIONS = ("optimization", "holdout")
METRIC = "geomean_mech_over_c_kernel"
AMENDMENT_PATH = FIXTURE / "performance-selection-policy-v4.json"
AMENDMENT = json.loads(AMENDMENT_PATH.read_text())
POLICY = AMENDMENT["policy"]
FIRST_OPTIMIZATION_ONLY = AMENDMENT["effective_from_history_index"]
assert FIRST_OPTIMIZATION_ONLY == 47



def fingerprint(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, allow_nan=False).encode()).hexdigest()


def scores(entry):
    result = {}
    for partition in PARTITIONS:
        group = entry["aggregates"][partition]
        value = group[METRIC]
        if group["count"] != 15 or not isinstance(value, (float, int)) or not math.isfinite(value) or value <= 0:
            raise ValueError("acceptance requires fifteen finite positive scores per partition")
        result[partition] = value
    return result


def comparison(current, reference):
    if all(current[p] < reference[p] for p in PARTITIONS):
        return "improved_both"
    if all(current[p] > reference[p] for p in PARTITIONS):
        return "failed_both_regressed"
    return "mixed_or_tied"


def classify(entries):
    if not entries:
        raise ValueError("performance history is empty")
    accepted = previous = None
    accepted_index = 0
    labels = set()
    decisions = []
    for index, entry in enumerate(entries):
        label = entry["candidate"]["label"]
        if label in labels:
            raise ValueError("duplicate candidate label")
        labels.add(label)
        current = scores(entry)
        optimization_only = index >= FIRST_OPTIMIZATION_ONLY
        promoted = accepted is None or (
            current["optimization"] < accepted["optimization"] if optimization_only
            else comparison(current, accepted) == "improved_both"
        )
        decision = {
            "history_index": index,
            "label": label,
            "source_commit": entry["candidate"]["source_commit"],
            "history_entry_sha256": fingerprint(entry),
            "scores": current,
            "previous_measured_index": index - 1 if previous is not None else None,
            "run_outcome": comparison(current, previous) if previous is not None else "baseline",
            "compared_to_accepted_index": accepted_index if accepted is not None else None,
            "multipliers_vs_accepted": {p: current[p] / accepted[p] for p in PARTITIONS} if accepted is not None else None,
            "performance_gate": "accepted" if promoted else "not_promoted",
        }
        if optimization_only:
            decision.update(
                selection_partition="optimization",
                selection_outcome="optimization_improved" if promoted else "optimization_not_improved",
                holdout_use="reported_only",
                selection_policy="optimization-only-v4",
            )
        if promoted:
            accepted, accepted_index = current, index
        decision["accepted_index_after_run"] = accepted_index
        decisions.append(decision)
        previous = current
    return decisions


def record(history_path, output_path, check=False):
    history_bytes = history_path.read_bytes()
    history = json.loads(history_bytes)
    if history["schema"] != "mech.polybench.performance-history.v1":
        raise ValueError("unknown performance history schema")
    decisions = classify(history["entries"])
    existing = json.loads(output_path.read_text()) if output_path.exists() else None
    amendment_sha = hashlib.sha256(AMENDMENT_PATH.read_bytes()).hexdigest()
    archive = AMENDMENT["previous_ledger"]
    compressed = (FIXTURE / archive["path"]).read_bytes()
    raw = gzip.decompress(compressed)
    if hashlib.sha256(compressed).hexdigest() != archive["compressed_sha256"] or hashlib.sha256(raw).hexdigest() != archive["sha256"]:
        raise ValueError("preserved prior acceptance ledger differs")
    prior = json.loads(raw)
    if prior["entries"][47] != AMENDMENT["previous_round47_decision"]:
        raise ValueError("selection amendment differs from the original decision")
    if prior["entries"][:FIRST_OPTIMIZATION_ONLY] != decisions[:FIRST_OPTIMIZATION_ONLY]:
        raise ValueError("earlier acceptance decisions changed")
    if existing is not None and existing["schema"] == SCHEMA:
        if existing["policy"] != POLICY or existing["policy_amendment_sha256"] != amendment_sha:
            raise ValueError("acceptance policy changed; preserve the old ledger")
        old = existing["entries"]
        if old != decisions[:len(old)]:
            raise ValueError("recorded measurements or acceptance decisions changed")
        ledger = dict(existing)
    else:
        if existing is not None and existing != prior:
            raise ValueError("the migration source differs from the preserved ledger")
        ledger = dict(prior)
        ledger.update(
            schema=SCHEMA,
            policy=POLICY,
            policy_amended_at=AMENDMENT["adopted_at"],
            policy_amendment="performance-selection-policy-v4.json",
            policy_amendment_sha256=amendment_sha,
            previous_ledger=archive,
            interpretation="Original decisions through46 remain exact. The explicit user-directed amendment accepts measured round47 using optimization only and applies that rule to subsequent rounds. Immutable measurements are unchanged; holdout is report-only for selection.",
        )
    ledger.update({
        "history_sha256": hashlib.sha256(history_bytes).hexdigest(),
        "history_entries": len(decisions),
        "entries": decisions,
        "accepted_history_index": decisions[-1]["accepted_index_after_run"],
    })
    if check:
        if ledger != existing:
            raise ValueError("acceptance ledger is missing or stale; record the new run first")
    else:
        output_path.write_text(json.dumps(ledger, indent=2, allow_nan=False) + "\n")
    return ledger


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--history", type=Path, default=FIXTURE / "performance-history.json")
    parser.add_argument("--output", type=Path, default=FIXTURE / "performance-acceptance.json")
    parser.add_argument("--check", action="store_true", help="verify without writing")
    parser.add_argument("--require-latest-accepted", action="store_true", help="exit unsuccessfully when the latest run did not pass the performance gate")
    args = parser.parse_args()
    ledger = record(args.history, args.output, args.check)
    latest = ledger["entries"][-1]
    accepted = ledger["entries"][ledger["accepted_history_index"]]
    print(json.dumps({"latest": latest["label"], "run_outcome": latest["run_outcome"],
                      "performance_gate": latest["performance_gate"],
                      "selection_outcome": latest.get("selection_outcome", latest["run_outcome"]),
                      "accepted": accepted["label"], "accepted_scores": accepted["scores"]}))
    if args.require_latest_accepted and latest["performance_gate"] != "accepted":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
