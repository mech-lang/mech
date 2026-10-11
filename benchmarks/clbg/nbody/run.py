#!/usr/bin/env python3
"""Validate and time pinned n-body programs, then select size/speed endpoints."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import io
import json
import math
import os
import platform
import random
import re
import shutil
import statistics
import subprocess
import tarfile
import time
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
AOT_MODES = ("aot", "aot-simd", "llvm-aot")
CRANELIFT_AOT_MODES = ("aot", "aot-simd")
JIT_MODES = ("jit", "jit-simd")

def compiler_sources():
    paths = [ROOT / "hosts/gpu/Cargo.toml", ROOT / "hosts/gpu/src/batched/aot.rs", ROOT / "hosts/gpu/src/batched/jit.rs", ROOT / "hosts/gpu/src/bin/mech-native-f64.rs"] + sorted((ROOT / "hosts/gpu/src/fused_f64").glob("*.*"))
    return {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}

def macos_gcc_flags(compiler):
    if platform.system() != "Darwin": return []
    version = command([compiler, "--version"]).stdout.lower()
    sdk = command(["xcrun", "--show-sdk-path"]).stdout.strip()
    flags = ["-isysroot", sdk]
    if "clang" not in version and not (Path(sdk) / "usr/include/_bounds.h").exists():
        # GCC 16's bottle was configured against SDK 26. The installed SDK 14
        # has its own stdio layout and lacks quick_exit declarations. Use the
        # installed stdio definitions and disable only those unused libstdc++
        # declarations; benchmark sources and arithmetic remain unchanged.
        header = HERE / ".build/gcc-sdk-compat.h"
        header.parent.mkdir(exist_ok=True)
        header.write_text('#include ' + json.dumps(sdk + '/usr/include/_stdio.h') + '\n#ifdef __cplusplus\n#include <bits/c++config.h>\n#undef _GLIBCXX_HAVE_AT_QUICK_EXIT\n#undef _GLIBCXX_HAVE_QUICK_EXIT\n#endif\n')
        flags += ["-include", str(header)]
    return flags

def compiler_snapshot(output):
    """Retain the exact uncommitted compiler, runtime, and benchmark inputs."""
    names = list(compiler_sources()) + ["Cargo.lock", "hosts/gpu/src/lib.rs", "hosts/gpu/src/batched/mod.rs", "scripts/unsafe-boundaries.allowlist"]
    names += [str(path.relative_to(ROOT)) for path in sorted(HERE.glob("*.mec"))]
    names += [str(path.relative_to(ROOT)) for path in sorted(HERE.glob("*.py"))]
    names += [str((HERE / path).relative_to(ROOT)) for path in ("manifest.json", "mech-driver/Cargo.toml", "mech-driver/Cargo.lock", "mech-driver/src/compiled-main.rs", "mech-driver/tests/fused.rs")]
    if (HERE / ".build/gcc-sdk-compat.h").exists(): names.append(str((HERE / ".build/gcc-sdk-compat.h").relative_to(ROOT)))
    archive = output.with_suffix(".compiler.tar.gz")
    archive.parent.mkdir(parents=True, exist_ok=True)
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w") as tar:
        for name in sorted(set(names)):
            contents = (ROOT / name).read_bytes()
            info = tarfile.TarInfo(name)
            info.size = len(contents)
            info.mode = 0o644
            tar.addfile(info, io.BytesIO(contents))
    archive.write_bytes(gzip.compress(buffer.getvalue(), compresslevel=6, mtime=0))
    return {"path": archive.name, "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
            "base_commit": command(["git", "rev-parse", "HEAD"]).stdout.strip(),
            "note": "Exact compiler/runtime/source/script snapshot; apply on this v0.4 base. Pinned competitor sources are separately hashed in manifest.json."}

def command(args, *, env=None, timeout=600):
    return subprocess.run(args, cwd=ROOT, env=env, check=True, text=True,
                          capture_output=True, timeout=timeout)

def strip_comments(source, language):
    """Small lexical scanner; retain quoted strings and comment newlines."""
    output = []
    index = 0
    line_comment = "#" if language == "julia" else "--" if language == "mech" else "//"
    while index < len(source):
        if source.startswith(line_comment, index):
            end = source.find("\n", index)
            index = len(source) if end < 0 else end
        elif source.startswith("/*", index) or (language == "julia" and source.startswith("#=", index)):
            opener, closer = ("#=", "=#") if language == "julia" else ("/*", "*/")
            end = source.find(closer, index + 2)
            if end < 0:
                raise ValueError("unterminated block comment")
            output.append(" " + "\n" * source[index:end + 2].count("\n"))
            index = end + 2
        elif source[index] in ('"', '`') or (source[index] == "'" and re.match(r"'(?:\\.|[^'\n])'", source[index:])):
            quote = source[index]
            start = index
            index += 1
            while index < len(source):
                if source[index] == "\\" and quote != '`':
                    index += 2
                elif source[index] == quote:
                    index += 1
                    break
                else:
                    index += 1
            output.append(source[start:index])
        else:
            output.append(source[index])
            index += 1
    return "".join(output)

def metrics(paths):
    sources = [(path, (HERE / path).read_text()) for path in paths]
    clean = "\n".join(strip_comments(text, "mech" if path.endswith(".mec") else
                                   "julia" if path.endswith(".jl") else "c")
                      for path, text in sources)
    normalized = re.sub(r"\s+", " ", clean).strip().encode("utf-8")
    return {
        "paths": paths,
        "sha256": {path: hashlib.sha256(text.encode()).hexdigest() for path, text in sources},
        "physical_lines": sum(len(text.splitlines()) for _, text in sources),
        "nonblank_noncomment_lines": sum(bool(line.strip()) for line in clean.splitlines()),
        "normalized_bytes": len(normalized),
        "gzip1_bytes": len(gzip.compress(normalized, compresslevel=1, mtime=0)),
    }

def energies(output):
    lines = output.strip().splitlines()
    if len(lines) != 2:
        raise ValueError(f"expected exactly two energy lines, got {output!r}")
    values = [float(x) for x in lines]
    if not all(math.isfinite(x) for x in values):
        raise ValueError("nonfinite energy")
    return values

def validate(output, expected):
    actual = energies(output)
    if any(abs(a - b) > 1e-8 for a, b in zip(actual, expected)):
        raise ValueError(f"energy mismatch: {actual} != {expected} within 1e-8")
    return actual

def endpoint_summary(rows):
    baseline_time = min(row["median_seconds"] for row in rows)
    baseline_size = min(row["source"]["gzip1_bytes"] for row in rows)
    endpoints = {}
    for language in sorted({row["language"] for row in rows}):
        candidates = [row for row in rows if row["language"] == language]
        fastest = min(candidates, key=lambda r: (r["median_seconds"], r["source"]["gzip1_bytes"]))
        smallest = min(candidates, key=lambda r: (r["source"]["gzip1_bytes"], r["median_seconds"]))
        speed_cost = smallest["median_seconds"] / fastest["median_seconds"]
        size_cost = fastest["source"]["gzip1_bytes"] / smallest["source"]["gzip1_bytes"]
        endpoints[language] = {
            "candidate_count": len(candidates), "fastest": fastest["id"], "smallest": smallest["id"],
            "same_entry": fastest["id"] == smallest["id"],
            "smallest_time_over_fastest": speed_cost, "fastest_size_over_smallest": size_cost,
            "log2_tradeoff_distance": math.hypot(math.log2(speed_cost), math.log2(size_cost)),
            "points": {name: {"size_ratio": row["source"]["gzip1_bytes"] / baseline_size,
                              "time_ratio": row["median_seconds"] / baseline_time}
                       for name, row in (("fastest", fastest), ("smallest", smallest))},
        }
    return {"time_baseline_seconds": baseline_time, "size_baseline_bytes": baseline_size,
            "normalization_cohort": "Successfully validated local programs in this run only",
            "languages": endpoints}

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--steps", type=int, default=1_000_000)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--seed", type=int, default=20261006)
    parser.add_argument("--timeout", type=int, default=600)
    parser.add_argument("--julia", default=shutil.which("julia"))
    parser.add_argument("--java-home", default=os.environ.get("JAVA_HOME", "/opt/homebrew/opt/openjdk"))
    parser.add_argument("--mech-mode", choices=("aot", "aot-simd", "llvm-aot", "jit", "jit-simd", "resident"), default="aot-simd", help="Cranelift SIMD object emission by default; llvm-aot is an optional comparison")
    parser.add_argument("--mech-fp-contract", action="store_true", help="permit FMA contraction; changes rounding")
    parser.add_argument("--mech-refined-reciprocal", action="store_true", help="permit mixed-precision inverse-cube refinement; changes arithmetic and assumes positive f32-normal inputs")
    parser.add_argument("--mech-schedule", action="store_true", help="schedule independent native arithmetic by demand")
    parser.add_argument("--mech-algebraic-powers", action="store_true", help="permit inverse-power algebraic reduction; changes rounding and intermediate range")
    parser.add_argument("--clang", default="/opt/homebrew/opt/llvm@22/bin/clang-22" if Path("/opt/homebrew/opt/llvm@22/bin/clang-22").exists() else shutil.which("clang"))
    parser.add_argument("--gcc", default=shutil.which("gcc-16") or shutil.which("gcc-15") or shutil.which("gcc-14") or shutil.which("gcc"))
    parser.add_argument("--gpp", default=shutil.which("g++-16") or shutil.which("g++-15") or shutil.which("g++-14") or shutil.which("g++"))
    parser.add_argument("--c-tune", choices=("native", "fastmath"), default="native", help="fastmath adds -ffast-math -funroll-loops -flto to all C/C++ candidates")
    parser.add_argument("--compare-cc", action="store_true", help="measure each C/C++ source with GCC and LLVM under both native O3 and fast-math/unrolling/LTO")
    parser.add_argument("--only", help="comma-separated program IDs; Rust #3 is required as the reference")
    parser.add_argument("--allow-partial", action="store_true", help="write results despite explicitly reported exclusions")
    parser.add_argument("--output", type=Path, default=HERE / "results/pilot.json")
    args = parser.parse_args()
    if args.steps < 0 or args.samples < 3:
        parser.error("steps must be nonnegative and at least three samples are required")
    native_flags = (["--fp-contract"] if args.mech_fp_contract else []) + (["--refined-reciprocal"] if args.mech_refined_reciprocal else []) + (["--schedule"] if args.mech_schedule else []) + (["--algebraic-powers"] if args.mech_algebraic_powers else [])
    c_tuning_flags = ["-ffast-math", "-funroll-loops", "-flto"] if args.c_tune == "fastmath" else []
    if args.mech_mode == "resident" and native_flags:
        parser.error("native optimization flags require a compiled Mech mode")
    environment = os.environ.copy()
    environment.update({"JULIA_NUM_THREADS": "1", "OMP_NUM_THREADS": "1", "OPENBLAS_NUM_THREADS": "1",
                        "CHPL_RT_NUM_THREADS_PER_LOCALE": "1", "GOMAXPROCS": "1"})
    tools = {"rust": shutil.which("rustc"), "go": shutil.which("go"), "chapel": shutil.which("chpl"),
             "julia": args.julia, "java": str(Path(args.java_home) / "bin/java"), "c": args.gcc, "cpp": args.gpp}
    build = HERE / ".build"
    build.mkdir(exist_ok=True)
    programs = json.loads((HERE / "manifest.json").read_text())["programs"]
    if args.compare_cc:
        programs = [candidate for program in programs for candidate in
                    ([{**program, "id": program["id"] + f"-{backend}-{tune}", "upstream_id": program["id"], "cc_backend": backend, "cc_tune": tune}
                      for backend in ("gcc", "llvm") for tune in ("native", "fastmath")]
                     if program["language"] in ("c", "cpp") else [program])]
    programs.extend([
        {"id": f"mech-{args.mech_mode}-sqrt-v04", "language": "mech", "path": "nbody.mec"},
        {"id": f"mech-{args.mech_mode}-power-v04", "language": "mech", "path": "nbody-power.mec"},
        {"id": f"mech-{args.mech_mode}-factored-v04", "language": "mech", "path": "nbody-factored.mec"},
        {"id": f"mech-{args.mech_mode}-pairwise-v04", "language": "mech", "path": "nbody-pairwise.mec"},
    ])
    if args.only:
        selected = set(args.only.split(","))
        programs = [p for p in programs if p["id"] in selected]
    if not any(p["id"] == "nbody-rust-3" for p in programs):
        parser.error("include nbody-rust-3 to supply the validated reference output")
    versions = {}
    if args.clang:
        versions["clang"] = command([args.clang, "--version"]).stdout.strip()
    for language, tool in tools.items():
        if tool:
            try:
                result = command([tool, "version" if language == "go" else "-version" if language == "java" else "--version"])
                versions[language] = (result.stdout + result.stderr).strip()
            except (OSError, subprocess.SubprocessError) as error:
                versions[language] = f"unavailable: {error}"
    gcc_flags = {language: macos_gcc_flags(tools[language]) for language in ("c", "cpp")}
    rows, excluded = [], []
    snapshot = compiler_snapshot(args.output)
    for program in programs:
        identifier, language = program["id"], program["language"]
        source = HERE / program["path"]
        try:
            if "sha256" in program and hashlib.sha256(source.read_bytes()).hexdigest() != program["sha256"]:
                raise ValueError("pinned upstream source SHA256 changed")
            if platform.machine() != "x86_64" and re.search(r'__builtin_ia32|x86intrin|immintrin|emmintrin|std::arch::x86', source.read_text()):
                raise ValueError("target architecture: pinned submission requires x86 SIMD intrinsics; this host is " + platform.machine())
            binary = build / identifier
            compile_command = None
            generation_command = None
            generated_sha256 = None
            object_sha256 = None
            env = environment.copy()
            if language == "rust":
                compile_command = [tools[language], "-C", "opt-level=3", "-C", "target-cpu=native", "-C", "codegen-units=1", str(source), "-o", str(binary)]
                run = [str(binary)]
            elif language in ("c", "cpp"):
                standard = "-std=c++20" if language == "cpp" and program["variant"] == 2 else "-std=c++17" if language == "cpp" else "-std=gnu11"
                cc = args.clang if program.get("cc_backend") == "llvm" else tools[language]
                llvm = "clang" in command([cc, "--version"]).stdout.lower()
                cpu_flag = "-mcpu=native" if llvm and platform.machine() in ("arm64", "aarch64") else "-march=native"
                compile_command = [cc, "-O3", cpu_flag, "-fomit-frame-pointer", standard, str(source), "-lm", "-o", str(binary)]
                compile_command += macos_gcc_flags(cc)
                # Some pinned submissions rely on GCC's transitive standard
                # includes. Supply the standard declaration header uniformly
                # for LLVM C++, without editing benchmark source or arithmetic.
                if llvm and language == "cpp": compile_command += ["-include", "cstdlib", "-lc++"]
                compile_command += ["-ffast-math", "-funroll-loops", "-flto"] if program.get("cc_tune", args.c_tune) == "fastmath" else []
                run = [str(binary)]
            elif language == "go":
                compile_command = [tools[language], "build", "-o", str(binary), str(source)]
                run = [str(binary)]
            elif language == "chapel":
                compile_command = [tools[language], "--fast", str(source), "-o", str(binary)]
                if platform.system() == "Darwin":
                    sdk = command(["xcrun", "--show-sdk-path"]).stdout.strip()
                    compile_command += [f"--ccflags=-isysroot {sdk}", f"--ldflags=-isysroot {sdk}"]
                run = [str(binary)]
            elif language == "julia":
                if not tools[language]:
                    raise ValueError("Julia executable is missing; pass --julia")
                run = [tools[language], "--startup-file=no", "--history-file=no", "-O3", "--math-mode=ieee", "--", str(source)]
                if program["variant"] == 5:
                    env["JULIA_LLVM_ARGS"] = "-unroll-threshold=500"
            elif language == "java":
                binary.mkdir(exist_ok=True)
                java_source = binary / "nbody.java"
                shutil.copyfile(source, java_source)
                compile_command = [str(Path(args.java_home) / "bin/javac"), "-d", str(binary), str(java_source)]
                run = [tools[language], "-XX:ActiveProcessorCount=1", "-cp", str(binary), "nbody"]
            else:
                compile_command = ["cargo", "build", "--locked", "--release", "--manifest-path", str(HERE / "mech-driver/Cargo.toml")]
                run = [str(HERE / "mech-driver/target/release/mech-clbg-nbody-compiled")]
                if args.mech_mode in AOT_MODES:
                    command(compile_command, env=env)
                    generated = build / f"{identifier}.c"
                    native_object = build / f"{identifier}.o"
                    generation_command = run + ["0", str(source), "emit-cranelift-simd" if args.mech_mode == "aot-simd" else "emit-cranelift" if args.mech_mode == "aot" else "emit-c", str(generated)] + ([str(native_object)] if args.mech_mode in CRANELIFT_AOT_MODES else [])
                    generation_command += ["--output", "energy"] + native_flags
                    command(generation_command, env=env)
                    generated_sha256 = hashlib.sha256(generated.read_bytes()).hexdigest()
                    if args.mech_mode in CRANELIFT_AOT_MODES:
                        object_sha256 = hashlib.sha256(native_object.read_bytes()).hexdigest()
                    compile_command = [args.clang, "-O3", "-mcpu=native", "-ffp-contract=off", "-fno-math-errno", str(generated)] + ([str(native_object)] if args.mech_mode in CRANELIFT_AOT_MODES else []) + ["-lm", "-o", str(binary)]
                    if platform.system() == "Darwin":
                        compile_command += ["-isysroot", command(["xcrun", "--show-sdk-path"]).stdout.strip()]
                    run = [str(binary)]
            print(f"Prepare {identifier}", flush=True)
            if compile_command:
                command(compile_command, env=env)
            def invocation(n, prefix=run, lang=language, path=source):
                return prefix + [f"--n={n}" if lang == "chapel" else str(n)] + ([str(path), args.mech_mode, "--output", "energy"] + native_flags if lang == "mech" and args.mech_mode not in AOT_MODES else [])
            check = command(invocation(1000), env=env, timeout=args.timeout)
            validate(check.stdout, [-0.169075164, -0.169087605])
            paths = [program["path"]]
            rows.append({**program, "execution_mode": "OpenJDK JIT" if language == "java" else
                         {"aot": "experimental fused f64 Cranelift AOT from v0.4 artifact", "aot-simd": "experimental fused f64 Cranelift AOT with structured SIMD from v0.4 artifact", "llvm-aot": "experimental fused f64 LLVM AOT from v0.4 artifact", "jit": "experimental fused f64 Cranelift JIT from v0.4 artifact", "jit-simd": "experimental fused f64 Cranelift JIT with structured SIMD from v0.4 artifact", "resident": "v0.4 resident execution without recording"}[args.mech_mode] if language == "mech" else language,
                         "source": metrics(paths), "compile_command": compile_command,
                         "generation_command": generation_command, "generated_c_sha256": generated_sha256,
                         "native_object_sha256": object_sha256,
                         "command": invocation(args.steps), "environment_overrides": {k: v for k, v in env.items() if environment.get(k) != v},
                         "correctness_1000_stdout": check.stdout, "samples_seconds": [], "samples_stdout": [],
                         "_environment": env})
            if language == "mech":
                rows[-1]["output_selection"] = "energy"
                rows[-1]["runtime_support"] = "hosts/gpu/src/fused_f64/runner.rs and runner.c; generic execution/formatting only, excluded from program size"
        except (ValueError, OSError, TypeError, subprocess.SubprocessError) as error:
            detail = (error.stderr or error.stdout) if isinstance(error, subprocess.CalledProcessError) else str(error)
            excluded.append({"id": identifier, "reason": str(error), "diagnostic": detail})
            print(f"Exclude {identifier}: {str(detail)[-800:]}", flush=True)
    reference = next((r for r in rows if r["id"] == "nbody-rust-3"), None)
    if not reference:
        raise SystemExit("Reference program failed; no timings are valid")
    reference_output = command(reference["command"], env=reference["_environment"], timeout=args.timeout).stdout
    expected = energies(reference_output)
    if args.steps == 50_000_000:
        validate(reference_output, [-0.169075164, -0.169059907])
    # Every process is fresh. This discarded run warms file caches; it does not
    # exclude JIT compilation from the later time-to-solution measurements.
    for row in rows:
        result = command(row["command"], env=row["_environment"], timeout=args.timeout)
        validate(result.stdout, expected)
        row["discarded_warmup_stdout"] = result.stdout
    rng = random.Random(args.seed)
    order = []
    for sample in range(args.samples):
        shuffled = rows.copy()
        rng.shuffle(shuffled)
        order.append([r["id"] for r in shuffled])
        for row in shuffled:
            print(f"Sample {sample + 1}/{args.samples} {row['id']}", flush=True)
            start = time.perf_counter_ns()
            result = command(row["command"], env=row["_environment"], timeout=args.timeout)
            elapsed = (time.perf_counter_ns() - start) / 1e9
            validate(result.stdout, expected)
            row["samples_seconds"].append(elapsed)
            row["samples_stdout"].append(result.stdout)
    for row in rows:
        row.pop("_environment")
        row["median_seconds"] = statistics.median(row["samples_seconds"])
        row["min_seconds"] = min(row["samples_seconds"])
        row["max_seconds"] = max(row["samples_seconds"])
        if row["language"] == "mech":
            row["mech_source_only_supplement"] = metrics([row["path"]])
    report = {
        "schema_version": 1, "created_at": datetime.now(timezone.utc).isoformat(),
        "git_commit": command(["git", "rev-parse", "HEAD"]).stdout.strip(),
        "tracked_dirty": command(["git", "status", "--porcelain", "--untracked-files=no"]).stdout.splitlines(),
        "hardware": {"platform": platform.platform(), "machine": platform.machine(),
                     "cpu": command(["sysctl", "-n", "machdep.cpu.brand_string"]).stdout.strip() if platform.system() == "Darwin" else platform.processor()},
        "toolchain_versions": versions,
        "compiler_snapshot": snapshot,
        "protocol": {"steps": args.steps, "samples": args.samples, "seed": args.seed,
                     "timed_region": "fresh process launch through exit; includes runtime initialization/JIT/source compilation and energy output; excludes ahead-of-time builds",
                     "source_metric": "comments removed, whitespace runs collapsed, UTF-8, gzip level 1 with mtime=0; locally specified metric, not guaranteed byte-identical to CLBG gz",
                     "source_scope": "Mech .mec source includes all initialization, simulation, energy math, and explicit energy export; generic runtime/compiler/CLI excluded. Other languages use complete pinned submissions.",
                     "mech_size_scope": "mech-only",
                     "mech_mode": args.mech_mode,
                     "mech_native_flags": native_flags,
                     "c_tuning_flags": c_tuning_flags,
                     "c_configurations": "GCC and LLVM, native O3 and fast-math/unrolling/LTO for each C/C++ source" if args.compare_cc else args.c_tune,
                     "experimental_compiler_sha256": compiler_sources(),
                     "aot_backport_source": "iros-workshop-benchmarks; scalar AOT commit b10593b75ec8b9283ae33e4ac330c00dda9957d7; autonomous f64/SIMD extension in this worktree",
                     "process_environment": {k: environment[k] for k in ("JULIA_NUM_THREADS", "OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "CHPL_RT_NUM_THREADS_PER_LOCALE", "GOMAXPROCS")},
                     "sample_order": order, "expected_energy": expected,
                     "status": "pilot" if args.steps != 50_000_000 else "full input; local protocol"},
        "programs": rows, "excluded": excluded, "endpoints": endpoint_summary(rows),
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Saved {args.output}; {len(rows)} validated programs, {len(excluded)} exclusions.")
    for language, endpoint in report["endpoints"]["languages"].items():
        print(language, endpoint["fastest"], endpoint["smallest"], endpoint["smallest_time_over_fastest"])
    if excluded and not args.allow_partial:
        raise SystemExit("Results contain exclusions; inspect them before using the figure (or explicitly --allow-partial).")

if __name__ == "__main__":
    main()
