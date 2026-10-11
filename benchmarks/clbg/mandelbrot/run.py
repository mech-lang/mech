#!/usr/bin/env python3
"""Pin, validate and time complete PBM output from local Mandelbrot programs."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import importlib.util
import io
import json
import os
import platform
import random
import re
import shutil
import statistics
import subprocess
import tarfile
import tempfile
import time
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
spec = importlib.util.spec_from_file_location("nbody_common", HERE.parent / "nbody/run.py")
common = importlib.util.module_from_spec(spec)
spec.loader.exec_module(common)
BUILD = HERE / ".build"
REFERENCE_URL = "https://benchmarksgame-team.pages.debian.net/benchmarksgame/download/mandelbrot-output.txt"
REFERENCE_SHA = "97610473750700638fc63d13cfa49d339b67c18e7f26b3f9c9acb61e746472d5"


def command(args, **kwargs):
    return common.command([str(x) for x in args], **kwargs)


def metrics(path):
    source = (HERE / path).read_text()
    language = "mech" if path.endswith(".mec") else "julia" if path.endswith(".jl") else "c"
    clean = common.strip_comments(source.lstrip("\ufeff"), language)
    normalized = re.sub(r"\s+", " ", clean).strip().encode()
    return {"paths": [path], "sha256": hashlib.sha256(source.encode()).hexdigest(),
            "physical_lines": len(source.splitlines()),
            "nonblank_noncomment_lines": sum(bool(line.strip()) for line in clean.splitlines()),
            "normalized_bytes": len(normalized),
            "gzip1_bytes": len(gzip.compress(normalized, compresslevel=1, mtime=0))}


def image_run(argv, env, n, timeout):
    # Identical output sink for all programs. Hashing/validation occurs after
    # the clock stops; writing and flushing the full PBM remain timed.
    with tempfile.TemporaryFile() as output:
        start = time.perf_counter_ns()
        result = subprocess.run(argv, cwd=ROOT, env=env, stdout=output,
                                stderr=subprocess.PIPE, timeout=timeout, check=True)
        elapsed = (time.perf_counter_ns() - start) / 1e9
        output.seek(0)
        data = output.read()
    header = f"P4\n{n} {n}\n".encode()
    if not data.startswith(header) or len(data) != len(header) + n * ((n + 7) // 8):
        raise ValueError(f"invalid {n}x{n} PBM: {len(data)} bytes; prefix={data[:64]!r}")
    return elapsed, data, result.stderr.decode(errors="replace")


def image_info(data, reference=None):
    info = {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
    if reference is not None:
        info["exact_reference_match"] = data == reference
        if data != reference:
            info["different_pixels"] = sum((a ^ b).bit_count() for a, b in zip(data, reference))
        else:
            info["different_pixels"] = 0
    return info


def worker_policy(program, threads):
    lang, variant = program["language"], program.get("variant")
    if lang == "mech": return f"{threads} pthread workers (including main)", True
    if lang == "chapel": return f"Chapel runtime tasks: {threads}", True
    if lang == "julia":
        return (f"Julia threads: {threads}" if variant != 1 else "serial"), True
    if lang == "rust":
        if variant == 1: return "fixed 20 std::thread workers", False
        if variant == 3: return "futures CpuPool::new_num_cpus(), 8 logical CPUs on this host", False
        return f"Rayon threads: {threads}", True
    if lang == "go":
        policies = {1: "fixed GOMAXPROCS=4, 4 workers", 2: "fixed GOMAXPROCS=8, 8 workers",
                    3: "NumCPU()*2: GOMAXPROCS=16, 16 workers", 4: "NumCPU()*2: GOMAXPROCS=16, 16 workers",
                    6: "fixed GOMAXPROCS=4, 4 workers", 8: "serial"}
        return policies[variant], variant == 8
    if lang == "java":
        if variant in (1, 8): return "serial", True
        if variant == 4: return f"Java parallel stream, ActiveProcessorCount={threads}", True
        workers = threads * (2 if variant in (2, 6) else 1)
        return f"ActiveProcessorCount={threads}, {workers} Java workers", workers == threads
    if lang == "c":
        if variant in (1, 3): return f"fixed {8 if variant == 1 else 3} pthread workers", False
        if variant == 8: return "serial", True
        return f"OpenMP threads: {threads}", True
    if variant == 0: return "std::thread::hardware_concurrency(), 8 workers on this host", False
    if variant in (2, 3, 5): return "serial", True
    return f"OpenMP threads: {threads}", True


def snapshot(output):
    names = list(common.compiler_sources()) + ["Cargo.lock", "hosts/gpu/src/lib.rs", "hosts/gpu/src/batched/mod.rs",
            "hosts/gpu/src/bin/mech-native-map.rs", "src/engine/src/resident/numeric/mod.rs",
            "scripts/unsafe-boundaries.allowlist", "benchmarks/clbg/nbody/run.py"]
    names += ["Cargo.toml", "hosts/gpu/src/native_build.rs", "hosts/gpu/src/sequence/application.rs.in", "src/cli/commands/build.rs", "src/engine/Cargo.toml", "src/engine/src/lib.rs", "src/stdlib/Cargo.toml", "src/stdlib/src/lib.rs", "src/stdlib/src/native_sequence.rs"]
    names += [str(p.relative_to(ROOT)) for p in (ROOT / "src/engine/src/native_sequence").glob("*.rs")]
    names += [str(p.relative_to(ROOT)) for p in HERE.rglob("*") if p.is_file()
              and not any(part in (".build", "results", "__pycache__", "target", "upstream") for part in p.relative_to(HERE).parts)]
    names += [str((HERE / "upstream/mandelbrot-200.pbm").relative_to(ROOT))]
    compat = common.HERE / ".build/gcc-sdk-compat.h"
    if compat.exists(): names.append(str(compat.relative_to(ROOT)))
    buffer = io.BytesIO()
    hashes = {}
    with tarfile.open(fileobj=buffer, mode="w") as tar:
        for name in sorted(set(names)):
            data = (ROOT / name).read_bytes()
            hashes[name] = hashlib.sha256(data).hexdigest()
            info = tarfile.TarInfo(name)
            info.size, info.mode = len(data), 0o644
            tar.addfile(info, io.BytesIO(data))
    archive = output.with_suffix(".compiler.tar.gz")
    archive.write_bytes(gzip.compress(buffer.getvalue(), compresslevel=6, mtime=0))
    return {"path": archive.name, "sha256": hashlib.sha256(archive.read_bytes()).hexdigest(),
            "base_commit": command(["git", "rev-parse", "HEAD"]).stdout.strip(), "source_sha256": hashes}


def prepare(program, args, environment):
    p = program.copy()
    identifier, lang = p["id"], p["language"]
    source, binary = HERE / p["path"], BUILD / identifier
    text = source.read_text()
    if "sha256" in p and hashlib.sha256(source.read_bytes()).hexdigest() != p["sha256"]:
        raise ValueError("pinned upstream source SHA256 changed")
    if platform.machine() != "x86_64" and re.search(r"__builtin_ia32|x86intrin|immintrin|emmintrin|std::arch::x86", text):
        raise ValueError("unmodified source requires x86 SIMD intrinsics; host is " + platform.machine())
    if lang == "c" and p["variant"] == 7 and platform.system() != "Linux":
        raise ValueError("unmodified source requires Linux sched_getaffinity/cpu_set_t")
    env = environment.copy()
    compile_command, generation = None, None
    if lang == "rust":
        deps = BUILD / "rust-deps/release/deps"
        compile_command = ["rustc", "--edition=2021", "-Copt-level=3", "-Ctarget-cpu=native", "-Ccodegen-units=1", "-Ldependency=" + str(deps)]
        for dep in re.findall(r"extern crate (\w+);", text):
            compile_command += ["--extern", dep + "=" + str(next(deps.glob(f"lib{dep}-*.rlib")))]
        compile_command += [str(source), "-o", str(binary)]
        prefix = [str(binary)]
    elif lang in ("c", "cpp"):
        cc = args.clang if p["cc_backend"] == "llvm" else args.gpp if lang == "cpp" else args.gcc
        llvm = p["cc_backend"] == "llvm"
        cpu = "-mcpu=native" if llvm and platform.machine() == "arm64" else "-march=native"
        compile_command = [cc, "-O3", cpu, "-fomit-frame-pointer", "-std=c++17" if lang == "cpp" else "-std=gnu11", str(source), "-o", str(binary), "-lm", "-pthread"]
        compile_command += common.macos_gcc_flags(cc)
        if llvm and lang == "cpp": compile_command += ["-include", "cstdlib", "-lc++"]
        if "omp" in text:
            compile_command += ["-fopenmp"]
            if llvm and platform.system() == "Darwin":
                compile_command += ["-I/opt/homebrew/opt/libomp/include", "-L/opt/homebrew/opt/libomp/lib", "-Wl,-rpath,/opt/homebrew/opt/libomp/lib"]
        if p["cc_tune"].startswith("fastmath"):
            compile_command += ["-ffast-math", "-funroll-loops", "-flto"]
        if p["cc_tune"] != "fastmath": compile_command += ["-ffp-contract=off"]
        prefix = [str(binary)]
    elif lang == "chapel":
        compile_command = ["chpl", "--fast", str(source), "-o", str(binary)]
        if platform.system() == "Darwin":
            sdk = command(["xcrun", "--show-sdk-path"]).stdout.strip()
            compile_command += [f"--ccflags=-isysroot {sdk}", f"--ldflags=-isysroot {sdk}"]
        prefix = [str(binary)]
    elif lang == "go":
        compile_command = ["go", "build"]
        if p.get("go_no_fma"): compile_command += ["-gcflags=-d=fmahash=n"]
        compile_command += ["-o", str(binary), str(source)]
        prefix = [str(binary)]
    elif lang == "julia":
        prefix = [args.julia, "--startup-file=no", "--history-file=no", "-O3", "--math-mode=ieee", "--", str(source)]
    elif lang == "java":
        binary.mkdir(exist_ok=True)
        java_source = binary / "mandelbrot.java"
        shutil.copyfile(source, java_source)
        compile_command = [str(Path(args.java_home) / "bin/javac"), "-d", str(binary), str(java_source)]
        prefix = [str(Path(args.java_home) / "bin/java"), f"-XX:ActiveProcessorCount={args.threads}", "-cp", str(binary), "mandelbrot"]
    else:
        generated, obj = BUILD / f"{identifier}.c", BUILD / f"{identifier}.o"
        driver = ROOT / "benchmarks/clbg/nbody/mech-driver/target/release/mech-clbg-mandelbrot-compiled"
        generation = [str(driver), str(source), "emit-cranelift-simd" if p["simd"] else "emit-cranelift", str(generated), str(obj)]
        generation += ["--source-program"] if p.get("source_program") else ["done", "inside", "width", "pixel,row"]
        if p.get("fma"): generation += ["--fp-contract"]
        if p.get("loop_fma"): generation += ["--fp-contract-loop"]
        if p.get("packed_bits"): generation += ["--packed-bits"]
        if p.get("source_program"):
            project = Path(str(binary) + ".project")
            if project.exists(): shutil.rmtree(project)
            generation = [args.mech, "build", str(source), "--backend", "cranelift", "--no-config", "--keep-project", "-o", str(binary)]
            if not p["simd"]: generation += ["--scalar"]
            if p.get("loop_fma"): generation += ["--fp-contract-loop"]
            command(generation, env=env | {"CC": args.clang})
            shutil.copyfile(project / "grid.c", generated)
            shutil.copyfile(project / "grid.o", obj)
            compile_command = None
            p["public_build"] = True
            p["mech_compiler_executable_sha256"] = hashlib.sha256(Path(args.mech).read_bytes()).hexdigest()
        else:
            command(generation)
            compile_command = [args.clang, "-O3", "-mcpu=native", "-ffp-contract=off", str(generated), str(obj), "-lm", "-pthread", "-o", str(binary)]
            if platform.system() == "Darwin": compile_command += ["-isysroot", command(["xcrun", "--show-sdk-path"]).stdout.strip()]
        p["generated_c_sha256"] = hashlib.sha256(generated.read_bytes()).hexdigest()
        p["native_object_sha256"] = hashlib.sha256(obj.read_bytes()).hexdigest()
        p["execution_mode"] = "experimental v0.4 Cranelift AOT native map; Clang links generic PBM/thread runtime"
        p["source_scope"] = "complete source program" if p.get("source_program") else "numerical kernel only; host traversal excluded"
        if p.get("source_program"):
            p["execution_mode"] = "experimental v0.4 Mech-defined kernel through canonical compiler; Cranelift AOT recurrence and column traversal; generic grid scheduler and PBM library runtime"
        prefix = [str(binary)]
    if compile_command: command(compile_command, env=env)
    def invocation(n):
        return prefix + [f"--n={n}" if lang == "chapel" else str(n)] + ([str(args.threads)] if lang == "mech" else [])
    policy, controllable = worker_policy(p, args.threads)
    p.update(source=metrics(p["path"]), compile_command=compile_command, generation_command=generation,
             command=invocation(args.size), correctness_command=invocation(200), worker_policy=policy,
             thread_controlled=controllable, samples_seconds=[], samples_images=[], _env=env)
    if lang == "java": p["execution_mode"] = "OpenJDK JIT; upstream GraalVM AOT submission source"
    _, image, _ = image_run(p["correctness_command"], env, 200, args.timeout)
    if hashlib.sha256(image).hexdigest() != REFERENCE_SHA:
        raise ValueError("N=200 output differs from the complete official bitmap")
    p["correctness_200"] = image_info(image)
    return p


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--size", type=int, default=16000)
    parser.add_argument("--threads", type=int, default=8)
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--seed", type=int, default=20261007)
    parser.add_argument("--timeout", type=int, default=600)
    parser.add_argument("--julia", default="/tmp/mech-clbg-julia/julia-1.13.1/bin/julia")
    parser.add_argument("--java-home", default="/opt/homebrew/opt/openjdk")
    parser.add_argument("--clang", default="/opt/homebrew/opt/llvm@22/bin/clang-22")
    parser.add_argument("--gcc", default=shutil.which("gcc-16"))
    parser.add_argument("--gpp", default=shutil.which("g++-16"))
    parser.add_argument("--mech", default=str(ROOT / "benchmarks/clbg/nbody/mech-driver/target/release/mech"))
    parser.add_argument("--only", help="comma-separated IDs; rust-4 required as full-image reference")
    parser.add_argument("--controlled-only", action="store_true")
    parser.add_argument("--output", type=Path, default=HERE / "results/apple-m3-16000.json")
    args = parser.parse_args()
    if args.size % 8 or args.size <= 0 or args.samples < 3 or args.threads <= 0:
        parser.error("size must be a positive multiple of 8; samples >=3; threads >0")
    BUILD.mkdir(exist_ok=True)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    if hashlib.sha256((HERE / "upstream/mandelbrot-200.pbm").read_bytes()).hexdigest() != REFERENCE_SHA:
        raise SystemExit("official reference SHA256 changed")
    environment = os.environ.copy()
    overrides = {"JULIA_NUM_THREADS": str(args.threads), "OMP_NUM_THREADS": str(args.threads),
                 "OMP_DYNAMIC": "FALSE", "CHPL_RT_NUM_THREADS_PER_LOCALE": str(args.threads),
                 "GOMAXPROCS": str(args.threads), "RAYON_NUM_THREADS": str(args.threads), "OPENBLAS_NUM_THREADS": "1"}
    environment.update(overrides)
    programs = json.loads((HERE / "manifest.json").read_text())["programs"]
    programs = [candidate for p in programs for candidate in
                ([{**p, "upstream_id": p["id"], "id": f"{p['id']}-{cc}-{tune}", "cc_backend": cc, "cc_tune": tune}
                  for cc in ("gcc", "llvm") for tune in (("native", "fastmath", "fastmath-nofma") if cc == "llvm" else ("native", "fastmath"))]
                 if p["language"] in ("c", "cpp") else
                 [p, {**p, "id": p["id"] + "-nofma", "upstream_id": p["id"], "go_no_fma": True}] if p["language"] == "go" else [p])]
    programs += [{"id": name, "language": "mech", "path": path, "simd": simd, "fma": fma, "upstream_starred": False}
                 for name, path, simd, fma in (
                     ("mech-compact", "mandelbrot-compact.mec", False, False),
                     ("mech-eight-scalar", "mandelbrot.mec", False, False),
                     ("mech-eight-simd", "mandelbrot.mec", True, False),
                     ("mech-eight-simd-fma", "mandelbrot.mec", True, True),
                     ("mech-unrolled-simd", "mandelbrot-unrolled.mec", True, False))]
    programs += [{"id": "mech-unrolled-simd-loop-fma", "language": "mech", "path": "mandelbrot-unrolled.mec",
                  "simd": True, "loop_fma": True, "upstream_starred": False}]
    programs += [{"id": name, "language": "mech", "path": path, "simd": True,
                  "loop_fma": True, "packed_bits": True, "upstream_starred": False}
                 for name, path in (
                     ("mech-unrolled-simd-loop-fma-packed-bits", "mandelbrot-unrolled.mec"),
                     ("mech-unrolled-simd-loop-fma-add-packed-bits", "mandelbrot-unrolled-add.mec"),
                     ("mech-unrolled-simd-loop-fma-ten-packed-bits", "mandelbrot-unrolled-10.mec"))]
    programs += [{"id": name, "language": "mech", "path": path, "simd": simd,
                  "loop_fma": loop_fma, "source_program": True, "packed_bits": True, "upstream_starred": False}
                 for name, path, simd, loop_fma in (
                     ("mech-program-compact", "mandelbrot-compact-program.mec", False, False),
                     ("mech-program-simd", "mandelbrot-program.mec", True, False),
                     ("mech-program-simd-loop-fma", "mandelbrot-program.mec", True, True))]
    if args.only: programs = [p for p in programs if p["id"] in set(args.only.split(","))]
    if args.controlled_only: programs = [p for p in programs if worker_policy(p, args.threads)[1]]
    if not any(p["id"] == "mandelbrot-rust-4" for p in programs): parser.error("include mandelbrot-rust-4 for full-image reference")
    if any(p["language"] == "rust" for p in programs):
        env = environment | {"CARGO_TARGET_DIR": str(BUILD / "rust-deps")}
        command(["cargo", "build", "--release", "--locked", "--manifest-path", HERE / "rust-deps/Cargo.toml"], env=env)
    if any(p["language"] == "mech" and not p.get("source_program") for p in programs):
        env = environment | {"CARGO_TARGET_DIR": str(ROOT / "benchmarks/clbg/nbody/mech-driver/target")}
        command(["cargo", "build", "--release", "--locked", "--manifest-path", HERE / "mech-driver/Cargo.toml"], env=env)
    versions = {}
    for name, argv in {"rust": ["rustc", "-vV"], "chapel": ["chpl", "--version"], "julia": [args.julia, "--version"],
                      "go": ["go", "version"], "java": [str(Path(args.java_home) / "bin/java"), "-version"],
                      "gcc": [args.gcc, "--version"], "gpp": [args.gpp, "--version"], "llvm": [args.clang, "--version"]}.items():
        r = command(argv)
        versions[name] = (r.stdout + r.stderr).strip()
    rows, excluded = [], []
    for p in programs:
        print("Prepare " + p["id"], flush=True)
        try: rows.append(prepare(p, args, environment))
        except (ValueError, OSError, StopIteration, subprocess.SubprocessError) as error:
            detail = error.stderr or error.stdout if isinstance(error, subprocess.CalledProcessError) else str(error)
            if isinstance(detail, bytes): detail = detail.decode(errors="replace")
            excluded.append({"id": p["id"], "path": p["path"], "reason": str(error), "diagnostic": detail})
            print(f"Exclude {p['id']}: {str(detail)[-500:]}", flush=True)
    reference_row = next((r for r in rows if r["id"] == "mandelbrot-rust-4"), None)
    if not reference_row: raise SystemExit("Rust #4 reference did not validate")
    _, reference, _ = image_run(reference_row["command"], reference_row["_env"], args.size, args.timeout)
    for row in rows:
        print("Warmup " + row["id"], flush=True)
        _, data, stderr = image_run(row["command"], row["_env"], args.size, args.timeout)
        row["warmup_image"] = image_info(data, reference)
        row["warmup_stderr"] = stderr
    rng, order = random.Random(args.seed), []
    for sample in range(args.samples):
        shuffled = rows.copy()
        rng.shuffle(shuffled)
        order.append([r["id"] for r in shuffled])
        for row in shuffled:
            print(f"Sample {sample + 1}/{args.samples} {row['id']}", flush=True)
            elapsed, data, stderr = image_run(row["command"], row["_env"], args.size, args.timeout)
            info = image_info(data)
            if info["sha256"] != row["warmup_image"]["sha256"]:
                raise ValueError(f"nondeterministic bitmap: {row['id']}")
            info.update({key: row["warmup_image"][key] for key in ("exact_reference_match", "different_pixels")})
            row["samples_seconds"].append(elapsed)
            row["samples_images"].append(info)
    for row in rows:
        row.pop("_env")
        row.update(median_seconds=statistics.median(row["samples_seconds"]),
                   min_seconds=min(row["samples_seconds"]), max_seconds=max(row["samples_seconds"]))
    report = {"schema_version": 1, "created_at": datetime.now(timezone.utc).isoformat(),
              "hardware": {"platform": platform.platform(), "machine": platform.machine(), "logical_cpus": os.cpu_count(),
                           "cpu": command(["sysctl", "-n", "machdep.cpu.brand_string"]).stdout.strip()},
              "toolchain_versions": versions, "compiler_snapshot": snapshot(args.output),
              "protocol": {"size": args.size, "samples": args.samples, "requested_threads": args.threads,
                           "controlled_only": args.controlled_only, "environment": overrides, "seed": args.seed, "sample_order": order,
                           "timed_region": "Fresh process through exit, including initialization/JIT and complete PBM output to a temporary regular file. Excludes AOT builds and output validation.",
                           "official_reference_url": REFERENCE_URL, "official_reference_200_sha256": REFERENCE_SHA,
                           "full_input_reference": "locally compiled unmodified Rust #4; every candidate additionally compared with full image; differing pixels retained and reported",
                           "full_input_reference_image": image_info(reference),
                           "source_metric": "Comment removal, whitespace collapse, UTF-8, gzip level 1 with mtime=0; also nonblank noncomment physical LOC. Local metric, not guaranteed CLBG gz.",
                           "source_scope": "Mech rows marked source_program count complete .mec including argument access, grid/map-until, the Mech-defined kernel and image/pbm output. Cranelift lowers recurrence and column traversal; generic thread scheduler/PBM library and compiler excluded. Older rows count kernel only and must be kept out of a complete-program size comparison. Grid coordinator extension is experimental; kernel uses the normal canonical artifact compiler. Competitors: complete unchanged upstream programs.",
                           "floating_point": "Native C/C++ disables contraction; fastmath configurations explicitly relax IEEE semantics; LLVM also tests fastmath with contraction disabled. Go tests default and compiler -d=fmahash=n. Mech FMA opt-in. Official N=200 requires byte equality; larger images record exact equality and differing pixel count.",
                           "threads": "Runtime controls request the chosen worker count where supported; each row records actual source worker policy. Fixed or automatic policies remain unmodified.",
                           "status": "full input; local protocol" if args.size == 16000 else "pilot"},
              "programs": rows, "excluded": excluded, "endpoints": common.endpoint_summary(rows),
              "nonheroic_endpoints": common.endpoint_summary([r for r in rows if not r["upstream_starred"]]),
              "exact_image_endpoints": common.endpoint_summary([r for r in rows if r["warmup_image"]["exact_reference_match"]])}
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Saved {args.output}: {len(rows)} programs; {len(excluded)} explicit exclusions", flush=True)
    for language, endpoint in report["endpoints"]["languages"].items():
        fastest = next(r for r in rows if r["id"] == endpoint["fastest"])
        print(language, endpoint["fastest"], round(fastest["median_seconds"], 6), endpoint["smallest"], flush=True)


if __name__ == "__main__":
    main()
