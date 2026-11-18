"""Measure opt-level 1 and 3 compiler binaries through final encoder linking."""

from importlib import import_module
from collections import Counter
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
study = q / "host-opt-levels"
output = study / "matched"
packet = q / "native-review-pr/checkout/benchmarks/mech-deflate/native-profile/native-compact-20261116"
measure.source_mech = packet / "program.mec"
recipe = json.loads((packet / "recipe.json").read_text())
assert measure.sha(measure.source_mech) == recipe["source_sha256"]
assert measure.sha(measure.roc_app) == "e2a7a543f4fb8863e1dd3f98d62a5a29a12d16cb3fa2db1960e8d55af0d0c2f1"
assert measure.sha(measure.source_c / "lib/deflate_compress.c") == "a46dae12f2020b68d20c965ae43f9710e3e792bf77f3ca4611582f8d8d0c9541"

builds = {
    level: json.loads((study / name / "rebuild.json").read_text())["compiler_build"]
    for level, name in ((1, "control-rebuild"), (3, "optimized-fixed-allocator"))
}
profile_key = "CARGO_PROFILE_RELEASE_OPT_LEVEL"
assert {k for k, value in builds[1]["profile"].items() if value != builds[3]["profile"][k]} == {profile_key}
assert [builds[level]["profile"][profile_key] for level in (1, 3)] == ["1", "3"]
assert builds[3]["package_profile"] == "profile.release.package.libmimalloc-sys.opt-level=1"
host_allocator = json.loads((study / "compiler-allocator-identity.json").read_text())
assert host_allocator["opt1_archive_sha256"] == host_allocator["opt3_archive_sha256"]
compiler_paths = {1: study / "opt1-compiler", 3: study / "opt3-cargo/release"}
compilers = {
    level: {name: {"path": str(folder / name), "sha256": measure.sha(folder / name)}
            for name in ("native_compact_frontend", "emit_compact_size")}
    for level, folder in compiler_paths.items()
}
control = json.loads((study / "opt1-binaries.json").read_text())
assert all(compilers[1][name]["sha256"] == control[name]["sha256"] for name in control)

variants = {}
for name, label in (
    ("baseline", "cranelift-continuation-registers-word-pages"),
    ("fresh-owner-counts-cold-tails", "cranelift-width-16-owners-cold"),
):
    reference = q / f"{label}-native/final-candidate"
    runtime = json.loads((reference / "runtime.json").read_text())
    pins = json.loads((reference / "receipt.json").read_text())
    assert runtime["image_sha256"] == recipe["bytecode_sha256"]
    assert pins["object_sha256"] == recipe[name.replace("-", "_") + "_object_sha256"]
    assert pins["metadata_sha256"] == recipe[name.replace("-", "_") + "_metadata_sha256"]
    assert not any("LLVM" in key for key in runtime["flags"])
    variants[name] = runtime, pins

runtime_libraries = {
    value.removeprefix("mech_gpu=")
    for runtime, _ in variants.values()
    for value in runtime["link"] if value.startswith("mech_gpu=")
}
library, = runtime_libraries
measure.mech_link = [library if path.endswith("/libmech_gpu_runtime.rlib") else path for path in measure.mech_link]
fixed_inputs = {
    str(path): measure.sha(path) for path in (
        *(Path(arg) for arg in measure.mech_link if arg.endswith((".rlib", ".o"))
          and arg != str(measure.reference / "program.o")),
        *(measure.setup / name for name in ("roc-adapter.o", "libdeflate-adapter.o", "libdeflate-alloc.o", "libdeflate-arm.o")),
    )
}
mech_actors = {
    f"mech-{short}-opt{level}": (variant, level)
    for short, variant in (("retained", "baseline"), ("smaller", "fresh-owner-counts-cold-tails"))
    for level in (1, 3)
}
actors = (*mech_actors, "roc", "libdeflate")
order_pattern = (0, 1, 5, 2, 4, 3)
orders = [[actors[(index + shift) % len(actors)] for index in order_pattern] for shift in range(len(actors))]
assert all(set(position) == set(actors) for position in zip(*orders))
assert len(Counter((first, second) for order in orders for first, second in zip(order, order[1:]))) == 30
run_phase = measure.run


def run(actor, directory, profile=False):
    if actor in mech_actors:
        variant, level = mech_actors[actor]
        measure.runtime, measure.reference_pins = variants[variant]
        measure.frontend = compiler_paths[level] / "native_compact_frontend"
        measure.emitter = compiler_paths[level] / "emit_compact_size"
    rss = {}

    def diagnostic_phase(command, log, environment, instrument=False):
        events = []
        if instrument:
            environment = environment | {"MECH_PROFILE_COMPILE_JOBS": "1"}
        with log.open("wb") as stream:
            start = perf_counter()
            with subprocess.Popen(command, stdout=subprocess.PIPE if instrument else stream,
                                  stderr=subprocess.STDOUT, env=environment) as child:
                if instrument:
                    for line in child.stdout:
                        if line.startswith((b"compact-stage:", b"compact-compile-jobs:")):
                            events.append([perf_counter() - start, line.decode().strip()])
                        stream.write(line)
                _, status, usage = os.wait4(child.pid, 0)
                child.returncode = os.waitstatus_to_exitcode(status)
                if child.returncode:
                    raise subprocess.CalledProcessError(child.returncode, command)
            elapsed = perf_counter() - start
        if instrument:
            log.with_suffix(".stages.json").write_text(json.dumps(events, indent=2) + "\n")
        rss[log.stem] = usage.ru_maxrss if platform.system() == "Darwin" else usage.ru_maxrss * 1024
        return elapsed

    if profile:
        measure.run = diagnostic_phase
    try:
        result = measure.measure("mech" if actor in mech_actors else actor, directory, profile=profile)
    finally:
        measure.run = run_phase
    result["actor"] = actor
    result["commands"] = json.loads((directory / "commands.json").read_text())
    if actor in mech_actors:
        phases = json.loads((directory / "frontend/compile.json").read_text())["phases"]
        result["frontend_phases"] = phases
        result["seconds"]["parse"] = next(row["seconds"] for row in phases if row["phase"] == "parse")
    if rss:
        result["phase_peak_rss_bytes"] = rss
    (directory / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


output.mkdir()
warmup = [run(actor, output / "warmup" / actor) for actor in actors]
trials = [run(actor, output / f"order-{index}" / actor)
          for index, order in enumerate(orders, 1) for actor in order]
diagnostics = [run(actor, output / "profile" / actor, profile=True) for actor in actors]
identical = ("frontend/program.mecb", "native/program.o", "native/metadata.json", "compressor")
anchors = {
    variant: output / "warmup" / actor
    for actor, (variant, level) in mech_actors.items() if level == 1
}
for row in (*warmup, *trials, *diagnostics):
    exe = Path(row["executable"])
    actor = row["actor"]
    if actor in mech_actors:
        reference = anchors[mech_actors[actor][0]]
        for path in identical:
            assert (reference / path).read_bytes() == (exe.parent / path).read_bytes(), (actor, path)
    else:
        measure.verify_peer(exe)

sys.path.insert(0, str(q))
macho = import_module("reachability-census")
sizes = {}
medians = {}
for actor in actors:
    directory = output / "warmup" / actor
    exe = directory / "compressor"
    if actor == "roc":
        archive = directory / "encoder.a"
        members = subprocess.check_output(["/usr/bin/ar", "-t", archive], text=True).splitlines()
        member, = [name for name in members if name.startswith("roc_app_")]
        obj = directory / "encoder-code.o"
        obj.write_bytes(subprocess.check_output(["/usr/bin/ar", "-p", archive, member]))
    else:
        obj = directory / ("native/program.o" if actor in mech_actors else "encoder.o")
    _, object_sections, _ = macho.load(obj)
    _, linked_sections, _ = macho.load(exe)
    sizes[actor] = {
        "encoder_text_bytes": object_sections[macho.TEXT][2],
        "encoder_file_bytes": obj.stat().st_size,
        "linked_text_bytes": linked_sections[macho.TEXT][2],
        "executable_file_bytes": exe.stat().st_size,
    }
    if actor in mech_actors:
        subprocess.run([sys.executable, "-B", q / "verify-worker.py", exe, directory / "native/metadata.json",
                        f"host-opt-levels-{actor}"], check=True)
    samples = [row["seconds"] for row in trials if row["actor"] == actor]
    medians[actor] = {key: median(row[key] for row in samples) for key in samples[0]}

record = {
    "machine": platform.platform(),
    "clang": subprocess.check_output([measure.cc[0], "--version"], text=True).splitlines()[0],
    "rustc": subprocess.check_output(["rustc", "+nightly-2026-03-03", "-vV"], text=True),
    "timing_boundary": "Fresh encoder source through final executable link; compilers, runtimes and adapters prebuilt.",
    "source_sha256": recipe["source_sha256"],
    "compiler_builds": builds, "compilers": compilers,
    "compiler_allocator": host_allocator,
    "fixed_mech_runtime_sha256": measure.sha(Path(library)),
    "allocator_sha256": measure.sha(measure.mimalloc),
    "fixed_link_input_sha256": fixed_inputs,
    "roc_compiler_sha256": measure.sha(measure.roc), "roc_source_sha256": measure.sha(measure.roc_app),
    "c_source_sha256": measure.sha(measure.source_c / "lib/deflate_compress.c"),
    "effective_native_flags": {name: runtime["flags"] for name, (runtime, _) in variants.items()},
    "diagnostic_native_flags": {"MECH_PROFILE_COMPILE_JOBS": "1"},
    "orders": orders, "warmup": warmup, "trials": trials, "diagnostics": diagnostics,
    "medians": medians, "sizes": sizes,
    "byte_identical_between_levels_for_each_encoder_configuration": identical,
    "verified_exact_calls": 162 * len(mech_actors),
}
(output / "timing.json").write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps({"medians": medians, "sizes": sizes}, indent=2), flush=True)
