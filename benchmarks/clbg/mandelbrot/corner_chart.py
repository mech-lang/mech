#!/usr/bin/env python3
"""Combine the complete cohort with new Mech builds using per-batch Rust controls."""
import copy
import hashlib
import json
from pathlib import Path
import run as benchmark


def normalized_endpoints(rows):
    # The common endpoint selector expects seconds. Here its temporary input is
    # a dimensionless measured ratio; raw timings in the report remain untouched.
    selected = copy.deepcopy(rows)
    for row in selected:
        row["median_seconds"] = row["comparison_time_ratio"]
    endpoints = benchmark.common.endpoint_summary(selected)
    endpoints.pop("time_baseline_seconds")
    endpoints["time_baseline_ratio"] = 1.0
    endpoints["normalization_cohort"] = "Each measurement batch normalized to its own unmodified Rust #4 control; size normalized across the combined cohort."
    for endpoint in endpoints["languages"].values():
        for point in endpoint["points"].values():
            # Always use Rust = 1, even in a filtered cohort that excludes it.
            point["time_ratio"] *= min(row["comparison_time_ratio"] for row in selected)
    return endpoints


def main():
    baseline_path = benchmark.HERE / "results/apple-m3-16000.json"
    latest_path = benchmark.HERE / "results/apple-m3-peeled-native-16000.json"
    earlier_path = benchmark.HERE / "results/apple-m3-packed-native-16000.json"
    reports = [json.loads(p.read_text()) for p in (baseline_path, latest_path, earlier_path)]
    baseline, latest, earlier = reports
    paths = [baseline_path, latest_path, earlier_path]
    batches = {}
    for path, report in zip(paths, reports):
        control = next(p for p in report["programs"] if p["id"] == "mandelbrot-rust-4")
        batches[path.name] = {
            "path": path.name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            "created_at": report["created_at"], "samples": report["protocol"]["samples"],
            "reference_id": control["id"], "reference_median_seconds": control["median_seconds"],
            "compiler_snapshot": report["compiler_snapshot"],
        }
        assert report["protocol"]["size"] == 16000 and report["protocol"]["requested_threads"] == 8
        assert report["protocol"]["full_input_reference_image"] == baseline["protocol"]["full_input_reference_image"]

    def calibrated(row, path):
        p = copy.deepcopy(row)
        p["measurement_batch"] = path.name
        p["reference_median_seconds"] = batches[path.name]["reference_median_seconds"]
        p["comparison_time_ratio"] = p["median_seconds"] / p["reference_median_seconds"]
        assert p["warmup_image"]["exact_reference_match"]
        return p

    rows = [calibrated(p, baseline_path) for p in baseline["programs"]]
    new_ids = {"mech-compact-boolean-masks", "mech-unrolled-simd-loop-fma-boolean-masks",
               "mech-unrolled-simd-loop-fma-packed-bits", "mech-unrolled-simd-loop-fma-add-packed-bits"}
    rows += [calibrated(p, latest_path) for p in latest["programs"] if p["id"] in new_ids]
    # Retain the additional measured source, using its latest available run.
    rows += [calibrated(p, earlier_path) for p in earlier["programs"]
             if p["id"] == "mech-unrolled-simd-loop-fma-ten-packed-bits"]
    assert len(rows) == 77 and len({p["id"] for p in rows}) == len(rows)
    report = copy.deepcopy(baseline)
    for key in ("metadata_corrections", "uncorrected_report", "compiler_snapshot"):
        report.pop(key, None)
    report.update(programs=rows, created_at=benchmark.datetime.now(benchmark.timezone.utc).isoformat(),
                  measurement_batches=batches, endpoints=normalized_endpoints(rows),
                  nonheroic_endpoints=normalized_endpoints([p for p in rows if not p["upstream_starred"]]),
                  exact_image_endpoints=normalized_endpoints(rows))
    report["historical_mech_endpoints"] = baseline["endpoints"]["languages"]["mech"]
    report["evaluation_status"] = {
        "kind": "numerical_kernel_with_host_grid_driver",
        "cross_language_complete_program_size_claim": "withdrawn",
        "reason": "The Mech source includes the numerical algorithm, while the host supplies grid traversal and PBM output composition counted in competitor programs.",
        "timing_scope": "Complete executables; Mech recurrence and stopping compiled from source by Cranelift.",
    }
    report["protocol"].update(status="complete baseline cohort plus Mech improvements; combined measurement batches",
                              samples_label="3–5 fresh runs per build",
                              comparison_time="median time divided by Rust #4 median from the same measurement batch; raw medians retained per row",
                              selection="All 72 accepted baseline configurations plus five improved/native Mech configurations. Removed first-turn peeling experiments excluded.",
                              chart_note="Combined measured batches, each normalized to its own Rust #4 control (Rust = 1). Not a new simultaneous full-cohort run.")
    report["protocol"].pop("sample_order", None)
    report["protocol"].pop("samples", None)
    report["protocol"].pop("seed", None)
    report["derived_from"] = [p.name for p in paths]
    output = benchmark.HERE / "results/apple-m3-improved-corner-16000.json"
    output.write_text(json.dumps(report, indent=2) + "\n")
    e = report["endpoints"]["languages"]["mech"]
    assert e["fastest"] == "mech-unrolled-simd-loop-fma-add-packed-bits"
    assert e["smallest"] == "mech-compact-boolean-masks"
    print(output)
    print("Mech endpoints:", e)


if __name__ == "__main__": main()
