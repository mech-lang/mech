#!/usr/bin/env python3
"""Confirm fast endpoints and test Chapel chunk size / Mech scheduling."""
import argparse
import copy
import hashlib
import json
import random
import statistics
from pathlib import Path
import run as benchmark


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    baseline = json.loads(args.baseline.read_text())
    selected = {e["fastest"] for e in baseline["endpoints"]["languages"].values()}
    selected |= {p["id"] for p in baseline["programs"] if p["language"] == "mech"}
    rows = [copy.deepcopy(p) for p in baseline["programs"] if p["id"] in selected]
    chapel = next(p for p in rows if p["id"] == "mandelbrot-chapel-3")
    for chunk in (64, 512, 4096):
        p = copy.deepcopy(chapel)
        p.update(id=f"mandelbrot-chapel-3-chunk{chunk}", upstream_id="mandelbrot-chapel-3", chunk_size=chunk)
        p["command"] += [f"--chunkSize={chunk}"]
        p["correctness_command"] += [f"--chunkSize={chunk}"]
        rows.append(p)
    # Same source and native math, with independent operations emitted by demand.
    old = next(p for p in rows if p["id"] == "mech-unrolled-simd-loop-fma")
    p = copy.deepcopy(old)
    p["id"] += "-scheduled"
    replacement = {str(benchmark.BUILD / (old["id"] + suffix)): str(benchmark.BUILD / (p["id"] + suffix))
                   for suffix in ("", ".c", ".o")}
    p["generation_command"] = [replacement.get(s, s) for s in p["generation_command"]] + ["--schedule"]
    p["compile_command"] = [replacement.get(s, s) for s in p["compile_command"]]
    p["command"] = [replacement.get(s, s) for s in p["command"]]
    p["correctness_command"] = [replacement.get(s, s) for s in p["correctness_command"]]
    benchmark.command(p["generation_command"])
    benchmark.command(p["compile_command"])
    for field, suffix in (("generated_c_sha256", ".c"), ("native_object_sha256", ".o")):
        p[field] = hashlib.sha256((benchmark.BUILD / (p["id"] + suffix)).read_bytes()).hexdigest()
    rows.append(p)
    env = benchmark.os.environ | baseline["protocol"]["environment"]
    n = baseline["protocol"]["size"]
    for p in rows:
        _, data, _ = benchmark.image_run(p["correctness_command"], env, 200, 600)
        if hashlib.sha256(data).hexdigest() != benchmark.REFERENCE_SHA:
            raise ValueError("official bitmap mismatch: " + p["id"])
        p["correctness_200"] = benchmark.image_info(data)
        p["samples_seconds"], p["samples_images"] = [], []
    reference_row = next(p for p in rows if p["id"] == "mandelbrot-rust-4")
    _, reference, _ = benchmark.image_run(reference_row["command"], env, n, 600)
    for p in rows:
        print("Warmup " + p["id"], flush=True)
        _, data, _ = benchmark.image_run(p["command"], env, n, 600)
        if data != reference: raise ValueError("full bitmap mismatch: " + p["id"])
        p["warmup_image"] = benchmark.image_info(data, reference)
    rng, order = random.Random(20261008), []
    for i in range(args.samples):
        shuffled = rows.copy()
        rng.shuffle(shuffled)
        order.append([p["id"] for p in shuffled])
        for p in shuffled:
            print(f"Sample {i+1}/{args.samples} {p['id']}", flush=True)
            elapsed, data, _ = benchmark.image_run(p["command"], env, n, 600)
            if data != reference: raise ValueError("full bitmap mismatch: " + p["id"])
            p["samples_seconds"].append(elapsed)
            p["samples_images"].append(benchmark.image_info(data, reference))
    for p in rows:
        p.update(median_seconds=statistics.median(p["samples_seconds"]), min_seconds=min(p["samples_seconds"]), max_seconds=max(p["samples_seconds"]))
    report = copy.deepcopy(baseline)
    for key in ("metadata_corrections", "uncorrected_report"): report.pop(key, None)
    report.update(programs=rows, excluded=[], created_at=benchmark.datetime.now(benchmark.timezone.utc).isoformat())
    report["protocol"].update(samples=args.samples, seed=20261008, sample_order=order,
                              status="full-input confirmation and runtime configuration study; selected endpoints",
                              selection="Fastest baseline build for every language, all five baseline Mech builds, Chapel #3 chunk sizes 64/512/4096 and Mech demand scheduling. Smallest competitor sources are not remeasured in this study.")
    report["baseline_report"] = {"path": args.baseline.name, "sha256": hashlib.sha256(args.baseline.read_bytes()).hexdigest()}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    report["compiler_snapshot"] = benchmark.snapshot(args.output)
    report["endpoints"] = benchmark.common.endpoint_summary(rows)
    report["nonheroic_endpoints"] = benchmark.common.endpoint_summary([p for p in rows if not p["upstream_starred"]])
    report["exact_image_endpoints"] = report["endpoints"]
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    for p in sorted(rows, key=lambda p: (p["language"], p["median_seconds"])):
        print(p["id"], p["median_seconds"], [round(x, 6) for x in p["samples_seconds"]], flush=True)


if __name__ == "__main__": main()
