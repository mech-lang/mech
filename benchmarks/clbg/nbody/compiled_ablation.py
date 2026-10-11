#!/usr/bin/env python3
"""Compare Cranelift AOT/JIT and SIMD; optionally include an LLVM comparison."""
import argparse
import hashlib
import json
import os
import platform
import random
import statistics
import time
from datetime import datetime, timezone
from pathlib import Path
from run import HERE, command, compiler_sources, compiler_snapshot, macos_gcc_flags, energies, validate

def state(output):
    line = next(line for line in output.splitlines() if line.startswith("STATE "))
    return json.loads(line[6:])

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--steps", type=int, default=50_000_000)
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--clang", default="/opt/homebrew/opt/llvm@22/bin/clang-22")
    parser.add_argument("--include-llvm", action="store_true")
    parser.add_argument("--include-c", action="store_true", help="compare pinned C/C++ sources with GCC and Clang, including fast-math/unrolling/LTO")
    parser.add_argument("--output", type=Path, default=HERE / "results/apple-m3-native-output-ablation-50m.json")
    args = parser.parse_args()
    if args.steps < 0 or args.samples < 3:
        parser.error("steps must be nonnegative; at least three samples")
    gcc_flags = macos_gcc_flags("gcc-16") if args.include_c else []
    snapshot = compiler_snapshot(args.output)
    build = HERE / ".build"
    build.mkdir(exist_ok=True)
    command(["cargo", "build", "--release", "--locked", "--manifest-path", str(HERE / "mech-driver/Cargo.toml")])
    driver = HERE / "mech-driver/target/release/mech-clbg-nbody-compiled"
    common = [args.clang, "-O3", "-mcpu=native", "-ffp-contract=off", "-fno-math-errno"]
    if platform.system() == "Darwin":
        common += ["-isysroot", command(["xcrun", "--show-sdk-path"]).stdout.strip()]
    variants = []
    for name, disabled in (("rust-3", False), ("rust-3-no-simd", True)):
        path = build / name
        compile_command = ["rustc", "-C", "opt-level=3", "-C", "target-cpu=native", "-C", "codegen-units=1"]
        if disabled:
            compile_command += ["-C", "no-vectorize-loops", "-C", "no-vectorize-slp"]
        compile_command += [str(HERE / "upstream/nbody-rust-3.rs"), "-o", str(path)]
        command(compile_command)
        variants.append({"id": name, "command": [str(path)], "compile_command": compile_command})
    state_checks = []
    check_environment = {**os.environ, "MECH_DUMP_STATE": "1"}
    # Instrument a separate copy for correctness only. Timed Rust binaries
    # remain the unmodified pinned submission.
    reference_source = (HERE / "upstream/nbody-rust-3.rs").read_text()
    hook = '    println!("{:.9}", compute_energy(&mut bodies));\n}'
    assert reference_source.count(hook) == 1
    dump = '''
    if std::env::var_os("MECH_DUMP_STATE").is_some() {
        let mut state = Vec::new();
        for velocity in [false, true] {
            for axis in 0..3 {
                for body in &bodies {
                    let xyz = if velocity { &body.velocity } else { &body.position };
                    state.push(match axis { 0 => xyz.0, 1 => xyz.1, _ => xyz.2 });
                }
            }
        }
        eprintln!("STATE {state:?}");
    }
'''
    instrumented = build / "rust-state-reference.rs"
    instrumented.write_text(reference_source.replace(hook, hook[:-1] + dump + "}"))
    reference_binary = build / "rust-state-reference"
    command(["rustc", "-C", "opt-level=3", "-C", "target-cpu=native", str(instrumented), "-o", str(reference_binary)])
    for name in ("sqrt", "power", "factored"):
        source = HERE / ("nbody.mec" if name == "sqrt" else f"nbody-{name}.mec")
        targets = [("cranelift-aot", "strict", [], []), ("cranelift-aot-simd", "strict", [], [])]
        if name in ("sqrt", "factored"):
            targets += [("cranelift-aot-simd", "contract", ["--fp-contract"], []),
                        ("cranelift-aot-simd", "schedule", ["--schedule"], []),
                        ("cranelift-aot-simd", "contract-schedule", ["--fp-contract", "--schedule"], [])]
        targets += [("cranelift-aot-simd", "refined", ["--refined-reciprocal"], [])]
        if name == "power":
            targets += [("cranelift-aot-simd", "algebraic", ["--algebraic-powers"], []),
                        ("cranelift-aot-simd", "algebraic-contract", ["--algebraic-powers", "--fp-contract"], [])]
        if name == "sqrt":
            targets += [("cranelift-aot-simd", "contract-refined", ["--fp-contract", "--refined-reciprocal"], [])]
        if args.include_llvm:
            targets.append(("llvm-aot", "strict", [], []))
            if name == "sqrt":
                targets += [("llvm-aot", "no-simd", [], ["-fno-vectorize", "-fno-slp-vectorize"]),
                            ("llvm-aot", "contract", [], ["-ffp-contract=fast"]),
                            ("llvm-aot", "fastmath", [], ["-ffast-math"])]
        for target, optimization, native_flags, cc_flags in targets:
            identifier = f"mech-{target}-{name}" + (f"-{optimization}" if optimization != "strict" else "")
            generated = build / f"{identifier}.c"
            native_object = build / f"{identifier}.o"
            cranelift = target.startswith("cranelift")
            generation = [str(driver), "0", str(source),
                          "emit-cranelift-simd" if target.endswith("-simd") else
                          "emit-cranelift" if cranelift else "emit-c", str(generated)]
            if cranelift:
                generation.append(str(native_object))
            generation += ["--output", "energy"] + native_flags
            command(generation)
            path = build / identifier
            flags = common + cc_flags
            compile_command = flags + [str(generated)] + ([str(native_object)] if cranelift else []) + ["-lm", "-o", str(path)]
            command(compile_command)
            # Verify the generated AOT loop's entire state against resident
            # execution, not merely its two rounded energy values.
            for steps in (0, 1, 1000, 10_000):
                resident = command([str(driver), str(steps), str(source), "resident", "--output", "energy"], env=check_environment)
                compiled = command([str(path), str(steps)], env=check_environment)
                expected, actual = state(resident.stderr), state(compiled.stderr)
                error = max(abs(a-b) / max(1.0, abs(b)) for a, b in zip(actual, expected))
                if len(actual) != len(expected) or error > 1e-10:
                    raise ValueError(f"{identifier}: full state mismatch at {steps}: {error}")
                state_checks.append({"id": identifier, "steps": steps, "max_scaled_state_error": error})
                rust = command([str(reference_binary), str(steps)], env=check_environment)
                reference_state = state(rust.stderr)
                reference_error = max(abs(a-b) / max(1.0, abs(b)) for a, b in zip(actual, reference_state))
                if len(actual) != len(reference_state) or reference_error > 1e-10:
                    raise ValueError(f"{identifier}: Rust full state mismatch at {steps}: {reference_error}")
                state_checks[-1]["max_scaled_rust_state_error"] = reference_error
            variants.append({"id": identifier, "command": [str(path)], "compile_command": compile_command,
                             "execution_mode": target, "optimization": optimization, "native_flags": native_flags, "generation_command": generation,
                             "generated_c_sha256": hashlib.sha256(generated.read_bytes()).hexdigest(),
                             "native_object_sha256": hashlib.sha256(native_object.read_bytes()).hexdigest() if cranelift else None})
        for mode in ("jit", "jit-simd"):
            variants.append({"id": f"mech-cranelift-{mode}-{name}", "execution_mode": f"cranelift-{mode}",
                             "command": [str(driver)], "source_arguments": [str(source), mode, "--output", "energy"]})
    if args.include_c:
        for language, source_name in (("c", "nbody-gcc-6.c"), ("cpp", "nbody-gpp-9.cpp")):
            for family in ("gcc", "llvm"):
                compiler = ("gcc-16" if language == "c" else "g++-16") if family == "gcc" else args.clang
                for optimized in (False, True):
                    identifier = f"{language}-{family}" + ("-fastmath-unroll-lto" if optimized else "-native")
                    path = build / identifier
                    flags = ["-O3", "-march=native"] if family == "gcc" else common[1:]
                    if family == "gcc": flags += gcc_flags
                    if language == "cpp": flags += ["-std=c++20"] + (["-lc++"] if family == "llvm" else [])
                    if optimized: flags += ["-ffast-math", "-funroll-loops", "-flto"]
                    compile_command = [compiler] + flags + [str(HERE / "upstream" / source_name), "-lm", "-o", str(path)]
                    command(compile_command)
                    variants.append({"id": identifier, "execution_mode": family,
                                     "source": source_name, "compile_command": compile_command,
                                     "command": [str(path)], "optimization": "fastmath/unrolling/LTO" if optimized else "native/O3"})
    def invocation(row, steps):
        return row["command"] + [str(steps)] + row.get("source_arguments", [])
    expected = energies(command(invocation(variants[0], args.steps)).stdout)
    if args.steps == 50_000_000:
        validate("\n".join(map(str, expected)), [-0.169075164, -0.169059907])
    for row in variants:
        validate(command(invocation(row, 1000)).stdout, [-0.169075164, -0.169087605])
        validate(command(invocation(row, args.steps)).stdout, expected)
        row["samples_seconds"] = []
        row["timing_diagnostics"] = []
    rng = random.Random(20261006)
    order = []
    timed_environment = {**os.environ, "MECH_TIMINGS": "1"}
    for sample in range(args.samples):
        shuffled = variants.copy()
        rng.shuffle(shuffled)
        order.append([row["id"] for row in shuffled])
        for row in shuffled:
            print(f"Sample {sample+1}/{args.samples}: {row['id']}", flush=True)
            start = time.perf_counter_ns()
            result = command(invocation(row, args.steps), env=timed_environment)
            elapsed = (time.perf_counter_ns()-start) / 1e9
            validate(result.stdout, expected)
            row["samples_seconds"].append(elapsed)
            row["timing_diagnostics"].extend(json.loads(line[7:]) for line in result.stderr.splitlines() if line.startswith("TIMING "))
    for row in variants:
        row["median_seconds"] = statistics.median(row["samples_seconds"])
        if row["timing_diagnostics"]:
            row["median_kernel_seconds"] = statistics.median(x["kernel_seconds"] for x in row["timing_diagnostics"])
            row["median_prepare_seconds"] = statistics.median(x["prepare_seconds"] for x in row["timing_diagnostics"])
    report = {"created_at": datetime.now(timezone.utc).isoformat(), "steps": args.steps,
              "samples": args.samples, "cpu": command(["sysctl", "-n", "machdep.cpu.brand_string"]).stdout.strip(),
              "git_commit": command(["git", "rev-parse", "HEAD"]).stdout.strip(),
              "clang": command([args.clang, "--version"]).stdout.strip(), "rustc": command(["rustc", "--version"]).stdout.strip(),
              "compiler_sha256": compiler_sources(),
              "compiler_snapshot": snapshot,
              "simd_strategy": "Pair coordinates for multi-column tensors and elements for vectors; preserve reduction and repeated-update order; carry packed state and use explicit lane shuffles.",
              "source_sha256": {name: hashlib.sha256((HERE / name).read_bytes()).hexdigest()
                                for name in ("nbody.mec", "nbody-power.mec", "nbody-factored.mec", "mech-driver/src/compiled-main.rs", "upstream/nbody-rust-3.rs")},
              "aot_backport_source": "iros-workshop-benchmarks; scalar AOT commit b10593b75ec8b9283ae33e4ac330c00dda9957d7; autonomous f64/SIMD extension in this worktree",
              "protocol": "Fresh processes; AOT compilation excluded; JIT/source/activation included. Kernel timing is separate. Strict SIMD preserves arithmetic order; FMA, algebraic powers, and reciprocal refinement are explicit opt-ins; each row records its flags. LLVM fast-math is a separately labelled comparison.",
              "expected_energy": expected, "sample_order": order, "full_state_checks": state_checks, "programs": variants}
    report["workload"] = {"bodies": 5, "unique_pairs_per_turn": 10, "coordinates_per_body": 3,
                          "dt": 0.01, "momentum_offset": True, "energy_evaluations": 2,
                          "state_reference": "unmodified Rust #3 plus a generated stderr-only diagnostic in a separate, untimed executable",
                          "reference_instrumentation_sha256": hashlib.sha256(instrumented.read_bytes()).hexdigest()}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    for row in variants:
        print(row["id"], row["median_seconds"], row.get("median_kernel_seconds"))

if __name__ == "__main__":
    main()
