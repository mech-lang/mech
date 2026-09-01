#!/usr/bin/env python3
"""Measure compiler and first-run specialization costs for every EKF control.

The throughput runners intentionally discard startup.  This companion runner
does the opposite: it times the command that creates each artifact (or the
first process invocation for a JIT runtime), records the exact command, and
keeps unavailable controls explicit.  The numbers are not mixed with the
steady-state throughput measurements.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import shutil
import statistics
import subprocess
import tempfile
import time
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
MINIMAL = HERE / "minimal"


def tool(name: str, explicit: str | None = None) -> str | None:
    return explicit or shutil.which(name)


def timed(
    command: list[str],
    env: dict[str, str],
    *,
    cwd: Path = ROOT,
) -> tuple[float, str]:
    started = time.perf_counter_ns()
    completed = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    elapsed = (time.perf_counter_ns() - started) / 1_000_000.0
    if completed.returncode != 0:
        raise RuntimeError(
            f"command exited {completed.returncode}: {' '.join(command)}\n"
            f"{completed.stdout}"
        )
    return elapsed, completed.stdout


def measured_command(
    command: list[str],
    env: dict[str, str],
    samples: int,
) -> tuple[list[float], list[str]]:
    times: list[float] = []
    outputs: list[str] = []
    for _ in range(samples):
        elapsed, output = timed(command, env)
        times.append(elapsed)
        outputs.append(output)
    return times, outputs


def add_row(
    rows: dict[str, dict[str, object]],
    language: str,
    variant: str,
    phase: str,
    command: list[str],
    times: list[float],
    *,
    notes: str = "",
) -> None:
    rows.setdefault(language, {})[variant] = {
        "available": True,
        "phase": phase,
        "command": command,
        "milliseconds": times,
        "median_milliseconds": statistics.median(times),
        "notes": notes,
    }


def unavailable(
    rows: dict[str, dict[str, object]],
    language: str,
    variant: str,
    phase: str,
    command: list[str],
    error: str,
) -> None:
    rows.setdefault(language, {})[variant] = {
        "available": False,
        "phase": phase,
        "command": command,
        "error": error,
    }


def python_command(python: str, source: Path, cache: Path) -> list[str]:
    return [python, "-B", "-m", "py_compile", str(source)]


def record_python(
    rows: dict[str, dict[str, object]],
    language: str,
    variant: str,
    source: Path,
    python: str | None,
    cache: Path,
    samples: int,
) -> None:
    command = python_command(python, source, cache) if python else ["python", "-m", "py_compile", str(source)]
    if python is None:
        unavailable(rows, language, variant, "CPython bytecode compile", command, "python interpreter not found")
        return
    env = os.environ.copy()
    env["PYTHONPYCACHEPREFIX"] = str(cache)
    try:
        times, _ = measured_command(command, env, samples)
    except (OSError, RuntimeError) as error:
        unavailable(rows, language, variant, "CPython bytecode compile", command, str(error))
    else:
        add_row(rows, language, variant, "CPython bytecode compile", command, times)


def record_lua(
    rows: dict[str, dict[str, object]],
    language: str,
    variant: str,
    source: Path,
    compiler: str | None,
    output: Path,
    samples: int,
    phase: str,
) -> None:
    command = [compiler or ("luac" if language == "Lua" else "luajit"), "-o", str(output), str(source)]
    if compiler is None:
        unavailable(rows, language, variant, phase, command, f"{language} compiler not found")
        return
    try:
        times, _ = measured_command(command, os.environ.copy(), samples)
    except (OSError, RuntimeError) as error:
        unavailable(rows, language, variant, phase, command, str(error))
    else:
        add_row(rows, language, variant, phase, command, times)


def record_jit_first_run(
    rows: dict[str, dict[str, object]],
    language: str,
    variant: str,
    command: list[str],
    interpreter: str | None,
    env: dict[str, str],
    samples: int,
    phase: str,
) -> None:
    if interpreter is None:
        unavailable(rows, language, variant, phase, command, f"{language} runtime not found")
        return
    try:
        times, _ = measured_command(command, env, samples)
    except (OSError, RuntimeError) as error:
        unavailable(rows, language, variant, phase, command, str(error))
    else:
        add_row(
            rows,
            language,
            variant,
            phase,
            command,
            times,
            notes="Cold process includes runtime startup and first-call specialization; it is not steady-state throughput.",
        )


def record_mech(rows: dict[str, dict[str, object]], path: Path | None) -> None:
    if path is None or not path.exists():
        return
    data = json.loads(path.read_text(encoding="utf-8"))
    outputs = data.get("runs", {}).get("mech_backend_settings", {}).get("measured_stdout", [])
    patterns = {
        "source + artifact + scalarization": "source + artifact + scalarization: ",
        "resident GPU prepare": "resident GPU prepare: ",
        "Cranelift JIT prepare": "Cranelift JIT prepare: ",
    }
    for label, prefix in patterns.items():
        values: list[float] = []
        for output in outputs:
            for line in output.splitlines():
                if line.startswith(prefix) and line.endswith(" ms"):
                    values.append(float(line[len(prefix) : -3]))
        if not values:
            continue
        entry = {
            "available": True,
            "phase": label,
            "command": data.get("runs", {}).get("mech_backend_settings", {}).get("command", []),
            "milliseconds": values,
            "median_milliseconds": statistics.median(values),
            "notes": "Timing was emitted by the Mech benchmark binary and retained in its evidence JSON.",
        }
        # The source-diff matrix uses the resident source path for the
        # baseline and the backend preparation path for the advanced lane.
        if label == "source + artifact + scalarization":
            rows.setdefault("Mech", {})["baseline"] = entry
        elif label == "resident GPU prepare":
            rows.setdefault("Mech", {})["advanced"] = entry
        elif label == "Cranelift JIT prepare" and "advanced" not in rows.get("Mech", {}):
            rows.setdefault("Mech", {})["advanced"] = entry


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--python", default=tool("python3"))
    parser.add_argument("--julia", default=tool("julia"))
    parser.add_argument("--lua", default=tool("lua"))
    parser.add_argument("--luajit", default=tool("luajit"))
    parser.add_argument("--luac", default=tool("luac"))
    parser.add_argument("--rustc", default=tool("rustc"))
    parser.add_argument("--clangxx", default=tool("clang++"))
    parser.add_argument("--futhark", default=tool("futhark"))
    parser.add_argument("--taichi-python", help="Python interpreter with Taichi installed")
    parser.add_argument("--cross-evidence", type=Path)
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument(
        "--output",
        type=Path,
        default=HERE / "results/apple-m1-compile-times-2026-09-01.json",
    )
    args = parser.parse_args()
    if args.samples < 1:
        raise SystemExit("--samples must be positive")
    env = os.environ.copy()
    env.update({name: "1" for name in ("OMP_NUM_THREADS", "OPENBLAS_NUM_THREADS", "MKL_NUM_THREADS", "VECLIB_MAXIMUM_THREADS")})
    rows: dict[str, dict[str, object]] = {}
    cross = args.cross_evidence or (HERE / "results/apple-m1-checked-cross-language-2026-08-31.json")
    record_mech(rows, cross if cross.exists() else None)

    with tempfile.TemporaryDirectory(prefix="mech-ekf-compile-") as directory:
        temp = Path(directory)
        cache = temp / "pycache"
        cache.mkdir()
        record_python(rows, "NumPy", "baseline", MINIMAL / "numpy_scalar.py", args.python, cache, args.samples)
        record_python(rows, "NumPy", "advanced", MINIMAL / "numpy_fast.py", args.python, cache, args.samples)
        record_python(rows, "Python", "baseline", MINIMAL / "pure_python.py", args.python, cache, args.samples)
        record_python(rows, "Python", "advanced", MINIMAL / "pure_python.py", args.python, cache, args.samples)

        record_lua(rows, "Lua", "baseline", MINIMAL / "luajit_fast.lua", args.luac, temp / "lua-baseline.luac", args.samples, "PUC Lua bytecode compile")
        record_lua(rows, "Lua", "advanced", MINIMAL / "lua_advanced.lua", args.luac, temp / "lua-advanced.luac", args.samples, "PUC Lua bytecode compile")
        # LuaJIT's bytecode switch has a different shape from PUC Lua's
        # compiler, so keep it explicit rather than treating it as Lua.
        for variant, source, output in (
            ("baseline", MINIMAL / "luajit_scalar.lua", temp / "luajit-baseline.ljbc"),
            ("advanced", MINIMAL / "luajit_fast.lua", temp / "luajit-advanced.ljbc"),
        ):
            command = [args.luajit or "luajit", "-b", str(source), str(output)]
            if args.luajit is None:
                unavailable(rows, "LuaJIT", variant, "LuaJIT bytecode compile", command, "luajit not found")
                continue
            try:
                times, _ = measured_command(command, env, args.samples)
            except (OSError, RuntimeError) as error:
                unavailable(rows, "LuaJIT", variant, "LuaJIT bytecode compile", command, str(error))
            else:
                add_row(rows, "LuaJIT", variant, "LuaJIT bytecode compile", command, times)

        if args.rustc:
            scalar_command = [args.rustc, "--edition=2024", "-C", "opt-level=3", "-C", "target-cpu=native", "-C", "codegen-units=1", str(MINIMAL / "rust_scalar.rs"), "-o", str(temp / "rust-scalar")]
            optimized_scalar_command = [args.rustc, "--edition=2024", "-C", "opt-level=3", "-C", "target-cpu=native", "-C", "codegen-units=1", str(MINIMAL / "rust_scalar_optimized.rs"), "-o", str(temp / "rust-scalar-optimized")]
            for language, variant, command, phase in (
                ("Rust", "baseline", scalar_command, "rustc AOT compile"),
                ("Rust", "advanced", optimized_scalar_command, "rustc AOT compile"),
            ):
                try:
                    times, _ = measured_command(command, env, args.samples)
                except (OSError, RuntimeError) as error:
                    unavailable(rows, language, variant, phase, command, str(error))
                else:
                    add_row(rows, language, variant, phase, command, times)
        else:
            unavailable(rows, "Rust", "baseline", "rustc AOT compile", ["rustc"], "rustc not found")
            unavailable(rows, "Rust", "advanced", "rustc AOT compile", ["rustc"], "rustc not found")

        if args.clangxx:
            halide_include = os.environ.get("HALIDE_INCLUDE", "/opt/homebrew/opt/halide/include")
            halide_lib = os.environ.get("HALIDE_LIB", "/opt/homebrew/opt/halide/lib")
            halide_command = [args.clangxx, "-O3", "-std=c++17", str(MINIMAL / "halide_ekf.cpp"), f"-I{halide_include}", f"-L{halide_lib}", "-lHalide", "-o", str(temp / "halide-ekf")]
            for variant in ("baseline", "advanced"):
                try:
                    times, _ = measured_command(halide_command, env, args.samples)
                except (OSError, RuntimeError) as error:
                    unavailable(rows, "Halide", variant, "clang++ + Halide AOT control build", halide_command, str(error))
                else:
                    add_row(rows, "Halide", variant, "clang++ + Halide AOT control build", halide_command, times)
        else:
            for variant in ("baseline", "advanced"):
                unavailable(rows, "Halide", variant, "clang++ + Halide AOT control build", ["clang++"], "clang++ not found")

        if args.futhark:
            for backend, variant in (("multicore", "baseline"), ("ispc", "advanced")):
                output = temp / f"futhark-{backend}"
                command = [args.futhark, backend, str(MINIMAL / "futhark_ekf.fut"), "-o", str(output)]
                compile_env = env.copy()
                if backend == "ispc":
                    wrapper = temp / "ispc-bin"
                    wrapper.mkdir()
                    (wrapper / "ispc").symlink_to(MINIMAL / "futhark-ispc-compat.sh")
                    compile_env["PATH"] = f"{wrapper}{os.pathsep}{compile_env.get('PATH', '')}"
                try:
                    times, _ = measured_command(command, compile_env, args.samples)
                except (OSError, RuntimeError) as error:
                    unavailable(rows, "Futhark", variant, f"futhark {backend} AOT compile", command, str(error))
                else:
                    add_row(rows, "Futhark", variant, f"futhark {backend} AOT compile", command, times)
        else:
            for variant in ("baseline", "advanced"):
                unavailable(rows, "Futhark", variant, "futhark AOT compile", ["futhark"], "futhark not found")

        julia_scripts = (("baseline", MINIMAL / "julia_scalar.jl", "1"), ("advanced", MINIMAL / "julia_simd.jl", "4"))
        for variant, source, instances in julia_scripts:
            command = [args.julia or "julia", "--startup-file=no", str(source), instances, "1", "unchecked"]
            record_jit_first_run(rows, "Julia", variant, command, args.julia, env, args.samples, "cold Julia process + first-call specialization")

        taichi_scripts = (("baseline", MINIMAL / "taichi_comparable.py"), ("advanced", MINIMAL / "taichi_optimized.py"))
        taichi_python = args.taichi_python
        taichi_env = env.copy()
        taichi_env["TAICHI_ARCH"] = os.environ.get("TAICHI_ARCH", "metal")
        for variant, source in taichi_scripts:
            command = [taichi_python or "python", str(source), "1", "1", "unchecked"]
            record_jit_first_run(rows, "Taichi", variant, command, taichi_python, taichi_env, args.samples, "cold Taichi process + first kernel compilation")

    result = {
        "schema_version": 1,
        "generated_at": dt.datetime.now().astimezone().isoformat(),
        "machine": {
            "platform": __import__("platform").platform(),
            "processor": __import__("platform").processor(),
            "python": __import__("platform").python_version(),
        },
        "configuration": {"samples": args.samples},
        "definition": "AOT rows time source-to-artifact commands. Bytecode rows time compiler output. Julia, Taichi, and Mech rows time the retained first-run or preparation interval; cold-process values include startup and specialization and must not be compared to steady-state throughput.",
        "rows": rows,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    for language in sorted(rows):
        for variant in ("baseline", "advanced"):
            row = rows[language].get(variant)
            if row is None:
                print(f"{language} {variant}: unavailable (not measured)")
            elif row.get("available"):
                print(f"{language} {variant}: {float(row['median_milliseconds']):.3f} ms ({row['phase']})")
            else:
                print(f"{language} {variant}: unavailable ({row.get('error', 'unknown error')})")


if __name__ == "__main__":
    main()
