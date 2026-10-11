#!/usr/bin/env python3
"""Measure fixed-shape fusion, FMA and scheduling on two authored Mech n-body forms."""
import argparse
import hashlib
import json
import os
import random
import re
import statistics
import time
from datetime import datetime, timezone
from pathlib import Path
from run import HERE, command, compiler_snapshot, compiler_sources, energies, metrics, validate


def state(output):
    return json.loads(next(line[6:] for line in output.splitlines() if line.startswith("STATE ")))


def loop_codegen(binary, destination):
    assembly = command(["/opt/homebrew/opt/llvm@22/bin/llvm-objdump", "--disassemble", "--no-show-raw-insn", str(binary)]).stdout
    if "<_mech_turns>:" not in assembly:
        return None
    text = assembly.split("<_mech_turns>:", 1)[1].split("\n000000", 1)[0]
    instructions = []
    for line in text.splitlines():
        match = re.match(r"\s*([0-9a-f]+):\s+([a-z][a-z0-9_.]*)\s*(.*)", line)
        if match:
            instructions.append((int(match[1], 16), match[2], match[3], line))
    backedges = [(pc, int(argument.split()[0], 16)) for pc, op, argument, _ in instructions
                 if op.startswith("b") and re.match(r"0x[0-9a-f]+", argument)
                 and int(argument.split()[0], 16) < pc]
    if len(backedges) != 1:
        raise ValueError(f"expected one native simulation loop in {binary}: {backedges}")
    end, start = backedges[0]
    loop = [(pc, op, arg, line) for pc, op, arg, line in instructions if start <= pc <= end]
    destination.write_text("_mech_turns\n" + "\n".join(line for _, _, _, line in loop) + "\n")
    return {"disassembly_path": destination.name, "loop_start_hex": hex(start), "loop_backedge_hex": hex(end),
            "static_instructions": len(loop),
            "stack_load_instructions": sum(op in ("ldr", "ldur", "ldp") and "[sp" in arg for _, op, arg, _ in loop),
            "stack_store_instructions": sum(op in ("str", "stur", "stp") and "[sp" in arg for _, op, arg, _ in loop),
            "fused_instructions": sum(op.startswith(("fmadd", "fmsub", "fnmadd", "fnmsub", "fmla", "fmls")) for _, op, _, _ in loop),
            "sqrt_instructions": sum(op.startswith("fsqrt") for _, op, _, _ in loop),
            "divide_instructions": sum(op.startswith("fdiv") for _, op, _, _ in loop),
            "loop_calls": [(hex(pc), op, arg) for pc, op, arg, _ in loop if op in ("bl", "blr")]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--steps", type=int, default=50_000_000)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--clang", default="/opt/homebrew/opt/llvm@22/bin/clang-22")
    parser.add_argument("--output", type=Path, default=HERE / "results/apple-m3-chapel-tricks-50m.json")
    args = parser.parse_args()
    if args.steps < 0 or args.samples < 3:
        parser.error("nonnegative steps and at least three samples are required")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    build = HERE / ".build"
    build.mkdir(exist_ok=True)
    snapshot = compiler_snapshot(args.output)
    command(["cargo", "build", "--release", "--locked", "--manifest-path", str(HERE / "mech-driver/Cargo.toml")])
    driver = HERE / "mech-driver/target/release/mech-clbg-nbody-compiled"
    sdk = command(["xcrun", "--show-sdk-path"]).stdout.strip()
    environment = {**os.environ, "CHPL_RT_NUM_THREADS_PER_LOCALE": "1", "OMP_NUM_THREADS": "1", "MECH_TIMINGS": "1"}
    check_environment = {**environment, "MECH_DUMP_STATE": "1"}
    programs = []
    reference = build / "chapel-tricks-rust-reference"
    rust_command = ["rustc", "-C", "opt-level=3", "-C", "target-cpu=native", "-C", "codegen-units=1", str(HERE / "upstream/nbody-rust-3.rs"), "-o", str(reference)]
    command(rust_command)
    programs.append({"id": "rust-3", "language": "rust", "command": [str(reference)], "compile_command": rust_command,
                     "source": metrics(["upstream/nbody-rust-3.rs"])})
    chapel = build / "chapel-tricks-chapel-reference"
    chapel_command = ["chpl", "--fast", str(HERE / "upstream/nbody-chapel-3.chpl"), "-o", str(chapel), f"--ccflags=-isysroot {sdk}", f"--ldflags=-isysroot {sdk}"]
    command(chapel_command)
    programs.append({"id": "chapel-3", "language": "chapel", "command": [str(chapel)], "compile_command": chapel_command,
                     "source": metrics(["upstream/nbody-chapel-3.chpl"])})

    # Add a state diagnostic to a separate, untimed Rust executable only.
    original = (HERE / "upstream/nbody-rust-3.rs").read_text()
    hook = '    println!("{:.9}", compute_energy(&mut bodies));\n}'
    assert original.count(hook) == 1
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
    instrumented = build / "chapel-tricks-state-reference.rs"
    instrumented.write_text(original.replace(hook, hook[:-1] + dump + "}"))
    reference_state_binary = build / "chapel-tricks-state-reference"
    command(["rustc", "-C", "opt-level=3", "-C", "target-cpu=native", str(instrumented), "-o", str(reference_state_binary)])
    state_checks = []
    fma = ["--fp-contract", "--algebraic-powers"]
    cases = [("sqrt-fma", "nbody.mec", fma),
             ("sqrt-fma-depth", "nbody.mec", fma + ["--schedule"]),
             ("power-fma", "nbody-power.mec", fma),
             ("factored-fma", "nbody-factored.mec", fma),
             ("pairwise-strict", "nbody-pairwise.mec", []),
             ("pairwise-fma", "nbody-pairwise.mec", fma),
             ("pairwise-fma-depth", "nbody-pairwise.mec", fma + ["--schedule"])]
    for identifier, source_name, flags in cases:
        identifier = "mech-" + identifier
        source = HERE / source_name
        generated = build / (identifier + "-chapel-tricks.c")
        obj = build / (identifier + "-chapel-tricks.o")
        binary = build / (identifier + "-chapel-tricks")
        generation = [str(driver), "0", str(source), "emit-cranelift-simd", str(generated), str(obj), "--output", "energy"] + flags
        command(generation)
        compile_command = [args.clang, "-O3", "-isysroot", sdk, str(generated), str(obj), "-lm", "-o", str(binary)]
        command(compile_command)
        codegen = loop_codegen(binary, args.output.with_name(args.output.stem + "." + identifier + ".disassembly.txt"))
        for steps in (0, 1, 1000, 10_000):
            actual = state(command([str(binary), str(steps)], env=check_environment).stderr)
            resident = state(command([str(driver), str(steps), str(source), "resident", "--output", "energy"], env=check_environment).stderr)
            independent = state(command([str(reference_state_binary), str(steps)], env=check_environment).stderr)
            if not len(actual) == len(resident) == len(independent):
                raise ValueError(f"state shape mismatch: {identifier}")
            resident_error = max(abs(a-b) / max(1.0, abs(b)) for a, b in zip(actual, resident))
            rust_error = max(abs(a-b) / max(1.0, abs(b)) for a, b in zip(actual, independent))
            if max(resident_error, rust_error) > 1e-10:
                raise ValueError(f"full state mismatch: {identifier}: {resident_error}, {rust_error}")
            state_checks.append({"id": identifier, "steps": steps, "max_scaled_resident_error": resident_error, "max_scaled_rust_error": rust_error})
        programs.append({"id": identifier, "language": "mech", "execution_mode": "Cranelift SIMD AOT", "native_flags": flags,
                         "command": [str(binary)], "generation_command": generation, "compile_command": compile_command,
                         "source": metrics([source_name]), "native_object_sha256": hashlib.sha256(obj.read_bytes()).hexdigest(), "codegen": codegen})

    def invocation(row, steps):
        return row["command"] + [f"--n={steps}" if row["language"] == "chapel" else str(steps)]
    expected = energies(command(invocation(programs[0], args.steps), env=environment).stdout)
    if args.steps == 50_000_000:
        validate("\n".join(map(str, expected)), [-0.169075164, -0.169059907])
    for row in programs:
        row["binary_sha256"] = hashlib.sha256(Path(row["command"][0]).read_bytes()).hexdigest()
        validate(command(invocation(row, 1000), env=environment).stdout, [-0.169075164, -0.169087605])
        validate(command(invocation(row, args.steps), env=environment).stdout, expected)
        row["samples_seconds"], row["samples_stdout"], row["timing_diagnostics"] = [], [], []
    rng = random.Random(20261007)
    sample_order = []
    for sample in range(args.samples):
        order = programs.copy()
        rng.shuffle(order)
        sample_order.append([row["id"] for row in order])
        for row in order:
            print(f"Sample {sample+1}/{args.samples} {row['id']}", flush=True)
            start = time.perf_counter_ns()
            result = command(invocation(row, args.steps), env=environment)
            elapsed = (time.perf_counter_ns() - start) / 1e9
            validate(result.stdout, expected)
            row["samples_seconds"].append(elapsed)
            row["samples_stdout"].append(result.stdout)
            row["timing_diagnostics"].extend(json.loads(line[7:]) for line in result.stderr.splitlines() if line.startswith("TIMING "))
    for row in programs:
        row["median_seconds"] = statistics.median(row["samples_seconds"])
        if row["timing_diagnostics"]:
            row["median_kernel_seconds"] = statistics.median(t["kernel_seconds"] for t in row["timing_diagnostics"])
    report = {"created_at": datetime.now(timezone.utc).isoformat(), "steps": args.steps, "samples": args.samples,
              "hardware": {"cpu": command(["sysctl", "-n", "machdep.cpu.brand_string"]).stdout.strip()},
              "toolchain_versions": {"rust": command(["rustc", "--version", "--verbose"]).stdout,
                                     "chapel": command(["chpl", "--version"]).stdout, "clang": command([args.clang, "--version"]).stdout},
              "compiler_snapshot": snapshot, "compiler_sha256": compiler_sources(), "sample_order": sample_order,
              "protocol": "Fresh processes; startup/initialization/energy included, AOT builds excluded; one discarded warmup; shuffled sequential samples; Mech source only, generic compiler/runtime excluded.",
              "workload": "Same five bodies, ten unique pairs, dt=0.01, momentum offset and two energy outputs. Pairwise Mech changes velocity accumulation order and shares d*mag; it is an authored accuracy-validated alternative, not strict-bit-identical to the earlier source.",
              "state_reference_source_sha256": hashlib.sha256(instrumented.read_bytes()).hexdigest(),
              "expected_energy": expected, "full_state_checks": state_checks, "programs": programs}
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    for row in programs:
        print(row["id"], row["median_seconds"], row.get("median_kernel_seconds"), row.get("codegen"))


if __name__ == "__main__":
    main()
