"""Measure split and in-process Cranelift builds with the same final links."""

from collections import Counter
from importlib import import_module
import json
import os
from pathlib import Path
import platform
from statistics import median
import subprocess
import sys
from time import perf_counter

measure = import_module("compare-builds")
q = measure.q
study = q / "direct-source-20261117"
output = study / "matched"
packet = q / "native-review-pr/checkout/benchmarks/mech-deflate/native-profile/native-compact-20261116"
measure.source_mech = packet / "program.mec"
recipe = json.loads((packet / "recipe.json").read_text())
assert measure.sha(measure.source_mech) == recipe["source_sha256"]
assert measure.sha(measure.roc_app) == "e2a7a543f4fb8863e1dd3f98d62a5a29a12d16cb3fa2db1960e8d55af0d0c2f1"
assert measure.sha(measure.source_c / "lib/deflate_compress.c") == "a46dae12f2020b68d20c965ae43f9710e3e792bf77f3ca4611582f8d8d0c9541"
target = q / "host-opt-levels/opt3-cargo/release"
measure.frontend = target / "native_compact_frontend"
measure.emitter = target / "emit_compact_size"
direct = target / "compile_profile"
variants = {}
for variant in ("retained", "smaller"):
    reference = q / f"stale-cache-accounting/pinned-{variant}"
    runtime = json.loads((reference / "runtime.json").read_text())
    assert runtime["image_sha256"] == recipe["bytecode_sha256"]
    runtime["flags"]["MECH_COMPACT_INLINE_SINGLE_CALLS"] = "preselected"
    assert not any("LLVM" in key for key in runtime["flags"])
    variants[variant] = runtime | {
        "expected": {
            "program.mecb": runtime["image_sha256"],
            "program.o": measure.sha(reference / "program.o"),
            "metadata.json": measure.sha(reference / "metadata.json"),
            "mech-worker": measure.sha(reference / "mech-worker"),
        }
    }

runtime_libraries = {
    arg.removeprefix("mech_gpu=")
    for runtime in variants.values() for arg in runtime["link"]
    if arg.startswith("mech_gpu=")
}
library, = runtime_libraries
measure.mech_link = [
    library if path.endswith("/libmech_gpu_runtime.rlib") else path for path in measure.mech_link
]
mech_link_inputs = {str(path): measure.sha(path) for path in (
    Path(arg) for arg in measure.mech_link if arg.endswith((".rlib", ".o"))
    and arg != str(measure.reference / "program.o")
)}
peer_link_inputs = {str(path): measure.sha(path) for path in (
    *(measure.setup / name for name in (
        "roc-adapter.o", "libdeflate-adapter.o", "libdeflate-alloc.o", "libdeflate-arm.o",
    )), measure.mimalloc,
)}

mech_actors = {
    f"mech-{variant}-{mode}": (variant, mode)
    for variant in variants for mode in ("split", "direct")
}
actors = (*mech_actors, "roc", "libdeflate")
pattern = (0, 1, 5, 2, 4, 3)
orders = [[actors[(shift + index) % len(actors)] for index in pattern] for shift in range(len(actors))]
assert all(set(position) == set(actors) for position in zip(*orders))
assert len(Counter((a, b) for order in orders for a, b in zip(order, order[1:]))) == 30


def run_child(command, log_path, env, instrument, peak_rss):
    start = perf_counter()
    markers = []
    with log_path.open("wb") as log:
        with subprocess.Popen(command, env=env, stdout=subprocess.PIPE if instrument else log,
                              stderr=subprocess.STDOUT) as worker:
            if instrument:
                for line in worker.stdout:
                    if line.startswith(b"compact-stage:"):
                        markers.append([perf_counter() - start, line.decode().strip()])
                    log.write(line)
            _, status, usage = os.wait4(worker.pid, 0)
            worker.returncode = os.waitstatus_to_exitcode(status)
            if worker.returncode:
                raise subprocess.CalledProcessError(worker.returncode, command)
        elapsed = perf_counter() - start
    if instrument:
        log_path.with_suffix(".stages.json").write_text(json.dumps(markers, indent=2) + "\n")
    peak_rss[log_path.stem] = usage.ru_maxrss
    return elapsed


def mech_build(actor, destination, profile, run):
    variant, mode = mech_actors[actor]
    runtime = variants[variant]
    destination.mkdir(parents=True)
    frontend_dir = destination / "frontend"
    native_dir = destination / "native"
    native_dir.mkdir()
    encoder = native_dir / "program.o"
    executable = destination / "compressor"
    native_env = measure.env | runtime["flags"]
    if mode == "split":
        commands = [
            ("frontend", [measure.frontend, measure.source_mech, frontend_dir, "frontend"], measure.env),
            ("native", [measure.emitter, frontend_dir / "program.mecb", native_dir], native_env),
        ]
    else:
        commands = [
            ("source_to_native", [direct, measure.source_mech, frontend_dir, "compact"], native_env)
        ]
    link = [
        str(encoder) if value == str(measure.reference / "program.o")
        else str(executable) if value == str(measure.setup / "mech-adapter-check") else value
        for value in measure.mech_link
    ]
    assert str(encoder) in link and str(executable) in link
    commands.append(("link", link, measure.env))
    timings = {}
    start = perf_counter()
    for phase, command, env in commands:
        timings[phase] = run(command, destination / f"{phase}.log", env,
                             instrument=profile and phase in ("native", "source_to_native"))
        if phase == "source_to_native":
            stage_start = perf_counter()
            for name in ("program.o", "metadata.json"):
                (frontend_dir / name).rename(native_dir / name)
            timings["stage_outputs"] = perf_counter() - stage_start
    timings["total"] = perf_counter() - start
    files = {"program.mecb": frontend_dir / "program.mecb", "program.o": encoder,
             "metadata.json": native_dir / "metadata.json", "compressor": executable}
    hashes = {name: measure.sha(path) for name, path in files.items()}
    for name in ("program.mecb", "program.o", "metadata.json"):
        assert hashes[name] == runtime["expected"][name], (actor, name)
    compiler = json.loads((frontend_dir / "compile.json").read_text())
    return {
        "actor": actor, "seconds": timings, "executable": str(executable),
        "sha256": hashes, "frontend_receipt": compiler,
        "commands": [[phase, list(map(str, command))] for phase, command, _ in commands],
    }


def complete(actor, directory, profile=False):
    peak_rss = {}

    def record(command, log_path, env, instrument=False):
        return run_child(command, log_path, env, instrument, peak_rss)

    if actor in mech_actors:
        result = mech_build(actor, directory, profile, record)
    else:
        original_run = measure.run
        measure.run = record
        try:
            result = measure.measure(actor, directory, profile=profile)
        finally:
            measure.run = original_run
        result["commands"] = json.loads((directory / "commands.json").read_text())
    result["phase_peak_rss_bytes"] = peak_rss
    print(actor, directory.parent.name, result["seconds"], max(peak_rss.values()), flush=True)
    (directory / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


def historical_worker(variant, destination, encoder, metadata):
    runtime = variants[variant]
    worker = destination / "mech-worker"
    flags = runtime["link"]
    output_argument = flags[flags.index("-o") + 1]
    args = [
        str(worker) if arg == output_argument else f"link-arg={encoder}" if arg.endswith("/program.o") else arg
        for arg in flags
    ]
    with (destination / "historical-link.log").open("w") as log:
        subprocess.run(args, env={k: v for k, v in os.environ.items() if not k.startswith("MECH_")},
                       stdout=log, stderr=subprocess.STDOUT, check=True)
    assert measure.sha(worker) == runtime["expected"]["mech-worker"]
    label = f"direct-source-historical-{variant}"
    subprocess.run([sys.executable, "-B", q / "verify-worker.py", worker, metadata, label], check=True)
    return {"command": args, "worker": str(worker), "worker_sha256": measure.sha(worker),
            "correctness": json.loads((q / f"{label}-qualification.json").read_text())}


def main():
    output.mkdir()
    inputs = {
        "scope": "Apple M1 encoder source through final link, with compiler, fixed runtime, adapters and allocator prebuilt.",
        "platform": platform.platform(),
        "source_sha256": recipe["source_sha256"], "bytecode_sha256": recipe["bytecode_sha256"],
        "compiler_sha256": {str(exe): measure.sha(exe) for exe in (measure.frontend, measure.emitter, direct)},
        "roc_compiler_sha256": measure.sha(measure.roc), "roc_source_sha256": measure.sha(measure.roc_app),
        "c_source_sha256": measure.sha(measure.source_c / "lib/deflate_compress.c"),
        "clang": subprocess.check_output([measure.cc[0], "--version"], text=True).splitlines()[0],
        "link_input_sha256": mech_link_inputs | peer_link_inputs,
        "native_flags": {name: runtime["flags"] for name, runtime in variants.items()},
        "expected_artifacts": {name: runtime["expected"] for name, runtime in variants.items()},
        "mech_rust_opt_level": 3, "orders": orders,
    }
    (output / "plan.json").write_text(json.dumps(inputs, indent=2) + "\n")
    warmups = [complete(actor, output / "warmup" / actor) for actor in actors]
    trials = [
        complete(actor, output / f"order-{number}" / actor)
        for number, order in enumerate(orders, 1) for actor in order
    ]
    profiles = [complete(actor, output / "profile" / actor, True) for actor in actors]
    for row in (*warmups, *trials, *profiles):
        actor = row["actor"]
        executable = Path(row["executable"])
        if actor in mech_actors:
            variant, _ = mech_actors[actor]
            control = next(item for item in warmups if item["actor"] == f"mech-{variant}-split")
            assert row["sha256"] == control["sha256"], (row, control)
        else:
            control = next(item for item in warmups if item["actor"] == actor)
            assert row["encoder_sha256"] == control["encoder_sha256"], actor
            assert row["executable_sha256"] == control["executable_sha256"], actor
            measure.verify_peer(executable)
    historical = {}
    for variant in variants:
        directory = output / "warmup" / f"mech-{variant}-direct"
        historical[variant] = historical_worker(
            variant, directory, directory / "native/program.o", directory / "native/metadata.json",
        )
        subprocess.run([sys.executable, "-B", q / "verify-worker.py", directory / "compressor",
                        directory / "native/metadata.json", f"direct-source-matched-{variant}"], check=True)

    sys.path.insert(0, str(q))
    macho = import_module("reachability-census")
    sizes = {}
    medians = {}
    for actor in actors:
        directory = output / "warmup" / actor
        executable = directory / "compressor"
        if actor == "roc":
            archive = directory / "encoder.a"
            members = subprocess.check_output(["/usr/bin/ar", "-t", archive], text=True).splitlines()
            member, = (name for name in members if name.startswith("roc_app_"))
            obj = directory / "encoder-code.o"
            obj.write_bytes(subprocess.check_output(["/usr/bin/ar", "-p", archive, member]))
        else:
            obj = directory / ("native/program.o" if actor in mech_actors else "encoder.o")
        _, object_sections, _ = macho.load(obj)
        _, linked_sections, _ = macho.load(executable)
        sizes[actor] = {
            "encoder_text_bytes": object_sections[macho.TEXT][2],
            "encoder_file_bytes": obj.stat().st_size,
            "linked_text_bytes": linked_sections[macho.TEXT][2],
            "executable_file_bytes": executable.stat().st_size,
        }
        samples = [row for row in trials if row["actor"] == actor]
        medians[actor] = {
            phase: median(row["seconds"][phase] for row in samples) for phase in samples[0]["seconds"]
        }
        medians[actor]["compiler_peak_rss_bytes"] = median(
            max(value for key, value in row["phase_peak_rss_bytes"].items() if key != "link")
            for row in samples
        )
        if actor in mech_actors:
            medians[actor]["compiler_phase_seconds"] = {
                entry["phase"]: median(
                    next(r["seconds"] for r in row["frontend_receipt"]["phases"] if r["phase"] == entry["phase"])
                    for row in samples
                ) for entry in samples[0]["frontend_receipt"]["phases"]
            }
    report = inputs | {
        "warmups": warmups, "trials": trials, "profiles": profiles,
        "medians": medians, "sizes": sizes, "historical_linked_workers": historical,
    }
    (output / "timing.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(medians, indent=2), flush=True)


if __name__ == "__main__":
    main()
