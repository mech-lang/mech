"""Compare exact encoder builds before and after one-shot cache accounting."""

from collections import Counter
from importlib import import_module
import json
import os
from pathlib import Path
from statistics import median
import subprocess
import sys
from time import perf_counter

measure = import_module("compare-builds")
q = measure.q
study = q / "stale-cache-accounting"
output = study / "matched"
checkout = q / "native-review-pr/checkout"
packet = checkout / "benchmarks/mech-deflate/native-profile/native-compact-20261116"
measure.source_mech = packet / "program.mec"
recipe = json.loads((packet / "recipe.json").read_text())
assert measure.sha(measure.source_mech) == recipe["source_sha256"]
assert measure.sha(measure.roc_app) == "e2a7a543f4fb8863e1dd3f98d62a5a29a12d16cb3fa2db1960e8d55af0d0c2f1"
assert measure.sha(measure.source_c / "lib/deflate_compress.c") == "a46dae12f2020b68d20c965ae43f9710e3e792bf77f3ca4611582f8d8d0c9541"
target = q / "host-opt-levels/opt3-cargo/release"
measure.emitter = target / "emit_compact_size"
frontends = {"control": study / "control-frontend", "accounting": target / "native_compact_frontend"}
assert measure.sha(frontends["control"]) != measure.sha(frontends["accounting"])
variants = {}
for case, name in (("retained", "cranelift-continuation-registers-word-pages"),
                   ("smaller", "cranelift-width-16-owners-cold")):
    reference = q / f"{name}-native/final-candidate"
    pins = json.loads((reference / "receipt.json").read_text())
    runtime = json.loads((reference / "runtime.json").read_text())
    assert runtime["image_sha256"] == recipe["bytecode_sha256"]
    assert not any("LLVM" in key for key in runtime["flags"])
    variants[case] = runtime, pins
libraries = {arg.removeprefix("mech_gpu=") for runtime, _ in variants.values()
             for arg in runtime["link"] if arg.startswith("mech_gpu=")}
library, = libraries
measure.mech_link = [library if path.endswith("/libmech_gpu_runtime.rlib") else path for path in measure.mech_link]
fixed_inputs = {str(path): measure.sha(path) for path in (
    *(Path(arg) for arg in measure.mech_link if arg.endswith((".rlib", ".o"))
      and arg != str(measure.reference / "program.o")),
    *(measure.setup / name for name in ("roc-adapter.o", "libdeflate-adapter.o", "libdeflate-alloc.o", "libdeflate-arm.o")),
)}
mech_actors = {f"mech-{case}-{mode}": (case, mode) for case in variants for mode in frontends}
actors = (*mech_actors, "roc", "libdeflate")
pattern = (0, 1, 5, 2, 4, 3)
orders = [[actors[(shift + i) % len(actors)] for i in pattern] for shift in range(len(actors))]
assert all(set(position) == set(actors) for position in zip(*orders))
assert len(Counter((a, b) for row in orders for a, b in zip(row, row[1:]))) == 30
normal_run = measure.run


def complete(actor, directory, profile=False):
    if actor in mech_actors:
        case, mode = mech_actors[actor]
        measure.runtime, measure.reference_pins = variants[case]
        measure.frontend = frontends[mode]
    peak_rss = {}

    def profile_run(command, output, env, instrument=False):
        start = perf_counter()
        markers = []
        with output.open("wb") as log:
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
            output.with_suffix(".stages.json").write_text(json.dumps(markers, indent=2) + "\n")
        peak_rss[output.stem] = usage.ru_maxrss
        return elapsed

    if profile:
        measure.run = profile_run
    try:
        result = measure.measure("mech" if actor in mech_actors else actor, directory, profile=profile)
    finally:
        measure.run = normal_run
    result["actor"] = actor
    result["commands"] = json.loads((directory / "commands.json").read_text())
    if actor in mech_actors:
        phases = json.loads((directory / "frontend/compile.json").read_text())["phases"]
        result["seconds"]["parse"] = next(row["seconds"] for row in phases if row["phase"] == "parse")
    if peak_rss:
        result["phase_peak_rss_bytes"] = peak_rss
    return result


output.mkdir()
warmups = [complete(actor, output / "warmup" / actor) for actor in actors]
trials = [complete(actor, output / f"order-{number}" / actor) for number, order in enumerate(orders, 1) for actor in order]
profiles = [complete(actor, output / "profile" / actor, True) for actor in actors]
identical = ("frontend/program.mecb", "native/program.o", "native/metadata.json", "compressor")
for row in (*warmups, *trials, *profiles):
    actor = row["actor"]
    exe = Path(row["executable"])
    if actor in mech_actors:
        case, _ = mech_actors[actor]
        anchor = output / "warmup" / f"mech-{case}-control"
        for name in identical:
            assert (anchor / name).read_bytes() == (exe.parent / name).read_bytes(), (actor, name)
    else:
        measure.verify_peer(exe)
for actor in mech_actors:
    directory = output / "warmup" / actor
    subprocess.run([sys.executable, "-B", q / "verify-worker.py", directory / "compressor",
                    directory / "native/metadata.json", f"stale-accounting-{actor}"], check=True)

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
        member, = (name for name in members if name.startswith("roc_app_"))
        obj = directory / "encoder-code.o"
        obj.write_bytes(subprocess.check_output(["/usr/bin/ar", "-p", archive, member]))
    else:
        obj = directory / ("native/program.o" if actor in mech_actors else "encoder.o")
    _, object_sections, _ = macho.load(obj)
    _, linked_sections, _ = macho.load(exe)
    sizes[actor] = {"encoder_text_bytes": object_sections[macho.TEXT][2], "encoder_file_bytes": obj.stat().st_size,
                    "linked_text_bytes": linked_sections[macho.TEXT][2], "executable_file_bytes": exe.stat().st_size}
    samples = [row["seconds"] for row in trials if row["actor"] == actor]
    medians[actor] = {key: median(row[key] for row in samples) for key in samples[0]}

record = {
    "scope": "Same Apple M1 source-to-executable boundary, including final linking. Compiler and fixed runtime, allocator and adapters prebuilt.",
    "frontend_sha256": {mode: measure.sha(path) for mode, path in frontends.items()},
    "emitter_sha256": measure.sha(measure.emitter), "source_sha256": recipe["source_sha256"],
    "bytecode_sha256": recipe["bytecode_sha256"], "fixed_link_inputs_sha256": fixed_inputs,
    "effective_native_flags": {case: runtime["flags"] for case, (runtime, _) in variants.items()},
    "roc_compiler_sha256": measure.sha(measure.roc), "roc_source_sha256": measure.sha(measure.roc_app),
    "c_source_sha256": measure.sha(measure.source_c / "lib/deflate_compress.c"),
    "orders": orders, "warmups": warmups, "trials": trials, "profiles": profiles,
    "identical_artifacts_within_each_configuration": identical, "verified_exact_calls": 648,
    "medians": medians, "sizes": sizes,
}
(output / "timing.json").write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps(medians, indent=2), flush=True)
