#!/usr/bin/env python3
"""Build and compare the saved CLBG sources before and after application syntax.

Builds and timing are separate phases. Historical reports and chart data are
never changed. All emitted programs use the ordinary AOT compiler and runtime.
"""

import argparse
import collections
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import random
import re
import statistics
import subprocess
import tempfile
import time
from datetime import datetime, timezone


CLBG = Path(__file__).resolve().parent
ROOT = CLBG.parents[1]
DEFAULT_AUDIT = CLBG / "function-application-audit/20261009T171437Z"
DEFAULT_COMPILER = CLBG / "nbody/mech-driver/target/release/mech"
DEFAULT_NBODY_DRIVER = CLBG / "nbody/mech-driver/target/release/mech-clbg-nbody-compiled"
POWER = {"source": "AC Power", "low_power_mode": 0}
FULL_N = {
    "nbody": 50_000_000, "mandelbrot": 16_000, "knucleotide": 25_000_000,
    "spectralnorm": 5_500, "revcomp": 25_000_000, "fannkuchredux": 12,
    "fasta": 25_000_000, "binarytrees": 21, "regexredux": 5_000_000,
    "pidigits": 10_000,
}
EDGE_N = {
    "nbody": [0, 1, 1_000], "mandelbrot": [0, 1, 7, 8, 9, 31, 200],
    "knucleotide": [1_000], "spectralnorm": [1, 3, 7, 100],
    "revcomp": [1_000], "fannkuchredux": [1, 2, 3, 7, 9],
    "fasta": [0, 1, 59, 60, 61, 1_000], "binarytrees": [0, 6, 10],
    "regexredux": [30], "pidigits": [0, 1, 9, 10, 11, 30, 100],
}


def sha(path):
    with Path(path).open("rb") as handle:
        return hashlib.file_digest(handle, "sha256").hexdigest()


def source_loc(path):
    # The benchmark's existing physical-line metric. Sources contain no block
    # comments or strings spanning multiple physical lines.
    return sum(bool(line.strip()) and not line.lstrip().startswith("--")
               for line in Path(path).read_text().splitlines())


def save(path, value):
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    temporary.replace(path)


def guard_module():
    spec = importlib.util.spec_from_file_location("clbg_application_guard", CLBG / "compact/run.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def input_path(benchmark, full):
    if benchmark in ("knucleotide", "revcomp"):
        return CLBG / ("knucleotide/.build/knucleotide-input25000000.txt" if full
                       else "knucleotide/inputs/knucleotide-input-1000.txt")
    if benchmark == "regexredux":
        return CLBG / ("regexredux/.build/input5000000.txt" if full
                       else "regexredux/inputs/regexredux-input-small.txt")
    return None


def check_original_pins(audit, baseline):
    for row in baseline["programs"]:
        if sha(audit / row["binary_copy"]) != row["binary_copy_sha256"]:
            raise ValueError("Saved binary changed: " + row["key"])
        if row.get("current_binary_copy") and sha(audit / row["current_binary_copy"]) != row["current_binary_sha256"]:
            raise ValueError("Saved current canonical binary changed: " + row["key"])
        for source in row["published_sources"]:
            if sha(audit / source["copy"]) != source["sha256"]:
                raise ValueError("Saved source changed: " + source["path"])
    pins = json.loads((audit / "input-pins.json").read_text())
    for name, expected in pins.items():
        path = CLBG / name
        if path.stat().st_size != expected["bytes"] or sha(path) != expected["sha256"]:
            raise ValueError("Pinned input/reference changed: " + name)


def current_source(row):
    # Recorded command source is the canonical source, whereas the published
    # binarytrees endpoint used archived callback syntax. Keep that distinction.
    for command in (row.get("generation_command"), row.get("compile_command")):
        if command and len(command) > 2 and command[1] == "build":
            return Path(command[2])
    return CLBG / row["published_sources"][0]["path"]


def build_environment(row):
    env = dict(os.environ)
    env.update(CARGO_TARGET_DIR=str(CLBG / "nbody/mech-driver/target"), RUSTFLAGS="-D warnings")
    if row["benchmark"] in ("regexredux", "pidigits"):
        env.update(C_INCLUDE_PATH="/opt/homebrew/include", LIBRARY_PATH="/opt/homebrew/lib",
                   RUSTFLAGS="-C target-cpu=native -L native=/opt/homebrew/lib")
    env.update(row.get("build_environment", {}))
    # The CLI's system-gmp feature is inherited by all generated applications.
    # Supply its existing Homebrew installation to every paired build, even
    # when that particular source does not perform bigint operations.
    if "-L native=/opt/homebrew/lib" not in env["RUSTFLAGS"]:
        env["RUSTFLAGS"] += " -L native=/opt/homebrew/lib"
    env.setdefault("LIBRARY_PATH", "/opt/homebrew/lib")
    env.setdefault("C_INCLUDE_PATH", "/opt/homebrew/include")
    return env


def build_one(args, row, source, binary):
    env = build_environment(row)
    binary.parent.mkdir(parents=True, exist_ok=True)
    log = binary.with_suffix(".build.log")
    if row["benchmark"] == "nbody":
        c_path, object_path = binary.with_suffix(".c"), binary.with_suffix(".o")
        generation = list(row["generation_command"])
        generation[0] = str(args.nbody_driver)
        generation[2:6] = [str(source), generation[3], str(c_path), str(object_path)]
        compilation = list(row["compile_command"])
        compilation = [str(c_path) if value.endswith(".c") else
                       str(object_path) if value.endswith(".o") else value
                       for value in compilation]
        compilation[compilation.index("-o") + 1] = str(binary)
        commands = [generation, compilation]
    else:
        commands = [[str(args.compiler), "build", str(source), "--backend", "cranelift",
                     "--offline", "--keep-project", "--no-config", "-o", str(binary)]]
        original = row.get("generation_command") or row.get("compile_command") or []
        for flag in ("--scalar", "--fp-contract-loop"):
            if flag in original:
                commands[0].append(flag)
    with log.open("w") as handle:
        for command in commands:
            subprocess.run(command, cwd=ROOT, env=env, stdout=handle,
                           stderr=subprocess.STDOUT, check=True)
    project = Path(str(binary) + ".project")
    artifacts = {str(path.relative_to(project)): sha(path)
                 for path in project.rglob("*") if path.is_file()
                 and path.suffix in (".o", ".json", ".mec", ".rs")}
    if row["benchmark"] == "nbody":
        artifacts = {object_path.name: sha(object_path)}
    return dict(source=str(source), source_sha256=sha(source), loc=source_loc(source),
                commands=commands, build_environment={key: env[key] for key in env
                if key.startswith("MECH_") or key in ("RUSTFLAGS", "CARGO_TARGET_DIR")},
                binary=str(binary), binary_sha256=sha(binary), artifacts=artifacts)


def do_build(args, baseline, path):
    if path.exists():
        report = json.loads(path.read_text())
        if report["compiler_sha256"] != sha(args.compiler):
            raise ValueError("Compiler changed; use a new --run name")
    else:
        report = dict(created_at=datetime.now(timezone.utc).isoformat(),
                      compiler=str(args.compiler), compiler_sha256=sha(args.compiler),
                      nbody_driver=str(args.nbody_driver), nbody_driver_sha256=sha(args.nbody_driver),
                      baseline_sha256=sha(args.audit / "baseline.json"), programs=[])
    existing = {row["key"]: row for row in report["programs"]}
    for row in baseline["programs"]:
        if args.only and row["benchmark"] not in args.only.split(","):
            continue
        result = existing.get(row["key"], dict(key=row["key"], benchmark=row["benchmark"], builds={}))
        sources = {"before": args.audit / row["published_sources"][0]["copy"],
                   "after": current_source(row)}
        if row["benchmark"] == "binarytrees":
            sources["paired-callback"] = args.audit / "migrated-endpoints/binarytrees/binarytrees-parallel.mec"
            sources["current-before"] = args.audit / row["current_source_before"]["copy"]
            result["source_distinction"] = "before: published callback program; after: current match/comprehension program; paired-callback: only application syntax changed"
        existing[row["key"]] = result
        report["programs"] = list(existing.values())
        for variant, source in sources.items():
            if variant in result["builds"]:
                built = result["builds"][variant]
                if sha(source) != built["source_sha256"] or sha(built["binary"]) != built["binary_sha256"]:
                    raise ValueError("Build/source changed; use a new --run name: " + row["key"])
                continue
            identical = next((built for built in result["builds"].values()
                              if built["source_sha256"] == sha(source)), None)
            if identical:
                result["builds"][variant] = dict(identical, source=str(source), reused_identical_source=True)
                continue
            binary = args.audit / args.run / "builds" / row["benchmark"] / (row["id"] + "-" + variant)
            if sha(args.compiler) != report["compiler_sha256"]:
                raise ValueError("Compiler changed during builds; use a new --run name")
            print("BUILD", row["key"], variant, flush=True)
            result["builds"][variant] = build_one(args, row, source, binary)
            save(path, report)
        existing[row["key"]] = result
        report["programs"] = list(existing.values())
        save(path, report)
    report.setdefault("generic_examples", {})
    for name in ("effects", "allocation"):
        source = args.audit / "generic-examples" / (name + ".mec")
        if name in report["generic_examples"]:
            previous = report["generic_examples"][name]
            if sha(source) != previous["source_sha256"] or sha(previous["binary"]) != previous["binary_sha256"]:
                raise ValueError("Generic example changed; use a new --run name")
            continue
        binary = args.audit / args.run / "builds/generic" / name
        print("BUILD generic", name, flush=True)
        report["generic_examples"][name] = build_one(args, {"benchmark": "generic"}, source, binary)
        save(path, report)
    report["builds_complete"] = True
    save(path, report)


def execute(command, benchmark, n, env, timeout, guard, watch=False):
    full = n == FULL_N[benchmark]
    ip = input_path(benchmark, full)
    argv = [value.replace("{n}", str(n)) for value in command]
    monitor = guard.CompilerMonitor(POWER) if watch else None
    with tempfile.TemporaryFile() as out, tempfile.TemporaryFile() as err:
        stdin = ip.open("rb") if ip else subprocess.DEVNULL
        try:
            if monitor:
                monitor.start()
            started = time.perf_counter_ns()
            returncode = 0
            try:
                guard.run_process(argv, stdin, out, err, timeout, env)
            except subprocess.CalledProcessError as error:
                returncode = error.returncode
            elapsed = (time.perf_counter_ns() - started) / 1e9
        finally:
            if ip:
                stdin.close()
            if monitor:
                monitor.finish()
        out.seek(0, os.SEEK_END)
        size = out.tell()
        out.seek(0)
        digest = hashlib.file_digest(out, "sha256").hexdigest()
        out.seek(0)
        # Small numerical outputs are retained; large PBM/FASTA output is hashed.
        output = out.read().decode() if benchmark in ("nbody", "spectralnorm", "fannkuchredux") else None
        err.seek(0)
        stderr = err.read().decode(errors="replace")
    contaminated = bool(monitor and (monitor.observations or monitor.power_changes))
    return dict(seconds=elapsed, returncode=returncode, bytes=size, sha256=digest, output=output, stderr=stderr,
                contaminated=contaminated,
                competing_compilers=monitor.observations if monitor else [],
                power_changes=monitor.power_changes if monitor else [])


def compare(actual, expected, benchmark):
    if actual.get("returncode", 0) != expected.get("returncode", 0):
        raise ValueError(f"{benchmark} exit status differs: {actual.get('returncode')} {actual.get('stderr')}")
    if expected.get("returncode", 0):
        if (actual["bytes"], actual["sha256"], actual["stderr"]) != (expected["bytes"], expected["sha256"], expected["stderr"]):
            raise ValueError("Expected rejection differs: " + benchmark)
    elif benchmark == "nbody":
        numbers = [float(line) for line in actual["output"].splitlines() if line.strip()]
        reference = [float(line) for line in expected["output"].splitlines() if line.strip()]
        if len(numbers) != len(reference) or any(abs(a-b) > 1e-8 for a,b in zip(numbers,reference)):
            raise ValueError("N-body output differs beyond 1e-8")
    elif (actual["bytes"], actual["sha256"]) != (expected["bytes"], expected["sha256"]):
        raise ValueError(f"Complete {benchmark} output differs: {actual['bytes']} {actual['sha256']}")


def pinned_reference(row, n):
    benchmark = row["benchmark"]
    if n != FULL_N[benchmark]:
        return None
    if benchmark == "nbody":
        return dict(output="-0.169075164\n-0.169059907\n")
    if benchmark in ("regexredux", "pidigits"):
        p = CLBG / benchmark / "inputs/output-full.txt"
        return dict(bytes=p.stat().st_size, sha256=sha(p))
    return row.get("correctness_full")


def do_verify(args, baseline, builds, path):
    guard = guard_module()
    env = guard.ENV
    report = json.loads(path.read_text()) if path.exists() else dict(
        created_at=datetime.now(timezone.utc).isoformat(), build_report_sha256=sha(builds),
        protocol=dict(full_workloads=args.full, timings=False, source_metric="Physical LOC excluding blank/comment-only lines",
                      comparison="Complete stdout byte equality, except N-body energies within 1e-8"), programs=[])
    if report["build_report_sha256"] != sha(builds):
        raise ValueError("Build report changed; use a new --run name")
    build_report = json.loads(builds.read_text())
    built = {row["key"]: row for row in build_report["programs"]}
    completed = {row["key"] for row in report["programs"]}
    for row in baseline["programs"]:
        if (row["key"] not in built or row["key"] in completed
                or (args.only and row["benchmark"] not in args.only.split(","))):
            continue
        runtime = env | row.get("runtime_environment", {})
        record = dict(key=row["key"], cases=[])
        sizes = list(EDGE_N[row["benchmark"]])
        if args.full:
            sizes.append(FULL_N[row["benchmark"]])
        for n in sizes:
            reference = execute(row["baseline_command"], row["benchmark"], n, runtime, args.timeout, guard)
            if reference["returncode"] and not (row["benchmark"] == "mandelbrot" and n == 0):
                raise ValueError("Saved baseline rejected a valid case: " + row["key"] + " " + reference["stderr"])
            pin = pinned_reference(row, n)
            if pin:
                compare(reference, pin, row["benchmark"])
            case = dict(n=n, input=str(input_path(row["benchmark"], n == FULL_N[row["benchmark"]])),
                        reference={k: reference[k] for k in ("bytes", "sha256", "output", "returncode", "stderr")},
                        expected_rejection=bool(reference["returncode"]), variants={})
            for variant, build in built[row["key"]]["builds"].items():
                if sha(build["binary"]) != build["binary_sha256"]:
                    raise ValueError("Compiled binary changed: " + build["binary"])
                command = [build["binary"], *row["baseline_command"][1:]]
                actual = execute(command, row["benchmark"], n, runtime, args.timeout, guard)
                try:
                    compare(actual, reference, row["benchmark"])
                except ValueError as error:
                    raise ValueError(f"{row['key']} {variant} N={n}: {error}") from error
                case["variants"][variant] = {k: actual[k] for k in ("bytes", "sha256", "output", "returncode", "stderr")}
            if row.get("current_binary_command"):
                actual = execute(row["current_binary_command"], row["benchmark"], n, runtime, args.timeout, guard)
                compare(actual, reference, row["benchmark"])
                case["variants"]["saved-current"] = {k: actual[k] for k in ("bytes", "sha256", "output", "returncode", "stderr")}
            record["cases"].append(case)
            print("PASS", row["key"], "N=" + str(n), flush=True)
        report["programs"].append(record)
        save(path, report)
    expected = json.loads((args.audit / "generic-examples/expected.json").read_text())
    report["generic_examples"] = {}
    for name, build in build_report.get("generic_examples", {}).items():
        if sha(build["binary"]) != build["binary_sha256"]:
            raise ValueError("Generic example binary changed: " + name)
        actual = subprocess.run([build["binary"]], cwd=ROOT, env=env,
                                capture_output=True, timeout=args.timeout, check=True)
        if actual.stdout != expected[name].encode():
            raise ValueError("Evaluation-once example differs: " + name + " " + repr(actual.stdout))
        report["generic_examples"][name] = dict(output=actual.stdout.decode(), passed=True)
        print("PASS evaluation-once", name, flush=True)
    report["status"] = "passed"
    save(path, report)


def do_measure(args, baseline, builds, verification, path):
    guard = guard_module()
    if not verification.exists():
        raise ValueError("Run --verify --full before measurement")
    verified = json.loads(verification.read_text())
    if not verified["protocol"]["full_workloads"] or verified["status"] != "passed":
        raise ValueError("Full verification must pass before measurement")
    built = {row["key"]: row for row in json.loads(builds.read_text())["programs"]}
    report = json.loads(path.read_text()) if path.exists() else dict(
        created_at=datetime.now(timezone.utc).isoformat(), build_report_sha256=sha(builds),
        verification_sha256=sha(verification),
        protocol=dict(samples=args.samples, power=POWER, required_workloads=FULL_N,
                      workers=8, metric="Physical LOC excluding blank/comment-only lines",
                      selected_variants=args.timing_variants,
                      timing="Sequential fresh process to exit with complete stdout to regular file; builds and output hashing excluded",
                      ordering="Randomized complete rounds; published binary plus new compiler before/after sources",
                      compiler_guard="Reject any sample with detected competing compiler or power change"), programs=[])
    if report["protocol"]["samples"] != args.samples or report["build_report_sha256"] != sha(builds):
        raise ValueError("Measurement settings changed; use a new --run name")
    if report["protocol"].get("selected_variants") != args.timing_variants:
        raise ValueError("Measurement variants changed; use a new --timing-report name")
    completed = {row["key"] for row in report["programs"]}
    rng = random.Random(20261009)
    for row in baseline["programs"]:
        if (row["key"] not in built or row["key"] in completed
                or (args.only and row["benchmark"] not in args.only.split(","))):
            continue
        bench = row["benchmark"]
        runtime = guard.ENV | row.get("runtime_environment", {})
        variants = {"published": dict(binary=row["baseline_command"][0],
                    binary_sha256=row["binary_copy_sha256"], loc=row["old_loc"]),
                    **built[row["key"]]["builds"]}
        if row.get("current_binary_command"):
            variants["saved-current"] = dict(binary=row["current_binary_command"][0],
                binary_sha256=row["current_binary_sha256"], loc=row["current_source_before"]["loc"])
        if args.timing_variants:
            selected = args.timing_variants.split(",")
            missing = set(selected) - set(variants)
            if missing:
                raise ValueError("Unavailable timing variants: " + repr(sorted(missing)))
            variants = {name: variants[name] for name in selected}
        guard.wait_for_environment(POWER)
        reference = execute(row["baseline_command"], bench, FULL_N[bench], runtime, args.timeout, guard)
        record = dict(key=row["key"], benchmark=bench, source_distinction=built[row["key"]].get("source_distinction"),
                      variants={key: dict(loc=value["loc"], binary_sha256=value["binary_sha256"],
                                samples_seconds=[]) for key,value in variants.items()}, discarded=[], order=[])
        for index in range(args.samples):
            order = list(variants)
            rng.shuffle(order)
            record["order"].append(order)
            for variant in order:
                candidate = variants[variant]
                command = [candidate["binary"], *row["baseline_command"][1:]]
                while True:
                    guard.wait_for_environment(POWER)
                    if sha(candidate["binary"]) != candidate["binary_sha256"]:
                        raise ValueError("Compiled binary changed: " + candidate["binary"])
                    result = execute(command, bench, FULL_N[bench], runtime, args.timeout, guard, watch=True)
                    compare(result, reference, bench)
                    if result["contaminated"]:
                        record["discarded"].append(dict(variant=variant, sample=index, **result))
                        continue
                    record["variants"][variant]["samples_seconds"].append(result["seconds"])
                    break
                print("SAMPLE", row["key"], variant, index + 1, f"{result['seconds']:.6f}", flush=True)
        for result in record["variants"].values():
            samples = result["samples_seconds"]
            result.update(median_seconds=statistics.median(samples), stdev_seconds=statistics.stdev(samples))
        for variant, result in record["variants"].items():
            if "published" in record["variants"]:
                result["time_ratio_to_published"] = result["median_seconds"] / record["variants"]["published"]["median_seconds"]
            if "before" in record["variants"]:
                result["time_ratio_to_before"] = result["median_seconds"] / record["variants"]["before"]["median_seconds"]
        report["programs"].append(record)
        save(path, report)
    report["status"] = "complete"
    save(path, report)


def do_inspect(args, builds, path):
    """Inspect generated native objects, excluding linked library/runtime code."""
    objdump = Path("/opt/homebrew/opt/llvm@22/bin/llvm-objdump")
    if not objdump.is_file():
        raise ValueError("Object inspection requires the existing llvm-objdump")
    report = dict(build_report_sha256=sha(builds), tool=str(objdump), programs=[])
    for row in json.loads(builds.read_text())["programs"]:
        record = dict(key=row["key"], variants={})
        for variant, build in row["builds"].items():
            objects = {}
            for relative, digest in build["artifacts"].items():
                if not relative.endswith(".o"):
                    continue
                object_path = Path(build["binary"] + ".project") / relative
                if not object_path.is_file():
                    object_path = Path(build["binary"]).parent / relative
                if sha(object_path) != digest:
                    raise ValueError("Object changed: " + str(object_path))
                disassembly = subprocess.check_output(
                    [str(objdump), "--no-show-raw-insn", "--disassemble", str(object_path)], text=True)
                dump = args.audit / args.run / "disassembly" / row["benchmark"] / (row["key"].split(":", 1)[1] + "-" + variant + "-" + Path(relative).stem + ".txt")
                dump.parent.mkdir(parents=True, exist_ok=True)
                dump.write_text(disassembly)
                instructions = []
                symbols = collections.Counter()
                symbol = "unknown"
                for line in disassembly.splitlines():
                    label = re.match(r"^[0-9a-f]+ <([^>]+)>:", line)
                    if label:
                        symbol = label[1]
                    instruction = re.match(r"^\s*[0-9a-f]+:\s+(.+)", line)
                    if instruction:
                        instructions.append(instruction[1].split(";", 1)[0].strip())
                        symbols[symbol] += 1
                opcodes = collections.Counter(line.split()[0] for line in instructions)
                objects[relative] = dict(
                    sha256=digest, instructions=len(instructions), symbols=dict(symbols),
                    opcodes=dict(sorted(opcodes.items())), disassembly=str(dump),
                    instruction_sequence_sha256=hashlib.sha256("\n".join(instructions).encode()).hexdigest(),
                    floating_divides=sum(n for op,n in opcodes.items() if op.startswith("fdiv")),
                    floating_sqrt=sum(n for op,n in opcodes.items() if op.startswith("fsqrt")),
                    direct_calls=opcodes.get("bl", 0),
                    stack_accesses=sum("[sp" in line for line in instructions))
            record["variants"][variant] = objects
        before = list(record["variants"]["before"].values())
        record["comparison_to_before"] = {}
        for variant, objects in record["variants"].items():
            current = list(objects.values())
            record["comparison_to_before"][variant] = dict(
                native_objects_byte_identical=[x["sha256"] for x in current] == [x["sha256"] for x in before],
                instruction_sequences_identical=[x["instruction_sequence_sha256"] for x in current] == [x["instruction_sequence_sha256"] for x in before],
                static_instruction_count=sum(x["instructions"] for x in current))
        if "current-before" in record["variants"]:
            current_before = list(record["variants"]["current-before"].values())
            after = list(record["variants"]["after"].values())
            record["after_vs_current_before"] = dict(
                native_objects_byte_identical=[x["sha256"] for x in after] == [x["sha256"] for x in current_before],
                instruction_sequences_identical=[x["instruction_sequence_sha256"] for x in after] == [x["instruction_sequence_sha256"] for x in current_before])
        report["programs"].append(record)
        print("OBJECT", row["key"], record["comparison_to_before"], flush=True)
    save(path, report)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--audit", type=Path, default=DEFAULT_AUDIT)
    parser.add_argument("--run", default="application-v1")
    parser.add_argument("--compiler", type=Path, default=DEFAULT_COMPILER)
    parser.add_argument("--nbody-driver", type=Path, default=DEFAULT_NBODY_DRIVER)
    parser.add_argument("--only", help="Comma-separated benchmark keys")
    parser.add_argument("--build", action="store_true")
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--full", action="store_true")
    parser.add_argument("--measure", action="store_true")
    parser.add_argument("--inspect", action="store_true", help="Disassemble and compare generated native objects")
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--timing-variants", help="Comma-separated variant names for a focused existing-binary comparison")
    parser.add_argument("--timing-report", default="timings.json", help="New report filename inside the audit run")
    parser.add_argument("--timeout", type=int, default=900)
    args = parser.parse_args()
    if not (args.build or args.verify or args.measure or args.inspect):
        parser.error("Choose --build, --verify, --inspect, or --measure")
    if args.build and args.measure:
        parser.error("Build and timing must be separate invocations")
    if args.samples < 3:
        parser.error("At least three timing samples are required")
    if Path(args.timing_report).name != args.timing_report:
        parser.error("--timing-report must be a filename inside the audit run")
    directory = args.audit / args.run
    directory.mkdir(parents=True, exist_ok=True)
    baseline = json.loads((args.audit / "baseline.json").read_text())
    check_original_pins(args.audit, baseline)
    builds = directory / "builds.json"
    verification = directory / "verification.json"
    if args.build:
        do_build(args, baseline, builds)
    if args.verify:
        do_verify(args, baseline, builds, verification)
    if args.inspect:
        do_inspect(args, builds, directory / "objects.json")
    if args.measure:
        do_measure(args, baseline, builds, verification, directory / args.timing_report)


if __name__ == "__main__":
    main()
