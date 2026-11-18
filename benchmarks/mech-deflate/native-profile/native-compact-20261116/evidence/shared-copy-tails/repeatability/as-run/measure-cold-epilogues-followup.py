"""Measure the qualified linked workers in three-actor paired blocks."""
import argparse
from collections import Counter
from hashlib import file_digest
from itertools import permutations
from math import ceil
from pathlib import Path
from random import Random
from statistics import mean, median
import gzip
import json
import os
import select
import subprocess
import time

ACTORS = ("control", "duplicate-control", "candidate")
COUNTERS = (
    "payload_allocation_requests",
    "requested_payload_bytes",
    "initialized_bytes_copied",
    "zeroed_bytes",
    "peak_private_payload_bytes",
    "nonreturned_private_bytes_at_return",
)
RESOLUTION = 0.005


class Worker:
    def __init__(self, folder, fixtures, stderr):
        self.buffer = b""
        self.stderr = stderr.open("wb")
        self.child = subprocess.Popen(
            [folder / "mech-worker", folder / "metadata.json", fixtures],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr,
        )
        assert self.line() == b"SESSION_READY"

    def line(self):
        while b"\n" not in self.buffer:
            assert select.select([self.child.stdout], [], [], 60)[0], "worker timed out"
            chunk = os.read(self.child.stdout.fileno(), 65536)
            assert chunk, self.child.poll()
            self.buffer += chunk
        line, self.buffer = self.buffer.split(b"\n", 1)
        return line

    def send(self, message):
        self.child.stdin.write(message)
        self.child.stdin.flush()

    def observe(self, fixture, warmup=False, profile=False):
        row = json.loads(self.line())
        assert row["case"] == fixture["name"]
        for field in ("input_bytes", "raw_bytes", "prefix_bytes", "capacity"):
            assert row[field] == fixture[field], (fixture["name"], field)
        for field in ("exact_reference", "independent_inflation", "input_alias_unchanged",
                      "borrowed_output_alias_unchanged"):
            assert row[field], (fixture["name"], field)
        assert row["warmup"] == warmup and row["profile"] == profile
        assert row["nonreturned_private_bytes_at_return"] == 0
        return row

    def close(self):
        if self.child.poll() is None:
            self.child.terminate()
            self.child.wait(timeout=10)
        self.stderr.close()


def interval(summary, contrast, seed):
    """Resample whole paired blocks within each of the six physical orders."""
    from math import exp, log, sqrt

    groups = {}
    for row in summary:
        a, b, c = (row["means"][actor] for actor in ACTORS)
        ratio = {"aa": b / a, "ab": c / sqrt(a * b)}[contrast]
        groups.setdefault(tuple(row["order"]), []).append(log(ratio))
    parts = list(groups.values())
    assert len(parts) == 6 and len(set(map(len, parts))) == 1
    rng = Random(seed)
    values = sorted(
        exp(sum(sum(rng.choices(p, k=len(p))) for p in parts) / len(summary)) - 1
        for _ in range(8000)
    )
    return {
        "estimate": exp(sum(map(sum, parts)) / len(summary)) - 1,
        "lower": values[400],
        "upper": values[7600],
    }


def measure(root, output, targets, seed, repetitions, candidate_dir, duplicate_control_dir=None, batch_floor=8):
    folders = {
        "control": root / "control",
        "duplicate-control": duplicate_control_dir or root / "control",
        "candidate": candidate_dir,
    }
    executables = {actor: folder / "mech-worker" for actor, folder in folders.items()}
    identities = {}
    inodes = {}
    for actor, path in executables.items():
        stat = path.stat()
        inodes[actor] = stat.st_dev, stat.st_ino
        with path.open("rb") as stream:
            identities[actor] = file_digest(stream, "sha256").hexdigest()
    assert identities["control"] == identities["duplicate-control"]
    distinct_files = len(set(inodes.values()))
    if duplicate_control_dir is not None:
        assert distinct_files == len(ACTORS)
    fixtures = root / "fixtures.json"
    cases = json.loads(fixtures.read_text())
    preliminary = json.loads((root / "cold-uncertainty.json").read_text())["cases"]
    if not targets:
        targets = [
            name for name, row in preliminary.items()
            if row["combined"]["upper"] > RESOLUTION
            or min(row["full"]["estimate"], row["reverse"]["estimate"]) > 0
        ]
    medians = {
        row["case"]: row["median_seconds"]["mech-control"]
        for row in json.loads((root / "full" / "summary.json").read_text())
    }
    output.mkdir()
    workers = {}
    report = {}
    rng = Random(seed)
    sequence = [(index, row) for index, row in enumerate(cases) if row["name"] in targets]
    rng.shuffle(sequence)
    assert len(sequence) == len(targets)
    (output / "study.json").write_text(json.dumps(dict(
        acceptance_resolution=RESOLUTION, seed=seed,
        order=[row["name"] for _, row in sequence],
        repetitions_per_order=repetitions, timer="worker complete-call elapsed durations",
        worker_sha256=identities, distinct_worker_files=distinct_files, batch_floor=batch_floor,
    ), indent=2) + "\n")
    try:
        for actor in ACTORS:
            workers[actor] = Worker(folders[actor], fixtures, output / f"{actor}.stderr")
        for index, fixture in sequence:
            started = time.monotonic()
            name = fixture["name"]
            # Aim for a half millisecond of measured compression per actor.
            # The small cap keeps the per-call correctness stream bounded.
            batch = max(batch_floor, min(8192, ceil(0.0005 / medians[name])))
            orders = list(permutations(ACTORS)) * repetitions
            rng.shuffle(orders)
            for worker in workers.values():
                worker.send(f"C\t{index}\t{len(orders) * batch}\n".encode())
                assert worker.line() == f"CASE_READY\t{name}".encode()
            warm_order = list(ACTORS)
            rng.shuffle(warm_order)
            for actor in warm_order:
                worker = workers[actor]
                worker.send(b"W")
                state, case, calls, elapsed = worker.line().decode().split("\t")
                assert state == "WARMUP_DONE" and case == name
                assert int(calls) > 0 and float(elapsed) >= 0.2
                worker.send(b"N")
                worker.observe(fixture, warmup=True)
            blocks = []
            counter_sets = {actor: Counter() for actor in ACTORS}
            for order in orders:
                raw = {}
                for actor in order:
                    worker = workers[actor]
                    samples = []
                    for start in range(0, batch, 512):
                        count = min(512, batch - start)
                        worker.send(b"N" * count)
                        for _ in range(count):
                            row = worker.observe(fixture)
                            counter_sets[actor][tuple(row[k] for k in COUNTERS)] += 1
                            samples.append(row["seconds"])
                    raw[actor] = samples
                blocks.append(dict(order=order, seconds=raw, means={
                    actor: mean(samples) for actor, samples in raw.items()
                }))
            for worker in workers.values():
                worker.send(b"N")
                worker.observe(fixture, profile=True)
                assert worker.line() == f"CASE_DONE\t{name}".encode()
            assert counter_sets["control"] == counter_sets["duplicate-control"] == counter_sets["candidate"]
            packed = dict(case=name, batch=batch, blocks=blocks, counters=list(COUNTERS),
                          counter_counts=list(counter_sets["control"].items()))
            (output / f"{name}.json.gz").write_bytes(
                gzip.compress(json.dumps(packed, separators=(",", ":")).encode(), mtime=0)
            )
            report[name] = dict(
                batch=batch, seconds=time.monotonic()-started,
                aa=interval(blocks, "aa", seed+index),
                ab=interval(blocks, "ab", seed+index),
                median_batch_ms=1000 * batch * median(
                    t["means"][actor] for t in blocks for actor in ACTORS
                ),
            )
            print(name, batch, report[name], flush=True)
            (output / "results.json").write_text(json.dumps(report, indent=2) + "\n")
        for worker in workers.values():
            worker.send(b"Q\n")
            assert worker.child.wait(timeout=60) == 0
    finally:
        for worker in workers.values():
            worker.close()
    for actor in ACTORS:
        assert (output/f"{actor}.stderr").read_text().endswith(
            "CLEANUP private_payload_bytes=0 live_owner_records=0\n"
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("evidence", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--seed", type=int, default=311298)
    parser.add_argument("--case", action="append")
    parser.add_argument("--repetitions-per-order", type=int, default=8)
    parser.add_argument("--candidate-dir", type=Path)
    parser.add_argument("--duplicate-control-dir", type=Path)
    parser.add_argument("--batch-floor", type=int, default=8, choices=range(1, 65), metavar="1-64")
    args = parser.parse_args()
    candidate = args.candidate_dir or args.evidence / "candidate"
    duplicate = args.duplicate_control_dir
    measure(args.evidence.resolve(), args.output.resolve(), args.case, args.seed,
            args.repetitions_per_order, candidate.resolve(),
            duplicate.resolve() if duplicate is not None else None, args.batch_floor)
