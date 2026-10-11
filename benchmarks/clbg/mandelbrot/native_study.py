#!/usr/bin/env python3
"""Compare preserved baseline binaries with current native-map code generation."""
import argparse
import copy
import hashlib
import json
import random
import statistics
import run as benchmark
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--extended", action="store_true", help="also test packed returns, addition doubling and ten-step unrolling")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.samples < 3: parser.error("samples must be at least three")
    baseline = json.loads(args.baseline.read_text())
    env = benchmark.os.environ | baseline["protocol"]["environment"]
    selected = {"mandelbrot-rust-4", "mech-unrolled-simd-loop-fma"}
    rows = [copy.deepcopy(p) for p in baseline["programs"] if p["id"] in selected]
    for p in rows:
        p["compiler_snapshot"] = baseline["compiler_snapshot"]
    # Unique artifact names preserve the baseline binaries for an actual A/B.
    variants = [("mech-compact", "boolean-masks", None, False), ("mech-unrolled-simd-loop-fma", "boolean-masks", None, False)]
    if args.extended:
        variants += [("mech-unrolled-simd-loop-fma", "packed-bits", None, True),
                     ("mech-unrolled-simd-loop-fma", "add-packed-bits", "mandelbrot-unrolled-add.mec", True)]
        variants += [("mech-unrolled-simd-loop-fma", "ten-packed-bits", "mandelbrot-unrolled-10.mec", True)]
    for name, suffix, source, packed in variants:
        p = copy.deepcopy(next(p for p in baseline["programs"] if p["id"] == name))
        old_id = p["id"]
        p["id"] += "-" + suffix
        replacement = {str(benchmark.BUILD / (old_id + suffix)): str(benchmark.BUILD / (p["id"] + suffix))
                       for suffix in ("", ".c", ".o")}
        for field in ("generation_command", "compile_command", "command", "correctness_command"):
            p[field] = [replacement.get(s, s) for s in p[field]]
        if source:
            p["path"] = source
            p["source"] = benchmark.metrics(source)
            p["generation_command"][1] = str(benchmark.HERE / source)
        if packed: p["generation_command"] += ["--packed-bits"]
        benchmark.command(p["generation_command"])
        benchmark.command(p["compile_command"])
        for field, suffix in (("generated_c_sha256", ".c"), ("native_object_sha256", ".o")):
            p[field] = hashlib.sha256((benchmark.BUILD / (p["id"] + suffix)).read_bytes()).hexdigest()
        p["code_generation"] = "integer Boolean masks; " + ("packed Boolean return ABI" if packed else "numeric observation ABI")
        p.pop("compiler_snapshot", None)
        rows.append(p)
    n = baseline["protocol"]["size"]
    reference_row = next(p for p in rows if p["id"] == "mandelbrot-rust-4")
    _, reference, _ = benchmark.image_run(reference_row["command"], env, n, 600)
    for p in rows:
        _, data, _ = benchmark.image_run(p["correctness_command"], env, 200, 600)
        if hashlib.sha256(data).hexdigest() != benchmark.REFERENCE_SHA:
            raise ValueError("official bitmap mismatch: " + p["id"])
        p["correctness_200"] = benchmark.image_info(data)
        p["samples_seconds"], p["samples_images"] = [], []
        print("Warmup " + p["id"], flush=True)
        _, data, _ = benchmark.image_run(p["command"], env, n, 600)
        if data != reference: raise ValueError("full bitmap mismatch: " + p["id"])
        p["warmup_image"] = benchmark.image_info(data, reference)
    rng, order = random.Random(20261009), []
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
    report["protocol"].update(samples=args.samples, seed=20261009, sample_order=order,
                              status="native compiler A/B; selected Rust and Mech programs",
                              selection="Preserved baseline Rust #4 and fastest Mech binary, current compiler on the same fastest and smallest Mech sources; optional packed-return and unrolling/doubling variants. Other languages are not remeasured.")
    report["baseline_report"] = {"path": args.baseline.name, "sha256": hashlib.sha256(args.baseline.read_bytes()).hexdigest()}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    report["compiler_snapshot"] = benchmark.snapshot(args.output)
    report["endpoints"] = benchmark.common.endpoint_summary(rows)
    report["nonheroic_endpoints"] = report["endpoints"]
    report["exact_image_endpoints"] = report["endpoints"]
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    for p in sorted(rows, key=lambda p: p["median_seconds"]):
        print(p["id"], p["median_seconds"], p["samples_seconds"], flush=True)


if __name__ == "__main__": main()
