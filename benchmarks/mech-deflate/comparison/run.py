#!/usr/bin/env python3
"""Build and measure pinned compressors on the maintained complete-stream cases."""
import argparse
import csv
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import signal
import statistics
import subprocess
import time
import zlib


ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def call(command, **kwargs):
    return subprocess.run([str(x) for x in command], check=True, **kwargs)


def prepare(args):
    fixture = json.loads((ROOT / "hosts/gpu/tests/fixtures/native/deflate-compress-reference.json").read_text())
    for relative, expected in fixture["reference"]["package_source_sha256"].items():
        assert digest(args.reference / relative) == expected, relative
    assert call(["git", "-C", args.reference, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip() == fixture["reference"]["source_pin"]
    allocator = args.reference / "benchmark/echo-clock/targets/arm64mac/mimalloc.o"
    assert digest(allocator) == "38c6ec82a41e90a96bb9f1ae2c89058a77909da3fbb69c16e542ae5c858bf874"
    args.out.mkdir(parents=True, exist_ok=True)
    cases = []
    for item in fixture["cases"]:
        cases.append((item["name"], bytes(item["input"]), bytes(item["prefix"]), bytes(item["expected"])))
    fixtures = ROOT / "hosts/gpu/tests/fixtures/native"
    for name in ["16384-distribution-change", "33027-periodic-window"]:
        base = fixtures / f"deflate-boundary-{name}"
        cases.append((name, base.with_suffix(".input").read_bytes(), b"", base.with_suffix(".deflate").read_bytes()))
    cases.append(("full-reymont", args.corpus.read_bytes(), b"", args.full_reference.read_bytes()))
    metadata = {
        "mech_comparison_base": call(["git", "rev-parse", "HEAD"], capture_output=True, text=True, cwd=ROOT).stdout.strip(),
        "reference_source_revision": fixture["reference"]["source_pin"],
        "reference_compiler_revision": fixture["reference"]["compiler_pin"],
        "reference_package_sha256": fixture["reference"]["package_source_sha256"],
        "roc_compiler_sha256": digest(args.roc),
        "roc_version": call([args.roc, "version"], capture_output=True, text=True).stdout.strip(),
        "rustc_version": call(["rustc", "--version", "--verbose"], capture_output=True, text=True).stdout,
        "platform": platform.platform(),
        "cpu": call(["sysctl", "-n", "machdep.cpu.brand_string"], capture_output=True, text=True).stdout.strip(),
        "allocator_object_sha256": digest(allocator),
        "allocator_revision": "8c532c32c3c96e5ba1f2283e032f69ead8add00f",
        "warmup_min_ns": 200000000,
        "reference_clock_resolution_ns": round(time.clock_getres(time.CLOCK_MONOTONIC) * 1000000000),
        "zero_elapsed_policy": "retain zero observations as below clock resolution; never replace them with a positive duration",
        "harness_sha256": {str(p.relative_to(HERE)): digest(p) for p in [HERE / "Bench.roc", HERE / "rust/Cargo.toml", HERE / "rust/build.rs", HERE / "rust/src/main.rs", HERE / "rust/src/pinned_allocator.rs"]},
        "timing_boundary": "compress_into only; clear/prefix, output reservation, warmup, validation, logging outside; compressor scratch initialization/destruction inside",
        "rust": {"miniz-oxide": {"crate": "miniz_oxide", "version": "0.9.1", "level": 10}, "zlib-rs": {"crate": "zlib-rs", "version": "0.6.8", "level": 9, "features": ["std"], "default_features": False}},
        "roc_level": 12,
        "cases": [],
    }
    stage = args.reference / "benchmark/echo-clock/.build/mech-comparison"
    stage.mkdir(parents=True, exist_ok=True)
    for name, data, prefix, expected in cases:
        assert expected[:len(prefix)] == prefix
        assert zlib.decompress(expected[len(prefix):], -15) == data
        directory = args.out / name
        directory.mkdir(exist_ok=True)
        for filename, contents in [("input", data), ("prefix", prefix), ("reference.deflate", expected)]:
            (directory / filename).write_bytes(contents)
        reps = args.full_reps if name == "full-reymont" else args.reps
        source_dir = stage / name
        source_dir.mkdir(exist_ok=True)
        for filename in ["input", "prefix", "reference.deflate"]:
            shutil.copyfile(directory / filename, source_dir / filename)
        shutil.copyfile(HERE / "Bench.roc", source_dir / "Bench.roc")
        rel = lambda path: json.dumps(os.path.relpath(path, source_dir))
        source = ("app [main!] {\n" + f" pf: platform {rel(args.reference / 'benchmark/echo-clock/main.roc')},\n" + f" deflate: {rel(args.reference / 'package/main.roc')},\n" + "}\nimport Bench\n" + 'import "input" as input : List(U8)\nimport "prefix" as prefix : List(U8)\nimport "reference.deflate" as expected : List(U8)\n' + f"main! = |level| Bench.run!(input, prefix, expected, {reps}, level)\n")
        (source_dir / "app.roc").write_text(source)
        shutil.copyfile(source_dir / "app.roc", directory / "app.roc")
        command = [args.roc, "build", "--no-cache", "--opt=speed", source_dir / "app.roc", f"--output={directory / 'rocbench'}"]
        started = time.monotonic()
        with (directory / "build.log").open("w") as log:
            call(command, stdout=log, stderr=subprocess.STDOUT)
        case = {"name": name, "input_bytes": len(data), "prefix_bytes": len(prefix), "reference_raw_bytes": len(expected) - len(prefix), "input_sha256": digest(directory / "input"), "reference_sha256": digest(directory / "reference.deflate"), "repetitions": reps, "roc_executable_sha256": digest(directory / "rocbench"), "roc_build_seconds": time.monotonic() - started, "roc_build_command": [str(x) for x in command]}
        metadata["cases"].append(case)
        print("BUILT", name, flush=True)
    manifest = HERE / "rust/Cargo.toml"
    env = dict(os.environ, DEFLATE_MIMALLOC_OBJECT=str(allocator), CARGO_TARGET_DIR=str(args.out / "rust-target"))
    command = ["cargo", "build", "--locked", "--offline", "--release", "--manifest-path", manifest]
    with (args.out / "rust-build.log").open("w") as log:
        call(command, env=env, stdout=log, stderr=subprocess.STDOUT)
    binary = args.out / "rust-target/release/deflate-comparison"
    metadata["rust_executable_sha256"] = digest(binary)
    metadata["rust_lock_sha256"] = digest(HERE / "rust/Cargo.lock")
    metadata["rust_build_command"] = [str(x) for x in command]
    metadata["rust_features"] = call(["cargo", "tree", "--offline", "-e", "features", "--manifest-path", manifest], env=env, capture_output=True, text=True).stdout
    (args.out / "build.json").write_text(json.dumps(metadata, indent=2) + "\n")
    return metadata, binary


def measure(args, metadata, binary):
    for relative, expected in metadata["harness_sha256"].items():
        assert digest(HERE / relative) == expected, relative
    assert digest(HERE / "rust/Cargo.lock") == metadata["rust_lock_sha256"]
    assert digest(binary) == metadata["rust_executable_sha256"]
    for case in metadata["cases"]:
        directory = args.out / case["name"]
        assert digest(directory / "input") == case["input_sha256"]
        assert digest(directory / "reference.deflate") == case["reference_sha256"]
        source = Path(case["roc_build_command"][-2]).parent
        for filename in ["input", "prefix", "reference.deflate", "app.roc"]:
            assert (source / filename).read_bytes() == (directory / filename).read_bytes()
    process = None
    if args.pause_pid:
        result = subprocess.run(["ps", "-p", str(args.pause_pid), "-o", "pid=,ppid=,etime=,time=,command="], capture_output=True, text=True)
        process = result.stdout.strip()
        assert result.returncode == 0 and process.split()[1] == "63322" and process.endswith("/private/tmp/mech-live-control-target/release/complete-reality"), process
        os.kill(args.pause_pid, signal.SIGSTOP)
    run = {"started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "rounds": args.rounds, "paused_process": process, "samples": 0, "runner_sha256": digest(__file__)}
    rows = []
    try:
        raw = args.out / "raw"
        raw.mkdir(exist_ok=True)
        for round_number in range(args.rounds):
            for case in metadata["cases"]:
                name = case["name"]
                directory = args.out / name
                engines = ["roc-deflate", "miniz-oxide", "zlib-rs"]
                offset = round_number % len(engines)
                engines = engines[offset:] + engines[:offset]
                for engine in engines:
                    if engine == "roc-deflate":
                        assert digest(directory / "rocbench") == case["roc_executable_sha256"]
                        command = [directory / "rocbench", 12]
                    else:
                        assert digest(binary) == metadata["rust_executable_sha256"]
                        command = [binary, engine, directory / "input", directory / "prefix", case["repetitions"], directory / f"{engine}.deflate", directory / "reference.deflate"]
                    started = time.monotonic()
                    result = call(command, capture_output=True, text=True)
                    (raw / f"{round_number}-{name}-{engine}.log").write_text(result.stdout + result.stderr)
                    samples = [line.split("\t") for line in result.stdout.splitlines() if line.startswith("rep\t")]
                    assert len(samples) == case["repetitions"], result.stdout
                    assert {int(row[2]) for row in samples} == set(range(case["repetitions"]))
                    for _, observed_engine, rep, ns, size in samples:
                        assert observed_engine == engine and int(ns) >= 0
                        if engine == "roc-deflate":
                            assert int(size) == case["reference_raw_bytes"]
                        rows.append({"case": name, "input_bytes": case["input_bytes"], "engine": engine, "round": round_number, "rep": int(rep), "ns": int(ns), "raw_bytes": int(size)})
                    if engine != "roc-deflate":
                        output = (directory / f"{engine}.deflate").read_bytes()
                        prefix = (directory / "prefix").read_bytes()
                        assert output[:len(prefix)] == prefix
                        assert zlib.decompress(output[len(prefix):], -15) == (directory / "input").read_bytes()
                    run["samples"] = len(rows)
                    (args.out / "samples.json").write_text(json.dumps(rows, indent=2) + "\n")
                    print("MEASURED", round_number, name, engine, f"{time.monotonic()-started:.3f}s", flush=True)
    finally:
        if args.pause_pid and process:
            os.kill(args.pause_pid, signal.SIGCONT)
            run["resumed_pid"] = args.pause_pid
        run["finished_utc"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
        (args.out / "run.json").write_text(json.dumps(run, indent=2) + "\n")
    summary = []
    for case in metadata["cases"]:
        for engine in ["roc-deflate", "miniz-oxide", "zlib-rs"]:
            samples = [r for r in rows if r["case"] == case["name"] and r["engine"] == engine]
            assert len(samples) == args.rounds * case["repetitions"]
            assert len({r["raw_bytes"] for r in samples}) == 1
            ns = [r["ns"] for r in samples]
            directory = args.out / case["name"]
            output = directory / ("reference.deflate" if engine == "roc-deflate" else f"{engine}.deflate")
            summary.append({"case": case["name"], "input_bytes": case["input_bytes"], "engine": engine, "samples": len(ns), "median_ns": statistics.median(ns), "min_ns": min(ns), "max_ns": max(ns), "raw_bytes": samples[0]["raw_bytes"], "output_sha256": digest(output), "exact_reference": output.read_bytes() == (directory / "reference.deflate").read_bytes(), "independent_decompression": True})
    (args.out / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    with (args.out / "summary.csv").open("w") as f:
        writer = csv.DictWriter(f, fieldnames=list(summary[0]))
        writer.writeheader()
        writer.writerows(summary)
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--roc", type=Path, required=True)
    parser.add_argument("--corpus", type=Path, required=True)
    parser.add_argument("--full-reference", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--rounds", type=int, default=3)
    parser.add_argument("--reps", type=int, default=11)
    parser.add_argument("--full-reps", type=int, default=5)
    parser.add_argument("--pause-pid", type=int)
    parser.add_argument("--measure-only", action="store_true", help="check and measure unchanged successful builds from this output directory")
    args = parser.parse_args()
    assert args.rounds > 0 and args.reps > 0 and args.full_reps > 0
    if args.measure_only:
        metadata = json.loads((args.out / "build.json").read_text())
        binary = args.out / "rust-target/release/deflate-comparison"
    else:
        metadata, binary = prepare(args)
    measure(args, metadata, binary)
